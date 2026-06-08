use std::collections::BTreeSet;

use crate::FurnaceIoError;
use crate::clickhouse::ClickHouseExecutor;
use crate::schema::DEFAULT_KDJ_OUTPUT_TABLE;
use crate::sql::first_tsv_value;
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
    let value = first_tsv_value(&executor.query(&format!(
        "EXISTS TABLE {DEFAULT_KDJ_OUTPUT_TABLE} FORMAT TSV"
    ))?)
    .unwrap_or_else(|| "0".to_string());
    Ok(value == "1")
}

pub(in crate::runners) fn table_exists<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
) -> Result<bool, FurnaceIoError> {
    let value = first_tsv_value(&executor.query(&format!("EXISTS TABLE {table} FORMAT TSV"))?)
        .unwrap_or_else(|| "0".to_string());
    Ok(value == "1")
}
