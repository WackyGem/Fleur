use std::path::{Path, PathBuf};

use rearview_core::api;
use rearview_core::clickhouse::ClickHouseClient;
use rearview_core::config::{AppConfig, load_dotenv_if_present};
use rearview_core::nats::{
    PortfolioTaskMessage, StrategyBacktestTaskMessage, connect_jetstream, ensure_portfolio_stream,
    publish_portfolio_task, publish_strategy_backtest_task,
};
use rearview_core::postgres::RearviewPg;
use rearview_core::service::AppState;
use rearview_core::service::catalog::load_catalog_from_policy;
use rearview_core::{RearviewError, RearviewResult};
use tokio::net::TcpListener;
use tokio::time::{Duration, sleep};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

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
    tokio::spawn(async move {
        run_outbox_dispatcher(dispatcher_postgres, dispatcher_nats).await;
    });
    let state = AppState::new(config, postgres, catalog, clickhouse);
    let app = api::routes().with_state(state);
    let listener = TcpListener::bind(bind).await?;
    info!(%bind, "starting rearview HTTP service");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn run_outbox_dispatcher(
    postgres: RearviewPg,
    nats_config: rearview_core::config::NatsConfig,
) {
    loop {
        match dispatch_outbox_once(&postgres, &nats_config).await {
            Ok(dispatched) => {
                if dispatched == 0 {
                    sleep(Duration::from_secs(2)).await;
                }
            }
            Err(error) => {
                error!(error = %error, "outbox dispatcher iteration failed");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn dispatch_outbox_once(
    postgres: &RearviewPg,
    nats_config: &rearview_core::config::NatsConfig,
) -> RearviewResult<usize> {
    let jetstream = connect_jetstream(nats_config).await?;
    ensure_portfolio_stream(&jetstream, nats_config).await?;
    let mut dispatched = 0;
    let records = postgres.list_pending_portfolio_outbox(50).await?;
    for record in records {
        let message = PortfolioTaskMessage {
            portfolio_run_id: record.portfolio_run_id.clone(),
            source_run_id: record.source_run_id.clone(),
        };
        match publish_portfolio_task(&jetstream, nats_config, &message).await {
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
                dispatched += 1;
            }
            Err(error) => {
                postgres
                    .mark_portfolio_outbox_failed(
                        &record.outbox_id,
                        &record.portfolio_run_id,
                        &error.to_string(),
                    )
                    .await?;
            }
        }
    }
    let records = postgres.list_pending_strategy_backtest_outbox(50).await?;
    for record in records {
        let message = StrategyBacktestTaskMessage::new(record.strategy_backtest_run_id.clone());
        match publish_strategy_backtest_task(&jetstream, nats_config, &message).await {
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
                dispatched += 1;
            }
            Err(error) => {
                postgres
                    .mark_strategy_backtest_outbox_failed(
                        &record.outbox_id,
                        &record.strategy_backtest_run_id,
                        &error.to_string(),
                    )
                    .await?;
            }
        }
    }
    Ok(dispatched)
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
        "could not find mono-fleur repo root from {}",
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
        "rearview-server\n\nUSAGE:\n  rearview-server serve\n  rearview-server catalog check\n  rearview-server catalog coverage\n  rearview-server catalog sync\n  rearview-server sample-rule\n\nENV:\n  REARVIEW_DATABASE_URL\n  REARVIEW_HTTP_BIND\n  CLICKHOUSE_HOST / CLICKHOUSE_PORT / CLICKHOUSE_USER / CLICKHOUSE_PASSWORD"
    );
}
