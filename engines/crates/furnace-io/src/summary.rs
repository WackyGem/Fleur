use furnace_core::{
    DEFAULT_BOLL_MAX_WINDOW, DEFAULT_BOLL_STDDEV_DDOF, DEFAULT_EMA_WINDOW,
    DEFAULT_MACD_FAST_WINDOW, DEFAULT_MACD_SIGNAL_WINDOW, DEFAULT_MACD_SLOW_WINDOW,
    DEFAULT_PRICE_MA_WINDOWS, DEFAULT_RSI_WINDOWS, DEFAULT_VOLUME_MA_WINDOWS, KdjParams,
};

use crate::{
    BollWriteMode, KdjWriteMode, MaWriteMode, MacdWriteMode, PricePatternWriteMode, RsiWriteMode,
};

mod boll;
mod common;
mod kdj;
mod ma;
mod macd;
mod price_pattern;
mod rsi;

use common::*;

pub use boll::BollRunSummary;
pub use common::{PartitionReplaceSummary, PerformanceMetrics, ValidationSummary};
pub use kdj::KdjRunSummary;
pub use ma::MaRunSummary;
pub use macd::MacdRunSummary;
pub use price_pattern::PricePatternRunSummary;
pub use rsi::RsiRunSummary;

pub(crate) use common::{PerformanceTimings, time_result};
