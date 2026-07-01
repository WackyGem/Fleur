use std::time::Duration;

use furnace_core::MaInput;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MaResultRow {
    pub(crate) security_code: String,
    pub(crate) trade_date: String,
    pub(crate) price_ma_3: Option<f64>,
    pub(crate) price_ma_5: Option<f64>,
    pub(crate) price_ma_6: Option<f64>,
    pub(crate) price_ma_10: Option<f64>,
    pub(crate) price_ma_12: Option<f64>,
    pub(crate) price_ma_14: Option<f64>,
    pub(crate) price_ma_20: Option<f64>,
    pub(crate) price_ma_24: Option<f64>,
    pub(crate) price_ma_28: Option<f64>,
    pub(crate) price_ma_30: Option<f64>,
    pub(crate) price_ma_57: Option<f64>,
    pub(crate) price_ma_60: Option<f64>,
    pub(crate) price_ma_114: Option<f64>,
    pub(crate) price_ma_250: Option<f64>,
    pub(crate) price_avg_ma_3_6_12_24: Option<f64>,
    pub(crate) price_avg_ma_14_28_57_114: Option<f64>,
    pub(crate) price_ema1_10_state: Option<f64>,
    pub(crate) price_ema2_10: Option<f64>,
    pub(crate) price_ema2_10_state: Option<f64>,
    pub(crate) volume_ma_5: Option<f64>,
    pub(crate) volume_ma_10: Option<f64>,
    pub(crate) volume_ma_20: Option<f64>,
    pub(crate) volume_ma_60: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MaCalculationResult {
    pub(crate) rows: Vec<MaResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) valid_volume_rows: u64,
    pub(crate) null_indicator_rows: u64,
    pub(crate) compute_elapsed: Duration,
    pub(crate) parallelism: &'static str,
    pub(crate) worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MaInputGroups {
    pub(crate) groups: Vec<MaGroupedInput>,
    pub(crate) input_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) valid_volume_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MaGroupedInput {
    pub(crate) security_code: String,
    pub(crate) inputs: Vec<MaInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MaSecurityCalculation {
    pub(crate) rows: Vec<MaResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) valid_volume_rows: u64,
    pub(crate) null_indicator_rows: u64,
}
