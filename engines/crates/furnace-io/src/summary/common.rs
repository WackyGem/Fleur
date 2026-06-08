use std::time::{Duration, Instant};

use furnace_core::DEFAULT_BOLL_CONFIGS;

use crate::FurnaceIoError;

/// Furnace KDJ 单次运行输出的性能指标。
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceMetrics {
    /// `run_kdj` 内部的端到端耗时。
    pub total_ms: u128,
    /// 读取价格输入行的耗时。
    pub read_input_ms: u128,
    /// 读取上一轮 K/D 状态的耗时。
    pub read_state_ms: u128,
    /// 按证券分组输入行的耗时。
    pub group_ms: u128,
    /// 计算 KDJ 输出的耗时。
    pub compute_ms: u128,
    /// 插入新 KDJ 行的耗时。
    pub write_ms: u128,
    /// staging DDL、旧行保留、校验和清理的总耗时。
    pub staging_ms: u128,
    /// 替换 ClickHouse 分区的耗时。
    pub partition_replace_ms: u128,
    /// 输入读取阶段的每秒读取行数。
    pub input_rows_per_sec: f64,
    /// 计算阶段的每秒输出行数。
    pub output_rows_per_sec: f64,
    /// 本次运行包含的证券数量。
    pub symbols_count: u64,
    /// Furnace 使用的计算策略。
    pub parallelism: String,
    /// 本次运行可用的 Rayon worker 线程数。
    pub worker_threads: usize,
}

impl PerformanceMetrics {
    pub(super) fn to_json(&self) -> String {
        format!(
            "{{\"total_ms\":{},\"read_input_ms\":{},\"read_state_ms\":{},\"group_ms\":{},\"compute_ms\":{},\"write_ms\":{},\"staging_ms\":{},\"partition_replace_ms\":{},\"input_rows_per_sec\":{},\"output_rows_per_sec\":{},\"symbols_count\":{},\"parallelism\":\"{}\",\"worker_threads\":{}}}",
            self.total_ms,
            self.read_input_ms,
            self.read_state_ms,
            self.group_ms,
            self.compute_ms,
            self.write_ms,
            self.staging_ms,
            self.partition_replace_ms,
            json_f64(self.input_rows_per_sec),
            json_f64(self.output_rows_per_sec),
            self.symbols_count,
            escape_json_string(&self.parallelism),
            self.worker_threads
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PerformanceTimings {
    pub(crate) run_started: Instant,
    pub(crate) read_input: Duration,
    pub(crate) read_state: Duration,
    pub(crate) group: Duration,
    pub(crate) compute: Duration,
    pub(crate) write: Duration,
    pub(crate) staging: Duration,
    pub(crate) partition_replace: Duration,
    pub(crate) parallelism: &'static str,
    pub(crate) worker_threads: usize,
}

impl PerformanceTimings {
    pub(crate) fn started() -> Self {
        Self {
            run_started: Instant::now(),
            read_input: Duration::ZERO,
            read_state: Duration::ZERO,
            group: Duration::ZERO,
            compute: Duration::ZERO,
            write: Duration::ZERO,
            staging: Duration::ZERO,
            partition_replace: Duration::ZERO,
            parallelism: "serial",
            worker_threads: rayon::current_num_threads(),
        }
    }

    pub(crate) fn finish(
        &self,
        input_rows: u64,
        output_rows: u64,
        symbols_count: u64,
    ) -> PerformanceMetrics {
        PerformanceMetrics {
            total_ms: self.run_started.elapsed().as_millis(),
            read_input_ms: self.read_input.as_millis(),
            read_state_ms: self.read_state.as_millis(),
            group_ms: self.group.as_millis(),
            compute_ms: self.compute.as_millis(),
            write_ms: self.write.as_millis(),
            staging_ms: self.staging.as_millis(),
            partition_replace_ms: self.partition_replace.as_millis(),
            input_rows_per_sec: rows_per_second(input_rows, self.read_input),
            output_rows_per_sec: rows_per_second(output_rows, self.compute),
            symbols_count,
            parallelism: self.parallelism.to_string(),
            worker_threads: self.worker_threads,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Timed<T> {
    pub(crate) value: T,
    pub(crate) elapsed: Duration,
}

pub(crate) fn time_result<T>(
    action: impl FnOnce() -> Result<T, FurnaceIoError>,
) -> Result<Timed<T>, FurnaceIoError> {
    let started = Instant::now();
    let value = action()?;
    Ok(Timed {
        value,
        elapsed: started.elapsed(),
    })
}

/// staging 表校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSummary {
    /// 校验状态。
    pub status: String,
    /// 重复键数量。
    pub duplicate_keys: u64,
}

impl ValidationSummary {
    pub(crate) fn not_applicable() -> Self {
        Self {
            status: "not_applicable".to_string(),
            duplicate_keys: 0,
        }
    }

    pub(crate) fn passed() -> Self {
        Self {
            status: "passed".to_string(),
            duplicate_keys: 0,
        }
    }

    pub(super) fn to_json(&self) -> String {
        format!(
            "{{\"status\":\"{}\",\"duplicate_keys\":{}}}",
            escape_json_string(&self.status),
            self.duplicate_keys
        )
    }
}

/// 分区替换结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionReplaceSummary {
    /// 替换状态。
    pub status: String,
    /// 已替换的年份分区。
    pub years: Vec<u16>,
}

impl PartitionReplaceSummary {
    pub(crate) fn not_applicable() -> Self {
        Self {
            status: "not_applicable".to_string(),
            years: Vec::new(),
        }
    }

    pub(crate) fn replaced(years: Vec<u16>) -> Self {
        Self {
            status: "replaced".to_string(),
            years,
        }
    }

    pub(super) fn to_json(&self) -> String {
        let years = self
            .years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"status\":\"{}\",\"years\":[{}]}}",
            escape_json_string(&self.status),
            years
        )
    }
}

pub(super) fn json_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json_string(value)),
        None => "null".to_string(),
    }
}

pub(super) fn boll_configs_json() -> String {
    let configs = DEFAULT_BOLL_CONFIGS
        .iter()
        .map(|config| {
            format!(
                "{{\"window\":{},\"multiplier\":{},\"field_suffix\":\"{}\"}}",
                config.window,
                json_f64(config.multiplier),
                escape_json_string(config.field_suffix)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{configs}]")
}

pub(super) fn json_f64(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "0".to_string()
    }
}

pub(super) fn rows_per_second(rows: u64, elapsed: Duration) -> f64 {
    let seconds = elapsed.as_secs_f64();
    if seconds == 0.0 {
        return 0.0;
    }
    rows as f64 / seconds
}

pub(super) fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    escaped
}
