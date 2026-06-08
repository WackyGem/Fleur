use furnace_core::{DEFAULT_D_SMOOTHING, DEFAULT_K_SMOOTHING, DEFAULT_RSV_WINDOW, KdjParams};
use furnace_io::{DEFAULT_INSERT_BATCH_SIZE, KdjRunRequest, KdjWriteMode};

use crate::cli::CliError;

use super::{parse_symbols, parse_u16_flag, parse_usize_flag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KdjCommandConfig {
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
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
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

    pub(crate) fn validate(&self) -> Result<(), CliError> {
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

    pub(crate) fn to_request(&self) -> KdjRunRequest {
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
