use super::tables::DEFAULT_KDJ_OUTPUT_TABLE;

/// 构造将单个年份分区从 staging 表替换到目标表的 SQL。
pub fn replace_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    format!("ALTER TABLE {output_table} REPLACE PARTITION {year} FROM {staging_table}")
}

/// 构造将单个年份分区从 KDJ staging 表替换到目标表的 SQL。
pub fn replace_kdj_partition_sql(staging_table: &str, year: u16) -> String {
    replace_partition_sql(DEFAULT_KDJ_OUTPUT_TABLE, staging_table, year)
}

/// 构造将单个年份分区从 MA staging 表替换到目标表的 SQL。
pub fn replace_ma_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    replace_partition_sql(output_table, staging_table, year)
}

/// 构造将单个年份分区从 RSI staging 表替换到目标表的 SQL。
pub fn replace_rsi_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    replace_partition_sql(output_table, staging_table, year)
}

/// 构造将单个年份分区从 Bollinger Bands staging 表替换到目标表的 SQL。
pub fn replace_boll_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    replace_partition_sql(output_table, staging_table, year)
}

/// 构造将单个年份分区从 MACD staging 表替换到目标表的 SQL。
pub fn replace_macd_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    replace_partition_sql(output_table, staging_table, year)
}

/// 构造将单个年份分区从 Price Pattern staging 表替换到目标表的 SQL。
pub fn replace_price_pattern_partition_sql(
    output_table: &str,
    staging_table: &str,
    year: u16,
) -> String {
    replace_partition_sql(output_table, staging_table, year)
}
