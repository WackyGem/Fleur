//! Furnace 计算引擎的纯金融指标计算库。
//!
//! 本 crate 只提供确定性、可复用的指标函数；不连接 ClickHouse，不读取环境变量，
//! 也不依赖 Dagster/dbt 的运行时概念。
//!
//! # 示例
//!
//! ```
//! use furnace_core::{calculate_kdj_series, KdjInput, KdjParams};
//!
//! let inputs = vec![
//!     KdjInput::new("2026-01-01", Some(10.0), Some(8.0), Some(9.0)),
//!     KdjInput::new("2026-01-02", Some(11.0), Some(8.5), Some(10.0)),
//!     KdjInput::new("2026-01-03", Some(12.0), Some(9.0), Some(11.0)),
//! ];
//! let params = KdjParams { rsv_window: 3, ..KdjParams::default() };
//!
//! let outputs = calculate_kdj_series(&inputs, params, None).unwrap();
//! assert!(outputs[0].rsv.is_none());
//! assert!(outputs[2].k_value.is_some());
//! ```

pub mod indicators;
pub mod operators;

pub use indicators::bollinger_bands::{
    BollBand, BollConfig, BollError, BollInput, BollOutput, BollParams, DEFAULT_BOLL_CONFIGS,
    DEFAULT_BOLL_MAX_WINDOW, DEFAULT_BOLL_STDDEV_DDOF, calculate_boll_series,
};
pub use indicators::kdj::{
    DEFAULT_D_SMOOTHING, DEFAULT_INITIAL_D, DEFAULT_INITIAL_K, DEFAULT_K_SMOOTHING,
    DEFAULT_RSV_WINDOW, KdjError, KdjInput, KdjOutput, KdjParams, KdjState, PriceBar,
    calculate_kdj_next, calculate_kdj_series,
};
pub use indicators::macd::{
    DEFAULT_MACD_FAST_WINDOW, DEFAULT_MACD_SIGNAL_WINDOW, DEFAULT_MACD_SLOW_WINDOW, MacdError,
    MacdInput, MacdOutput, MacdOutputValues, MacdParams, MacdPreviousState, MacdState,
    calculate_macd, calculate_macd_series, calculate_macd_series_from_previous_state,
    visit_macd_series_from_previous_state,
};
pub use indicators::moving_average::{
    DEFAULT_EMA_WINDOW, DEFAULT_PRICE_MA_WINDOWS, DEFAULT_VOLUME_MA_WINDOWS, MaError, MaInput,
    MaOutput, MaParams, MaPreviousState, MaState, calculate_ma_series,
    calculate_ma_series_from_previous_state,
};
pub use indicators::price_pattern::{
    DEFAULT_N_STRUCTURE_WINDOW, PricePatternError, PricePatternInput, PricePatternOutput,
    PricePatternParams, PricePatternPreviousState, PricePatternState, StructurePriceBar,
    calculate_price_pattern_series,
};
pub use indicators::rsi::{
    DEFAULT_RSI_WINDOWS, RsiError, RsiInput, RsiOutput, RsiParams, RsiPreviousState, RsiState,
    RsiWindowState, calculate_rsi_series, calculate_rsi_series_from_previous_state,
};
pub use operators::{
    EmaState, RollingMeanStdDev, RollingSma, RollingStdDev, SmaSeededEma,
    calculate_sma_seeded_ema_series, calculate_sma_series, calculate_stddev_series,
};
