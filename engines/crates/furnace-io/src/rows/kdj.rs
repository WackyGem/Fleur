use std::time::Duration;

use furnace_core::KdjInput;

use crate::FurnaceIoError;
use crate::rowbinary::{push_rowbinary_date, push_rowbinary_nullable_f64, push_rowbinary_string};

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

impl KdjResultRow {
    pub(crate) fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        bytes.extend_from_slice(&self.rsv_window.to_le_bytes());
        bytes.extend_from_slice(&self.k_smoothing.to_le_bytes());
        bytes.extend_from_slice(&self.d_smoothing.to_le_bytes());
        push_rowbinary_nullable_f64(bytes, self.rsv);
        push_rowbinary_nullable_f64(bytes, self.k_value);
        push_rowbinary_nullable_f64(bytes, self.d_value);
        push_rowbinary_nullable_f64(bytes, self.j_value);
        Ok(())
    }
}
