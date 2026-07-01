use std::time::Duration;

use furnace_core::KdjInput;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KdjResultRow {
    pub(crate) security_code: String,
    pub(crate) trade_date: String,
    pub(crate) rsv_window: u16,
    pub(crate) k_smoothing: u16,
    pub(crate) d_smoothing: u16,
    pub(crate) rsv: Option<f64>,
    pub(crate) k_value: Option<f64>,
    pub(crate) d_value: Option<f64>,
    pub(crate) j_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KdjCalculationResult {
    pub(crate) rows: Vec<KdjResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) null_indicator_rows: u64,
    pub(crate) compute_elapsed: Duration,
    pub(crate) parallelism: &'static str,
    pub(crate) worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KdjInputGroups {
    pub(crate) groups: Vec<KdjGroupedInput>,
    pub(crate) input_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KdjGroupedInput {
    pub(crate) security_code: String,
    pub(crate) inputs: Vec<KdjInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KdjSecurityCalculation {
    pub(crate) rows: Vec<KdjResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) null_indicator_rows: u64,
}
