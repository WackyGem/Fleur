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
    DEFAULT_EMA_WINDOW, DEFAULT_MA_WINDOWS, KdjInput, KdjParams, KdjState, MaInput, MaParams,
    MaPreviousState, MaState, calculate_kdj_series, calculate_ma_series_from_previous_state,
};
use rayon::prelude::*;

/// 默认的 dbt 中间层输入表，存放前复权日行情价格。
pub const DEFAULT_INPUT_TABLE: &str = "fleur_intermediate.int_stock_quotes_daily_adj";

/// Furnace 负责写入的日频 KDJ 计算结果表。
pub const DEFAULT_KDJ_OUTPUT_TABLE: &str = "fleur_calculation.calc_stock_kdj_daily";

/// Furnace 负责写入的日频 Moving Average 计算结果表。
pub const DEFAULT_MA_OUTPUT_TABLE: &str = "fleur_calculation.calc_stock_ma_daily";

/// Moving Average 第一版使用的 canonical 前复权收盘价字段。
pub const DEFAULT_MA_PRICE_COLUMN: &str = "close_price_forward_adj";

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
    ma_3 Nullable(Float64),
    ma_5 Nullable(Float64),
    ma_6 Nullable(Float64),
    ma_10 Nullable(Float64),
    ma_12 Nullable(Float64),
    ma_14 Nullable(Float64),
    ma_20 Nullable(Float64),
    ma_24 Nullable(Float64),
    ma_28 Nullable(Float64),
    ma_57 Nullable(Float64),
    ma_60 Nullable(Float64),
    ma_114 Nullable(Float64),
    ma_250 Nullable(Float64),
    avg_ma_3_6_12_24 Nullable(Float64),
    avg_ma_14_28_57_114 Nullable(Float64),
    ema1_10_state Nullable(Float64),
    ema2_10 Nullable(Float64),
    ema2_10_state Nullable(Float64)
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
    /// 输出表。
    pub output_table: String,
    /// close 输入字段名。
    pub price_column: String,
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
        if self.mode.writes_applied() && self.price_column != DEFAULT_MA_PRICE_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MA writes only allow price column {DEFAULT_MA_PRICE_COLUMN}"
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
            output_table: DEFAULT_MA_OUTPUT_TABLE.to_string(),
            price_column: DEFAULT_MA_PRICE_COLUMN.to_string(),
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
        let ma_windows = DEFAULT_MA_WINDOWS
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"indicator\":\"ma\",\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"valid_close_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"ma_windows\":[{}],\"ema_window\":{},\"ema_state_source\":\"{}\",\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
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
            ma_windows,
            DEFAULT_EMA_WINDOW,
            escape_json_string(&self.ema_state_source),
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
    ma_3: Option<f64>,
    ma_5: Option<f64>,
    ma_6: Option<f64>,
    ma_10: Option<f64>,
    ma_12: Option<f64>,
    ma_14: Option<f64>,
    ma_20: Option<f64>,
    ma_24: Option<f64>,
    ma_28: Option<f64>,
    ma_57: Option<f64>,
    ma_60: Option<f64>,
    ma_114: Option<f64>,
    ma_250: Option<f64>,
    avg_ma_3_6_12_24: Option<f64>,
    avg_ma_14_28_57_114: Option<f64>,
    ema1_10_state: Option<f64>,
    ema2_10: Option<f64>,
    ema2_10_state: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct MaCalculationResult {
    rows: Vec<MaResultRow>,
    output_rows: u64,
    valid_close_rows: u64,
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
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        ema_state_source: if can_use_previous_state {
            if missing_state_symbols.is_empty() {
                format!("previous-state:{}", ma_states.len())
            } else {
                format!(
                    "mixed:previous-state:{},full-history:{}",
                    ma_states.len(),
                    missing_state_symbols.len()
                )
            }
        } else {
            "full-history".to_string()
        },
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
SELECT security_code, toString(trade_date), ema1_10_state, ema2_10_state
FROM (
    SELECT
        security_code,
        trade_date,
        ema1_10_state,
        ema2_10_state,
        row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
    FROM {}
    WHERE trade_date < toDate('{}')
      AND ema1_10_state IS NOT NULL
      AND ema2_10_state IS NOT NULL
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
            FurnaceIoError::Parse("previous ema1_10_state must not be null".to_string())
        })?;
        let ema2 = parse_f64(fields[3])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous ema2_10_state must not be null".to_string())
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

fn resolve_ma_lookback_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &MaRunRequest,
    symbols: &[String],
    all_symbols_requested: bool,
) -> Result<String, FurnaceIoError> {
    let symbol_filter = symbol_where_clause(symbols, all_symbols_requested);
    let lookback_window = DEFAULT_MA_WINDOWS.iter().copied().max().unwrap_or(250);
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM (
    SELECT trade_date
    FROM {}
    WHERE trade_date <= toDate('{}')
      AND {symbol_filter}
    GROUP BY trade_date
    ORDER BY trade_date DESC
    LIMIT {lookback_window}
)
FORMAT TSV",
        request.input_table,
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
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        if close_price.is_some() {
            valid_close_rows += 1;
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

        current_inputs.push(MaInput::new(trade_date.to_string(), close_price));
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
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(MaSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
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
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(MaSecurityCalculation {
        rows,
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
            output_rows.push(MaResultRow {
                security_code: group.security_code.clone(),
                ma_3: output.ma(3),
                ma_5: output.ma(5),
                ma_6: output.ma(6),
                ma_10: output.ma(10),
                ma_12: output.ma(12),
                ma_14: output.ma(14),
                ma_20: output.ma(20),
                ma_24: output.ma(24),
                ma_28: output.ma(28),
                ma_57: output.ma(57),
                ma_60: output.ma(60),
                ma_114: output.ma(114),
                ma_250: output.ma(250),
                avg_ma_3_6_12_24: output.avg_ma_3_6_12_24,
                avg_ma_14_28_57_114: output.avg_ma_14_28_57_114,
                ema1_10_state: output.ema1_10_state,
                ema2_10: output.ema2_10,
                ema2_10_state: output.ema2_10_state,
                trade_date: output.trade_date,
            });
        }
    }
    Ok(MaSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
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
    ma_3,
    ma_5,
    ma_6,
    ma_10,
    ma_12,
    ma_14,
    ma_20,
    ma_24,
    ma_28,
    ma_57,
    ma_60,
    ma_114,
    ma_250,
    avg_ma_3_6_12_24,
    avg_ma_14_28_57_114,
    ema1_10_state,
    ema2_10,
    ema2_10_state
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
        push_rowbinary_nullable_f64(bytes, self.ma_3);
        push_rowbinary_nullable_f64(bytes, self.ma_5);
        push_rowbinary_nullable_f64(bytes, self.ma_6);
        push_rowbinary_nullable_f64(bytes, self.ma_10);
        push_rowbinary_nullable_f64(bytes, self.ma_12);
        push_rowbinary_nullable_f64(bytes, self.ma_14);
        push_rowbinary_nullable_f64(bytes, self.ma_20);
        push_rowbinary_nullable_f64(bytes, self.ma_24);
        push_rowbinary_nullable_f64(bytes, self.ma_28);
        push_rowbinary_nullable_f64(bytes, self.ma_57);
        push_rowbinary_nullable_f64(bytes, self.ma_60);
        push_rowbinary_nullable_f64(bytes, self.ma_114);
        push_rowbinary_nullable_f64(bytes, self.ma_250);
        push_rowbinary_nullable_f64(bytes, self.avg_ma_3_6_12_24);
        push_rowbinary_nullable_f64(bytes, self.avg_ma_14_28_57_114);
        push_rowbinary_nullable_f64(bytes, self.ema1_10_state);
        push_rowbinary_nullable_f64(bytes, self.ema2_10);
        push_rowbinary_nullable_f64(bytes, self.ema2_10_state);
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
    format!("security_code IN ({values})")
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
    type MaRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>);

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

        assert!(sql.contains("ma_57 Nullable(Float64)"));
        assert!(sql.contains("avg_ma_14_28_57_114 Nullable(Float64)"));
        assert!(!sql.contains("ma_47"));
        assert!(sql.contains("ema1_10_state Nullable(Float64)"));
        assert!(sql.contains("ema2_10_state Nullable(Float64)"));
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
                )
            })
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
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
        assert!(summary.null_indicator_rows > 0);
        assert_eq!(summary.ema_state_source, "full-history");
        assert!(summary.to_json().contains("\"indicator\":\"ma\""));
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
                    .map(|day| MaInput::new(format!("2026-01-{day:02}"), Some(day as f64)))
                    .collect(),
            },
            MaGroupedInput {
                security_code: "sz.000001".to_string(),
                inputs: (1..=20)
                    .map(|day| MaInput::new(format!("2026-01-{day:02}"), Some((day + 20) as f64)))
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
            .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
            .collect::<Vec<_>>();
        let row_refs = rows
            .iter()
            .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
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
        assert!(executor.byte_inserts[0].0.contains("ema2_10_state"));
        assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
    }

    #[test]
    fn ma_result_row_writes_clickhouse_rowbinary_encoding() {
        let row = MaResultRow {
            security_code: "sh.600000".to_string(),
            trade_date: "2026-01-03".to_string(),
            ma_3: Some(1.0),
            ma_5: None,
            ma_6: None,
            ma_10: None,
            ma_12: None,
            ma_14: None,
            ma_20: None,
            ma_24: None,
            ma_28: None,
            ma_57: Some(57.0),
            ma_60: None,
            ma_114: None,
            ma_250: None,
            avg_ma_3_6_12_24: None,
            avg_ma_14_28_57_114: Some(2.0),
            ema1_10_state: Some(3.0),
            ema2_10: Some(4.0),
            ema2_10_state: Some(4.0),
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
}
