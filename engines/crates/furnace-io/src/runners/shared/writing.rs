use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::sql::{first_tsv_value, parse_u64};
use crate::summary::ValidationSummary;
pub(in crate::runners) fn partition_year_fully_covered(year: u16, from: &str, to: &str) -> bool {
    let year_start = format!("{year}-01-01");
    let year_end = format!("{year}-12-31");
    from <= year_start.as_str() && to >= year_end.as_str()
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
