use furnace_core::MacdParams;

use crate::FurnaceIoError;
use crate::schema::{
    DEFAULT_INPUT_TABLE, DEFAULT_INSERT_BATCH_SIZE, DEFAULT_MACD_OUTPUT_TABLE,
    DEFAULT_MACD_PRICE_COLUMN, MIN_INSERT_BATCH_SIZE,
};
use crate::validation::{validate_date, validate_identifier, validate_table_name};

/// CLI or Dagster request write mode for MACD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacdWriteMode {
    /// Calculate and summarize without writing to ClickHouse.
    DryRun,
    /// Append a latest range when target rows do not already exist.
    AppendLatest,
    /// Recalculate a historical range and cascade to latest affected input date.
    ReplaceCascade,
    /// Drop and recreate the output table before writing this run's full results.
    RebuildTable,
}

impl MacdWriteMode {
    /// Parse the CLI spelling for a MACD write mode.
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            "rebuild-table" => Ok(Self::RebuildTable),
            other => Err(FurnaceIoError::InvalidRequest(format!(
                "invalid MACD write mode: {other}"
            ))),
        }
    }

    /// Return the CLI spelling for this write mode.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::AppendLatest => "append-latest",
            Self::ReplaceCascade => "replace-cascade",
            Self::RebuildTable => "rebuild-table",
        }
    }

    /// Return true when this mode writes production ClickHouse data.
    pub fn writes_applied(self) -> bool {
        !matches!(self, Self::DryRun)
    }
}

/// Single Furnace MACD run request.
#[derive(Debug, Clone, PartialEq)]
pub struct MacdRunRequest {
    /// Requested output start date.
    pub request_from: String,
    /// Requested output end date.
    pub request_to: String,
    /// Optional security code allowlist; empty means infer from inputs.
    pub symbols: Vec<String>,
    /// Run identifier from Dagster or CLI.
    pub run_id: Option<String>,
    /// Write mode.
    pub mode: MacdWriteMode,
    /// MACD parameters.
    pub params: MacdParams,
    /// Input table.
    pub input_table: String,
    /// Output table.
    pub output_table: String,
    /// Close input column.
    pub price_column: String,
    /// ClickHouse target insert batch size.
    pub insert_batch_size: usize,
}

impl MacdRunRequest {
    /// Validate the request before ClickHouse operations.
    pub fn validate(&self) -> Result<(), FurnaceIoError> {
        validate_date("request_from", &self.request_from)?;
        validate_date("request_to", &self.request_to)?;
        if self.request_to < self.request_from {
            return Err(FurnaceIoError::InvalidRequest(
                "request_to must be greater than or equal to request_from".to_string(),
            ));
        }
        if self.mode.writes_applied() && !self.params.is_canonical() {
            return Err(FurnaceIoError::InvalidRequest(
                "production MACD writes only allow canonical parameters".to_string(),
            ));
        }
        if self.mode.writes_applied() && self.input_table != DEFAULT_INPUT_TABLE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MACD writes only allow input table {DEFAULT_INPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied() && self.output_table != DEFAULT_MACD_OUTPUT_TABLE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MACD writes only allow output table {DEFAULT_MACD_OUTPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied() && self.price_column != DEFAULT_MACD_PRICE_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production MACD writes only allow price column {DEFAULT_MACD_PRICE_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.insert_batch_size < MIN_INSERT_BATCH_SIZE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production insert batch size must be at least {MIN_INSERT_BATCH_SIZE}"
            )));
        }
        validate_table_name("input_table", &self.input_table)?;
        validate_table_name("output_table", &self.output_table)?;
        validate_identifier("price_column", &self.price_column)?;
        Ok(())
    }
}

impl Default for MacdRunRequest {
    fn default() -> Self {
        Self {
            request_from: String::new(),
            request_to: String::new(),
            symbols: Vec::new(),
            run_id: None,
            mode: MacdWriteMode::DryRun,
            params: MacdParams::default(),
            input_table: DEFAULT_INPUT_TABLE.to_string(),
            output_table: DEFAULT_MACD_OUTPUT_TABLE.to_string(),
            price_column: DEFAULT_MACD_PRICE_COLUMN.to_string(),
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
        }
    }
}
