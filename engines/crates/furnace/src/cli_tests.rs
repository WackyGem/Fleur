use super::*;
use std::any::{Any, type_name};
use std::collections::VecDeque;

use clickhouse::{RowOwned, RowRead, RowWrite};
use furnace_io::{FurnaceIoError, testing};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

#[derive(Default)]
struct FakeExecutor {
    responses: VecDeque<Box<dyn Any>>,
}

impl FakeExecutor {
    fn with_responses(responses: Vec<Box<dyn Any>>) -> Self {
        Self {
            responses: responses.into(),
        }
    }
}

impl ClickHouseExecutor for FakeExecutor {
    fn fetch_all<T>(&mut self, _sql: &str) -> Result<Vec<T>, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send,
    {
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
        _table: &str,
        _rows: &[T],
        _batch_size: usize,
    ) -> Result<(), FurnaceIoError>
    where
        T: RowOwned + RowWrite + Clone + Send + Sync,
    {
        Ok(())
    }

    fn execute(&mut self, _sql: &str) -> Result<(), FurnaceIoError> {
        Ok(())
    }
}

#[test]
fn run_version_returns_package_version() {
    let mut executor = FakeExecutor::default();

    let output = run_with_executor(args(&["--version"]), &mut executor).unwrap();

    assert_eq!(output, "furnace 0.1.0");
}

#[test]
fn run_kdj_returns_json_summary_for_dry_run() {
    let mut executor = FakeExecutor::with_responses(vec![
        testing::optional_date(Some("2026-01-01")),
        testing::count(0),
        testing::kdj_input_rows(&[
            ("sh.600000", "2026-01-01", Some(10.0), Some(8.0), Some(9.0)),
            ("sz.000001", "2026-01-01", Some(11.0), Some(9.0), Some(10.0)),
        ]),
    ]);

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
            (
                *security_code,
                trade_date.as_str(),
                Some(*close),
                Some(*volume),
            )
        })
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        testing::count(0),
        testing::optional_date(Some("2026-01-01")),
        testing::ma_input_rows(&row_refs),
    ]);

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
    let rows = (1..=51)
        .map(|day| ("sh.600000", testing::fixture_trade_date(day), day as f64))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| {
            (*security_code, trade_date.as_str(), Some(*close))
        })
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        testing::optional_date(Some("2026-01-01")),
        testing::count(0),
        testing::close_input_rows(&row_refs),
    ]);

    let output = run_with_executor(
        args(&[
            "rsi",
            "--from",
            "2026-01-01",
            "--to",
            "2026-02-20",
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
    let rows = (1..=20)
        .map(|day| ("sh.600000", format!("2026-01-{day:02}"), day as f64))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| {
            (*security_code, trade_date.as_str(), Some(*close))
        })
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        testing::optional_date(Some("2026-01-01")),
        testing::close_input_rows(&row_refs),
    ]);

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
fn run_macd_returns_json_summary_for_dry_run() {
    let rows = (1..=40)
        .map(|day| ("sh.600000", testing::fixture_trade_date(day), day as f64))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| {
            (*security_code, trade_date.as_str(), Some(*close))
        })
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        testing::optional_date(Some("2026-01-01")),
        testing::count(0),
        testing::close_input_rows(&row_refs),
    ]);

    let output = run_with_executor(
        args(&[
            "macd",
            "--from",
            "2026-01-01",
            "--to",
            "2026-02-09",
            "--symbols",
            "sh.600000",
            "--run-id",
            "macd-run-1",
        ]),
        &mut executor,
    )
    .unwrap();

    assert!(output.contains("\"indicator\":\"macd\""));
    assert!(output.contains("\"symbols_count\":1"));
    assert!(output.contains("\"mode\":\"dry-run\""));
    assert!(output.contains("\"histogram_mode\":\"DIF - DEA\""));
    assert!(output.contains("\"run_id\":\"macd-run-1\""));
}

#[test]
fn run_price_pattern_returns_json_summary_for_dry_run() {
    let input_rows = [
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
    ];
    let mut executor = FakeExecutor::with_responses(vec![
        testing::optional_date(Some("2026-01-01")),
        testing::price_pattern_input_rows(&input_rows),
    ]);

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
fn run_price_pattern_accepts_rebuild_table_mode() {
    let input_rows = [
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
    ];
    let mut executor = FakeExecutor::with_responses(vec![
        testing::optional_date(Some("2026-01-01")),
        testing::price_pattern_input_rows(&input_rows),
    ]);

    let output = run_with_executor(
        args(&[
            "price-pattern",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
            "--symbols",
            "sh.600000",
            "--mode",
            "rebuild-table",
            "--run-id",
            "price-pattern-rebuild-1",
        ]),
        &mut executor,
    )
    .unwrap();

    assert!(output.contains("\"indicator\":\"price_pattern\""));
    assert!(output.contains("\"mode\":\"rebuild-table\""));
    assert!(output.contains("\"writes_applied\":true"));
    assert!(output.contains("\"run_id\":\"price-pattern-rebuild-1\""));
}

#[test]
fn run_kdj_rejects_non_canonical_write_parameters() {
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
fn run_macd_rejects_unknown_output_format() {
    let mut executor = FakeExecutor::default();

    let error = run_with_executor(
        args(&[
            "macd",
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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
    let mut executor = FakeExecutor::default();

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
