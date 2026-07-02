use chrono::{NaiveDate, Utc};
use serde_json::Value;

use crate::error::{RearviewError, RearviewResult};
use crate::postgres::{NewStrategyPortfolio, RearviewPg, StrategyPortfolioRecord};
use crate::strategy_portfolio::new_portfolio_code;

#[derive(Debug, Clone)]
pub struct StrategyPortfolioSnapshotInput {
    pub name: String,
    pub rule_snapshot: Value,
    pub rule_hash: String,
    pub execution_config: Value,
    pub execution_config_hash: String,
    pub benchmark_security_code: String,
    pub catalog_hash: Option<String>,
    pub required_metrics: Value,
    pub required_marts: Value,
    pub source_strategy_backtest_run_id: String,
    pub source_result_attempt_id: String,
    pub source_period_key: String,
    pub source_start_date: NaiveDate,
    pub source_end_date: NaiveDate,
    pub initial_signal_date: NaiveDate,
    pub live_start_date: NaiveDate,
    pub pending_buy_signal_snapshot: Value,
    pub ui_display_snapshot: Value,
    pub client_request_id: Option<String>,
    pub request_hash: String,
    pub source_kind: String,
    pub example_case_id: Option<String>,
    pub example_version: Option<String>,
    pub fixture_hash: Option<String>,
}

pub async fn create_strategy_portfolio_from_snapshot(
    postgres: &RearviewPg,
    input: StrategyPortfolioSnapshotInput,
) -> RearviewResult<StrategyPortfolioRecord> {
    let mut last_error = None;
    for _ in 0..5 {
        let result = postgres
            .create_strategy_portfolio(NewStrategyPortfolio {
                portfolio_code: new_portfolio_code(Utc::now()),
                name: input.name.clone(),
                rule_snapshot: input.rule_snapshot.clone(),
                rule_hash: input.rule_hash.clone(),
                execution_config: input.execution_config.clone(),
                execution_config_hash: input.execution_config_hash.clone(),
                benchmark_security_code: input.benchmark_security_code.clone(),
                catalog_hash: input.catalog_hash.clone(),
                required_metrics: input.required_metrics.clone(),
                required_marts: input.required_marts.clone(),
                source_strategy_backtest_run_id: input.source_strategy_backtest_run_id.clone(),
                source_result_attempt_id: input.source_result_attempt_id.clone(),
                source_period_key: input.source_period_key.clone(),
                source_start_date: input.source_start_date,
                source_end_date: input.source_end_date,
                initial_signal_date: input.initial_signal_date,
                live_start_date: input.live_start_date,
                pending_buy_signal_snapshot: input.pending_buy_signal_snapshot.clone(),
                ui_display_snapshot: input.ui_display_snapshot.clone(),
                client_request_id: input.client_request_id.clone(),
                request_hash: input.request_hash.clone(),
                source_kind: input.source_kind.clone(),
                example_case_id: input.example_case_id.clone(),
                example_version: input.example_version.clone(),
                fixture_hash: input.fixture_hash.clone(),
            })
            .await;
        match result {
            Ok(record) => return Ok(record),
            Err(error)
                if postgres_unique_constraint(&error) == Some("uq_strategy_portfolio_code") =>
            {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        RearviewError::Conflict("could not allocate unique portfolio_code after 5 attempts".into())
    }))
}

fn postgres_unique_constraint(error: &RearviewError) -> Option<&str> {
    match error {
        RearviewError::Postgres(sqlx::Error::Database(database_error)) => {
            database_error.constraint()
        }
        _ => None,
    }
}
