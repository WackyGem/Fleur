use std::time::Duration;

use furnace_core::PricePatternInput;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PricePatternResultRow {
    pub(crate) security_code: String,
    pub(crate) trade_date: String,
    pub(crate) close_direction: Option<i8>,
    pub(crate) close_up_streak_days: Option<u16>,
    pub(crate) close_down_streak_days: Option<u16>,
    pub(crate) n_structure_20_valid_bars: u16,
    pub(crate) n_structure_20_high_date: Option<String>,
    pub(crate) n_structure_20_high_price: Option<f64>,
    pub(crate) n_structure_20_low_date: Option<String>,
    pub(crate) n_structure_20_low_price: Option<f64>,
    pub(crate) n_structure_20_second_low_date: Option<String>,
    pub(crate) n_structure_20_second_low_price: Option<f64>,
    pub(crate) n_structure_20_second_low_ratio: Option<f64>,
    pub(crate) n_structure_20_is_valid: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PricePatternCalculationResult {
    pub(crate) rows: Vec<PricePatternResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_streak_rows: u64,
    pub(crate) valid_structure_bar_rows: u64,
    pub(crate) null_streak_rows: u64,
    pub(crate) null_second_low_rows: u64,
    pub(crate) compute_elapsed: Duration,
    pub(crate) parallelism: &'static str,
    pub(crate) worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PricePatternInputGroups {
    pub(crate) groups: Vec<PricePatternGroupedInput>,
    pub(crate) input_rows: u64,
    pub(crate) input_valid_streak_rows: u64,
    pub(crate) input_valid_structure_bar_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PricePatternGroupedInput {
    pub(crate) security_code: String,
    pub(crate) inputs: Vec<PricePatternInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PricePatternSecurityCalculation {
    pub(crate) rows: Vec<PricePatternResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_streak_rows: u64,
    pub(crate) valid_structure_bar_rows: u64,
    pub(crate) null_streak_rows: u64,
    pub(crate) null_second_low_rows: u64,
}
