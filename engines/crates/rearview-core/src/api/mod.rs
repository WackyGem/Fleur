use std::collections::{BTreeMap, BTreeSet};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::{Months, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tower_http::cors::CorsLayer;

use crate::clickhouse::{
    AnalysisQuoteAdjustment, MomentumIndicatorRow, QuoteMartRow, SecurityDisplayRow,
    TrendIndicatorRow,
};
use crate::domain::RuleVersionSpec;
use crate::domain::metric::{MetricCatalog, MetricDefinition, ValueKind};
use crate::error::{RearviewError, RearviewResult};
use crate::planner::{CompiledQuery, QueryPlanner, QuerySettings};
use crate::portfolio_performance::BenchmarkReturn;
use crate::postgres::{
    BuySignalRecord, NewAccountTemplate, NewPortfolioRun, NewRuleSet, NewRuleVersion, NewRun,
    NewStrategyBacktestRun, Page, PatchAccountTemplate, PlannedChunk, PoolMemberRecord,
    PortfolioClosedTradeFilter, PortfolioClosedTradeRecord, PortfolioEventFilter,
    PortfolioNavRecord, PortfolioOrderFilter, PortfolioPerformanceMetricRecord,
    PortfolioPerformanceMetricStatusRecord, PortfolioPositionFilter, PortfolioPositionRecord,
    PortfolioRunListFilter, PortfolioTargetFilter, PortfolioTradeFilter,
    PortfolioTradeMetricFilter, PortfolioTradeRecord, ResultRowsFilter, ResultRowsSort,
    RuleSetListFilter, RuleVersionListFilter, RunListFilter, StrategyBacktestRunRecord,
    plan_date_chunks,
};
use crate::service::AppState;
use crate::service::runner::execute_run;
use crate::strategy_backtest::{
    BacktestDateRange, BacktestExecutionConfig, BacktestExecutionSummary,
    StrategyBacktestDraftResponse, StrategyBacktestValidateRequest, hash_json,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/rearview/metrics", get(list_metrics))
        .route(
            "/rearview/rule-sets",
            get(list_rule_sets).post(create_rule_set),
        )
        .route(
            "/rearview/market-fee-templates/default",
            get(get_default_market_fee_template),
        )
        .route(
            "/rearview/rule-sets/{rule_set_id}/account-templates",
            get(list_account_templates).post(create_account_template),
        )
        .route(
            "/rearview/account-templates/{account_template_id}",
            patch(update_account_template),
        )
        .route(
            "/rearview/rule-sets/{rule_set_id}/versions",
            get(list_rule_versions).post(create_rule_version),
        )
        .route(
            "/rearview/portfolio-runs",
            get(list_portfolio_runs).post(create_portfolio_run),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}",
            get(get_portfolio_run),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/nav",
            get(list_portfolio_nav),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/targets",
            get(list_portfolio_targets),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/orders",
            get(list_portfolio_orders),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/trades",
            get(list_portfolio_trades),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/positions",
            get(list_portfolio_positions),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/events",
            get(list_portfolio_events),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/performance",
            get(get_portfolio_performance),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/closed-trades",
            get(list_portfolio_closed_trades),
        )
        .route(
            "/rearview/portfolio-runs/{portfolio_run_id}/trade-metrics",
            get(list_portfolio_trade_metrics),
        )
        .route("/rearview/runs", get(list_runs).post(create_run))
        .route("/rearview/runs/{run_id}", get(get_run))
        .route("/rearview/runs/{run_id}/chunks", get(list_run_chunks))
        .route("/rearview/runs/{run_id}/days", get(list_run_days))
        .route(
            "/rearview/runs/{run_id}/securities/{security_code}/analysis",
            get(get_security_analysis),
        )
        .route("/rearview/security-analysis", post(analyze_security))
        .route("/rearview/runs/{run_id}/pool", get(list_pool_members))
        .route("/rearview/runs/{run_id}/signals", get(list_buy_signals))
        .route("/rearview/explain", post(explain_rule))
        .route(
            "/rearview/strategy-backtests/validate",
            post(validate_strategy_backtest),
        )
        .route(
            "/rearview/strategy-backtests/options",
            get(get_strategy_backtest_options),
        )
        .route(
            "/rearview/strategy-backtests",
            post(create_strategy_backtest),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}",
            get(get_strategy_backtest),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/nav",
            get(list_strategy_backtest_nav),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/rebalance-records",
            get(list_strategy_backtest_rebalance_records),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/targets",
            get(list_strategy_backtest_targets),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/orders",
            get(list_strategy_backtest_orders),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/trades",
            get(list_strategy_backtest_trades),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/positions",
            get(list_strategy_backtest_positions),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/events",
            get(list_strategy_backtest_events),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/performance",
            get(get_strategy_backtest_performance),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/closed-trades",
            get(list_strategy_backtest_closed_trades),
        )
        .route(
            "/rearview/strategy-backtests/{strategy_backtest_run_id}/trade-metrics",
            get(list_strategy_backtest_trade_metrics),
        )
        .route("/rearview/strategy-preview", post(preview_strategy))
        .route(
            "/rearview/strategy-preview/timeline",
            post(preview_strategy_timeline),
        )
        .route(
            "/rearview/strategy-preview/pool-page",
            post(preview_strategy_pool_page),
        )
        .route(
            "/rearview/strategy-preview/security-analysis",
            post(preview_strategy_security_analysis),
        )
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

async fn get_default_market_fee_template(
    State(state): State<AppState>,
    Query(query): Query<DefaultMarketFeeTemplateQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let market = query.market.unwrap_or_else(default_market);
    Ok(Json(
        state
            .postgres
            .get_default_market_fee_template(&market)
            .await?,
    ))
}

async fn list_account_templates(
    State(state): State<AppState>,
    Path(rule_set_id): Path<String>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(
        state.postgres.list_account_templates(&rule_set_id).await?,
    ))
}

async fn create_account_template(
    State(state): State<AppState>,
    Path(rule_set_id): Path<String>,
    Json(request): Json<CreateAccountTemplateRequest>,
) -> RearviewResult<(StatusCode, Json<impl Serialize>)> {
    let market = request.market.unwrap_or_else(default_market);
    let market_template = state
        .postgres
        .get_default_market_fee_template(&market)
        .await?;
    let record = state
        .postgres
        .create_account_template(NewAccountTemplate {
            rule_set_id,
            market_fee_template_id: request
                .market_fee_template_id
                .or(Some(market_template.market_fee_template_id)),
            name: request
                .name
                .unwrap_or_else(|| "Default research account".to_string()),
            initial_cash: request.initial_cash.unwrap_or(1_000_000.0),
            currency: request.currency.unwrap_or(market_template.currency),
            fee_profile: request.fee_profile.unwrap_or(market_template.fee_profile),
            slippage_profile: request
                .slippage_profile
                .unwrap_or(market_template.slippage_profile),
            rebalance_policy: request
                .rebalance_policy
                .unwrap_or_else(default_rebalance_policy),
            risk_exit_policy: request
                .risk_exit_policy
                .unwrap_or_else(default_risk_exit_policy),
            is_default: request.is_default.unwrap_or(false),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

async fn update_account_template(
    State(state): State<AppState>,
    Path(account_template_id): Path<String>,
    Json(request): Json<PatchAccountTemplateRequest>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(
        state
            .postgres
            .update_account_template(PatchAccountTemplate {
                account_template_id,
                name: request.name,
                initial_cash: request.initial_cash,
                currency: request.currency,
                fee_profile: request.fee_profile,
                slippage_profile: request.slippage_profile,
                rebalance_policy: request.rebalance_policy,
                risk_exit_policy: request.risk_exit_policy,
                is_default: request.is_default,
                status: request.status,
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

async fn create_portfolio_run(
    State(state): State<AppState>,
    Json(request): Json<CreatePortfolioRunRequest>,
) -> RearviewResult<(StatusCode, Json<impl Serialize>)> {
    let record = state
        .postgres
        .create_portfolio_run(NewPortfolioRun {
            source_run_id: request.source_run_id,
            account_template_id: request.account_template_id,
            subject: state.config.nats.portfolio_request_subject.clone(),
        })
        .await?;
    Ok((StatusCode::ACCEPTED, Json(record)))
}

async fn validate_strategy_backtest(
    State(state): State<AppState>,
    Json(request): Json<StrategyBacktestValidateRequest>,
) -> RearviewResult<Json<StrategyBacktestDraftResponse>> {
    Ok(Json(request.validate(&state.catalog)?))
}

async fn get_strategy_backtest_options(
    State(state): State<AppState>,
    Query(query): Query<StrategyBacktestOptionsQuery>,
) -> RearviewResult<Json<StrategyBacktestOptionsResponse>> {
    let benchmark_security_code =
        non_empty(query.benchmark_security_code).unwrap_or_else(default_benchmark);
    validate_strategy_backtest_benchmark(&benchmark_security_code)?;
    let as_of_date = query.as_of_date.unwrap_or_else(|| Utc::now().date_naive());
    let resolution =
        resolve_strategy_backtest_range(&state, &benchmark_security_code, as_of_date).await?;
    let benchmark_options = strategy_backtest_benchmark_options(&benchmark_security_code, true);

    Ok(Json(StrategyBacktestOptionsResponse {
        default_period_key: "1y".to_string(),
        default_benchmark_security_code: default_benchmark(),
        selected_benchmark_security_code: benchmark_security_code,
        as_of_date,
        latest_available_trade_date: resolution.latest_available_trade_date,
        period_options: resolution.period_options,
        benchmark_options,
        range_resolution_snapshot: resolution.range_resolution_snapshot,
    }))
}

async fn create_strategy_backtest(
    State(state): State<AppState>,
    Json(request): Json<StrategyBacktestCreateRequest>,
) -> RearviewResult<(StatusCode, Json<StrategyBacktestRunResponse>)> {
    validate_strategy_backtest_period_key(&request.period_key)?;
    validate_strategy_backtest_benchmark(&request.benchmark_security_code)?;
    let range_as_of_date = Utc::now().date_naive();
    let resolution =
        resolve_strategy_backtest_range(&state, &request.benchmark_security_code, range_as_of_date)
            .await?;
    let period_option = resolution
        .period_options
        .iter()
        .find(|option| option.period_key == request.period_key)
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "period_key is not available for resolved range: {}",
                request.period_key
            ))
        })?;

    let validate_request = StrategyBacktestValidateRequest {
        rule: request.rule.clone(),
        preview_id: request.preview_id.clone(),
        preview_range: request.preview_range.clone(),
        execution_config: request.execution_config.clone(),
        range: Some(BacktestDateRange {
            start_date: period_option.resolved_start_date,
            end_date: period_option.resolved_end_date,
        }),
        benchmark: Some(request.benchmark_security_code.clone()),
    };
    let draft = validate_request.validate(&state.catalog)?;
    if let Some(rule_hash) = &request.rule_hash
        && rule_hash != &draft.rule_hash
    {
        return Err(RearviewError::Validation(
            "rule_hash does not match server canonical hash".to_string(),
        ));
    }
    if let Some(execution_config_hash) = &request.execution_config_hash
        && execution_config_hash != &draft.execution_config_hash
    {
        return Err(RearviewError::Validation(
            "execution_config_hash does not match server canonical hash".to_string(),
        ));
    }
    if let Some(top_n) = request.top_n
        && top_n != draft.execution_config.signal_policy.buy_signal_top_n
    {
        return Err(RearviewError::Validation(
            "top_n must match execution_config.signal_policy.buy_signal_top_n".to_string(),
        ));
    }

    let rule_snapshot = serde_json::to_value(&request.rule)?;
    let execution_config = serde_json::to_value(&draft.execution_config)?;
    let preview_range = request
        .preview_range
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let catalog_hash = Some(hash_catalog(&state.catalog)?);
    let risk_free_tenor = "1y";
    let (risk_free_return_count, risk_free_preflight_error) = match state
        .clickhouse
        .query_mart_risk_free_rates(
            risk_free_tenor,
            period_option.resolved_start_date,
            period_option.resolved_end_date,
            &format!(
                "strategy-backtest-preflight-risk-free-{}-{}",
                request.benchmark_security_code, request.period_key
            ),
        )
        .await
    {
        Ok(rows) => (Some(rows.len()), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let data_preflight_snapshot = build_strategy_backtest_preflight_snapshot(
        &request,
        period_option,
        &resolution,
        &draft,
        &catalog_hash,
        StrategyBacktestRiskFreePreflight {
            tenor: risk_free_tenor,
            return_count: risk_free_return_count,
            error: risk_free_preflight_error,
        },
    );
    let ui_display_snapshot = request.ui_display_snapshot.unwrap_or_else(|| json!({}));
    let request_hash = hash_json(&json!({
        "rule_hash": &draft.rule_hash,
        "execution_config_hash": &draft.execution_config_hash,
        "period_key": &request.period_key,
        "start_date": period_option.resolved_start_date,
        "end_date": period_option.resolved_end_date,
        "benchmark_security_code": &request.benchmark_security_code,
    }))?;
    let client_request_id = non_empty(request.client_request_id);
    if let Some(client_request_id) = &client_request_id
        && let Some(existing) = state
            .postgres
            .get_strategy_backtest_run_by_client_request_id(client_request_id)
            .await?
    {
        if existing.request_hash == request_hash {
            return Ok((
                StatusCode::ACCEPTED,
                Json(strategy_backtest_run_response(existing)?),
            ));
        }
        return Err(RearviewError::Conflict(
            "client_request_id already exists for a different strategy backtest request"
                .to_string(),
        ));
    }

    let record = state
        .postgres
        .create_strategy_backtest_run(NewStrategyBacktestRun {
            rule_snapshot,
            rule_hash: draft.rule_hash,
            execution_config,
            execution_config_hash: draft.execution_config_hash,
            catalog_hash,
            data_preflight_snapshot,
            preview_id: request.preview_id,
            preview_range,
            period_key: request.period_key,
            range_as_of_date: Some(range_as_of_date),
            range_resolution_snapshot: resolution.range_resolution_snapshot.clone(),
            start_date: period_option.resolved_start_date,
            end_date: period_option.resolved_end_date,
            benchmark_security_code: request.benchmark_security_code,
            ui_display_snapshot,
            client_request_id,
            request_hash,
            subject: state.config.nats.portfolio_request_subject.clone(),
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(strategy_backtest_run_response(record)?),
    ))
}

async fn get_strategy_backtest(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
) -> RearviewResult<Json<StrategyBacktestRunResponse>> {
    let record = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    Ok(Json(strategy_backtest_run_response(record)?))
}

async fn list_strategy_backtest_nav(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioNavQuery>,
) -> RearviewResult<Json<Vec<StrategyBacktestNavPoint>>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    let nav = state
        .clickhouse
        .query_portfolio_nav(&strategy_backtest_run_id, &attempt_id)
        .await?;
    let benchmark_returns = state
        .clickhouse
        .query_mart_benchmark_returns(
            &run.benchmark_security_code,
            run.start_date,
            run.end_date,
            &format!("strategy-backtest-{strategy_backtest_run_id}-nav-benchmark"),
        )
        .await?;
    Ok(Json(strategy_backtest_nav_points(nav, benchmark_returns)))
}

async fn list_strategy_backtest_rebalance_records(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<StrategyBacktestRebalanceQuery>,
) -> RearviewResult<Json<StrategyBacktestRebalanceRecordsResponse>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    let nav = state
        .clickhouse
        .query_portfolio_nav(&strategy_backtest_run_id, &attempt_id)
        .await?;
    let selected_trade_date = query
        .trade_date
        .or_else(|| {
            nav.iter()
                .rev()
                .find(|row| row.position_count > 0 || row.turnover > 0.0)
                .map(|row| row.trade_date)
        })
        .or_else(|| nav.last().map(|row| row.trade_date))
        .ok_or_else(|| {
            RearviewError::NotFound(format!(
                "no nav rows for strategy backtest: {strategy_backtest_run_id}"
            ))
        })?;
    let selected_nav = nav.iter().find(|row| row.trade_date == selected_trade_date);
    let page = Page {
        limit: 500,
        offset: 0,
    };
    let trades = state
        .clickhouse
        .query_portfolio_trades(
            &PortfolioTradeFilter {
                portfolio_run_id: strategy_backtest_run_id.clone(),
                trade_date: Some(selected_trade_date),
                security_code: None,
                page,
            },
            &attempt_id,
        )
        .await?;
    let positions = state
        .clickhouse
        .query_portfolio_positions(
            &PortfolioPositionFilter {
                portfolio_run_id: strategy_backtest_run_id.clone(),
                trade_date: Some(selected_trade_date),
                security_code: None,
                page,
            },
            &attempt_id,
        )
        .await?;
    let closed_trades = state
        .clickhouse
        .query_portfolio_closed_trades(
            &PortfolioClosedTradeFilter {
                portfolio_run_id: strategy_backtest_run_id.clone(),
                security_code: None,
                exit_date: Some(selected_trade_date),
                page,
            },
            &attempt_id,
        )
        .await?;
    let security_codes = trades
        .items
        .iter()
        .map(|trade| trade.security_code.clone())
        .chain(
            positions
                .items
                .iter()
                .map(|position| position.security_code.clone()),
        )
        .chain(
            closed_trades
                .items
                .iter()
                .map(|closed| closed.security_code.clone()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let display = security_display_map(
        &state,
        &security_codes,
        &format!("strategy-backtest-{strategy_backtest_run_id}-rebalance-display"),
    )
    .await;
    let total_equity = selected_nav.map(|row| row.total_equity);
    let rows = build_strategy_backtest_rebalance_rows(
        trades.items,
        positions.items,
        closed_trades.items,
        &display,
        total_equity,
    );
    let buy_count = rows.iter().filter(|row| row.direction == "buy").count();
    let hold_count = rows.iter().filter(|row| row.direction == "hold").count();
    let sell_count = rows.iter().filter(|row| row.direction == "sell").count();
    let records = nav
        .into_iter()
        .map(|row| StrategyBacktestRebalanceRecord {
            trade_date: row.trade_date,
            position_count: row.position_count,
            buy_count: if row.trade_date == selected_trade_date {
                buy_count
            } else {
                0
            },
            hold_count: if row.trade_date == selected_trade_date {
                hold_count
            } else {
                0
            },
            sell_count: if row.trade_date == selected_trade_date {
                sell_count
            } else {
                0
            },
            rows: if row.trade_date == selected_trade_date {
                rows.clone()
            } else {
                Vec::new()
            },
        })
        .collect();
    Ok(Json(StrategyBacktestRebalanceRecordsResponse {
        selected_trade_date,
        records,
    }))
}

async fn list_strategy_backtest_targets(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioTargetQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_targets(
                &PortfolioTargetFilter {
                    portfolio_run_id: strategy_backtest_run_id,
                    signal_date: query.signal_date,
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_strategy_backtest_orders(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioOrderQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_orders(
                &PortfolioOrderFilter {
                    portfolio_run_id: strategy_backtest_run_id,
                    execution_date: query.execution_date,
                    security_code: non_empty(query.security_code),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_strategy_backtest_trades(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioTradeQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_trades(
                &PortfolioTradeFilter {
                    portfolio_run_id: strategy_backtest_run_id,
                    trade_date: query.trade_date,
                    security_code: non_empty(query.security_code),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_strategy_backtest_positions(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioPositionQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_positions(
                &PortfolioPositionFilter {
                    portfolio_run_id: strategy_backtest_run_id,
                    trade_date: query.trade_date,
                    security_code: non_empty(query.security_code),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_strategy_backtest_events(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioEventQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_events(
                &PortfolioEventFilter {
                    portfolio_run_id: strategy_backtest_run_id,
                    trade_date: query.trade_date,
                    event_type: non_empty(query.event_type),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn get_strategy_backtest_performance(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioPerformanceQuery>,
) -> RearviewResult<Json<StrategyBacktestPerformanceView>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    let security_code = non_empty(query.security_code).unwrap_or(run.benchmark_security_code);
    let window_key = non_empty(query.window_key).unwrap_or_else(default_metric_window);
    let performance = state
        .clickhouse
        .query_portfolio_performance(
            &strategy_backtest_run_id,
            &attempt_id,
            &security_code,
            &window_key,
        )
        .await?;
    let nav = state
        .clickhouse
        .query_portfolio_nav(&strategy_backtest_run_id, &attempt_id)
        .await?;
    Ok(Json(StrategyBacktestPerformanceView {
        metric: performance.metric,
        statuses: performance.statuses,
        daily_win_rate: daily_win_rate(&nav),
    }))
}

async fn list_strategy_backtest_closed_trades(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioClosedTradeQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_closed_trades(
                &PortfolioClosedTradeFilter {
                    portfolio_run_id: strategy_backtest_run_id,
                    security_code: non_empty(query.security_code),
                    exit_date: query.exit_date,
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_strategy_backtest_trade_metrics(
    State(state): State<AppState>,
    Path(strategy_backtest_run_id): Path<String>,
    Query(query): Query<PortfolioTradeMetricQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let run = state
        .postgres
        .get_strategy_backtest_run(&strategy_backtest_run_id)
        .await?;
    let attempt_id =
        resolve_strategy_backtest_result_attempt(&run, query.result_attempt_id.as_deref())?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_trade_metrics(
                &PortfolioTradeMetricFilter {
                    portfolio_run_id: strategy_backtest_run_id,
                    window_key: non_empty(query.window_key),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_portfolio_runs(
    State(state): State<AppState>,
    Query(query): Query<ListPortfolioRunsQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(
        state
            .postgres
            .list_portfolio_runs(PortfolioRunListFilter {
                source_run_id: query.source_run_id,
                status: query.status,
                dispatch_status: query.dispatch_status,
                page: page(query.limit, query.offset)?,
            })
            .await?,
    ))
}

async fn get_portfolio_run(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
) -> RearviewResult<Json<impl Serialize>> {
    Ok(Json(
        state.postgres.get_portfolio_run(&portfolio_run_id).await?,
    ))
}

async fn list_portfolio_nav(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioNavQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_nav(&portfolio_run_id, &attempt_id)
            .await?,
    ))
}

async fn list_portfolio_targets(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioTargetQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_targets(
                &PortfolioTargetFilter {
                    portfolio_run_id,
                    signal_date: query.signal_date,
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_portfolio_orders(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioOrderQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_orders(
                &PortfolioOrderFilter {
                    portfolio_run_id,
                    execution_date: query.execution_date,
                    security_code: non_empty(query.security_code),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_portfolio_trades(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioTradeQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_trades(
                &PortfolioTradeFilter {
                    portfolio_run_id,
                    trade_date: query.trade_date,
                    security_code: non_empty(query.security_code),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_portfolio_positions(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioPositionQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_positions(
                &PortfolioPositionFilter {
                    portfolio_run_id,
                    trade_date: query.trade_date,
                    security_code: non_empty(query.security_code),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_portfolio_events(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioEventQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_events(
                &PortfolioEventFilter {
                    portfolio_run_id,
                    trade_date: query.trade_date,
                    event_type: non_empty(query.event_type),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn get_portfolio_performance(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioPerformanceQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    let security_code = non_empty(query.security_code).unwrap_or_else(default_benchmark);
    let window_key = non_empty(query.window_key).unwrap_or_else(default_metric_window);
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_performance(
                &portfolio_run_id,
                &attempt_id,
                &security_code,
                &window_key,
            )
            .await?,
    ))
}

async fn list_portfolio_closed_trades(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioClosedTradeQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_closed_trades(
                &PortfolioClosedTradeFilter {
                    portfolio_run_id,
                    security_code: non_empty(query.security_code),
                    exit_date: query.exit_date,
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
}

async fn list_portfolio_trade_metrics(
    State(state): State<AppState>,
    Path(portfolio_run_id): Path<String>,
    Query(query): Query<PortfolioTradeMetricQuery>,
) -> RearviewResult<Json<impl Serialize>> {
    let attempt_id = resolve_result_attempt(
        &state,
        &portfolio_run_id,
        query.result_attempt_id.as_deref(),
    )
    .await?;
    Ok(Json(
        state
            .clickhouse
            .query_portfolio_trade_metrics(
                &PortfolioTradeMetricFilter {
                    portfolio_run_id,
                    window_key: non_empty(query.window_key),
                    page: page(query.limit, query.offset)?,
                },
                &attempt_id,
            )
            .await?,
    ))
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
        AnalysisSource::Preview => {
            return Err(RearviewError::Validation(
                "preview analysis source must use /rearview/strategy-preview/security-analysis"
                    .to_string(),
            ));
        }
    };
    let display = security_display_for_one(
        &state,
        &security_code,
        &format!(
            "rearview-analysis-display-{run_id}-{security_code}-{}",
            request.trade_date
        ),
    )
    .await;
    let security_name = display
        .as_ref()
        .and_then(|display| display.security_name.clone());
    let exchange_code = display
        .as_ref()
        .and_then(|display| display.exchange_code.clone());
    let security_board = display
        .as_ref()
        .and_then(|display| display.security_board.clone());

    let query_id_prefix = format!(
        "rearview-analysis-{run_id}-{security_code}-{}",
        request.trade_date
    );
    let quote_start_date = resolve_analysis_quote_start_date(
        &state,
        request.quote_start_date,
        request.quote_end_date,
        request.lookback_trading_days,
        &format!("{query_id_prefix}-date-window"),
    )
    .await?;
    let quote_rows = if quote_start_date.is_some() {
        state
            .clickhouse
            .query_analysis_quote_rows(
                &security_code,
                quote_start_date,
                request.quote_end_date,
                request.lookback_trading_days,
                &format!("{query_id_prefix}-quotes"),
            )
            .await?
    } else {
        Vec::new()
    };

    let (chart_start_date, chart_end_date) = quote_rows
        .first()
        .zip(quote_rows.last())
        .map(|(first, last)| (first.trade_date, last.trade_date))
        .unwrap_or((
            quote_start_date.unwrap_or(request.quote_end_date),
            request.quote_end_date,
        ));

    let (trend_rows, momentum_rows) = if quote_rows.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let trend_query_id = format!("{query_id_prefix}-trend");
        let momentum_query_id = format!("{query_id_prefix}-momentum");
        tokio::try_join!(
            state.clickhouse.query_analysis_trend_rows(
                &security_code,
                chart_start_date,
                chart_end_date,
                &trend_query_id,
            ),
            state.clickhouse.query_analysis_momentum_rows(
                &security_code,
                chart_start_date,
                chart_end_date,
                &momentum_query_id,
            )
        )?
    };

    let response = build_security_analysis_response(
        SecurityAnalysisBuildInput {
            run_id: Some(run_id),
            security_code,
            security_name,
            exchange_code,
            security_board,
            trade_date: request.trade_date,
            source: request.source,
            adjustment: request.adjustment,
            ma_windows: request.ma_windows,
            lookback_trading_days: request.lookback_trading_days,
            chart_start_date,
            chart_end_date,
            include_quote_rows: true,
        },
        Some(result_snapshot),
        None,
        quote_rows,
        trend_rows,
        momentum_rows,
        state.config.clickhouse.marts_database.clone(),
    );
    Ok(Json(response))
}

async fn analyze_security(
    State(state): State<AppState>,
    Json(request): Json<SecurityAnalysisContextRequest>,
) -> RearviewResult<Json<impl Serialize>> {
    let request = request.into_parts()?;
    let display = security_display_for_one(
        &state,
        &request.security_code,
        &format!(
            "rearview-security-analysis-display-{}-{}",
            request.security_code, request.analysis.trade_date
        ),
    )
    .await;
    let security_name = display
        .as_ref()
        .and_then(|display| display.security_name.clone());
    let exchange_code = display
        .as_ref()
        .and_then(|display| display.exchange_code.clone());
    let security_board = display
        .as_ref()
        .and_then(|display| display.security_board.clone());

    let query_id_prefix = format!(
        "rearview-security-analysis-{}-{}-{}",
        ulid::Ulid::new(),
        request.security_code,
        request.analysis.trade_date
    );
    let quote_start_date = resolve_analysis_quote_start_date(
        &state,
        request.analysis.quote_start_date,
        request.analysis.quote_end_date,
        request.analysis.lookback_trading_days,
        &format!("{query_id_prefix}-date-window"),
    )
    .await?;
    let (quote_rows, selected_quote) = if let Some(quote_start_date) = quote_start_date {
        if request.include_quote_rows {
            (
                state
                    .clickhouse
                    .query_analysis_quote_rows(
                        &request.security_code,
                        Some(quote_start_date),
                        request.analysis.quote_end_date,
                        request.analysis.lookback_trading_days,
                        &format!("{query_id_prefix}-quotes"),
                    )
                    .await?,
                None,
            )
        } else {
            let chart_query_id = format!("{query_id_prefix}-chart-quotes");
            let selected_query_id = format!("{query_id_prefix}-selected-quote");
            tokio::try_join!(
                state.clickhouse.query_analysis_chart_quote_rows(
                    &request.security_code,
                    quote_start_date,
                    request.analysis.quote_end_date,
                    request.analysis.adjustment.into(),
                    &chart_query_id,
                ),
                state.clickhouse.query_analysis_selected_quote_row(
                    &request.security_code,
                    request.analysis.trade_date,
                    &selected_query_id,
                )
            )?
        }
    } else {
        (Vec::new(), None)
    };

    let (chart_start_date, chart_end_date) = quote_rows
        .first()
        .zip(quote_rows.last())
        .map(|(first, last)| (first.trade_date, last.trade_date))
        .unwrap_or((
            quote_start_date.unwrap_or(request.analysis.quote_end_date),
            request.analysis.quote_end_date,
        ));

    let (trend_rows, momentum_rows) = if quote_rows.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let trend_query_id = format!("{query_id_prefix}-trend");
        let momentum_query_id = format!("{query_id_prefix}-momentum");
        tokio::try_join!(
            state.clickhouse.query_analysis_trend_rows(
                &request.security_code,
                chart_start_date,
                chart_end_date,
                &trend_query_id,
            ),
            state.clickhouse.query_analysis_momentum_rows(
                &request.security_code,
                chart_start_date,
                chart_end_date,
                &momentum_query_id,
            )
        )?
    };

    let response = build_security_analysis_response(
        SecurityAnalysisBuildInput {
            run_id: None,
            security_code: request.security_code,
            security_name,
            exchange_code,
            security_board,
            trade_date: request.analysis.trade_date,
            source: AnalysisSource::Preview,
            adjustment: request.analysis.adjustment,
            ma_windows: request.analysis.ma_windows,
            lookback_trading_days: request.analysis.lookback_trading_days,
            chart_start_date,
            chart_end_date,
            include_quote_rows: request.include_quote_rows,
        },
        None,
        selected_quote,
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

async fn preview_strategy(
    State(state): State<AppState>,
    Json(request): Json<StrategyPreviewRequest>,
) -> RearviewResult<Json<StrategyPreviewResponse>> {
    let request = request.into_parts(state.config.chunk_small_range_trading_days)?;
    let planner = QueryPlanner::new(state.catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: state.config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: state.config.clickhouse.max_rows_to_read,
        max_bytes_to_read: state.config.clickhouse.max_bytes_to_read,
    };
    let compiled = planner.compile_preview(
        &request.rule,
        request.start_date,
        request.end_date,
        request.preview_row_limit,
        settings,
    )?;
    let preview_id = ulid::Ulid::new().to_string();
    let query_id = format!("rearview-preview-{preview_id}");
    let rows = state
        .clickhouse
        .query_screening_rows(&compiled.sql, &query_id)
        .await?;
    let display_by_code = security_display_map(
        &state,
        &collect_security_codes(&rows),
        &format!("{query_id}-display"),
    )
    .await;
    let trade_dates =
        build_strategy_preview_trade_dates(rows, request.preview_row_limit, &display_by_code)?;

    Ok(Json(StrategyPreviewResponse {
        preview_id,
        sql_hash: compiled.sql_hash,
        required_metrics: compiled.required_metrics,
        required_marts: compiled.required_marts,
        required_columns: compiled.required_columns,
        start_date: request.start_date,
        end_date: request.end_date,
        preview_row_limit: request.preview_row_limit,
        top_n: request.preview_row_limit,
        trade_dates,
    }))
}

async fn preview_strategy_timeline(
    State(state): State<AppState>,
    Json(request): Json<StrategyPreviewTimelineRequest>,
) -> RearviewResult<Json<StrategyPreviewTimelineResponse>> {
    let request = request.into_parts()?;
    let planner = QueryPlanner::new(state.catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: state.config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: state.config.clickhouse.max_rows_to_read,
        max_bytes_to_read: state.config.clickhouse.max_bytes_to_read,
    };
    let compiled = planner.compile_preview_timeline(
        &request.rule,
        request.start_date,
        request.end_date,
        settings,
    )?;
    let preview_id = ulid::Ulid::new().to_string();
    let query_id = format!("rearview-preview-timeline-{preview_id}");
    let rows = state
        .clickhouse
        .query_preview_timeline_rows(&compiled.sql, &query_id)
        .await?;
    let trade_dates = rows
        .into_iter()
        .map(|row| StrategyPreviewTimelineTradeDate {
            trade_date: row.trade_date,
            pool_count: row.pool_count,
        })
        .collect();

    Ok(Json(StrategyPreviewTimelineResponse {
        preview_id,
        sql_hash: compiled.sql_hash,
        required_metrics: compiled.required_metrics,
        required_marts: compiled.required_marts,
        required_columns: compiled.required_columns,
        start_date: request.start_date,
        end_date: request.end_date,
        trade_dates,
    }))
}

async fn preview_strategy_pool_page(
    State(state): State<AppState>,
    Json(request): Json<StrategyPreviewPoolPageRequest>,
) -> RearviewResult<Json<StrategyPreviewPoolPageResponse>> {
    let request = request.into_parts()?;
    let planner = QueryPlanner::new(state.catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: state.config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: state.config.clickhouse.max_rows_to_read,
        max_bytes_to_read: state.config.clickhouse.max_bytes_to_read,
    };
    let query_limit = request.limit.saturating_add(1);
    let compiled = planner.compile_preview_pool_page(
        &request.rule,
        request.trade_date,
        query_limit,
        request.offset,
        request.security_code.as_deref(),
        settings,
    )?;
    let query_id = format!(
        "rearview-preview-pool-page-{}-{}",
        ulid::Ulid::new(),
        request.trade_date
    );
    let mut rows = state
        .clickhouse
        .query_screening_rows(&compiled.sql, &query_id)
        .await?;
    let has_more = rows.len() > request.limit as usize;
    if has_more {
        rows.truncate(request.limit as usize);
    }
    let pool_count = rows
        .iter()
        .filter_map(|row| row.pool_count)
        .max()
        .unwrap_or(0);
    let display_by_code = security_display_map(
        &state,
        &collect_security_codes(&rows),
        &format!("{query_id}-display"),
    )
    .await;
    let items = rows
        .into_iter()
        .map(|row| build_strategy_preview_signal(row, &display_by_code))
        .collect::<RearviewResult<Vec<_>>>()?;

    Ok(Json(StrategyPreviewPoolPageResponse {
        trade_date: request.trade_date,
        pool_count,
        items,
        limit: request.limit,
        offset: request.offset,
        has_more,
    }))
}

async fn preview_strategy_security_analysis(
    State(state): State<AppState>,
    Json(request): Json<StrategyPreviewSecurityAnalysisRequest>,
) -> RearviewResult<Json<impl Serialize>> {
    let request = request.into_parts()?;
    let planner = QueryPlanner::new(state.catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: state.config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: state.config.clickhouse.max_rows_to_read,
        max_bytes_to_read: state.config.clickhouse.max_bytes_to_read,
    };
    let compiled = planner.compile_preview_pool_page(
        &request.rule,
        request.analysis.trade_date,
        1,
        0,
        Some(&request.security_code),
        settings,
    )?;
    let query_id_prefix = format!(
        "rearview-preview-analysis-{}-{}-{}",
        ulid::Ulid::new(),
        request.security_code,
        request.analysis.trade_date
    );
    let mut rows = state
        .clickhouse
        .query_screening_rows(&compiled.sql, &format!("{query_id_prefix}-member"))
        .await?;
    let row = rows.pop().ok_or_else(|| {
        RearviewError::NotFound(format!(
            "security {} is not in preview pool on {}",
            request.security_code, request.analysis.trade_date
        ))
    })?;
    let display = security_display_for_one(
        &state,
        &request.security_code,
        &format!("{query_id_prefix}-display"),
    )
    .await;
    let security_name = display
        .as_ref()
        .and_then(|display| display.security_name.clone());
    let exchange_code = display
        .as_ref()
        .and_then(|display| display.exchange_code.clone());
    let security_board = display
        .as_ref()
        .and_then(|display| display.security_board.clone());
    let quote_start_date = resolve_analysis_quote_start_date(
        &state,
        request.analysis.quote_start_date,
        request.analysis.quote_end_date,
        request.analysis.lookback_trading_days,
        &format!("{query_id_prefix}-date-window"),
    )
    .await?;
    let (quote_rows, selected_quote) = if let Some(quote_start_date) = quote_start_date {
        if request.include_quote_rows {
            (
                state
                    .clickhouse
                    .query_analysis_quote_rows(
                        &request.security_code,
                        Some(quote_start_date),
                        request.analysis.quote_end_date,
                        request.analysis.lookback_trading_days,
                        &format!("{query_id_prefix}-quotes"),
                    )
                    .await?,
                None,
            )
        } else {
            let chart_query_id = format!("{query_id_prefix}-chart-quotes");
            let selected_query_id = format!("{query_id_prefix}-selected-quote");
            tokio::try_join!(
                state.clickhouse.query_analysis_chart_quote_rows(
                    &request.security_code,
                    quote_start_date,
                    request.analysis.quote_end_date,
                    request.analysis.adjustment.into(),
                    &chart_query_id,
                ),
                state.clickhouse.query_analysis_selected_quote_row(
                    &request.security_code,
                    request.analysis.trade_date,
                    &selected_query_id,
                )
            )?
        }
    } else {
        (Vec::new(), None)
    };

    let (chart_start_date, chart_end_date) = quote_rows
        .first()
        .zip(quote_rows.last())
        .map(|(first, last)| (first.trade_date, last.trade_date))
        .unwrap_or((
            quote_start_date.unwrap_or(request.analysis.quote_end_date),
            request.analysis.quote_end_date,
        ));

    let (trend_rows, momentum_rows) = if quote_rows.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let trend_query_id = format!("{query_id_prefix}-trend");
        let momentum_query_id = format!("{query_id_prefix}-momentum");
        tokio::try_join!(
            state.clickhouse.query_analysis_trend_rows(
                &request.security_code,
                chart_start_date,
                chart_end_date,
                &trend_query_id,
            ),
            state.clickhouse.query_analysis_momentum_rows(
                &request.security_code,
                chart_start_date,
                chart_end_date,
                &momentum_query_id,
            )
        )?
    };

    let response = build_security_analysis_response(
        SecurityAnalysisBuildInput {
            run_id: None,
            security_code: request.security_code,
            security_name,
            exchange_code,
            security_board,
            trade_date: request.analysis.trade_date,
            source: AnalysisSource::Preview,
            adjustment: request.analysis.adjustment,
            ma_windows: request.analysis.ma_windows,
            lookback_trading_days: request.analysis.lookback_trading_days,
            chart_start_date,
            chart_end_date,
            include_quote_rows: request.include_quote_rows,
        },
        Some(ResultSnapshot::from_preview(row)?),
        selected_quote,
        quote_rows,
        trend_rows,
        momentum_rows,
        state.config.clickhouse.marts_database.clone(),
    );
    Ok(Json(response))
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
struct CreatePortfolioRunRequest {
    source_run_id: String,
    #[serde(default)]
    account_template_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StrategyBacktestOptionsQuery {
    #[serde(default)]
    benchmark_security_code: Option<String>,
    #[serde(default)]
    as_of_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
struct StrategyBacktestCreateRequest {
    rule: RuleVersionSpec,
    period_key: String,
    benchmark_security_code: String,
    execution_config: BacktestExecutionConfig,
    #[serde(default)]
    preview_id: Option<String>,
    #[serde(default)]
    preview_range: Option<BacktestDateRange>,
    #[serde(default)]
    top_n: Option<u32>,
    #[serde(default)]
    rule_hash: Option<String>,
    #[serde(default)]
    execution_config_hash: Option<String>,
    #[serde(default)]
    client_request_id: Option<String>,
    #[serde(default)]
    ui_display_snapshot: Option<Value>,
    #[serde(default)]
    range_hint: Option<BacktestDateRange>,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestOptionsResponse {
    default_period_key: String,
    default_benchmark_security_code: String,
    selected_benchmark_security_code: String,
    as_of_date: NaiveDate,
    latest_available_trade_date: NaiveDate,
    period_options: Vec<StrategyBacktestPeriodOption>,
    benchmark_options: Vec<StrategyBacktestBenchmarkOption>,
    range_resolution_snapshot: Value,
}

#[derive(Debug, Clone, Serialize)]
struct StrategyBacktestPeriodOption {
    period_key: String,
    label: String,
    resolved_start_date: NaiveDate,
    resolved_end_date: NaiveDate,
    latest_available_trade_date: NaiveDate,
    benchmark_security_code: String,
    range_resolution_snapshot: Value,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestBenchmarkOption {
    security_code: String,
    label: String,
    is_default: bool,
    availability_status: String,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestRunResponse {
    #[serde(flatten)]
    record: StrategyBacktestRunRecord,
    config_summary: BacktestExecutionSummary,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestNavPoint {
    trade_date: NaiveDate,
    strategy_nav: f64,
    benchmark_nav: Option<f64>,
    excess_return: Option<f64>,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestPerformanceView {
    metric: PortfolioPerformanceMetricRecord,
    statuses: Vec<PortfolioPerformanceMetricStatusRecord>,
    daily_win_rate: StrategyBacktestDailyWinRate,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestDailyWinRate {
    value: Option<f64>,
    observation_count: usize,
    winning_day_count: usize,
}

#[derive(Debug, Deserialize)]
struct StrategyBacktestRebalanceQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    trade_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestRebalanceRecordsResponse {
    selected_trade_date: NaiveDate,
    records: Vec<StrategyBacktestRebalanceRecord>,
}

#[derive(Debug, Serialize)]
struct StrategyBacktestRebalanceRecord {
    trade_date: NaiveDate,
    position_count: i32,
    buy_count: usize,
    hold_count: usize,
    sell_count: usize,
    rows: Vec<StrategyBacktestRebalanceRow>,
}

#[derive(Debug, Clone, Serialize)]
struct StrategyBacktestRebalanceRow {
    direction: String,
    security_code: String,
    security_name: Option<String>,
    quantity: f64,
    holding_days: Option<i32>,
    change_pct: Option<f64>,
    cost_price: Option<f64>,
    current_price: Option<f64>,
    contribution_pct: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug)]
struct StrategyBacktestRangeResolution {
    latest_available_trade_date: NaiveDate,
    period_options: Vec<StrategyBacktestPeriodOption>,
    trade_date_count: usize,
    benchmark_return_count: usize,
    range_resolution_snapshot: Value,
}

#[derive(Debug, Deserialize)]
struct DefaultMarketFeeTemplateQuery {
    #[serde(default)]
    market: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateAccountTemplateRequest {
    #[serde(default)]
    market: Option<String>,
    #[serde(default)]
    market_fee_template_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    initial_cash: Option<f64>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    fee_profile: Option<serde_json::Value>,
    #[serde(default)]
    slippage_profile: Option<serde_json::Value>,
    #[serde(default)]
    rebalance_policy: Option<serde_json::Value>,
    #[serde(default)]
    risk_exit_policy: Option<serde_json::Value>,
    #[serde(default)]
    is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PatchAccountTemplateRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    initial_cash: Option<f64>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    fee_profile: Option<serde_json::Value>,
    #[serde(default)]
    slippage_profile: Option<serde_json::Value>,
    #[serde(default)]
    rebalance_policy: Option<serde_json::Value>,
    #[serde(default)]
    risk_exit_policy: Option<serde_json::Value>,
    #[serde(default)]
    is_default: Option<bool>,
    #[serde(default)]
    status: Option<String>,
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
struct ListPortfolioRunsQuery {
    #[serde(default)]
    source_run_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    dispatch_status: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PortfolioNavQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PortfolioTargetQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    signal_date: Option<NaiveDate>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PortfolioOrderQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    execution_date: Option<NaiveDate>,
    #[serde(default)]
    security_code: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PortfolioTradeQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    trade_date: Option<NaiveDate>,
    #[serde(default)]
    security_code: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PortfolioPositionQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    trade_date: Option<NaiveDate>,
    #[serde(default)]
    security_code: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PortfolioEventQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    trade_date: Option<NaiveDate>,
    #[serde(default)]
    event_type: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PortfolioPerformanceQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    security_code: Option<String>,
    #[serde(default)]
    window_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PortfolioClosedTradeQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    security_code: Option<String>,
    #[serde(default)]
    exit_date: Option<NaiveDate>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PortfolioTradeMetricQuery {
    #[serde(default)]
    result_attempt_id: Option<String>,
    #[serde(default)]
    window_key: Option<String>,
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
    Preview,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Adjustment {
    ForwardAdjusted,
    BackwardAdjusted,
    Unadjusted,
}

impl From<Adjustment> for AnalysisQuoteAdjustment {
    fn from(value: Adjustment) -> Self {
        match value {
            Adjustment::ForwardAdjusted => Self::ForwardAdjusted,
            Adjustment::BackwardAdjusted => Self::BackwardAdjusted,
            Adjustment::Unadjusted => Self::Unadjusted,
        }
    }
}

#[derive(Debug, Serialize)]
struct SecurityAnalysisResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    trade_date: NaiveDate,
    security_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exchange_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_board: Option<String>,
    source: AnalysisSource,
    adjustment: Adjustment,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_snapshot: Option<ResultSnapshot>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_values: Option<serde_json::Value>,
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
            raw_values: None,
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
            raw_values: None,
            filter_snapshot: Some(record.filter_snapshot),
        }
    }

    fn from_preview(row: crate::clickhouse::ScreeningRow) -> RearviewResult<Self> {
        Ok(Self {
            rank: None,
            signal_rank: Some(i32::try_from(row.signal_rank).map_err(|error| {
                RearviewError::Validation(format!("preview signal_rank out of range: {error}"))
            })?),
            score: Some(row.score),
            score_breakdown: Some(parse_preview_json_field(&row.score_breakdown)?),
            selected_metrics: parse_preview_json_field(&row.selected_metrics)?,
            raw_values: Some(parse_preview_json_field(&row.raw_values)?),
            filter_snapshot: None,
        })
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
    basis_adjustment: Adjustment,
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
    run_id: Option<String>,
    security_code: String,
    security_name: Option<String>,
    exchange_code: Option<String>,
    security_board: Option<String>,
    trade_date: NaiveDate,
    source: AnalysisSource,
    adjustment: Adjustment,
    ma_windows: Vec<u32>,
    lookback_trading_days: u32,
    chart_start_date: NaiveDate,
    chart_end_date: NaiveDate,
    include_quote_rows: bool,
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

#[derive(Debug, Deserialize)]
struct StrategyPreviewRequest {
    rule: RuleVersionSpec,
    start_date: NaiveDate,
    end_date: NaiveDate,
    #[serde(default)]
    preview_row_limit: Option<u32>,
    #[serde(default)]
    top_n: Option<u32>,
}

impl StrategyPreviewRequest {
    fn into_parts(self, max_range_days: u32) -> RearviewResult<StrategyPreviewRequestParts> {
        let preview_row_limit = self.preview_row_limit.or(self.top_n).unwrap_or(50);
        if preview_row_limit == 0 {
            return Err(RearviewError::Validation(
                "preview_row_limit must be greater than 0".to_string(),
            ));
        }
        if preview_row_limit > 500 {
            return Err(RearviewError::Validation(
                "preview_row_limit must not exceed 500".to_string(),
            ));
        }
        if self.start_date > self.end_date {
            return Err(RearviewError::Validation(
                "start_date must be earlier than or equal to end_date".to_string(),
            ));
        }
        if max_range_days == 0 {
            return Err(RearviewError::Validation(
                "preview max date range must be greater than 0".to_string(),
            ));
        }

        let day_count = (self.end_date - self.start_date).num_days() + 1;
        if day_count > i64::from(max_range_days) {
            return Err(RearviewError::Validation(format!(
                "preview date range must not exceed {max_range_days} days"
            )));
        }

        Ok(StrategyPreviewRequestParts {
            rule: self.rule,
            start_date: self.start_date,
            end_date: self.end_date,
            preview_row_limit,
        })
    }
}

#[derive(Debug)]
struct StrategyPreviewRequestParts {
    rule: RuleVersionSpec,
    start_date: NaiveDate,
    end_date: NaiveDate,
    preview_row_limit: u32,
}

#[derive(Debug, Serialize)]
struct StrategyPreviewResponse {
    preview_id: String,
    sql_hash: String,
    required_metrics: Vec<String>,
    required_marts: Vec<String>,
    required_columns: BTreeMap<String, Vec<String>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    preview_row_limit: u32,
    top_n: u32,
    trade_dates: Vec<StrategyPreviewTradeDate>,
}

#[derive(Debug, Deserialize)]
struct StrategyPreviewTimelineRequest {
    rule: RuleVersionSpec,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl StrategyPreviewTimelineRequest {
    fn into_parts(self) -> RearviewResult<StrategyPreviewTimelineRequestParts> {
        const MAX_TIMELINE_DAYS: i64 = 370;
        if self.start_date > self.end_date {
            return Err(RearviewError::Validation(
                "start_date must be earlier than or equal to end_date".to_string(),
            ));
        }
        let day_count = (self.end_date - self.start_date).num_days() + 1;
        if day_count > MAX_TIMELINE_DAYS {
            return Err(RearviewError::Validation(format!(
                "preview timeline date range must not exceed {MAX_TIMELINE_DAYS} days"
            )));
        }

        Ok(StrategyPreviewTimelineRequestParts {
            rule: self.rule,
            start_date: self.start_date,
            end_date: self.end_date,
        })
    }
}

#[derive(Debug)]
struct StrategyPreviewTimelineRequestParts {
    rule: RuleVersionSpec,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[derive(Debug, Serialize)]
struct StrategyPreviewTimelineResponse {
    preview_id: String,
    sql_hash: String,
    required_metrics: Vec<String>,
    required_marts: Vec<String>,
    required_columns: BTreeMap<String, Vec<String>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    trade_dates: Vec<StrategyPreviewTimelineTradeDate>,
}

#[derive(Debug, Serialize)]
struct StrategyPreviewTimelineTradeDate {
    trade_date: NaiveDate,
    pool_count: usize,
}

#[derive(Debug, Serialize)]
struct StrategyPreviewTradeDate {
    trade_date: NaiveDate,
    pool_count: usize,
    signals: Vec<StrategyPreviewSignal>,
}

#[derive(Debug, Serialize)]
struct StrategyPreviewSignal {
    security_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exchange_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_board: Option<String>,
    raw_score: f64,
    score: f64,
    signal_rank: u32,
    is_buy_signal: bool,
    score_breakdown: serde_json::Value,
    selected_metrics: serde_json::Value,
    raw_values: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct StrategyPreviewPoolPageRequest {
    rule: RuleVersionSpec,
    trade_date: NaiveDate,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    security_code: Option<String>,
}

impl StrategyPreviewPoolPageRequest {
    fn into_parts(self) -> RearviewResult<StrategyPreviewPoolPageRequestParts> {
        if self.sort.as_deref().unwrap_or("score_desc") != "score_desc" {
            return Err(RearviewError::Validation(
                "strategy preview pool-page only supports score_desc sort".to_string(),
            ));
        }
        let page = page(self.limit, self.offset)?;
        Ok(StrategyPreviewPoolPageRequestParts {
            rule: self.rule,
            trade_date: self.trade_date,
            limit: u32::try_from(page.limit).map_err(|error| {
                RearviewError::Validation(format!("limit out of range: {error}"))
            })?,
            offset: u32::try_from(page.offset).map_err(|error| {
                RearviewError::Validation(format!("offset out of range: {error}"))
            })?,
            security_code: non_empty(self.security_code),
        })
    }
}

#[derive(Debug)]
struct StrategyPreviewPoolPageRequestParts {
    rule: RuleVersionSpec,
    trade_date: NaiveDate,
    limit: u32,
    offset: u32,
    security_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct StrategyPreviewPoolPageResponse {
    trade_date: NaiveDate,
    pool_count: usize,
    items: Vec<StrategyPreviewSignal>,
    limit: u32,
    offset: u32,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct StrategyPreviewSecurityAnalysisRequest {
    rule: RuleVersionSpec,
    trade_date: NaiveDate,
    security_code: String,
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
    #[serde(default)]
    include_quote_rows: Option<bool>,
}

impl StrategyPreviewSecurityAnalysisRequest {
    fn into_parts(self) -> RearviewResult<StrategyPreviewSecurityAnalysisRequestParts> {
        let security_code = self.security_code.trim().to_string();
        if security_code.is_empty() {
            return Err(RearviewError::Validation(
                "security_code must not be empty".to_string(),
            ));
        }
        let analysis = SecurityAnalysisQuery {
            trade_date: self.trade_date,
            source: AnalysisSource::Preview,
            adjustment: self.adjustment,
            quote_end_date: self.quote_end_date,
            lookback_trading_days: self.lookback_trading_days,
            quote_start_date: self.quote_start_date,
            ma_windows: self.ma_windows,
        }
        .into_request()?;
        Ok(StrategyPreviewSecurityAnalysisRequestParts {
            rule: self.rule,
            security_code,
            analysis,
            include_quote_rows: self.include_quote_rows.unwrap_or(true),
        })
    }
}

#[derive(Debug)]
struct StrategyPreviewSecurityAnalysisRequestParts {
    rule: RuleVersionSpec,
    security_code: String,
    analysis: SecurityAnalysisRequest,
    include_quote_rows: bool,
}

#[derive(Debug, Deserialize)]
struct SecurityAnalysisContextRequest {
    trade_date: NaiveDate,
    security_code: String,
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
    #[serde(default)]
    include_quote_rows: Option<bool>,
}

impl SecurityAnalysisContextRequest {
    fn into_parts(self) -> RearviewResult<SecurityAnalysisContextRequestParts> {
        let security_code = self.security_code.trim().to_string();
        if security_code.is_empty() {
            return Err(RearviewError::Validation(
                "security_code must not be empty".to_string(),
            ));
        }
        let analysis = SecurityAnalysisQuery {
            trade_date: self.trade_date,
            source: AnalysisSource::Preview,
            adjustment: self.adjustment,
            quote_end_date: self.quote_end_date,
            lookback_trading_days: self.lookback_trading_days,
            quote_start_date: self.quote_start_date,
            ma_windows: self.ma_windows,
        }
        .into_request()?;
        Ok(SecurityAnalysisContextRequestParts {
            security_code,
            analysis,
            include_quote_rows: self.include_quote_rows.unwrap_or(false),
        })
    }
}

#[derive(Debug)]
struct SecurityAnalysisContextRequestParts {
    security_code: String,
    analysis: SecurityAnalysisRequest,
    include_quote_rows: bool,
}

fn build_strategy_preview_trade_dates(
    rows: Vec<crate::clickhouse::ScreeningRow>,
    preview_row_limit: u32,
    display_by_code: &BTreeMap<String, SecurityDisplayRow>,
) -> RearviewResult<Vec<StrategyPreviewTradeDate>> {
    let mut grouped: BTreeMap<NaiveDate, StrategyPreviewTradeDate> = BTreeMap::new();

    for row in rows {
        let entry = grouped
            .entry(row.trade_date)
            .or_insert_with(|| StrategyPreviewTradeDate {
                trade_date: row.trade_date,
                pool_count: 0,
                signals: Vec::new(),
            });
        if let Some(pool_count) = row.pool_count {
            entry.pool_count = entry.pool_count.max(pool_count);
        } else {
            entry.pool_count += 1;
        }
        if row.is_buy_signal || row.signal_rank <= preview_row_limit {
            entry
                .signals
                .push(build_strategy_preview_signal(row, display_by_code)?);
        }
    }

    Ok(grouped.into_values().collect())
}

fn build_strategy_preview_signal(
    row: crate::clickhouse::ScreeningRow,
    display_by_code: &BTreeMap<String, SecurityDisplayRow>,
) -> RearviewResult<StrategyPreviewSignal> {
    let display = display_by_code.get(&row.security_code);
    Ok(StrategyPreviewSignal {
        security_name: display.and_then(|display| display.security_name.clone()),
        exchange_code: display.and_then(|display| display.exchange_code.clone()),
        security_board: display.and_then(|display| display.security_board.clone()),
        security_code: row.security_code,
        raw_score: row.raw_score,
        score: row.score,
        signal_rank: row.signal_rank,
        is_buy_signal: row.is_buy_signal,
        score_breakdown: parse_preview_json_field(&row.score_breakdown)?,
        selected_metrics: parse_preview_json_field(&row.selected_metrics)?,
        raw_values: parse_preview_json_field(&row.raw_values)?,
    })
}

fn collect_security_codes(rows: &[crate::clickhouse::ScreeningRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.security_code.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

async fn security_display_map(
    state: &AppState,
    security_codes: &[String],
    query_id: &str,
) -> BTreeMap<String, SecurityDisplayRow> {
    match state
        .clickhouse
        .query_security_display_rows(security_codes, query_id)
        .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| (row.security_code.clone(), row))
            .collect(),
        Err(_) => BTreeMap::new(),
    }
}

async fn security_display_for_one(
    state: &AppState,
    security_code: &str,
    query_id: &str,
) -> Option<SecurityDisplayRow> {
    let rows = security_display_map(state, &[security_code.to_string()], query_id).await;
    rows.get(security_code).cloned()
}

fn parse_preview_json_field(raw: &str) -> RearviewResult<serde_json::Value> {
    if raw.trim().is_empty() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }

    Ok(serde_json::from_str(raw)?)
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

async fn resolve_analysis_quote_start_date(
    state: &AppState,
    quote_start_date: Option<NaiveDate>,
    quote_end_date: NaiveDate,
    lookback_trading_days: u32,
    query_id: &str,
) -> RearviewResult<Option<NaiveDate>> {
    if quote_start_date.is_some() {
        return Ok(quote_start_date);
    }
    state
        .clickhouse
        .query_trade_date_lookback_start(quote_end_date, lookback_trading_days, query_id)
        .await
}

fn default_market() -> String {
    "CN_A_SHARE".to_string()
}

fn default_benchmark() -> String {
    "000300.SH".to_string()
}

fn default_metric_window() -> String {
    "full_period".to_string()
}

fn validate_strategy_backtest_period_key(period_key: &str) -> RearviewResult<()> {
    if matches!(period_key, "1y" | "2y" | "3y") {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!(
            "period_key must be one of 1y, 2y, or 3y: {period_key}"
        )))
    }
}

fn validate_strategy_backtest_benchmark(security_code: &str) -> RearviewResult<()> {
    if strategy_backtest_benchmark_label(security_code).is_some() {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!(
            "unsupported benchmark_security_code: {security_code}"
        )))
    }
}

fn strategy_backtest_benchmark_options(
    selected_security_code: &str,
    selected_available: bool,
) -> Vec<StrategyBacktestBenchmarkOption> {
    strategy_backtest_benchmark_allowlist()
        .iter()
        .map(|(security_code, label)| StrategyBacktestBenchmarkOption {
            security_code: (*security_code).to_string(),
            label: (*label).to_string(),
            is_default: *security_code == "000300.SH",
            availability_status: if *security_code == selected_security_code {
                if selected_available {
                    "available".to_string()
                } else {
                    "unavailable".to_string()
                }
            } else {
                "not_checked".to_string()
            },
        })
        .collect()
}

fn strategy_backtest_benchmark_label(security_code: &str) -> Option<&'static str> {
    strategy_backtest_benchmark_allowlist()
        .iter()
        .find_map(|(candidate, label)| (*candidate == security_code).then_some(*label))
}

fn strategy_backtest_benchmark_allowlist() -> &'static [(&'static str, &'static str)] {
    &[
        ("000903.SH", "中证A100"),
        ("000300.SH", "沪深300"),
        ("000905.SH", "中证500"),
        ("000906.SH", "中证800"),
        ("000852.SH", "中证1000"),
        ("399311.SZ", "国证1000"),
    ]
}

async fn resolve_strategy_backtest_range(
    state: &AppState,
    benchmark_security_code: &str,
    as_of_date: NaiveDate,
) -> RearviewResult<StrategyBacktestRangeResolution> {
    let earliest_date = as_of_date
        .checked_sub_months(Months::new(36))
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "could not resolve 3y range before as_of_date {as_of_date}"
            ))
        })?;
    let query_suffix = format!("{benchmark_security_code}-{as_of_date}");
    let trade_dates = state
        .clickhouse
        .query_trade_dates(
            earliest_date,
            as_of_date,
            &format!("strategy-backtest-options-trade-dates-{query_suffix}"),
        )
        .await?;
    if trade_dates.is_empty() {
        return Err(RearviewError::Validation(format!(
            "no trade dates available between {earliest_date} and {as_of_date}"
        )));
    }
    let benchmark_returns = state
        .clickhouse
        .query_mart_benchmark_returns(
            benchmark_security_code,
            earliest_date,
            as_of_date,
            &format!("strategy-backtest-options-benchmark-{query_suffix}"),
        )
        .await?;
    let benchmark_dates = benchmark_returns
        .iter()
        .filter(|row| row.return_daily.is_some())
        .map(|row| row.trade_date)
        .collect::<BTreeSet<_>>();
    let latest_available_trade_date = trade_dates
        .iter()
        .rev()
        .copied()
        .find(|date| benchmark_dates.contains(date))
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "benchmark {benchmark_security_code} has no usable returns through {as_of_date}"
            ))
        })?;
    let mut period_options = Vec::new();
    for (period_key, label, years) in [
        ("1y", "近一年", 1_u32),
        ("2y", "近两年", 2_u32),
        ("3y", "近三年", 3_u32),
    ] {
        let target_start = latest_available_trade_date
            .checked_sub_months(Months::new(years * 12))
            .ok_or_else(|| {
                RearviewError::Validation(format!(
                    "could not resolve {period_key} start before {latest_available_trade_date}"
                ))
            })?;
        let resolved_start_date = trade_dates
            .iter()
            .copied()
            .find(|date| *date >= target_start && *date <= latest_available_trade_date)
            .ok_or_else(|| {
                RearviewError::Validation(format!(
                    "no trade date available for {period_key} start on or after {target_start}"
                ))
            })?;
        let range_resolution_snapshot = json!({
            "period_key": period_key,
            "as_of_date": as_of_date,
            "target_start_date": target_start,
            "resolved_start_date": resolved_start_date,
            "resolved_end_date": latest_available_trade_date,
            "benchmark_security_code": benchmark_security_code,
            "method": "quote_trade_dates_intersect_benchmark_returns",
        });
        period_options.push(StrategyBacktestPeriodOption {
            period_key: period_key.to_string(),
            label: label.to_string(),
            resolved_start_date,
            resolved_end_date: latest_available_trade_date,
            latest_available_trade_date,
            benchmark_security_code: benchmark_security_code.to_string(),
            range_resolution_snapshot,
        });
    }
    let range_resolution_snapshot = json!({
        "as_of_date": as_of_date,
        "earliest_query_date": earliest_date,
        "latest_available_trade_date": latest_available_trade_date,
        "benchmark_security_code": benchmark_security_code,
        "trade_date_count": trade_dates.len(),
        "benchmark_return_count": benchmark_returns.len(),
        "usable_benchmark_return_count": benchmark_dates.len(),
        "method": "quote_trade_dates_intersect_benchmark_returns",
    });
    Ok(StrategyBacktestRangeResolution {
        latest_available_trade_date,
        period_options,
        trade_date_count: trade_dates.len(),
        benchmark_return_count: benchmark_returns.len(),
        range_resolution_snapshot,
    })
}

fn strategy_backtest_run_response(
    record: StrategyBacktestRunRecord,
) -> RearviewResult<StrategyBacktestRunResponse> {
    let execution_config =
        serde_json::from_value::<BacktestExecutionConfig>(record.execution_config.clone())?;
    let config_summary = execution_config.summary()?;
    Ok(StrategyBacktestRunResponse {
        record,
        config_summary,
    })
}

struct StrategyBacktestRiskFreePreflight<'a> {
    tenor: &'a str,
    return_count: Option<usize>,
    error: Option<String>,
}

fn build_strategy_backtest_preflight_snapshot(
    request: &StrategyBacktestCreateRequest,
    period_option: &StrategyBacktestPeriodOption,
    resolution: &StrategyBacktestRangeResolution,
    draft: &StrategyBacktestDraftResponse,
    catalog_hash: &Option<String>,
    risk_free: StrategyBacktestRiskFreePreflight<'_>,
) -> Value {
    json!({
        "period_key": request.period_key,
        "range_hint": request.range_hint,
        "resolved_start_date": period_option.resolved_start_date,
        "resolved_end_date": period_option.resolved_end_date,
        "latest_available_trade_date": resolution.latest_available_trade_date,
        "resolved_trading_date_count": resolution.trade_date_count,
        "benchmark_security_code": request.benchmark_security_code,
        "benchmark_return_count": resolution.benchmark_return_count,
        "risk_free_tenor": risk_free.tenor,
        "risk_free_return_count": risk_free.return_count,
        "risk_free_preflight_error": risk_free.error,
        "catalog_hash": catalog_hash,
        "required_metrics": [],
        "required_marts": [],
        "execution_summary": draft.summary,
        "range_resolution_snapshot": period_option.range_resolution_snapshot,
    })
}

fn hash_catalog(catalog: &MetricCatalog) -> RearviewResult<String> {
    let metrics = catalog.iter().collect::<Vec<_>>();
    hash_json(&metrics)
}

fn default_rebalance_policy() -> serde_json::Value {
    serde_json::json!({
        "frequency": "signal_day",
        "target_weighting": "equal_weight_capped",
        "max_positions": 10,
        "single_position_limit_pct": 0.1,
        "lot_size": 100,
        "min_trade_lots": 1,
        "cash_reserve_pct": 0,
        "empty_signal_action": "hold"
    })
}

fn default_risk_exit_policy() -> serde_json::Value {
    serde_json::json!({
        "trigger_timing": "close_confirm_next_open",
        "exit_rules": []
    })
}

fn build_security_analysis_response(
    input: SecurityAnalysisBuildInput,
    result_snapshot: Option<ResultSnapshot>,
    selected_quote: Option<QuoteMartRow>,
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
    let selected_quote = selected_quote.or_else(|| {
        quote_rows
            .iter()
            .find(|row| row.trade_date == input.trade_date)
            .cloned()
    });
    let series = quote_rows
        .iter()
        .map(|quote| {
            let trend = trend_by_date.get(&quote.trade_date);
            let momentum = momentum_by_date.get(&quote.trade_date);
            ChartSeriesRow {
                trade_date: quote.trade_date,
                ohlc: ohlc_for_adjustment(quote, input.adjustment),
                volume: quote.volume,
                ma: ma_values(trend, &input.ma_windows),
                price_overlays: price_overlay_values(trend),
                kdj: kdj_values(momentum, quote),
                rsi: rsi_values(momentum),
                macd: macd_values(trend),
                boll: boll_values(trend),
            }
        })
        .collect::<Vec<_>>();
    let requested_windows = input.ma_windows;
    let available_windows = requested_windows.clone();
    let quote_rows_for_response = if input.include_quote_rows {
        quote_rows
    } else {
        Vec::new()
    };

    SecurityAnalysisResponse {
        run_id: input.run_id,
        trade_date: input.trade_date,
        security_code: input.security_code,
        security_name: input.security_name,
        exchange_code: input.exchange_code,
        security_board: input.security_board,
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
                table: "mart_stock_trend_indicator_daily",
                value_semantics: "current_mart_query",
                adjustment: Some(Adjustment::ForwardAdjusted),
            },
            momentum: SourceMetadata {
                database: marts_database,
                table: "mart_stock_momentum_indicator_daily",
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
                basis_adjustment: Adjustment::ForwardAdjusted,
                status: "available",
            },
            price_overlays: ChartPriceOverlayMetadata {
                default_visible_keys: vec!["price_ma_5", "price_ma_10", "price_ma_30"],
                available_keys: PRICE_OVERLAY_KEYS.to_vec(),
                adjustment: Adjustment::ForwardAdjusted,
                status: "available",
            },
            indicator_panels: ["kdj", "rsi", "macd", "boll"],
            series,
        },
        quote_rows: quote_rows_for_response,
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
) -> BTreeMap<String, Option<f64>> {
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

fn price_overlay_values(trend: Option<&TrendIndicatorRow>) -> BTreeMap<&'static str, Option<f64>> {
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
        let display_group = metric
            .display
            .as_ref()
            .and_then(|display| display.group.as_deref())
            .unwrap_or_default();
        let display_label_zh = metric
            .display
            .as_ref()
            .and_then(|display| display.label_zh.as_deref())
            .unwrap_or_default();
        return metric.logical_metric.to_lowercase().contains(keyword)
            || metric.column_name.to_lowercase().contains(keyword)
            || description.to_lowercase().contains(keyword)
            || display_group.to_lowercase().contains(keyword)
            || display_label_zh.to_lowercase().contains(keyword);
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

async fn resolve_result_attempt(
    state: &AppState,
    portfolio_run_id: &str,
    override_attempt: Option<&str>,
) -> RearviewResult<String> {
    if let Some(attempt) = override_attempt {
        return Ok(attempt.to_string());
    }
    state
        .postgres
        .get_current_result_attempt_id(portfolio_run_id)
        .await?
        .ok_or_else(|| {
            RearviewError::NotFound(format!(
                "no result attempt for portfolio run: {portfolio_run_id}"
            ))
        })
}

fn resolve_strategy_backtest_result_attempt(
    run: &StrategyBacktestRunRecord,
    override_attempt: Option<&str>,
) -> RearviewResult<String> {
    if let Some(attempt) = override_attempt {
        return Ok(attempt.to_string());
    }
    run.current_result_attempt_id.clone().ok_or_else(|| {
        RearviewError::NotFound(format!(
            "no result attempt for strategy backtest: {}",
            run.strategy_backtest_run_id
        ))
    })
}

fn strategy_backtest_nav_points(
    nav: Vec<PortfolioNavRecord>,
    benchmark_returns: Vec<BenchmarkReturn>,
) -> Vec<StrategyBacktestNavPoint> {
    let returns_by_date = benchmark_returns
        .into_iter()
        .map(|row| (row.trade_date, row.return_daily))
        .collect::<BTreeMap<_, _>>();
    let mut benchmark_nav = 1.0_f64;
    let mut benchmark_available = true;
    nav.into_iter()
        .enumerate()
        .map(|(index, row)| {
            let point_benchmark_nav = if index == 0 {
                Some(1.0)
            } else if benchmark_available {
                match returns_by_date.get(&row.trade_date).copied().flatten() {
                    Some(return_daily) => {
                        benchmark_nav *= 1.0 + return_daily;
                        Some(benchmark_nav)
                    }
                    None => {
                        benchmark_available = false;
                        None
                    }
                }
            } else {
                None
            };
            StrategyBacktestNavPoint {
                trade_date: row.trade_date,
                strategy_nav: row.nav,
                benchmark_nav: point_benchmark_nav,
                excess_return: point_benchmark_nav.map(|value| row.nav - value),
            }
        })
        .collect()
}

fn daily_win_rate(nav: &[PortfolioNavRecord]) -> StrategyBacktestDailyWinRate {
    let mut observation_count = 0_usize;
    let mut winning_day_count = 0_usize;
    for row in nav {
        if let Some(daily_return) = row.daily_return {
            observation_count += 1;
            if daily_return > 0.0 {
                winning_day_count += 1;
            }
        }
    }
    StrategyBacktestDailyWinRate {
        value: (observation_count > 0).then(|| winning_day_count as f64 / observation_count as f64),
        observation_count,
        winning_day_count,
    }
}

fn build_strategy_backtest_rebalance_rows(
    trades: Vec<PortfolioTradeRecord>,
    positions: Vec<PortfolioPositionRecord>,
    closed_trades: Vec<PortfolioClosedTradeRecord>,
    display: &BTreeMap<String, SecurityDisplayRow>,
    total_equity: Option<f64>,
) -> Vec<StrategyBacktestRebalanceRow> {
    let position_by_code = positions
        .iter()
        .map(|position| (position.security_code.clone(), position))
        .collect::<BTreeMap<_, _>>();
    let buy_codes = trades
        .iter()
        .filter(|trade| trade.side.eq_ignore_ascii_case("buy"))
        .map(|trade| trade.security_code.clone())
        .collect::<BTreeSet<_>>();
    let sell_codes = trades
        .iter()
        .filter(|trade| trade.side.eq_ignore_ascii_case("sell"))
        .map(|trade| trade.security_code.clone())
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::new();

    for trade in trades
        .iter()
        .filter(|trade| trade.side.eq_ignore_ascii_case("buy"))
    {
        let position = position_by_code.get(&trade.security_code).copied();
        rows.push(StrategyBacktestRebalanceRow {
            direction: "buy".to_string(),
            security_code: trade.security_code.clone(),
            security_name: security_display_name(display, &trade.security_code),
            quantity: trade.quantity,
            holding_days: position.map(|position| position.holding_days),
            change_pct: position.map(|position| position.unrealized_return),
            cost_price: position
                .map(|position| position.average_entry_price)
                .or(Some(trade.execution_price)),
            current_price: position.map(|position| position.close_price),
            contribution_pct: position
                .and_then(|position| contribution_pct(position.unrealized_pnl, total_equity)),
            reason: Some(trade.reason.clone()),
        });
    }

    for position in positions.iter().filter(|position| {
        !buy_codes.contains(&position.security_code)
            && !sell_codes.contains(&position.security_code)
    }) {
        rows.push(StrategyBacktestRebalanceRow {
            direction: "hold".to_string(),
            security_code: position.security_code.clone(),
            security_name: security_display_name(display, &position.security_code),
            quantity: position.quantity,
            holding_days: Some(position.holding_days),
            change_pct: Some(position.unrealized_return),
            cost_price: Some(position.average_entry_price),
            current_price: Some(position.close_price),
            contribution_pct: contribution_pct(position.unrealized_pnl, total_equity),
            reason: None,
        });
    }

    for closed in closed_trades {
        rows.push(StrategyBacktestRebalanceRow {
            direction: "sell".to_string(),
            security_code: closed.security_code.clone(),
            security_name: security_display_name(display, &closed.security_code),
            quantity: closed.quantity,
            holding_days: i32::try_from(closed.holding_days).ok(),
            change_pct: closed.realized_return,
            cost_price: Some(closed.entry_gross_amount / closed.quantity),
            current_price: Some(closed.exit_gross_amount / closed.quantity),
            contribution_pct: contribution_pct(closed.realized_pnl, total_equity),
            reason: Some(closed.exit_reason),
        });
    }

    rows
}

fn security_display_name(
    display: &BTreeMap<String, SecurityDisplayRow>,
    security_code: &str,
) -> Option<String> {
    display
        .get(security_code)
        .and_then(|row| row.security_name.clone())
}

fn contribution_pct(pnl: f64, total_equity: Option<f64>) -> Option<f64> {
    let total_equity = total_equity?;
    if total_equity == 0.0 {
        None
    } else {
        Some(pnl / total_equity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clickhouse::ScreeningRow;
    use crate::domain::{FilterExpr, ScoreClamp, ScoringSpec, UniverseSpec};
    use serde_json::json;

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

    #[test]
    fn strategy_preview_request_should_reject_zero_preview_row_limit() {
        let error = preview_request("2026-06-01", "2026-06-02", 0)
            .into_parts(90)
            .unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn strategy_preview_request_should_reject_inverted_range() {
        let error = preview_request("2026-06-03", "2026-06-02", 10)
            .into_parts(90)
            .unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn strategy_preview_request_should_reject_range_above_preview_limit() {
        let error = preview_request("2026-06-01", "2026-06-04", 10)
            .into_parts(3)
            .unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn preview_timeline_request_should_accept_near_one_year_range() {
        let request = preview_timeline_request("2025-06-01", "2026-06-01");

        let parts = request.into_parts().unwrap();

        assert_eq!(parts.start_date, date("2025-06-01"));
        assert_eq!(parts.end_date, date("2026-06-01"));
    }

    #[test]
    fn preview_timeline_request_should_reject_range_above_one_year_window() {
        let error = preview_timeline_request("2025-01-01", "2026-06-01")
            .into_parts()
            .unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn preview_pool_page_request_should_reject_non_score_sort() {
        let mut request = preview_pool_page_request();
        request.sort = Some("rank_asc".to_string());

        let error = request.into_parts().unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn preview_security_analysis_request_should_reject_empty_security_code() {
        let mut request = preview_security_analysis_request();
        request.security_code = " ".to_string();

        let error = request.into_parts().unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn preview_security_analysis_request_should_default_to_include_quote_rows() {
        let request = preview_security_analysis_request();

        let parts = request.into_parts().unwrap();

        assert!(parts.include_quote_rows);
    }

    #[test]
    fn preview_security_analysis_request_should_accept_quote_rows_omission() {
        let mut request = preview_security_analysis_request();
        request.include_quote_rows = Some(false);

        let parts = request.into_parts().unwrap();

        assert!(!parts.include_quote_rows);
    }

    #[test]
    fn build_strategy_preview_trade_dates_should_group_rows_and_keep_top_signals() {
        let trade_date = date("2026-06-02");
        let rows = vec![
            screening_row("000001.SZ", trade_date, 80.0, 3, 1, true),
            screening_row("000002.SZ", trade_date, 70.0, 3, 2, true),
        ];
        let trade_dates = build_strategy_preview_trade_dates(rows, 2, &BTreeMap::new()).unwrap();

        assert_eq!(trade_dates.len(), 1);
        assert_eq!(trade_dates[0].pool_count, 3);
        assert_eq!(trade_dates[0].signals.len(), 2);
        assert_eq!(trade_dates[0].signals[0].security_code, "000001.SZ");
        assert_eq!(trade_dates[0].signals[0].score_breakdown, json!({"w1": 80}));
    }

    #[test]
    fn build_strategy_preview_signal_should_include_security_board() {
        let trade_date = date("2026-06-02");
        let row = screening_row("000001.SZ", trade_date, 80.0, 3, 1, true);
        let display_by_code = BTreeMap::from([(
            "000001.SZ".to_string(),
            SecurityDisplayRow {
                security_code: "000001.SZ".to_string(),
                security_name: Some("平安银行".to_string()),
                exchange_code: Some("SZ".to_string()),
                security_board: Some("szse_main_board".to_string()),
            },
        )]);

        let signal = build_strategy_preview_signal(row, &display_by_code).unwrap();

        assert_eq!(signal.security_board.as_deref(), Some("szse_main_board"));
    }

    #[test]
    fn ma_values_should_return_forward_adjusted_values_for_any_chart_adjustment() {
        let trend = trend_row("000001.SZ", date("2026-06-02"));

        let values = ma_values(Some(&trend), &[5, 10, 30]);

        assert_eq!(values.get("5").copied().flatten(), Some(10.0));
        assert_eq!(values.get("10").copied().flatten(), Some(11.0));
        assert_eq!(values.get("30").copied().flatten(), Some(12.0));
    }

    fn preview_request(start_date: &str, end_date: &str, top_n: u32) -> StrategyPreviewRequest {
        StrategyPreviewRequest {
            rule: RuleVersionSpec {
                universe: UniverseSpec {
                    base: "all_a_shares".to_string(),
                    exclude_st: true,
                    exclude_suspend: true,
                    include_security_codes: Vec::new(),
                    exclude_security_codes: Vec::new(),
                },
                pool_filters: FilterExpr::All {
                    conditions: Vec::new(),
                },
                scoring: ScoringSpec {
                    rules: Vec::new(),
                    clamp: ScoreClamp {
                        min: 0.0,
                        max: 100.0,
                    },
                },
                top_n_default: 20,
                output_metrics: Vec::new(),
            },
            start_date: date(start_date),
            end_date: date(end_date),
            preview_row_limit: Some(top_n),
            top_n: None,
        }
    }

    fn preview_timeline_request(
        start_date: &str,
        end_date: &str,
    ) -> StrategyPreviewTimelineRequest {
        StrategyPreviewTimelineRequest {
            rule: preview_request(start_date, end_date, 10).rule,
            start_date: date(start_date),
            end_date: date(end_date),
        }
    }

    fn preview_pool_page_request() -> StrategyPreviewPoolPageRequest {
        StrategyPreviewPoolPageRequest {
            rule: preview_request("2026-06-01", "2026-06-01", 10).rule,
            trade_date: date("2026-06-01"),
            limit: Some(50),
            offset: Some(0),
            sort: Some("score_desc".to_string()),
            security_code: None,
        }
    }

    fn preview_security_analysis_request() -> StrategyPreviewSecurityAnalysisRequest {
        StrategyPreviewSecurityAnalysisRequest {
            rule: preview_request("2026-06-01", "2026-06-01", 10).rule,
            trade_date: date("2026-06-01"),
            security_code: "600000.SH".to_string(),
            adjustment: Some(Adjustment::ForwardAdjusted),
            quote_end_date: None,
            lookback_trading_days: Some(240),
            quote_start_date: None,
            ma_windows: None,
            include_quote_rows: None,
        }
    }

    fn screening_row(
        security_code: &str,
        trade_date: NaiveDate,
        score: f64,
        pool_count: usize,
        signal_rank: u32,
        is_buy_signal: bool,
    ) -> ScreeningRow {
        ScreeningRow {
            security_code: security_code.to_string(),
            trade_date,
            raw_score: score,
            score,
            signal_rank,
            pool_count: Some(pool_count),
            is_buy_signal,
            score_breakdown: r#"{"w1":80}"#.to_string(),
            selected_metrics: r#"{"close_price":10}"#.to_string(),
            raw_values: r#"{"close_price":10}"#.to_string(),
        }
    }

    fn trend_row(security_code: &str, trade_date: NaiveDate) -> TrendIndicatorRow {
        TrendIndicatorRow {
            security_code: security_code.to_string(),
            trade_date,
            price_ma_5: Some(10.0),
            price_ma_10: Some(11.0),
            price_ma_20: None,
            price_ma_30: Some(12.0),
            price_ma_60: None,
            price_ma_250: None,
            price_avg_ma_3_6_12_24: None,
            price_avg_ma_14_28_57_114: None,
            price_ema2_10: None,
            boll_mid_20_2: None,
            boll_up_20_2: None,
            boll_dn_20_2: None,
            macd_dif: None,
            macd_dea: None,
            macd_histogram: None,
        }
    }

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }
}
