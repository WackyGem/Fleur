use std::any::{Any, type_name};
use std::collections::VecDeque;

use clickhouse::{RowOwned, RowRead, RowWrite};

use super::*;
use crate::validation::parse_clickhouse_date;

pub(super) type KdjInputFixture<'a> = (&'a str, &'a str, Option<f64>, Option<f64>, Option<f64>);
pub(super) type MaInputFixture<'a> = (&'a str, &'a str, Option<f64>, Option<f64>);
pub(super) type CloseInputFixture<'a> = (&'a str, &'a str, Option<f64>);
pub(super) type PricePatternInputFixture<'a> = (
    &'a str,
    &'a str,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);
pub(super) type RsiPreviousStateFixture<'a> = (
    &'a str,
    &'a str,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FakeInsert {
    pub(super) table: String,
    pub(super) rows: usize,
    pub(super) batch_size: usize,
    pub(super) row_type: &'static str,
}

#[derive(Default)]
pub(super) struct FakeExecutor {
    pub(super) queries: Vec<String>,
    pub(super) multi_queries: Vec<Vec<String>>,
    pub(super) inserts: Vec<FakeInsert>,
    responses: VecDeque<Box<dyn Any>>,
}

impl FakeExecutor {
    pub(super) fn with_responses(responses: Vec<Box<dyn Any>>) -> Self {
        Self {
            responses: responses.into(),
            ..Self::default()
        }
    }
}

impl ClickHouseExecutor for FakeExecutor {
    fn fetch_all<T>(&mut self, sql: &str) -> Result<Vec<T>, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send,
    {
        self.queries.push(sql.to_string());
        let Some(response) = self.responses.pop_front() else {
            return Ok(Vec::new());
        };
        response
            .downcast::<Vec<T>>()
            .map(|rows| *rows)
            .map_err(|_| {
                FurnaceIoError::Parse(format!(
                    "fake ClickHouse response type mismatch; expected {}",
                    type_name::<Vec<T>>()
                ))
            })
    }

    fn insert_rows<T>(
        &mut self,
        table: &str,
        rows: &[T],
        batch_size: usize,
    ) -> Result<(), FurnaceIoError>
    where
        T: RowOwned + RowWrite + Clone + Send + Sync,
    {
        self.inserts.push(FakeInsert {
            table: table.to_string(),
            rows: rows.len(),
            batch_size,
            row_type: type_name::<T>(),
        });
        Ok(())
    }

    fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError> {
        self.queries.push(sql.to_string());
        Ok(())
    }

    fn execute_many(&mut self, sqls: &[String]) -> Result<(), FurnaceIoError> {
        self.multi_queries.push(sqls.to_vec());
        Ok(())
    }
}

pub(super) fn response<T: 'static>(rows: Vec<T>) -> Box<dyn Any> {
    Box::new(rows)
}

pub(super) fn security_codes(values: &[&str]) -> Vec<SecurityCodeRow> {
    values
        .iter()
        .map(|value| SecurityCodeRow {
            security_code: (*value).to_string(),
        })
        .collect()
}

pub(super) fn count(value: u64) -> Vec<CountRow> {
    vec![CountRow { value }]
}

pub(super) fn optional_date(value: Option<&str>) -> Vec<OptionalDateValueRow> {
    vec![OptionalDateValueRow {
        value: value.map(date).transpose().unwrap(),
    }]
}

pub(super) fn gap_count(gap_symbols: u64, gap_fill_from: Option<&str>) -> Vec<GapCountRow> {
    vec![GapCountRow {
        gap_symbols,
        gap_fill_from: gap_fill_from.map(date).transpose().unwrap(),
    }]
}

pub(super) fn kdj_input_rows(rows: &[KdjInputFixture<'_>]) -> Vec<KdjInputRow> {
    rows.iter()
        .map(
            |(security_code, trade_date, high_price, low_price, close_price)| KdjInputRow {
                security_code: (*security_code).to_string(),
                trade_date: date(trade_date).unwrap(),
                high_price: *high_price,
                low_price: *low_price,
                close_price: *close_price,
            },
        )
        .collect()
}

pub(super) fn ma_input_rows(rows: &[MaInputFixture<'_>]) -> Vec<MaInputRow> {
    rows.iter()
        .map(
            |(security_code, trade_date, close_price, volume)| MaInputRow {
                security_code: (*security_code).to_string(),
                trade_date: date(trade_date).unwrap(),
                close_price: *close_price,
                volume: *volume,
            },
        )
        .collect()
}

pub(super) fn close_input_rows(rows: &[CloseInputFixture<'_>]) -> Vec<CloseInputRow> {
    rows.iter()
        .map(|(security_code, trade_date, close_price)| CloseInputRow {
            security_code: (*security_code).to_string(),
            trade_date: date(trade_date).unwrap(),
            close_price: *close_price,
        })
        .collect()
}

pub(super) fn price_pattern_input_rows(
    rows: &[PricePatternInputFixture<'_>],
) -> Vec<PricePatternInputRow> {
    rows.iter()
        .map(
            |(security_code, trade_date, high_price, low_price, close_price, prev_close_price)| {
                PricePatternInputRow {
                    security_code: (*security_code).to_string(),
                    trade_date: date(trade_date).unwrap(),
                    high_price: *high_price,
                    low_price: *low_price,
                    close_price: *close_price,
                    prev_close_price: *prev_close_price,
                }
            },
        )
        .collect()
}

pub(super) fn ma_previous_states(rows: &[(&str, &str, f64, f64)]) -> Vec<MaPreviousStateRow> {
    rows.iter()
        .map(
            |(security_code, trade_date, price_ema1_10_state, price_ema2_10_state)| {
                MaPreviousStateRow {
                    security_code: (*security_code).to_string(),
                    trade_date: date(trade_date).unwrap(),
                    price_ema1_10_state: *price_ema1_10_state,
                    price_ema2_10_state: *price_ema2_10_state,
                }
            },
        )
        .collect()
}

pub(super) fn rsi_previous_states(
    rows: &[RsiPreviousStateFixture<'_>],
) -> Vec<RsiPreviousStateRow> {
    rows.iter()
        .map(
            |(
                security_code,
                state_date,
                previous_close,
                state_avg_gain_6,
                state_avg_loss_6,
                state_avg_gain_12,
                state_avg_loss_12,
                state_avg_gain_14,
                state_avg_loss_14,
                state_avg_gain_24,
                state_avg_loss_24,
                state_avg_gain_25,
                state_avg_loss_25,
                state_avg_gain_50,
                state_avg_loss_50,
            )| RsiPreviousStateRow {
                security_code: (*security_code).to_string(),
                state_date: date(state_date).unwrap(),
                previous_close: *previous_close,
                state_avg_gain_6: *state_avg_gain_6,
                state_avg_loss_6: *state_avg_loss_6,
                state_avg_gain_12: *state_avg_gain_12,
                state_avg_loss_12: *state_avg_loss_12,
                state_avg_gain_14: *state_avg_gain_14,
                state_avg_loss_14: *state_avg_loss_14,
                state_avg_gain_24: *state_avg_gain_24,
                state_avg_loss_24: *state_avg_loss_24,
                state_avg_gain_25: *state_avg_gain_25,
                state_avg_loss_25: *state_avg_loss_25,
                state_avg_gain_50: *state_avg_gain_50,
                state_avg_loss_50: *state_avg_loss_50,
            },
        )
        .collect()
}

pub(super) fn macd_previous_states(
    rows: &[(&str, &str, f64, f64, f64)],
) -> Vec<MacdPreviousStateRow> {
    rows.iter()
        .map(
            |(security_code, trade_date, ema_fast_state_12, ema_slow_state_26, macd_dea_state)| {
                MacdPreviousStateRow {
                    security_code: (*security_code).to_string(),
                    trade_date: date(trade_date).unwrap(),
                    ema_fast_state_12: *ema_fast_state_12,
                    ema_slow_state_26: *ema_slow_state_26,
                    macd_dea_state: *macd_dea_state,
                }
            },
        )
        .collect()
}

pub(super) fn fixture_trade_date(day: u8) -> String {
    if day <= 31 {
        format!("2026-01-{day:02}")
    } else {
        format!("2026-02-{:02}", day - 31)
    }
}

fn date(value: &str) -> Result<time::Date, FurnaceIoError> {
    parse_clickhouse_date(value)
}
