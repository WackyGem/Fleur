use super::tables::DEFAULT_KDJ_OUTPUT_TABLE;

/// 根据输出表和运行 ID 构造确定性的临时 staging 表名。
///
/// 非字母数字字符会被规范化为 `_`，确保结果可以安全地作为 ClickHouse 标识符后缀。
pub fn staging_table_name(output_table: &str, run_id: &str) -> String {
    let normalized = run_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    let suffix = if normalized.is_empty() {
        "manual".to_string()
    } else {
        normalized
    };
    format!("{output_table}__staging__{suffix}")
}

/// 根据运行 ID 构造 KDJ staging 表名。
pub fn kdj_staging_table_name(run_id: &str) -> String {
    staging_table_name(DEFAULT_KDJ_OUTPUT_TABLE, run_id)
}

/// 根据运行 ID 构造 MA staging 表名。
pub fn ma_staging_table_name(output_table: &str, run_id: &str) -> String {
    staging_table_name(output_table, run_id)
}

/// 根据运行 ID 构造 RSI staging 表名。
pub fn rsi_staging_table_name(output_table: &str, run_id: &str) -> String {
    staging_table_name(output_table, run_id)
}

/// 根据运行 ID 构造 Bollinger Bands staging 表名。
pub fn boll_staging_table_name(output_table: &str, run_id: &str) -> String {
    staging_table_name(output_table, run_id)
}

/// 构造 staging 表创建 SQL，表结构与目标表一致。
pub fn create_staging_table_sql(output_table: &str, staging_table: &str) -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {staging_table}
AS {output_table}
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// 构造 KDJ staging 表创建 SQL，表结构与目标表一致。
pub fn create_kdj_staging_table_sql(staging_table: &str) -> String {
    create_staging_table_sql(DEFAULT_KDJ_OUTPUT_TABLE, staging_table)
}

/// 构造 MA staging 表创建 SQL，表结构与目标表一致。
pub fn create_ma_staging_table_sql(output_table: &str, staging_table: &str) -> String {
    create_staging_table_sql(output_table, staging_table)
}

/// 构造 RSI staging 表创建 SQL，表结构与目标表一致。
pub fn create_rsi_staging_table_sql(output_table: &str, staging_table: &str) -> String {
    create_staging_table_sql(output_table, staging_table)
}

/// 构造 Bollinger Bands staging 表创建 SQL，表结构与目标表一致。
pub fn create_boll_staging_table_sql(output_table: &str, staging_table: &str) -> String {
    create_staging_table_sql(output_table, staging_table)
}

/// 构造删除临时 staging 表的 SQL。
pub fn drop_staging_table_sql(staging_table: &str) -> String {
    format!("DROP TABLE IF EXISTS {staging_table}")
}

/// 构造删除 KDJ 临时 staging 表的 SQL。
pub fn drop_kdj_staging_table_sql(staging_table: &str) -> String {
    drop_staging_table_sql(staging_table)
}

/// 构造删除 MA 临时 staging 表的 SQL。
pub fn drop_ma_staging_table_sql(staging_table: &str) -> String {
    drop_staging_table_sql(staging_table)
}

/// 构造删除 RSI 临时 staging 表的 SQL。
pub fn drop_rsi_staging_table_sql(staging_table: &str) -> String {
    drop_staging_table_sql(staging_table)
}

/// 构造删除 Bollinger Bands 临时 staging 表的 SQL。
pub fn drop_boll_staging_table_sql(staging_table: &str) -> String {
    drop_staging_table_sql(staging_table)
}
