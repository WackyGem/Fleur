use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::request::MaRunRequest;
use crate::rows::MaResultRow;
use crate::runners::shared::{count_year_rows, partition_year_fully_covered};
use crate::sql::{first_tsv_value, parse_u64, sql_string, symbol_where_clause};
pub(super) fn ensure_ma_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
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
pub(super) fn retain_old_ma_rows_for_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
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
pub(super) fn insert_ma_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[MaResultRow],
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
    price_ma_3,
    price_ma_5,
    price_ma_6,
    price_ma_10,
    price_ma_12,
    price_ma_14,
    price_ma_20,
    price_ma_24,
    price_ma_28,
    price_ma_57,
    price_ma_60,
    price_ma_114,
    price_ma_250,
    price_avg_ma_3_6_12_24,
    price_avg_ma_14_28_57_114,
    price_ema1_10_state,
    price_ema2_10,
    price_ema2_10_state,
    volume_ma_5,
    volume_ma_10,
    volume_ma_20,
    volume_ma_60
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
