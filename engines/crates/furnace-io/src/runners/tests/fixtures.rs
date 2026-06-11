use super::*;
pub(super) type RowBinaryInputFixture<'a> =
    (&'a str, &'a str, Option<f64>, Option<f64>, Option<f64>);
pub(super) type MaRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>, Option<f64>);
pub(super) type RsiRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>);
pub(super) type BollRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>);
pub(super) type MacdRowBinaryInputFixture<'a> = (&'a str, &'a str, Option<f64>);
pub(super) type PricePatternRowBinaryInputFixture<'a> = (
    &'a str,
    &'a str,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

#[derive(Debug, Default)]
pub(super) struct FakeExecutor {
    pub(super) queries: Vec<String>,
    pub(super) multi_queries: Vec<Vec<String>>,
    pub(super) inserts: Vec<(String, String)>,
    pub(super) byte_inserts: Vec<(String, Vec<u8>)>,
    responses: Vec<String>,
    byte_responses: Vec<Vec<u8>>,
}

impl FakeExecutor {
    pub(super) fn with_responses_and_bytes(
        responses: &[&str],
        byte_responses: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            responses: responses.iter().map(ToString::to_string).collect(),
            byte_responses,
            ..Self::default()
        }
    }
}

impl ClickHouseExecutor for FakeExecutor {
    fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError> {
        self.queries.push(sql.to_string());
        if self.responses.is_empty() {
            return Ok(String::new());
        }
        Ok(self.responses.remove(0))
    }

    fn query_bytes(&mut self, sql: &str) -> Result<Vec<u8>, FurnaceIoError> {
        self.queries.push(sql.to_string());
        if self.byte_responses.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.byte_responses.remove(0))
    }

    fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError> {
        self.inserts.push((sql.to_string(), tsv.to_string()));
        Ok(())
    }

    fn insert_bytes(&mut self, sql: &str, bytes: &[u8]) -> Result<(), FurnaceIoError> {
        self.byte_inserts.push((sql.to_string(), bytes.to_vec()));
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

pub(super) fn rowbinary_input_rows(rows: &[RowBinaryInputFixture<'_>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, high_price, low_price, close_price) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, *high_price);
        write_rowbinary_nullable_f64(&mut bytes, *low_price);
        write_rowbinary_nullable_f64(&mut bytes, *close_price);
    }
    bytes
}

pub(super) fn ma_rowbinary_input_rows(rows: &[MaRowBinaryInputFixture<'_>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, close_price, volume) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, *close_price);
        write_rowbinary_nullable_f64(&mut bytes, *volume);
    }
    bytes
}

pub(super) fn rsi_rowbinary_input_rows(rows: &[RsiRowBinaryInputFixture<'_>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, close_price) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, *close_price);
    }
    bytes
}

pub(super) fn boll_rowbinary_input_rows(rows: &[BollRowBinaryInputFixture<'_>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, close_price) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, *close_price);
    }
    bytes
}

pub(super) fn macd_rowbinary_input_rows(rows: &[MacdRowBinaryInputFixture<'_>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, close_price) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, *close_price);
    }
    bytes
}

pub(super) fn price_pattern_rowbinary_input_rows(
    rows: &[PricePatternRowBinaryInputFixture<'_>],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, high_price, low_price, close_price, prev_close_price) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, *high_price);
        write_rowbinary_nullable_f64(&mut bytes, *low_price);
        write_rowbinary_nullable_f64(&mut bytes, *close_price);
        write_rowbinary_nullable_f64(&mut bytes, *prev_close_price);
    }
    bytes
}

fn write_rowbinary_string(bytes: &mut Vec<u8>, value: &str) {
    write_rowbinary_var_uint(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn write_rowbinary_var_uint(bytes: &mut Vec<u8>, mut value: usize) {
    while value >= 0x80 {
        bytes.push((value as u8) | 0x80);
        value >>= 7;
    }
    bytes.push(value as u8);
}

fn write_rowbinary_nullable_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(0);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        None => bytes.push(1),
    }
}
