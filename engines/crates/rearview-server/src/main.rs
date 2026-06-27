use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_nats::jetstream;
use chrono::{DateTime, Utc};
use rearview_core::api;
use rearview_core::clickhouse::ClickHouseClient;
use rearview_core::config::{AppConfig, NatsConfig, load_dotenv_if_present};
use rearview_core::nats::{
    PortfolioTaskMessage, StrategyBacktestTaskMessage, StrategyPortfolioDailyRunTaskMessage,
    connect_jetstream, ensure_portfolio_stream, publish_portfolio_task,
    publish_strategy_backtest_task, publish_strategy_portfolio_daily_run_task,
};
use rearview_core::postgres::RearviewPg;
use rearview_core::service::AppState;
use rearview_core::service::catalog::load_catalog_from_policy;
use rearview_core::{RearviewError, RearviewResult};
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
        "rearview-server\n\nUSAGE:\n  rearview-server serve\n  rearview-server catalog check\n  rearview-server catalog coverage\n  rearview-server catalog sync\n  rearview-server sample-rule\n  rearview-server --version\n\nENV:\n  REARVIEW_DATABASE_URL\n  REARVIEW_HTTP_BIND\n  CLICKHOUSE_HOST / CLICKHOUSE_PORT / CLICKHOUSE_USER / CLICKHOUSE_PASSWORD"
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
