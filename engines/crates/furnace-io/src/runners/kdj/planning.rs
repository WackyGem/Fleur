use std::collections::HashMap;

use furnace_core::KdjState;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{KdjRunRequest, KdjWriteMode};
use crate::rows::{KdjInputRow, KdjPreviousStateRow, OptionalDateValueRow, SecurityCodeRow};
use crate::runners::shared::normalize_symbols;
use crate::schema::{DEFAULT_INPUT_TABLE, DEFAULT_KDJ_OUTPUT_TABLE, DEFAULT_WARMUP_MULTIPLE};
use crate::sql::{sql_string, symbol_where_clause};
use crate::validation::format_clickhouse_date;
pub(super) fn resolve_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
) -> Result<Vec<String>, FurnaceIoError> {
    if !request.symbols.is_empty() {
        return Ok(normalize_symbols(&request.symbols));
    }

    let sql = format!(
        "\
SELECT security_code
FROM {DEFAULT_INPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
GROUP BY security_code
ORDER BY security_code
",
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    Ok(executor
        .fetch_all::<SecurityCodeRow>(&sql)?
        .into_iter()
        .map(|row| row.security_code)
        .collect())
}
pub(super) fn resolve_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != KdjWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT if(count() = 0, NULL, max(trade_date)) AS value
FROM {DEFAULT_INPUT_TABLE}
WHERE {}",
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value = executor
        .fetch_one::<OptionalDateValueRow>(&sql)?
        .value
        .map(format_clickhouse_date)
        .unwrap_or_else(|| request.request_to.clone());
    Ok(value.max(request.request_to.clone()))
}

pub(super) fn resolve_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let warmup_window = u32::from(
        request
            .params
            .rsv_window
            .max(request.params.k_smoothing)
            .max(request.params.d_smoothing),
    ) * u32::from(DEFAULT_WARMUP_MULTIPLE);

    let symbol_filter = symbol_where_clause(symbols, all_symbols_requested);
    let sql = format!(
        "\
SELECT if(count() = 0, NULL, min(trade_date)) AS value
FROM (
    SELECT trade_date
    FROM {DEFAULT_INPUT_TABLE}
    WHERE trade_date <= toDate('{}')
      AND {symbol_filter}
    GROUP BY trade_date
    ORDER BY trade_date DESC
    LIMIT {warmup_window}
)",
        sql_string(&request.request_from)
    );
    Ok(executor
        .fetch_one::<OptionalDateValueRow>(&sql)?
        .value
        .map(format_clickhouse_date)
        .unwrap_or_else(|| request.request_from.clone()))
}
pub(super) fn read_previous_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<HashMap<String, KdjState>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    assumeNotNull(k_value) AS k_value,
    assumeNotNull(d_value) AS d_value
FROM (
    SELECT
        security_code,
        trade_date,
        k_value,
        d_value,
        row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
    FROM {DEFAULT_KDJ_OUTPUT_TABLE}
    WHERE trade_date < toDate('{}')
      AND k_value IS NOT NULL
      AND d_value IS NOT NULL
      AND {}
)
WHERE rn = 1",
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    let mut states = HashMap::new();
    for row in executor.fetch_all::<KdjPreviousStateRow>(&sql)? {
        states.insert(row.security_code, KdjState::new(row.k_value, row.d_value));
    }
    Ok(states)
}
pub(super) fn read_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<KdjInputRow>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    trade_date,
    high_price_forward_adj AS high_price,
    low_price_forward_adj AS low_price,
    close_price_forward_adj AS close_price
FROM {DEFAULT_INPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date",
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    executor.fetch_all::<KdjInputRow>(&sql)
}
