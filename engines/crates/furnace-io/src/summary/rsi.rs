use super::*;

/// Furnace RSI 单次运行输出摘要。
#[derive(Debug, Clone, PartialEq)]
pub struct RsiRunSummary {
    /// 请求输出的起始日期。
    pub request_from: String,
    /// 请求输出的结束日期。
    pub request_to: String,
    /// 实际写入输出的起始日期。
    pub effective_output_from: String,
    /// 实际写入输出的结束日期。
    pub effective_output_to: String,
    /// 实际读取输入的起始日期。
    pub input_from: String,
    /// 实际读取输入的结束日期。
    pub input_to: String,
    /// 写入模式。
    pub mode: RsiWriteMode,
    /// 本次运行选中的证券。
    pub symbols: Vec<String>,
    /// 输入行数。
    pub input_rows: u64,
    /// 输出行数。
    pub output_rows: u64,
    /// 有效 close 行数。
    pub valid_close_rows: u64,
    /// 所有业务指标值均不可用的输出行数。
    pub null_indicator_rows: u64,
    /// 受影响的 ClickHouse 年度分区。
    pub affected_years: Vec<u16>,
    /// staging 分区中保留的旧行数。
    pub retained_rows: u64,
    /// 本次运行使用的临时 staging 表；未使用时为空。
    pub staging_table: Option<String>,
    /// staging 表校验结果。
    pub staging_validation: ValidationSummary,
    /// 分区替换结果。
    pub partition_replace: PartitionReplaceSummary,
    /// 历史 RSI 状态来源摘要。
    pub rsi_state_source: String,
    /// 有结果缺口的证券数量。
    pub gap_symbols_count: u64,
    /// 建议补算起点。
    pub gap_fill_from: Option<String>,
    /// 来自 Dagster 或 Furnace CLI 的运行标识。
    pub run_id: Option<String>,
    /// 是否实际写入了生产数据。
    pub writes_applied: bool,
    /// 内部耗时和吞吐指标。
    pub performance_metrics: PerformanceMetrics,
}

impl RsiRunSummary {
    /// 将摘要序列化为 JSON。
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let rsi_windows = DEFAULT_RSI_WINDOWS
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"indicator\":\"rsi\",\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"input_rows\":{},\"output_rows\":{},\"valid_close_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"rsi_windows\":[{}],\"rsi_state_source\":\"{}\",\"gap_symbols_count\":{},\"gap_fill_from\":{},\"run_id\":{},\"writes_applied\":{},\"performance_metrics\":{}}}",
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
            rsi_windows,
            escape_json_string(&self.rsi_state_source),
            self.gap_symbols_count,
            json_optional_string(self.gap_fill_from.as_deref()),
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied,
            self.performance_metrics.to_json()
        )
    }
}
