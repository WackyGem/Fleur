use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::KdjRunRequest;
use crate::rows::KdjResultRow;
use crate::runners::shared::{count_year_rows, partition_year_fully_covered};
use crate::schema::DEFAULT_KDJ_OUTPUT_TABLE;
use crate::sql::{first_tsv_value, parse_u64, sql_string, symbol_where_clause};
pub(super) fn ensure_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<(), FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(());
    }
    let sql = format!(
        "\
SELECT count()
FROM {DEFAULT_KDJ_OUTPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND {}
FORMAT TSV",
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let existing_rows = parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())?;
    if existing_rows > 0 {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "append-latest found {existing_rows} existing same-or-later result rows; use replace-cascade"
        )));
    }
    Ok(())
}
pub(in crate::runners) fn retain_old_rows_for_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    staging_table: &str,
    symbols: &[String],
    all_symbols_requested: bool,
    years: &[u16],
    effective_output_to: &str,
) -> Result<u64, FurnaceIoError> {
    let mut retained = 0;
    for year in years {
        if all_symbols_requested
            && partition_year_fully_covered(*year, &request.request_from, effective_output_to)
        {
            continue;
        }
        let sql = format!(
            "\
INSERT INTO {staging_table}
SELECT *
FROM {DEFAULT_KDJ_OUTPUT_TABLE}
WHERE toYear(trade_date) = {year}
  AND NOT (
      {}
      AND trade_date >= toDate('{}')
      AND trade_date <= toDate('{}')
  )",
            symbol_where_clause(symbols, all_symbols_requested),
            sql_string(&request.request_from),
            sql_string(effective_output_to)
        );
        executor.execute(&sql)?;
        retained += count_year_rows(executor, staging_table, *year)?;
    }
    Ok(retained)
}
pub(super) fn insert_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[KdjResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    if rows.is_empty() {
        return Ok(());
    }
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    rsv_window,
    k_smoothing,
    d_smoothing,
    rsv,
    k_value,
    d_value,
    j_value
)
FORMAT RowBinary"
    );
    for batch in rows.chunks(batch_size) {
        let mut row_binary = Vec::with_capacity(batch.len().saturating_mul(80));
        for row in batch {
            row.write_row_binary(&mut row_binary)?;
        }
        executor.insert_bytes(&insert_sql, &row_binary)?;
    }
    Ok(())
}
