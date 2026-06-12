use std::path::{Path, PathBuf};

use rearview::api;
use rearview::clickhouse::ClickHouseClient;
use rearview::config::{AppConfig, load_dotenv_if_present};
use rearview::postgres::RearviewPg;
use rearview::service::AppState;
use rearview::service::catalog::load_catalog_from_policy;
use rearview::{RearviewError, RearviewResult};
use tokio::net::TcpListener;
use tracing::info;
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
            Some("sync") => catalog_sync().await,
            Some(other) => Err(RearviewError::Config(format!(
                "unknown catalog command: {other}"
            ))),
            None => Err(RearviewError::Config(
                "missing catalog command; expected check or sync".to_string(),
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
    let state = AppState::new(config, postgres, catalog, clickhouse);
    let app = api::routes().with_state(state);
    let listener = TcpListener::bind(bind).await?;
    info!(%bind, "starting rearview HTTP service");
    axum::serve(listener, app).await?;
    Ok(())
}

fn sample_rule() -> RearviewResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&rearview::domain::representative_rule())?
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

async fn catalog_sync() -> RearviewResult<()> {
    let config = AppConfig::from_env()?;
    let (catalog, metric_count) = load_default_catalog()?;
    let postgres = RearviewPg::connect(&config.rearview_database_url).await?;
    postgres.check_schema_readiness().await?;
    let written = postgres.sync_metric_catalog(&catalog).await?;
    println!("metric catalog sync completed: {metric_count} metrics, {written} rows affected");
    Ok(())
}

fn load_default_catalog() -> RearviewResult<(rearview::domain::MetricCatalog, usize)> {
    let repo_root = find_repo_root()?;
    let policy_path = repo_root.join("engines/crates/rearview/config/metric_policy.yml");
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
        "rearview\n\nUSAGE:\n  rearview serve\n  rearview catalog check\n  rearview catalog sync\n  rearview sample-rule\n\nENV:\n  REARVIEW_DATABASE_URL\n  REARVIEW_HTTP_BIND\n  CLICKHOUSE_HOST / CLICKHOUSE_PORT / CLICKHOUSE_USER / CLICKHOUSE_PASSWORD"
    );
}
