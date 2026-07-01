use std::collections::HashMap;

use furnace_core::{MacdPreviousState, MacdState};

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{MacdRunRequest, MacdWriteMode};
use crate::rows::{
    CloseInputRow, CountRow, GapCountRow, MacdPreviousStateRow, OptionalDateValueRow,
    SecurityCodeRow,
};
use crate::runners::shared::normalize_symbols;
use crate::sql::{sql_string, symbol_where_clause, symbol_where_clause_for};
use crate::validation::format_clickhouse_date;

pub(super) fn read_previous_macd_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<HashMap<String, MacdPreviousState>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    trade_date,
    assumeNotNull(ema_fast_state_12) AS ema_fast_state_12,
    assumeNotNull(ema_slow_state_26) AS ema_slow_state_26,
    assumeNotNull(macd_dea_state) AS macd_dea_state
FROM (
    SELECT
        security_code,
        trade_date,
        ema_fast_state_12,
        ema_slow_state_26,
        macd_dea_state,
        row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
    FROM {}
    WHERE trade_date < toDate('{}')
      AND ema_fast_state_12 IS NOT NULL
      AND ema_slow_state_26 IS NOT NULL
      AND macd_dea_state IS NOT NULL
      AND {}
)
WHERE rn = 1",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    let mut states = HashMap::new();
    for row in executor.fetch_all::<MacdPreviousStateRow>(&sql)? {
        states.insert(
            row.security_code,
            MacdPreviousState::new(
                format_clickhouse_date(row.trade_date),
                MacdState::new(
                    row.ema_fast_state_12,
                    row.ema_slow_state_26,
                    row.macd_dea_state,
                )
                .map_err(|source| FurnaceIoError::Parse(source.to_string()))?,
            ),
        );
    }
    Ok(states)
}

pub(super) fn count_macd_incomplete_state_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<u64, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(0);
    }
    let sql = format!(
        "\
SELECT countDistinct(security_code) AS value
FROM {}
WHERE trade_date < toDate('{}')
  AND (ema_fast_state_12 IS NOT NULL OR ema_slow_state_26 IS NOT NULL OR macd_dea_state IS NOT NULL)
  AND (ema_fast_state_12 IS NULL OR ema_slow_state_26 IS NULL OR macd_dea_state IS NULL)
  AND {}",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );
    Ok(executor.fetch_one::<CountRow>(&sql)?.value)
}

pub(super) fn count_macd_gap_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<(u64, Option<String>), FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok((0, None));
    }
    let sql = format!(
        "\
WITH states AS (
    SELECT security_code, max(trade_date) AS state_date
    FROM {}
    WHERE trade_date < toDate('{}')
      AND ema_fast_state_12 IS NOT NULL
      AND ema_slow_state_26 IS NOT NULL
      AND macd_dea_state IS NOT NULL
      AND {}
    GROUP BY security_code
)
SELECT
    countDistinct(input.security_code) AS gap_symbols,
    if(gap_symbols = 0, NULL, min(input.trade_date)) AS gap_fill_from
FROM {} AS input
INNER JOIN states
    ON input.security_code = states.security_code
WHERE input.trade_date > states.state_date
  AND input.trade_date < toDate('{}')
  AND input.{} IS NOT NULL",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table,
        sql_string(&request.request_from),
        request.price_column
    );
    let row = executor.fetch_one::<GapCountRow>(&sql)?;
    Ok((
        row.gap_symbols,
        row.gap_fill_from.map(format_clickhouse_date),
    ))
}

pub(super) fn resolve_macd_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
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

pub(super) fn resolve_macd_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != MacdWriteMode::ReplaceCascade {
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

pub(super) fn resolve_macd_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
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

pub(super) fn read_macd_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<CloseInputRow>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    trade_date,
    {} AS close_price
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date",
        request.price_column,
        request.input_table,
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    executor.fetch_all::<CloseInputRow>(&sql)
}

pub(super) fn read_macd_mixed_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_to: &str,
) -> Result<Vec<CloseInputRow>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
WITH states AS (
    SELECT security_code, max(trade_date) AS state_date
    FROM {}
    WHERE trade_date < toDate('{}')
      AND ema_fast_state_12 IS NOT NULL
      AND ema_slow_state_26 IS NOT NULL
      AND macd_dea_state IS NOT NULL
      AND {}
    GROUP BY security_code
)
SELECT
    input.security_code,
    input.trade_date,
    input.{} AS close_price
FROM {} AS input
LEFT JOIN states
    ON input.security_code = states.security_code
WHERE input.trade_date <= toDate('{}')
  AND {}
  AND (states.state_date IS NULL OR input.trade_date >= states.state_date)
ORDER BY input.security_code, input.trade_date",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.price_column,
        request.input_table,
        sql_string(input_to),
        symbol_where_clause_for("input.security_code", symbols, all_symbols_requested)
    );

    executor.fetch_all::<CloseInputRow>(&sql)
}
