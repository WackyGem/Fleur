/// 默认的 dbt 中间层输入表，存放前复权日行情价格。
pub const DEFAULT_INPUT_TABLE: &str = "fleur_intermediate.int_stock_quotes_daily_adj";

/// 默认的 dbt 中间层成交量输入表，存放未复权日行情成交量。
pub const DEFAULT_MA_VOLUME_INPUT_TABLE: &str = "fleur_intermediate.int_stock_quotes_daily_unadj";

/// Furnace 负责写入的日频 KDJ 计算结果表。
pub const DEFAULT_KDJ_OUTPUT_TABLE: &str = "fleur_calculation.calc_stock_kdj_daily";

/// Furnace 负责写入的日频 Moving Average 计算结果表。
pub const DEFAULT_MA_OUTPUT_TABLE: &str = "fleur_calculation.calc_stock_ma_daily";

/// Furnace 负责写入的日频 RSI 计算结果表。
pub const DEFAULT_RSI_OUTPUT_TABLE: &str = "fleur_calculation.calc_stock_rsi_daily";

/// Furnace 负责写入的日频 Bollinger Bands 计算结果表。
pub const DEFAULT_BOLL_OUTPUT_TABLE: &str = "fleur_calculation.calc_stock_boll_daily";

/// Moving Average 第一版使用的 canonical 前复权收盘价字段。
pub const DEFAULT_MA_PRICE_COLUMN: &str = "close_price_forward_adj";

/// Moving Average 第一版使用的 canonical 成交量字段。
pub const DEFAULT_MA_VOLUME_COLUMN: &str = "volume";

/// RSI 第一版使用的 canonical 前复权收盘价字段。
pub const DEFAULT_RSI_PRICE_COLUMN: &str = "close_price_forward_adj";

/// Bollinger Bands 第一版使用的 canonical 前复权收盘价字段。
pub const DEFAULT_BOLL_PRICE_COLUMN: &str = "close_price_forward_adj";

/// ClickHouse 单批插入的默认目标行数。
pub const DEFAULT_INSERT_BATCH_SIZE: usize = 10_000;

/// 生产写入模式允许的最小插入批次行数。
pub const MIN_INSERT_BATCH_SIZE: usize = 1_000;

/// 构造 KDJ 状态和 RSV 窗口时使用的默认预热倍数。
pub const DEFAULT_WARMUP_MULTIPLE: u16 = 3;

/// 返回生产 ClickHouse 数据库的创建 SQL。
pub fn create_calculation_database_sql() -> &'static str {
    "CREATE DATABASE IF NOT EXISTS fleur_calculation"
}

/// 返回 `calc_stock_kdj_daily` 生产表的 ClickHouse DDL。
///
/// # 示例
///
/// ```
/// let ddl = furnace_io::create_kdj_output_table_sql();
/// assert!(ddl.contains("fleur_calculation.calc_stock_kdj_daily"));
/// assert!(ddl.contains("PARTITION BY toYear(trade_date)"));
/// ```
pub fn create_kdj_output_table_sql() -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {DEFAULT_KDJ_OUTPUT_TABLE}
(
    security_code String,
    trade_date Date,
    rsv_window UInt16,
    k_smoothing UInt16,
    d_smoothing UInt16,
    rsv Nullable(Float64),
    k_value Nullable(Float64),
    d_value Nullable(Float64),
    j_value Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// 返回 Moving Average 结果表的 ClickHouse DDL。
pub fn create_ma_output_table_sql(output_table: &str) -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {output_table}
(
    security_code String,
    trade_date Date,
    price_ma_3 Nullable(Float64),
    price_ma_5 Nullable(Float64),
    price_ma_6 Nullable(Float64),
    price_ma_10 Nullable(Float64),
    price_ma_12 Nullable(Float64),
    price_ma_14 Nullable(Float64),
    price_ma_20 Nullable(Float64),
    price_ma_24 Nullable(Float64),
    price_ma_28 Nullable(Float64),
    price_ma_57 Nullable(Float64),
    price_ma_60 Nullable(Float64),
    price_ma_114 Nullable(Float64),
    price_ma_250 Nullable(Float64),
    price_avg_ma_3_6_12_24 Nullable(Float64),
    price_avg_ma_14_28_57_114 Nullable(Float64),
    price_ema1_10_state Nullable(Float64),
    price_ema2_10 Nullable(Float64),
    price_ema2_10_state Nullable(Float64),
    volume_ma_5 Nullable(Float64),
    volume_ma_10 Nullable(Float64),
    volume_ma_20 Nullable(Float64),
    volume_ma_60 Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// 返回 RSI 结果表的 ClickHouse DDL。
pub fn create_rsi_output_table_sql(output_table: &str) -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {output_table}
(
    security_code String,
    trade_date Date,
    rsi_6 Nullable(Float64),
    rsi_12 Nullable(Float64),
    rsi_14 Nullable(Float64),
    rsi_24 Nullable(Float64),
    rsi_25 Nullable(Float64),
    rsi_50 Nullable(Float64),
    avg_gain_6_state Nullable(Float64),
    avg_loss_6_state Nullable(Float64),
    avg_gain_12_state Nullable(Float64),
    avg_loss_12_state Nullable(Float64),
    avg_gain_14_state Nullable(Float64),
    avg_loss_14_state Nullable(Float64),
    avg_gain_24_state Nullable(Float64),
    avg_loss_24_state Nullable(Float64),
    avg_gain_25_state Nullable(Float64),
    avg_loss_25_state Nullable(Float64),
    avg_gain_50_state Nullable(Float64),
    avg_loss_50_state Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// 返回 Bollinger Bands 结果表的 ClickHouse DDL。
pub fn create_boll_output_table_sql(output_table: &str) -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {output_table}
(
    security_code String,
    trade_date Date,
    boll_mid_10_1p5 Nullable(Float64),
    boll_up_10_1p5 Nullable(Float64),
    boll_dn_10_1p5 Nullable(Float64),
    boll_mid_20_2 Nullable(Float64),
    boll_up_20_2 Nullable(Float64),
    boll_dn_20_2 Nullable(Float64),
    boll_mid_50_2p5 Nullable(Float64),
    boll_up_50_2p5 Nullable(Float64),
    boll_dn_50_2p5 Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// 根据运行 ID 构造确定性的临时 staging 表名。
///
/// 非字母数字字符会被规范化为 `_`，确保结果可以安全地作为 ClickHouse 标识符后缀。
pub fn kdj_staging_table_name(run_id: &str) -> String {
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
    format!("fleur_calculation.calc_stock_kdj_daily__staging__{suffix}")
}

/// 构造 staging 表创建 SQL，表结构与生产表一致。
pub fn create_kdj_staging_table_sql(staging_table: &str) -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {staging_table}
AS {DEFAULT_KDJ_OUTPUT_TABLE}
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// 构造将单个年份分区从 staging 表替换到生产表的 SQL。
pub fn replace_kdj_partition_sql(staging_table: &str, year: u16) -> String {
    format!("ALTER TABLE {DEFAULT_KDJ_OUTPUT_TABLE} REPLACE PARTITION {year} FROM {staging_table}")
}

/// 构造删除临时 staging 表的 SQL。
pub fn drop_kdj_staging_table_sql(staging_table: &str) -> String {
    format!("DROP TABLE IF EXISTS {staging_table}")
}

/// 根据运行 ID 构造 MA staging 表名。
pub fn ma_staging_table_name(output_table: &str, run_id: &str) -> String {
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

/// 构造 MA staging 表创建 SQL，表结构与目标表一致。
pub fn create_ma_staging_table_sql(output_table: &str, staging_table: &str) -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {staging_table}
AS {output_table}
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// 构造将单个年份分区从 MA staging 表替换到目标表的 SQL。
pub fn replace_ma_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    format!("ALTER TABLE {output_table} REPLACE PARTITION {year} FROM {staging_table}")
}

/// 构造删除 MA staging 表的 SQL。
pub fn drop_ma_staging_table_sql(staging_table: &str) -> String {
    format!("DROP TABLE IF EXISTS {staging_table}")
}

/// 根据运行 ID 构造 RSI staging 表名。
pub fn rsi_staging_table_name(output_table: &str, run_id: &str) -> String {
    ma_staging_table_name(output_table, run_id)
}

/// 构造 RSI staging 表创建 SQL，表结构与目标表一致。
pub fn create_rsi_staging_table_sql(output_table: &str, staging_table: &str) -> String {
    create_ma_staging_table_sql(output_table, staging_table)
}

/// 构造将单个年份分区从 RSI staging 表替换到目标表的 SQL。
pub fn replace_rsi_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    replace_ma_partition_sql(output_table, staging_table, year)
}

/// 构造删除 RSI staging 表的 SQL。
pub fn drop_rsi_staging_table_sql(staging_table: &str) -> String {
    drop_ma_staging_table_sql(staging_table)
}

/// 根据运行 ID 构造 Bollinger Bands staging 表名。
pub fn boll_staging_table_name(output_table: &str, run_id: &str) -> String {
    ma_staging_table_name(output_table, run_id)
}

/// 构造 Bollinger Bands staging 表创建 SQL，表结构与目标表一致。
pub fn create_boll_staging_table_sql(output_table: &str, staging_table: &str) -> String {
    create_ma_staging_table_sql(output_table, staging_table)
}

/// 构造将单个年份分区从 Bollinger Bands staging 表替换到目标表的 SQL。
pub fn replace_boll_partition_sql(output_table: &str, staging_table: &str, year: u16) -> String {
    replace_ma_partition_sql(output_table, staging_table, year)
}

/// 构造删除 Bollinger Bands 临时 staging 表的 SQL。
pub fn drop_boll_staging_table_sql(staging_table: &str) -> String {
    drop_ma_staging_table_sql(staging_table)
}
