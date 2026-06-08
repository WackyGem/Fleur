//! 可跨指标复用的时间序列基础算子。

pub mod ema;
pub mod sma;

pub use ema::{EmaState, SmaSeededEma, calculate_sma_seeded_ema_series};
pub use sma::{RollingSma, calculate_sma_series};
