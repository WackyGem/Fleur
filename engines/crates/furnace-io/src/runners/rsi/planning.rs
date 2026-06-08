use std::collections::HashMap;

use furnace_core::{RsiPreviousState, RsiState, RsiWindowState};

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{RsiRunRequest, RsiWriteMode};
use crate::runners::shared::normalize_symbols;
use crate::sql::{
    first_tsv_value, parse_f64, parse_single_column_strings, parse_u64, sql_string,
    symbol_where_clause, symbol_where_clause_for,
};
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
    toString(state.state_date),
    input.close_price_forward_adj,
    state.state_avg_gain_6,
    state.state_avg_loss_6,
    state.state_avg_gain_12,
    state.state_avg_loss_12,
    state.state_avg_gain_14,
    state.state_avg_loss_14,
    state.state_avg_gain_24,
    state.state_avg_loss_24,
    state.state_avg_gain_25,
    state.state_avg_loss_25,
    state.state_avg_gain_50,
    state.state_avg_loss_50
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
WHERE input.close_price_forward_adj IS NOT NULL
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table
    );

    let mut states = HashMap::new();
    for line in executor
        .query(&sql)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 15 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 15 previous RSI state fields, got {}",
                fields.len()
            )));
        }
        let previous_close = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous RSI close must not be null".to_string())
        })?;
        let state = RsiState::new(
            previous_close,
            rsi_window_state(fields[3], fields[4])?,
            rsi_window_state(fields[5], fields[6])?,
            rsi_window_state(fields[7], fields[8])?,
            rsi_window_state(fields[9], fields[10])?,
            rsi_window_state(fields[11], fields[12])?,
            rsi_window_state(fields[13], fields[14])?,
        )
        .map_err(|source| FurnaceIoError::Parse(source.to_string()))?;
        states.insert(
            fields[0].to_string(),
            RsiPreviousState::new(fields[1].to_string(), state),
        );
    }
    Ok(states)
}

pub(super) fn rsi_window_state(gain: &str, loss: &str) -> Result<RsiWindowState, FurnaceIoError> {
    let gain = parse_f64(gain)?
        .ok_or_else(|| FurnaceIoError::Parse("previous RSI gain must not be null".to_string()))?;
    let loss = parse_f64(loss)?
        .ok_or_else(|| FurnaceIoError::Parse("previous RSI loss must not be null".to_string()))?;
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
    countDistinct(input.security_code),
    toString(min(input.trade_date))
FROM {} AS input
INNER JOIN states
    ON input.security_code = states.security_code
WHERE input.trade_date > states.state_date
  AND input.trade_date < toDate('{}')
  AND input.close_price_forward_adj IS NOT NULL
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table,
        sql_string(&request.request_from)
    );
    let output = executor.query(&sql)?;
    let fields = output
        .lines()
        .next()
        .unwrap_or_default()
        .split('\t')
        .collect::<Vec<_>>();
    let gap_symbols = fields
        .first()
        .map(|value| parse_u64(value))
        .transpose()?
        .unwrap_or(0);
    let gap_fill_from = fields.get(1).and_then(|value| {
        if gap_symbols == 0 || value.is_empty() || *value == "\\N" {
            None
        } else {
            Some((*value).to_string())
        }
    });
    Ok((gap_symbols, gap_fill_from))
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
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
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
SELECT toString(max(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
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
SELECT toString(min(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}
pub(super) fn read_rsi_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<u8>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    toString(trade_date),
    {}
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date
FORMAT RowBinary",
        request.price_column,
        request.input_table,
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}

pub(super) fn read_rsi_mixed_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_to: &str,
) -> Result<Vec<u8>, FurnaceIoError> {
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
    toString(input.trade_date),
    input.{}
FROM {} AS input
LEFT JOIN states
    ON input.security_code = states.security_code
WHERE input.trade_date <= toDate('{}')
  AND {}
  AND (states.state_date IS NULL OR input.trade_date >= states.state_date)
ORDER BY input.security_code, input.trade_date
FORMAT RowBinary",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.price_column,
        request.input_table,
        sql_string(input_to),
        symbol_where_clause_for("input.security_code", symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}
