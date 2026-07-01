use furnace_core::DEFAULT_BOLL_MAX_WINDOW;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::{BollRunRequest, BollWriteMode};
use crate::rows::{CloseInputRow, OptionalDateValueRow, SecurityCodeRow};
use crate::runners::shared::normalize_symbols;
use crate::sql::{sql_string, symbol_where_clause};
use crate::validation::format_clickhouse_date;
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
          AND {symbol_filter}
    )
    WHERE rn <= {max_window}
)",
        request.input_table,
        sql_string(&request.request_from),
        request.price_column
    );
    Ok(executor
        .fetch_one::<OptionalDateValueRow>(&sql)?
        .value
        .map(format_clickhouse_date)
        .unwrap_or_else(|| request.request_from.clone()))
}
pub(super) fn read_boll_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
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
