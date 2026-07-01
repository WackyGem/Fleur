use std::collections::BTreeSet;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::rows::CountRow;
use crate::schema::DEFAULT_KDJ_OUTPUT_TABLE;
use crate::sql::sql_string;
use crate::validation::validate_table_name;
pub(in crate::runners) fn normalize_symbols(symbols: &[String]) -> Vec<String> {
    let mut unique = BTreeSet::new();
    for symbol in symbols {
        let symbol = symbol.trim();
        if !symbol.is_empty() {
            unique.insert(symbol.to_string());
        }
    }
    unique.into_iter().collect()
}
pub(in crate::runners) fn target_table_exists<E: ClickHouseExecutor>(
    executor: &mut E,
) -> Result<bool, FurnaceIoError> {
    table_exists(executor, DEFAULT_KDJ_OUTPUT_TABLE)
}

pub(in crate::runners) fn table_exists<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
) -> Result<bool, FurnaceIoError> {
    let (database, name) = split_table_name(table)?;
    let sql = format!(
        "\
SELECT count() AS value
FROM system.tables
WHERE database = '{}'
  AND name = '{}'",
        sql_string(database),
        sql_string(name)
    );
    Ok(executor.fetch_one::<CountRow>(&sql)?.value > 0)
}

fn split_table_name(table: &str) -> Result<(&str, &str), FurnaceIoError> {
    validate_table_name("table", table)?;
    let Some((database, name)) = table.split_once('.') else {
        return Err(FurnaceIoError::InvalidRequest(
            "table must use database.table format".to_string(),
        ));
    };
    Ok((database, name))
}
