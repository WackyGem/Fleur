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

pub use indicators::kdj::{
    DEFAULT_D_SMOOTHING, DEFAULT_INITIAL_D, DEFAULT_INITIAL_K, DEFAULT_K_SMOOTHING,
    DEFAULT_RSV_WINDOW, KdjError, KdjInput, KdjOutput, KdjParams, KdjState, PriceBar,
    calculate_kdj_next, calculate_kdj_series,
};
pub use indicators::moving_average::{
    DEFAULT_EMA_WINDOW, DEFAULT_MA_WINDOWS, MaError, MaInput, MaOutput, MaParams, MaPreviousState,
    MaState, calculate_ma_series, calculate_ma_series_from_previous_state,
};
pub use operators::{
    EmaState, RollingSma, SmaSeededEma, calculate_sma_seeded_ema_series, calculate_sma_series,
};
