use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::schema::replace_partition_sql;
use crate::sql::{first_tsv_value, parse_u64};
use crate::sql::{sql_string, symbol_where_clause};
use crate::summary::ValidationSummary;

pub(in crate::runners) fn partition_year_fully_covered(year: u16, from: &str, to: &str) -> bool {
    let year_start = format!("{year}-01-01");
    let year_end = format!("{year}-12-31");
    from <= year_start.as_str() && to >= year_end.as_str()
}

pub(in crate::runners) fn ensure_output_schema<E: ClickHouseExecutor>(
    executor: &mut E,
    output_table_sql: &str,
) -> Result<(), FurnaceIoError> {
    executor.execute(crate::schema::create_calculation_database_sql())?;
    executor.execute(output_table_sql)
}

pub(in crate::runners) fn ensure_production_symbols(
    indicator: &str,
    writes_applied: bool,
    symbols: &[String],
) -> Result<(), FurnaceIoError> {
    if writes_applied && symbols.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "production {indicator} writes require at least one input security"
        )));
    }
    Ok(())
}

pub(in crate::runners) fn ensure_production_output_rows(
    indicator: &str,
    writes_applied: bool,
    output_is_empty: bool,
) -> Result<(), FurnaceIoError> {
    if writes_applied && output_is_empty {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "production {indicator} writes produced no output rows"
        )));
    }
    Ok(())
}

pub(in crate::runners) fn ensure_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    output_table: &str,
    request_from: &str,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<(), FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(());
    }
    let sql = format!(
        "\
SELECT count()
FROM {output_table}
WHERE trade_date >= toDate('{}')
  AND {}
FORMAT TSV",
        sql_string(request_from),
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

pub(in crate::runners) struct RetainStagingRows<'a> {
    pub(in crate::runners) output_table: &'a str,
    pub(in crate::runners) staging_table: &'a str,
    pub(in crate::runners) request_from: &'a str,
    pub(in crate::runners) symbols: &'a [String],
    pub(in crate::runners) all_symbols_requested: bool,
    pub(in crate::runners) years: &'a [u16],
    pub(in crate::runners) effective_output_to: &'a str,
}

pub(in crate::runners) fn retain_existing_rows_for_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    plan: &RetainStagingRows<'_>,
) -> Result<u64, FurnaceIoError> {
    let mut retained = 0;
    for year in plan.years {
        if plan.all_symbols_requested
            && partition_year_fully_covered(*year, plan.request_from, plan.effective_output_to)
        {
            continue;
        }
        let sql = format!(
            "\
INSERT INTO {staging_table}
SELECT *
FROM {output_table}
WHERE toYear(trade_date) = {year}
  AND NOT (
      {where_clause}
      AND trade_date >= toDate('{request_from}')
      AND trade_date <= toDate('{effective_output_to}')
  )",
            staging_table = plan.staging_table,
            output_table = plan.output_table,
            where_clause = symbol_where_clause(plan.symbols, plan.all_symbols_requested),
            request_from = sql_string(plan.request_from),
            effective_output_to = sql_string(plan.effective_output_to),
        );
        executor.execute(&sql)?;
        retained += count_year_rows(executor, plan.staging_table, *year)?;
    }
    Ok(retained)
}

pub(in crate::runners) fn setup_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    drop_sql: String,
    create_sql: String,
) -> Result<(), FurnaceIoError> {
    executor.execute_many(&[drop_sql, create_sql])
}

pub(in crate::runners) fn validate_staging_or_error<E: ClickHouseExecutor>(
    executor: &mut E,
    staging_table: &str,
    years: &[u16],
) -> Result<ValidationSummary, FurnaceIoError> {
    let validation = validate_staging(executor, staging_table, years)?;
    if validation.status != "passed" {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "staging validation failed with {} duplicate keys",
            validation.duplicate_keys
        )));
    }
    Ok(validation)
}

pub(in crate::runners) fn replace_partitions<E: ClickHouseExecutor>(
    executor: &mut E,
    output_table: &str,
    staging_table: &str,
    years: &[u16],
) -> Result<(), FurnaceIoError> {
    let replace_sql = years
        .iter()
        .map(|year| replace_partition_sql(output_table, staging_table, *year))
        .collect::<Vec<_>>();
    executor.execute_many(&replace_sql)
}

pub(in crate::runners) fn cleanup_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    drop_sql: &str,
) -> Result<(), FurnaceIoError> {
    executor.execute(drop_sql)
}

pub(in crate::runners) fn insert_rowbinary_rows<E, R>(
    executor: &mut E,
    insert_sql: &str,
    rows: &[R],
    batch_size: usize,
    estimated_row_bytes: usize,
    mut write_row: impl FnMut(&R, &mut Vec<u8>) -> Result<(), FurnaceIoError>,
) -> Result<(), FurnaceIoError>
where
    E: ClickHouseExecutor,
{
    if rows.is_empty() {
        return Ok(());
    }
    for batch in rows.chunks(batch_size) {
        let mut row_binary = Vec::with_capacity(batch.len().saturating_mul(estimated_row_bytes));
        for row in batch {
            write_row(row, &mut row_binary)?;
        }
        executor.insert_bytes(insert_sql, &row_binary)?;
    }
    Ok(())
}

pub(in crate::runners) fn validate_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    staging_table: &str,
    years: &[u16],
) -> Result<ValidationSummary, FurnaceIoError> {
    if years.is_empty() {
        return Ok(ValidationSummary::passed());
    }
    let years = years
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "\
SELECT sum(duplicates)
FROM (
    SELECT count() - uniqExact(security_code, trade_date) AS duplicates
    FROM {staging_table}
    WHERE toYear(trade_date) IN ({years})
    GROUP BY toYear(trade_date)
)
FORMAT TSV"
    );
    let duplicates = parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())?;
    if duplicates > 0 {
        return Ok(ValidationSummary {
            status: "failed".to_string(),
            duplicate_keys: duplicates,
        });
    }
    Ok(ValidationSummary::passed())
}

pub(in crate::runners) fn count_year_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    year: u16,
) -> Result<u64, FurnaceIoError> {
    let sql = format!(
        "\
SELECT count()
FROM {table}
WHERE toYear(trade_date) = {year}
FORMAT TSV"
    );
    parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())
}
