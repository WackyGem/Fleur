use std::env;
use std::error::Error;
use std::fmt;
use std::process::ExitCode;

use furnace_core::{DEFAULT_D_SMOOTHING, DEFAULT_K_SMOOTHING, DEFAULT_RSV_WINDOW, KdjParams};
use furnace_io::{
    ClickHouseCliExecutor, ClickHouseExecutor, DEFAULT_INSERT_BATCH_SIZE, KdjRunRequest,
    KdjWriteMode, run_kdj,
};

const EXIT_USAGE: u8 = 2;
const EXIT_RUNTIME: u8 = 3;

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            print_help();
            ExitCode::from(EXIT_USAGE)
        }
        Err(CliError::Runtime(message)) => {
            eprintln!("{message}");
            ExitCode::from(EXIT_RUNTIME)
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<String, CliError> {
    let mut executor = ClickHouseCliExecutor::from_env();
    run_with_executor(args, &mut executor)
}

fn run_with_executor<E: ClickHouseExecutor>(
    args: impl IntoIterator<Item = String>,
    executor: &mut E,
) -> Result<String, CliError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Err(CliError::Usage("missing command".to_string()));
    };

    match command.as_str() {
        "kdj" => {
            let config = KdjCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_kdj(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "--help" | "-h" => {
            print_help();
            Ok(String::new())
        }
        other => Err(CliError::Usage(format!("unknown command: {other}"))),
    }
}

fn print_help() {
    eprintln!(
        "Usage: furnace kdj --from YYYY-MM-DD --to YYYY-MM-DD [--mode dry-run|append-latest|replace-cascade] [--symbols CODE1,CODE2] [--run-id ID] [--rsv-window 9] [--k-smoothing 3] [--d-smoothing 3] [--insert-batch-size 10000] [--output-format json]"
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => f.write_str(message),
        }
    }
}

impl Error for CliError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KdjCommandConfig {
    request_from: String,
    request_to: String,
    symbols: Vec<String>,
    run_id: Option<String>,
    mode: KdjWriteMode,
    rsv_window: u16,
    k_smoothing: u16,
    d_smoothing: u16,
    insert_batch_size: usize,
    output_format: String,
}

impl KdjCommandConfig {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut request_from = None;
        let mut request_to = None;
        let mut symbols = Vec::new();
        let mut run_id = None;
        let mut mode = KdjWriteMode::DryRun;
        let mut rsv_window = DEFAULT_RSV_WINDOW;
        let mut k_smoothing = DEFAULT_K_SMOOTHING;
        let mut d_smoothing = DEFAULT_D_SMOOTHING;
        let mut insert_batch_size = DEFAULT_INSERT_BATCH_SIZE;
        let mut output_format = "json".to_string();

        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            let value = match flag.as_str() {
                "--from"
                | "--to"
                | "--symbols"
                | "--run-id"
                | "--mode"
                | "--rsv-window"
                | "--k-smoothing"
                | "--d-smoothing"
                | "--insert-batch-size"
                | "--output-format" => args
                    .next()
                    .ok_or_else(|| CliError::Usage(format!("missing value for {flag}")))?,
                other => return Err(CliError::Usage(format!("unknown option: {other}"))),
            };

            match flag.as_str() {
                "--from" => request_from = Some(value),
                "--to" => request_to = Some(value),
                "--symbols" => symbols = parse_symbols(&value),
                "--run-id" => run_id = Some(value),
                "--mode" => {
                    mode = KdjWriteMode::parse(&value)
                        .map_err(|error| CliError::Usage(error.to_string()))?
                }
                "--rsv-window" => rsv_window = parse_u16_flag("--rsv-window", &value)?,
                "--k-smoothing" => k_smoothing = parse_u16_flag("--k-smoothing", &value)?,
                "--d-smoothing" => d_smoothing = parse_u16_flag("--d-smoothing", &value)?,
                "--insert-batch-size" => {
                    insert_batch_size = parse_usize_flag("--insert-batch-size", &value)?;
                }
                "--output-format" => output_format = value,
                _ => unreachable!("flag match is exhaustive"),
            }
        }

        Ok(Self {
            request_from: request_from
                .ok_or_else(|| CliError::Usage("missing required --from".to_string()))?,
            request_to: request_to
                .ok_or_else(|| CliError::Usage("missing required --to".to_string()))?,
            symbols,
            run_id,
            mode,
            rsv_window,
            k_smoothing,
            d_smoothing,
            insert_batch_size,
            output_format,
        })
    }

    fn validate(&self) -> Result<(), CliError> {
        if self.output_format != "json" {
            return Err(CliError::Usage(format!(
                "unsupported --output-format value: {}",
                self.output_format
            )));
        }
        if self.request_to < self.request_from {
            return Err(CliError::Usage(
                "--to must be greater than or equal to --from".to_string(),
            ));
        }

        let params = KdjParams {
            rsv_window: self.rsv_window,
            k_smoothing: self.k_smoothing,
            d_smoothing: self.d_smoothing,
            ..KdjParams::default()
        };
        if !params.is_canonical() && self.mode.writes_applied() {
            return Err(CliError::Runtime(
                "production KDJ writes only allow canonical parameters 9/3/3".to_string(),
            ));
        }

        Ok(())
    }

    fn to_request(&self) -> KdjRunRequest {
        KdjRunRequest {
            request_from: self.request_from.clone(),
            request_to: self.request_to.clone(),
            symbols: self.symbols.clone(),
            run_id: self.run_id.clone(),
            mode: self.mode,
            params: KdjParams {
                rsv_window: self.rsv_window,
                k_smoothing: self.k_smoothing,
                d_smoothing: self.d_smoothing,
                ..KdjParams::default()
            },
            insert_batch_size: self.insert_batch_size,
        }
    }
}

fn parse_symbols(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_u16_flag(flag: &str, value: &str) -> Result<u16, CliError> {
    value
        .parse::<u16>()
        .map_err(|_| CliError::Usage(format!("{flag} must be a positive integer")))
}

fn parse_usize_flag(flag: &str, value: &str) -> Result<usize, CliError> {
    value
        .parse::<usize>()
        .map_err(|_| CliError::Usage(format!("{flag} must be a positive integer")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use furnace_io::FurnaceIoError;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[derive(Debug)]
    struct FakeExecutor {
        responses: Vec<String>,
    }

    impl FakeExecutor {
        fn with_responses(responses: &[&str]) -> Self {
            Self {
                responses: responses.iter().map(ToString::to_string).collect(),
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

        fn insert_tsv(&mut self, _sql: &str, _tsv: &str) -> Result<(), FurnaceIoError> {
            Ok(())
        }
    }

    #[test]
    fn run_kdj_returns_json_summary_for_dry_run() {
        let responses = [
            "2026-01-01\n",
            "0\n",
            "sh.600000\t2026-01-01\t10\t8\t9\nsz.000001\t2026-01-01\t11\t9\t10\n",
        ];
        let mut executor = FakeExecutor::with_responses(&responses);

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
}
