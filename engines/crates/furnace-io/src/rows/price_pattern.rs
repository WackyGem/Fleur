use std::time::Duration;

use furnace_core::PricePatternInput;

use crate::FurnaceIoError;
use crate::rowbinary::{
    push_rowbinary_bool, push_rowbinary_date, push_rowbinary_nullable_date,
    push_rowbinary_nullable_f64, push_rowbinary_nullable_i8, push_rowbinary_nullable_u16,
    push_rowbinary_string, push_rowbinary_u16,
};

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

impl PricePatternResultRow {
    pub(crate) fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_i8(bytes, self.close_direction);
        push_rowbinary_nullable_u16(bytes, self.close_up_streak_days);
        push_rowbinary_nullable_u16(bytes, self.close_down_streak_days);
        push_rowbinary_u16(bytes, self.n_structure_20_valid_bars);
        push_rowbinary_nullable_date(bytes, self.n_structure_20_high_date.as_deref())?;
        push_rowbinary_nullable_f64(bytes, self.n_structure_20_high_price);
        push_rowbinary_nullable_date(bytes, self.n_structure_20_low_date.as_deref())?;
        push_rowbinary_nullable_f64(bytes, self.n_structure_20_low_price);
        push_rowbinary_nullable_date(bytes, self.n_structure_20_second_low_date.as_deref())?;
        push_rowbinary_nullable_f64(bytes, self.n_structure_20_second_low_price);
        push_rowbinary_nullable_f64(bytes, self.n_structure_20_second_low_ratio);
        push_rowbinary_bool(bytes, self.n_structure_20_is_valid);
        Ok(())
    }
}
