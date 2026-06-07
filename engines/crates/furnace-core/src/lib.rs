//! Pure financial indicator calculations for the Furnace compute engine.
//!
//! This crate contains deterministic, reusable indicator functions. It does
//! not connect to ClickHouse, read environment variables, or depend on
//! Dagster/dbt runtime concepts.
//!
//! # Examples
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

pub use indicators::kdj::{
    DEFAULT_D_SMOOTHING, DEFAULT_INITIAL_D, DEFAULT_INITIAL_K, DEFAULT_K_SMOOTHING,
    DEFAULT_RSV_WINDOW, KdjError, KdjInput, KdjOutput, KdjParams, KdjState, PriceBar,
    calculate_kdj_next, calculate_kdj_series,
};
