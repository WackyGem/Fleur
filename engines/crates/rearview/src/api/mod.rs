use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::clickhouse::{MomentumIndicatorRow, QuoteMartRow, TrendIndicatorRow};
use crate::domain::RuleVersionSpec;
use crate::domain::metric::{MetricDefinition, ValueKind};
use crate::error::{RearviewError, RearviewResult};
use crate::planner::{CompiledQuery, QueryPlanner, QuerySettings};
use crate::postgres::{
    BuySignalRecord, NewRuleSet, NewRuleVersion, NewRun, Page, PlannedChunk, PoolMemberRecord,
    ResultRowsFilter, ResultRowsSort, RuleSetListFilter, RuleVersionListFilter, RunListFilter,
    plan_date_chunks,
};
use crate::service::AppState;
use crate::service::runner::execute_run;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/rearview/metrics", get(list_metrics))
        .route(
            "/rearview/rule-sets",
            get(list_rule_sets).post(create_rule_set),
        )
        .route(
            "/rearview/rule-sets/{rule_set_id}/versions",
            get(list_rule_versions).post(create_rule_version),
        )
        .route("/rearview/runs", get(list_runs).post(create_run))
        .route("/rearview/runs/{run_id}", get(get_run))
        .route("/rearview/runs/{run_id}/chunks", get(list_run_chunks))
        .route("/rearview/runs/{run_id}/days", get(list_run_days))
        .route(
            "/rearview/runs/{run_id}/securities/{security_code}/analysis",
            get(get_security_analysis),
        )
        .route("/rearview/runs/{run_id}/pool", get(list_pool_members))
        .route("/rearview/runs/{run_id}/signals", get(list_buy_signals))
        .route("/rearview/explain", post(explain_rule))
        .layer(CorsLayer::permissive())
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn create_rule_set(
    State(state): State<AppState>,
    Json(request): Json<CreateRuleSetRequest>,
) -> RearviewResult<(StatusCode, Json<impl Serialize>)> {
    let record = state
        .postgres
        .create_rule_set(NewRuleSet {
            name: request.name,
            description: request.description,
            owner: request.owner,
            tags: request.tags,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

async fn list_rule_sets(
    State(state): State<AppState>,
    Query(query): Query<ListRuleSetsQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(
        state
            .postgres
            .list_rule_sets(RuleSetListFilter {
                status: query.status,
                keyword: non_empty(query.keyword),
                page: page(query.limit, query.offset)?,
            })
            .await?,
    ))
}

async fn create_rule_version(
    State(state): State<AppState>,
    Path(rule_set_id): Path<String>,
    Json(request): Json<CreateRuleVersionRequest>,
) -> RearviewResult<(StatusCode, Json<impl Serialize>)> {
    let report = request.rule.validate(&state.catalog)?;
    let record = state
        .postgres
        .create_rule_version(NewRuleVersion {
            rule_set_id,
            rule: request.rule,
            dependencies: report.dependencies,
            rule_hash: report.rule_hash,
            activate: request.activate.unwrap_or(true),
            created_by: request.created_by,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

async fn list_rule_versions(
    State(state): State<AppState>,
    Path(rule_set_id): Path<String>,
    Query(query): Query<ListRuleVersionsQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(
        state
            .postgres
            .list_rule_versions(RuleVersionListFilter {
                rule_set_id,
                status: query.status,
                page: page(query.limit, query.offset)?,
            })
            .await?,
    ))
}

async fn create_run(
    State(state): State<AppState>,
    Json(request): Json<CreateRunRequest>,
) -> RearviewResult<(StatusCode, Json<impl Serialize>)> {
    let rule_version = match (request.rule_version_id, request.rule_set_id) {
        (Some(rule_version_id), None) => state.postgres.get_rule_version(&rule_version_id).await?,
        (None, Some(rule_set_id)) => {
            state
                .postgres
                .resolve_current_rule_version(&rule_set_id)
                .await?
        }
        (Some(_), Some(_)) => {
            return Err(RearviewError::Validation(
                "provide only one of rule_version_id or rule_set_id".to_string(),
            ));
        }
        (None, None) => {
            return Err(RearviewError::Validation(
                "rule_version_id or rule_set_id is required".to_string(),
            ));
        }
    };
    let record = state
        .postgres
        .create_run(
            NewRun {
                rule_version,
                start_date: request.start_date,
                end_date: request.end_date,
                top_n: request.top_n,
                universe_snapshot: request.universe_snapshot,
            },
            state.config.chunk_small_range_trading_days,
        )
        .await?;
    let run_id = record.run_id.clone();
    tokio::spawn(async move {
        let Ok(_permit) = state.run_semaphore.clone().acquire_owned().await else {
            return;
        };
        execute_run(state, run_id).await;
    });
    Ok((StatusCode::ACCEPTED, Json(record)))
}

async fn list_runs(
    State(state): State<AppState>,
    Query(query): Query<ListRunsQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(
        state
            .postgres
            .list_runs(RunListFilter {
                status: query.status,
                rule_set_id: query.rule_set_id,
                start_date: query.start_date,
                end_date: query.end_date,
                keyword: non_empty(query.keyword),
                page: page(query.limit, query.offset)?,
            })
            .await?,
    ))
}

async fn get_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(state.postgres.get_run(&run_id).await?))
}

async fn list_run_days(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(state.postgres.list_run_days(&run_id).await?))
}

async fn list_run_chunks(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(state.postgres.list_run_chunks(&run_id).await?))
}

async fn list_pool_members(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Query(query): Query<ResultRowsQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let sort = query.pool_sort()?;
    Ok(Json(
        state
            .postgres
            .list_pool_members(
                &run_id,
                ResultRowsFilter {
                    trade_date: query.trade_date,
                    security_code: non_empty(query.security_code),
                    sort,
                    page: page(query.limit, query.offset)?,
                },
            )
            .await?,
    ))
}

async fn list_buy_signals(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Query(query): Query<ResultRowsQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let sort = query.signal_sort()?;
    Ok(Json(
        state
            .postgres
            .list_buy_signals(
                &run_id,
                ResultRowsFilter {
                    trade_date: query.trade_date,
                    security_code: non_empty(query.security_code),
                    sort,
                    page: page(query.limit, query.offset)?,
                },
            )
            .await?,
    ))
}

async fn get_security_analysis(
    State(state): State<AppState>,
    Path((run_id, security_code)): Path<(String, String)>,
    Query(query): Query<SecurityAnalysisQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let request = query.into_request()?;
    let result_snapshot = match request.source {
        AnalysisSource::Signals => {
            let signal = state
                .postgres
                .get_buy_signal(&run_id, request.trade_date, &security_code)
                .await?
                .ok_or_else(|| {
                    RearviewError::NotFound(format!(
                        "security {security_code} is not a signal for run {run_id} on {}",
                        request.trade_date
                    ))
                })?;
            ResultSnapshot::from_signal(signal)
        }
        AnalysisSource::Pool => {
            let pool_member = state
                .postgres
                .get_pool_member(&run_id, request.trade_date, &security_code)
                .await?
                .ok_or_else(|| {
                    RearviewError::NotFound(format!(
                        "security {security_code} is not in pool for run {run_id} on {}",
                        request.trade_date
                    ))
                })?;
            ResultSnapshot::from_pool_member(pool_member)
        }
    };

    let query_id_prefix = format!(
        "rearview-analysis-{run_id}-{security_code}-{}",
        request.trade_date
    );
    let quote_rows = state
        .clickhouse
        .query_analysis_quote_rows(
            &security_code,
            request.quote_start_date,
            request.quote_end_date,
            request.lookback_trading_days,
            &format!("{query_id_prefix}-quotes"),
        )
        .await?;

    let (chart_start_date, chart_end_date) = quote_rows
        .first()
        .zip(quote_rows.last())
        .map(|(first, last)| (first.trade_date, last.trade_date))
        .unwrap_or((
            request.quote_start_date.unwrap_or(request.quote_end_date),
            request.quote_end_date,
        ));

    let (trend_rows, momentum_rows) = if quote_rows.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let trend_rows = state
            .clickhouse
            .query_analysis_trend_rows(
                &security_code,
                chart_start_date,
                chart_end_date,
                &format!("{query_id_prefix}-trend"),
            )
            .await?;
        let momentum_rows = state
            .clickhouse
            .query_analysis_momentum_rows(
                &security_code,
                chart_start_date,
                chart_end_date,
                &format!("{query_id_prefix}-momentum"),
            )
            .await?;
        (trend_rows, momentum_rows)
    };

    let response = build_security_analysis_response(
        SecurityAnalysisBuildInput {
            run_id,
            security_code,
            trade_date: request.trade_date,
            source: request.source,
            adjustment: request.adjustment,
            ma_windows: request.ma_windows,
            lookback_trading_days: request.lookback_trading_days,
            chart_start_date,
            chart_end_date,
        },
        result_snapshot,
        quote_rows,
        trend_rows,
        momentum_rows,
        state.config.clickhouse.marts_database.clone(),
    );
    Ok(Json(response))
}

async fn list_metrics(
    State(state): State<AppState>,
    Query(query): Query<ListMetricsQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let keyword = query.keyword.as_deref().map(str::to_lowercase);
    let mut items = state
        .catalog
        .iter()
        .filter(|metric| metric_matches_query(metric, &query, keyword.as_deref()))
        .cloned()
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.logical_metric.cmp(&right.logical_metric));
    Ok(Json(items))
}

async fn explain_rule(
    State(state): State<AppState>,
    Json(request): Json<ExplainRequest>,
) -> RearviewResult<Json<impl Serialize>> {
    let request = request.into_parts()?;
    let planner = QueryPlanner::new(state.catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: state.config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: state.config.clickhouse.max_rows_to_read,
        max_bytes_to_read: state.config.clickhouse.max_bytes_to_read,
    };
    let compiled = match (request.start_date, request.end_date) {
        (Some(start_date), Some(end_date)) => planner.compile(
            &request.rule,
            Some(start_date),
            Some(end_date),
            request.top_n.unwrap_or(request.rule.top_n_default),
            settings,
        )?,
        (None, None) => planner.compile_explain(&request.rule)?,
        _ => {
            return Err(RearviewError::Validation(
                "start_date and end_date must be provided together".to_string(),
            ));
        }
    };
    let chunk_plan = match (request.start_date, request.end_date) {
        (Some(start_date), Some(end_date)) => Some(plan_date_chunks(
            start_date,
            end_date,
            state.config.chunk_small_range_trading_days,
        )?),
        _ => None,
    };
    Ok(Json(ExplainResponse {
        compiled,
        chunk_plan,
    }))
}

#[derive(Debug, Deserialize)]
struct CreateRuleSetRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateRuleVersionRequest {
    rule: RuleVersionSpec,
    #[serde(default)]
    activate: Option<bool>,
    #[serde(default)]
    created_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateRunRequest {
    #[serde(default)]
    rule_set_id: Option<String>,
    #[serde(default)]
    rule_version_id: Option<String>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    #[serde(default)]
    top_n: Option<i32>,
    #[serde(default)]
    universe_snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ListRuleSetsQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    keyword: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListRuleVersionsQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListRunsQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    rule_set_id: Option<String>,
    #[serde(default)]
    start_date: Option<NaiveDate>,
    #[serde(default)]
    end_date: Option<NaiveDate>,
    #[serde(default)]
    keyword: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListMetricsQuery {
    #[serde(default)]
    mart_table: Option<String>,
    #[serde(default)]
    value_kind: Option<ValueKind>,
    #[serde(default)]
    allow_filter: Option<bool>,
    #[serde(default)]
    allow_scoring: Option<bool>,
    #[serde(default)]
    keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResultRowsQuery {
    trade_date: NaiveDate,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(default)]
    security_code: Option<String>,
    #[serde(default)]
    sort: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SecurityAnalysisQuery {
    trade_date: NaiveDate,
    source: AnalysisSource,
    #[serde(default)]
    adjustment: Option<Adjustment>,
    #[serde(default)]
    quote_end_date: Option<NaiveDate>,
    #[serde(default)]
    lookback_trading_days: Option<u32>,
    #[serde(default)]
    quote_start_date: Option<NaiveDate>,
    #[serde(default)]
    ma_windows: Option<String>,
}

impl SecurityAnalysisQuery {
    fn into_request(self) -> RearviewResult<SecurityAnalysisRequest> {
        let adjustment = self.adjustment.unwrap_or(Adjustment::ForwardAdjusted);
        let quote_end_date = self.quote_end_date.unwrap_or(self.trade_date);
        let lookback_trading_days = self.lookback_trading_days.unwrap_or(240);
        if lookback_trading_days == 0 || lookback_trading_days > 1000 {
            return Err(RearviewError::Validation(
                "lookback_trading_days must be between 1 and 1000".to_string(),
            ));
        }
        if let Some(quote_start_date) = self.quote_start_date
            && quote_start_date > quote_end_date
        {
            return Err(RearviewError::Validation(
                "quote_start_date must be <= quote_end_date".to_string(),
            ));
        }

        Ok(SecurityAnalysisRequest {
            trade_date: self.trade_date,
            source: self.source,
            adjustment,
            quote_end_date,
            lookback_trading_days,
            quote_start_date: self.quote_start_date,
            ma_windows: parse_ma_windows(self.ma_windows)?,
        })
    }
}

#[derive(Debug, Clone)]
struct SecurityAnalysisRequest {
    trade_date: NaiveDate,
    source: AnalysisSource,
    adjustment: Adjustment,
    quote_end_date: NaiveDate,
    lookback_trading_days: u32,
    quote_start_date: Option<NaiveDate>,
    ma_windows: Vec<u32>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AnalysisSource {
    Signals,
    Pool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Adjustment {
    ForwardAdjusted,
    BackwardAdjusted,
    Unadjusted,
}

#[derive(Debug, Serialize)]
struct SecurityAnalysisResponse {
    run_id: String,
    trade_date: NaiveDate,
    security_code: String,
    source: AnalysisSource,
    adjustment: Adjustment,
    result_snapshot: ResultSnapshot,
    sources: AnalysisSources,
    chart_window: ChartWindow,
    chart: ChartPayload,
    quote_rows: Vec<QuoteMartRow>,
    selected_quote: Option<QuoteMartRow>,
}

#[derive(Debug, Serialize)]
struct ResultSnapshot {
    rank: Option<i32>,
    signal_rank: Option<i32>,
    score: Option<f64>,
    score_breakdown: Option<serde_json::Value>,
    selected_metrics: serde_json::Value,
    filter_snapshot: Option<serde_json::Value>,
}

impl ResultSnapshot {
    fn from_signal(record: BuySignalRecord) -> Self {
        Self {
            rank: Some(record.rank),
            signal_rank: None,
            score: Some(record.score),
            score_breakdown: Some(record.score_breakdown),
            selected_metrics: record.selected_metrics,
            filter_snapshot: None,
        }
    }

    fn from_pool_member(record: PoolMemberRecord) -> Self {
        Self {
            rank: None,
            signal_rank: record.signal_rank,
            score: record.score,
            score_breakdown: None,
            selected_metrics: record.selected_metrics,
            filter_snapshot: Some(record.filter_snapshot),
        }
    }
}

#[derive(Debug, Serialize)]
struct AnalysisSources {
    quote: SourceMetadata,
    adjusted_quote: AdjustedQuoteSourceMetadata,
    trend: SourceMetadata,
    momentum: SourceMetadata,
}

#[derive(Debug, Serialize)]
struct SourceMetadata {
    database: String,
    table: &'static str,
    value_semantics: &'static str,
    adjustment: Option<Adjustment>,
}

#[derive(Debug, Serialize)]
struct AdjustedQuoteSourceMetadata {
    database: String,
    table: &'static str,
    value_semantics: &'static str,
    adjustment_fields: Vec<Adjustment>,
}

#[derive(Debug, Serialize)]
struct ChartWindow {
    start_date: NaiveDate,
    end_date: NaiveDate,
    lookback_trading_days: u32,
}

#[derive(Debug, Serialize)]
struct ChartPayload {
    ma: ChartMaMetadata,
    price_overlays: ChartPriceOverlayMetadata,
    indicator_panels: [&'static str; 4],
    series: Vec<ChartSeriesRow>,
}

#[derive(Debug, Serialize)]
struct ChartMaMetadata {
    requested_windows: Vec<u32>,
    default_visible_windows: Vec<u32>,
    available_windows: Vec<u32>,
    adjustment: Adjustment,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct ChartPriceOverlayMetadata {
    default_visible_keys: Vec<&'static str>,
    available_keys: Vec<&'static str>,
    adjustment: Adjustment,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct ChartSeriesRow {
    trade_date: NaiveDate,
    ohlc: Option<ChartOhlc>,
    volume: Option<f64>,
    ma: BTreeMap<String, Option<f64>>,
    price_overlays: BTreeMap<&'static str, Option<f64>>,
    kdj: KdjSeries,
    rsi: RsiSeries,
    macd: MacdSeries,
    boll: BollSeries,
}

#[derive(Debug, Serialize)]
struct ChartOhlc {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[derive(Debug, Serialize, Default)]
struct KdjSeries {
    k: Option<f64>,
    d: Option<f64>,
    j: Option<f64>,
    rsv: Option<f64>,
}

#[derive(Debug, Serialize, Default)]
struct RsiSeries {
    #[serde(rename = "6")]
    rsi_6: Option<f64>,
    #[serde(rename = "12")]
    rsi_12: Option<f64>,
    #[serde(rename = "24")]
    rsi_24: Option<f64>,
}

#[derive(Debug, Serialize, Default)]
struct MacdSeries {
    dif: Option<f64>,
    dea: Option<f64>,
    histogram: Option<f64>,
}

#[derive(Debug, Serialize, Default)]
struct BollSeries {
    mid_20_2: Option<f64>,
    up_20_2: Option<f64>,
    dn_20_2: Option<f64>,
}

struct SecurityAnalysisBuildInput {
    run_id: String,
    security_code: String,
    trade_date: NaiveDate,
    source: AnalysisSource,
    adjustment: Adjustment,
    ma_windows: Vec<u32>,
    lookback_trading_days: u32,
    chart_start_date: NaiveDate,
    chart_end_date: NaiveDate,
}

impl ResultRowsQuery {
    fn pool_sort(&self) -> RearviewResult<ResultRowsSort> {
        ResultRowsSort::pool(self.sort.as_deref())
    }

    fn signal_sort(&self) -> RearviewResult<ResultRowsSort> {
        ResultRowsSort::signals(self.sort.as_deref())
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ExplainRequest {
    Rule(RuleVersionSpec),
    WithRange(ExplainRequestWithRange),
}

impl ExplainRequest {
    fn into_parts(self) -> RearviewResult<ExplainRequestParts> {
        match self {
            Self::Rule(rule) => Ok(ExplainRequestParts {
                rule,
                start_date: None,
                end_date: None,
                top_n: None,
            }),
            Self::WithRange(request) => {
                if request.top_n == Some(0) {
                    return Err(RearviewError::Validation(
                        "top_n must be greater than 0".to_string(),
                    ));
                }
                Ok(ExplainRequestParts {
                    rule: request.rule,
                    start_date: request.start_date,
                    end_date: request.end_date,
                    top_n: request.top_n,
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ExplainRequestWithRange {
    rule: RuleVersionSpec,
    #[serde(default)]
    start_date: Option<NaiveDate>,
    #[serde(default)]
    end_date: Option<NaiveDate>,
    #[serde(default)]
    top_n: Option<u32>,
}

struct ExplainRequestParts {
    rule: RuleVersionSpec,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    top_n: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ExplainResponse {
    #[serde(flatten)]
    compiled: CompiledQuery,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_plan: Option<Vec<PlannedChunk>>,
}

fn page(limit: Option<u32>, offset: Option<u32>) -> RearviewResult<Page> {
    const DEFAULT_LIMIT: u32 = 50;
    const MAX_LIMIT: u32 = 500;
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 || limit > MAX_LIMIT {
        return Err(RearviewError::Validation(format!(
            "limit must be between 1 and {MAX_LIMIT}"
        )));
    }
    Ok(Page {
        limit: i64::from(limit),
        offset: i64::from(offset.unwrap_or(0)),
    })
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn build_security_analysis_response(
    input: SecurityAnalysisBuildInput,
    result_snapshot: ResultSnapshot,
    quote_rows: Vec<QuoteMartRow>,
    trend_rows: Vec<TrendIndicatorRow>,
    momentum_rows: Vec<MomentumIndicatorRow>,
    marts_database: String,
) -> SecurityAnalysisResponse {
    let trend_by_date = trend_rows
        .into_iter()
        .map(|row| (row.trade_date, row))
        .collect::<BTreeMap<_, _>>();
    let momentum_by_date = momentum_rows
        .into_iter()
        .map(|row| (row.trade_date, row))
        .collect::<BTreeMap<_, _>>();
    let selected_quote = quote_rows
        .iter()
        .find(|row| row.trade_date == input.trade_date)
        .cloned();
    let series = quote_rows
        .iter()
        .map(|quote| {
            let trend = trend_by_date.get(&quote.trade_date);
            let momentum = momentum_by_date.get(&quote.trade_date);
            ChartSeriesRow {
                trade_date: quote.trade_date,
                ohlc: ohlc_for_adjustment(quote, input.adjustment),
                volume: quote.volume,
                ma: ma_values(trend, &input.ma_windows, input.adjustment),
                price_overlays: price_overlay_values(trend, input.adjustment),
                kdj: kdj_values(momentum, quote),
                rsi: rsi_values(momentum),
                macd: macd_values(trend),
                boll: boll_values(trend),
            }
        })
        .collect::<Vec<_>>();
    let ma_supported = input.adjustment == Adjustment::ForwardAdjusted;
    let requested_windows = input.ma_windows;
    let available_windows = if ma_supported {
        requested_windows.clone()
    } else {
        Vec::new()
    };

    SecurityAnalysisResponse {
        run_id: input.run_id,
        trade_date: input.trade_date,
        security_code: input.security_code,
        source: input.source,
        adjustment: input.adjustment,
        result_snapshot,
        sources: AnalysisSources {
            quote: SourceMetadata {
                database: marts_database.clone(),
                table: "mart_stock_quotes_daily",
                value_semantics: "current_mart_query",
                adjustment: Some(Adjustment::Unadjusted),
            },
            adjusted_quote: AdjustedQuoteSourceMetadata {
                database: marts_database.clone(),
                table: "mart_stock_quotes_daily",
                value_semantics: "current_mart_query",
                adjustment_fields: vec![Adjustment::ForwardAdjusted, Adjustment::BackwardAdjusted],
            },
            trend: SourceMetadata {
                database: marts_database.clone(),
                table: "mart_stock_trend_indicator",
                value_semantics: "current_mart_query",
                adjustment: Some(Adjustment::ForwardAdjusted),
            },
            momentum: SourceMetadata {
                database: marts_database,
                table: "mart_stock_momentum_indicator",
                value_semantics: "current_mart_query",
                adjustment: Some(Adjustment::ForwardAdjusted),
            },
        },
        chart_window: ChartWindow {
            start_date: input.chart_start_date,
            end_date: input.chart_end_date,
            lookback_trading_days: input.lookback_trading_days,
        },
        chart: ChartPayload {
            ma: ChartMaMetadata {
                requested_windows,
                default_visible_windows: available_windows.clone(),
                available_windows,
                adjustment: Adjustment::ForwardAdjusted,
                status: if ma_supported {
                    "available"
                } else {
                    "forward_adjusted_only"
                },
            },
            price_overlays: ChartPriceOverlayMetadata {
                default_visible_keys: vec!["price_ma_5", "price_ma_10", "price_ma_30"],
                available_keys: if ma_supported {
                    PRICE_OVERLAY_KEYS.to_vec()
                } else {
                    Vec::new()
                },
                adjustment: Adjustment::ForwardAdjusted,
                status: if ma_supported {
                    "available"
                } else {
                    "forward_adjusted_only"
                },
            },
            indicator_panels: ["kdj", "rsi", "macd", "boll"],
            series,
        },
        quote_rows,
        selected_quote,
    }
}

fn ohlc_for_adjustment(row: &QuoteMartRow, adjustment: Adjustment) -> Option<ChartOhlc> {
    let (open, high, low, close) = match adjustment {
        Adjustment::ForwardAdjusted => (
            row.open_price_forward_adj,
            row.high_price_forward_adj,
            row.low_price_forward_adj,
            row.close_price_forward_adj,
        ),
        Adjustment::BackwardAdjusted => (
            row.open_price_backward_adj,
            row.high_price_backward_adj,
            row.low_price_backward_adj,
            row.close_price_backward_adj,
        ),
        Adjustment::Unadjusted => (
            row.open_price,
            row.high_price,
            row.low_price,
            row.close_price,
        ),
    };
    Some(ChartOhlc {
        open: open?,
        high: high?,
        low: low?,
        close: close?,
    })
}

const PRICE_OVERLAY_KEYS: [&str; 6] = [
    "price_ma_5",
    "price_ma_10",
    "price_ma_30",
    "price_ema2_10",
    "price_avg_ma_3_6_12_24",
    "price_avg_ma_14_28_57_114",
];

fn ma_values(
    trend: Option<&TrendIndicatorRow>,
    ma_windows: &[u32],
    adjustment: Adjustment,
) -> BTreeMap<String, Option<f64>> {
    if adjustment != Adjustment::ForwardAdjusted {
        return BTreeMap::new();
    }
    let mut values = BTreeMap::new();
    for window in ma_windows {
        let value = trend.and_then(|trend| match window {
            5 => trend.price_ma_5,
            10 => trend.price_ma_10,
            30 => trend.price_ma_30,
            _ => None,
        });
        values.insert(window.to_string(), value);
    }
    values
}

fn price_overlay_values(
    trend: Option<&TrendIndicatorRow>,
    adjustment: Adjustment,
) -> BTreeMap<&'static str, Option<f64>> {
    if adjustment != Adjustment::ForwardAdjusted {
        return BTreeMap::new();
    }
    let mut values = BTreeMap::new();
    for key in PRICE_OVERLAY_KEYS {
        let value = trend.and_then(|trend| match key {
            "price_ma_5" => trend.price_ma_5,
            "price_ma_10" => trend.price_ma_10,
            "price_ma_30" => trend.price_ma_30,
            "price_ema2_10" => trend.price_ema2_10,
            "price_avg_ma_3_6_12_24" => trend.price_avg_ma_3_6_12_24,
            "price_avg_ma_14_28_57_114" => trend.price_avg_ma_14_28_57_114,
            _ => None,
        });
        values.insert(key, value);
    }
    values
}

fn kdj_values(momentum: Option<&MomentumIndicatorRow>, quote: &QuoteMartRow) -> KdjSeries {
    KdjSeries {
        k: momentum
            .and_then(|momentum| momentum.kdj_k_value)
            .or(quote.kdj_k_value),
        d: momentum
            .and_then(|momentum| momentum.kdj_d_value)
            .or(quote.kdj_d_value),
        j: momentum
            .and_then(|momentum| momentum.kdj_j_value)
            .or(quote.kdj_j_value),
        rsv: momentum
            .and_then(|momentum| momentum.kdj_rsv)
            .or(quote.kdj_rsv),
    }
}

fn rsi_values(momentum: Option<&MomentumIndicatorRow>) -> RsiSeries {
    RsiSeries {
        rsi_6: momentum.and_then(|momentum| momentum.rsi_6),
        rsi_12: momentum.and_then(|momentum| momentum.rsi_12),
        rsi_24: momentum.and_then(|momentum| momentum.rsi_24),
    }
}

fn macd_values(trend: Option<&TrendIndicatorRow>) -> MacdSeries {
    MacdSeries {
        dif: trend.and_then(|trend| trend.macd_dif),
        dea: trend.and_then(|trend| trend.macd_dea),
        histogram: trend.and_then(|trend| trend.macd_histogram),
    }
}

fn boll_values(trend: Option<&TrendIndicatorRow>) -> BollSeries {
    BollSeries {
        mid_20_2: trend.and_then(|trend| trend.boll_mid_20_2),
        up_20_2: trend.and_then(|trend| trend.boll_up_20_2),
        dn_20_2: trend.and_then(|trend| trend.boll_dn_20_2),
    }
}

fn parse_ma_windows(value: Option<String>) -> RearviewResult<Vec<u32>> {
    let Some(value) = value else {
        return Ok(vec![5, 10, 30]);
    };
    let requested = value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            item.parse::<u32>().map_err(|error| {
                RearviewError::Validation(format!("invalid ma_windows value {item:?}: {error}"))
            })
        })
        .collect::<RearviewResult<Vec<_>>>()?;
    if requested.is_empty() {
        return Err(RearviewError::Validation(
            "ma_windows must include at least one window".to_string(),
        ));
    }

    let mut canonical = Vec::new();
    for allowed in [5, 10, 30] {
        if requested.contains(&allowed) {
            canonical.push(allowed);
        }
    }
    if canonical.len()
        != requested
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    {
        return Err(RearviewError::Validation(
            "ma_windows only supports 5,10,30".to_string(),
        ));
    }
    Ok(canonical)
}

fn metric_matches_query(
    metric: &MetricDefinition,
    query: &ListMetricsQuery,
    keyword: Option<&str>,
) -> bool {
    if query
        .mart_table
        .as_ref()
        .is_some_and(|mart_table| metric.mart_table != *mart_table)
    {
        return false;
    }
    if query
        .value_kind
        .is_some_and(|value_kind| metric.value_kind != value_kind)
    {
        return false;
    }
    if query
        .allow_filter
        .is_some_and(|allow_filter| metric.allow_filter != allow_filter)
    {
        return false;
    }
    if query
        .allow_scoring
        .is_some_and(|allow_scoring| metric.allow_scoring != allow_scoring)
    {
        return false;
    }
    if let Some(keyword) = keyword {
        let description = metric.description.as_deref().unwrap_or_default();
        return metric.logical_metric.to_lowercase().contains(keyword)
            || metric.column_name.to_lowercase().contains(keyword)
            || description.to_lowercase().contains(keyword);
    }
    true
}

impl ResultRowsSort {
    fn pool(value: Option<&str>) -> RearviewResult<Self> {
        match value.unwrap_or("signal_rank_asc") {
            "signal_rank_asc" => Ok(Self::PoolSignalRankAsc),
            "score_desc" => Ok(Self::PoolScoreDesc),
            "score_asc" => Ok(Self::PoolScoreAsc),
            "security_code_asc" => Ok(Self::SecurityCodeAsc),
            other => Err(RearviewError::Validation(format!(
                "unsupported pool sort: {other}"
            ))),
        }
    }

    fn signals(value: Option<&str>) -> RearviewResult<Self> {
        match value.unwrap_or("rank_asc") {
            "rank_asc" => Ok(Self::SignalRankAsc),
            "score_desc" => Ok(Self::SignalScoreDesc),
            "security_code_asc" => Ok(Self::SecurityCodeAsc),
            other => Err(RearviewError::Validation(format!(
                "unsupported signals sort: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_should_default_limit_and_offset() {
        let page = page(None, None).unwrap();

        assert_eq!(page.limit, 50);
        assert_eq!(page.offset, 0);
    }

    #[test]
    fn page_should_reject_zero_limit() {
        let error = page(Some(0), None).unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn result_rows_sort_should_reject_unknown_signal_sort() {
        let error = ResultRowsSort::signals(Some("rank_desc")).unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn parse_ma_windows_should_accept_canonical_subset() {
        let windows = parse_ma_windows(Some("10,5,5".to_string())).unwrap();

        assert_eq!(windows, vec![5, 10]);
    }

    #[test]
    fn parse_ma_windows_should_reject_unsupported_window() {
        let error = parse_ma_windows(Some("5,28,30".to_string())).unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn security_analysis_query_should_reject_large_lookback() {
        let query = SecurityAnalysisQuery {
            trade_date: NaiveDate::from_ymd_opt(2026, 6, 12).unwrap(),
            source: AnalysisSource::Signals,
            adjustment: None,
            quote_end_date: None,
            lookback_trading_days: Some(1001),
            quote_start_date: None,
            ma_windows: None,
        };

        let error = query.into_request().unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }
}
