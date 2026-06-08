use std::env;
use std::error::Error;
use std::fmt;
use std::process::ExitCode;

use furnace_core::{DEFAULT_D_SMOOTHING, DEFAULT_K_SMOOTHING, DEFAULT_RSV_WINDOW, KdjParams};
use furnace_io::{
    BollRunRequest, BollWriteMode, ClickHouseCliExecutor, ClickHouseExecutor,
    DEFAULT_BOLL_OUTPUT_TABLE, DEFAULT_BOLL_PRICE_COLUMN, DEFAULT_INPUT_TABLE,
    DEFAULT_INSERT_BATCH_SIZE, DEFAULT_MA_OUTPUT_TABLE, DEFAULT_MA_PRICE_COLUMN,
    DEFAULT_MA_VOLUME_COLUMN, DEFAULT_MA_VOLUME_INPUT_TABLE, DEFAULT_RSI_OUTPUT_TABLE,
    DEFAULT_RSI_PRICE_COLUMN, KdjRunRequest, KdjWriteMode, MaRunRequest, MaWriteMode,
    RsiRunRequest, RsiWriteMode, run_boll, run_kdj, run_ma, run_rsi,
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
        "ma" => {
            let config = MaCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_ma(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "rsi" => {
            let config = RsiCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_rsi(executor, &config.to_request())
                .map_err(|error| CliError::Runtime(error.to_string()))?;
            Ok(summary.to_json())
        }
        "boll" => {
            let config = BollCommandConfig::parse(args)?;
            config.validate()?;
            let summary = run_boll(executor, &config.to_request())
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
        "Usage: furnace kdj --from YYYY-MM-DD --to YYYY-MM-DD [--mode dry-run|append-latest|replace-cascade] [--symbols CODE1,CODE2] [--run-id ID] [--rsv-window 9] [--k-smoothing 3] [--d-smoothing 3] [--insert-batch-size 10000] [--output-format json]\n       furnace ma --from YYYY-MM-DD --to YYYY-MM-DD [--mode dry-run|append-latest|replace-cascade] [--symbols CODE1,CODE2] [--run-id ID] [--input-table fleur_intermediate.int_stock_quotes_daily_adj] [--volume-input-table fleur_intermediate.int_stock_quotes_daily_unadj] [--output-table fleur_calculation.calc_stock_ma_daily] [--price-column close_price_forward_adj] [--volume-column volume] [--insert-batch-size 10000] [--output-format json]\n       furnace rsi --from YYYY-MM-DD --to YYYY-MM-DD [--mode dry-run|append-latest|replace-cascade] [--symbols CODE1,CODE2] [--run-id ID] [--input-table fleur_intermediate.int_stock_quotes_daily_adj] [--output-table fleur_calculation.calc_stock_rsi_daily] [--price-column close_price_forward_adj] [--insert-batch-size 10000] [--output-format json]\n       furnace boll --from YYYY-MM-DD --to YYYY-MM-DD [--mode dry-run|append-latest|replace-cascade] [--symbols CODE1,CODE2] [--run-id ID] [--input-table fleur_intermediate.int_stock_quotes_daily_adj] [--output-table fleur_calculation.calc_stock_boll_daily] [--price-column close_price_forward_adj] [--insert-batch-size 10000] [--output-format json]"
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaCommandConfig {
    request_from: String,
    request_to: String,
    symbols: Vec<String>,
    run_id: Option<String>,
    mode: MaWriteMode,
    input_table: String,
    volume_input_table: String,
    output_table: String,
    price_column: String,
    volume_column: String,
    insert_batch_size: usize,
    output_format: String,
}

impl MaCommandConfig {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut request_from = None;
        let mut request_to = None;
        let mut symbols = Vec::new();
        let mut run_id = None;
        let mut mode = MaWriteMode::DryRun;
        let mut input_table = DEFAULT_INPUT_TABLE.to_string();
        let mut volume_input_table = DEFAULT_MA_VOLUME_INPUT_TABLE.to_string();
        let mut output_table = DEFAULT_MA_OUTPUT_TABLE.to_string();
        let mut price_column = DEFAULT_MA_PRICE_COLUMN.to_string();
        let mut volume_column = DEFAULT_MA_VOLUME_COLUMN.to_string();
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
                | "--input-table"
                | "--volume-input-table"
                | "--output-table"
                | "--price-column"
                | "--volume-column"
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
                    mode = MaWriteMode::parse(&value)
                        .map_err(|error| CliError::Usage(error.to_string()))?
                }
                "--input-table" => input_table = value,
                "--volume-input-table" => volume_input_table = value,
                "--output-table" => output_table = value,
                "--price-column" => price_column = value,
                "--volume-column" => volume_column = value,
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
            input_table,
            volume_input_table,
            output_table,
            price_column,
            volume_column,
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
        Ok(())
    }

    fn to_request(&self) -> MaRunRequest {
        MaRunRequest {
            request_from: self.request_from.clone(),
            request_to: self.request_to.clone(),
            symbols: self.symbols.clone(),
            run_id: self.run_id.clone(),
            mode: self.mode,
            input_table: self.input_table.clone(),
            volume_input_table: self.volume_input_table.clone(),
            output_table: self.output_table.clone(),
            price_column: self.price_column.clone(),
            volume_column: self.volume_column.clone(),
            insert_batch_size: self.insert_batch_size,
            ..MaRunRequest::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RsiCommandConfig {
    request_from: String,
    request_to: String,
    symbols: Vec<String>,
    run_id: Option<String>,
    mode: RsiWriteMode,
    input_table: String,
    output_table: String,
    price_column: String,
    insert_batch_size: usize,
    output_format: String,
}

impl RsiCommandConfig {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut request_from = None;
        let mut request_to = None;
        let mut symbols = Vec::new();
        let mut run_id = None;
        let mut mode = RsiWriteMode::DryRun;
        let mut input_table = DEFAULT_INPUT_TABLE.to_string();
        let mut output_table = DEFAULT_RSI_OUTPUT_TABLE.to_string();
        let mut price_column = DEFAULT_RSI_PRICE_COLUMN.to_string();
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
                | "--input-table"
                | "--output-table"
                | "--price-column"
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
                    mode = RsiWriteMode::parse(&value)
                        .map_err(|error| CliError::Usage(error.to_string()))?
                }
                "--input-table" => input_table = value,
                "--output-table" => output_table = value,
                "--price-column" => price_column = value,
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
            input_table,
            output_table,
            price_column,
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
        Ok(())
    }

    fn to_request(&self) -> RsiRunRequest {
        RsiRunRequest {
            request_from: self.request_from.clone(),
            request_to: self.request_to.clone(),
            symbols: self.symbols.clone(),
            run_id: self.run_id.clone(),
            mode: self.mode,
            input_table: self.input_table.clone(),
            output_table: self.output_table.clone(),
            price_column: self.price_column.clone(),
            insert_batch_size: self.insert_batch_size,
            ..RsiRunRequest::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BollCommandConfig {
    request_from: String,
    request_to: String,
    symbols: Vec<String>,
    run_id: Option<String>,
    mode: BollWriteMode,
    input_table: String,
    output_table: String,
    price_column: String,
    insert_batch_size: usize,
    output_format: String,
}

impl BollCommandConfig {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
        let mut request_from = None;
        let mut request_to = None;
        let mut symbols = Vec::new();
        let mut run_id = None;
        let mut mode = BollWriteMode::DryRun;
        let mut input_table = DEFAULT_INPUT_TABLE.to_string();
        let mut output_table = DEFAULT_BOLL_OUTPUT_TABLE.to_string();
        let mut price_column = DEFAULT_BOLL_PRICE_COLUMN.to_string();
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
                | "--input-table"
                | "--output-table"
                | "--price-column"
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
                    mode = BollWriteMode::parse(&value)
                        .map_err(|error| CliError::Usage(error.to_string()))?
                }
                "--input-table" => input_table = value,
                "--output-table" => output_table = value,
                "--price-column" => price_column = value,
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
            input_table,
            output_table,
            price_column,
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
        Ok(())
    }

    fn to_request(&self) -> BollRunRequest {
        BollRunRequest {
            request_from: self.request_from.clone(),
            request_to: self.request_to.clone(),
            symbols: self.symbols.clone(),
            run_id: self.run_id.clone(),
            mode: self.mode,
            input_table: self.input_table.clone(),
            output_table: self.output_table.clone(),
            price_column: self.price_column.clone(),
            insert_batch_size: self.insert_batch_size,
            ..BollRunRequest::default()
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
}
