use std::time::Duration;

use furnace_core::RsiInput;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RsiResultRow {
    pub(crate) security_code: String,
    pub(crate) trade_date: String,
    pub(crate) rsi_6: Option<f64>,
    pub(crate) rsi_12: Option<f64>,
    pub(crate) rsi_14: Option<f64>,
    pub(crate) rsi_24: Option<f64>,
    pub(crate) rsi_25: Option<f64>,
    pub(crate) rsi_50: Option<f64>,
    pub(crate) avg_gain_6_state: Option<f64>,
    pub(crate) avg_loss_6_state: Option<f64>,
    pub(crate) avg_gain_12_state: Option<f64>,
    pub(crate) avg_loss_12_state: Option<f64>,
    pub(crate) avg_gain_14_state: Option<f64>,
    pub(crate) avg_loss_14_state: Option<f64>,
    pub(crate) avg_gain_24_state: Option<f64>,
    pub(crate) avg_loss_24_state: Option<f64>,
    pub(crate) avg_gain_25_state: Option<f64>,
    pub(crate) avg_loss_25_state: Option<f64>,
    pub(crate) avg_gain_50_state: Option<f64>,
    pub(crate) avg_loss_50_state: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RsiCalculationResult {
    pub(crate) rows: Vec<RsiResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) null_indicator_rows: u64,
    pub(crate) compute_elapsed: Duration,
    pub(crate) parallelism: &'static str,
    pub(crate) worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RsiInputGroups {
    pub(crate) groups: Vec<RsiGroupedInput>,
    pub(crate) input_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) input_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RsiGroupedInput {
    pub(crate) security_code: String,
    pub(crate) inputs: Vec<RsiInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RsiSecurityCalculation {
    pub(crate) rows: Vec<RsiResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) null_indicator_rows: u64,
}
