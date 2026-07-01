use std::collections::HashMap;

use furnace_core::{DEFAULT_PRICE_MA_WINDOWS, DEFAULT_VOLUME_MA_WINDOWS, MaPreviousState, MaState};

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{MaRunRequest, MaWriteMode};
use crate::rows::{
    CountRow, MaInputRow, MaPreviousStateRow, OptionalDateValueRow, SecurityCodeRow,
};
use crate::runners::shared::normalize_symbols;
use crate::sql::{sql_string, symbol_where_clause, symbol_where_clause_for_column};
use crate::validation::format_clickhouse_date;
pub(super) fn read_previous_ma_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<HashMap<String, MaPreviousState>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    trade_date,
    assumeNotNull(price_ema1_10_state) AS price_ema1_10_state,
    assumeNotNull(price_ema2_10_state) AS price_ema2_10_state
FROM (
    SELECT
        security_code,
        trade_date,
        price_ema1_10_state,
        price_ema2_10_state,
        row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
    FROM {}
    WHERE trade_date < toDate('{}')
      AND price_ema1_10_state IS NOT NULL
      AND price_ema2_10_state IS NOT NULL
      AND {}
)
WHERE rn = 1",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    let mut states = HashMap::new();
    for row in executor.fetch_all::<MaPreviousStateRow>(&sql)? {
        states.insert(
            row.security_code,
            MaPreviousState::new(
                format_clickhouse_date(row.trade_date),
                MaState::new(row.price_ema1_10_state, row.price_ema2_10_state)
                    .map_err(|source| FurnaceIoError::Parse(source.to_string()))?,
            ),
        );
    }
    Ok(states)
}
pub(super) fn resolve_ma_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
) -> Result<Vec<String>, FurnaceIoError> {
    if !request.symbols.is_empty() {
        return Ok(normalize_symbols(&request.symbols));
    }

    let sql = format!(
        "\
SELECT security_code
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
GROUP BY security_code
ORDER BY security_code
",
        request.input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    Ok(executor
        .fetch_all::<SecurityCodeRow>(&sql)?
        .into_iter()
        .map(|row| row.security_code)
        .collect())
}

pub(super) fn resolve_ma_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != MaWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT if(count() = 0, NULL, max(trade_date)) AS value
FROM {}
WHERE {}",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value = executor
        .fetch_one::<OptionalDateValueRow>(&sql)?
        .value
        .map(format_clickhouse_date)
        .unwrap_or_else(|| request.request_to.clone());
    Ok(value.max(request.request_to.clone()))
}

pub(super) fn resolve_ma_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let sql = format!(
        "\
SELECT if(count() = 0, NULL, min(trade_date)) AS value
FROM {}
WHERE {}",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    Ok(executor
        .fetch_one::<OptionalDateValueRow>(&sql)?
        .value
        .map(format_clickhouse_date)
        .unwrap_or_else(|| request.request_from.clone()))
}
pub(super) fn resolve_ma_lookback_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let price_symbol_filter = symbol_where_clause(symbols, all_symbols_requested);
    let volume_symbol_filter =
        symbol_where_clause_for_column("adj.security_code", symbols, all_symbols_requested);
    let price_lookback_window = DEFAULT_PRICE_MA_WINDOWS
        .iter()
        .copied()
        .max()
        .unwrap_or(250);
    let volume_lookback_window = DEFAULT_VOLUME_MA_WINDOWS
        .iter()
        .copied()
        .max()
        .unwrap_or(60);
    let sql = format!(
        "\
SELECT if(count() = 0, NULL, min(trade_date)) AS value
FROM (
    SELECT trade_date
    FROM (
        SELECT
            security_code,
            trade_date,
            row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
        FROM {}
        WHERE trade_date <= toDate('{}')
          AND {} IS NOT NULL
          AND {price_symbol_filter}
    )
    WHERE rn <= {price_lookback_window}
    UNION ALL
    SELECT trade_date
    FROM (
        SELECT
            adj.security_code,
            adj.trade_date,
            row_number() OVER (PARTITION BY adj.security_code ORDER BY adj.trade_date DESC) AS rn
        FROM {} AS adj
        LEFT JOIN {} AS unadj
          ON adj.security_code = unadj.security_code
         AND adj.trade_date = unadj.trade_date
        WHERE adj.trade_date <= toDate('{}')
          AND unadj.{} IS NOT NULL
          AND {volume_symbol_filter}
    )
    WHERE rn <= {volume_lookback_window}
)
",
        request.input_table,
        sql_string(&request.request_from),
        request.price_column,
        request.input_table,
        request.volume_input_table,
        sql_string(&request.request_from),
        request.volume_column
    );
    Ok(executor
        .fetch_one::<OptionalDateValueRow>(&sql)?
        .value
        .map(format_clickhouse_date)
        .unwrap_or_else(|| request.request_from.clone()))
}

pub(super) fn ma_symbols_started_before<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    input_from: &str,
) -> Result<bool, FurnaceIoError> {
    if symbols.is_empty() {
        return Ok(false);
    }
    let sql = format!(
        "\
SELECT count() AS value
FROM {}
WHERE trade_date < toDate('{}')
  AND {}",
        request.input_table,
        sql_string(input_from),
        symbol_where_clause(symbols, false)
    );
    let count = executor.fetch_one::<CountRow>(&sql)?.value;
    Ok(count > 0)
}
pub(super) fn read_ma_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<MaInputRow>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    adj.security_code,
    adj.trade_date,
    adj.{} AS close_price,
    CAST(unadj.{}, 'Nullable(Float64)') AS volume
FROM {} AS adj
LEFT JOIN {} AS unadj
  ON adj.security_code = unadj.security_code
 AND adj.trade_date = unadj.trade_date
WHERE adj.trade_date >= toDate('{}')
  AND adj.trade_date <= toDate('{}')
  AND {}
ORDER BY adj.security_code, adj.trade_date",
        request.price_column,
        request.volume_column,
        request.input_table,
        request.volume_input_table,
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause_for_column("adj.security_code", symbols, all_symbols_requested)
    );

    executor.fetch_all::<MaInputRow>(&sql)
}
