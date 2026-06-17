use chrono::{Days, NaiveDate};
use futures_util::StreamExt;
use rearview_core::clickhouse::ClickHouseClient;
use rearview_core::config::{AppConfig, load_dotenv_if_present};
use rearview_core::nats::{
    PortfolioTaskMessage, connect_jetstream, ensure_portfolio_consumer, ensure_portfolio_stream,
};
use rearview_core::portfolio::{
    ExitRule, FeeProfile, PortfolioSimulationInput, SlippageProfile, simulate_portfolio,
};
use rearview_core::postgres::{PortfolioRunRecord, RearviewPg};
use rearview_core::{RearviewError, RearviewResult};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> RearviewResult<()> {
    init_tracing();
    load_local_dotenv()?;

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("run") | None => run().await,
        Some("--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some(other) => Err(RearviewError::Config(format!(
            "unknown worker command: {other}"
        ))),
    }
}

async fn run() -> RearviewResult<()> {
    let once = std::env::args().any(|arg| arg == "--once");
    let config = AppConfig::from_env()?;
    let postgres = RearviewPg::connect(&config.rearview_database_url).await?;
    let clickhouse = ClickHouseClient::new(config.clickhouse.clone())?;
    clickhouse.ensure_portfolio_schema().await?;
    let jetstream = connect_jetstream(&config.nats).await?;
    let stream = ensure_portfolio_stream(&jetstream, &config.nats).await?;
    let consumer = ensure_portfolio_consumer(&stream, &config.nats).await?;
    info!(
        database_url_configured = !config.rearview_database_url.is_empty(),
        "starting rearview portfolio worker"
    );
    let mut messages = consumer
        .messages()
        .await
        .map_err(|error| RearviewError::Nats(error.to_string()))?;
    while let Some(message) = messages.next().await {
        let message = message.map_err(|error| RearviewError::Nats(error.to_string()))?;
        let payload = serde_json::from_slice::<PortfolioTaskMessage>(&message.payload)?;
        let run = postgres
            .get_portfolio_run(&payload.portfolio_run_id)
            .await?;
        if is_terminal_status(&run.status) {
            message
                .ack()
                .await
                .map_err(|error| RearviewError::Nats(error.to_string()))?;
            if once {
                return Ok(());
            }
            continue;
        }
        let Some(claimed_run) = postgres
            .claim_portfolio_run_for_calculation(&payload.portfolio_run_id)
            .await?
        else {
            message
                .ack()
                .await
                .map_err(|error| RearviewError::Nats(error.to_string()))?;
            if once {
                return Ok(());
            }
            continue;
        };
        match process_run(&postgres, &clickhouse, &claimed_run).await {
            Ok(()) => {
                message
                    .ack()
                    .await
                    .map_err(|error| RearviewError::Nats(error.to_string()))?;
            }
            Err(error) => {
                error!(
                    portfolio_run_id = claimed_run.portfolio_run_id,
                    error = %error,
                    "portfolio run failed"
                );
                postgres
                    .set_portfolio_run_status(
                        &claimed_run.portfolio_run_id,
                        portfolio_failure_status(&error),
                        Some(&error),
                    )
                    .await?;
                message
                    .ack()
                    .await
                    .map_err(|error| RearviewError::Nats(error.to_string()))?;
            }
        }
        if once {
            return Ok(());
        }
    }
    Ok(())
}

async fn process_run(
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    run: &PortfolioRunRecord,
) -> RearviewResult<()> {
    let input = build_simulation_input(postgres, clickhouse, run).await?;
    let output = simulate_portfolio(&input)?;

    let result_attempt_id = ulid::Ulid::new().to_string();

    // Write results to ClickHouse. Write failures must be "failed_write",
    // not "failed_market_data" (which is for read failures), so we handle
    // the error here rather than relying on portfolio_failure_status.
    if let Err(error) = clickhouse
        .write_portfolio_results(run, &result_attempt_id, &output)
        .await
    {
        error!(
            portfolio_run_id = run.portfolio_run_id,
            result_attempt_id = result_attempt_id,
            error = %error,
            "clickhouse write failed"
        );
        postgres
            .set_portfolio_run_status(&run.portfolio_run_id, "failed_write", Some(&error))
            .await?;
        return Ok(());
    }

    postgres
        .finalize_portfolio_run_to_clickhouse(
            &run.portfolio_run_id,
            &result_attempt_id,
            &output.summary,
        )
        .await?;

    info!(
        portfolio_run_id = run.portfolio_run_id,
        result_attempt_id = result_attempt_id,
        nav_points = output.nav.len(),
        trades = output.trades.len(),
        "portfolio run succeeded"
    );
    Ok(())
}

async fn build_simulation_input(
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    run: &PortfolioRunRecord,
) -> RearviewResult<PortfolioSimulationInput> {
    let account_snapshot: AccountSnapshot = serde_json::from_value(run.account_snapshot.clone())?;
    let execution_snapshot: ExecutionSnapshot =
        serde_json::from_value(run.execution_snapshot.clone())?;
    let signals = postgres
        .list_portfolio_source_signals(&run.source_run_id)
        .await?;
    let query_end_date = run
        .end_date
        .checked_add_days(Days::new(14))
        .ok_or_else(|| RearviewError::Validation(format!("end_date overflow: {}", run.end_date)))?;
    let trade_dates = clickhouse
        .query_trade_dates(
            run.start_date,
            query_end_date,
            &format!("portfolio-{}-trade-dates", run.portfolio_run_id),
        )
        .await?;
    let mut signal_inputs = Vec::new();
    for signal in signals {
        let Some(execution_date) = next_trade_date(&trade_dates, signal.trade_date) else {
            continue;
        };
        signal_inputs.push(signal.into_input(execution_date)?);
    }
    let security_codes = signal_inputs
        .iter()
        .map(|signal| signal.security_code.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let prices = clickhouse
        .query_portfolio_price_bars(
            &security_codes,
            run.start_date,
            query_end_date,
            &format!("portfolio-{}-prices", run.portfolio_run_id),
        )
        .await?;
    Ok(PortfolioSimulationInput {
        start_date: run.start_date,
        initial_cash: account_snapshot.initial_cash,
        max_positions: execution_snapshot.rebalance_policy.max_positions,
        cash_reserve_pct: execution_snapshot.rebalance_policy.cash_reserve_pct,
        lot_size: execution_snapshot.rebalance_policy.lot_size,
        min_trade_lots: execution_snapshot.rebalance_policy.min_trade_lots,
        fee_profile: execution_snapshot.fee_profile,
        slippage_profile: execution_snapshot.slippage_profile,
        exit_rules: execution_snapshot.risk_exit_policy.exit_rules()?,
        signals: signal_inputs,
        prices,
    })
}

fn next_trade_date(trade_dates: &[NaiveDate], signal_date: NaiveDate) -> Option<NaiveDate> {
    trade_dates.iter().copied().find(|date| *date > signal_date)
}

fn portfolio_failure_status(error: &RearviewError) -> &'static str {
    match error {
        RearviewError::Validation(_) => "failed_validation",
        RearviewError::ClickHouse(_) => "failed_market_data",
        RearviewError::Postgres(_) => "failed_write",
        RearviewError::Config(_)
        | RearviewError::Io(_)
        | RearviewError::Http(_)
        | RearviewError::Json(_)
        | RearviewError::Yaml(_)
        | RearviewError::NotFound(_)
        | RearviewError::MetricCatalog(_)
        | RearviewError::Planner(_)
        | RearviewError::Nats(_) => "failed_simulation",
    }
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "succeeded"
            | "failed_validation"
            | "failed_market_data"
            | "failed_simulation"
            | "failed_write"
            | "cancelled"
    )
}

#[derive(Debug, Deserialize)]
struct AccountSnapshot {
    initial_cash: f64,
}

#[derive(Debug, Deserialize)]
struct ExecutionSnapshot {
    fee_profile: FeeProfile,
    slippage_profile: SlippageProfile,
    rebalance_policy: RebalancePolicy,
    risk_exit_policy: RiskExitPolicy,
}

#[derive(Debug, Deserialize)]
struct RebalancePolicy {
    max_positions: usize,
    #[serde(default)]
    cash_reserve_pct: f64,
    #[serde(default = "default_lot_size")]
    lot_size: u32,
    #[serde(default = "default_min_trade_lots")]
    min_trade_lots: u32,
}

#[derive(Debug, Deserialize)]
struct RiskExitPolicy {
    #[serde(default)]
    exit_rules: Vec<Value>,
}

impl RiskExitPolicy {
    fn exit_rules(self) -> RearviewResult<Vec<ExitRule>> {
        self.exit_rules
            .into_iter()
            .map(|rule| {
                let rule_type = rule
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        RearviewError::Validation("exit rule type is required".to_string())
                    })?;
                match rule_type {
                    "fixed_stop_loss" => Ok(ExitRule::FixedStopLoss {
                        loss_pct: read_pct(&rule, "loss_pct")?,
                    }),
                    "take_profit" => Ok(ExitRule::TakeProfit {
                        profit_pct: read_pct(&rule, "profit_pct")?,
                    }),
                    "time_stop_loss" => Ok(ExitRule::TimeStopLoss {
                        holding_days: read_u32(&rule, "holding_days")?,
                        max_return_pct: read_pct(&rule, "max_return_pct")?,
                    }),
                    "indicator_stop_loss" => Err(RearviewError::Validation(
                        "indicator_stop_loss is not supported until indicator inputs are available for portfolio worker".to_string(),
                    )),
                    other => Err(RearviewError::Validation(format!(
                        "unsupported exit rule type: {other}"
                    ))),
                }
            })
            .collect()
    }
}

fn read_pct(rule: &Value, key: &str) -> RearviewResult<f64> {
    rule.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| RearviewError::Validation(format!("exit rule {key} is required")))
}

fn read_u32(rule: &Value, key: &str) -> RearviewResult<u32> {
    let value = rule
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| RearviewError::Validation(format!("exit rule {key} is required")))?;
    u32::try_from(value)
        .map_err(|error| RearviewError::Validation(format!("exit rule {key} is invalid: {error}")))
}

fn default_lot_size() -> u32 {
    100
}

fn default_min_trade_lots() -> u32 {
    1
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
        "rearview-portfolio-worker\n\nUSAGE:\n  rearview-portfolio-worker run\n\nENV:\n  REARVIEW_DATABASE_URL\n  REARVIEW_NATS_URL\n  CLICKHOUSE_HOST / CLICKHOUSE_PORT / CLICKHOUSE_USER / CLICKHOUSE_PASSWORD"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_rules_should_reject_indicator_stop_loss_until_worker_inputs_exist() {
        let policy = RiskExitPolicy {
            exit_rules: vec![serde_json::json!({
                "type": "indicator_stop_loss",
                "indicator_metric": "price_ma_20"
            })],
        };

        let error = policy
            .exit_rules()
            .expect_err("indicator stop loss should not be silently ignored");

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn terminal_status_should_include_all_portfolio_terminal_states() {
        for status in [
            "succeeded",
            "failed_validation",
            "failed_market_data",
            "failed_simulation",
            "failed_write",
            "cancelled",
        ] {
            assert!(is_terminal_status(status));
        }
    }
}
