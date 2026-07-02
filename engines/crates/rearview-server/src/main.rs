use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_nats::jetstream;
use chrono::{DateTime, Days, Months, NaiveDate, Utc};
use rearview_core::api;
use rearview_core::clickhouse::ClickHouseClient;
use rearview_core::config::{AppConfig, NatsConfig, load_dotenv_if_present};
use rearview_core::domain::representative_rule;
use rearview_core::nats::{
    PortfolioTaskMessage, StrategyBacktestTaskMessage, StrategyPortfolioDailyRunTaskMessage,
    connect_jetstream, ensure_portfolio_stream, publish_portfolio_task,
    publish_strategy_backtest_task, publish_strategy_portfolio_daily_run_task,
};
use rearview_core::planner::{QueryPlanner, QuerySettings};
use rearview_core::postgres::{NewStrategyPortfolio, NewSucceededStrategyBacktestSeed, RearviewPg};
use rearview_core::service::AppState;
use rearview_core::service::catalog::load_catalog_from_policy;
use rearview_core::strategy_backtest::{
    BacktestAccountConfig, BacktestDateRange, BacktestExecutionConfig, BacktestFeeProfile,
    BacktestRebalancePolicy, BacktestRiskExitPolicy, BacktestSignalPolicy, BacktestSlippageProfile,
    ExitRuleConfig, StrategyBacktestValidateRequest,
};
use rearview_core::strategy_portfolio::new_portfolio_code;
use rearview_core::{RearviewError, RearviewResult};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

const OUTBOX_IDLE_SLEEP: Duration = Duration::from_secs(2);
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> RearviewResult<()> {
    init_tracing();
    load_local_dotenv()?;

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("serve") | None => serve().await,
        Some("catalog") => match args.next().as_deref() {
            Some("check") => catalog_check(),
            Some("coverage") => catalog_coverage(),
            Some("sync") => catalog_sync().await,
            Some(other) => Err(RearviewError::Config(format!(
                "unknown catalog command: {other}"
            ))),
            None => Err(RearviewError::Config(
                "missing catalog command; expected check, coverage, or sync".to_string(),
            )),
        },
        Some("dev") => match args.next().as_deref() {
            Some("seed-statement-portfolio") => seed_statement_portfolio().await,
            Some(other) => Err(RearviewError::Config(format!(
                "unknown dev command: {other}"
            ))),
            None => Err(RearviewError::Config(
                "missing dev command; expected seed-statement-portfolio".to_string(),
            )),
        },
        Some("sample-rule") => sample_rule(),
        Some("--version" | "-V") => {
            println!("rearview-server {VERSION}");
            Ok(())
        }
        Some("--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some(other) => Err(RearviewError::Config(format!("unknown command: {other}"))),
    }
}

async fn serve() -> RearviewResult<()> {
    let config = AppConfig::from_env()?;
    let bind = config.http_bind;
    let (catalog, _) = load_default_catalog()?;
    let postgres = RearviewPg::connect(&config.rearview_database_url).await?;
    postgres.check_schema_readiness().await?;
    let clickhouse = ClickHouseClient::new(config.clickhouse.clone())?;
    clickhouse.check_readiness().await?;
    let dispatcher_postgres = postgres.clone();
    let dispatcher_nats = config.nats.clone();
    let outbox_notifier = Arc::new(Notify::new());
    let dispatcher_notifier = Arc::clone(&outbox_notifier);
    tokio::spawn(async move {
        run_outbox_dispatcher(dispatcher_postgres, dispatcher_nats, dispatcher_notifier).await;
    });
    let state = AppState::new_with_service_identity(
        config,
        postgres,
        catalog,
        clickhouse,
        outbox_notifier,
        "rearview-server",
        VERSION,
    );
    let app = api::routes().with_state(state);
    let listener = TcpListener::bind(bind).await?;
    info!(%bind, "starting rearview HTTP service");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn run_outbox_dispatcher(
    postgres: RearviewPg,
    nats_config: NatsConfig,
    outbox_notifier: Arc<Notify>,
) {
    let mut jetstream = loop {
        match connect_outbox_jetstream(&nats_config).await {
            Ok(jetstream) => break jetstream,
            Err(error) => {
                error!(error = %error, "outbox dispatcher failed to connect to nats");
                sleep(Duration::from_secs(5)).await;
            }
        }
    };

    loop {
        match dispatch_outbox_once(&postgres, &nats_config, &jetstream).await {
            Ok(dispatched) => {
                if dispatched == 0 {
                    let wake_reason =
                        wait_for_outbox_signal(&outbox_notifier, OUTBOX_IDLE_SLEEP).await;
                    debug!(
                        wake_reason = ?wake_reason,
                        "outbox dispatcher idle wait completed"
                    );
                }
            }
            Err(error) => {
                let should_reconnect = matches!(error, RearviewError::Nats(_));
                error!(error = %error, "outbox dispatcher iteration failed");
                if should_reconnect {
                    jetstream = loop {
                        match connect_outbox_jetstream(&nats_config).await {
                            Ok(jetstream) => break jetstream,
                            Err(error) => {
                                error!(
                                    error = %error,
                                    "outbox dispatcher failed to reconnect to nats"
                                );
                                sleep(Duration::from_secs(5)).await;
                            }
                        }
                    };
                }
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn connect_outbox_jetstream(nats_config: &NatsConfig) -> RearviewResult<jetstream::Context> {
    let jetstream = connect_jetstream(nats_config).await?;
    ensure_portfolio_stream(&jetstream, nats_config).await?;
    Ok(jetstream)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutboxWakeReason {
    Notified,
    IdleTimeout,
}

async fn wait_for_outbox_signal(notifier: &Notify, idle_sleep: Duration) -> OutboxWakeReason {
    tokio::select! {
        () = notifier.notified() => OutboxWakeReason::Notified,
        () = sleep(idle_sleep) => OutboxWakeReason::IdleTimeout,
    }
}

async fn dispatch_outbox_once(
    postgres: &RearviewPg,
    nats_config: &NatsConfig,
    jetstream: &jetstream::Context,
) -> RearviewResult<usize> {
    let mut dispatched = 0;
    let records = postgres.list_pending_portfolio_outbox(50).await?;
    log_outbox_scan("portfolio", records.len());
    for record in records {
        let message = PortfolioTaskMessage {
            portfolio_run_id: record.portfolio_run_id.clone(),
            source_run_id: record.source_run_id.clone(),
        };
        match publish_portfolio_task(jetstream, nats_config, &message).await {
            Ok(sequence) => {
                postgres
                    .mark_portfolio_outbox_published(
                        &record.outbox_id,
                        &record.portfolio_run_id,
                        i64::try_from(sequence).map_err(|error| {
                            RearviewError::Nats(format!("stream sequence out of range: {error}"))
                        })?,
                    )
                    .await?;
                info!(
                    outbox_kind = "portfolio",
                    outbox_id = %record.outbox_id,
                    portfolio_run_id = %record.portfolio_run_id,
                    attempt_count = record.attempt_count,
                    nats_stream_sequence = sequence,
                    publish_elapsed_ms = elapsed_ms_since(record.created_at),
                    "outbox task published"
                );
                dispatched += 1;
            }
            Err(error) => {
                let error_message = error.to_string();
                postgres
                    .mark_portfolio_outbox_failed(
                        &record.outbox_id,
                        &record.portfolio_run_id,
                        &error_message,
                    )
                    .await?;
                error!(
                    outbox_kind = "portfolio",
                    outbox_id = %record.outbox_id,
                    portfolio_run_id = %record.portfolio_run_id,
                    attempt_count = record.attempt_count,
                    publish_elapsed_ms = elapsed_ms_since(record.created_at),
                    error = %error_message,
                    "outbox task publish failed"
                );
            }
        }
    }
    let records = postgres.list_pending_strategy_backtest_outbox(50).await?;
    log_outbox_scan("strategy_backtest", records.len());
    for record in records {
        let message = StrategyBacktestTaskMessage::new(record.strategy_backtest_run_id.clone());
        match publish_strategy_backtest_task(jetstream, nats_config, &message).await {
            Ok(sequence) => {
                postgres
                    .mark_strategy_backtest_outbox_published(
                        &record.outbox_id,
                        &record.strategy_backtest_run_id,
                        i64::try_from(sequence).map_err(|error| {
                            RearviewError::Nats(format!("stream sequence out of range: {error}"))
                        })?,
                    )
                    .await?;
                info!(
                    outbox_kind = "strategy_backtest",
                    outbox_id = %record.outbox_id,
                    strategy_backtest_run_id = %record.strategy_backtest_run_id,
                    attempt_count = record.attempt_count,
                    nats_stream_sequence = sequence,
                    publish_elapsed_ms = elapsed_ms_since(record.created_at),
                    "outbox task published"
                );
                dispatched += 1;
            }
            Err(error) => {
                let error_message = error.to_string();
                postgres
                    .mark_strategy_backtest_outbox_failed(
                        &record.outbox_id,
                        &record.strategy_backtest_run_id,
                        &error_message,
                    )
                    .await?;
                error!(
                    outbox_kind = "strategy_backtest",
                    outbox_id = %record.outbox_id,
                    strategy_backtest_run_id = %record.strategy_backtest_run_id,
                    attempt_count = record.attempt_count,
                    publish_elapsed_ms = elapsed_ms_since(record.created_at),
                    error = %error_message,
                    "outbox task publish failed"
                );
            }
        }
    }
    let records = postgres
        .list_pending_strategy_portfolio_daily_outbox(50)
        .await?;
    log_outbox_scan("strategy_portfolio_daily", records.len());
    for record in records {
        let message = StrategyPortfolioDailyRunTaskMessage::new(
            record.strategy_portfolio_daily_run_id.clone(),
        );
        match publish_strategy_portfolio_daily_run_task(jetstream, nats_config, &message).await {
            Ok(sequence) => {
                postgres
                    .mark_strategy_portfolio_daily_outbox_published(
                        &record.outbox_id,
                        &record.strategy_portfolio_daily_run_id,
                        i64::try_from(sequence).map_err(|error| {
                            RearviewError::Nats(format!("stream sequence out of range: {error}"))
                        })?,
                    )
                    .await?;
                info!(
                    outbox_kind = "strategy_portfolio_daily",
                    outbox_id = %record.outbox_id,
                    strategy_portfolio_daily_run_id = %record.strategy_portfolio_daily_run_id,
                    attempt_count = record.attempt_count,
                    nats_stream_sequence = sequence,
                    publish_elapsed_ms = elapsed_ms_since(record.created_at),
                    "outbox task published"
                );
                dispatched += 1;
            }
            Err(error) => {
                let error_message = error.to_string();
                postgres
                    .mark_strategy_portfolio_daily_outbox_failed(
                        &record.outbox_id,
                        &record.strategy_portfolio_daily_run_id,
                        &error_message,
                    )
                    .await?;
                error!(
                    outbox_kind = "strategy_portfolio_daily",
                    outbox_id = %record.outbox_id,
                    strategy_portfolio_daily_run_id = %record.strategy_portfolio_daily_run_id,
                    attempt_count = record.attempt_count,
                    publish_elapsed_ms = elapsed_ms_since(record.created_at),
                    error = %error_message,
                    "outbox task publish failed"
                );
            }
        }
    }
    Ok(dispatched)
}

fn log_outbox_scan(outbox_kind: &'static str, pending_batch_size: usize) {
    if pending_batch_size == 0 {
        debug!(outbox_kind, pending_batch_size, "outbox pending scan");
    } else {
        info!(outbox_kind, pending_batch_size, "outbox pending scan");
    }
}

fn elapsed_ms_since(created_at: DateTime<Utc>) -> i64 {
    Utc::now()
        .signed_duration_since(created_at)
        .num_milliseconds()
}

async fn seed_statement_portfolio() -> RearviewResult<()> {
    const SOURCE_CLIENT_REQUEST_ID: &str =
        "dev-statement-source-backtest-first-signal-after-2025-01-02-v1";
    const PORTFOLIO_CLIENT_REQUEST_ID: &str =
        "dev-statement-portfolio-first-signal-after-2025-01-02-v1";
    const SOURCE_RESULT_ATTEMPT_ID: &str = "dev-statement-source-attempt-first-signal-v1";
    const BENCHMARK_SECURITY_CODE: &str = "000300.SH";
    const SOURCE_PERIOD_KEY: &str = "1y";

    let signal_search_start_date = date_ymd(2025, 1, 2)?;
    let signal_search_end_date = date_ymd(2025, 12, 31)?;

    let config = AppConfig::from_env()?;
    let (catalog, metric_count) = load_default_catalog()?;
    let postgres = RearviewPg::connect(&config.rearview_database_url).await?;
    postgres.check_schema_readiness().await?;
    let clickhouse = ClickHouseClient::new(config.clickhouse.clone())?;
    clickhouse.check_readiness().await?;

    let rule = representative_rule();
    let execution_config = default_statement_seed_execution_config().canonicalized()?;
    let top_n = execution_config.signal_policy.buy_signal_top_n;
    let query_settings = QuerySettings {
        max_execution_time_seconds: config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: config.clickhouse.max_rows_to_read,
        max_bytes_to_read: config.clickhouse.max_bytes_to_read,
    };
    let compiled = QueryPlanner::new(catalog.clone()).compile_backtest_signals(
        &rule,
        signal_search_start_date,
        signal_search_end_date,
        top_n,
        query_settings,
    )?;
    let signal_rows = clickhouse
        .query_backtest_signal_rows(
            &compiled.sql,
            "rearview-dev-seed-statement-portfolio-signals",
        )
        .await?;
    let Some(source_signal_date) = signal_rows.iter().map(|row| row.trade_date).min() else {
        return Err(RearviewError::Validation(format!(
            "dev statement seed found no buy signals between {signal_search_start_date} and {signal_search_end_date}"
        )));
    };
    let live_start_date = resolve_next_trade_date(&clickhouse, source_signal_date).await?;
    let source_start_date = source_signal_date
        .checked_sub_months(Months::new(12))
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "could not resolve one-year source start before {source_signal_date}"
            ))
        })?;
    let draft = StrategyBacktestValidateRequest {
        rule: rule.clone(),
        preview_id: None,
        preview_range: None,
        execution_config,
        range: Some(BacktestDateRange {
            start_date: source_start_date,
            end_date: source_signal_date,
        }),
        benchmark: Some(BENCHMARK_SECURITY_CODE.to_string()),
    }
    .validate(&catalog)?;
    let signal_rows = signal_rows
        .into_iter()
        .filter(|row| row.trade_date == source_signal_date)
        .collect::<Vec<_>>();
    if signal_rows.is_empty() {
        return Err(RearviewError::Validation(format!(
            "dev statement seed found no buy signals on {source_signal_date}"
        )));
    }

    let required_metrics = json!(compiled.required_metrics);
    let required_marts = json!(compiled.required_marts);
    let pending_buy_signals = signal_rows
        .iter()
        .map(|row| {
            json!({
                "security_code": row.security_code,
                "security_name": null,
                "source_rank": row.signal_rank,
                "source_score": row.score,
                "signal_date": source_signal_date,
                "execution_date": live_start_date
            })
        })
        .collect::<Vec<_>>();
    let pending_buy_signal_snapshot = serde_json::to_value(&pending_buy_signals)?;
    let source_request_hash = format!(
        "dev-statement-source-v1:{}:{}:{}:{}",
        draft.rule_hash, draft.execution_config_hash, source_start_date, source_signal_date
    );
    let source_run = match postgres
        .get_strategy_backtest_run_by_client_request_id(SOURCE_CLIENT_REQUEST_ID)
        .await?
    {
        Some(existing) => {
            if existing.request_hash != source_request_hash {
                return Err(RearviewError::Conflict(format!(
                    "client_request_id {SOURCE_CLIENT_REQUEST_ID} already exists with a different request_hash"
                )));
            }
            existing
        }
        None => {
            postgres
                .create_succeeded_strategy_backtest_seed(NewSucceededStrategyBacktestSeed {
                    rule_snapshot: serde_json::to_value(&rule)?,
                    rule_hash: draft.rule_hash.clone(),
                    execution_config: serde_json::to_value(&draft.execution_config)?,
                    execution_config_hash: draft.execution_config_hash.clone(),
                    catalog_hash: Some(format!("dev-seed-policy-{metric_count}-metrics")),
                    compiled_sql_hash: compiled.sql_hash.clone(),
                    required_metrics: required_metrics.clone(),
                    required_marts: required_marts.clone(),
                    data_preflight_snapshot: json!({
                        "kind": "dev_statement_seed",
                        "signal_search_start_date": signal_search_start_date,
                        "signal_search_end_date": signal_search_end_date,
                        "source_signal_date": source_signal_date,
                        "live_start_date": live_start_date,
                        "note": "control-plane seed only; live facts must be produced by strategy portfolio daily runs"
                    }),
                    period_key: SOURCE_PERIOD_KEY.to_string(),
                    range_as_of_date: Some(source_signal_date),
                    range_resolution_snapshot: json!({
                        "kind": "dev_statement_seed",
                        "as_of_date": source_signal_date,
                        "period_options": [{
                            "period_key": SOURCE_PERIOD_KEY,
                            "resolved_start_date": source_start_date,
                            "resolved_end_date": source_signal_date,
                            "benchmark_security_code": BENCHMARK_SECURITY_CODE
                        }]
                    }),
                    start_date: source_start_date,
                    end_date: source_signal_date,
                    benchmark_security_code: BENCHMARK_SECURITY_CODE.to_string(),
                    ui_display_snapshot: json!({
                        "kind": "dev_statement_seed",
                        "source": "rearview-server dev seed-statement-portfolio"
                    }),
                    client_request_id: Some(SOURCE_CLIENT_REQUEST_ID.to_string()),
                    request_hash: source_request_hash.clone(),
                    current_result_attempt_id: SOURCE_RESULT_ATTEMPT_ID.to_string(),
                    progress: json!({"stage": "dev_seed", "completed": true}),
                    summary: json!({
                        "kind": "dev_statement_seed",
                        "signal_row_count": signal_rows.len(),
                        "live_facts_written": false
                    }),
                    signal_summary: json!({
                        "signal_date": source_signal_date,
                        "signal_row_count": signal_rows.len(),
                        "top_n": top_n
                    }),
                    data_coverage_summary: json!({
                        "source_signal_date": source_signal_date,
                        "required_marts": required_marts
                    }),
                })
                .await?
        }
    };

    let portfolio_request_hash = format!(
        "dev-statement-portfolio-v1:{}:{}:{}:{}",
        source_run.strategy_backtest_run_id,
        SOURCE_RESULT_ATTEMPT_ID,
        source_signal_date,
        live_start_date
    );
    let portfolio = match postgres
        .get_strategy_portfolio_by_client_request_id(PORTFOLIO_CLIENT_REQUEST_ID)
        .await?
    {
        Some(existing) => {
            if existing.request_hash != portfolio_request_hash {
                return Err(RearviewError::Conflict(format!(
                    "client_request_id {PORTFOLIO_CLIENT_REQUEST_ID} already exists with a different request_hash"
                )));
            }
            existing
        }
        None => {
            let mut last_error = None;
            let mut created = None;
            for _ in 0..5 {
                match postgres
                    .create_strategy_portfolio(NewStrategyPortfolio {
                        portfolio_code: new_portfolio_code(Utc::now()),
                        name: "Plan 0062 Statement Acceptance Seed".to_string(),
                        rule_snapshot: source_run.rule_snapshot.clone(),
                        rule_hash: source_run.rule_hash.clone(),
                        execution_config: source_run.execution_config.clone(),
                        execution_config_hash: source_run.execution_config_hash.clone(),
                        benchmark_security_code: source_run.benchmark_security_code.clone(),
                        catalog_hash: source_run.catalog_hash.clone(),
                        required_metrics: source_run.required_metrics.clone(),
                        required_marts: source_run.required_marts.clone(),
                        source_strategy_backtest_run_id: source_run
                            .strategy_backtest_run_id
                            .clone(),
                        source_result_attempt_id: SOURCE_RESULT_ATTEMPT_ID.to_string(),
                        source_period_key: source_run.period_key.clone(),
                        source_start_date: source_run.start_date,
                        source_end_date: source_run.end_date,
                        initial_signal_date: source_signal_date,
                        live_start_date,
                        pending_buy_signal_snapshot: pending_buy_signal_snapshot.clone(),
                        ui_display_snapshot: json!({
                            "kind": "dev_statement_seed",
                            "source": "rearview-server dev seed-statement-portfolio"
                        }),
                        client_request_id: Some(PORTFOLIO_CLIENT_REQUEST_ID.to_string()),
                        request_hash: portfolio_request_hash.clone(),
                        source_kind: "backtest_publish".to_string(),
                        example_case_id: None,
                        example_version: None,
                        fixture_hash: None,
                    })
                    .await
                {
                    Ok(record) => {
                        created = Some(record);
                        break;
                    }
                    Err(error) => {
                        last_error = Some(error);
                    }
                }
            }
            created.ok_or_else(|| {
                last_error.unwrap_or_else(|| {
                    RearviewError::Conflict(
                        "could not create dev statement portfolio seed".to_string(),
                    )
                })
            })?
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "source_strategy_backtest_run_id": source_run.strategy_backtest_run_id,
            "source_result_attempt_id": SOURCE_RESULT_ATTEMPT_ID,
            "strategy_portfolio_id": portfolio.strategy_portfolio_id,
            "signal_search_start_date": signal_search_start_date,
            "signal_search_end_date": signal_search_end_date,
            "initial_signal_date": portfolio.initial_signal_date,
            "live_start_date": portfolio.live_start_date,
            "pending_buy_signal_count": pending_buy_signals.len(),
            "required_marts": portfolio.required_marts,
            "live_facts_written_by_seed": false
        }))?
    );
    Ok(())
}

async fn resolve_next_trade_date(
    clickhouse: &ClickHouseClient,
    source_signal_date: NaiveDate,
) -> RearviewResult<NaiveDate> {
    let start_date = source_signal_date
        .checked_add_days(Days::new(1))
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "could not resolve next date after signal date {source_signal_date}"
            ))
        })?;
    let end_date = source_signal_date
        .checked_add_days(Days::new(45))
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "could not resolve trade-date search window after signal date {source_signal_date}"
            ))
        })?;
    let mut trade_dates = clickhouse
        .query_trade_calendar_dates(
            start_date,
            end_date,
            &format!("rearview-dev-seed-statement-next-trade-date-{source_signal_date}"),
        )
        .await?;
    trade_dates.sort_unstable();
    trade_dates
        .into_iter()
        .find(|trade_date| *trade_date > source_signal_date)
        .ok_or_else(|| {
            RearviewError::Validation(format!(
                "could not resolve next trading date after signal date {source_signal_date}"
            ))
        })
}

fn default_statement_seed_execution_config() -> BacktestExecutionConfig {
    BacktestExecutionConfig {
        market: "CN_A_SHARE".to_string(),
        account: BacktestAccountConfig {
            initial_cash: 1_000_000.0,
            currency: "CNY".to_string(),
        },
        signal_policy: BacktestSignalPolicy {
            buy_signal_top_n: 5,
            signal_timing: "close_confirm_next_open".to_string(),
        },
        rebalance_policy: BacktestRebalancePolicy {
            target_weighting: "equal_weight_capped".to_string(),
            max_positions: 5,
            single_position_limit_pct: Some(0.10),
            cash_reserve_pct: 0.0,
            lot_size: 100,
            min_trade_lots: 1,
            empty_signal_action: "hold".to_string(),
        },
        fee_profile: BacktestFeeProfile {
            commission_rate: 0.0001,
            commission_rate_max: 0.003,
            min_commission: 5.0,
            stamp_duty_rate_sell: 0.0005,
            transfer_fee_rate: 0.00001,
        },
        slippage_profile: BacktestSlippageProfile {
            mode: "bps".to_string(),
            buy_bps: 10.0,
            sell_bps: 10.0,
        },
        risk_exit_policy: BacktestRiskExitPolicy {
            trigger_timing: "close_confirm_next_open".to_string(),
            exit_rules: vec![
                ExitRuleConfig::FixedStopLoss { loss_pct: 0.08 },
                ExitRuleConfig::TakeProfit { profit_pct: 0.20 },
                ExitRuleConfig::TimeStopLoss {
                    holding_days: 20,
                    max_return_pct: 0.0,
                },
                ExitRuleConfig::IndicatorStopLoss {
                    source: "trend".to_string(),
                    metric: "price_ma_10".to_string(),
                    operator: "close_below_metric".to_string(),
                },
            ],
        },
        price_basis: "backward_adjusted".to_string(),
    }
}

fn date_ymd(year: i32, month: u32, day: u32) -> RearviewResult<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        RearviewError::Config(format!(
            "invalid built-in dev seed date: {year}-{month}-{day}"
        ))
    })
}

fn sample_rule() -> RearviewResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&rearview_core::domain::representative_rule())?
    );
    Ok(())
}

fn catalog_check() -> RearviewResult<()> {
    let (catalog, _) = load_default_catalog()?;
    println!(
        "metric catalog check passed: {} metrics",
        catalog.iter().count()
    );
    Ok(())
}

fn catalog_coverage() -> RearviewResult<()> {
    let repo_root = find_repo_root()?;
    let policy_path = repo_root.join("engines/crates/rearview-core/config/metric_policy.yml");
    let dbt_marts_dir = repo_root.join("pipeline/elt/models/marts");
    let policy = rearview_core::domain::MetricPolicyFile::load(policy_path)?;
    let checked = policy.check_coverage(dbt_marts_dir)?;
    println!("metric catalog coverage passed: {checked} dbt fields checked");
    Ok(())
}

async fn catalog_sync() -> RearviewResult<()> {
    let config = AppConfig::from_env()?;
    let (catalog, metric_count) = load_default_catalog()?;
    let postgres = RearviewPg::connect(&config.rearview_database_url).await?;
    postgres.check_schema_readiness().await?;
    let written = postgres.sync_metric_catalog(&catalog).await?;
    println!("metric catalog sync completed: {metric_count} metrics, {written} rows affected");
    Ok(())
}

fn load_default_catalog() -> RearviewResult<(rearview_core::domain::MetricCatalog, usize)> {
    let repo_root = find_repo_root()?;
    let policy_path = repo_root.join("engines/crates/rearview-core/config/metric_policy.yml");
    let dbt_marts_dir = repo_root.join("pipeline/elt/models/marts");
    let marts_database = std::env::var("REARVIEW_CLICKHOUSE_MARTS_DATABASE")
        .unwrap_or_else(|_| "fleur_marts".to_string());
    let catalog = load_catalog_from_policy(policy_path, dbt_marts_dir, &marts_database)?;
    let metric_count = catalog.iter().count();
    Ok((catalog, metric_count))
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn load_local_dotenv() -> RearviewResult<()> {
    let repo_root = find_repo_root()?;
    let candidate = repo_root.join(".env");
    if candidate.exists() {
        load_dotenv_if_present(candidate)?;
    }
    Ok(())
}

fn find_repo_root() -> RearviewResult<PathBuf> {
    let current_dir = std::env::current_dir()?;
    for ancestor in current_dir.ancestors() {
        if is_repo_root(ancestor) {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err(RearviewError::Config(format!(
        "could not find fleur repo root from {}",
        current_dir.display()
    )))
}

fn is_repo_root(path: &Path) -> bool {
    path.join(".env.example").is_file()
        && path.join("engines/Cargo.toml").is_file()
        && path.join("pipeline/pyproject.toml").is_file()
}

fn print_help() {
    println!(
        "rearview-server\n\nUSAGE:\n  rearview-server serve\n  rearview-server catalog check\n  rearview-server catalog coverage\n  rearview-server catalog sync\n  rearview-server dev seed-statement-portfolio\n  rearview-server sample-rule\n  rearview-server --version\n\nENV:\n  REARVIEW_DATABASE_URL\n  REARVIEW_HTTP_BIND\n  CLICKHOUSE_HOST / CLICKHOUSE_PORT / CLICKHOUSE_USER / CLICKHOUSE_PASSWORD"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_for_outbox_signal_should_return_notified_when_signal_is_available() {
        let notifier = Notify::new();
        notifier.notify_one();

        let reason = wait_for_outbox_signal(&notifier, Duration::from_secs(60)).await;

        assert_eq!(reason, OutboxWakeReason::Notified);
    }

    #[tokio::test]
    async fn wait_for_outbox_signal_should_return_timeout_without_signal() {
        let notifier = Notify::new();

        let reason = wait_for_outbox_signal(&notifier, Duration::from_millis(1)).await;

        assert_eq!(reason, OutboxWakeReason::IdleTimeout);
    }
}
