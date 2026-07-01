use std::collections::HashMap;

use furnace_core::{RsiPreviousState, RsiState, RsiWindowState};

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{RsiRunRequest, RsiWriteMode};
use crate::rows::{
    CloseInputRow, GapCountRow, OptionalDateValueRow, RsiPreviousStateRow, SecurityCodeRow,
};
use crate::runners::shared::normalize_symbols;
use crate::sql::{sql_string, symbol_where_clause, symbol_where_clause_for};
use crate::validation::format_clickhouse_date;
pub(super) fn read_previous_rsi_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<HashMap<String, RsiPreviousState>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT
    state.security_code,
    state.state_date,
    assumeNotNull(input.close_price_forward_adj) AS previous_close,
    assumeNotNull(state.state_avg_gain_6) AS state_avg_gain_6,
    assumeNotNull(state.state_avg_loss_6) AS state_avg_loss_6,
    assumeNotNull(state.state_avg_gain_12) AS state_avg_gain_12,
    assumeNotNull(state.state_avg_loss_12) AS state_avg_loss_12,
    assumeNotNull(state.state_avg_gain_14) AS state_avg_gain_14,
    assumeNotNull(state.state_avg_loss_14) AS state_avg_loss_14,
    assumeNotNull(state.state_avg_gain_24) AS state_avg_gain_24,
    assumeNotNull(state.state_avg_loss_24) AS state_avg_loss_24,
    assumeNotNull(state.state_avg_gain_25) AS state_avg_gain_25,
    assumeNotNull(state.state_avg_loss_25) AS state_avg_loss_25,
    assumeNotNull(state.state_avg_gain_50) AS state_avg_gain_50,
    assumeNotNull(state.state_avg_loss_50) AS state_avg_loss_50
FROM (
    SELECT
        security_code,
        max(trade_date) AS state_date,
        argMax(avg_gain_6_state, trade_date) AS state_avg_gain_6,
        argMax(avg_loss_6_state, trade_date) AS state_avg_loss_6,
        argMax(avg_gain_12_state, trade_date) AS state_avg_gain_12,
        argMax(avg_loss_12_state, trade_date) AS state_avg_loss_12,
        argMax(avg_gain_14_state, trade_date) AS state_avg_gain_14,
        argMax(avg_loss_14_state, trade_date) AS state_avg_loss_14,
        argMax(avg_gain_24_state, trade_date) AS state_avg_gain_24,
        argMax(avg_loss_24_state, trade_date) AS state_avg_loss_24,
        argMax(avg_gain_25_state, trade_date) AS state_avg_gain_25,
        argMax(avg_loss_25_state, trade_date) AS state_avg_loss_25,
        argMax(avg_gain_50_state, trade_date) AS state_avg_gain_50,
        argMax(avg_loss_50_state, trade_date) AS state_avg_loss_50
    FROM {}
    WHERE trade_date < toDate('{}')
      AND avg_gain_6_state IS NOT NULL
      AND avg_loss_6_state IS NOT NULL
      AND avg_gain_12_state IS NOT NULL
      AND avg_loss_12_state IS NOT NULL
      AND avg_gain_14_state IS NOT NULL
      AND avg_loss_14_state IS NOT NULL
      AND avg_gain_24_state IS NOT NULL
      AND avg_loss_24_state IS NOT NULL
      AND avg_gain_25_state IS NOT NULL
      AND avg_loss_25_state IS NOT NULL
      AND avg_gain_50_state IS NOT NULL
      AND avg_loss_50_state IS NOT NULL
      AND {}
    GROUP BY security_code
) AS state
INNER JOIN {} AS input
    ON input.security_code = state.security_code
   AND input.trade_date = state.state_date
WHERE input.close_price_forward_adj IS NOT NULL",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table
    );

    let mut states = HashMap::new();
    for row in executor.fetch_all::<RsiPreviousStateRow>(&sql)? {
        let state = RsiState::new(
            row.previous_close,
            rsi_window_state(row.state_avg_gain_6, row.state_avg_loss_6)?,
            rsi_window_state(row.state_avg_gain_12, row.state_avg_loss_12)?,
            rsi_window_state(row.state_avg_gain_14, row.state_avg_loss_14)?,
            rsi_window_state(row.state_avg_gain_24, row.state_avg_loss_24)?,
            rsi_window_state(row.state_avg_gain_25, row.state_avg_loss_25)?,
            rsi_window_state(row.state_avg_gain_50, row.state_avg_loss_50)?,
        )
        .map_err(|source| FurnaceIoError::Parse(source.to_string()))?;
        states.insert(
            row.security_code,
            RsiPreviousState::new(format_clickhouse_date(row.state_date), state),
        );
    }
    Ok(states)
}

pub(super) fn rsi_window_state(gain: f64, loss: f64) -> Result<RsiWindowState, FurnaceIoError> {
    RsiWindowState::new(gain, loss).map_err(|source| FurnaceIoError::Parse(source.to_string()))
}

pub(super) fn count_rsi_gap_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
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
      AND avg_gain_6_state IS NOT NULL
      AND avg_loss_6_state IS NOT NULL
      AND avg_gain_12_state IS NOT NULL
      AND avg_loss_12_state IS NOT NULL
      AND avg_gain_14_state IS NOT NULL
      AND avg_loss_14_state IS NOT NULL
      AND avg_gain_24_state IS NOT NULL
      AND avg_loss_24_state IS NOT NULL
      AND avg_gain_25_state IS NOT NULL
      AND avg_loss_25_state IS NOT NULL
      AND avg_gain_50_state IS NOT NULL
      AND avg_loss_50_state IS NOT NULL
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
  AND input.close_price_forward_adj IS NOT NULL",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table,
        sql_string(&request.request_from)
    );
    let row = executor.fetch_one::<GapCountRow>(&sql)?;
    Ok((
        row.gap_symbols,
        row.gap_fill_from.map(format_clickhouse_date),
    ))
}
pub(super) fn resolve_rsi_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
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

pub(super) fn resolve_rsi_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != RsiWriteMode::ReplaceCascade {
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

pub(super) fn resolve_rsi_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
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
pub(super) fn read_rsi_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
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

pub(super) fn read_rsi_mixed_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
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
      AND avg_gain_6_state IS NOT NULL
      AND avg_loss_6_state IS NOT NULL
      AND avg_gain_12_state IS NOT NULL
      AND avg_loss_12_state IS NOT NULL
      AND avg_gain_14_state IS NOT NULL
      AND avg_loss_14_state IS NOT NULL
      AND avg_gain_24_state IS NOT NULL
      AND avg_loss_24_state IS NOT NULL
      AND avg_gain_25_state IS NOT NULL
      AND avg_loss_25_state IS NOT NULL
      AND avg_gain_50_state IS NOT NULL
      AND avg_loss_50_state IS NOT NULL
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
