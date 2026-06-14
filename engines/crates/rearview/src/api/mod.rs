use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::domain::RuleVersionSpec;
use crate::domain::metric::{MetricDefinition, ValueKind};
use crate::error::{RearviewError, RearviewResult};
use crate::planner::{CompiledQuery, QueryPlanner, QuerySettings};
use crate::postgres::{
    NewRuleSet, NewRuleVersion, NewRun, Page, PlannedChunk, ResultRowsFilter, ResultRowsSort,
    RuleSetListFilter, RuleVersionListFilter, RunListFilter, plan_date_chunks,
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
}
