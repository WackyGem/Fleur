use furnace_io::DEFAULT_INSERT_BATCH_SIZE;

use crate::cli::CliError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommonCommandOptions<M> {
    pub(crate) request_from: String,
    pub(crate) request_to: String,
    pub(crate) symbols: Vec<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) mode: M,
    pub(crate) insert_batch_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommonCommandOptionsBuilder<M> {
    request_from: Option<String>,
    request_to: Option<String>,
    symbols: Vec<String>,
    run_id: Option<String>,
    mode: M,
    insert_batch_size: usize,
    output_format: String,
}

impl<M: Copy> CommonCommandOptionsBuilder<M> {
    pub(crate) fn new(default_mode: M) -> Self {
        Self {
            request_from: None,
            request_to: None,
            symbols: Vec::new(),
            run_id: None,
            mode: default_mode,
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
            output_format: "json".to_string(),
        }
    }

    pub(crate) fn apply_flag(
        &mut self,
        flag: &str,
        value: String,
        parse_mode: impl FnOnce(&str) -> Result<M, CliError>,
    ) -> Result<bool, CliError> {
        match flag {
            "--from" => self.request_from = Some(value),
            "--to" => self.request_to = Some(value),
            "--symbols" => self.symbols = parse_symbols(&value),
            "--run-id" => self.run_id = Some(value),
            "--mode" => self.mode = parse_mode(&value)?,
            "--insert-batch-size" => {
                self.insert_batch_size = parse_usize_flag("--insert-batch-size", &value)?;
            }
            "--output-format" => self.output_format = value,
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(crate) fn finish(self) -> Result<CommonCommandOptions<M>, CliError> {
        if self.output_format != "json" {
            return Err(CliError::Usage(format!(
                "unsupported --output-format value: {}",
                self.output_format
            )));
        }
        let request_from = self
            .request_from
            .ok_or_else(|| CliError::Usage("missing required --from".to_string()))?;
        let request_to = self
            .request_to
            .ok_or_else(|| CliError::Usage("missing required --to".to_string()))?;
        if request_to < request_from {
            return Err(CliError::Usage(
                "--to must be greater than or equal to --from".to_string(),
            ));
        }

        Ok(CommonCommandOptions {
            request_from,
            request_to,
            symbols: self.symbols,
            run_id: self.run_id,
            mode: self.mode,
            insert_batch_size: self.insert_batch_size,
        })
    }
}

pub(crate) fn is_common_flag(flag: &str) -> bool {
    matches!(
        flag,
        "--from"
            | "--to"
            | "--symbols"
            | "--run-id"
            | "--mode"
            | "--insert-batch-size"
            | "--output-format"
    )
}

pub(crate) fn next_flag_value(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, CliError> {
    args.next()
        .ok_or_else(|| CliError::Usage(format!("missing value for {flag}")))
}

pub(crate) fn parse_mode<M, E: std::fmt::Display>(
    value: &str,
    parse: impl FnOnce(&str) -> Result<M, E>,
) -> Result<M, CliError> {
    parse(value).map_err(|error| CliError::Usage(error.to_string()))
}

pub(crate) fn parse_symbols(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn parse_u16_flag(flag: &str, value: &str) -> Result<u16, CliError> {
    value
        .parse::<u16>()
        .map_err(|_| CliError::Usage(format!("{flag} must be a positive integer")))
}

pub(crate) fn parse_usize_flag(flag: &str, value: &str) -> Result<usize, CliError> {
    value
        .parse::<usize>()
        .map_err(|_| CliError::Usage(format!("{flag} must be a positive integer")))
}
