use furnace_core::PricePatternParams;

use crate::FurnaceIoError;
use crate::schema::{
    DEFAULT_INSERT_BATCH_SIZE, DEFAULT_PRICE_PATTERN_CLOSE_COLUMN,
    DEFAULT_PRICE_PATTERN_HIGH_COLUMN, DEFAULT_PRICE_PATTERN_LOW_COLUMN,
    DEFAULT_PRICE_PATTERN_OUTPUT_TABLE, DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN,
    DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE, DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE,
    MIN_INSERT_BATCH_SIZE,
};
use crate::validation::{validate_date, validate_identifier, validate_table_name};

/// CLI or Dagster request write mode for Price Pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PricePatternWriteMode {
    /// Compute and summarize without writing ClickHouse.
    DryRun,
    /// Append latest rows when target table has no same-or-later rows.
    AppendLatest,
    /// Recompute history and cascade to latest affected input date.
    ReplaceCascade,
}

impl PricePatternWriteMode {
    /// Parse CLI spelling.
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            other => Err(FurnaceIoError::InvalidRequest(format!(
                "invalid Price Pattern write mode: {other}"
            ))),
        }
    }

    /// Return CLI spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::AppendLatest => "append-latest",
            Self::ReplaceCascade => "replace-cascade",
        }
    }

    /// Return true when this mode writes production rows.
    pub fn writes_applied(self) -> bool {
        !matches!(self, Self::DryRun)
    }
}

/// Single Furnace Price Pattern run request.
#[derive(Debug, Clone, PartialEq)]
pub struct PricePatternRunRequest {
    /// Requested output start date.
    pub request_from: String,
    /// Requested output end date.
    pub request_to: String,
    /// Optional security whitelist; empty means infer from input rows.
    pub symbols: Vec<String>,
    /// Run identifier from Dagster or CLI.
    pub run_id: Option<String>,
    /// Write mode.
    pub mode: PricePatternWriteMode,
    /// Price Pattern parameters.
    pub params: PricePatternParams,
    /// Forward-adjusted structure input table.
    pub structure_input_table: String,
    /// Unadjusted close streak input table.
    pub streak_input_table: String,
    /// Output table.
    pub output_table: String,
    /// Structure high column.
    pub high_column: String,
    /// Structure low column.
    pub low_column: String,
    /// Close streak close column.
    pub close_column: String,
    /// Close streak previous close column.
    pub prev_close_column: String,
    /// Target rows per ClickHouse insert batch.
    pub insert_batch_size: usize,
}

impl PricePatternRunRequest {
    /// Validate request before ClickHouse operations.
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
                "production Price Pattern writes only allow canonical parameters".to_string(),
            ));
        }
        if self.mode.writes_applied()
            && self.structure_input_table != DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE
        {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Price Pattern writes only allow structure input table {DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied()
            && self.streak_input_table != DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE
        {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Price Pattern writes only allow streak input table {DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE}"
            )));
        }
        if self.mode.writes_applied() && self.high_column != DEFAULT_PRICE_PATTERN_HIGH_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Price Pattern writes only allow high column {DEFAULT_PRICE_PATTERN_HIGH_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.low_column != DEFAULT_PRICE_PATTERN_LOW_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Price Pattern writes only allow low column {DEFAULT_PRICE_PATTERN_LOW_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.close_column != DEFAULT_PRICE_PATTERN_CLOSE_COLUMN {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Price Pattern writes only allow close column {DEFAULT_PRICE_PATTERN_CLOSE_COLUMN}"
            )));
        }
        if self.mode.writes_applied()
            && self.prev_close_column != DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN
        {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production Price Pattern writes only allow prev close column {DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN}"
            )));
        }
        if self.mode.writes_applied() && self.insert_batch_size < MIN_INSERT_BATCH_SIZE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production insert batch size must be at least {MIN_INSERT_BATCH_SIZE}"
            )));
        }
        validate_table_name("structure_input_table", &self.structure_input_table)?;
        validate_table_name("streak_input_table", &self.streak_input_table)?;
        validate_table_name("output_table", &self.output_table)?;
        validate_identifier("high_column", &self.high_column)?;
        validate_identifier("low_column", &self.low_column)?;
        validate_identifier("close_column", &self.close_column)?;
        validate_identifier("prev_close_column", &self.prev_close_column)?;
        Ok(())
    }
}

impl Default for PricePatternRunRequest {
    fn default() -> Self {
        Self {
            request_from: String::new(),
            request_to: String::new(),
            symbols: Vec::new(),
            run_id: None,
            mode: PricePatternWriteMode::DryRun,
            params: PricePatternParams::default(),
            structure_input_table: DEFAULT_PRICE_PATTERN_STRUCTURE_INPUT_TABLE.to_string(),
            streak_input_table: DEFAULT_PRICE_PATTERN_STREAK_INPUT_TABLE.to_string(),
            output_table: DEFAULT_PRICE_PATTERN_OUTPUT_TABLE.to_string(),
            high_column: DEFAULT_PRICE_PATTERN_HIGH_COLUMN.to_string(),
            low_column: DEFAULT_PRICE_PATTERN_LOW_COLUMN.to_string(),
            close_column: DEFAULT_PRICE_PATTERN_CLOSE_COLUMN.to_string(),
            prev_close_column: DEFAULT_PRICE_PATTERN_PREV_CLOSE_COLUMN.to_string(),
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
        }
    }
}
