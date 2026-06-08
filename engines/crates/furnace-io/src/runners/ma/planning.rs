use std::collections::HashMap;

use furnace_core::{DEFAULT_PRICE_MA_WINDOWS, DEFAULT_VOLUME_MA_WINDOWS, MaPreviousState, MaState};

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{MaRunRequest, MaWriteMode};
use crate::runners::shared::normalize_symbols;
use crate::sql::{
    first_tsv_value, parse_f64, parse_single_column_strings, parse_u64, sql_string,
    symbol_where_clause, symbol_where_clause_for_column,
};
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
SELECT security_code, toString(trade_date), price_ema1_10_state, price_ema2_10_state
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
        if fields.len() != 4 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 4 previous MA state fields, got {}",
                fields.len()
            )));
        }
        let ema1 = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous price_ema1_10_state must not be null".to_string())
        })?;
        let ema2 = parse_f64(fields[3])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous price_ema2_10_state must not be null".to_string())
        })?;
        states.insert(
            fields[0].to_string(),
            MaPreviousState::new(
                fields[1].to_string(),
                MaState::new(ema1, ema2)
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
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
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

pub(super) fn resolve_ma_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
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
SELECT toString(min(trade_date))
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
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        request.price_column,
        request.input_table,
        request.volume_input_table,
        sql_string(&request.request_from),
        request.volume_column
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
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
SELECT count()
FROM {}
WHERE trade_date < toDate('{}')
  AND {}
FORMAT TSV",
        request.input_table,
        sql_string(input_from),
        symbol_where_clause(symbols, false)
    );
    let count = parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())?;
    Ok(count > 0)
}
pub(super) fn read_ma_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
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
    adj.security_code,
    toString(adj.trade_date),
    adj.{},
    CAST(unadj.{}, 'Nullable(Float64)')
FROM {} AS adj
LEFT JOIN {} AS unadj
  ON adj.security_code = unadj.security_code
 AND adj.trade_date = unadj.trade_date
WHERE adj.trade_date >= toDate('{}')
  AND adj.trade_date <= toDate('{}')
  AND {}
ORDER BY adj.security_code, adj.trade_date
FORMAT RowBinary",
        request.price_column,
        request.volume_column,
        request.input_table,
        request.volume_input_table,
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause_for_column("adj.security_code", symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}
