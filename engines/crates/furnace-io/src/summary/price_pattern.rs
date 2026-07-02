use super::*;

/// Furnace Price Pattern single-run summary.
#[derive(Debug, Clone, PartialEq)]
pub struct PricePatternRunSummary {
    /// Requested output start date.
    pub request_from: String,
    /// Requested output end date.
    pub request_to: String,
    /// Actual output start date.
    pub effective_output_from: String,
    /// Actual output end date.
    pub effective_output_to: String,
    /// Actual input start date.
    pub input_from: String,
    /// Actual input end date.
    pub input_to: String,
    /// Write mode.
    pub mode: PricePatternWriteMode,
    /// Selected securities.
    pub symbols: Vec<String>,
    /// Input rows.
    pub input_rows: u64,
    /// Output rows.
    pub output_rows: u64,
    /// Input rows with close and previous close.
    pub input_valid_streak_rows: u64,
    /// Input rows with high and low.
    pub input_valid_structure_bar_rows: u64,
    /// Output rows with valid close direction.
    pub valid_streak_rows: u64,
    /// Output rows with valid high/low structure bars.
    pub valid_structure_bar_rows: u64,
    /// Output rows where streak fields are null.
    pub null_streak_rows: u64,
    /// Output rows where N-structure fields are null.
    pub null_n_structure_rows: u64,
    /// Affected ClickHouse year partitions.
    pub affected_years: Vec<u16>,
    /// Existing rows retained in staging partitions.
    pub retained_rows: u64,
    /// Staging table name when used.
    pub staging_table: Option<String>,
    /// Staging validation result.
    pub staging_validation: ValidationSummary,
    /// Partition replace result.
    pub partition_replace: PartitionReplaceSummary,
    /// State reconstruction source.
    pub state_source: String,
    /// Structure window.
    pub n_structure_window: usize,
    /// Run identifier.
    pub run_id: Option<String>,
    /// Whether production writes were applied.
    pub writes_applied: bool,
    /// Performance metrics.
    pub performance_metrics: PerformanceMetrics,
}

impl PricePatternRunSummary {
    /// Serialize summary as JSON.
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"indicator\":\"price_pattern\",\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"input_valid_streak_rows\":{},\"input_valid_structure_bar_rows\":{},\"valid_streak_rows\":{},\"valid_structure_bar_rows\":{},\"null_streak_rows\":{},\"null_n_structure_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"state_source\":\"{}\",\"n_structure_window\":{},\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
            escape_json_string(&self.request_from),
            escape_json_string(&self.request_to),
            escape_json_string(&self.effective_output_from),
            escape_json_string(&self.effective_output_to),
            escape_json_string(&self.input_from),
            escape_json_string(&self.input_to),
            self.mode.as_str(),
            self.symbols.len(),
            self.input_rows,
            self.output_rows,
            self.input_valid_streak_rows,
            self.input_valid_structure_bar_rows,
            self.valid_streak_rows,
            self.valid_structure_bar_rows,
            self.null_streak_rows,
            self.null_n_structure_rows,
            affected_years,
            self.retained_rows,
            json_optional_string(self.staging_table.as_deref()),
            self.staging_validation.to_json(),
            self.partition_replace.to_json(),
            escape_json_string(&self.state_source),
            self.n_structure_window,
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied,
            self.performance_metrics.to_json()
        )
    }
}
