use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{PricePatternRunRequest, PricePatternWriteMode};
use crate::runners::shared::normalize_symbols;
use crate::sql::{first_tsv_value, parse_single_column_strings, sql_string, symbol_where_clause};

pub(super) fn resolve_price_pattern_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &PricePatternRunRequest,
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
        request.structure_input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
}

pub(super) fn resolve_price_pattern_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &PricePatternRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != PricePatternWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT toString(max(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.structure_input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
    Ok(value.max(request.request_to.clone()))
}

pub(super) fn resolve_price_pattern_full_history_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &PricePatternRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.structure_input_table,
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

pub(super) fn read_price_pattern_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &PricePatternRunRequest,
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
    adj.{high_column},
    adj.{low_column},
    unadj.{close_column},
    unadj.{prev_close_column}
FROM {structure_input_table} AS adj
LEFT JOIN {streak_input_table} AS unadj
  ON adj.security_code = unadj.security_code
 AND adj.trade_date = unadj.trade_date
WHERE adj.trade_date >= toDate('{input_from}')
  AND adj.trade_date <= toDate('{input_to}')
  AND {symbol_filter}
ORDER BY adj.security_code, adj.trade_date
FORMAT RowBinary",
        high_column = request.high_column,
        low_column = request.low_column,
        close_column = request.close_column,
        prev_close_column = request.prev_close_column,
        structure_input_table = request.structure_input_table,
        streak_input_table = request.streak_input_table,
        input_from = sql_string(input_from),
        input_to = sql_string(input_to),
        symbol_filter = symbol_where_clause(symbols, all_symbols_requested)
            .replace("security_code", "adj.security_code"),
    );

    executor.query_bytes(&sql)
}
