use std::any::Any;

use crate::rows::{
    CloseInputRow, CountRow, GapCountRow, KdjInputRow, MaInputRow, OptionalDateValueRow,
    PricePatternInputRow, SecurityCodeRow,
};
use crate::validation::parse_clickhouse_date;

pub type KdjInputFixture<'a> = (&'a str, &'a str, Option<f64>, Option<f64>, Option<f64>);
pub type MaInputFixture<'a> = (&'a str, &'a str, Option<f64>, Option<f64>);
pub type CloseInputFixture<'a> = (&'a str, &'a str, Option<f64>);
pub type PricePatternInputFixture<'a> = (
    &'a str,
    &'a str,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

pub fn response<T: 'static>(rows: Vec<T>) -> Box<dyn Any> {
    Box::new(rows)
}

pub fn security_codes(values: &[&str]) -> Box<dyn Any> {
    response(
        values
            .iter()
            .map(|value| SecurityCodeRow {
                security_code: (*value).to_string(),
            })
            .collect::<Vec<_>>(),
    )
}

pub fn count(value: u64) -> Box<dyn Any> {
    response(vec![CountRow { value }])
}

pub fn optional_date(value: Option<&str>) -> Box<dyn Any> {
    response(vec![OptionalDateValueRow {
        value: value.map(parse_clickhouse_date).transpose().unwrap(),
    }])
}

pub fn gap_count(gap_symbols: u64, gap_fill_from: Option<&str>) -> Box<dyn Any> {
    response(vec![GapCountRow {
        gap_symbols,
        gap_fill_from: gap_fill_from
            .map(parse_clickhouse_date)
            .transpose()
            .unwrap(),
    }])
}

pub fn kdj_input_rows(rows: &[KdjInputFixture<'_>]) -> Box<dyn Any> {
    response(
        rows.iter()
            .map(
                |(security_code, trade_date, high_price, low_price, close_price)| KdjInputRow {
                    security_code: (*security_code).to_string(),
                    trade_date: parse_clickhouse_date(trade_date).unwrap(),
                    high_price: *high_price,
                    low_price: *low_price,
                    close_price: *close_price,
                },
            )
            .collect::<Vec<_>>(),
    )
}

pub fn ma_input_rows(rows: &[MaInputFixture<'_>]) -> Box<dyn Any> {
    response(
        rows.iter()
            .map(
                |(security_code, trade_date, close_price, volume)| MaInputRow {
                    security_code: (*security_code).to_string(),
                    trade_date: parse_clickhouse_date(trade_date).unwrap(),
                    close_price: *close_price,
                    volume: *volume,
                },
            )
            .collect::<Vec<_>>(),
    )
}

pub fn close_input_rows(rows: &[CloseInputFixture<'_>]) -> Box<dyn Any> {
    response(
        rows.iter()
            .map(|(security_code, trade_date, close_price)| CloseInputRow {
                security_code: (*security_code).to_string(),
                trade_date: parse_clickhouse_date(trade_date).unwrap(),
                close_price: *close_price,
            })
            .collect::<Vec<_>>(),
    )
}

pub fn price_pattern_input_rows(rows: &[PricePatternInputFixture<'_>]) -> Box<dyn Any> {
    response(
        rows.iter()
            .map(
                |(
                    security_code,
                    trade_date,
                    high_price,
                    low_price,
                    close_price,
                    prev_close_price,
                )| PricePatternInputRow {
                    security_code: (*security_code).to_string(),
                    trade_date: parse_clickhouse_date(trade_date).unwrap(),
                    high_price: *high_price,
                    low_price: *low_price,
                    close_price: *close_price,
                    prev_close_price: *prev_close_price,
                },
            )
            .collect::<Vec<_>>(),
    )
}

pub fn fixture_trade_date(day: u8) -> String {
    if day <= 31 {
        format!("2026-01-{day:02}")
    } else {
        format!("2026-02-{:02}", day - 31)
    }
}
