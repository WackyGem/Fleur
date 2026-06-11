use std::collections::HashMap;

use furnace_core::{MacdPreviousState, MacdState};

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{MacdRunRequest, MacdWriteMode};
use crate::runners::shared::normalize_symbols;
use crate::sql::{
    first_tsv_value, parse_f64, parse_single_column_strings, parse_u64, sql_string,
    symbol_where_clause, symbol_where_clause_for,
};

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
SELECT security_code, toString(trade_date), ema_fast_state_12, ema_slow_state_26, macd_dea_state
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
WHERE rn = 1
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    let mut states = HashMap::new();
    for line in executor
        .query(&sql)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 5 previous MACD state fields, got {}",
                fields.len()
            )));
        }
        let fast = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous ema_fast_state_12 must not be null".to_string())
        })?;
        let slow = parse_f64(fields[3])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous ema_slow_state_26 must not be null".to_string())
        })?;
        let dea = parse_f64(fields[4])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous macd_dea_state must not be null".to_string())
        })?;
        states.insert(
            fields[0].to_string(),
            MacdPreviousState::new(
                fields[1].to_string(),
                MacdState::new(fast, slow, dea)
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
SELECT countDistinct(security_code)
FROM {}
WHERE trade_date < toDate('{}')
  AND (ema_fast_state_12 IS NOT NULL OR ema_slow_state_26 IS NOT NULL OR macd_dea_state IS NOT NULL)
  AND (ema_fast_state_12 IS NULL OR ema_slow_state_26 IS NULL OR macd_dea_state IS NULL)
  AND {}
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );
    parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())
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
    countDistinct(input.security_code),
    toString(min(input.trade_date))
FROM {} AS input
INNER JOIN states
    ON input.security_code = states.security_code
WHERE input.trade_date > states.state_date
  AND input.trade_date < toDate('{}')
  AND input.{} IS NOT NULL
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table,
        sql_string(&request.request_from),
        request.price_column
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
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
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

pub(super) fn resolve_macd_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
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

pub(super) fn read_macd_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
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

pub(super) fn read_macd_mixed_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MacdRunRequest,
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
      AND ema_fast_state_12 IS NOT NULL
      AND ema_slow_state_26 IS NOT NULL
      AND macd_dea_state IS NOT NULL
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
