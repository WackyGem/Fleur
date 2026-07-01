/// Furnace I/O 返回的错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FurnaceIoError {
    /// 请求无法安全执行。
    #[error("{0}")]
    InvalidRequest(String),
    /// ClickHouse 查询或写入执行失败。
    #[error("{message}{details_suffix}", details_suffix = details.as_ref().map(|value| format!(": {value}")).unwrap_or_default())]
    ClickHouseCommand {
        /// 错误摘要。
        message: String,
        /// 可选的底层来源细节。
        details: Option<String>,
    },
    /// ClickHouse HTTP client 配置无效。
    #[error("{0}")]
    Config(String),
    /// 日期和 ClickHouse Date 类型之间转换失败。
    #[error("{0}")]
    DateConversion(String),
    /// 无法解析 ClickHouse 输出。
    #[error("{0}")]
    Parse(String),
    /// 指标计算失败。
    #[error("{0}")]
    Compute(String),
}

impl From<clickhouse::error::Error> for FurnaceIoError {
    fn from(source: clickhouse::error::Error) -> Self {
        Self::ClickHouseCommand {
            message: "ClickHouse HTTP client error".to_string(),
            details: Some(source.to_string()),
        }
    }
}
