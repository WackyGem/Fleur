use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::RsiRunRequest;
use crate::rows::RsiResultRow;
use crate::runners::shared::{count_year_rows, partition_year_fully_covered};
use crate::sql::{first_tsv_value, parse_u64, sql_string, symbol_where_clause};
pub(super) fn ensure_rsi_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<(), FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(());
    }
    let sql = format!(
        "\
SELECT count()
FROM {}
WHERE trade_date >= toDate('{}')
  AND {}
FORMAT TSV",
        request.output_table,
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
pub(super) fn retain_old_rsi_rows_for_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
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
FROM {}
WHERE toYear(trade_date) = {year}
  AND NOT (
      {}
      AND trade_date >= toDate('{}')
      AND trade_date <= toDate('{}')
  )",
            request.output_table,
            symbol_where_clause(symbols, all_symbols_requested),
            sql_string(&request.request_from),
            sql_string(effective_output_to)
        );
        executor.execute(&sql)?;
        retained += count_year_rows(executor, staging_table, *year)?;
    }
    Ok(retained)
}
pub(super) fn insert_rsi_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[RsiResultRow],
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
    rsi_6,
    rsi_12,
    rsi_14,
    rsi_24,
    rsi_25,
    rsi_50,
    avg_gain_6_state,
    avg_loss_6_state,
    avg_gain_12_state,
    avg_loss_12_state,
    avg_gain_14_state,
    avg_loss_14_state,
    avg_gain_24_state,
    avg_loss_24_state,
    avg_gain_25_state,
    avg_loss_25_state,
    avg_gain_50_state,
    avg_loss_50_state
)
FORMAT RowBinary"
    );
    for batch in rows.chunks(batch_size) {
        let mut row_binary = Vec::with_capacity(batch.len().saturating_mul(170));
        for row in batch {
            row.write_row_binary(&mut row_binary)?;
        }
        executor.insert_bytes(&insert_sql, &row_binary)?;
    }
    Ok(())
}
