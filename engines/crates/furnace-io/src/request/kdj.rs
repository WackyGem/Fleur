use furnace_core::KdjParams;

use crate::FurnaceIoError;
use crate::schema::{DEFAULT_INSERT_BATCH_SIZE, MIN_INSERT_BATCH_SIZE};
use crate::validation::validate_date;

/// CLI 或 Dagster 请求的 KDJ 写入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdjWriteMode {
    /// 只计算并汇总结果，不写入 ClickHouse。
    DryRun,
    /// 当目标表不存在同日或更晚结果时，追加最新区间。
    AppendLatest,
    /// 重算历史区间，并级联到受影响的最新输入日期。
    ReplaceCascade,
    /// 删除并重建输出表，再写入本次全量计算结果。
    RebuildTable,
}

impl KdjWriteMode {
    /// 解析该模式在 CLI 中使用的拼写。
    ///
    /// # 错误
    ///
    /// 当 `value` 不是 `dry-run`、`append-latest`、`replace-cascade` 或 `rebuild-table` 时，
    /// 返回 [`FurnaceIoError::InvalidRequest`]。
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            "rebuild-table" => Ok(Self::RebuildTable),
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
            Self::RebuildTable => "rebuild-table",
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
