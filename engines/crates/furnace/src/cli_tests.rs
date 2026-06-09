use super::*;
use furnace_io::FurnaceIoError;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

#[derive(Debug)]
struct FakeExecutor {
    responses: Vec<String>,
    byte_responses: Vec<Vec<u8>>,
}

impl FakeExecutor {
    fn with_responses(responses: &[&str]) -> Self {
        Self {
            responses: responses.iter().map(ToString::to_string).collect(),
            byte_responses: Vec::new(),
        }
    }

    fn with_responses_and_bytes(responses: &[&str], byte_responses: Vec<Vec<u8>>) -> Self {
        Self {
            responses: responses.iter().map(ToString::to_string).collect(),
            byte_responses,
        }
    }
}

impl ClickHouseExecutor for FakeExecutor {
    fn query(&mut self, _sql: &str) -> Result<String, FurnaceIoError> {
        if self.responses.is_empty() {
            return Ok(String::new());
        }
        Ok(self.responses.remove(0))
    }

    fn query_bytes(&mut self, _sql: &str) -> Result<Vec<u8>, FurnaceIoError> {
        if self.byte_responses.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.byte_responses.remove(0))
    }

    fn insert_tsv(&mut self, _sql: &str, _tsv: &str) -> Result<(), FurnaceIoError> {
        Ok(())
    }

    fn insert_bytes(&mut self, _sql: &str, _bytes: &[u8]) -> Result<(), FurnaceIoError> {
        Ok(())
    }
}

fn rowbinary_input_rows(rows: &[(&str, &str, f64, f64, f64)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, high_price, low_price, close_price) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, Some(*high_price));
        write_rowbinary_nullable_f64(&mut bytes, Some(*low_price));
        write_rowbinary_nullable_f64(&mut bytes, Some(*close_price));
    }
    bytes
}

fn ma_rowbinary_input_rows(rows: &[(&str, &str, f64, f64)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, close_price, volume) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, Some(*close_price));
        write_rowbinary_nullable_f64(&mut bytes, Some(*volume));
    }
    bytes
}

fn rsi_rowbinary_input_rows(rows: &[(&str, &str, f64)]) -> Vec<u8> {
    close_rowbinary_input_rows(rows)
}

fn boll_rowbinary_input_rows(rows: &[(&str, &str, f64)]) -> Vec<u8> {
    close_rowbinary_input_rows(rows)
}

type PricePatternFixtureRow<'a> = (
    &'a str,
    &'a str,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

fn price_pattern_rowbinary_input_rows(rows: &[PricePatternFixtureRow<'_>]) -> Vec<u8> {
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

fn close_rowbinary_input_rows(rows: &[(&str, &str, f64)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (security_code, trade_date, close_price) in rows {
        write_rowbinary_string(&mut bytes, security_code);
        write_rowbinary_string(&mut bytes, trade_date);
        write_rowbinary_nullable_f64(&mut bytes, Some(*close_price));
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

#[test]
fn run_kdj_returns_json_summary_for_dry_run() {
    let responses = ["2026-01-01\n", "0\n"];
    let input_rows = rowbinary_input_rows(&[
        ("sh.600000", "2026-01-01", 10.0, 8.0, 9.0),
        ("sz.000001", "2026-01-01", 11.0, 9.0, 10.0),
    ]);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);

    let output = run_with_executor(
        args(&[
            "kdj",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--symbols",
            "sh.600000, sz.000001",
            "--run-id",
            "run-1",
        ]),
        &mut executor,
    )
    .unwrap();

    assert!(output.contains("\"symbols_count\":2"));
    assert!(output.contains("\"mode\":\"dry-run\""));
    assert!(output.contains("\"run_id\":\"run-1\""));
}

#[test]
fn run_ma_returns_json_summary_for_dry_run() {
    let responses = ["2026-01-01\n", "2026-01-01\n"];
    let rows = (1..=20)
        .map(|day| {
            (
                "sh.600000",
                format!("2026-01-{day:02}"),
                day as f64,
                (day * 100) as f64,
            )
        })
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close, volume)| {
            (*security_code, trade_date.as_str(), *close, *volume)
        })
        .collect::<Vec<_>>();
    let input_rows = ma_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);

    let output = run_with_executor(
        args(&[
            "ma",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-20",
            "--symbols",
            "sh.600000",
            "--run-id",
            "ma-run-1",
        ]),
        &mut executor,
    )
    .unwrap();

    assert!(output.contains("\"indicator\":\"ma\""));
    assert!(output.contains("\"symbols_count\":1"));
    assert!(output.contains("\"mode\":\"dry-run\""));
    assert!(output.contains("\"run_id\":\"ma-run-1\""));
    assert!(output.contains("\"valid_volume_rows\":20"));
    assert!(output.contains("\"volume_ma_windows\":[5,10,20,60]"));
}

#[test]
fn run_rsi_returns_json_summary_for_dry_run() {
    let responses = ["2026-01-01\n", "2026-01-01\n"];
    let rows = (1..=51)
        .map(|day| ("sh.600000", format!("2026-01-{day:02}"), day as f64))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let input_rows = rsi_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);

    let output = run_with_executor(
        args(&[
            "rsi",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-31",
            "--symbols",
            "sh.600000",
            "--run-id",
            "rsi-run-1",
        ]),
        &mut executor,
    )
    .unwrap();

    assert!(output.contains("\"indicator\":\"rsi\""));
    assert!(output.contains("\"symbols_count\":1"));
    assert!(output.contains("\"mode\":\"dry-run\""));
    assert!(output.contains("\"run_id\":\"rsi-run-1\""));
}

#[test]
fn run_boll_returns_json_summary_for_dry_run() {
    let responses = ["sh.600000\n", "2026-01-01\n"];
    let rows = (1..=20)
        .map(|day| ("sh.600000", format!("2026-01-{day:02}"), day as f64))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let input_rows = boll_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);

    let output = run_with_executor(
        args(&[
            "boll",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-20",
            "--symbols",
            "sh.600000",
            "--run-id",
            "boll-run-1",
        ]),
        &mut executor,
    )
    .unwrap();

    assert!(output.contains("\"indicator\":\"boll\""));
    assert!(output.contains("\"symbols_count\":1"));
    assert!(output.contains("\"mode\":\"dry-run\""));
    assert!(output.contains("\"stddev_ddof\":0"));
    assert!(output.contains("\"run_id\":\"boll-run-1\""));
}

#[test]
fn run_price_pattern_returns_json_summary_for_dry_run() {
    let responses = ["sh.600000\n", "2026-01-01\n"];
    let input_rows = price_pattern_rowbinary_input_rows(&[
        (
            "sh.600000",
            "2026-01-01",
            Some(10.0),
            Some(5.0),
            Some(11.0),
            Some(10.0),
        ),
        (
            "sh.600000",
            "2026-01-02",
            Some(15.0),
            Some(7.0),
            Some(12.0),
            Some(11.0),
        ),
        (
            "sh.600000",
            "2026-01-03",
            Some(12.0),
            Some(8.0),
            Some(13.0),
            Some(12.0),
        ),
    ]);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);

    let output = run_with_executor(
        args(&[
            "price-pattern",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-03",
            "--symbols",
            "sh.600000",
            "--run-id",
            "price-pattern-run-1",
        ]),
        &mut executor,
    )
    .unwrap();

    assert!(output.contains("\"indicator\":\"price_pattern\""));
    assert!(output.contains("\"symbols_count\":1"));
    assert!(output.contains("\"mode\":\"dry-run\""));
    assert!(output.contains("\"run_id\":\"price-pattern-run-1\""));
    assert!(output.contains("\"valid_streak_rows\":3"));
    assert!(output.contains("\"n_structure_window\":20"));
}

#[test]
fn run_kdj_rejects_non_canonical_write_parameters() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "kdj",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--mode",
            "append-latest",
            "--rsv-window",
            "5",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Runtime(_)));
}

#[test]
fn run_kdj_rejects_unknown_output_format() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "kdj",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--output-format",
            "text",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Usage(_)));
}

#[test]
fn run_ma_rejects_unknown_output_format() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "ma",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--output-format",
            "text",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Usage(_)));
}

#[test]
fn run_rsi_rejects_unknown_output_format() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "rsi",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--output-format",
            "text",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Usage(_)));
}

#[test]
fn run_boll_rejects_unknown_output_format() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "boll",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--output-format",
            "text",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Usage(_)));
}

#[test]
fn run_price_pattern_rejects_unknown_output_format() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "price-pattern",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--output-format",
            "text",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Usage(_)));
}

#[test]
fn run_ma_rejects_non_canonical_write_price_column() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "ma",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--mode",
            "append-latest",
            "--price-column",
            "close_price",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Runtime(_)));
}

#[test]
fn run_ma_rejects_non_canonical_write_volume_column() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "ma",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--mode",
            "append-latest",
            "--volume-column",
            "amount",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Runtime(_)));
}

#[test]
fn run_ma_rejects_non_canonical_write_volume_input_table() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "ma",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--mode",
            "append-latest",
            "--volume-input-table",
            "fleur_intermediate.some_other_volume_table",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Runtime(_)));
}

#[test]
fn run_boll_rejects_non_canonical_write_price_column() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "boll",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--mode",
            "append-latest",
            "--price-column",
            "close_price",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Runtime(_)));
}

#[test]
fn run_rsi_rejects_non_canonical_write_price_column() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "rsi",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--mode",
            "append-latest",
            "--price-column",
            "close_price",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Runtime(_)));
}

#[test]
fn run_price_pattern_rejects_non_canonical_write_close_column() {
    let mut executor = FakeExecutor::with_responses(&[]);

    let error = run_with_executor(
        args(&[
            "price-pattern",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--mode",
            "append-latest",
            "--close-column",
            "close_price_forward_adj",
        ]),
        &mut executor,
    )
    .unwrap_err();

    assert!(matches!(error, CliError::Runtime(_)));
}
