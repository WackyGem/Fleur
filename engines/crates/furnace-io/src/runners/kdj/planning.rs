use std::collections::HashMap;

use furnace_core::KdjState;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{KdjRunRequest, KdjWriteMode};
use crate::runners::shared::normalize_symbols;
use crate::schema::{DEFAULT_INPUT_TABLE, DEFAULT_KDJ_OUTPUT_TABLE, DEFAULT_WARMUP_MULTIPLE};
use crate::sql::{
    first_tsv_value, parse_f64, parse_single_column_strings, sql_string, symbol_where_clause,
};
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
FORMAT TSV",
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
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
SELECT toString(max(trade_date))
FROM {DEFAULT_INPUT_TABLE}
WHERE {}
FORMAT TSV",
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
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
SELECT toString(min(trade_date))
FROM (
    SELECT trade_date
    FROM {DEFAULT_INPUT_TABLE}
    WHERE trade_date <= toDate('{}')
      AND {symbol_filter}
    GROUP BY trade_date
    ORDER BY trade_date DESC
    LIMIT {warmup_window}
)
FORMAT TSV",
        sql_string(&request.request_from)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
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
SELECT security_code, k_value, d_value
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
WHERE rn = 1
FORMAT TSV",
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
        if fields.len() != 3 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 3 previous-state fields, got {}",
                fields.len()
            )));
        }
        let k_value = parse_f64(fields[1])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous k_value must not be null".to_string())
        })?;
        let d_value = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous d_value must not be null".to_string())
        })?;
        states.insert(fields[0].to_string(), KdjState::new(k_value, d_value));
    }
    Ok(states)
}
pub(super) fn read_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
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
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj
FROM {DEFAULT_INPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date
FORMAT RowBinary",
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}
