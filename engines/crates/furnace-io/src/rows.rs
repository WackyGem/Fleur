mod boll;
mod kdj;
mod ma;
mod macd;
mod price_pattern;
mod rsi;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::FurnaceIoError;
use crate::validation::parse_clickhouse_date;

#[derive(Debug, Clone, PartialEq, Eq, clickhouse::Row, Deserialize)]
pub(crate) struct SecurityCodeRow {
    pub(crate) security_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, clickhouse::Row, Deserialize)]
pub(crate) struct CountRow {
    pub(crate) value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, clickhouse::Row, Deserialize)]
pub(crate) struct OptionalDateValueRow {
    #[serde(with = "clickhouse::serde::time::date::option")]
    pub(crate) value: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct KdjInputRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) high_price: Option<f64>,
    pub(crate) low_price: Option<f64>,
    pub(crate) close_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct MaInputRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) close_price: Option<f64>,
    pub(crate) volume: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct CloseInputRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) close_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct PricePatternInputRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) high_price: Option<f64>,
    pub(crate) low_price: Option<f64>,
    pub(crate) close_price: Option<f64>,
    pub(crate) prev_close_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct KdjPreviousStateRow {
    pub(crate) security_code: String,
    pub(crate) k_value: f64,
    pub(crate) d_value: f64,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct MaPreviousStateRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) price_ema1_10_state: f64,
    pub(crate) price_ema2_10_state: f64,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct RsiPreviousStateRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) state_date: Date,
    pub(crate) previous_close: f64,
    pub(crate) state_avg_gain_6: f64,
    pub(crate) state_avg_loss_6: f64,
    pub(crate) state_avg_gain_12: f64,
    pub(crate) state_avg_loss_12: f64,
    pub(crate) state_avg_gain_14: f64,
    pub(crate) state_avg_loss_14: f64,
    pub(crate) state_avg_gain_24: f64,
    pub(crate) state_avg_loss_24: f64,
    pub(crate) state_avg_gain_25: f64,
    pub(crate) state_avg_loss_25: f64,
    pub(crate) state_avg_gain_50: f64,
    pub(crate) state_avg_loss_50: f64,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Deserialize)]
pub(crate) struct MacdPreviousStateRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) ema_fast_state_12: f64,
    pub(crate) ema_slow_state_26: f64,
    pub(crate) macd_dea_state: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, clickhouse::Row, Deserialize)]
pub(crate) struct GapCountRow {
    pub(crate) gap_symbols: u64,
    #[serde(with = "clickhouse::serde::time::date::option")]
    pub(crate) gap_fill_from: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Serialize)]
pub(crate) struct KdjInsertRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) rsv_window: u16,
    pub(crate) k_smoothing: u16,
    pub(crate) d_smoothing: u16,
    pub(crate) rsv: Option<f64>,
    pub(crate) k_value: Option<f64>,
    pub(crate) d_value: Option<f64>,
    pub(crate) j_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Serialize)]
pub(crate) struct MaInsertRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
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

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Serialize)]
pub(crate) struct RsiInsertRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
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

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Serialize)]
pub(crate) struct BollInsertRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
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

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Serialize)]
pub(crate) struct MacdInsertRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) ema_fast_state_12: Option<f64>,
    pub(crate) ema_slow_state_26: Option<f64>,
    pub(crate) macd_dif: Option<f64>,
    pub(crate) macd_dea: Option<f64>,
    pub(crate) macd_dea_state: Option<f64>,
    pub(crate) macd_histogram: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, clickhouse::Row, Serialize)]
pub(crate) struct PricePatternInsertRow {
    pub(crate) security_code: String,
    #[serde(with = "clickhouse::serde::time::date")]
    pub(crate) trade_date: Date,
    pub(crate) close_direction: Option<i8>,
    pub(crate) close_up_streak_days: Option<u16>,
    pub(crate) close_down_streak_days: Option<u16>,
    pub(crate) n_structure_20_is_valid: bool,
    pub(crate) n_structure_20_stage: String,
    pub(crate) n_structure_20_higher_low_ratio: Option<f64>,
    pub(crate) n_structure_20_pullback_depth: Option<f64>,
    pub(crate) n_structure_20_rebound_ratio: Option<f64>,
}

pub(crate) use boll::{
    BollCalculationResult, BollGroupedInput, BollInputGroups, BollResultRow,
    BollSecurityCalculation,
};
pub(crate) use kdj::{
    KdjCalculationResult, KdjGroupedInput, KdjInputGroups, KdjResultRow, KdjSecurityCalculation,
};
pub(crate) use ma::{
    MaCalculationResult, MaGroupedInput, MaInputGroups, MaResultRow, MaSecurityCalculation,
};
pub(crate) use macd::{
    MacdCalculationResult, MacdGroupedInput, MacdInputGroups, MacdResultRow,
    MacdSecurityCalculation,
};
pub(crate) use price_pattern::{
    PricePatternCalculationResult, PricePatternGroupedInput, PricePatternInputGroups,
    PricePatternResultRow, PricePatternSecurityCalculation,
};
pub(crate) use rsi::{
    RsiCalculationResult, RsiGroupedInput, RsiInputGroups, RsiResultRow, RsiSecurityCalculation,
};

impl TryFrom<&KdjResultRow> for KdjInsertRow {
    type Error = FurnaceIoError;

    fn try_from(row: &KdjResultRow) -> Result<Self, Self::Error> {
        Ok(Self {
            security_code: row.security_code.clone(),
            trade_date: parse_clickhouse_date(&row.trade_date)?,
            rsv_window: row.rsv_window,
            k_smoothing: row.k_smoothing,
            d_smoothing: row.d_smoothing,
            rsv: row.rsv,
            k_value: row.k_value,
            d_value: row.d_value,
            j_value: row.j_value,
        })
    }
}

impl TryFrom<&MaResultRow> for MaInsertRow {
    type Error = FurnaceIoError;

    fn try_from(row: &MaResultRow) -> Result<Self, Self::Error> {
        Ok(Self {
            security_code: row.security_code.clone(),
            trade_date: parse_clickhouse_date(&row.trade_date)?,
            price_ma_3: row.price_ma_3,
            price_ma_5: row.price_ma_5,
            price_ma_6: row.price_ma_6,
            price_ma_10: row.price_ma_10,
            price_ma_12: row.price_ma_12,
            price_ma_14: row.price_ma_14,
            price_ma_20: row.price_ma_20,
            price_ma_24: row.price_ma_24,
            price_ma_28: row.price_ma_28,
            price_ma_30: row.price_ma_30,
            price_ma_57: row.price_ma_57,
            price_ma_60: row.price_ma_60,
            price_ma_114: row.price_ma_114,
            price_ma_250: row.price_ma_250,
            price_avg_ma_3_6_12_24: row.price_avg_ma_3_6_12_24,
            price_avg_ma_14_28_57_114: row.price_avg_ma_14_28_57_114,
            price_ema1_10_state: row.price_ema1_10_state,
            price_ema2_10: row.price_ema2_10,
            price_ema2_10_state: row.price_ema2_10_state,
            volume_ma_5: row.volume_ma_5,
            volume_ma_10: row.volume_ma_10,
            volume_ma_20: row.volume_ma_20,
            volume_ma_60: row.volume_ma_60,
        })
    }
}

impl TryFrom<&RsiResultRow> for RsiInsertRow {
    type Error = FurnaceIoError;

    fn try_from(row: &RsiResultRow) -> Result<Self, Self::Error> {
        Ok(Self {
            security_code: row.security_code.clone(),
            trade_date: parse_clickhouse_date(&row.trade_date)?,
            rsi_6: row.rsi_6,
            rsi_12: row.rsi_12,
            rsi_14: row.rsi_14,
            rsi_24: row.rsi_24,
            rsi_25: row.rsi_25,
            rsi_50: row.rsi_50,
            avg_gain_6_state: row.avg_gain_6_state,
            avg_loss_6_state: row.avg_loss_6_state,
            avg_gain_12_state: row.avg_gain_12_state,
            avg_loss_12_state: row.avg_loss_12_state,
            avg_gain_14_state: row.avg_gain_14_state,
            avg_loss_14_state: row.avg_loss_14_state,
            avg_gain_24_state: row.avg_gain_24_state,
            avg_loss_24_state: row.avg_loss_24_state,
            avg_gain_25_state: row.avg_gain_25_state,
            avg_loss_25_state: row.avg_loss_25_state,
            avg_gain_50_state: row.avg_gain_50_state,
            avg_loss_50_state: row.avg_loss_50_state,
        })
    }
}

impl TryFrom<&BollResultRow> for BollInsertRow {
    type Error = FurnaceIoError;

    fn try_from(row: &BollResultRow) -> Result<Self, Self::Error> {
        Ok(Self {
            security_code: row.security_code.clone(),
            trade_date: parse_clickhouse_date(&row.trade_date)?,
            boll_mid_10_1p5: row.boll_mid_10_1p5,
            boll_up_10_1p5: row.boll_up_10_1p5,
            boll_dn_10_1p5: row.boll_dn_10_1p5,
            boll_mid_20_2: row.boll_mid_20_2,
            boll_up_20_2: row.boll_up_20_2,
            boll_dn_20_2: row.boll_dn_20_2,
            boll_mid_50_2p5: row.boll_mid_50_2p5,
            boll_up_50_2p5: row.boll_up_50_2p5,
            boll_dn_50_2p5: row.boll_dn_50_2p5,
        })
    }
}

impl TryFrom<&MacdResultRow> for MacdInsertRow {
    type Error = FurnaceIoError;

    fn try_from(row: &MacdResultRow) -> Result<Self, Self::Error> {
        Ok(Self {
            security_code: row.security_code.clone(),
            trade_date: parse_clickhouse_date(&row.trade_date)?,
            ema_fast_state_12: row.ema_fast_state_12,
            ema_slow_state_26: row.ema_slow_state_26,
            macd_dif: row.macd_dif,
            macd_dea: row.macd_dea,
            macd_dea_state: row.macd_dea_state,
            macd_histogram: row.macd_histogram,
        })
    }
}

impl TryFrom<&PricePatternResultRow> for PricePatternInsertRow {
    type Error = FurnaceIoError;

    fn try_from(row: &PricePatternResultRow) -> Result<Self, Self::Error> {
        Ok(Self {
            security_code: row.security_code.clone(),
            trade_date: parse_clickhouse_date(&row.trade_date)?,
            close_direction: row.close_direction,
            close_up_streak_days: row.close_up_streak_days,
            close_down_streak_days: row.close_down_streak_days,
            n_structure_20_is_valid: row.n_structure_20_is_valid,
            n_structure_20_stage: row.n_structure_20_stage.clone(),
            n_structure_20_higher_low_ratio: row.n_structure_20_higher_low_ratio,
            n_structure_20_pullback_depth: row.n_structure_20_pullback_depth,
            n_structure_20_rebound_ratio: row.n_structure_20_rebound_ratio,
        })
    }
}
