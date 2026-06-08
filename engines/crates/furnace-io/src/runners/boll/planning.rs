use furnace_core::DEFAULT_BOLL_MAX_WINDOW;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{BollRunRequest, BollWriteMode};
use crate::runners::shared::normalize_symbols;
use crate::sql::{first_tsv_value, parse_single_column_strings, sql_string, symbol_where_clause};
pub(super) fn resolve_boll_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
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

pub(super) fn resolve_boll_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != BollWriteMode::ReplaceCascade {
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

pub(super) fn resolve_boll_lookback_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let symbol_filter = symbol_where_clause(symbols, all_symbols_requested);
    let max_window = request.params.max_window().max(DEFAULT_BOLL_MAX_WINDOW);
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
          AND {symbol_filter}
    )
    WHERE rn <= {max_window}
)
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        request.price_column
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}
pub(super) fn read_boll_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
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
