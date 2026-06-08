use furnace_io::{
    DEFAULT_INPUT_TABLE, DEFAULT_INSERT_BATCH_SIZE, DEFAULT_MA_OUTPUT_TABLE,
    DEFAULT_MA_PRICE_COLUMN, DEFAULT_MA_VOLUME_COLUMN, DEFAULT_MA_VOLUME_INPUT_TABLE, MaRunRequest,
    MaWriteMode,
};

use crate::cli::CliError;

use super::{parse_symbols, parse_usize_flag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaCommandConfig {
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
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, CliError> {
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
        Ok(())
    }

    pub(crate) fn to_request(&self) -> MaRunRequest {
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
