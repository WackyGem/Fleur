use super::*;

/// Furnace MACD single-run output summary.
#[derive(Debug, Clone, PartialEq)]
pub struct MacdRunSummary {
    /// Requested output start date.
    pub request_from: String,
    /// Requested output end date.
    pub request_to: String,
    /// Effective output start date.
    pub effective_output_from: String,
    /// Effective output end date.
    pub effective_output_to: String,
    /// Actual input start date read for calculation.
    pub input_from: String,
    /// Actual input end date read for calculation.
    pub input_to: String,
    /// Write mode.
    pub mode: MacdWriteMode,
    /// Selected securities.
    pub symbols: Vec<String>,
    /// Input row count.
    pub input_rows: u64,
    /// Output row count.
    pub output_rows: u64,
    /// Valid close input row count.
    pub valid_close_rows: u64,
    /// Output rows with all business indicator fields null.
    pub null_indicator_rows: u64,
    /// Affected ClickHouse yearly partitions.
    pub affected_years: Vec<u16>,
    /// Existing rows retained in staging partitions.
    pub retained_rows: u64,
    /// Run-scoped staging table, if used.
    pub staging_table: Option<String>,
    /// Staging validation result.
    pub staging_validation: ValidationSummary,
    /// Partition replacement result.
    pub partition_replace: PartitionReplaceSummary,
    /// MACD state source summary.
    pub macd_state_source: String,
    /// Symbols with incomplete historical MACD state before the request range.
    pub incomplete_state_symbols_count: u64,
    /// Symbols with result gaps before request range.
    pub gap_symbols_count: u64,
    /// Suggested gap fill start date.
    pub gap_fill_from: Option<String>,
    /// Run identifier from Dagster or CLI.
    pub run_id: Option<String>,
    /// Whether production writes were applied.
    pub writes_applied: bool,
    /// Internal timings and throughput.
    pub performance_metrics: PerformanceMetrics,
}

impl MacdRunSummary {
    /// Serialize the summary as JSON.
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"indicator\":\"macd\",\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"valid_close_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"macd_params\":{{\"fast_window\":{},\"slow_window\":{},\"signal_window\":{}}},\"histogram_mode\":\"DIF - DEA\",\"macd_state_source\":\"{}\",\"incomplete_state_symbols_count\":{},\"gap_symbols_count\":{},\"gap_fill_from\":{},\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
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
            self.valid_close_rows,
            self.null_indicator_rows,
            affected_years,
            self.retained_rows,
            json_optional_string(self.staging_table.as_deref()),
            self.staging_validation.to_json(),
            self.partition_replace.to_json(),
            DEFAULT_MACD_FAST_WINDOW,
            DEFAULT_MACD_SLOW_WINDOW,
            DEFAULT_MACD_SIGNAL_WINDOW,
            escape_json_string(&self.macd_state_source),
            self.incomplete_state_symbols_count,
            self.gap_symbols_count,
            json_optional_string(self.gap_fill_from.as_deref()),
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied,
            self.performance_metrics.to_json()
        )
    }
}
