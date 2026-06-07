//! ClickHouse I/O boundary for Furnace.
//!
//! This crate owns database-facing table names, DDL, SQL generation,
//! `clickhouse-client` execution, and run summaries. Pure indicator formulas
//! remain in `furnace-core`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::error::Error;
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

use furnace_core::{KdjInput, KdjParams, KdjState, calculate_kdj_series};

/// Default dbt intermediate input table for forward-adjusted daily prices.
pub const DEFAULT_INPUT_TABLE: &str = "fleur_intermediate.int_stock_quotes_daily_adj";

/// Furnace-owned calculation output table for daily KDJ.
pub const DEFAULT_KDJ_OUTPUT_TABLE: &str = "fleur_calculation.calc_stock_kdj_daily";

/// Default target rows per ClickHouse insert batch.
pub const DEFAULT_INSERT_BATCH_SIZE: usize = 10_000;

/// Minimum insert batch size allowed for production write modes.
pub const MIN_INSERT_BATCH_SIZE: usize = 1_000;

/// Default warmup multiple for KDJ state and RSV window construction.
pub const DEFAULT_WARMUP_MULTIPLE: u16 = 3;

/// Returns the production ClickHouse database creation SQL.
pub fn create_calculation_database_sql() -> &'static str {
    "CREATE DATABASE IF NOT EXISTS fleur_calculation"
}

/// Returns the production ClickHouse DDL for `calc_stock_kdj_daily`.
///
/// # Examples
///
/// ```
/// let ddl = furnace_io::create_kdj_output_table_sql();
/// assert!(ddl.contains("fleur_calculation.calc_stock_kdj_daily"));
/// assert!(ddl.contains("PARTITION BY toYear(trade_date)"));
/// ```
pub fn create_kdj_output_table_sql() -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {DEFAULT_KDJ_OUTPUT_TABLE}
(
    security_code String,
    trade_date Date,
    rsv_window UInt16,
    k_smoothing UInt16,
    d_smoothing UInt16,
    rsv Nullable(Float64),
    k_value Nullable(Float64),
    d_value Nullable(Float64),
    j_value Nullable(Float64)
)
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// Builds a deterministic temporary staging table name from a run id.
///
/// Non-alphanumeric characters are normalized to `_` so the result is safe to
/// interpolate as a ClickHouse identifier suffix.
pub fn kdj_staging_table_name(run_id: &str) -> String {
    let normalized = run_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    let suffix = if normalized.is_empty() {
        "manual".to_string()
    } else {
        normalized
    };
    format!("fleur_calculation.calc_stock_kdj_daily__staging__{suffix}")
}

/// Builds SQL for creating a staging table with the same schema as production.
pub fn create_kdj_staging_table_sql(staging_table: &str) -> String {
    format!(
        "\
CREATE TABLE IF NOT EXISTS {staging_table}
AS {DEFAULT_KDJ_OUTPUT_TABLE}
ENGINE = MergeTree()
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)"
    )
}

/// Builds SQL for replacing one yearly partition from staging into production.
pub fn replace_kdj_partition_sql(staging_table: &str, year: u16) -> String {
    format!("ALTER TABLE {DEFAULT_KDJ_OUTPUT_TABLE} REPLACE PARTITION {year} FROM {staging_table}")
}

/// Builds SQL for dropping a temporary staging table.
pub fn drop_kdj_staging_table_sql(staging_table: &str) -> String {
    format!("DROP TABLE IF EXISTS {staging_table}")
}

/// KDJ write mode requested by the CLI or Dagster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdjWriteMode {
    /// Compute and summarize without writing ClickHouse.
    DryRun,
    /// Append latest range when no same-or-later result already exists.
    AppendLatest,
    /// Recompute a historical range and cascade to latest affected input.
    ReplaceCascade,
}

impl KdjWriteMode {
    /// Parses the CLI spelling for this mode.
    ///
    /// # Errors
    ///
    /// Returns [`FurnaceIoError::InvalidRequest`] when `value` is not one of
    /// `dry-run`, `append-latest`, or `replace-cascade`.
    pub fn parse(value: &str) -> Result<Self, FurnaceIoError> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "append-latest" => Ok(Self::AppendLatest),
            "replace-cascade" => Ok(Self::ReplaceCascade),
            other => Err(FurnaceIoError::InvalidRequest(format!(
                "invalid KDJ write mode: {other}"
            ))),
        }
    }

    /// Returns the CLI spelling for this mode.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::AppendLatest => "append-latest",
            Self::ReplaceCascade => "replace-cascade",
        }
    }

    /// Returns true when the mode writes production ClickHouse data.
    pub fn writes_applied(self) -> bool {
        !matches!(self, Self::DryRun)
    }
}

/// Request for one Furnace KDJ run.
#[derive(Debug, Clone, PartialEq)]
pub struct KdjRunRequest {
    /// Requested output start date.
    pub request_from: String,
    /// Requested output end date.
    pub request_to: String,
    /// Optional security-code allowlist. Empty means infer from input rows.
    pub symbols: Vec<String>,
    /// Run identifier from Dagster or Furnace CLI.
    pub run_id: Option<String>,
    /// Write mode.
    pub mode: KdjWriteMode,
    /// KDJ parameters.
    pub params: KdjParams,
    /// Target ClickHouse rows per insert batch.
    pub insert_batch_size: usize,
}

impl KdjRunRequest {
    /// Validates a request before any ClickHouse work.
    ///
    /// # Errors
    ///
    /// Returns [`FurnaceIoError::InvalidRequest`] when dates, parameters, or
    /// batch-size settings cannot be used safely.
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
                "production KDJ writes only allow canonical parameters 9/3/3".to_string(),
            ));
        }
        if self.mode.writes_applied() && self.insert_batch_size < MIN_INSERT_BATCH_SIZE {
            return Err(FurnaceIoError::InvalidRequest(format!(
                "production insert batch size must be at least {MIN_INSERT_BATCH_SIZE}"
            )));
        }
        Ok(())
    }
}

impl Default for KdjRunRequest {
    fn default() -> Self {
        Self {
            request_from: String::new(),
            request_to: String::new(),
            symbols: Vec::new(),
            run_id: None,
            mode: KdjWriteMode::DryRun,
            params: KdjParams::default(),
            insert_batch_size: DEFAULT_INSERT_BATCH_SIZE,
        }
    }
}

/// Summary emitted by a Furnace KDJ run.
#[derive(Debug, Clone, PartialEq)]
pub struct KdjRunSummary {
    /// Requested output start date.
    pub request_from: String,
    /// Requested output end date.
    pub request_to: String,
    /// Effective written output start date.
    pub effective_output_from: String,
    /// Effective written output end date.
    pub effective_output_to: String,
    /// Actual input start date.
    pub input_from: String,
    /// Actual input end date.
    pub input_to: String,
    /// Write mode.
    pub mode: KdjWriteMode,
    /// Selected securities.
    pub symbols: Vec<String>,
    /// Input row count.
    pub input_rows: u64,
    /// Output row count.
    pub output_rows: u64,
    /// Output rows with all indicator values unavailable.
    pub null_indicator_rows: u64,
    /// Affected yearly ClickHouse partitions.
    pub affected_years: Vec<u16>,
    /// Old rows retained in staging partitions.
    pub retained_rows: u64,
    /// Temporary staging table, when used.
    pub staging_table: Option<String>,
    /// Staging validation result.
    pub staging_validation: ValidationSummary,
    /// Partition replacement result.
    pub partition_replace: PartitionReplaceSummary,
    /// KDJ parameters.
    pub params: KdjParams,
    /// State source summary.
    pub state_source: String,
    /// Run identifier from Dagster or Furnace CLI.
    pub run_id: Option<String>,
    /// Whether production writes were applied.
    pub writes_applied: bool,
}

impl KdjRunSummary {
    /// Serializes the summary as JSON without requiring a runtime dependency.
    pub fn to_json(&self) -> String {
        let affected_years = self
            .affected_years
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"request_from\":\"{}\",\"request_to\":\"{}\",\"effective_output_from\":\"{}\",\"effective_output_to\":\"{}\",\"input_from\":\"{}\",\"input_to\":\"{}\",\"mode\":\"{}\",\"symbols_count\":{},\"symbols\":{},\"input_rows\":{},\"output_rows\":{},\"null_indicator_rows\":{},\"affected_years\":[{}],\"retained_rows\":{},\"staging_table\":{},\"staging_validation\":{},\"partition_replace\":{},\"kdj_params\":{{\"rsv_window\":{},\"k_smoothing\":{},\"d_smoothing\":{}}},\"state_source\":\"{}\",\"run_id\":{},\"writes_applied\":{}}}",
            escape_json_string(&self.request_from),
            escape_json_string(&self.request_to),
            escape_json_string(&self.effective_output_from),
            escape_json_string(&self.effective_output_to),
            escape_json_string(&self.input_from),
            escape_json_string(&self.input_to),
            self.mode.as_str(),
            self.symbols.len(),
            json_string_array(&self.symbols),
            self.input_rows,
            self.output_rows,
            self.null_indicator_rows,
            affected_years,
            self.retained_rows,
            json_optional_string(self.staging_table.as_deref()),
            self.staging_validation.to_json(),
            self.partition_replace.to_json(),
            self.params.rsv_window,
            self.params.k_smoothing,
            self.params.d_smoothing,
            escape_json_string(&self.state_source),
            json_optional_string(self.run_id.as_deref()),
            self.writes_applied
        )
    }
}

/// Staging validation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSummary {
    /// Validation status.
    pub status: String,
    /// Duplicate key count.
    pub duplicate_keys: u64,
}

impl ValidationSummary {
    fn not_applicable() -> Self {
        Self {
            status: "not_applicable".to_string(),
            duplicate_keys: 0,
        }
    }

    fn passed() -> Self {
        Self {
            status: "passed".to_string(),
            duplicate_keys: 0,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"status\":\"{}\",\"duplicate_keys\":{}}}",
            escape_json_string(&self.status),
            self.duplicate_keys
        )
    }
}

/// Partition replacement result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionReplaceSummary {
    /// Replacement status.
    pub status: String,
    /// Years replaced.
    pub years: Vec<u16>,
}

impl PartitionReplaceSummary {
    fn not_applicable() -> Self {
        Self {
            status: "not_applicable".to_string(),
            years: Vec::new(),
        }
    }

    fn replaced(years: Vec<u16>) -> Self {
        Self {
            status: "replaced".to_string(),
            years,
        }
    }

    fn to_json(&self) -> String {
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

/// A minimal ClickHouse execution interface for tests and CLI adapters.
pub trait ClickHouseExecutor {
    /// Executes a query and returns stdout.
    ///
    /// # Errors
    ///
    /// Returns [`FurnaceIoError`] when ClickHouse execution fails.
    fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError>;

    /// Executes an INSERT statement with TSV rows provided on stdin.
    ///
    /// # Errors
    ///
    /// Returns [`FurnaceIoError`] when ClickHouse execution fails.
    fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError>;

    /// Executes a statement whose stdout is ignored.
    ///
    /// # Errors
    ///
    /// Returns [`FurnaceIoError`] when ClickHouse execution fails.
    fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError> {
        self.query(sql).map(|_| ())
    }
}

/// `clickhouse-client` subprocess executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseCliExecutor {
    command: String,
    command_args: Vec<String>,
    host: String,
    port: String,
    user: Option<String>,
    password: Option<String>,
    secure: bool,
    connect_timeout_seconds: Option<String>,
    query_timeout_seconds: Option<String>,
}

impl ClickHouseCliExecutor {
    /// Builds a CLI executor from environment variables.
    ///
    /// Supported variables: `FURNACE_CLICKHOUSE_CLIENT`, `CLICKHOUSE_HOST`,
    /// `FURNACE_CLICKHOUSE_CLIENT_ARGS`, `CLICKHOUSE_NATIVE_PORT`,
    /// `CLICKHOUSE_USER`, `CLICKHOUSE_PASSWORD`, `CLICKHOUSE_SECURE`,
    /// `CLICKHOUSE_CONNECT_TIMEOUT_SECONDS`, and `CLICKHOUSE_QUERY_TIMEOUT_SECONDS`.
    pub fn from_env() -> Self {
        Self {
            command: env::var("FURNACE_CLICKHOUSE_CLIENT")
                .or_else(|_| env::var("CLICKHOUSE_CLIENT"))
                .unwrap_or_else(|_| "clickhouse-client".to_string()),
            command_args: env::var("FURNACE_CLICKHOUSE_CLIENT_ARGS")
                .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
                .unwrap_or_default(),
            host: env::var("CLICKHOUSE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("CLICKHOUSE_NATIVE_PORT").unwrap_or_else(|_| "9000".to_string()),
            user: env::var("CLICKHOUSE_USER").ok(),
            password: env::var("CLICKHOUSE_PASSWORD").ok(),
            secure: env::var("CLICKHOUSE_SECURE")
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false),
            connect_timeout_seconds: env::var("CLICKHOUSE_CONNECT_TIMEOUT_SECONDS").ok(),
            query_timeout_seconds: env::var("CLICKHOUSE_QUERY_TIMEOUT_SECONDS").ok(),
        }
    }

    fn base_command(&self) -> Command {
        let mut command = Command::new(&self.command);
        command.args(&self.command_args);
        command.arg("--host").arg(&self.host);
        command.arg("--port").arg(&self.port);
        if let Some(user) = &self.user {
            command.arg("--user").arg(user);
        }
        if let Some(password) = &self.password {
            command.arg("--password").arg(password);
        }
        if self.secure {
            command.arg("--secure");
        }
        if let Some(timeout) = &self.connect_timeout_seconds {
            command.arg("--connect_timeout").arg(timeout);
        }
        if let Some(timeout) = &self.query_timeout_seconds {
            command.arg("--receive_timeout").arg(timeout);
            command.arg("--send_timeout").arg(timeout);
        }
        command
    }
}

impl ClickHouseExecutor for ClickHouseCliExecutor {
    fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError> {
        let output = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .output()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError> {
        let mut child = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;

        let Some(stdin) = child.stdin.as_mut() else {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: "failed to open clickhouse-client stdin".to_string(),
                source: None,
            });
        };
        stdin
            .write_all(tsv.as_bytes())
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: "failed to write TSV rows to clickhouse-client".to_string(),
                source: Some(source.to_string()),
            })?;

        let output =
            child
                .wait_with_output()
                .map_err(|source| FurnaceIoError::ClickHouseCommand {
                    message: format!("failed to wait for {}", self.command),
                    source: Some(source.to_string()),
                })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(())
    }
}

/// Errors returned by Furnace I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FurnaceIoError {
    /// Request cannot be executed safely.
    InvalidRequest(String),
    /// ClickHouse subprocess or query failed.
    ClickHouseCommand {
        /// Error summary.
        message: String,
        /// Optional stderr/source detail.
        source: Option<String>,
    },
    /// ClickHouse output could not be parsed.
    Parse(String),
    /// Indicator calculation failed.
    Compute(String),
}

impl fmt::Display for FurnaceIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::Parse(message) | Self::Compute(message) => {
                f.write_str(message)
            }
            Self::ClickHouseCommand { message, source } => {
                if let Some(source) = source {
                    write!(f, "{message}: {source}")
                } else {
                    f.write_str(message)
                }
            }
        }
    }
}

impl Error for FurnaceIoError {}

#[derive(Debug, Clone, PartialEq)]
struct PriceInputRow {
    security_code: String,
    trade_date: String,
    high_price: Option<f64>,
    low_price: Option<f64>,
    close_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct KdjResultRow {
    security_code: String,
    trade_date: String,
    rsv_window: u16,
    k_smoothing: u16,
    d_smoothing: u16,
    rsv: Option<f64>,
    k_value: Option<f64>,
    d_value: Option<f64>,
    j_value: Option<f64>,
}

/// Runs a full KDJ calculation against ClickHouse.
///
/// # Errors
///
/// Returns [`FurnaceIoError`] when validation, ClickHouse I/O, or indicator
/// computation fails.
pub fn run_kdj<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
) -> Result<KdjRunSummary, FurnaceIoError> {
    request.validate()?;

    if request.mode.writes_applied() {
        executor.execute(create_calculation_database_sql())?;
        executor.execute(&create_kdj_output_table_sql())?;
    }

    let symbols = resolve_symbols(executor, request)?;
    if request.mode.writes_applied() && symbols.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production KDJ writes require at least one input security".to_string(),
        ));
    }
    let effective_output_to = resolve_effective_output_to(executor, request, &symbols)?;
    let input_from = resolve_input_from(executor, request, &symbols)?;
    let target_exists = target_table_exists(executor)?;
    let states = if target_exists {
        read_previous_states(executor, request, &symbols)?
    } else {
        HashMap::new()
    };
    let input_rows = read_input_rows(
        executor,
        request,
        &symbols,
        &input_from,
        &effective_output_to,
    )?;
    let output_rows = calculate_outputs(request, &effective_output_to, &input_rows, &states)?;
    if request.mode.writes_applied() && output_rows.is_empty() {
        return Err(FurnaceIoError::InvalidRequest(
            "production KDJ writes produced no output rows".to_string(),
        ));
    }
    let affected_years = affected_years(&request.request_from, &effective_output_to)?;
    let null_indicator_rows = output_rows
        .iter()
        .filter(|row| row.rsv.is_none() && row.k_value.is_none() && row.d_value.is_none())
        .count() as u64;

    let mut retained_rows = 0;
    let mut staging_table = None;
    let mut staging_validation = ValidationSummary::not_applicable();
    let mut partition_replace = PartitionReplaceSummary::not_applicable();

    match request.mode {
        KdjWriteMode::DryRun => {}
        KdjWriteMode::AppendLatest => {
            ensure_append_latest_is_safe(executor, request, &symbols)?;
            insert_result_rows(
                executor,
                DEFAULT_KDJ_OUTPUT_TABLE,
                &output_rows,
                request.insert_batch_size,
            )?;
        }
        KdjWriteMode::ReplaceCascade => {
            let run_id = request
                .run_id
                .as_deref()
                .unwrap_or("manual_replace_cascade");
            let staging = kdj_staging_table_name(run_id);
            executor.execute(&drop_kdj_staging_table_sql(&staging))?;
            executor.execute(&create_kdj_staging_table_sql(&staging))?;
            retained_rows = retain_old_rows_for_staging(
                executor,
                request,
                &staging,
                &symbols,
                &affected_years,
                &effective_output_to,
            )?;
            insert_result_rows(executor, &staging, &output_rows, request.insert_batch_size)?;
            staging_validation = validate_staging(executor, &staging, &affected_years)?;
            if staging_validation.status != "passed" {
                return Err(FurnaceIoError::InvalidRequest(format!(
                    "staging validation failed with {} duplicate keys",
                    staging_validation.duplicate_keys
                )));
            }
            for year in &affected_years {
                executor.execute(&replace_kdj_partition_sql(&staging, *year))?;
            }
            executor.execute(&drop_kdj_staging_table_sql(&staging))?;
            partition_replace = PartitionReplaceSummary::replaced(affected_years.clone());
            staging_table = Some(staging);
        }
    }

    let state_source = if states.is_empty() {
        "initial_50".to_string()
    } else {
        format!("previous_kd_rows:{}", states.len())
    };

    Ok(KdjRunSummary {
        request_from: request.request_from.clone(),
        request_to: request.request_to.clone(),
        effective_output_from: request.request_from.clone(),
        effective_output_to: effective_output_to.clone(),
        input_from,
        input_to: effective_output_to,
        mode: request.mode,
        symbols,
        input_rows: input_rows.len() as u64,
        output_rows: output_rows.len() as u64,
        null_indicator_rows,
        affected_years,
        retained_rows,
        staging_table,
        staging_validation,
        partition_replace,
        params: request.params,
        state_source,
        run_id: request.run_id.clone(),
        writes_applied: request.mode.writes_applied(),
    })
}

fn resolve_symbols<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
) -> Result<Vec<String>, FurnaceIoError> {
    if !request.symbols.is_empty() {
        return Ok(normalize_symbols(&request.symbols));
    }

    let sql = format!(
        "\
SELECT security_code
FROM {DEFAULT_INPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
GROUP BY security_code
ORDER BY security_code
FORMAT TSV",
        sql_string(&request.request_from),
        sql_string(&request.request_to)
    );
    parse_single_column_strings(&executor.query(&sql)?)
}

fn normalize_symbols(symbols: &[String]) -> Vec<String> {
    let mut unique = BTreeSet::new();
    for symbol in symbols {
        let symbol = symbol.trim();
        if !symbol.is_empty() {
            unique.insert(symbol.to_string());
        }
    }
    unique.into_iter().collect()
}

fn resolve_effective_output_to<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
) -> Result<String, FurnaceIoError> {
    if symbols.is_empty() || request.mode != KdjWriteMode::ReplaceCascade {
        return Ok(request.request_to.clone());
    }
    let sql = format!(
        "\
SELECT toString(max(trade_date))
FROM {DEFAULT_INPUT_TABLE}
WHERE {}
FORMAT TSV",
        symbol_where_clause(symbols)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_to.clone());
    if value.is_empty() || value == "\\N" {
        return Ok(request.request_to.clone());
    }
    Ok(value.max(request.request_to.clone()))
}

fn resolve_input_from<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
) -> Result<String, FurnaceIoError> {
    let warmup_window = u32::from(
        request
            .params
            .rsv_window
            .max(request.params.k_smoothing)
            .max(request.params.d_smoothing),
    ) * u32::from(DEFAULT_WARMUP_MULTIPLE);

    let symbol_filter = if symbols.is_empty() {
        "1 = 1".to_string()
    } else {
        symbol_where_clause(symbols)
    };
    let sql = format!(
        "\
SELECT toString(min(trade_date))
FROM (
    SELECT trade_date
    FROM {DEFAULT_INPUT_TABLE}
    WHERE trade_date <= toDate('{}')
      AND {symbol_filter}
    GROUP BY trade_date
    ORDER BY trade_date DESC
    LIMIT {warmup_window}
)
FORMAT TSV",
        sql_string(&request.request_from)
    );
    let value =
        first_tsv_value(&executor.query(&sql)?).unwrap_or_else(|| request.request_from.clone());
    if value.is_empty() || value == "\\N" {
        Ok(request.request_from.clone())
    } else {
        Ok(value)
    }
}

fn target_table_exists<E: ClickHouseExecutor>(executor: &mut E) -> Result<bool, FurnaceIoError> {
    let value = first_tsv_value(&executor.query(&format!(
        "EXISTS TABLE {DEFAULT_KDJ_OUTPUT_TABLE} FORMAT TSV"
    ))?)
    .unwrap_or_else(|| "0".to_string());
    Ok(value == "1")
}

fn read_previous_states<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
) -> Result<HashMap<String, KdjState>, FurnaceIoError> {
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "\
SELECT security_code, k_value, d_value
FROM (
    SELECT
        security_code,
        trade_date,
        k_value,
        d_value,
        row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC) AS rn
    FROM {DEFAULT_KDJ_OUTPUT_TABLE}
    WHERE trade_date < toDate('{}')
      AND k_value IS NOT NULL
      AND d_value IS NOT NULL
      AND {}
)
WHERE rn = 1
FORMAT TSV",
        sql_string(&request.request_from),
        symbol_where_clause(symbols)
    );

    let mut states = HashMap::new();
    for line in executor
        .query(&sql)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 3 previous-state fields, got {}",
                fields.len()
            )));
        }
        let k_value = parse_f64(fields[1])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous k_value must not be null".to_string())
        })?;
        let d_value = parse_f64(fields[2])?.ok_or_else(|| {
            FurnaceIoError::Parse("previous d_value must not be null".to_string())
        })?;
        states.insert(fields[0].to_string(), KdjState::new(k_value, d_value));
    }
    Ok(states)
}

fn read_input_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
    input_from: &str,
    input_to: &str,
) -> Result<Vec<PriceInputRow>, FurnaceIoError> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "\
SELECT
    security_code,
    toString(trade_date),
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj
FROM {DEFAULT_INPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND trade_date <= toDate('{}')
  AND {}
ORDER BY security_code, trade_date
FORMAT TSV",
        sql_string(input_from),
        sql_string(input_to),
        symbol_where_clause(symbols)
    );

    let mut rows = Vec::new();
    for line in executor
        .query(&sql)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(FurnaceIoError::Parse(format!(
                "expected 5 input fields, got {}",
                fields.len()
            )));
        }
        rows.push(PriceInputRow {
            security_code: fields[0].to_string(),
            trade_date: fields[1].to_string(),
            high_price: parse_f64(fields[2])?,
            low_price: parse_f64(fields[3])?,
            close_price: parse_f64(fields[4])?,
        });
    }
    let _ = request;
    Ok(rows)
}

fn calculate_outputs(
    request: &KdjRunRequest,
    effective_output_to: &str,
    input_rows: &[PriceInputRow],
    states: &HashMap<String, KdjState>,
) -> Result<Vec<KdjResultRow>, FurnaceIoError> {
    let mut grouped: BTreeMap<&str, Vec<KdjInput>> = BTreeMap::new();
    for row in input_rows {
        grouped
            .entry(&row.security_code)
            .or_default()
            .push(KdjInput::new(
                row.trade_date.clone(),
                row.high_price,
                row.low_price,
                row.close_price,
            ));
    }

    let mut output_rows = Vec::new();
    for (security_code, inputs) in grouped {
        let previous_state = states.get(security_code).copied();
        let outputs = calculate_kdj_series(&inputs, request.params, previous_state)
            .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
        for output in outputs {
            if output.trade_date.as_str() < request.request_from.as_str()
                || output.trade_date.as_str() > effective_output_to
            {
                continue;
            }
            output_rows.push(KdjResultRow {
                security_code: security_code.to_string(),
                trade_date: output.trade_date,
                rsv_window: request.params.rsv_window,
                k_smoothing: request.params.k_smoothing,
                d_smoothing: request.params.d_smoothing,
                rsv: output.rsv,
                k_value: output.k_value,
                d_value: output.d_value,
                j_value: output.j_value,
            });
        }
    }
    Ok(output_rows)
}

fn ensure_append_latest_is_safe<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    symbols: &[String],
) -> Result<(), FurnaceIoError> {
    if symbols.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "\
SELECT count()
FROM {DEFAULT_KDJ_OUTPUT_TABLE}
WHERE trade_date >= toDate('{}')
  AND {}
FORMAT TSV",
        sql_string(&request.request_from),
        symbol_where_clause(symbols)
    );
    let existing_rows = parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())?;
    if existing_rows > 0 {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "append-latest found {existing_rows} existing same-or-later result rows; use replace-cascade"
        )));
    }
    Ok(())
}

fn retain_old_rows_for_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    request: &KdjRunRequest,
    staging_table: &str,
    symbols: &[String],
    years: &[u16],
    effective_output_to: &str,
) -> Result<u64, FurnaceIoError> {
    let mut retained = 0;
    for year in years {
        let sql = format!(
            "\
INSERT INTO {staging_table}
SELECT *
FROM {DEFAULT_KDJ_OUTPUT_TABLE}
WHERE toYear(trade_date) = {year}
  AND NOT (
      {}
      AND trade_date >= toDate('{}')
      AND trade_date <= toDate('{}')
  )",
            symbol_where_clause(symbols),
            sql_string(&request.request_from),
            sql_string(effective_output_to)
        );
        executor.execute(&sql)?;
        retained += count_year_rows(executor, staging_table, *year)?;
    }
    Ok(retained)
}

fn validate_staging<E: ClickHouseExecutor>(
    executor: &mut E,
    staging_table: &str,
    years: &[u16],
) -> Result<ValidationSummary, FurnaceIoError> {
    for year in years {
        let sql = format!(
            "\
SELECT count() - uniqExact(security_code, trade_date)
FROM {staging_table}
WHERE toYear(trade_date) = {year}
FORMAT TSV"
        );
        let duplicates = parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())?;
        if duplicates > 0 {
            return Ok(ValidationSummary {
                status: "failed".to_string(),
                duplicate_keys: duplicates,
            });
        }
    }
    Ok(ValidationSummary::passed())
}

fn count_year_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    year: u16,
) -> Result<u64, FurnaceIoError> {
    let sql = format!(
        "\
SELECT count()
FROM {table}
WHERE toYear(trade_date) = {year}
FORMAT TSV"
    );
    parse_u64(&first_tsv_value(&executor.query(&sql)?).unwrap_or_default())
}

fn insert_result_rows<E: ClickHouseExecutor>(
    executor: &mut E,
    table: &str,
    rows: &[KdjResultRow],
    batch_size: usize,
) -> Result<(), FurnaceIoError> {
    if rows.is_empty() {
        return Ok(());
    }
    let insert_sql = format!(
        "\
INSERT INTO {table}
(
    security_code,
    trade_date,
    rsv_window,
    k_smoothing,
    d_smoothing,
    rsv,
    k_value,
    d_value,
    j_value
)
FORMAT TSV"
    );
    for batch in rows.chunks(batch_size) {
        let mut tsv = String::new();
        for row in batch {
            tsv.push_str(&row.to_tsv());
            tsv.push('\n');
        }
        executor.insert_tsv(&insert_sql, &tsv)?;
    }
    Ok(())
}

impl KdjResultRow {
    fn to_tsv(&self) -> String {
        [
            escape_tsv(&self.security_code),
            escape_tsv(&self.trade_date),
            self.rsv_window.to_string(),
            self.k_smoothing.to_string(),
            self.d_smoothing.to_string(),
            tsv_f64(self.rsv),
            tsv_f64(self.k_value),
            tsv_f64(self.d_value),
            tsv_f64(self.j_value),
        ]
        .join("\t")
    }
}

fn affected_years(from: &str, to: &str) -> Result<Vec<u16>, FurnaceIoError> {
    let from_year = parse_year(from)?;
    let to_year = parse_year(to)?;
    Ok((from_year..=to_year).collect())
}

fn parse_year(date: &str) -> Result<u16, FurnaceIoError> {
    validate_date("date", date)?;
    date[0..4]
        .parse::<u16>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date year: {date}")))
}

fn validate_date(name: &str, value: &str) -> Result<(), FurnaceIoError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[0..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must use YYYY-MM-DD format"
        )));
    }
    Ok(())
}

fn parse_single_column_strings(output: &str) -> Result<Vec<String>, FurnaceIoError> {
    Ok(output
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn first_tsv_value(output: &str) -> Option<String> {
    output.lines().next().map(|line| line.trim().to_string())
}

fn parse_f64(value: &str) -> Result<Option<f64>, FurnaceIoError> {
    if value == "\\N" || value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|_| FurnaceIoError::Parse(format!("invalid Float64 value: {value}")))
}

fn parse_u64(value: &str) -> Result<u64, FurnaceIoError> {
    if value.is_empty() || value == "\\N" {
        return Ok(0);
    }
    value
        .parse::<u64>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid UInt64 value: {value}")))
}

fn symbol_where_clause(symbols: &[String]) -> String {
    if symbols.is_empty() {
        return "1 = 0".to_string();
    }
    let values = symbols
        .iter()
        .map(|symbol| format!("'{}'", sql_string(symbol)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("security_code IN ({values})")
}

fn sql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn tsv_f64(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "\\N".to_string())
}

fn escape_tsv(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn json_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json_string(value)),
        None => "null".to_string(),
    }
}

fn json_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", escape_json_string(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn escape_json_string(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct FakeExecutor {
        queries: Vec<String>,
        inserts: Vec<(String, String)>,
        responses: Vec<String>,
    }

    impl FakeExecutor {
        fn with_responses(responses: &[&str]) -> Self {
            Self {
                responses: responses.iter().map(ToString::to_string).collect(),
                ..Self::default()
            }
        }
    }

    impl ClickHouseExecutor for FakeExecutor {
        fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError> {
            self.queries.push(sql.to_string());
            if self.responses.is_empty() {
                return Ok(String::new());
            }
            Ok(self.responses.remove(0))
        }

        fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError> {
            self.inserts.push((sql.to_string(), tsv.to_string()));
            Ok(())
        }

        fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError> {
            self.queries.push(sql.to_string());
            Ok(())
        }
    }

    #[test]
    fn create_kdj_output_table_sql_uses_year_partition_and_expected_order() {
        let sql = create_kdj_output_table_sql();

        assert!(sql.contains("PARTITION BY toYear(trade_date)"));
        assert!(sql.contains("ORDER BY (trade_date, security_code)"));
    }

    #[test]
    fn kdj_staging_table_name_normalizes_run_id() {
        let table_name = kdj_staging_table_name("RUN/2026-01-01");

        assert_eq!(
            table_name,
            "fleur_calculation.calc_stock_kdj_daily__staging__run_2026_01_01"
        );
    }

    #[test]
    fn replace_kdj_partition_sql_replaces_year_partition_from_staging() {
        let sql = replace_kdj_partition_sql("fleur_calculation.stage", 2026);

        assert_eq!(
            sql,
            "ALTER TABLE fleur_calculation.calc_stock_kdj_daily REPLACE PARTITION 2026 FROM fleur_calculation.stage"
        );
    }

    #[test]
    fn run_kdj_dry_run_reads_inputs_and_computes_summary() {
        let responses = [
            "sh.600000\n",
            "2026-01-01\n",
            "1\n",
            "",
            "sh.600000\t2026-01-01\t10\t8\t9\nsh.600000\t2026-01-02\t11\t8\t10\nsh.600000\t2026-01-03\t12\t8\t11\n",
        ];
        let mut executor = FakeExecutor::with_responses(&responses);
        let request = KdjRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-03".to_string(),
            params: KdjParams {
                rsv_window: 3,
                ..KdjParams::default()
            },
            ..KdjRunRequest::default()
        };

        let summary = run_kdj(&mut executor, &request).unwrap();

        assert_eq!(summary.input_rows, 3);
        assert_eq!(summary.output_rows, 3);
        assert_eq!(summary.null_indicator_rows, 2);
        assert!(!summary.writes_applied);
    }

    #[test]
    fn run_kdj_append_latest_inserts_result_rows() {
        let responses = [
            "2026-01-01\n",
            "1\n",
            "",
            "sh.600000\t2026-01-01\t10\t8\t9\nsh.600000\t2026-01-02\t11\t8\t10\nsh.600000\t2026-01-03\t12\t8\t11\n",
            "0\n",
        ];
        let mut executor = FakeExecutor::with_responses(&responses);
        let request = KdjRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-03".to_string(),
            symbols: vec!["sh.600000".to_string()],
            mode: KdjWriteMode::AppendLatest,
            insert_batch_size: MIN_INSERT_BATCH_SIZE,
            ..KdjRunRequest::default()
        };

        let summary = run_kdj(&mut executor, &request).unwrap();

        assert!(summary.writes_applied);
        assert_eq!(executor.inserts.len(), 1);
        assert!(executor.inserts[0].1.contains("sh.600000\t2026-01-03"));
    }

    #[test]
    fn request_validation_rejects_small_production_batches() {
        let request = KdjRunRequest {
            request_from: "2026-01-01".to_string(),
            request_to: "2026-01-03".to_string(),
            mode: KdjWriteMode::AppendLatest,
            insert_batch_size: 10,
            ..KdjRunRequest::default()
        };

        let error = request.validate().unwrap_err();

        assert!(matches!(error, FurnaceIoError::InvalidRequest(_)));
    }
}
