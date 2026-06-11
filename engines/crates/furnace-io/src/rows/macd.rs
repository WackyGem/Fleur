use std::time::Duration;

use furnace_core::MacdInput;

use crate::FurnaceIoError;
use crate::rowbinary::{push_rowbinary_date, push_rowbinary_nullable_f64, push_rowbinary_string};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacdResultRow {
    pub(crate) security_code: String,
    pub(crate) trade_date: String,
    pub(crate) ema_fast_state_12: Option<f64>,
    pub(crate) ema_slow_state_26: Option<f64>,
    pub(crate) macd_dif: Option<f64>,
    pub(crate) macd_dea: Option<f64>,
    pub(crate) macd_dea_state: Option<f64>,
    pub(crate) macd_histogram: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacdCalculationResult {
    pub(crate) rows: Vec<MacdResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) null_indicator_rows: u64,
    pub(crate) compute_elapsed: Duration,
    pub(crate) parallelism: &'static str,
    pub(crate) worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacdInputGroups {
    pub(crate) groups: Vec<MacdGroupedInput>,
    pub(crate) input_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) input_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacdGroupedInput {
    pub(crate) security_code: String,
    pub(crate) inputs: Vec<MacdInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MacdSecurityCalculation {
    pub(crate) rows: Vec<MacdResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) valid_close_rows: u64,
    pub(crate) null_indicator_rows: u64,
}

impl MacdResultRow {
    pub(crate) fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_f64(bytes, self.ema_fast_state_12);
        push_rowbinary_nullable_f64(bytes, self.ema_slow_state_26);
        push_rowbinary_nullable_f64(bytes, self.macd_dif);
        push_rowbinary_nullable_f64(bytes, self.macd_dea);
        push_rowbinary_nullable_f64(bytes, self.macd_dea_state);
        push_rowbinary_nullable_f64(bytes, self.macd_histogram);
        Ok(())
    }
}
