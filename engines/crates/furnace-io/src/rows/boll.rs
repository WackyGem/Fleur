use std::time::Duration;

use furnace_core::BollInput;

use crate::FurnaceIoError;
use crate::rowbinary::{push_rowbinary_date, push_rowbinary_nullable_f64, push_rowbinary_string};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BollResultRow {
    pub(crate) security_code: String,
    pub(crate) trade_date: String,
    pub(crate) boll_mid_10_1p5: Option<f64>,
    pub(crate) boll_up_10_1p5: Option<f64>,
    pub(crate) boll_dn_10_1p5: Option<f64>,
    pub(crate) boll_mid_20_2: Option<f64>,
    pub(crate) boll_up_20_2: Option<f64>,
    pub(crate) boll_dn_20_2: Option<f64>,
    pub(crate) boll_mid_50_2p5: Option<f64>,
    pub(crate) boll_up_50_2p5: Option<f64>,
    pub(crate) boll_dn_50_2p5: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BollCalculationResult {
    pub(crate) rows: Vec<BollResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) output_valid_close_rows: u64,
    pub(crate) null_indicator_rows: u64,
    pub(crate) compute_elapsed: Duration,
    pub(crate) parallelism: &'static str,
    pub(crate) worker_threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BollInputGroups {
    pub(crate) groups: Vec<BollGroupedInput>,
    pub(crate) input_rows: u64,
    pub(crate) input_valid_close_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BollGroupedInput {
    pub(crate) security_code: String,
    pub(crate) inputs: Vec<BollInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BollSecurityCalculation {
    pub(crate) rows: Vec<BollResultRow>,
    pub(crate) output_rows: u64,
    pub(crate) output_valid_close_rows: u64,
    pub(crate) null_indicator_rows: u64,
}

impl BollResultRow {
    pub(crate) fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_f64(bytes, self.boll_mid_10_1p5);
        push_rowbinary_nullable_f64(bytes, self.boll_up_10_1p5);
        push_rowbinary_nullable_f64(bytes, self.boll_dn_10_1p5);
        push_rowbinary_nullable_f64(bytes, self.boll_mid_20_2);
        push_rowbinary_nullable_f64(bytes, self.boll_up_20_2);
        push_rowbinary_nullable_f64(bytes, self.boll_dn_20_2);
        push_rowbinary_nullable_f64(bytes, self.boll_mid_50_2p5);
        push_rowbinary_nullable_f64(bytes, self.boll_up_50_2p5);
        push_rowbinary_nullable_f64(bytes, self.boll_dn_50_2p5);
        Ok(())
    }
}
