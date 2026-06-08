use std::error::Error;
use std::fmt;

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
