use furnace_core::RsiParams;

use crate::FurnaceIoError;
use crate::schema::{
    DEFAULT_INPUT_TABLE, DEFAULT_INSERT_BATCH_SIZE, DEFAULT_RSI_OUTPUT_TABLE,
    DEFAULT_RSI_PRICE_COLUMN, MIN_INSERT_BATCH_SIZE,
};
use crate::validation::{validate_date, validate_identifier, validate_table_name};

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
