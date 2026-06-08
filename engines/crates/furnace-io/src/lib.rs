//! Furnace 的 ClickHouse I/O 边界。
//!
//! 本 crate 负责面向数据库的表名、DDL、SQL 生成、`clickhouse-client` 执行以及运行摘要。
//! 纯指标公式保留在 `furnace-core` 中。

use std::collections::{BTreeSet, HashMap};
use std::env;
use std::error::Error;
use std::fmt;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};
use std::str;
use std::time::{Duration, Instant};

use furnace_core::{
    BollInput, BollParams, DEFAULT_BOLL_CONFIGS, DEFAULT_BOLL_MAX_WINDOW, DEFAULT_BOLL_STDDEV_DDOF,
    DEFAULT_EMA_WINDOW, DEFAULT_PRICE_MA_WINDOWS, DEFAULT_RSI_WINDOWS, DEFAULT_VOLUME_MA_WINDOWS,
    KdjInput, KdjParams, KdjState, MaInput, MaParams, MaPreviousState, MaState, RsiInput,
    RsiParams, RsiPreviousState, RsiState, RsiWindowState, calculate_boll_series,
    calculate_kdj_series, calculate_ma_series_from_previous_state,
    calculate_rsi_series_from_previous_state,
};
use rayon::prelude::*;

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

/// CLI 或 Dagster 请求的 KDJ 写入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdjWriteMode {
    /// 只计算并汇总结果，不写入 ClickHouse。
    DryRun,
    /// 当目标表不存在同日或更晚结果时，追加最新区间。
    AppendLatest,
    /// 重算历史区间，并级联到受影响的最新输入日期。
    ReplaceCascade,
}

impl KdjWriteMode {
    /// 解析该模式在 CLI 中使用的拼写。
    ///
    /// # 错误
    ///
    /// 当 `value` 不是 `dry-run`、`append-latest` 或 `replace-cascade` 时，
    /// 返回 [`FurnaceIoError::InvalidRequest`]。
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            other => Err(FurnaceIoError::InvalidRequest(format!(
                "invalid KDJ write mode: {other}"
            ))),
        }
    }

    /// 返回该模式在 CLI 中使用的拼写。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::AppendLatest => "append-latest",
            Self::ReplaceCascade => "replace-cascade",
        }
    }

    /// 判断该模式是否会写入生产 ClickHouse 数据。
    pub fn writes_applied(self) -> bool {
        !matches!(self, Self::DryRun)
    }
}

/// CLI 或 Dagster 请求的 Moving Average 写入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaWriteMode {
    /// 只计算并汇总结果，不写入 ClickHouse。
    DryRun,
    /// 当目标表不存在同日或更晚结果时，追加最新区间。
    AppendLatest,
    /// 重算历史区间，并级联到受影响的最新输入日期。
    ReplaceCascade,
}

impl MaWriteMode {
    /// 解析该模式在 CLI 中使用的拼写。
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            other => Err(FurnaceIoError::InvalidRequest(format!(
                "invalid MA write mode: {other}"
            ))),
        }
    }

    /// 返回该模式在 CLI 中使用的拼写。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::AppendLatest => "append-latest",
            Self::ReplaceCascade => "replace-cascade",
        }
    }

    /// 判断该模式是否会写入生产 ClickHouse 数据。
    pub fn writes_applied(self) -> bool {
        !matches!(self, Self::DryRun)
    }
}

/// CLI 或 Dagster 请求的 RSI 写入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsiWriteMode {
    /// 只计算并汇总结果，不写入 ClickHouse。
    DryRun,
    /// 当目标表不存在同日或更晚结果时，追加最新区间。
    AppendLatest,
    /// 重算历史区间，并级联到受影响的最新输入日期。
    ReplaceCascade,
}

impl RsiWriteMode {
    /// 解析该模式在 CLI 中使用的拼写。
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            other => Err(FurnaceIoError::InvalidRequest(format!(
                "invalid RSI write mode: {other}"
            ))),
        }
    }

    /// 返回该模式在 CLI 中使用的拼写。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::AppendLatest => "append-latest",
            Self::ReplaceCascade => "replace-cascade",
        }
    }

    /// 判断该模式是否会写入生产 ClickHouse 数据。
    pub fn writes_applied(self) -> bool {
        !matches!(self, Self::DryRun)
    }
}

/// CLI 或 Dagster 请求的 Bollinger Bands 写入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BollWriteMode {
    /// 只计算并汇总结果，不写入 ClickHouse。
    DryRun,
    /// 当目标表不存在同日或更晚结果时，追加最新区间。
    AppendLatest,
    /// 重算历史区间，并级联到受影响的最新输入日期。
    ReplaceCascade,
}

impl BollWriteMode {
    /// 解析该模式在 CLI 中使用的拼写。
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            other => Err(FurnaceIoError::InvalidRequest(format!(
                "invalid Bollinger Bands write mode: {other}"
            ))),
        }
    }

    /// 返回该模式在 CLI 中使用的拼写。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::AppendLatest => "append-latest",
            Self::ReplaceCascade => "replace-cascade",
        }
    }

    /// 判断该模式是否会写入生产 ClickHouse 数据。
    pub fn writes_applied(self) -> bool {
        !matches!(self, Self::DryRun)
    }
}

/// 单次 Furnace KDJ 运行请求。
#[derive(Debug, Clone, PartialEq)]
pub struct KdjRunRequest {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 可选证券代码白名单；为空时从输入行中推断。
    pub symbols: Vec<String>,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 写入模式。
    pub mode: KdjWriteMode,
    /// KDJ 参数。
    pub params: KdjParams,
    /// ClickHouse 每批插入的目标行数。
    pub insert_batch_size: usize,
}

impl KdjRunRequest {
    /// 在执行任何 ClickHouse 操作前校验请求。
    ///
    /// # 错误
    ///
    /// 当日期、参数或批次大小设置无法安全使用时，返回 [`FurnaceIoError::InvalidRequest`]。
    pub fn validate(&self) -> Result<(), FurnaceIoError> {
        validate_date("request_from", &self.request_from)?;
        validate_date("request_to", &self.request_to)?;
        if self.request_to < self.request_from {
            return Err(FurnaceIoError::InvalidRequest(
                "request_to must be greater than or equal to request_from".to_string(),
            ));
        }
        if self.mode.writes_applied() && !self.params.is_canonical() {
            return Err(FurnaceIoError::InvalidRequest(
                "production KDJ writes only allow canonical parameters 9/3/3".to_string(),
            ));
        }
        if self.mode.writes_applied() && self.insert_batch_size < MIN_INSERT_BATCH_SIZE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production insert batch size must be at least {MIN_INSERT_BATCH_SIZE}"
            )));
        }
        Ok(())
    }
}

impl Default for KdjRunRequest {
    fn default() -> Self {
        Self {
            request_from: String::new(),
            request_to: String::new(),
            symbols: Vec::new(),
            run_id: None,
            mode: KdjWriteMode::DryRun,
            params: KdjParams::default(),
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
        }
    }
}

/// 单次 Furnace Moving Average 运行请求。
#[derive(Debug, Clone, PartialEq)]
pub struct MaRunRequest {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 可选证券代码白名单；为空时从输入行中推断。
    pub symbols: Vec<String>,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 写入模式。
    pub mode: MaWriteMode,
    /// Moving Average 参数。
    pub params: MaParams,
    /// 输入表。
    pub input_table: String,
    /// 成交量输入表。
    pub volume_input_table: String,
    /// 输出表。
    pub output_table: String,
    /// close 输入字段名。
    pub price_column: String,
    /// volume 输入字段名。
    pub volume_column: String,
    /// ClickHouse 每批插入的目标行数。
    pub insert_batch_size: usize,
}

impl MaRunRequest {
    /// 在执行 ClickHouse 操作前校验请求。
    pub fn validate(&self) -> Result<(), FurnaceIoError> {
        validate_date("request_from", &self.request_from)?;
        validate_date("request_to", &self.request_to)?;
        if self.request_to < self.request_from {
            return Err(FurnaceIoError::InvalidRequest(
                "request_to must be greater than or equal to request_from".to_string(),
            ));
        }
        if self.mode.writes_applied() && !self.params.is_canonical() {
            return Err(FurnaceIoError::InvalidRequest(
                "production MA writes only allow canonical parameters".to_string(),
            ));
        }
        if self.mode.writes_applied() && self.input_table != DEFAULT_INPUT_TABLE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MA writes only allow input table {DEFAULT_INPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied() && self.volume_input_table != DEFAULT_MA_VOLUME_INPUT_TABLE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MA writes only allow volume input table {DEFAULT_MA_VOLUME_INPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied() && self.price_column != DEFAULT_MA_PRICE_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MA writes only allow price column {DEFAULT_MA_PRICE_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.volume_column != DEFAULT_MA_VOLUME_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MA writes only allow volume column {DEFAULT_MA_VOLUME_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.insert_batch_size < MIN_INSERT_BATCH_SIZE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production insert batch size must be at least {MIN_INSERT_BATCH_SIZE}"
            )));
        }
        validate_table_name("input_table", &self.input_table)?;
        validate_table_name("volume_input_table", &self.volume_input_table)?;
        validate_table_name("output_table", &self.output_table)?;
        validate_identifier("price_column", &self.price_column)?;
        validate_identifier("volume_column", &self.volume_column)?;
        Ok(())
    }
}

impl Default for MaRunRequest {
    fn default() -> Self {
        Self {
            request_from: String::new(),
            request_to: String::new(),
            symbols: Vec::new(),
            run_id: None,
            mode: MaWriteMode::DryRun,
            params: MaParams::default(),
            input_table: DEFAULT_INPUT_TABLE.to_string(),
            volume_input_table: DEFAULT_MA_VOLUME_INPUT_TABLE.to_string(),
            output_table: DEFAULT_MA_OUTPUT_TABLE.to_string(),
            price_column: DEFAULT_MA_PRICE_COLUMN.to_string(),
            volume_column: DEFAULT_MA_VOLUME_COLUMN.to_string(),
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
        }
    }
}

/// 单次 Furnace RSI 运行请求。
#[derive(Debug, Clone, PartialEq)]
pub struct RsiRunRequest {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 可选证券代码白名单；为空时从输入行中推断。
    pub symbols: Vec<String>,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 写入模式。
    pub mode: RsiWriteMode,
    /// RSI 参数。
    pub params: RsiParams,
    /// 输入表。
    pub input_table: String,
    /// 输出表。
    pub output_table: String,
    /// close 输入字段名。
    pub price_column: String,
    /// ClickHouse 每批插入的目标行数。
    pub insert_batch_size: usize,
}

impl RsiRunRequest {
    /// 在执行 ClickHouse 操作前校验请求。
    pub fn validate(&self) -> Result<(), FurnaceIoError> {
        validate_date("request_from", &self.request_from)?;
        validate_date("request_to", &self.request_to)?;
        if self.request_to < self.request_from {
            return Err(FurnaceIoError::InvalidRequest(
                "request_to must be greater than or equal to request_from".to_string(),
            ));
        }
        if self.mode.writes_applied() && !self.params.is_canonical() {
            return Err(FurnaceIoError::InvalidRequest(
                "production RSI writes only allow canonical parameters".to_string(),
            ));
        }
        if self.mode.writes_applied() && self.input_table != DEFAULT_INPUT_TABLE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production RSI writes only allow input table {DEFAULT_INPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied() && self.price_column != DEFAULT_RSI_PRICE_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production RSI writes only allow price column {DEFAULT_RSI_PRICE_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.insert_batch_size < MIN_INSERT_BATCH_SIZE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production insert batch size must be at least {MIN_INSERT_BATCH_SIZE}"
            )));
        }
        validate_table_name("input_table", &self.input_table)?;
        validate_table_name("output_table", &self.output_table)?;
        validate_identifier("price_column", &self.price_column)?;
        Ok(())
    }
}

impl Default for RsiRunRequest {
    fn default() -> Self {
        Self {
            request_from: String::new(),
            request_to: String::new(),
            symbols: Vec::new(),
            run_id: None,
            mode: RsiWriteMode::DryRun,
            params: RsiParams::default(),
            input_table: DEFAULT_INPUT_TABLE.to_string(),
            output_table: DEFAULT_RSI_OUTPUT_TABLE.to_string(),
            price_column: DEFAULT_RSI_PRICE_COLUMN.to_string(),
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
        }
    }
}

/// 单次 Furnace Bollinger Bands 运行请求。
#[derive(Debug, Clone, PartialEq)]
pub struct BollRunRequest {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 可选证券代码白名单；为空时从输入行中推断。
    pub symbols: Vec<String>,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 写入模式。
    pub mode: BollWriteMode,
    /// Bollinger Bands 参数。
    pub params: BollParams,
    /// 输入表。
    pub input_table: String,
    /// 输出表。
    pub output_table: String,
    /// close 输入字段名。
    pub price_column: String,
    /// ClickHouse 每批插入的目标行数。
    pub insert_batch_size: usize,
}

impl BollRunRequest {
    /// 在执行 ClickHouse 操作前校验请求。
    pub fn validate(&self) -> Result<(), FurnaceIoError> {
        validate_date("request_from", &self.request_from)?;
        validate_date("request_to", &self.request_to)?;
        if self.request_to < self.request_from {
            return Err(FurnaceIoError::InvalidRequest(
                "request_to must be greater than or equal to request_from".to_string(),
            ));
        }
        if self.mode.writes_applied() && !self.params.is_canonical() {
            return Err(FurnaceIoError::InvalidRequest(
                "production Bollinger Bands writes only allow canonical parameters".to_string(),
            ));
        }
        if self.mode.writes_applied() && self.input_table != DEFAULT_INPUT_TABLE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Bollinger Bands writes only allow input table {DEFAULT_INPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied() && self.price_column != DEFAULT_BOLL_PRICE_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Bollinger Bands writes only allow price column {DEFAULT_BOLL_PRICE_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.insert_batch_size < MIN_INSERT_BATCH_SIZE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production insert batch size must be at least {MIN_INSERT_BATCH_SIZE}"
            )));
        }
        validate_table_name("input_table", &self.input_table)?;
        validate_table_name("output_table", &self.output_table)?;
        validate_identifier("price_column", &self.price_column)?;
        Ok(())
    }
}

impl Default for BollRunRequest {
    fn default() -> Self {
        Self {
            request_from: String::new(),
            request_to: String::new(),
            symbols: Vec::new(),
            run_id: None,
            mode: BollWriteMode::DryRun,
            params: BollParams::default(),
            input_table: DEFAULT_INPUT_TABLE.to_string(),
            output_table: DEFAULT_BOLL_OUTPUT_TABLE.to_string(),
            price_column: DEFAULT_BOLL_PRICE_COLUMN.to_string(),
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
        }
    }
}

/// Furnace KDJ 单次运行输出的摘要。
#[derive(Debug, Clone, PartialEq)]
pub struct KdjRunSummary {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 实际写入输出的起始日期。
    pub effective_output_from: String,
    /// 实际写入输出的结束日期。
    pub effective_output_to: String,
    /// 实际读取输入的起始日期。
    pub input_from: String,
    /// 实际读取输入的结束日期。
    pub input_to: String,
    /// 写入模式。
    pub mode: KdjWriteMode,
    /// 本次运行选中的证券。
    pub symbols: Vec<String>,
    /// 输入行数。
    pub input_rows: u64,
    /// 输出行数。
    pub output_rows: u64,
    /// 所有指标值均不可用的输出行数。
    pub null_indicator_rows: u64,
    /// 受影响的 ClickHouse 年度分区。
    pub affected_years: Vec<u16>,
    /// staging 分区中保留的旧行数。
    pub retained_rows: u64,
    /// 本次运行使用的临时 staging 表；未使用时为空。
    pub staging_table: Option<String>,
    /// staging 表校验结果。
    pub staging_validation: ValidationSummary,
    /// 分区替换结果。
    pub partition_replace: PartitionReplaceSummary,
    /// KDJ 参数。
    pub params: KdjParams,
    /// 历史状态来源摘要。
    pub state_source: String,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 是否实际写入了生产数据。
    pub writes_applied: bool,
    /// 内部耗时和吞吐指标。
    pub performance_metrics: PerformanceMetrics,
}

impl KdjRunSummary {
    /// 将摘要序列化为 JSON，避免引入运行时序列化依赖。
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"kdj_params\":{{\"rsv_window\":{},\"k_smoothing\":{},\"d_smoothing\":{}}},\"state_source\":\"{}\",\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
            escape_json_string(&self.request_from),
            escape_json_string(&self.request_to),
            escape_json_string(&self.effective_output_from),
            escape_json_string(&self.effective_output_to),
            escape_json_string(&self.input_from),
            escape_json_string(&self.input_to),
            self.mode.as_str(),
            self.symbols.len(),
            self.input_rows,
            self.output_rows,
            self.null_indicator_rows,
            affected_years,
            self.retained_rows,
            json_optional_string(self.staging_table.as_deref()),
            self.staging_validation.to_json(),
            self.partition_replace.to_json(),
            self.params.rsv_window,
            self.params.k_smoothing,
            self.params.d_smoothing,
            escape_json_string(&self.state_source),
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied,
            self.performance_metrics.to_json()
        )
    }
}

/// Furnace Moving Average 单次运行输出摘要。
#[derive(Debug, Clone, PartialEq)]
pub struct MaRunSummary {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 实际写入输出的起始日期。
    pub effective_output_from: String,
    /// 实际写入输出的结束日期。
    pub effective_output_to: String,
    /// 实际读取输入的起始日期。
    pub input_from: String,
    /// 实际读取输入的结束日期。
    pub input_to: String,
    /// 写入模式。
    pub mode: MaWriteMode,
    /// 本次运行选中的证券。
    pub symbols: Vec<String>,
    /// 输入行数。
    pub input_rows: u64,
    /// 输出行数。
    pub output_rows: u64,
    /// 有效 close 行数。
    pub valid_close_rows: u64,
    /// 有效 volume 行数。
    pub valid_volume_rows: u64,
    /// 所有业务指标值均不可用的输出行数。
    pub null_indicator_rows: u64,
    /// 受影响的 ClickHouse 年度分区。
    pub affected_years: Vec<u16>,
    /// staging 分区中保留的旧行数。
    pub retained_rows: u64,
    /// 本次运行使用的临时 staging 表；未使用时为空。
    pub staging_table: Option<String>,
    /// staging 表校验结果。
    pub staging_validation: ValidationSummary,
    /// 分区替换结果。
    pub partition_replace: PartitionReplaceSummary,
    /// 历史 EMA 状态来源摘要。
    pub ema_state_source: String,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 是否实际写入了生产数据。
    pub writes_applied: bool,
    /// 内部耗时和吞吐指标。
    pub performance_metrics: PerformanceMetrics,
}

impl MaRunSummary {
    /// 将摘要序列化为 JSON。
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let price_ma_windows = DEFAULT_PRICE_MA_WINDOWS
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let volume_ma_windows = DEFAULT_VOLUME_MA_WINDOWS
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"indicator\":\"ma\",\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"valid_close_rows\":{},\"valid_volume_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"price_ma_windows\":[{}],\"volume_ma_windows\":[{}],\"ema_window\":{},\"ema_state_source\":\"{}\",\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
            escape_json_string(&self.request_from),
            escape_json_string(&self.request_to),
            escape_json_string(&self.effective_output_from),
            escape_json_string(&self.effective_output_to),
            escape_json_string(&self.input_from),
            escape_json_string(&self.input_to),
            self.mode.as_str(),
            self.symbols.len(),
            self.input_rows,
            self.output_rows,
            self.valid_close_rows,
            self.valid_volume_rows,
            self.null_indicator_rows,
            affected_years,
            self.retained_rows,
            json_optional_string(self.staging_table.as_deref()),
            self.staging_validation.to_json(),
            self.partition_replace.to_json(),
            price_ma_windows,
            volume_ma_windows,
            DEFAULT_EMA_WINDOW,
            escape_json_string(&self.ema_state_source),
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied,
            self.performance_metrics.to_json()
        )
    }
}

/// Furnace RSI 单次运行输出摘要。
#[derive(Debug, Clone, PartialEq)]
pub struct RsiRunSummary {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 实际写入输出的起始日期。
    pub effective_output_from: String,
    /// 实际写入输出的结束日期。
    pub effective_output_to: String,
    /// 实际读取输入的起始日期。
    pub input_from: String,
    /// 实际读取输入的结束日期。
    pub input_to: String,
    /// 写入模式。
    pub mode: RsiWriteMode,
    /// 本次运行选中的证券。
    pub symbols: Vec<String>,
    /// 输入行数。
    pub input_rows: u64,
    /// 输出行数。
    pub output_rows: u64,
    /// 有效 close 行数。
    pub valid_close_rows: u64,
    /// 所有业务指标值均不可用的输出行数。
    pub null_indicator_rows: u64,
    /// 受影响的 ClickHouse 年度分区。
    pub affected_years: Vec<u16>,
    /// staging 分区中保留的旧行数。
    pub retained_rows: u64,
    /// 本次运行使用的临时 staging 表；未使用时为空。
    pub staging_table: Option<String>,
    /// staging 表校验结果。
    pub staging_validation: ValidationSummary,
    /// 分区替换结果。
    pub partition_replace: PartitionReplaceSummary,
    /// 历史 RSI 状态来源摘要。
    pub rsi_state_source: String,
    /// 有结果缺口的证券数量。
    pub gap_symbols_count: u64,
    /// 建议补算起点。
    pub gap_fill_from: Option<String>,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 是否实际写入了生产数据。
    pub writes_applied: bool,
    /// 内部耗时和吞吐指标。
    pub performance_metrics: PerformanceMetrics,
}

impl RsiRunSummary {
    /// 将摘要序列化为 JSON。
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let rsi_windows = DEFAULT_RSI_WINDOWS
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"indicator\":\"rsi\",\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"valid_close_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"rsi_windows\":[{}],\"rsi_state_source\":\"{}\",\"gap_symbols_count\":{},\"gap_fill_from\":{},\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
            escape_json_string(&self.request_from),
            escape_json_string(&self.request_to),
            escape_json_string(&self.effective_output_from),
            escape_json_string(&self.effective_output_to),
            escape_json_string(&self.input_from),
            escape_json_string(&self.input_to),
            self.mode.as_str(),
            self.symbols.len(),
            self.input_rows,
            self.output_rows,
            self.valid_close_rows,
            self.null_indicator_rows,
            affected_years,
            self.retained_rows,
            json_optional_string(self.staging_table.as_deref()),
            self.staging_validation.to_json(),
            self.partition_replace.to_json(),
            rsi_windows,
            escape_json_string(&self.rsi_state_source),
            self.gap_symbols_count,
            json_optional_string(self.gap_fill_from.as_deref()),
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied,
            self.performance_metrics.to_json()
        )
    }
}

/// Furnace Bollinger Bands 单次运行输出摘要。
#[derive(Debug, Clone, PartialEq)]
pub struct BollRunSummary {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 实际写入输出的起始日期。
    pub effective_output_from: String,
    /// 实际写入输出的结束日期。
    pub effective_output_to: String,
    /// 实际读取输入的起始日期。
    pub input_from: String,
    /// 实际读取输入的结束日期。
    pub input_to: String,
    /// 写入模式。
    pub mode: BollWriteMode,
    /// 本次运行选中的证券。
    pub symbols: Vec<String>,
    /// 输入行数。
    pub input_rows: u64,
    /// 输出行数。
    pub output_rows: u64,
    /// 输入区间有效 close 行数。
    pub input_valid_close_rows: u64,
    /// 输出区间有效 close 行数。
    pub output_valid_close_rows: u64,
    /// 所有业务指标值均不可用的输出行数。
    pub null_indicator_rows: u64,
    /// 受影响的 ClickHouse 年度分区。
    pub affected_years: Vec<u16>,
    /// staging 分区中保留的旧行数。
    pub retained_rows: u64,
    /// 本次运行使用的临时 staging 表；未使用时为空。
    pub staging_table: Option<String>,
    /// staging 表校验结果。
    pub staging_validation: ValidationSummary,
    /// 分区替换结果。
    pub partition_replace: PartitionReplaceSummary,
    /// rolling 状态来源摘要。
    pub state_source: String,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 是否实际写入了生产数据。
    pub writes_applied: bool,
    /// 内部耗时和吞吐指标。
    pub performance_metrics: PerformanceMetrics,
}

impl BollRunSummary {
    /// 将摘要序列化为 JSON。
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"indicator\":\"boll\",\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"input_valid_close_rows\":{},\"output_valid_close_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"boll_configs\":{},\"max_window\":{},\"stddev_ddof\":{},\"state_source\":\"{}\",\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
            escape_json_string(&self.request_from),
            escape_json_string(&self.request_to),
            escape_json_string(&self.effective_output_from),
            escape_json_string(&self.effective_output_to),
            escape_json_string(&self.input_from),
            escape_json_string(&self.input_to),
            self.mode.as_str(),
            self.symbols.len(),
            self.input_rows,
            self.output_rows,
            self.input_valid_close_rows,
            self.output_valid_close_rows,
            self.null_indicator_rows,
            affected_years,
            self.retained_rows,
            json_optional_string(self.staging_table.as_deref()),
            self.staging_validation.to_json(),
            self.partition_replace.to_json(),
            boll_configs_json(),
            DEFAULT_BOLL_MAX_WINDOW,
            DEFAULT_BOLL_STDDEV_DDOF,
            escape_json_string(&self.state_source),
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied,
            self.performance_metrics.to_json()
        )
    }
}

/// Furnace KDJ 单次运行输出的性能指标。
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceMetrics {
    /// `run_kdj` 内部的端到端耗时。
    pub total_ms: u128,
    /// 读取价格输入行的耗时。
    pub read_input_ms: u128,
    /// 读取上一轮 K/D 状态的耗时。
    pub read_state_ms: u128,
    /// 按证券分组输入行的耗时。
    pub group_ms: u128,
    /// 计算 KDJ 输出的耗时。
    pub compute_ms: u128,
    /// 插入新 KDJ 行的耗时。
    pub write_ms: u128,
    /// staging DDL、旧行保留、校验和清理的总耗时。
    pub staging_ms: u128,
    /// 替换 ClickHouse 分区的耗时。
    pub partition_replace_ms: u128,
    /// 输入读取阶段的每秒读取行数。
    pub input_rows_per_sec: f64,
    /// 计算阶段的每秒输出行数。
    pub output_rows_per_sec: f64,
    /// 本次运行包含的证券数量。
    pub symbols_count: u64,
    /// Furnace 使用的计算策略。
    pub parallelism: String,
    /// 本次运行可用的 Rayon worker 线程数。
    pub worker_threads: usize,
}

impl PerformanceMetrics {
    fn to_json(&self) -> String {
        format!(
            "{{\"total_ms\":{},\"read_input_ms\":{},\"read_state_ms\":{},\"group_ms\":{},\"compute_ms\":{},\"write_ms\":{},\"staging_ms\":{},\"partition_replace_ms\":{},\"input_rows_per_sec\":{},\"output_rows_per_sec\":{},\"symbols_count\":{},\"parallelism\":\"{}\",\"worker_threads\":{}}}",
            self.total_ms,
            self.read_input_ms,
            self.read_state_ms,
            self.group_ms,
            self.compute_ms,
            self.write_ms,
            self.staging_ms,
            self.partition_replace_ms,
            json_f64(self.input_rows_per_sec),
            json_f64(self.output_rows_per_sec),
            self.symbols_count,
            escape_json_string(&self.parallelism),
            self.worker_threads
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PerformanceTimings {
    run_started: Instant,
    read_input: Duration,
    read_state: Duration,
    group: Duration,
    compute: Duration,
    write: Duration,
    staging: Duration,
    partition_replace: Duration,
    parallelism: &'static str,
    worker_threads: usize,
}

impl PerformanceTimings {
    fn started() -> Self {
        Self {
            run_started: Instant::now(),
            read_input: Duration::ZERO,
            read_state: Duration::ZERO,
            group: Duration::ZERO,
            compute: Duration::ZERO,
            write: Duration::ZERO,
            staging: Duration::ZERO,
            partition_replace: Duration::ZERO,
            parallelism: "serial",
            worker_threads: rayon::current_num_threads(),
        }
    }

    fn finish(&self, input_rows: u64, output_rows: u64, symbols_count: u64) -> PerformanceMetrics {
        PerformanceMetrics {
            total_ms: self.run_started.elapsed().as_millis(),
            read_input_ms: self.read_input.as_millis(),
            read_state_ms: self.read_state.as_millis(),
            group_ms: self.group.as_millis(),
            compute_ms: self.compute.as_millis(),
            write_ms: self.write.as_millis(),
            staging_ms: self.staging.as_millis(),
            partition_replace_ms: self.partition_replace.as_millis(),
            input_rows_per_sec: rows_per_second(input_rows, self.read_input),
            output_rows_per_sec: rows_per_second(output_rows, self.compute),
            symbols_count,
            parallelism: self.parallelism.to_string(),
            worker_threads: self.worker_threads,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Timed<T> {
    value: T,
    elapsed: Duration,
}

fn time_result<T>(
    action: impl FnOnce() -> Result<T, FurnaceIoError>,
) -> Result<Timed<T>, FurnaceIoError> {
    let started = Instant::now();
    let value = action()?;
    Ok(Timed {
        value,
        elapsed: started.elapsed(),
    })
}

/// staging 表校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSummary {
    /// 校验状态。
    pub status: String,
    /// 重复键数量。
    pub duplicate_keys: u64,
}

impl ValidationSummary {
    fn not_applicable() -> Self {
        Self {
            status: "not_applicable".to_string(),
            duplicate_keys: 0,
        }
    }

    fn passed() -> Self {
        Self {
            status: "passed".to_string(),
            duplicate_keys: 0,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"status\":\"{}\",\"duplicate_keys\":{}}}",
            escape_json_string(&self.status),
            self.duplicate_keys
        )
    }
}

/// 分区替换结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionReplaceSummary {
    /// 替换状态。
    pub status: String,
    /// 已替换的年份分区。
    pub years: Vec<u16>,
}

impl PartitionReplaceSummary {
    fn not_applicable() -> Self {
        Self {
            status: "not_applicable".to_string(),
            years: Vec::new(),
        }
    }

    fn replaced(years: Vec<u16>) -> Self {
        Self {
            status: "replaced".to_string(),
            years,
        }
    }

    fn to_json(&self) -> String {
        let years = self
            .years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"status\":\"{}\",\"years\":[{}]}}",
            escape_json_string(&self.status),
            years
        )
    }
}

/// 面向测试和 CLI 适配器的最小 ClickHouse 执行接口。
pub trait ClickHouseExecutor {
    /// 执行查询并返回标准输出。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError>;

    /// 执行查询并返回原始标准输出字节。
    ///
    /// RowBinary 等二进制格式可以避免大规模扫描时的文本解析开销。
    /// 测试执行器可以继续使用默认的 UTF-8 实现。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn query_bytes(&mut self, sql: &str) -> Result<Vec<u8>, FurnaceIoError> {
        self.query(sql).map(String::into_bytes)
    }

    /// 执行 INSERT 语句，并通过 stdin 提供 TSV 行。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError>;

    /// 执行 INSERT 语句，并通过 stdin 提供原始字节。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn insert_bytes(&mut self, sql: &str, bytes: &[u8]) -> Result<(), FurnaceIoError>;

    /// 执行语句并忽略其标准输出。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError> {
        self.query(sql).map(|_| ())
    }

    /// 执行多条语句并忽略其标准输出。
    ///
    /// 默认实现会逐条执行语句。基于 CLI 的执行器可以覆盖该方法，
    /// 以减少子进程往返次数。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn execute_many(&mut self, sqls: &[String]) -> Result<(), FurnaceIoError> {
        for sql in sqls {
            self.execute(sql)?;
        }
        Ok(())
    }
}

/// `clickhouse-client` 子进程执行器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseCliExecutor {
    command: String,
    command_args: Vec<String>,
    host: String,
    port: String,
    user: Option<String>,
    password: Option<String>,
    secure: bool,
    connect_timeout_seconds: Option<String>,
    query_timeout_seconds: Option<String>,
}

impl ClickHouseCliExecutor {
    /// 根据环境变量构造 CLI 执行器。
    ///
    /// 支持的变量包括：`FURNACE_CLICKHOUSE_CLIENT`、`CLICKHOUSE_HOST`、
    /// `FURNACE_CLICKHOUSE_CLIENT_ARGS`, `CLICKHOUSE_NATIVE_PORT`,
    /// `CLICKHOUSE_USER`, `CLICKHOUSE_PASSWORD`, `CLICKHOUSE_SECURE`,
    /// `CLICKHOUSE_CONNECT_TIMEOUT_SECONDS` 和 `CLICKHOUSE_QUERY_TIMEOUT_SECONDS`。
    pub fn from_env() -> Self {
        Self {
            command: env::var("FURNACE_CLICKHOUSE_CLIENT")
                .or_else(|_| env::var("CLICKHOUSE_CLIENT"))
                .unwrap_or_else(|_| "clickhouse-client".to_string()),
            command_args: env::var("FURNACE_CLICKHOUSE_CLIENT_ARGS")
                .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
                .unwrap_or_default(),
            host: env::var("CLICKHOUSE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("CLICKHOUSE_NATIVE_PORT").unwrap_or_else(|_| "9000".to_string()),
            user: env::var("CLICKHOUSE_USER").ok(),
            password: env::var("CLICKHOUSE_PASSWORD").ok(),
            secure: env::var("CLICKHOUSE_SECURE")
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false),
            connect_timeout_seconds: env::var("CLICKHOUSE_CONNECT_TIMEOUT_SECONDS").ok(),
            query_timeout_seconds: env::var("CLICKHOUSE_QUERY_TIMEOUT_SECONDS").ok(),
        }
    }

    fn base_command(&self) -> Command {
        let mut command = Command::new(&self.command);
        command.args(&self.command_args);
        command.arg("--host").arg(&self.host);
        command.arg("--port").arg(&self.port);
        if let Some(user) = &self.user {
            command.arg("--user").arg(user);
        }
        if let Some(password) = &self.password {
            command.arg("--password").arg(password);
        }
        if self.secure {
            command.arg("--secure");
        }
        if let Some(timeout) = &self.connect_timeout_seconds {
            command.arg("--connect_timeout").arg(timeout);
        }
        if let Some(timeout) = &self.query_timeout_seconds {
            command.arg("--receive_timeout").arg(timeout);
            command.arg("--send_timeout").arg(timeout);
        }
        command
    }
}

impl ClickHouseExecutor for ClickHouseCliExecutor {
    fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError> {
        let output = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .output()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn query_bytes(&mut self, sql: &str) -> Result<Vec<u8>, FurnaceIoError> {
        let output = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .output()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(output.stdout)
    }

    fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError> {
        self.insert_bytes(sql, tsv.as_bytes())
    }

    fn insert_bytes(&mut self, sql: &str, bytes: &[u8]) -> Result<(), FurnaceIoError> {
        let mut child = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;

        let Some(stdin) = child.stdin.as_mut() else {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: "failed to open clickhouse-client stdin".to_string(),
                source: None,
            });
        };
        stdin
            .write_all(bytes)
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: "failed to write insert bytes to clickhouse-client".to_string(),
                source: Some(source.to_string()),
            })?;

        let output =
            child
                .wait_with_output()
                .map_err(|source| FurnaceIoError::ClickHouseCommand {
                    message: format!("failed to wait for {}", self.command),
                    source: Some(source.to_string()),
                })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(())
    }

    fn execute_many(&mut self, sqls: &[String]) -> Result<(), FurnaceIoError> {
        if sqls.is_empty() {
            return Ok(());
        }
        let mut command = self.base_command();
        for sql in sqls {
            command.arg("--query").arg(sql);
        }
        let output = command
            .output()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(())
    }
}

/// Furnace I/O 返回的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FurnaceIoError {
    /// 请求无法安全执行。
    InvalidRequest(String),
    /// ClickHouse 子进程或查询执行失败。
    ClickHouseCommand {
        /// 错误摘要。
        message: String,
        /// 可选的 stderr 或底层来源细节。
        source: Option<String>,
    },
    /// 无法解析 ClickHouse 输出。
    Parse(String),
    /// 指标计算失败。
    Compute(String),
}

impl fmt::Display for FurnaceIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::Parse(message) | Self::Compute(message) => {
                f.write_str(message)
            }
            Self::ClickHouseCommand { message, source } => {
                if let Some(source) = source {
                    write!(f, "{message}: {source}")
                } else {
                    f.write_str(message)
                }
            }
        }
    }
}

impl Error for FurnaceIoError {}

#[derive(Debug, Clone, PartialEq)]
struct KdjResultRow {
    security_code: String,
    trade_date: String,
    rsv_window: u16,
    k_smoothing: u16,
    d_smoothing: u16,
    rsv: Option<f64>,
    k_value: Option<f64>,
    d_value: Option<f64>,
    j_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct KdjCalculationResult {
    rows: Vec<KdjResultRow>,
    output_rows: u64,
    null_indicator_rows: u64,
    compute_elapsed: Duration,
    parallelism: &'static str,
    worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct KdjInputGroups {
    groups: Vec<KdjGroupedInput>,
    input_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct KdjGroupedInput {
    security_code: String,
    inputs: Vec<KdjInput>,
}

#[derive(Debug, Clone, PartialEq)]
struct KdjSecurityCalculation {
    rows: Vec<KdjResultRow>,
    output_rows: u64,
    null_indicator_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct MaResultRow {
    security_code: String,
    trade_date: String,
    price_ma_3: Option<f64>,
    price_ma_5: Option<f64>,
    price_ma_6: Option<f64>,
    price_ma_10: Option<f64>,
    price_ma_12: Option<f64>,
    price_ma_14: Option<f64>,
    price_ma_20: Option<f64>,
    price_ma_24: Option<f64>,
    price_ma_28: Option<f64>,
    price_ma_57: Option<f64>,
    price_ma_60: Option<f64>,
    price_ma_114: Option<f64>,
    price_ma_250: Option<f64>,
    price_avg_ma_3_6_12_24: Option<f64>,
    price_avg_ma_14_28_57_114: Option<f64>,
    price_ema1_10_state: Option<f64>,
    price_ema2_10: Option<f64>,
    price_ema2_10_state: Option<f64>,
    volume_ma_5: Option<f64>,
    volume_ma_10: Option<f64>,
    volume_ma_20: Option<f64>,
    volume_ma_60: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct MaCalculationResult {
    rows: Vec<MaResultRow>,
    output_rows: u64,
    valid_close_rows: u64,
    valid_volume_rows: u64,
    null_indicator_rows: u64,
    compute_elapsed: Duration,
    parallelism: &'static str,
    worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct MaInputGroups {
    groups: Vec<MaGroupedInput>,
    input_rows: u64,
    valid_close_rows: u64,
    valid_volume_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct MaGroupedInput {
    security_code: String,
    inputs: Vec<MaInput>,
}

#[derive(Debug, Clone, PartialEq)]
struct MaSecurityCalculation {
    rows: Vec<MaResultRow>,
    output_rows: u64,
    valid_close_rows: u64,
    valid_volume_rows: u64,
    null_indicator_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct RsiResultRow {
    security_code: String,
    trade_date: String,
    rsi_6: Option<f64>,
    rsi_12: Option<f64>,
    rsi_14: Option<f64>,
    rsi_24: Option<f64>,
    rsi_25: Option<f64>,
    rsi_50: Option<f64>,
    avg_gain_6_state: Option<f64>,
    avg_loss_6_state: Option<f64>,
    avg_gain_12_state: Option<f64>,
    avg_loss_12_state: Option<f64>,
    avg_gain_14_state: Option<f64>,
    avg_loss_14_state: Option<f64>,
    avg_gain_24_state: Option<f64>,
    avg_loss_24_state: Option<f64>,
    avg_gain_25_state: Option<f64>,
    avg_loss_25_state: Option<f64>,
    avg_gain_50_state: Option<f64>,
    avg_loss_50_state: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct RsiCalculationResult {
    rows: Vec<RsiResultRow>,
    output_rows: u64,
    valid_close_rows: u64,
    null_indicator_rows: u64,
    compute_elapsed: Duration,
    parallelism: &'static str,
    worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct RsiInputGroups {
    groups: Vec<RsiGroupedInput>,
    input_rows: u64,
    valid_close_rows: u64,
    input_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct RsiGroupedInput {
    security_code: String,
    inputs: Vec<RsiInput>,
}

#[derive(Debug, Clone, PartialEq)]
struct RsiSecurityCalculation {
    rows: Vec<RsiResultRow>,
    output_rows: u64,
    valid_close_rows: u64,
    null_indicator_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct BollResultRow {
    security_code: String,
    trade_date: String,
    boll_mid_10_1p5: Option<f64>,
    boll_up_10_1p5: Option<f64>,
    boll_dn_10_1p5: Option<f64>,
    boll_mid_20_2: Option<f64>,
    boll_up_20_2: Option<f64>,
    boll_dn_20_2: Option<f64>,
    boll_mid_50_2p5: Option<f64>,
    boll_up_50_2p5: Option<f64>,
    boll_dn_50_2p5: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct BollCalculationResult {
    rows: Vec<BollResultRow>,
    output_rows: u64,
    output_valid_close_rows: u64,
    null_indicator_rows: u64,
    compute_elapsed: Duration,
    parallelism: &'static str,
    worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct BollInputGroups {
    groups: Vec<BollGroupedInput>,
    input_rows: u64,
    input_valid_close_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct BollGroupedInput {
    security_code: String,
    inputs: Vec<BollInput>,
}

#[derive(Debug, Clone, PartialEq)]
struct BollSecurityCalculation {
    rows: Vec<BollResultRow>,
    output_rows: u64,
    output_valid_close_rows: u64,
    null_indicator_rows: u64,
}

/// 基于 ClickHouse 执行完整 KDJ 计算。
///
/// # 错误
///
/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_kdj<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
) -> Result<KdjRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        executor.execute(create_calculation_database_sql())?;
        executor.execute(&create_kdj_output_table_sql())?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_symbols(executor, request)?;
    if request.mode.writes_applied() && symbols.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production KDJ writes require at least one input security".to_string(),
        ));
    }
    let effective_output_to =
        resolve_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let input_from = resolve_input_from(executor, request, &symbols, all_symbols_requested)?;
    let target_exists = target_table_exists(executor)?;
    let states = if target_exists {
        let timed = time_result(|| {
            read_previous_states(executor, request, &symbols, all_symbols_requested)
        })?;
        timings.read_state = timed.elapsed;
        timed.value
    } else {
        HashMap::new()
    };
    let timed_input = time_result(|| {
        read_input_row_binary(
            executor,
            &symbols,
            all_symbols_requested,
            &input_from,
            &effective_output_to,
        )
    })?;
    timings.read_input = timed_input.elapsed;
    let input_bytes = timed_input.value;
    let timed_groups = time_result(|| group_input_rows(&input_bytes))?;
    timings.group = timed_groups.elapsed;
    drop(input_bytes);
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;

    let calculated = calculate_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        &states,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    if request.mode.writes_applied() && output_rows.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production KDJ writes produced no output rows".to_string(),
        ));
    }
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        KdjWriteMode::DryRun => {}
        KdjWriteMode::AppendLatest => {
            ensure_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_result_rows(
                    executor,
                    DEFAULT_KDJ_OUTPUT_TABLE,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        KdjWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = kdj_staging_table_name(run_id);
            let staging_setup_sql = vec![
                drop_kdj_staging_table_sql(&staging),
                create_kdj_staging_table_sql(&staging),
            ];
            let timed = time_result(|| executor.execute_many(&staging_setup_sql))?;
            timings.staging += timed.elapsed;
            let timed = time_result(|| {
                retain_old_rows_for_staging(
                    executor,
                    request,
                    &staging,
                    &symbols,
                    all_symbols_requested,
                    &affected_years,
                    &effective_output_to,
                )
            })?;
            timings.staging += timed.elapsed;
            retained_rows = timed.value;
            let timed = time_result(|| {
                insert_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
            })?;
            timings.write += timed.elapsed;
            let timed = time_result(|| validate_staging(executor, &staging, &affected_years))?;
            timings.staging += timed.elapsed;
            staging_validation = timed.value;
            if staging_validation.status != "passed" {
                return Err(FurnaceIoError::InvalidRequest(format!(
                    "staging validation failed with {} duplicate keys",
                    staging_validation.duplicate_keys
                )));
            }
            let replace_sql = affected_years
                .iter()
                .map(|year| replace_kdj_partition_sql(&staging, *year))
                .collect::<Vec<_>>();
            let timed = time_result(|| executor.execute_many(&replace_sql))?;
            timings.partition_replace += timed.elapsed;
            let timed = time_result(|| executor.execute(&drop_kdj_staging_table_sql(&staging)))?;
            timings.staging += timed.elapsed;
            partition_replace = PartitionReplaceSummary::replaced(affected_years.clone());
            staging_table = Some(staging);
        }
    }

    let state_source = if states.is_empty() {
        "initial_50".to_string()
    } else {
        format!("previous_kd_rows:{}", states.len())
    };
    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(KdjRunSummary {
        request_from: request.request_from.clone(),
        request_to: request.request_to.clone(),
        effective_output_from: request.request_from.clone(),
        effective_output_to: effective_output_to.clone(),
        input_from,
        input_to: effective_output_to,
        mode: request.mode,
        symbols,
        input_rows: input_rows_count,
        output_rows: output_rows_count,
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        params: request.params,
        state_source,
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}

/// 基于 ClickHouse 执行完整 Moving Average 计算。
///
/// # 错误
///
/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_ma<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
) -> Result<MaRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        executor.execute(create_calculation_database_sql())?;
        executor.execute(&create_ma_output_table_sql(&request.output_table))?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_ma_symbols(executor, request)?;
    if request.mode.writes_applied() && symbols.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production MA writes require at least one input security".to_string(),
        ));
    }
    let effective_output_to =
        resolve_ma_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let ma_target_exists = table_exists(executor, &request.output_table)?;
    let ma_states = if ma_target_exists {
        let timed = time_result(|| {
            read_previous_ma_states(executor, request, &symbols, all_symbols_requested)
        })?;
        timings.read_state = timed.elapsed;
        timed.value
    } else {
        HashMap::new()
    };
    let missing_state_symbols = symbols
        .iter()
        .filter(|symbol| !ma_states.contains_key(symbol.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let can_consider_previous_state =
        request.mode != MaWriteMode::ReplaceCascade && !symbols.is_empty() && !ma_states.is_empty();
    let lookback_input_from = if can_consider_previous_state {
        Some(resolve_ma_lookback_input_from(
            executor,
            request,
            &symbols,
            all_symbols_requested,
        )?)
    } else {
        None
    };
    let missing_started_before_lookback = if let Some(input_from) = lookback_input_from.as_deref() {
        !missing_state_symbols.is_empty()
            && ma_symbols_started_before(executor, request, &missing_state_symbols, input_from)?
    } else {
        false
    };
    let can_use_previous_state = can_consider_previous_state && !missing_started_before_lookback;
    let input_from = if can_use_previous_state {
        lookback_input_from.unwrap_or_else(|| request.request_from.clone())
    } else {
        resolve_ma_input_from(executor, request, &symbols, all_symbols_requested)?
    };
    let timed_input = time_result(|| {
        read_ma_input_row_binary(
            executor,
            request,
            &symbols,
            all_symbols_requested,
            &input_from,
            &effective_output_to,
        )
    })?;
    timings.read_input = timed_input.elapsed;
    let input_bytes = timed_input.value;
    let timed_groups = time_result(|| group_ma_input_rows(&input_bytes))?;
    timings.group = timed_groups.elapsed;
    drop(input_bytes);
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;
    let input_valid_close_rows = input_groups.valid_close_rows;
    let input_valid_volume_rows = input_groups.valid_volume_rows;

    let calculated = calculate_ma_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        &ma_states,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    if request.mode.writes_applied() && output_rows.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production MA writes produced no output rows".to_string(),
        ));
    }
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        MaWriteMode::DryRun => {}
        MaWriteMode::AppendLatest => {
            ensure_ma_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_ma_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        MaWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = ma_staging_table_name(&request.output_table, run_id);
            let staging_setup_sql = vec![
                drop_ma_staging_table_sql(&staging),
                create_ma_staging_table_sql(&request.output_table, &staging),
            ];
            let timed = time_result(|| executor.execute_many(&staging_setup_sql))?;
            timings.staging += timed.elapsed;
            let timed = time_result(|| {
                retain_old_ma_rows_for_staging(
                    executor,
                    request,
                    &staging,
                    &symbols,
                    all_symbols_requested,
                    &affected_years,
                    &effective_output_to,
                )
            })?;
            timings.staging += timed.elapsed;
            retained_rows = timed.value;
            let timed = time_result(|| {
                insert_ma_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
            })?;
            timings.write += timed.elapsed;
            let timed = time_result(|| validate_staging(executor, &staging, &affected_years))?;
            timings.staging += timed.elapsed;
            staging_validation = timed.value;
            if staging_validation.status != "passed" {
                return Err(FurnaceIoError::InvalidRequest(format!(
                    "staging validation failed with {} duplicate keys",
                    staging_validation.duplicate_keys
                )));
            }
            let replace_sql = affected_years
                .iter()
                .map(|year| replace_ma_partition_sql(&request.output_table, &staging, *year))
                .collect::<Vec<_>>();
            let timed = time_result(|| executor.execute_many(&replace_sql))?;
            timings.partition_replace += timed.elapsed;
            let timed = time_result(|| executor.execute(&drop_ma_staging_table_sql(&staging)))?;
            timings.staging += timed.elapsed;
            partition_replace = PartitionReplaceSummary::replaced(affected_years.clone());
            staging_table = Some(staging);
        }
    }

    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(MaRunSummary {
        request_from: request.request_from.clone(),
        request_to: request.request_to.clone(),
        effective_output_from: request.request_from.clone(),
        effective_output_to: effective_output_to.clone(),
        input_from,
        input_to: effective_output_to,
        mode: request.mode,
        symbols,
        input_rows: input_rows_count,
        output_rows: output_rows_count,
        valid_close_rows: calculated.valid_close_rows.min(input_valid_close_rows),
        valid_volume_rows: calculated.valid_volume_rows.min(input_valid_volume_rows),
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        ema_state_source: if can_use_previous_state {
            if missing_state_symbols.is_empty() {
                "previous-state".to_string()
            } else {
                "mixed".to_string()
            }
        } else {
            "full-history".to_string()
        },
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}

/// 基于 ClickHouse 执行完整 RSI 计算。
///
/// # 错误
///
/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_rsi<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
) -> Result<RsiRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        executor.execute(create_calculation_database_sql())?;
        executor.execute(&create_rsi_output_table_sql(&request.output_table))?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_rsi_symbols(executor, request)?;
    if request.mode.writes_applied() && symbols.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production RSI writes require at least one input security".to_string(),
        ));
    }
    let effective_output_to =
        resolve_rsi_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let full_history_input_from =
        resolve_rsi_input_from(executor, request, &symbols, all_symbols_requested)?;
    let request_covers_full_history =
        request.request_from.as_str() <= full_history_input_from.as_str();
    let rsi_target_exists = table_exists(executor, &request.output_table)?;
    let previous_states = if rsi_target_exists
        && request.mode != RsiWriteMode::ReplaceCascade
        && !request_covers_full_history
    {
        let timed = time_result(|| {
            read_previous_rsi_states(executor, request, &symbols, all_symbols_requested)
        })?;
        timings.read_state = timed.elapsed;
        timed.value
    } else {
        HashMap::new()
    };
    let timed_gap =
        time_result(|| count_rsi_gap_symbols(executor, request, &symbols, all_symbols_requested))?;
    timings.read_state += timed_gap.elapsed;
    let (gap_symbols_count, gap_fill_from) = timed_gap.value;
    if request.mode == RsiWriteMode::AppendLatest && gap_symbols_count > 0 {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "append-latest found RSI result gaps for {gap_symbols_count} symbols; rerun from {} or use replace-cascade",
            gap_fill_from.as_deref().unwrap_or(&request.request_from)
        )));
    }

    let can_use_previous_state = request.mode != RsiWriteMode::ReplaceCascade
        && gap_symbols_count == 0
        && !previous_states.is_empty();
    let states_for_compute = if can_use_previous_state {
        previous_states
    } else {
        HashMap::new()
    };

    let timed_input = if can_use_previous_state {
        time_result(|| {
            read_rsi_mixed_input_row_binary(
                executor,
                request,
                &symbols,
                all_symbols_requested,
                &effective_output_to,
            )
        })?
    } else {
        time_result(|| {
            read_rsi_input_row_binary(
                executor,
                request,
                &symbols,
                all_symbols_requested,
                &full_history_input_from,
                &effective_output_to,
            )
        })?
    };
    timings.read_input = timed_input.elapsed;
    let input_bytes = timed_input.value;
    let timed_groups = time_result(|| group_rsi_input_rows(&input_bytes))?;
    timings.group = timed_groups.elapsed;
    drop(input_bytes);
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;
    let input_valid_close_rows = input_groups.valid_close_rows;
    let input_from = input_groups
        .input_from
        .clone()
        .unwrap_or(full_history_input_from);

    let calculated = calculate_rsi_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        &states_for_compute,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    if request.mode.writes_applied() && output_rows.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production RSI writes produced no output rows".to_string(),
        ));
    }
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        RsiWriteMode::DryRun => {}
        RsiWriteMode::AppendLatest => {
            ensure_rsi_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_rsi_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        RsiWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = rsi_staging_table_name(&request.output_table, run_id);
            let staging_setup_sql = vec![
                drop_rsi_staging_table_sql(&staging),
                create_rsi_staging_table_sql(&request.output_table, &staging),
            ];
            let timed = time_result(|| executor.execute_many(&staging_setup_sql))?;
            timings.staging += timed.elapsed;
            let timed = time_result(|| {
                retain_old_rsi_rows_for_staging(
                    executor,
                    request,
                    &staging,
                    &symbols,
                    all_symbols_requested,
                    &affected_years,
                    &effective_output_to,
                )
            })?;
            timings.staging += timed.elapsed;
            retained_rows = timed.value;
            let timed = time_result(|| {
                insert_rsi_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
            })?;
            timings.write += timed.elapsed;
            let timed = time_result(|| validate_staging(executor, &staging, &affected_years))?;
            timings.staging += timed.elapsed;
            staging_validation = timed.value;
            if staging_validation.status != "passed" {
                return Err(FurnaceIoError::InvalidRequest(format!(
                    "staging validation failed with {} duplicate keys",
                    staging_validation.duplicate_keys
                )));
            }
            let replace_sql = affected_years
                .iter()
                .map(|year| replace_rsi_partition_sql(&request.output_table, &staging, *year))
                .collect::<Vec<_>>();
            let timed = time_result(|| executor.execute_many(&replace_sql))?;
            timings.partition_replace += timed.elapsed;
            let timed = time_result(|| executor.execute(&drop_rsi_staging_table_sql(&staging)))?;
            timings.staging += timed.elapsed;
            partition_replace = PartitionReplaceSummary::replaced(affected_years.clone());
            staging_table = Some(staging);
        }
    }

    let rsi_state_source = if can_use_previous_state {
        if states_for_compute.len() == symbols.len() {
            format!("previous-state:{}", states_for_compute.len())
        } else {
            format!(
                "mixed:previous-state:{},full-history:{}",
                states_for_compute.len(),
                symbols.len().saturating_sub(states_for_compute.len())
            )
        }
    } else {
        "full-history".to_string()
    };
    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(RsiRunSummary {
        request_from: request.request_from.clone(),
        request_to: request.request_to.clone(),
        effective_output_from: request.request_from.clone(),
        effective_output_to: effective_output_to.clone(),
        input_from,
        input_to: effective_output_to,
        mode: request.mode,
        symbols,
        input_rows: input_rows_count,
        output_rows: output_rows_count,
        valid_close_rows: input_valid_close_rows,
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        rsi_state_source,
        gap_symbols_count,
        gap_fill_from,
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}

/// 基于 ClickHouse 执行完整 Bollinger Bands 计算。
///
/// # 错误
///
/// 当请求校验、ClickHouse I/O 或指标计算失败时，返回 [`FurnaceIoError`]。
pub fn run_boll<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
) -> Result<BollRunSummary, FurnaceIoError> {
    let mut timings = PerformanceTimings::started();

    request.validate()?;

    if request.mode.writes_applied() {
        executor.execute(create_calculation_database_sql())?;
        executor.execute(&create_boll_output_table_sql(&request.output_table))?;
    }

    let all_symbols_requested = request.symbols.is_empty();
    let symbols = resolve_boll_symbols(executor, request)?;
    if request.mode.writes_applied() && symbols.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production Bollinger Bands writes require at least one input security".to_string(),
        ));
    }
    let effective_output_to =
        resolve_boll_effective_output_to(executor, request, &symbols, all_symbols_requested)?;
    let input_from =
        resolve_boll_lookback_input_from(executor, request, &symbols, all_symbols_requested)?;
    let timed_input = time_result(|| {
        read_boll_input_row_binary(
            executor,
            request,
            &symbols,
            all_symbols_requested,
            &input_from,
            &effective_output_to,
        )
    })?;
    timings.read_input = timed_input.elapsed;
    let input_bytes = timed_input.value;
    let timed_groups = time_result(|| group_boll_input_rows(&input_bytes))?;
    timings.group = timed_groups.elapsed;
    drop(input_bytes);
    let input_groups = timed_groups.value;
    let input_rows_count = input_groups.input_rows;
    let input_valid_close_rows = input_groups.input_valid_close_rows;

    let calculated = calculate_boll_outputs(
        request,
        &effective_output_to,
        input_groups.groups,
        input_rows_count as usize,
        request.mode.writes_applied(),
    )?;
    timings.compute = calculated.compute_elapsed;
    timings.parallelism = calculated.parallelism;
    timings.worker_threads = calculated.worker_threads;
    let output_rows = calculated.rows;
    if request.mode.writes_applied() && output_rows.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production Bollinger Bands writes produced no output rows".to_string(),
        ));
    }
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let output_rows_count = calculated.output_rows;
    let output_valid_close_rows = calculated.output_valid_close_rows;
    let null_indicator_rows = calculated.null_indicator_rows;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        BollWriteMode::DryRun => {}
        BollWriteMode::AppendLatest => {
            ensure_boll_append_latest_is_safe(executor, request, &symbols, all_symbols_requested)?;
            let timed = time_result(|| {
                insert_boll_result_rows(
                    executor,
                    &request.output_table,
                    &output_rows,
                    request.insert_batch_size,
                )
            })?;
            timings.write += timed.elapsed;
        }
        BollWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = boll_staging_table_name(&request.output_table, run_id);
            let staging_setup_sql = vec![
                drop_boll_staging_table_sql(&staging),
                create_boll_staging_table_sql(&request.output_table, &staging),
            ];
            let timed = time_result(|| executor.execute_many(&staging_setup_sql))?;
            timings.staging += timed.elapsed;
            let timed = time_result(|| {
                retain_old_boll_rows_for_staging(
                    executor,
                    request,
                    &staging,
                    &symbols,
                    all_symbols_requested,
                    &affected_years,
                    &effective_output_to,
                )
            })?;
            timings.staging += timed.elapsed;
            retained_rows = timed.value;
            let timed = time_result(|| {
                insert_boll_result_rows(executor, &staging, &output_rows, request.insert_batch_size)
            })?;
            timings.write += timed.elapsed;
            let timed = time_result(|| validate_staging(executor, &staging, &affected_years))?;
            timings.staging += timed.elapsed;
            staging_validation = timed.value;
            if staging_validation.status != "passed" {
                return Err(FurnaceIoError::InvalidRequest(format!(
                    "staging validation failed with {} duplicate keys",
                    staging_validation.duplicate_keys
                )));
            }
            let replace_sql = affected_years
                .iter()
                .map(|year| replace_boll_partition_sql(&request.output_table, &staging, *year))
                .collect::<Vec<_>>();
            let timed = time_result(|| executor.execute_many(&replace_sql))?;
            timings.partition_replace += timed.elapsed;
            let timed = time_result(|| executor.execute(&drop_boll_staging_table_sql(&staging)))?;
            timings.staging += timed.elapsed;
            partition_replace = PartitionReplaceSummary::replaced(affected_years.clone());
            staging_table = Some(staging);
        }
    }

    let symbols_count = symbols.len() as u64;
    let performance_metrics = timings.finish(input_rows_count, output_rows_count, symbols_count);

    Ok(BollRunSummary {
        request_from: request.request_from.clone(),
        request_to: request.request_to.clone(),
        effective_output_from: request.request_from.clone(),
        effective_output_to: effective_output_to.clone(),
        input_from,
        input_to: effective_output_to,
        mode: request.mode,
        symbols,
        input_rows: input_rows_count,
        output_rows: output_rows_count,
        input_valid_close_rows,
        output_valid_close_rows,
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        state_source: "rolling-lookback".to_string(),
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
        performance_metrics,
    })
}

fn resolve_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
) -> Result<Vec<String>, FurnaceIoError> {
    if !request.symbols.is_empty() {
        return Ok(normalize_symbols(&request.symbols));
    }

    let sql = format!(
        "\
SELECT security_code
FROM {DEFAULT_INPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
GROUP BY security_code
ORDER BY security_code
FORMAT TSV",
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
}

fn normalize_symbols(symbols: &[String]) -> Vec<String> {
    let mut unique = BTreeSet::new();
    for symbol in symbols {
        let symbol = symbol.trim();
        if !symbol.is_empty() {
            unique.insert(symbol.to_string());
        }
    }
    unique.into_iter().collect()
}

fn resolve_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != KdjWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT toString(max(trade_date))
FROM {DEFAULT_INPUT_TABLE}
WHERE {}
FORMAT TSV",
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
    Ok(value.max(request.request_to.clone()))
}

fn resolve_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let warmup_window = u32::from(
        request
            .params
            .rsv_window
            .max(request.params.k_smoothing)
            .max(request.params.d_smoothing),
    ) * u32::from(DEFAULT_WARMUP_MULTIPLE);

    let symbol_filter = symbol_where_clause(symbols, all_symbols_requested);
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM (
    SELECT trade_date
    FROM {DEFAULT_INPUT_TABLE}
    WHERE trade_date <= toDate('{}')
      AND {symbol_filter}
    GROUP BY trade_date
    ORDER BY trade_date DESC
    LIMIT {warmup_window}
)
FORMAT TSV",
        sql_string(&request.request_from)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}

fn target_table_exists<E: ClickHouseExecutor>(executor: &mut E) -> Result<bool, FurnaceIoError> {
    let value = first_tsv_value(&executor.query(&format!(
        "EXISTS TABLE {DEFAULT_KDJ_OUTPUT_TABLE} FORMAT TSV"
    ))?)
    .unwrap_or_else(|| "0".to_string());
    Ok(value == "1")
}

fn table_exists<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
) -> Result<bool, FurnaceIoError> {
    let value = first_tsv_value(&executor.query(&format!("EXISTS TABLE {table} FORMAT TSV"))?)
        .unwrap_or_else(|| "0".to_string());
    Ok(value == "1")
}

fn read_previous_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<HashMap<String, KdjState>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT security_code, k_value, d_value
FROM (
    SELECT
        security_code,
        trade_date,
        k_value,
        d_value,
        row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
    FROM {DEFAULT_KDJ_OUTPUT_TABLE}
    WHERE trade_date < toDate('{}')
      AND k_value IS NOT NULL
      AND d_value IS NOT NULL
      AND {}
)
WHERE rn = 1
FORMAT TSV",
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    let mut states = HashMap::new();
    for line in executor
        .query(&sql)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 3 previous-state fields, got {}",
                fields.len()
            )));
        }
        let k_value = parse_f64(fields[1])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous k_value must not be null".to_string())
        })?;
        let d_value = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous d_value must not be null".to_string())
        })?;
        states.insert(fields[0].to_string(), KdjState::new(k_value, d_value));
    }
    Ok(states)
}

fn read_previous_ma_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<HashMap<String, MaPreviousState>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT security_code, toString(trade_date), price_ema1_10_state, price_ema2_10_state
FROM (
    SELECT
        security_code,
        trade_date,
        price_ema1_10_state,
        price_ema2_10_state,
        row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
    FROM {}
    WHERE trade_date < toDate('{}')
      AND price_ema1_10_state IS NOT NULL
      AND price_ema2_10_state IS NOT NULL
      AND {}
)
WHERE rn = 1
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    let mut states = HashMap::new();
    for line in executor
        .query(&sql)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 4 previous MA state fields, got {}",
                fields.len()
            )));
        }
        let ema1 = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous price_ema1_10_state must not be null".to_string())
        })?;
        let ema2 = parse_f64(fields[3])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous price_ema2_10_state must not be null".to_string())
        })?;
        states.insert(
            fields[0].to_string(),
            MaPreviousState::new(
                fields[1].to_string(),
                MaState::new(ema1, ema2)
                    .map_err(|source| FurnaceIoError::Parse(source.to_string()))?,
            ),
        );
    }
    Ok(states)
}

fn read_previous_rsi_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<HashMap<String, RsiPreviousState>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT
    state.security_code,
    toString(state.state_date),
    input.close_price_forward_adj,
    state.state_avg_gain_6,
    state.state_avg_loss_6,
    state.state_avg_gain_12,
    state.state_avg_loss_12,
    state.state_avg_gain_14,
    state.state_avg_loss_14,
    state.state_avg_gain_24,
    state.state_avg_loss_24,
    state.state_avg_gain_25,
    state.state_avg_loss_25,
    state.state_avg_gain_50,
    state.state_avg_loss_50
FROM (
    SELECT
        security_code,
        max(trade_date) AS state_date,
        argMax(avg_gain_6_state, trade_date) AS state_avg_gain_6,
        argMax(avg_loss_6_state, trade_date) AS state_avg_loss_6,
        argMax(avg_gain_12_state, trade_date) AS state_avg_gain_12,
        argMax(avg_loss_12_state, trade_date) AS state_avg_loss_12,
        argMax(avg_gain_14_state, trade_date) AS state_avg_gain_14,
        argMax(avg_loss_14_state, trade_date) AS state_avg_loss_14,
        argMax(avg_gain_24_state, trade_date) AS state_avg_gain_24,
        argMax(avg_loss_24_state, trade_date) AS state_avg_loss_24,
        argMax(avg_gain_25_state, trade_date) AS state_avg_gain_25,
        argMax(avg_loss_25_state, trade_date) AS state_avg_loss_25,
        argMax(avg_gain_50_state, trade_date) AS state_avg_gain_50,
        argMax(avg_loss_50_state, trade_date) AS state_avg_loss_50
    FROM {}
    WHERE trade_date < toDate('{}')
      AND avg_gain_6_state IS NOT NULL
      AND avg_loss_6_state IS NOT NULL
      AND avg_gain_12_state IS NOT NULL
      AND avg_loss_12_state IS NOT NULL
      AND avg_gain_14_state IS NOT NULL
      AND avg_loss_14_state IS NOT NULL
      AND avg_gain_24_state IS NOT NULL
      AND avg_loss_24_state IS NOT NULL
      AND avg_gain_25_state IS NOT NULL
      AND avg_loss_25_state IS NOT NULL
      AND avg_gain_50_state IS NOT NULL
      AND avg_loss_50_state IS NOT NULL
      AND {}
    GROUP BY security_code
) AS state
INNER JOIN {} AS input
    ON input.security_code = state.security_code
   AND input.trade_date = state.state_date
WHERE input.close_price_forward_adj IS NOT NULL
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table
    );

    let mut states = HashMap::new();
    for line in executor
        .query(&sql)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 15 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 15 previous RSI state fields, got {}",
                fields.len()
            )));
        }
        let previous_close = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous RSI close must not be null".to_string())
        })?;
        let state = RsiState::new(
            previous_close,
            rsi_window_state(fields[3], fields[4])?,
            rsi_window_state(fields[5], fields[6])?,
            rsi_window_state(fields[7], fields[8])?,
            rsi_window_state(fields[9], fields[10])?,
            rsi_window_state(fields[11], fields[12])?,
            rsi_window_state(fields[13], fields[14])?,
        )
        .map_err(|source| FurnaceIoError::Parse(source.to_string()))?;
        states.insert(
            fields[0].to_string(),
            RsiPreviousState::new(fields[1].to_string(), state),
        );
    }
    Ok(states)
}

fn rsi_window_state(gain: &str, loss: &str) -> Result<RsiWindowState, FurnaceIoError> {
    let gain = parse_f64(gain)?
        .ok_or_else(|| FurnaceIoError::Parse("previous RSI gain must not be null".to_string()))?;
    let loss = parse_f64(loss)?
        .ok_or_else(|| FurnaceIoError::Parse("previous RSI loss must not be null".to_string()))?;
    RsiWindowState::new(gain, loss).map_err(|source| FurnaceIoError::Parse(source.to_string()))
}

fn count_rsi_gap_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<(u64, Option<String>), FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok((0, None));
    }
    let sql = format!(
        "\
WITH states AS (
    SELECT security_code, max(trade_date) AS state_date
    FROM {}
    WHERE trade_date < toDate('{}')
      AND avg_gain_6_state IS NOT NULL
      AND avg_loss_6_state IS NOT NULL
      AND avg_gain_12_state IS NOT NULL
      AND avg_loss_12_state IS NOT NULL
      AND avg_gain_14_state IS NOT NULL
      AND avg_loss_14_state IS NOT NULL
      AND avg_gain_24_state IS NOT NULL
      AND avg_loss_24_state IS NOT NULL
      AND avg_gain_25_state IS NOT NULL
      AND avg_loss_25_state IS NOT NULL
      AND avg_gain_50_state IS NOT NULL
      AND avg_loss_50_state IS NOT NULL
      AND {}
    GROUP BY security_code
)
SELECT
    countDistinct(input.security_code),
    toString(min(input.trade_date))
FROM {} AS input
INNER JOIN states
    ON input.security_code = states.security_code
WHERE input.trade_date > states.state_date
  AND input.trade_date < toDate('{}')
  AND input.close_price_forward_adj IS NOT NULL
FORMAT TSV",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.input_table,
        sql_string(&request.request_from)
    );
    let output = executor.query(&sql)?;
    let fields = output
        .lines()
        .next()
        .unwrap_or_default()
        .split('\t')
        .collect::<Vec<_>>();
    let gap_symbols = fields
        .first()
        .map(|value| parse_u64(value))
        .transpose()?
        .unwrap_or(0);
    let gap_fill_from = fields.get(1).and_then(|value| {
        if gap_symbols == 0 || value.is_empty() || *value == "\\N" {
            None
        } else {
            Some((*value).to_string())
        }
    });
    Ok((gap_symbols, gap_fill_from))
}

fn read_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<u8>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    toString(trade_date),
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj
FROM {DEFAULT_INPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date
FORMAT RowBinary",
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}

fn resolve_ma_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
) -> Result<Vec<String>, FurnaceIoError> {
    if !request.symbols.is_empty() {
        return Ok(normalize_symbols(&request.symbols));
    }

    let sql = format!(
        "\
SELECT security_code
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
GROUP BY security_code
ORDER BY security_code
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
}

fn resolve_ma_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != MaWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT toString(max(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
    Ok(value.max(request.request_to.clone()))
}

fn resolve_ma_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}

fn resolve_rsi_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
) -> Result<Vec<String>, FurnaceIoError> {
    if !request.symbols.is_empty() {
        return Ok(normalize_symbols(&request.symbols));
    }

    let sql = format!(
        "\
SELECT security_code
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
GROUP BY security_code
ORDER BY security_code
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
}

fn resolve_rsi_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != RsiWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT toString(max(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
    Ok(value.max(request.request_to.clone()))
}

fn resolve_rsi_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}

fn resolve_ma_lookback_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let price_symbol_filter = symbol_where_clause(symbols, all_symbols_requested);
    let volume_symbol_filter =
        symbol_where_clause_for_column("adj.security_code", symbols, all_symbols_requested);
    let price_lookback_window = DEFAULT_PRICE_MA_WINDOWS
        .iter()
        .copied()
        .max()
        .unwrap_or(250);
    let volume_lookback_window = DEFAULT_VOLUME_MA_WINDOWS
        .iter()
        .copied()
        .max()
        .unwrap_or(60);
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM (
    SELECT trade_date
    FROM (
        SELECT
            security_code,
            trade_date,
            row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
        FROM {}
        WHERE trade_date <= toDate('{}')
          AND {} IS NOT NULL
          AND {price_symbol_filter}
    )
    WHERE rn <= {price_lookback_window}
    UNION ALL
    SELECT trade_date
    FROM (
        SELECT
            adj.security_code,
            adj.trade_date,
            row_number() OVER (PARTITION BY adj.security_code ORDER BY adj.trade_date DESC) AS rn
        FROM {} AS adj
        LEFT JOIN {} AS unadj
          ON adj.security_code = unadj.security_code
         AND adj.trade_date = unadj.trade_date
        WHERE adj.trade_date <= toDate('{}')
          AND unadj.{} IS NOT NULL
          AND {volume_symbol_filter}
    )
    WHERE rn <= {volume_lookback_window}
)
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        request.price_column,
        request.input_table,
        request.volume_input_table,
        sql_string(&request.request_from),
        request.volume_column
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}

fn ma_symbols_started_before<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    input_from: &str,
) -> Result<bool, FurnaceIoError> {
    if symbols.is_empty() {
        return Ok(false);
    }
    let sql = format!(
        "\
SELECT count()
FROM {}
WHERE trade_date < toDate('{}')
  AND {}
FORMAT TSV",
        request.input_table,
        sql_string(input_from),
        symbol_where_clause(symbols, false)
    );
    let count = parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())?;
    Ok(count > 0)
}

fn resolve_boll_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
) -> Result<Vec<String>, FurnaceIoError> {
    if !request.symbols.is_empty() {
        return Ok(normalize_symbols(&request.symbols));
    }

    let sql = format!(
        "\
SELECT security_code
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
GROUP BY security_code
ORDER BY security_code
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
}

fn resolve_boll_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != BollWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT toString(max(trade_date))
FROM {}
WHERE {}
FORMAT TSV",
        request.input_table,
        symbol_where_clause(symbols, all_symbols_requested)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
    Ok(value.max(request.request_to.clone()))
}

fn resolve_boll_lookback_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let symbol_filter = symbol_where_clause(symbols, all_symbols_requested);
    let max_window = request.params.max_window().max(DEFAULT_BOLL_MAX_WINDOW);
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM (
    SELECT trade_date
    FROM (
        SELECT
            security_code,
            trade_date,
            row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
        FROM {}
        WHERE trade_date <= toDate('{}')
          AND {} IS NOT NULL
          AND {symbol_filter}
    )
    WHERE rn <= {max_window}
)
FORMAT TSV",
        request.input_table,
        sql_string(&request.request_from),
        request.price_column
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}

fn read_ma_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<u8>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    adj.security_code,
    toString(adj.trade_date),
    adj.{},
    CAST(unadj.{}, 'Nullable(Float64)')
FROM {} AS adj
LEFT JOIN {} AS unadj
  ON adj.security_code = unadj.security_code
 AND adj.trade_date = unadj.trade_date
WHERE adj.trade_date >= toDate('{}')
  AND adj.trade_date <= toDate('{}')
  AND {}
ORDER BY adj.security_code, adj.trade_date
FORMAT RowBinary",
        request.price_column,
        request.volume_column,
        request.input_table,
        request.volume_input_table,
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause_for_column("adj.security_code", symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}

fn read_rsi_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<u8>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    toString(trade_date),
    {}
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date
FORMAT RowBinary",
        request.price_column,
        request.input_table,
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}

fn read_rsi_mixed_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &RsiRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_to: &str,
) -> Result<Vec<u8>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
WITH states AS (
    SELECT security_code, max(trade_date) AS state_date
    FROM {}
    WHERE trade_date < toDate('{}')
      AND avg_gain_6_state IS NOT NULL
      AND avg_loss_6_state IS NOT NULL
      AND avg_gain_12_state IS NOT NULL
      AND avg_loss_12_state IS NOT NULL
      AND avg_gain_14_state IS NOT NULL
      AND avg_loss_14_state IS NOT NULL
      AND avg_gain_24_state IS NOT NULL
      AND avg_loss_24_state IS NOT NULL
      AND avg_gain_25_state IS NOT NULL
      AND avg_loss_25_state IS NOT NULL
      AND avg_gain_50_state IS NOT NULL
      AND avg_loss_50_state IS NOT NULL
      AND {}
    GROUP BY security_code
)
SELECT
    input.security_code,
    toString(input.trade_date),
    input.{}
FROM {} AS input
LEFT JOIN states
    ON input.security_code = states.security_code
WHERE input.trade_date <= toDate('{}')
  AND {}
  AND (states.state_date IS NULL OR input.trade_date >= states.state_date)
ORDER BY input.security_code, input.trade_date
FORMAT RowBinary",
        request.output_table,
        sql_string(&request.request_from),
        symbol_where_clause(symbols, all_symbols_requested),
        request.price_column,
        request.input_table,
        sql_string(input_to),
        symbol_where_clause_for("input.security_code", symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}

fn read_boll_input_row_binary<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
    input_from: &str,
    input_to: &str,
) -> Result<Vec<u8>, FurnaceIoError> {
    if symbols.is_empty() && !all_symbols_requested {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    toString(trade_date),
    {}
FROM {}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date
FORMAT RowBinary",
        request.price_column,
        request.input_table,
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols, all_symbols_requested)
    );

    executor.query_bytes(&sql)
}

fn group_input_rows(input_bytes: &[u8]) -> Result<KdjInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let high_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        let low_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(KdjGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(KdjInput::new(
            trade_date.to_string(),
            high_price,
            low_price,
            close_price,
        ));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(KdjGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(KdjInputGroups { groups, input_rows })
}

fn group_ma_input_rows(input_bytes: &[u8]) -> Result<MaInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        let volume = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        if close_price.is_some() {
            valid_close_rows += 1;
        }
        if volume.is_some() {
            valid_volume_rows += 1;
        }

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(MaGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(MaInput::new(trade_date.to_string(), close_price, volume));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(MaGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(MaInputGroups {
        groups,
        input_rows,
        valid_close_rows,
        valid_volume_rows,
    })
}

fn group_rsi_input_rows(input_bytes: &[u8]) -> Result<RsiInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut valid_close_rows = 0;
    let mut input_from = None::<String>;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        input_from = match input_from {
            Some(current) if current.as_str() <= trade_date => Some(current),
            _ => Some(trade_date.to_string()),
        };
        if close_price.is_some() {
            valid_close_rows += 1;
        }

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(RsiGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(RsiInput::new(trade_date.to_string(), close_price));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(RsiGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(RsiInputGroups {
        groups,
        input_rows,
        valid_close_rows,
        input_from,
    })
}

fn group_boll_input_rows(input_bytes: &[u8]) -> Result<BollInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut input_valid_close_rows = 0;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        if close_price.is_some() {
            input_valid_close_rows += 1;
        }

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(BollGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(BollInput::new(trade_date.to_string(), close_price));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(BollGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(BollInputGroups {
        groups,
        input_rows,
        input_valid_close_rows,
    })
}

fn read_rowbinary_string<'a>(
    input: &'a [u8],
    cursor: &mut usize,
) -> Result<&'a str, FurnaceIoError> {
    let length = read_rowbinary_var_uint(input, cursor)?;
    let end = cursor
        .checked_add(length)
        .ok_or_else(|| FurnaceIoError::Parse("RowBinary string length overflow".to_string()))?;
    if end > input.len() {
        return Err(FurnaceIoError::Parse(
            "truncated RowBinary string field".to_string(),
        ));
    }
    let value = str::from_utf8(&input[*cursor..end])
        .map_err(|source| FurnaceIoError::Parse(format!("invalid RowBinary UTF-8: {source}")))?;
    *cursor = end;
    Ok(value)
}

fn read_rowbinary_var_uint(input: &[u8], cursor: &mut usize) -> Result<usize, FurnaceIoError> {
    let mut value = 0u64;
    let mut shift = 0;
    loop {
        if *cursor >= input.len() {
            return Err(FurnaceIoError::Parse(
                "truncated RowBinary VarUInt".to_string(),
            ));
        }
        let byte = input[*cursor];
        *cursor += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return usize::try_from(value)
                .map_err(|_| FurnaceIoError::Parse("RowBinary VarUInt too large".to_string()));
        }
        shift += 7;
        if shift >= 64 {
            return Err(FurnaceIoError::Parse(
                "RowBinary VarUInt exceeds u64".to_string(),
            ));
        }
    }
}

fn read_rowbinary_nullable_f64(
    input: &[u8],
    cursor: &mut usize,
) -> Result<Option<f64>, FurnaceIoError> {
    if *cursor >= input.len() {
        return Err(FurnaceIoError::Parse(
            "truncated RowBinary Nullable(Float64) marker".to_string(),
        ));
    }
    let is_null = input[*cursor];
    *cursor += 1;
    match is_null {
        0 => {
            let end = cursor
                .checked_add(8)
                .ok_or_else(|| FurnaceIoError::Parse("RowBinary Float64 overflow".to_string()))?;
            if end > input.len() {
                return Err(FurnaceIoError::Parse(
                    "truncated RowBinary Float64".to_string(),
                ));
            }
            let bytes = input[*cursor..end].try_into().map_err(|_| {
                FurnaceIoError::Parse("invalid RowBinary Float64 width".to_string())
            })?;
            *cursor = end;
            Ok(Some(f64::from_le_bytes(bytes)))
        }
        1 => Ok(None),
        other => Err(FurnaceIoError::Parse(format!(
            "invalid RowBinary Nullable(Float64) marker: {other}"
        ))),
    }
}

fn calculate_outputs(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: Vec<KdjGroupedInput>,
    input_row_count: usize,
    states: &HashMap<String, KdjState>,
    collect_rows: bool,
) -> Result<KdjCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    } else {
        calculate_grouped_outputs_serial_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    };
    if collect_rows {
        calculated.rows.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
    }
    Ok(KdjCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}

fn calculate_ma_outputs(
    request: &MaRunRequest,
    effective_output_to: &str,
    groups: Vec<MaGroupedInput>,
    input_row_count: usize,
    states: &HashMap<String, MaPreviousState>,
    collect_rows: bool,
) -> Result<MaCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_ma_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    } else {
        calculate_ma_grouped_outputs_serial_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    };
    if collect_rows {
        calculated.rows.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
    }
    Ok(MaCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        valid_close_rows: calculated.valid_close_rows,
        valid_volume_rows: calculated.valid_volume_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}

fn calculate_rsi_outputs(
    request: &RsiRunRequest,
    effective_output_to: &str,
    groups: Vec<RsiGroupedInput>,
    input_row_count: usize,
    states: &HashMap<String, RsiPreviousState>,
    collect_rows: bool,
) -> Result<RsiCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_rsi_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    } else {
        calculate_rsi_grouped_outputs_serial_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    };
    if collect_rows {
        calculated.rows.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
    }
    Ok(RsiCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        valid_close_rows: calculated.valid_close_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}

fn calculate_boll_outputs(
    request: &BollRunRequest,
    effective_output_to: &str,
    groups: Vec<BollGroupedInput>,
    input_row_count: usize,
    collect_rows: bool,
) -> Result<BollCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_boll_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            collect_rows,
        )?
    } else {
        calculate_boll_grouped_outputs_serial_with_collection(
            request,
            effective_output_to,
            &groups,
            collect_rows,
        )?
    };
    if collect_rows {
        calculated.rows.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
    }
    Ok(BollCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        output_valid_close_rows: calculated.output_valid_close_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}

fn should_parallelize(group_count: usize, input_row_count: usize, worker_threads: usize) -> bool {
    worker_threads > 1
        && group_count >= worker_threads.saturating_mul(2).max(2)
        && input_row_count > 0
}

fn calculate_boll_grouped_outputs_serial_with_collection(
    request: &BollRunRequest,
    effective_output_to: &str,
    groups: &[BollGroupedInput],
    collect_rows: bool,
) -> Result<BollSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut output_valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated =
            calculate_boll_security_outputs(request, effective_output_to, group, collect_rows)?;
        output_row_count += calculated.output_rows;
        output_valid_close_rows += calculated.output_valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(BollSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        output_valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_boll_grouped_outputs_parallel_with_collection(
    request: &BollRunRequest,
    effective_output_to: &str,
    groups: &[BollGroupedInput],
    collect_rows: bool,
) -> Result<BollSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_boll_security_outputs(request, effective_output_to, group, collect_rows)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut output_valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        output_valid_close_rows += calculated.output_valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(BollSecurityCalculation {
        rows,
        output_rows: output_row_count,
        output_valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_boll_security_outputs(
    request: &BollRunRequest,
    effective_output_to: &str,
    group: &BollGroupedInput,
    collect_rows: bool,
) -> Result<BollSecurityCalculation, FurnaceIoError> {
    let outputs = calculate_boll_series(&group.inputs, &request.params)
        .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut output_valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for (input, output) in group.inputs.iter().zip(outputs) {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if input.close_price.is_some() {
            output_valid_close_rows += 1;
        }
        if output.all_business_indicators_null() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(BollResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                boll_mid_10_1p5: output.boll_mid_10_1p5,
                boll_up_10_1p5: output.boll_up_10_1p5,
                boll_dn_10_1p5: output.boll_dn_10_1p5,
                boll_mid_20_2: output.boll_mid_20_2,
                boll_up_20_2: output.boll_up_20_2,
                boll_dn_20_2: output.boll_dn_20_2,
                boll_mid_50_2p5: output.boll_mid_50_2p5,
                boll_up_50_2p5: output.boll_up_50_2p5,
                boll_dn_50_2p5: output.boll_dn_50_2p5,
            });
        }
    }
    Ok(BollSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        output_valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_ma_grouped_outputs_serial_with_collection(
    request: &MaRunRequest,
    effective_output_to: &str,
    groups: &[MaGroupedInput],
    states: &HashMap<String, MaPreviousState>,
    collect_rows: bool,
) -> Result<MaSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated = calculate_ma_security_outputs(
            request,
            effective_output_to,
            states,
            group,
            collect_rows,
        )?;
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        valid_volume_rows += calculated.valid_volume_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(MaSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        valid_volume_rows,
        null_indicator_rows,
    })
}

fn calculate_ma_grouped_outputs_parallel_with_collection(
    request: &MaRunRequest,
    effective_output_to: &str,
    groups: &[MaGroupedInput],
    states: &HashMap<String, MaPreviousState>,
    collect_rows: bool,
) -> Result<MaSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_ma_security_outputs(request, effective_output_to, states, group, collect_rows)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        valid_volume_rows += calculated.valid_volume_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(MaSecurityCalculation {
        rows,
        output_rows: output_row_count,
        valid_close_rows,
        valid_volume_rows,
        null_indicator_rows,
    })
}

fn calculate_rsi_grouped_outputs_serial_with_collection(
    request: &RsiRunRequest,
    effective_output_to: &str,
    groups: &[RsiGroupedInput],
    states: &HashMap<String, RsiPreviousState>,
    collect_rows: bool,
) -> Result<RsiSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated = calculate_rsi_security_outputs(
            request,
            effective_output_to,
            states,
            group,
            collect_rows,
        )?;
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(RsiSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_rsi_grouped_outputs_parallel_with_collection(
    request: &RsiRunRequest,
    effective_output_to: &str,
    groups: &[RsiGroupedInput],
    states: &HashMap<String, RsiPreviousState>,
    collect_rows: bool,
) -> Result<RsiSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_rsi_security_outputs(
                request,
                effective_output_to,
                states,
                group,
                collect_rows,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(RsiSecurityCalculation {
        rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_rsi_security_outputs(
    request: &RsiRunRequest,
    effective_output_to: &str,
    states: &HashMap<String, RsiPreviousState>,
    group: &RsiGroupedInput,
    collect_rows: bool,
) -> Result<RsiSecurityCalculation, FurnaceIoError> {
    let previous_state = states.get(group.security_code.as_str()).cloned();
    let outputs =
        calculate_rsi_series_from_previous_state(&group.inputs, &request.params, previous_state)
            .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for (input, output) in group.inputs.iter().zip(outputs) {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if input.close_price.is_some() {
            valid_close_rows += 1;
        }
        if output.all_business_indicators_null() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(RsiResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                rsi_6: output.rsi_6,
                rsi_12: output.rsi_12,
                rsi_14: output.rsi_14,
                rsi_24: output.rsi_24,
                rsi_25: output.rsi_25,
                rsi_50: output.rsi_50,
                avg_gain_6_state: output.avg_gain_6_state,
                avg_loss_6_state: output.avg_loss_6_state,
                avg_gain_12_state: output.avg_gain_12_state,
                avg_loss_12_state: output.avg_loss_12_state,
                avg_gain_14_state: output.avg_gain_14_state,
                avg_loss_14_state: output.avg_loss_14_state,
                avg_gain_24_state: output.avg_gain_24_state,
                avg_loss_24_state: output.avg_loss_24_state,
                avg_gain_25_state: output.avg_gain_25_state,
                avg_loss_25_state: output.avg_loss_25_state,
                avg_gain_50_state: output.avg_gain_50_state,
                avg_loss_50_state: output.avg_loss_50_state,
            });
        }
    }
    Ok(RsiSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_ma_security_outputs(
    request: &MaRunRequest,
    effective_output_to: &str,
    states: &HashMap<String, MaPreviousState>,
    group: &MaGroupedInput,
    collect_rows: bool,
) -> Result<MaSecurityCalculation, FurnaceIoError> {
    let previous_state = states.get(group.security_code.as_str()).cloned();
    let outputs =
        calculate_ma_series_from_previous_state(&group.inputs, &request.params, previous_state)
            .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let mut null_indicator_rows = 0;
    for (input, output) in group.inputs.iter().zip(outputs) {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if input.close_price.is_some() {
            valid_close_rows += 1;
        }
        if input.volume.is_some() {
            valid_volume_rows += 1;
        }
        if output.all_business_indicators_null() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(MaResultRow {
                security_code: group.security_code.clone(),
                price_ma_3: output.price_ma(3),
                price_ma_5: output.price_ma(5),
                price_ma_6: output.price_ma(6),
                price_ma_10: output.price_ma(10),
                price_ma_12: output.price_ma(12),
                price_ma_14: output.price_ma(14),
                price_ma_20: output.price_ma(20),
                price_ma_24: output.price_ma(24),
                price_ma_28: output.price_ma(28),
                price_ma_57: output.price_ma(57),
                price_ma_60: output.price_ma(60),
                price_ma_114: output.price_ma(114),
                price_ma_250: output.price_ma(250),
                price_avg_ma_3_6_12_24: output.price_avg_ma_3_6_12_24,
                price_avg_ma_14_28_57_114: output.price_avg_ma_14_28_57_114,
                price_ema1_10_state: output.price_ema1_10_state,
                price_ema2_10: output.price_ema2_10,
                price_ema2_10_state: output.price_ema2_10_state,
                volume_ma_5: output.volume_ma(5),
                volume_ma_10: output.volume_ma(10),
                volume_ma_20: output.volume_ma(20),
                volume_ma_60: output.volume_ma(60),
                trade_date: output.trade_date,
            });
        }
    }
    Ok(MaSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        valid_volume_rows,
        null_indicator_rows,
    })
}

#[cfg(test)]
fn calculate_grouped_outputs_serial(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
) -> Result<Vec<KdjResultRow>, FurnaceIoError> {
    Ok(calculate_grouped_outputs_serial_with_collection(
        request,
        effective_output_to,
        groups,
        states,
        true,
    )?
    .rows)
}

fn calculate_grouped_outputs_serial_with_collection(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
    collect_rows: bool,
) -> Result<KdjSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated =
            calculate_security_outputs(request, effective_output_to, states, group, collect_rows)?;
        output_row_count += calculated.output_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(KdjSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        null_indicator_rows,
    })
}

#[cfg(test)]
fn calculate_grouped_outputs_parallel(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
) -> Result<Vec<KdjResultRow>, FurnaceIoError> {
    Ok(calculate_grouped_outputs_parallel_with_collection(
        request,
        effective_output_to,
        groups,
        states,
        true,
    )?
    .rows)
}

fn calculate_grouped_outputs_parallel_with_collection(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
    collect_rows: bool,
) -> Result<KdjSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_security_outputs(request, effective_output_to, states, group, collect_rows)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(KdjSecurityCalculation {
        rows,
        output_rows: output_row_count,
        null_indicator_rows,
    })
}

fn calculate_security_outputs(
    request: &KdjRunRequest,
    effective_output_to: &str,
    states: &HashMap<String, KdjState>,
    group: &KdjGroupedInput,
    collect_rows: bool,
) -> Result<KdjSecurityCalculation, FurnaceIoError> {
    let previous_state = states.get(group.security_code.as_str()).copied();
    let outputs = calculate_kdj_series(&group.inputs, request.params, previous_state)
        .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut null_indicator_rows = 0;
    for output in outputs {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if output.rsv.is_none() && output.k_value.is_none() && output.d_value.is_none() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(KdjResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                rsv_window: request.params.rsv_window,
                k_smoothing: request.params.k_smoothing,
                d_smoothing: request.params.d_smoothing,
                rsv: output.rsv,
                k_value: output.k_value,
                d_value: output.d_value,
                j_value: output.j_value,
            });
        }
    }
    Ok(KdjSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        null_indicator_rows,
    })
}

fn ensure_append_latest_is_safe<E: ClickHouseExecutor>(
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

fn ensure_ma_append_latest_is_safe<E: ClickHouseExecutor>(
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

fn ensure_rsi_append_latest_is_safe<E: ClickHouseExecutor>(
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

fn ensure_boll_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
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

fn retain_old_rows_for_staging<E: ClickHouseExecutor>(
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

fn retain_old_ma_rows_for_staging<E: ClickHouseExecutor>(
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

fn retain_old_rsi_rows_for_staging<E: ClickHouseExecutor>(
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

fn retain_old_boll_rows_for_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &BollRunRequest,
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

fn partition_year_fully_covered(year: u16, from: &str, to: &str) -> bool {
    let year_start = format!("{year}-01-01");
    let year_end = format!("{year}-12-31");
    from <= year_start.as_str() && to >= year_end.as_str()
}

fn validate_staging<E: ClickHouseExecutor>(
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

fn count_year_rows<E: ClickHouseExecutor>(
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

fn insert_result_rows<E: ClickHouseExecutor>(
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

fn insert_ma_result_rows<E: ClickHouseExecutor>(
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

fn insert_rsi_result_rows<E: ClickHouseExecutor>(
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

fn insert_boll_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[BollResultRow],
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
    boll_mid_10_1p5,
    boll_up_10_1p5,
    boll_dn_10_1p5,
    boll_mid_20_2,
    boll_up_20_2,
    boll_dn_20_2,
    boll_mid_50_2p5,
    boll_up_50_2p5,
    boll_dn_50_2p5
)
FORMAT RowBinary"
    );
    for batch in rows.chunks(batch_size) {
        let mut row_binary = Vec::with_capacity(batch.len().saturating_mul(105));
        for row in batch {
            row.write_row_binary(&mut row_binary)?;
        }
        executor.insert_bytes(&insert_sql, &row_binary)?;
    }
    Ok(())
}

impl KdjResultRow {
    fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        bytes.extend_from_slice(&self.rsv_window.to_le_bytes());
        bytes.extend_from_slice(&self.k_smoothing.to_le_bytes());
        bytes.extend_from_slice(&self.d_smoothing.to_le_bytes());
        push_rowbinary_nullable_f64(bytes, self.rsv);
        push_rowbinary_nullable_f64(bytes, self.k_value);
        push_rowbinary_nullable_f64(bytes, self.d_value);
        push_rowbinary_nullable_f64(bytes, self.j_value);
        Ok(())
    }
}

impl MaResultRow {
    fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_f64(bytes, self.price_ma_3);
        push_rowbinary_nullable_f64(bytes, self.price_ma_5);
        push_rowbinary_nullable_f64(bytes, self.price_ma_6);
        push_rowbinary_nullable_f64(bytes, self.price_ma_10);
        push_rowbinary_nullable_f64(bytes, self.price_ma_12);
        push_rowbinary_nullable_f64(bytes, self.price_ma_14);
        push_rowbinary_nullable_f64(bytes, self.price_ma_20);
        push_rowbinary_nullable_f64(bytes, self.price_ma_24);
        push_rowbinary_nullable_f64(bytes, self.price_ma_28);
        push_rowbinary_nullable_f64(bytes, self.price_ma_57);
        push_rowbinary_nullable_f64(bytes, self.price_ma_60);
        push_rowbinary_nullable_f64(bytes, self.price_ma_114);
        push_rowbinary_nullable_f64(bytes, self.price_ma_250);
        push_rowbinary_nullable_f64(bytes, self.price_avg_ma_3_6_12_24);
        push_rowbinary_nullable_f64(bytes, self.price_avg_ma_14_28_57_114);
        push_rowbinary_nullable_f64(bytes, self.price_ema1_10_state);
        push_rowbinary_nullable_f64(bytes, self.price_ema2_10);
        push_rowbinary_nullable_f64(bytes, self.price_ema2_10_state);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_5);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_10);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_20);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_60);
        Ok(())
    }
}

impl RsiResultRow {
    fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_f64(bytes, self.rsi_6);
        push_rowbinary_nullable_f64(bytes, self.rsi_12);
        push_rowbinary_nullable_f64(bytes, self.rsi_14);
        push_rowbinary_nullable_f64(bytes, self.rsi_24);
        push_rowbinary_nullable_f64(bytes, self.rsi_25);
        push_rowbinary_nullable_f64(bytes, self.rsi_50);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_6_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_6_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_12_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_12_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_14_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_14_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_24_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_24_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_25_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_25_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_50_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_50_state);
        Ok(())
    }
}

impl BollResultRow {
    fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_f64(bytes, self.boll_mid_10_1p5);
        push_rowbinary_nullable_f64(bytes, self.boll_up_10_1p5);
        push_rowbinary_nullable_f64(bytes, self.boll_dn_10_1p5);
        push_rowbinary_nullable_f64(bytes, self.boll_mid_20_2);
        push_rowbinary_nullable_f64(bytes, self.boll_up_20_2);
        push_rowbinary_nullable_f64(bytes, self.boll_dn_20_2);
        push_rowbinary_nullable_f64(bytes, self.boll_mid_50_2p5);
        push_rowbinary_nullable_f64(bytes, self.boll_up_50_2p5);
        push_rowbinary_nullable_f64(bytes, self.boll_dn_50_2p5);
        Ok(())
    }
}

fn push_rowbinary_string(bytes: &mut Vec<u8>, value: &str) {
    push_rowbinary_var_uint(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_rowbinary_var_uint(bytes: &mut Vec<u8>, mut value: usize) {
    while value >= 0x80 {
        bytes.push((value as u8) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
}

fn push_rowbinary_date(bytes: &mut Vec<u8>, value: &str) -> Result<(), FurnaceIoError> {
    let days = date_days_since_unix_epoch(value)?;
    bytes.extend_from_slice(&days.to_le_bytes());
    Ok(())
}

fn push_rowbinary_nullable_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(0);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        None => bytes.push(1),
    }
}

fn date_days_since_unix_epoch(value: &str) -> Result<u16, FurnaceIoError> {
    validate_date("date", value)?;
    let year = value[0..4]
        .parse::<i32>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date year: {value}")))?;
    let month = value[5..7]
        .parse::<u32>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date month: {value}")))?;
    let day = value[8..10]
        .parse::<u32>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date day: {value}")))?;
    let days = days_from_civil(year, month, day);
    u16::try_from(days).map_err(|_| FurnaceIoError::Parse(format!("Date out of range: {value}")))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i32;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn affected_years(from: &str, to: &str) -> Result<Vec<u16>, FurnaceIoError> {
    let from_year = parse_year(from)?;
    let to_year = parse_year(to)?;
    Ok((from_year..=to_year).collect())
}

fn parse_year(date: &str) -> Result<u16, FurnaceIoError> {
    validate_date("date", date)?;
    date[0..4]
        .parse::<u16>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date year: {date}")))
}

fn validate_date(name: &str, value: &str) -> Result<(), FurnaceIoError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[0..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must use YYYY-MM-DD format"
        )));
    }
    Ok(())
}

fn validate_table_name(name: &str, value: &str) -> Result<(), FurnaceIoError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must use database.table format"
        )));
    }
    for part in parts {
        validate_identifier(name, part)?;
    }
    Ok(())
}

fn validate_identifier(name: &str, value: &str) -> Result<(), FurnaceIoError> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must not be empty"
        )));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must start with an ASCII letter or underscore"
        )));
    }
    if !chars.all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must contain only ASCII letters, digits, or underscores"
        )));
    }
    Ok(())
}

fn parse_single_column_strings(output: &str) -> Result<Vec<String>, FurnaceIoError> {
    Ok(output
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn first_tsv_value(output: &str) -> Option<String> {
    output.lines().next().map(|line| line.trim().to_string())
}

fn parse_f64(value: &str) -> Result<Option<f64>, FurnaceIoError> {
    if value == "\\N" || value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|_| FurnaceIoError::Parse(format!("invalid Float64 value: {value}")))
}

fn parse_u64(value: &str) -> Result<u64, FurnaceIoError> {
    if value.is_empty() || value == "\\N" {
        return Ok(0);
    }
    value
        .parse::<u64>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid UInt64 value: {value}")))
}

fn symbol_where_clause(symbols: &[String], all_symbols_requested: bool) -> String {
    symbol_where_clause_for_column("security_code", symbols, all_symbols_requested)
}

fn symbol_where_clause_for_column(
    column: &str,
    symbols: &[String],
    all_symbols_requested: bool,
) -> String {
    if all_symbols_requested {
        return "1 = 1".to_string();
    }
    if symbols.is_empty() {
        return "1 = 0".to_string();
    }
    let values = symbols
        .iter()
        .map(|symbol| format!("'{}'", sql_string(symbol)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{column} IN ({values})")
}

fn symbol_where_clause_for(
    column: &str,
    symbols: &[String],
    all_symbols_requested: bool,
) -> String {
    symbol_where_clause_for_column(column, symbols, all_symbols_requested)
}

fn sql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn json_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json_string(value)),
        None => "null".to_string(),
    }
}

fn boll_configs_json() -> String {
    let configs = DEFAULT_BOLL_CONFIGS
        .iter()
        .map(|config| {
            format!(
                "{{\"window\":{},\"multiplier\":{},\"field_suffix\":\"{}\"}}",
                config.window,
                json_f64(config.multiplier),
                escape_json_string(config.field_suffix)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{configs}]")
}

fn json_f64(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "0".to_string()
    }
}

fn rows_per_second(rows: u64, elapsed: Duration) -> f64 {
    let seconds = elapsed.as_secs_f64();
    if seconds == 0.0 {
        return 0.0;
    }
    rows as f64 / seconds
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    type RowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>, Option<f64>, Option<f64>);
    type MaRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>, Option<f64>);
    type RsiRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>);
    type BollRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>);

    #[derive(Debug, Default)]
    struct FakeExecutor {
        queries: Vec<String>,
        multi_queries: Vec<Vec<String>>,
        inserts: Vec<(String, String)>,
        byte_inserts: Vec<(String, Vec<u8>)>,
        responses: Vec<String>,
        byte_responses: Vec<Vec<u8>>,
    }

    impl FakeExecutor {
        fn with_responses_and_bytes(responses: &[&str], byte_responses: Vec<Vec<u8>>) -> Self {
            Self {
                responses: responses.iter().map(ToString::to_string).collect(),
                byte_responses,
                ..Self::default()
            }
        }
    }

    impl ClickHouseExecutor for FakeExecutor {
        fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError> {
            self.queries.push(sql.to_string());
            if self.responses.is_empty() {
                return Ok(String::new());
            }
            Ok(self.responses.remove(0))
        }

        fn query_bytes(&mut self, sql: &str) -> Result<Vec<u8>, FurnaceIoError> {
            self.queries.push(sql.to_string());
            if self.byte_responses.is_empty() {
                return Ok(Vec::new());
            }
            Ok(self.byte_responses.remove(0))
        }

        fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError> {
            self.inserts.push((sql.to_string(), tsv.to_string()));
            Ok(())
        }

        fn insert_bytes(&mut self, sql: &str, bytes: &[u8]) -> Result<(), FurnaceIoError> {
            self.byte_inserts.push((sql.to_string(), bytes.to_vec()));
            Ok(())
        }

        fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError> {
            self.queries.push(sql.to_string());
            Ok(())
        }

        fn execute_many(&mut self, sqls: &[String]) -> Result<(), FurnaceIoError> {
            self.multi_queries.push(sqls.to_vec());
            Ok(())
        }
    }

    fn rowbinary_input_rows(rows: &[RowBinaryInputFixture<'_>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (security_code, trade_date, high_price, low_price, close_price) in rows {
            write_rowbinary_string(&mut bytes, security_code);
            write_rowbinary_string(&mut bytes, trade_date);
            write_rowbinary_nullable_f64(&mut bytes, *high_price);
            write_rowbinary_nullable_f64(&mut bytes, *low_price);
            write_rowbinary_nullable_f64(&mut bytes, *close_price);
        }
        bytes
    }

    fn ma_rowbinary_input_rows(rows: &[MaRowBinaryInputFixture<'_>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (security_code, trade_date, close_price, volume) in rows {
            write_rowbinary_string(&mut bytes, security_code);
            write_rowbinary_string(&mut bytes, trade_date);
            write_rowbinary_nullable_f64(&mut bytes, *close_price);
            write_rowbinary_nullable_f64(&mut bytes, *volume);
        }
        bytes
    }

    fn rsi_rowbinary_input_rows(rows: &[RsiRowBinaryInputFixture<'_>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (security_code, trade_date, close_price) in rows {
            write_rowbinary_string(&mut bytes, security_code);
            write_rowbinary_string(&mut bytes, trade_date);
            write_rowbinary_nullable_f64(&mut bytes, *close_price);
        }
        bytes
    }

    fn boll_rowbinary_input_rows(rows: &[BollRowBinaryInputFixture<'_>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (security_code, trade_date, close_price) in rows {
            write_rowbinary_string(&mut bytes, security_code);
            write_rowbinary_string(&mut bytes, trade_date);
            write_rowbinary_nullable_f64(&mut bytes, *close_price);
        }
        bytes
    }

    fn write_rowbinary_string(bytes: &mut Vec<u8>, value: &str) {
        write_rowbinary_var_uint(bytes, value.len());
        bytes.extend_from_slice(value.as_bytes());
    }

    fn write_rowbinary_var_uint(bytes: &mut Vec<u8>, mut value: usize) {
        while value >= 0x80 {
            bytes.push((value as u8) | 0x80);
            value >>= 7;
        }
        bytes.push(value as u8);
    }

    fn write_rowbinary_nullable_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
        match value {
            Some(value) => {
                bytes.push(0);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            None => bytes.push(1),
        }
    }

    #[test]
    fn create_kdj_output_table_sql_uses_year_partition_and_expected_order() {
        let sql = create_kdj_output_table_sql();

        assert!(sql.contains("PARTITION BY toYear(trade_date)"));
        assert!(sql.contains("ORDER BY (trade_date, security_code)"));
    }

    #[test]
    fn kdj_staging_table_name_normalizes_run_id() {
        let table_name = kdj_staging_table_name("RUN/2026-01-01");

        assert_eq!(
            table_name,
            "fleur_calculation.calc_stock_kdj_daily__staging__run_2026_01_01"
        );
    }

    #[test]
    fn replace_kdj_partition_sql_replaces_year_partition_from_staging() {
        let sql = replace_kdj_partition_sql("fleur_calculation.stage", 2026);

        assert_eq!(
            sql,
            "ALTER TABLE fleur_calculation.calc_stock_kdj_daily REPLACE PARTITION 2026 FROM fleur_calculation.stage"
        );
    }

    #[test]
    fn create_ma_output_table_sql_uses_canonical_fields() {
        let sql = create_ma_output_table_sql(DEFAULT_MA_OUTPUT_TABLE);

        assert!(sql.contains("price_ma_57 Nullable(Float64)"));
        assert!(sql.contains("price_avg_ma_14_28_57_114 Nullable(Float64)"));
        assert!(sql.contains("volume_ma_5 Nullable(Float64)"));
        assert!(!sql.contains("ma_47"));
        assert!(!sql.contains("price_ma57"));
        assert!(sql.contains("price_ema1_10_state Nullable(Float64)"));
        assert!(sql.contains("price_ema2_10_state Nullable(Float64)"));
        assert!(sql.contains("ORDER BY (trade_date, security_code)"));
    }

    #[test]
    fn create_rsi_output_table_sql_uses_canonical_fields() {
        let sql = create_rsi_output_table_sql(DEFAULT_RSI_OUTPUT_TABLE);

        assert!(sql.contains("rsi_6 Nullable(Float64)"));
        assert!(sql.contains("rsi_50 Nullable(Float64)"));
        assert!(sql.contains("avg_gain_50_state Nullable(Float64)"));
        assert!(sql.contains("avg_loss_50_state Nullable(Float64)"));
        assert!(sql.contains("ORDER BY (trade_date, security_code)"));
    }

    #[test]
    fn ma_staging_table_name_normalizes_run_id() {
        let table_name = ma_staging_table_name(DEFAULT_MA_OUTPUT_TABLE, "RUN/2026-01-01");

        assert_eq!(
            table_name,
            "fleur_calculation.calc_stock_ma_daily__staging__run_2026_01_01"
        );
    }

    #[test]
    fn replace_ma_partition_sql_uses_configurable_output_table() {
        let sql = replace_ma_partition_sql("db.calc_ma", "db.stage", 2026);

        assert_eq!(
            sql,
            "ALTER TABLE db.calc_ma REPLACE PARTITION 2026 FROM db.stage"
        );
    }

    #[test]
    fn create_boll_output_table_sql_uses_canonical_fields() {
        let sql = create_boll_output_table_sql(DEFAULT_BOLL_OUTPUT_TABLE);

        assert!(sql.contains("boll_mid_10_1p5 Nullable(Float64)"));
        assert!(sql.contains("boll_up_20_2 Nullable(Float64)"));
        assert!(sql.contains("boll_dn_50_2p5 Nullable(Float64)"));
        assert!(!sql.contains("boll_mid_n20_k2"));
        assert!(sql.contains("ORDER BY (trade_date, security_code)"));
    }

    #[test]
    fn run_kdj_dry_run_reads_inputs_and_computes_summary() {
        let responses = ["sh.600000\n", "2026-01-01\n", "1\n", ""];
        let input_rows = rowbinary_input_rows(&[
            ("sh.600000", "2026-01-01", Some(10.0), Some(8.0), Some(9.0)),
            ("sh.600000", "2026-01-02", Some(11.0), Some(8.0), Some(10.0)),
            ("sh.600000", "2026-01-03", Some(12.0), Some(8.0), Some(11.0)),
        ]);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = KdjRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-03".to_string(),
            params: KdjParams {
                rsv_window: 3,
                ..KdjParams::default()
            },
            ..KdjRunRequest::default()
        };

        let summary = run_kdj(&mut executor, &request).unwrap();

        assert_eq!(summary.input_rows, 3);
        assert_eq!(summary.output_rows, 3);
        assert_eq!(summary.null_indicator_rows, 2);
        assert!(summary.performance_metrics.input_rows_per_sec.is_finite());
        assert!(summary.to_json().contains("\"performance_metrics\""));
        assert!(executor.queries.iter().any(|query| {
            query.contains("AND 1 = 1\nORDER BY security_code, trade_date\nFORMAT RowBinary")
        }));
        assert!(!summary.writes_applied);
    }

    #[test]
    fn run_ma_dry_run_reads_close_inputs_and_computes_summary() {
        let responses = ["sh.600000\n", "2026-01-01\n", ""];
        let rows = (1..=20)
            .map(|day| {
                (
                    "sh.600000",
                    format!("2026-01-{day:02}"),
                    if day == 11 { None } else { Some(day as f64) },
                    if day == 12 {
                        None
                    } else {
                        Some((day * 100) as f64)
                    },
                )
            })
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close, volume)| {
                (*security_code, trade_date.as_str(), *close, *volume)
            })
            .collect::<Vec<_>>();
        let input_rows = ma_rowbinary_input_rows(&row_refs);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = MaRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-20".to_string(),
            ..MaRunRequest::default()
        };

        let summary = run_ma(&mut executor, &request).unwrap();

        assert_eq!(summary.input_rows, 20);
        assert_eq!(summary.output_rows, 20);
        assert_eq!(summary.valid_close_rows, 19);
        assert_eq!(summary.valid_volume_rows, 19);
        assert!(summary.null_indicator_rows > 0);
        assert_eq!(summary.ema_state_source, "full-history");
        assert!(summary.to_json().contains("\"indicator\":\"ma\""));
        assert!(
            summary
                .to_json()
                .contains("\"volume_ma_windows\":[5,10,20,60]")
        );
        assert!(executor.queries.iter().any(|query| {
            query.contains("close_price_forward_adj")
                && query.contains("CAST(unadj.volume, 'Nullable(Float64)')")
                && query.contains("ORDER BY adj.security_code, adj.trade_date")
                && query.contains("FORMAT RowBinary")
        }));
    }

    #[test]
    fn run_ma_with_previous_state_uses_per_security_valid_price_and_volume_lookback() {
        let responses = ["1\n", "sh.600000\t2026-01-10\t10\t9\n", "2025-01-01\n"];
        let rows = (1..=20)
            .map(|day| {
                (
                    "sh.600000",
                    format!("2026-01-{day:02}"),
                    Some(day as f64),
                    Some((day * 100) as f64),
                )
            })
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close, volume)| {
                (*security_code, trade_date.as_str(), *close, *volume)
            })
            .collect::<Vec<_>>();
        let input_rows = ma_rowbinary_input_rows(&row_refs);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = MaRunRequest {
            request_from: "2026-01-11".to_string(),
            request_to: "2026-01-20".to_string(),
            symbols: vec!["sh.600000".to_string()],
            output_table: "fleur_calculation.calc_stock_ma_daily_validation".to_string(),
            ..MaRunRequest::default()
        };

        let summary = run_ma(&mut executor, &request).unwrap();

        assert_eq!(summary.input_from, "2025-01-01");
        assert_eq!(summary.ema_state_source, "previous-state");
        let lookback_query = executor
            .queries
            .iter()
            .find(|query| query.contains("rn <= 250") && query.contains("rn <= 60"))
            .expect("MA lookback query should use explicit valid-row windows");
        assert!(lookback_query.contains("PARTITION BY security_code ORDER BY trade_date DESC"));
        assert!(lookback_query.contains("close_price_forward_adj IS NOT NULL"));
        assert!(
            lookback_query
                .contains("LEFT JOIN fleur_intermediate.int_stock_quotes_daily_unadj AS unadj")
        );
        assert!(lookback_query.contains("unadj.volume IS NOT NULL"));
    }

    #[test]
    fn run_rsi_dry_run_reads_close_inputs_and_computes_summary() {
        let responses = ["sh.600000\n", "2026-01-01\n", ""];
        let rows = (1..=51)
            .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
            .collect::<Vec<_>>();
        let input_rows = rsi_rowbinary_input_rows(&row_refs);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = RsiRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-51".to_string(),
            ..RsiRunRequest::default()
        };

        let summary = run_rsi(&mut executor, &request).unwrap();

        assert_eq!(summary.input_rows, 51);
        assert_eq!(summary.output_rows, 51);
        assert_eq!(summary.valid_close_rows, 51);
        assert!(summary.null_indicator_rows > 0);
        assert_eq!(summary.rsi_state_source, "full-history");
        assert!(summary.to_json().contains("\"indicator\":\"rsi\""));
        assert!(executor.queries.iter().any(|query| {
            query.contains("close_price_forward_adj")
                && query.contains("ORDER BY security_code, trade_date")
                && query.contains("FORMAT RowBinary")
        }));
    }

    #[test]
    fn parallel_ma_outputs_match_serial_outputs() {
        let request = MaRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-20".to_string(),
            ..MaRunRequest::default()
        };
        let groups = vec![
            MaGroupedInput {
                security_code: "sh.600000".to_string(),
                inputs: (1..=20)
                    .map(|day| {
                        MaInput::new(
                            format!("2026-01-{day:02}"),
                            Some(day as f64),
                            Some((day * 100) as f64),
                        )
                    })
                    .collect(),
            },
            MaGroupedInput {
                security_code: "sz.000001".to_string(),
                inputs: (1..=20)
                    .map(|day| {
                        MaInput::new(
                            format!("2026-01-{day:02}"),
                            Some((day + 20) as f64),
                            Some((day * 200) as f64),
                        )
                    })
                    .collect(),
            },
        ];

        let mut serial = calculate_ma_grouped_outputs_serial_with_collection(
            &request,
            "2026-01-20",
            &groups,
            &HashMap::new(),
            true,
        )
        .unwrap()
        .rows;
        let mut parallel = calculate_ma_grouped_outputs_parallel_with_collection(
            &request,
            "2026-01-20",
            &groups,
            &HashMap::new(),
            true,
        )
        .unwrap()
        .rows;
        serial.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
        parallel.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });

        assert_eq!(parallel, serial);
    }

    #[test]
    fn parallel_rsi_outputs_match_serial_outputs() {
        let request = RsiRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-20".to_string(),
            ..RsiRunRequest::default()
        };
        let groups = vec![
            RsiGroupedInput {
                security_code: "sh.600000".to_string(),
                inputs: (1..=20)
                    .map(|day| RsiInput::new(format!("2026-01-{day:02}"), Some(day as f64)))
                    .collect(),
            },
            RsiGroupedInput {
                security_code: "sz.000001".to_string(),
                inputs: (1..=20)
                    .map(|day| RsiInput::new(format!("2026-01-{day:02}"), Some((day + 20) as f64)))
                    .collect(),
            },
        ];

        let mut serial = calculate_rsi_grouped_outputs_serial_with_collection(
            &request,
            "2026-01-20",
            &groups,
            &HashMap::new(),
            true,
        )
        .unwrap()
        .rows;
        let mut parallel = calculate_rsi_grouped_outputs_parallel_with_collection(
            &request,
            "2026-01-20",
            &groups,
            &HashMap::new(),
            true,
        )
        .unwrap()
        .rows;
        serial.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
        parallel.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });

        assert_eq!(parallel, serial);
    }

    #[test]
    fn parallel_kdj_outputs_match_serial_outputs() {
        let request = KdjRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-04".to_string(),
            params: KdjParams {
                rsv_window: 3,
                ..KdjParams::default()
            },
            ..KdjRunRequest::default()
        };
        let groups = vec![
            KdjGroupedInput {
                security_code: "sh.600000".to_string(),
                inputs: vec![
                    KdjInput::new("2026-01-01".to_string(), Some(10.0), Some(8.0), Some(9.0)),
                    KdjInput::new("2026-01-02".to_string(), Some(11.0), Some(8.0), Some(10.0)),
                    KdjInput::new("2026-01-03".to_string(), Some(12.0), Some(8.0), Some(11.0)),
                    KdjInput::new("2026-01-04".to_string(), Some(13.0), Some(8.0), Some(12.0)),
                ],
            },
            KdjGroupedInput {
                security_code: "sz.000001".to_string(),
                inputs: vec![
                    KdjInput::new("2026-01-01".to_string(), Some(20.0), Some(18.0), Some(19.0)),
                    KdjInput::new("2026-01-02".to_string(), Some(21.0), Some(18.0), Some(20.0)),
                    KdjInput::new("2026-01-03".to_string(), Some(22.0), Some(18.0), Some(21.0)),
                    KdjInput::new("2026-01-04".to_string(), Some(23.0), Some(18.0), Some(22.0)),
                ],
            },
        ];
        let states = HashMap::from([("sz.000001".to_string(), KdjState::new(52.0, 48.0))]);

        let mut serial =
            calculate_grouped_outputs_serial(&request, "2026-01-04", &groups, &states).unwrap();
        let mut parallel =
            calculate_grouped_outputs_parallel(&request, "2026-01-04", &groups, &states).unwrap();
        serial.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
        parallel.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });

        assert_eq!(parallel, serial);
    }

    #[test]
    fn run_boll_dry_run_reads_close_inputs_and_computes_summary() {
        let responses = ["sh.600000\n", "2026-01-01\n"];
        let rows = (1..=20)
            .map(|day| {
                (
                    "sh.600000",
                    format!("2026-01-{day:02}"),
                    if day == 11 { None } else { Some(day as f64) },
                )
            })
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
            .collect::<Vec<_>>();
        let input_rows = boll_rowbinary_input_rows(&row_refs);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = BollRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-20".to_string(),
            ..BollRunRequest::default()
        };

        let summary = run_boll(&mut executor, &request).unwrap();

        assert_eq!(summary.input_rows, 20);
        assert_eq!(summary.output_rows, 20);
        assert_eq!(summary.input_valid_close_rows, 19);
        assert_eq!(summary.output_valid_close_rows, 19);
        assert!(summary.null_indicator_rows > 0);
        assert_eq!(summary.state_source, "rolling-lookback");
        assert!(summary.to_json().contains("\"indicator\":\"boll\""));
        assert!(summary.to_json().contains("\"stddev_ddof\":0"));
        assert!(summary.to_json().contains("\"field_suffix\":\"10_1p5\""));
        assert!(executor.queries.iter().any(|query| {
            query.contains("close_price_forward_adj")
                && query.contains("ORDER BY security_code, trade_date")
                && query.contains("FORMAT RowBinary")
        }));
    }

    #[test]
    fn parallel_boll_outputs_match_serial_outputs() {
        let request = BollRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-02-20".to_string(),
            ..BollRunRequest::default()
        };
        let groups = vec![
            BollGroupedInput {
                security_code: "sh.600000".to_string(),
                inputs: (1..=51)
                    .map(|day| BollInput::new(format!("2026-02-{day:02}"), Some(day as f64)))
                    .collect(),
            },
            BollGroupedInput {
                security_code: "sz.000001".to_string(),
                inputs: (1..=51)
                    .map(|day| BollInput::new(format!("2026-02-{day:02}"), Some((day + 20) as f64)))
                    .collect(),
            },
        ];

        let mut serial = calculate_boll_grouped_outputs_serial_with_collection(
            &request,
            "2026-02-20",
            &groups,
            true,
        )
        .unwrap()
        .rows;
        let mut parallel = calculate_boll_grouped_outputs_parallel_with_collection(
            &request,
            "2026-02-20",
            &groups,
            true,
        )
        .unwrap()
        .rows;
        serial.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
        parallel.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });

        assert_eq!(parallel, serial);
    }

    #[test]
    fn run_kdj_append_latest_inserts_result_rows() {
        let responses = ["2026-01-01\n", "1\n", "", "0\n"];
        let input_rows = rowbinary_input_rows(&[
            ("sh.600000", "2026-01-01", Some(10.0), Some(8.0), Some(9.0)),
            ("sh.600000", "2026-01-02", Some(11.0), Some(8.0), Some(10.0)),
            ("sh.600000", "2026-01-03", Some(12.0), Some(8.0), Some(11.0)),
        ]);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = KdjRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-03".to_string(),
            symbols: vec!["sh.600000".to_string()],
            mode: KdjWriteMode::AppendLatest,
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..KdjRunRequest::default()
        };

        let summary = run_kdj(&mut executor, &request).unwrap();

        assert!(summary.writes_applied);
        assert_eq!(executor.byte_inserts.len(), 1);
        assert!(executor.byte_inserts[0].0.contains("FORMAT RowBinary"));
        assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
    }

    #[test]
    fn run_ma_append_latest_inserts_result_rows() {
        let responses = ["2026-01-01\n", "", "0\n"];
        let rows = (1..=20)
            .map(|day| {
                (
                    "sh.600000",
                    format!("2026-01-{day:02}"),
                    Some(day as f64),
                    Some((day * 100) as f64),
                )
            })
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close, volume)| {
                (*security_code, trade_date.as_str(), *close, *volume)
            })
            .collect::<Vec<_>>();
        let input_rows = ma_rowbinary_input_rows(&row_refs);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = MaRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-20".to_string(),
            symbols: vec!["sh.600000".to_string()],
            mode: MaWriteMode::AppendLatest,
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..MaRunRequest::default()
        };

        let summary = run_ma(&mut executor, &request).unwrap();

        assert!(summary.writes_applied);
        assert_eq!(executor.byte_inserts.len(), 1);
        assert!(executor.byte_inserts[0].0.contains("calc_stock_ma_daily"));
        assert!(executor.byte_inserts[0].0.contains("price_ema2_10_state"));
        assert!(executor.byte_inserts[0].0.contains("volume_ma_5"));
        assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
    }

    #[test]
    fn run_rsi_append_latest_inserts_result_rows() {
        let responses = ["2026-01-01\n", "", "0\n"];
        let rows = (1..=51)
            .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
            .collect::<Vec<_>>();
        let input_rows = rsi_rowbinary_input_rows(&row_refs);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = RsiRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-51".to_string(),
            symbols: vec!["sh.600000".to_string()],
            mode: RsiWriteMode::AppendLatest,
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..RsiRunRequest::default()
        };

        let summary = run_rsi(&mut executor, &request).unwrap();

        assert!(summary.writes_applied);
        assert_eq!(executor.byte_inserts.len(), 1);
        assert!(executor.byte_inserts[0].0.contains("calc_stock_rsi_daily"));
        assert!(executor.byte_inserts[0].0.contains("avg_loss_50_state"));
        assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
    }

    #[test]
    fn run_rsi_append_latest_rejects_previous_state_gaps() {
        let responses = [
            "2026-01-01\n",
            "1\n",
            "sh.600000\t2026-01-10\t10\t0\t1\t0\t1\t0\t1\t0\t1\t0\t1\t0\t1\n",
            "1\t2026-01-11\n",
        ];
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, Vec::new());
        let request = RsiRunRequest {
            request_from: "2026-01-20".to_string(),
            request_to: "2026-01-21".to_string(),
            symbols: vec!["sh.600000".to_string()],
            mode: RsiWriteMode::AppendLatest,
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..RsiRunRequest::default()
        };

        let error = run_rsi(&mut executor, &request).unwrap_err();

        assert!(error.to_string().contains("RSI result gaps"));
        assert!(error.to_string().contains("2026-01-11"));
        assert!(executor.byte_inserts.is_empty());
        assert!(
            executor
                .queries
                .iter()
                .any(|query| query.contains("countDistinct(input.security_code)"))
        );
    }

    #[test]
    fn ma_result_row_writes_clickhouse_rowbinary_encoding() {
        let row = MaResultRow {
            security_code: "sh.600000".to_string(),
            trade_date: "2026-01-03".to_string(),
            price_ma_3: Some(1.0),
            price_ma_5: None,
            price_ma_6: None,
            price_ma_10: None,
            price_ma_12: None,
            price_ma_14: None,
            price_ma_20: None,
            price_ma_24: None,
            price_ma_28: None,
            price_ma_57: Some(57.0),
            price_ma_60: None,
            price_ma_114: None,
            price_ma_250: None,
            price_avg_ma_3_6_12_24: None,
            price_avg_ma_14_28_57_114: Some(2.0),
            price_ema1_10_state: Some(3.0),
            price_ema2_10: Some(4.0),
            price_ema2_10_state: Some(4.0),
            volume_ma_5: Some(5.0),
            volume_ma_10: None,
            volume_ma_20: None,
            volume_ma_60: None,
        };
        let mut bytes = Vec::new();

        row.write_row_binary(&mut bytes).unwrap();

        let mut cursor = 0;
        assert_eq!(
            read_rowbinary_string(&bytes, &mut cursor).unwrap(),
            "sh.600000"
        );
        cursor += 2;
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            Some(1.0)
        );
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            None
        );
        for _ in 0..7 {
            assert_eq!(
                read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
                None
            );
        }
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            Some(57.0)
        );
    }

    #[test]
    fn run_boll_append_latest_inserts_result_rows() {
        let responses = ["2026-01-01\n", "0\n"];
        let rows = (1..=20)
            .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
            .collect::<Vec<_>>();
        let input_rows = boll_rowbinary_input_rows(&row_refs);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = BollRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-20".to_string(),
            symbols: vec!["sh.600000".to_string()],
            mode: BollWriteMode::AppendLatest,
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..BollRunRequest::default()
        };

        let summary = run_boll(&mut executor, &request).unwrap();

        assert!(summary.writes_applied);
        assert_eq!(executor.byte_inserts.len(), 1);
        assert!(executor.byte_inserts[0].0.contains("calc_stock_boll_daily"));
        assert!(executor.byte_inserts[0].0.contains("boll_dn_50_2p5"));
        assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
    }

    #[test]
    fn boll_result_row_writes_clickhouse_rowbinary_encoding() {
        let row = BollResultRow {
            security_code: "sh.600000".to_string(),
            trade_date: "2026-01-03".to_string(),
            boll_mid_10_1p5: Some(1.0),
            boll_up_10_1p5: Some(2.0),
            boll_dn_10_1p5: Some(0.0),
            boll_mid_20_2: None,
            boll_up_20_2: None,
            boll_dn_20_2: None,
            boll_mid_50_2p5: Some(3.0),
            boll_up_50_2p5: Some(4.0),
            boll_dn_50_2p5: Some(5.0),
        };
        let mut bytes = Vec::new();

        row.write_row_binary(&mut bytes).unwrap();

        let mut cursor = 0;
        assert_eq!(
            read_rowbinary_string(&bytes, &mut cursor).unwrap(),
            "sh.600000"
        );
        cursor += 2;
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            Some(1.0)
        );
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            Some(2.0)
        );
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            Some(0.0)
        );
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            None
        );
    }

    #[test]
    fn kdj_result_row_writes_clickhouse_rowbinary_encoding() {
        let row = KdjResultRow {
            security_code: "sh.600000".to_string(),
            trade_date: "2026-01-03".to_string(),
            rsv_window: 9,
            k_smoothing: 3,
            d_smoothing: 3,
            rsv: None,
            k_value: Some(12.5),
            d_value: None,
            j_value: Some(1.25),
        };
        let mut bytes = Vec::new();

        row.write_row_binary(&mut bytes).unwrap();

        let mut expected = Vec::new();
        expected.push(9);
        expected.extend_from_slice(b"sh.600000");
        expected.extend_from_slice(&20_456_u16.to_le_bytes());
        expected.extend_from_slice(&9_u16.to_le_bytes());
        expected.extend_from_slice(&3_u16.to_le_bytes());
        expected.extend_from_slice(&3_u16.to_le_bytes());
        expected.push(1);
        expected.push(0);
        expected.extend_from_slice(&12.5_f64.to_le_bytes());
        expected.push(1);
        expected.push(0);
        expected.extend_from_slice(&1.25_f64.to_le_bytes());
        assert_eq!(bytes, expected);
    }

    #[test]
    fn retain_old_rows_skips_fully_covered_all_market_year_partitions() {
        let mut executor = FakeExecutor::default();
        let request = KdjRunRequest {
            request_from: "2020-01-01".to_string(),
            request_to: "2022-12-31".to_string(),
            mode: KdjWriteMode::ReplaceCascade,
            ..KdjRunRequest::default()
        };

        let retained = retain_old_rows_for_staging(
            &mut executor,
            &request,
            "fleur_calculation.stage",
            &[],
            true,
            &[2020, 2021, 2022],
            "2022-12-31",
        )
        .unwrap();

        assert_eq!(retained, 0);
        assert!(executor.queries.is_empty());
    }

    #[test]
    fn validate_staging_checks_all_years_with_one_query() {
        let mut executor = FakeExecutor::with_responses_and_bytes(&["0\n"], Vec::new());

        let summary = validate_staging(
            &mut executor,
            "fleur_calculation.stage",
            &[2020, 2021, 2022],
        )
        .unwrap();

        assert_eq!(summary, ValidationSummary::passed());
        assert_eq!(executor.queries.len(), 1);
        assert!(executor.queries[0].contains("toYear(trade_date) IN (2020,2021,2022)"));
    }

    #[test]
    fn run_kdj_replace_cascade_batches_partition_replace_statements() {
        let responses = [
            "2027-01-02\n",
            "2026-12-30\n",
            "1\n",
            "",
            "0\n",
            "0\n",
            "0\n",
        ];
        let input_rows = rowbinary_input_rows(&[
            ("sh.600000", "2026-12-30", Some(10.0), Some(8.0), Some(9.0)),
            ("sh.600000", "2026-12-31", Some(11.0), Some(8.0), Some(10.0)),
            ("sh.600000", "2027-01-01", Some(12.0), Some(8.0), Some(11.0)),
            ("sh.600000", "2027-01-02", Some(13.0), Some(8.0), Some(12.0)),
        ]);
        let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
        let request = KdjRunRequest {
            request_from: "2026-12-30".to_string(),
            request_to: "2026-12-31".to_string(),
            symbols: vec!["sh.600000".to_string()],
            run_id: Some("replace-batch-test".to_string()),
            mode: KdjWriteMode::ReplaceCascade,
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..KdjRunRequest::default()
        };

        let summary = run_kdj(&mut executor, &request).unwrap();

        assert_eq!(summary.partition_replace.years, vec![2026, 2027]);
        assert_eq!(executor.multi_queries.len(), 2);
        assert_eq!(executor.multi_queries[0].len(), 2);
        assert_eq!(executor.multi_queries[1].len(), 2);
        assert!(
            executor.multi_queries[1]
                .iter()
                .all(|sql| sql.contains("REPLACE PARTITION"))
        );
    }

    #[test]
    fn request_validation_rejects_small_production_batches() {
        let request = KdjRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-03".to_string(),
            mode: KdjWriteMode::AppendLatest,
            insert_batch_size: 10,
            ..KdjRunRequest::default()
        };

        let error = request.validate().unwrap_err();

        assert!(matches!(error, FurnaceIoError::InvalidRequest(_)));
    }

    #[test]
    fn ma_request_validation_rejects_non_canonical_price_column_for_writes() {
        let request = MaRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-03".to_string(),
            mode: MaWriteMode::AppendLatest,
            price_column: "close_price".to_string(),
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..MaRunRequest::default()
        };

        let error = request.validate().unwrap_err();

        assert!(matches!(error, FurnaceIoError::InvalidRequest(_)));
    }

    #[test]
    fn rsi_result_row_writes_clickhouse_rowbinary_encoding() {
        let row = RsiResultRow {
            security_code: "sh.600000".to_string(),
            trade_date: "2026-01-03".to_string(),
            rsi_6: Some(1.0),
            rsi_12: None,
            rsi_14: None,
            rsi_24: None,
            rsi_25: None,
            rsi_50: Some(50.0),
            avg_gain_6_state: Some(0.1),
            avg_loss_6_state: Some(0.2),
            avg_gain_12_state: None,
            avg_loss_12_state: None,
            avg_gain_14_state: None,
            avg_loss_14_state: None,
            avg_gain_24_state: None,
            avg_loss_24_state: None,
            avg_gain_25_state: None,
            avg_loss_25_state: None,
            avg_gain_50_state: Some(0.5),
            avg_loss_50_state: Some(0.6),
        };
        let mut bytes = Vec::new();

        row.write_row_binary(&mut bytes).unwrap();

        let mut cursor = 0;
        assert_eq!(
            read_rowbinary_string(&bytes, &mut cursor).unwrap(),
            "sh.600000"
        );
        cursor += 2;
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            Some(1.0)
        );
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            None
        );
    }
}
