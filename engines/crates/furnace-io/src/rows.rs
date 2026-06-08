use std::time::Duration;

use furnace_core::{BollInput, KdjInput, MaInput, RsiInput};

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

/// 基于 ClickHouse 执行完整 KDJ 计算。
///
/// # 错误
///
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

impl MaResultRow {
    pub(crate) fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_f64(bytes, self.price_ma_3);
        push_rowbinary_nullable_f64(bytes, self.price_ma_5);
        push_rowbinary_nullable_f64(bytes, self.price_ma_6);
        push_rowbinary_nullable_f64(bytes, self.price_ma_10);
        push_rowbinary_nullable_f64(bytes, self.price_ma_12);
        push_rowbinary_nullable_f64(bytes, self.price_ma_14);
        push_rowbinary_nullable_f64(bytes, self.price_ma_20);
        push_rowbinary_nullable_f64(bytes, self.price_ma_24);
        push_rowbinary_nullable_f64(bytes, self.price_ma_28);
        push_rowbinary_nullable_f64(bytes, self.price_ma_57);
        push_rowbinary_nullable_f64(bytes, self.price_ma_60);
        push_rowbinary_nullable_f64(bytes, self.price_ma_114);
        push_rowbinary_nullable_f64(bytes, self.price_ma_250);
        push_rowbinary_nullable_f64(bytes, self.price_avg_ma_3_6_12_24);
        push_rowbinary_nullable_f64(bytes, self.price_avg_ma_14_28_57_114);
        push_rowbinary_nullable_f64(bytes, self.price_ema1_10_state);
        push_rowbinary_nullable_f64(bytes, self.price_ema2_10);
        push_rowbinary_nullable_f64(bytes, self.price_ema2_10_state);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_5);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_10);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_20);
        push_rowbinary_nullable_f64(bytes, self.volume_ma_60);
        Ok(())
    }
}

impl RsiResultRow {
    pub(crate) fn write_row_binary(&self, bytes: &mut Vec<u8>) -> Result<(), FurnaceIoError> {
        push_rowbinary_string(bytes, &self.security_code);
        push_rowbinary_date(bytes, &self.trade_date)?;
        push_rowbinary_nullable_f64(bytes, self.rsi_6);
        push_rowbinary_nullable_f64(bytes, self.rsi_12);
        push_rowbinary_nullable_f64(bytes, self.rsi_14);
        push_rowbinary_nullable_f64(bytes, self.rsi_24);
        push_rowbinary_nullable_f64(bytes, self.rsi_25);
        push_rowbinary_nullable_f64(bytes, self.rsi_50);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_6_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_6_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_12_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_12_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_14_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_14_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_24_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_24_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_25_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_25_state);
        push_rowbinary_nullable_f64(bytes, self.avg_gain_50_state);
        push_rowbinary_nullable_f64(bytes, self.avg_loss_50_state);
        Ok(())
    }
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
