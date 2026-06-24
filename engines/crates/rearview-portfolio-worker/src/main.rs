use chrono::{Days, NaiveDate};
use futures_util::StreamExt;
use rearview_core::clickhouse::ClickHouseClient;
use rearview_core::clickhouse::calculation_write::CalculationWriteBatch;
use rearview_core::config::{AppConfig, load_dotenv_if_present};
use rearview_core::domain::{MetricCatalog, RuleVersionSpec};
use rearview_core::nats::{
    RearviewTaskMessage, connect_jetstream, ensure_portfolio_consumer, ensure_portfolio_stream,
};
use rearview_core::planner::{QueryPlanner, QuerySettings};
use rearview_core::portfolio::{
    BuySignalInput, ExitRule, FeeProfile, PortfolioSimulationInput, SlippageProfile,
    simulate_portfolio,
};
use rearview_core::portfolio_performance::{PerformanceMetricConfig, compute_performance_metric};
use rearview_core::portfolio_trade_metrics::compute_trade_calculation_outputs;
use rearview_core::postgres::{
    PortfolioRunRecord, RearviewPg, StrategyBacktestRunRecord, StrategyPortfolioDailyRunRecord,
    StrategyPortfolioRecord, plan_date_chunks,
};
use rearview_core::service::catalog::load_catalog_from_policy;
use rearview_core::strategy_backtest::hash_json;
use rearview_core::{RearviewError, RearviewResult};
use serde::Deserialize;
use serde_json::{Value, json};
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
    let catalog = load_default_catalog()?;
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
        let payload = serde_json::from_slice::<RearviewTaskMessage>(&message.payload)?;
        match payload {
            RearviewTaskMessage::PortfolioRun {
                portfolio_run_id, ..
            } => handle_portfolio_task(&postgres, &clickhouse, &portfolio_run_id).await?,
            RearviewTaskMessage::StrategyBacktest { run_id } => {
                handle_strategy_backtest_task(&config, &postgres, &clickhouse, &catalog, &run_id)
                    .await?
            }
            RearviewTaskMessage::StrategyPortfolioDailyRun { daily_run_id } => {
                handle_strategy_portfolio_daily_run_task(
                    &config,
                    &postgres,
                    &clickhouse,
                    &catalog,
                    &daily_run_id,
                )
                .await?
            }
        }
        message
            .ack()
            .await
            .map_err(|error| RearviewError::Nats(error.to_string()))?;
        if once {
            return Ok(());
        }
    }
    Ok(())
}

async fn handle_portfolio_task(
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    portfolio_run_id: &str,
) -> RearviewResult<()> {
    let run = postgres.get_portfolio_run(portfolio_run_id).await?;
    if is_terminal_status(&run.status) {
        return Ok(());
    }
    let Some(claimed_run) = postgres
        .claim_portfolio_run_for_calculation(portfolio_run_id)
        .await?
    else {
        return Ok(());
    };
    match process_run(postgres, clickhouse, &claimed_run).await {
        Ok(()) => Ok(()),
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
                .await
        }
    }
}

async fn handle_strategy_backtest_task(
    config: &AppConfig,
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    catalog: &MetricCatalog,
    strategy_backtest_run_id: &str,
) -> RearviewResult<()> {
    let run = postgres
        .get_strategy_backtest_run(strategy_backtest_run_id)
        .await?;
    if is_terminal_status(&run.status) {
        return Ok(());
    }
    let Some(claimed_run) = postgres
        .claim_strategy_backtest_run(strategy_backtest_run_id, 900)
        .await?
    else {
        return Ok(());
    };
    match process_strategy_backtest_run(config, postgres, clickhouse, catalog, &claimed_run).await {
        Ok(()) => Ok(()),
        Err(error) => {
            error!(
                strategy_backtest_run_id = claimed_run.strategy_backtest_run_id,
                error = %error,
                "strategy backtest run failed"
            );
            postgres
                .fail_strategy_backtest_run(
                    &claimed_run.strategy_backtest_run_id,
                    strategy_backtest_failure_status(&error),
                    &error,
                )
                .await
        }
    }
}

async fn handle_strategy_portfolio_daily_run_task(
    config: &AppConfig,
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    catalog: &MetricCatalog,
    strategy_portfolio_daily_run_id: &str,
) -> RearviewResult<()> {
    let run = postgres
        .get_strategy_portfolio_daily_run(strategy_portfolio_daily_run_id)
        .await?;
    if is_terminal_status(&run.status) {
        return Ok(());
    }
    let Some(claimed_run) = postgres
        .claim_strategy_portfolio_daily_run(strategy_portfolio_daily_run_id, 900)
        .await?
    else {
        return Ok(());
    };
    match process_strategy_portfolio_daily_run(config, postgres, clickhouse, catalog, &claimed_run)
        .await
    {
        Ok(()) => Ok(()),
        Err(error) => {
            error!(
                strategy_portfolio_daily_run_id = claimed_run.strategy_portfolio_daily_run_id,
                error = %error,
                "strategy portfolio daily run failed"
            );
            postgres
                .fail_strategy_portfolio_daily_run(
                    &claimed_run.strategy_portfolio_daily_run_id,
                    strategy_backtest_failure_status(&error),
                    &error,
                )
                .await
        }
    }
}

async fn process_strategy_backtest_run(
    config: &AppConfig,
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    catalog: &MetricCatalog,
    run: &StrategyBacktestRunRecord,
) -> RearviewResult<()> {
    let execution_config =
        serde_json::from_value::<StrategyExecutionConfig>(run.execution_config.clone())?;
    let materialized = materialize_strategy_backtest_signals(
        config,
        postgres,
        clickhouse,
        catalog,
        run,
        &execution_config,
    )
    .await?;
    postgres
        .update_strategy_backtest_progress(
            &run.strategy_backtest_run_id,
            "loading_market_data",
            &json!({
                "stage": "loading_market_data",
                "security_count": materialized.security_codes.len(),
            }),
        )
        .await?;
    let prices = clickhouse
        .query_portfolio_price_bars(
            &materialized.security_codes,
            run.start_date,
            run.end_date,
            &format!("strategy-backtest-{}-prices", run.strategy_backtest_run_id),
        )
        .await?;
    let data_coverage_summary = json!({
        "price_bar_count": prices.len(),
        "price_bar_security_count": materialized.security_codes.len(),
        "start_date": run.start_date,
        "end_date": run.end_date,
        "indicator_stop_loss_metrics": execution_config.risk_exit_policy.indicator_metrics()?,
    });
    postgres
        .update_strategy_backtest_data_coverage(
            &run.strategy_backtest_run_id,
            &data_coverage_summary,
        )
        .await?;
    postgres
        .update_strategy_backtest_progress(
            &run.strategy_backtest_run_id,
            "calculating_nav",
            &json!({
                "stage": "calculating_nav",
                "signal_count": materialized.signals.len(),
                "price_bar_count": prices.len(),
            }),
        )
        .await?;
    let input = PortfolioSimulationInput {
        start_date: run.start_date,
        end_date: run.end_date,
        initial_cash: execution_config.account.initial_cash,
        max_positions: execution_config.rebalance_policy.max_positions,
        single_position_limit_pct: execution_config.rebalance_policy.single_position_limit_pct,
        cash_reserve_pct: execution_config.rebalance_policy.cash_reserve_pct,
        lot_size: execution_config.rebalance_policy.lot_size,
        min_trade_lots: execution_config.rebalance_policy.min_trade_lots,
        fee_profile: execution_config.fee_profile,
        slippage_profile: execution_config.slippage_profile,
        exit_rules: execution_config.risk_exit_policy.exit_rules()?,
        signals: materialized.signals,
        prices,
    };
    let output = simulate_portfolio(&input)?;

    postgres
        .update_strategy_backtest_progress(
            &run.strategy_backtest_run_id,
            "computing_performance",
            &json!({
                "stage": "computing_performance",
                "nav_points": output.nav.len(),
                "trade_count": output.trades.len(),
            }),
        )
        .await?;
    let result_attempt_id = ulid::Ulid::new().to_string();
    let metric_config = PerformanceMetricConfig::full_period_with_benchmark(
        &run.strategy_backtest_run_id,
        &result_attempt_id,
        &run.benchmark_security_code,
    );
    let benchmark_returns = clickhouse
        .query_mart_benchmark_returns(
            &metric_config.security_code,
            run.start_date,
            run.end_date,
            &format!("strategy-backtest-{result_attempt_id}-benchmark"),
        )
        .await?;
    let risk_free_rates = clickhouse
        .query_mart_risk_free_rates(
            &metric_config.risk_free_tenor,
            run.start_date,
            run.end_date,
            &format!("strategy-backtest-{result_attempt_id}-risk-free"),
        )
        .await?;
    let (performance_metric, performance_metric_statuses) = compute_performance_metric(
        &metric_config,
        &output.nav,
        &benchmark_returns,
        &risk_free_rates,
    );
    let (closed_trades, trade_metrics) = compute_trade_calculation_outputs(
        &run.strategy_backtest_run_id,
        &result_attempt_id,
        &output,
    );
    let calculation_batch = CalculationWriteBatch {
        performance_metrics: vec![performance_metric],
        performance_metric_statuses,
        closed_trades,
        trade_metrics,
    };

    postgres
        .update_strategy_backtest_progress(
            &run.strategy_backtest_run_id,
            "writing_results",
            &json!({
                "stage": "writing_results",
                "result_attempt_id": &result_attempt_id,
            }),
        )
        .await?;
    let write_result: RearviewResult<()> = async {
        let latest_run = postgres
            .get_strategy_backtest_run(&run.strategy_backtest_run_id)
            .await?;
        let portfolio_run = strategy_backtest_as_portfolio_run(&latest_run)?;
        let batch = clickhouse
            .write_portfolio_result_facts(&portfolio_run, &result_attempt_id, &output)
            .await?;
        postgres
            .insert_strategy_backtest_metric_config(&metric_config)
            .await?;
        clickhouse
            .write_portfolio_calculation_outputs(&result_attempt_id, &calculation_batch)
            .await?;
        clickhouse
            .write_portfolio_run_snapshot(&result_attempt_id, &batch.run_snapshot)
            .await?;
        Ok(())
    }
    .await;
    if let Err(error) = write_result {
        postgres
            .fail_strategy_backtest_run(&run.strategy_backtest_run_id, "failed_write", &error)
            .await?;
        return Ok(());
    }

    postgres
        .finalize_strategy_backtest_run_to_clickhouse(
            &run.strategy_backtest_run_id,
            &result_attempt_id,
            &serde_json::to_value(&output.summary)?,
        )
        .await?;
    info!(
        strategy_backtest_run_id = run.strategy_backtest_run_id,
        result_attempt_id = result_attempt_id,
        nav_points = output.nav.len(),
        trades = output.trades.len(),
        "strategy backtest run succeeded"
    );
    Ok(())
}

async fn process_strategy_portfolio_daily_run(
    config: &AppConfig,
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    catalog: &MetricCatalog,
    run: &StrategyPortfolioDailyRunRecord,
) -> RearviewResult<()> {
    let portfolio = postgres
        .get_strategy_portfolio(&run.strategy_portfolio_id)
        .await?;
    let execution_config =
        serde_json::from_value::<StrategyExecutionConfig>(portfolio.execution_config.clone())?;
    let materialized = materialize_strategy_portfolio_daily_run_signals(
        config,
        postgres,
        clickhouse,
        catalog,
        &portfolio,
        run,
        &execution_config,
    )
    .await?;
    postgres
        .update_strategy_portfolio_daily_progress(
            &run.strategy_portfolio_daily_run_id,
            "loading_market_data",
            &json!({
                "stage": "loading_market_data",
                "security_count": materialized.security_codes.len(),
            }),
        )
        .await?;
    let prices = clickhouse
        .query_portfolio_price_bars(
            &materialized.security_codes,
            run.run_start_date,
            run.trade_date,
            &format!(
                "strategy-portfolio-daily-{}-prices",
                run.strategy_portfolio_daily_run_id
            ),
        )
        .await?;
    let data_coverage_summary = json!({
        "price_bar_count": prices.len(),
        "price_bar_security_count": materialized.security_codes.len(),
        "start_date": run.run_start_date,
        "end_date": run.trade_date,
        "indicator_stop_loss_metrics": execution_config.risk_exit_policy.indicator_metrics()?,
    });
    postgres
        .update_strategy_portfolio_daily_data_coverage(
            &run.strategy_portfolio_daily_run_id,
            &data_coverage_summary,
        )
        .await?;
    postgres
        .update_strategy_portfolio_daily_progress(
            &run.strategy_portfolio_daily_run_id,
            "calculating_nav",
            &json!({
                "stage": "calculating_nav",
                "signal_count": materialized.signals.len(),
                "price_bar_count": prices.len(),
            }),
        )
        .await?;
    let input = PortfolioSimulationInput {
        start_date: run.run_start_date,
        end_date: run.trade_date,
        initial_cash: execution_config.account.initial_cash,
        max_positions: execution_config.rebalance_policy.max_positions,
        single_position_limit_pct: execution_config.rebalance_policy.single_position_limit_pct,
        cash_reserve_pct: execution_config.rebalance_policy.cash_reserve_pct,
        lot_size: execution_config.rebalance_policy.lot_size,
        min_trade_lots: execution_config.rebalance_policy.min_trade_lots,
        fee_profile: execution_config.fee_profile,
        slippage_profile: execution_config.slippage_profile,
        exit_rules: execution_config.risk_exit_policy.exit_rules()?,
        signals: materialized.signals,
        prices,
    };
    let output = simulate_portfolio(&input)?;

    postgres
        .update_strategy_portfolio_daily_progress(
            &run.strategy_portfolio_daily_run_id,
            "computing_performance",
            &json!({
                "stage": "computing_performance",
                "nav_points": output.nav.len(),
                "trade_count": output.trades.len(),
            }),
        )
        .await?;
    let result_attempt_id = ulid::Ulid::new().to_string();
    let metric_config = PerformanceMetricConfig::full_period_with_benchmark(
        &run.strategy_portfolio_daily_run_id,
        &result_attempt_id,
        &portfolio.benchmark_security_code,
    );
    let benchmark_returns = clickhouse
        .query_mart_benchmark_returns(
            &metric_config.security_code,
            run.run_start_date,
            run.trade_date,
            &format!("strategy-portfolio-daily-{result_attempt_id}-benchmark"),
        )
        .await?;
    let risk_free_rates = clickhouse
        .query_mart_risk_free_rates(
            &metric_config.risk_free_tenor,
            run.run_start_date,
            run.trade_date,
            &format!("strategy-portfolio-daily-{result_attempt_id}-risk-free"),
        )
        .await?;
    let (performance_metric, performance_metric_statuses) = compute_performance_metric(
        &metric_config,
        &output.nav,
        &benchmark_returns,
        &risk_free_rates,
    );
    let (closed_trades, trade_metrics) = compute_trade_calculation_outputs(
        &run.strategy_portfolio_daily_run_id,
        &result_attempt_id,
        &output,
    );
    let calculation_batch = CalculationWriteBatch {
        performance_metrics: vec![performance_metric],
        performance_metric_statuses,
        closed_trades,
        trade_metrics,
    };

    postgres
        .update_strategy_portfolio_daily_progress(
            &run.strategy_portfolio_daily_run_id,
            "writing_results",
            &json!({
                "stage": "writing_results",
                "result_attempt_id": &result_attempt_id,
            }),
        )
        .await?;
    let write_result: RearviewResult<()> = async {
        let portfolio_run = strategy_portfolio_daily_as_portfolio_run(&portfolio, run)?;
        let batch = clickhouse
            .write_portfolio_result_facts(&portfolio_run, &result_attempt_id, &output)
            .await?;
        clickhouse
            .write_portfolio_calculation_outputs(&result_attempt_id, &calculation_batch)
            .await?;
        clickhouse
            .write_portfolio_run_snapshot(&result_attempt_id, &batch.run_snapshot)
            .await?;
        Ok(())
    }
    .await;
    if let Err(error) = write_result {
        postgres
            .fail_strategy_portfolio_daily_run(
                &run.strategy_portfolio_daily_run_id,
                "failed_write",
                &error,
            )
            .await?;
        return Ok(());
    }

    postgres
        .finalize_strategy_portfolio_daily_run_to_clickhouse(
            &run.strategy_portfolio_daily_run_id,
            &result_attempt_id,
            &serde_json::to_value(&output.summary)?,
        )
        .await?;
    info!(
        strategy_portfolio_daily_run_id = run.strategy_portfolio_daily_run_id,
        strategy_portfolio_id = portfolio.strategy_portfolio_id,
        result_attempt_id = result_attempt_id,
        nav_points = output.nav.len(),
        trades = output.trades.len(),
        "strategy portfolio daily run succeeded"
    );
    Ok(())
}

async fn materialize_strategy_backtest_signals(
    config: &AppConfig,
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    catalog: &MetricCatalog,
    run: &StrategyBacktestRunRecord,
    execution_config: &StrategyExecutionConfig,
) -> RearviewResult<MaterializedSignals> {
    postgres
        .update_strategy_backtest_progress(
            &run.strategy_backtest_run_id,
            "compiling_signals",
            &json!({"stage": "compiling_signals"}),
        )
        .await?;
    let rule = serde_json::from_value::<RuleVersionSpec>(run.rule_snapshot.clone())?;
    let trade_dates = clickhouse
        .query_trade_dates(
            run.start_date,
            run.end_date,
            &format!(
                "strategy-backtest-{}-trade-dates",
                run.strategy_backtest_run_id
            ),
        )
        .await?;
    if trade_dates.len() < 2 {
        return Err(RearviewError::Validation(format!(
            "strategy backtest range must contain at least two trade dates: {} to {}",
            run.start_date, run.end_date
        )));
    }

    let planner = QueryPlanner::new(catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: config.clickhouse.max_rows_to_read,
        max_bytes_to_read: config.clickhouse.max_bytes_to_read,
    };
    let chunks = plan_date_chunks(
        run.start_date,
        run.end_date,
        config.chunk_small_range_trading_days,
    )?;
    let top_n = execution_config.signal_policy.buy_signal_top_n;
    let mut compiled_hashes = Vec::with_capacity(chunks.len());
    let mut required_metrics = BTreeSet::new();
    let mut required_marts = BTreeSet::new();
    let mut signals = Vec::new();
    let mut security_codes = BTreeSet::new();
    let mut signal_dates = BTreeSet::new();
    let mut generated_candidate_count = 0_usize;
    let mut top_n_candidate_count = 0_usize;
    let mut dropped_no_next_trade_date = 0_usize;
    let mut dropped_after_end_date = 0_usize;

    for chunk in chunks {
        postgres
            .update_strategy_backtest_progress(
                &run.strategy_backtest_run_id,
                "running_clickhouse",
                &json!({
                    "stage": "running_clickhouse",
                    "chunk_no": chunk.chunk_no,
                    "chunk_start_date": chunk.start_date,
                    "chunk_end_date": chunk.end_date,
                    "generated_signal_count": signals.len(),
                }),
            )
            .await?;
        let compiled = planner.compile(
            &rule,
            Some(chunk.start_date),
            Some(chunk.end_date),
            top_n,
            settings,
        )?;
        compiled_hashes.push(compiled.sql_hash.clone());
        required_metrics.extend(compiled.required_metrics.iter().cloned());
        required_marts.extend(compiled.required_marts.iter().cloned());
        let rows = clickhouse
            .query_screening_rows(
                &compiled.sql,
                &format!(
                    "strategy-backtest-{}-chunk-{}",
                    run.strategy_backtest_run_id, chunk.chunk_no
                ),
            )
            .await?;
        generated_candidate_count += rows.len();
        for row in rows {
            signal_dates.insert(row.trade_date);
            if !row.is_buy_signal || row.signal_rank > top_n {
                continue;
            }
            top_n_candidate_count += 1;
            let Some(execution_date) = next_trade_date(&trade_dates, row.trade_date) else {
                dropped_no_next_trade_date += 1;
                continue;
            };
            if execution_date > run.end_date {
                dropped_after_end_date += 1;
                continue;
            }
            security_codes.insert(row.security_code.clone());
            signals.push(BuySignalInput {
                signal_date: row.trade_date,
                execution_date,
                security_code: row.security_code,
                rank: row.signal_rank,
                score: row.score,
            });
        }
    }

    signals.sort_by_key(|signal| {
        (
            signal.execution_date,
            signal.rank,
            signal.security_code.clone(),
        )
    });
    let required_metrics = required_metrics.into_iter().collect::<Vec<_>>();
    let required_marts = required_marts.into_iter().collect::<Vec<_>>();
    let signal_summary = json!({
        "signal_date_count": signal_dates.len(),
        "generated_candidate_count": generated_candidate_count,
        "top_n_candidate_count": top_n_candidate_count,
        "executable_signal_count": signals.len(),
        "dropped_signal_count": dropped_no_next_trade_date + dropped_after_end_date,
        "dropped_signal_reasons": {
            "no_next_trade_date": dropped_no_next_trade_date,
            "execution_date_after_end_date": dropped_after_end_date,
        },
        "buy_signal_top_n": top_n,
    });
    let compiled_sql_hash = combined_hash(&compiled_hashes)?;
    postgres
        .update_strategy_backtest_signal_materialization(
            &run.strategy_backtest_run_id,
            &compiled_sql_hash,
            &json!(required_metrics),
            &json!(required_marts),
            &signal_summary,
        )
        .await?;

    Ok(MaterializedSignals {
        signals,
        security_codes: security_codes.into_iter().collect(),
    })
}

async fn materialize_strategy_portfolio_daily_run_signals(
    config: &AppConfig,
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    catalog: &MetricCatalog,
    portfolio: &StrategyPortfolioRecord,
    run: &StrategyPortfolioDailyRunRecord,
    execution_config: &StrategyExecutionConfig,
) -> RearviewResult<MaterializedSignals> {
    postgres
        .update_strategy_portfolio_daily_progress(
            &run.strategy_portfolio_daily_run_id,
            "compiling_signals",
            &json!({"stage": "compiling_signals"}),
        )
        .await?;
    let rule = serde_json::from_value::<RuleVersionSpec>(portfolio.rule_snapshot.clone())?;
    let trade_dates = clickhouse
        .query_trade_dates(
            run.run_start_date,
            run.trade_date,
            &format!(
                "strategy-portfolio-daily-{}-trade-dates",
                run.strategy_portfolio_daily_run_id
            ),
        )
        .await?;
    if trade_dates.len() < 2 {
        return Err(RearviewError::Validation(format!(
            "strategy portfolio daily run range must contain at least two trade dates: {} to {}",
            run.run_start_date, run.trade_date
        )));
    }

    let planner = QueryPlanner::new(catalog.clone());
    let settings = QuerySettings {
        max_execution_time_seconds: config.clickhouse.max_execution_time_seconds,
        max_rows_to_read: config.clickhouse.max_rows_to_read,
        max_bytes_to_read: config.clickhouse.max_bytes_to_read,
    };
    let chunks = plan_date_chunks(
        run.run_start_date,
        run.trade_date,
        config.chunk_small_range_trading_days,
    )?;
    let top_n = execution_config.signal_policy.buy_signal_top_n;
    let mut compiled_hashes = Vec::with_capacity(chunks.len());
    let mut required_metrics = BTreeSet::new();
    let mut required_marts = BTreeSet::new();
    let mut signals = Vec::new();
    let mut security_codes = BTreeSet::new();
    let mut signal_dates = BTreeSet::new();
    let mut generated_candidate_count = 0_usize;
    let mut top_n_candidate_count = 0_usize;
    let mut dropped_no_next_trade_date = 0_usize;
    let mut dropped_after_end_date = 0_usize;

    for chunk in chunks {
        postgres
            .update_strategy_portfolio_daily_progress(
                &run.strategy_portfolio_daily_run_id,
                "running_clickhouse",
                &json!({
                    "stage": "running_clickhouse",
                    "chunk_no": chunk.chunk_no,
                    "chunk_start_date": chunk.start_date,
                    "chunk_end_date": chunk.end_date,
                    "generated_signal_count": signals.len(),
                }),
            )
            .await?;
        let compiled = planner.compile(
            &rule,
            Some(chunk.start_date),
            Some(chunk.end_date),
            top_n,
            settings,
        )?;
        compiled_hashes.push(compiled.sql_hash.clone());
        required_metrics.extend(compiled.required_metrics.iter().cloned());
        required_marts.extend(compiled.required_marts.iter().cloned());
        let rows = clickhouse
            .query_screening_rows(
                &compiled.sql,
                &format!(
                    "strategy-portfolio-daily-{}-chunk-{}",
                    run.strategy_portfolio_daily_run_id, chunk.chunk_no
                ),
            )
            .await?;
        generated_candidate_count += rows.len();
        for row in rows {
            signal_dates.insert(row.trade_date);
            if !row.is_buy_signal || row.signal_rank > top_n {
                continue;
            }
            top_n_candidate_count += 1;
            let Some(execution_date) = next_trade_date(&trade_dates, row.trade_date) else {
                dropped_no_next_trade_date += 1;
                continue;
            };
            if execution_date > run.trade_date {
                dropped_after_end_date += 1;
                continue;
            }
            security_codes.insert(row.security_code.clone());
            signals.push(BuySignalInput {
                signal_date: row.trade_date,
                execution_date,
                security_code: row.security_code,
                rank: row.signal_rank,
                score: row.score,
            });
        }
    }

    signals.sort_by_key(|signal| {
        (
            signal.execution_date,
            signal.rank,
            signal.security_code.clone(),
        )
    });
    let required_metrics = required_metrics.into_iter().collect::<Vec<_>>();
    let required_marts = required_marts.into_iter().collect::<Vec<_>>();
    let signal_summary = json!({
        "signal_date_count": signal_dates.len(),
        "generated_candidate_count": generated_candidate_count,
        "top_n_candidate_count": top_n_candidate_count,
        "executable_signal_count": signals.len(),
        "dropped_signal_count": dropped_no_next_trade_date + dropped_after_end_date,
        "dropped_signal_reasons": {
            "no_next_trade_date": dropped_no_next_trade_date,
            "execution_date_after_end_date": dropped_after_end_date,
        },
        "buy_signal_top_n": top_n,
        "compiled_sql_hash": combined_hash(&compiled_hashes)?,
        "required_metrics": required_metrics,
        "required_marts": required_marts,
    });
    postgres
        .update_strategy_portfolio_daily_signal_materialization(
            &run.strategy_portfolio_daily_run_id,
            &signal_summary,
        )
        .await?;

    Ok(MaterializedSignals {
        signals,
        security_codes: security_codes.into_iter().collect(),
    })
}

fn strategy_backtest_as_portfolio_run(
    run: &StrategyBacktestRunRecord,
) -> RearviewResult<PortfolioRunRecord> {
    let account_snapshot = run
        .execution_config
        .get("account")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let execution_snapshot = json!({
        "source_kind": "strategy_backtest",
        "strategy_backtest_run_id": &run.strategy_backtest_run_id,
        "rule_hash": &run.rule_hash,
        "execution_config_hash": &run.execution_config_hash,
        "benchmark_security_code": &run.benchmark_security_code,
        "catalog_hash": &run.catalog_hash,
        "compiled_sql_hash": &run.compiled_sql_hash,
        "required_metrics": &run.required_metrics,
        "required_marts": &run.required_marts,
        "period_key": &run.period_key,
        "start_date": run.start_date,
        "end_date": run.end_date,
        "execution_config": &run.execution_config,
    });
    Ok(PortfolioRunRecord {
        portfolio_run_id: run.strategy_backtest_run_id.clone(),
        source_run_id: run
            .preview_id
            .clone()
            .unwrap_or_else(|| run.strategy_backtest_run_id.clone()),
        rule_version_id: "strategy_backtest".to_string(),
        rule_hash: run.rule_hash.clone(),
        account_template_id: None,
        account_snapshot,
        execution_snapshot,
        price_basis: run.price_basis.clone(),
        start_date: run.start_date,
        end_date: run.end_date,
        status: run.status.clone(),
        dispatch_status: run.dispatch_status.clone(),
        nats_stream_sequence: run.nats_stream_sequence,
        summary: run.summary.clone(),
        error_type: run.error_type.clone(),
        error_message: run.error_message.clone(),
        current_result_attempt_id: run.current_result_attempt_id.clone(),
    })
}

fn strategy_portfolio_daily_as_portfolio_run(
    portfolio: &StrategyPortfolioRecord,
    run: &StrategyPortfolioDailyRunRecord,
) -> RearviewResult<PortfolioRunRecord> {
    let account_snapshot = portfolio
        .execution_config
        .get("account")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let execution_snapshot = json!({
        "source_kind": "strategy_portfolio_daily_run",
        "strategy_portfolio_id": &portfolio.strategy_portfolio_id,
        "strategy_portfolio_daily_run_id": &run.strategy_portfolio_daily_run_id,
        "source_strategy_backtest_run_id": &portfolio.source_strategy_backtest_run_id,
        "source_result_attempt_id": &portfolio.source_result_attempt_id,
        "rule_hash": &portfolio.rule_hash,
        "execution_config_hash": &portfolio.execution_config_hash,
        "benchmark_security_code": &portfolio.benchmark_security_code,
        "catalog_hash": &portfolio.catalog_hash,
        "required_metrics": &portfolio.required_metrics,
        "required_marts": &portfolio.required_marts,
        "source_period_key": &portfolio.source_period_key,
        "run_start_date": run.run_start_date,
        "trade_date": run.trade_date,
        "execution_config": &portfolio.execution_config,
    });
    Ok(PortfolioRunRecord {
        portfolio_run_id: run.strategy_portfolio_daily_run_id.clone(),
        source_run_id: portfolio.source_strategy_backtest_run_id.clone(),
        rule_version_id: "strategy_portfolio_daily_run".to_string(),
        rule_hash: portfolio.rule_hash.clone(),
        account_template_id: None,
        account_snapshot,
        execution_snapshot,
        price_basis: portfolio.price_basis.clone(),
        start_date: run.run_start_date,
        end_date: run.trade_date,
        status: run.status.clone(),
        dispatch_status: run.dispatch_status.clone(),
        nats_stream_sequence: run.nats_stream_sequence,
        summary: run.summary.clone(),
        error_type: run.error_type.clone(),
        error_message: run.error_message.clone(),
        current_result_attempt_id: run.current_result_attempt_id.clone(),
    })
}

fn strategy_backtest_failure_status(error: &RearviewError) -> &'static str {
    match error {
        RearviewError::Validation(_)
        | RearviewError::Conflict(_)
        | RearviewError::MetricCatalog(_) => "failed_validation",
        RearviewError::Planner(_) => "failed_compile",
        RearviewError::ClickHouse(_) | RearviewError::Http(_) => "failed_market_data",
        RearviewError::Postgres(_) | RearviewError::Nats(_) => "failed_write",
        RearviewError::Config(_)
        | RearviewError::Io(_)
        | RearviewError::Json(_)
        | RearviewError::Yaml(_)
        | RearviewError::NotFound(_) => "failed_simulation",
    }
}

fn combined_hash(parts: &[String]) -> RearviewResult<String> {
    hash_json(&parts)
}

async fn process_run(
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    run: &PortfolioRunRecord,
) -> RearviewResult<()> {
    let input = build_simulation_input(postgres, clickhouse, run).await?;
    let output = simulate_portfolio(&input)?;

    let result_attempt_id = ulid::Ulid::new().to_string();
    let metric_config =
        PerformanceMetricConfig::default_full_period(&run.portfolio_run_id, &result_attempt_id);
    let benchmark_returns = clickhouse
        .query_mart_benchmark_returns(
            &metric_config.security_code,
            run.start_date,
            run.end_date,
            &format!("portfolio-{result_attempt_id}-benchmark"),
        )
        .await?;
    let risk_free_rates = clickhouse
        .query_mart_risk_free_rates(
            &metric_config.risk_free_tenor,
            run.start_date,
            run.end_date,
            &format!("portfolio-{result_attempt_id}-risk-free"),
        )
        .await?;
    let (performance_metric, performance_metric_statuses) = compute_performance_metric(
        &metric_config,
        &output.nav,
        &benchmark_returns,
        &risk_free_rates,
    );
    let (closed_trades, trade_metrics) =
        compute_trade_calculation_outputs(&run.portfolio_run_id, &result_attempt_id, &output);
    let calculation_batch = CalculationWriteBatch {
        performance_metrics: vec![performance_metric],
        performance_metric_statuses,
        closed_trades,
        trade_metrics,
    };

    // Write results to ClickHouse/PostgreSQL. Write failures must be "failed_write",
    // not "failed_market_data" (which is for read failures), so we handle
    // the error here rather than relying on portfolio_failure_status.
    let write_result: RearviewResult<()> = async {
        let batch = clickhouse
            .write_portfolio_result_facts(run, &result_attempt_id, &output)
            .await?;
        postgres
            .insert_portfolio_metric_config(&metric_config)
            .await?;
        clickhouse
            .write_portfolio_calculation_outputs(&result_attempt_id, &calculation_batch)
            .await?;
        clickhouse
            .write_portfolio_run_snapshot(&result_attempt_id, &batch.run_snapshot)
            .await?;
        Ok(())
    }
    .await;
    if let Err(error) = write_result {
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
        end_date: run.end_date,
        initial_cash: account_snapshot.initial_cash,
        max_positions: execution_snapshot.rebalance_policy.max_positions,
        single_position_limit_pct: execution_snapshot
            .rebalance_policy
            .single_position_limit_pct,
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
        RearviewError::Validation(_) | RearviewError::Conflict(_) => "failed_validation",
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
            | "failed_compile"
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
struct StrategyExecutionConfig {
    account: AccountSnapshot,
    signal_policy: SignalPolicy,
    fee_profile: FeeProfile,
    slippage_profile: SlippageProfile,
    rebalance_policy: RebalancePolicy,
    risk_exit_policy: RiskExitPolicy,
}

#[derive(Debug, Deserialize)]
struct SignalPolicy {
    buy_signal_top_n: u32,
}

#[derive(Debug)]
struct MaterializedSignals {
    signals: Vec<BuySignalInput>,
    security_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RebalancePolicy {
    max_positions: usize,
    #[serde(default)]
    single_position_limit_pct: Option<f64>,
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
    fn indicator_metrics(&self) -> RearviewResult<Vec<String>> {
        let mut metrics = Vec::new();
        for rule in &self.exit_rules {
            let rule_type = rule.get("type").and_then(Value::as_str).ok_or_else(|| {
                RearviewError::Validation("exit rule type is required".to_string())
            })?;
            if rule_type == "indicator_stop_loss" {
                metrics.push(read_str(rule, "metric")?.to_string());
            }
        }
        Ok(metrics)
    }

    fn exit_rules(self) -> RearviewResult<Vec<ExitRule>> {
        self.exit_rules
            .into_iter()
            .map(|rule| {
                let rule_type = rule.get("type").and_then(Value::as_str).ok_or_else(|| {
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
                    "indicator_stop_loss" => {
                        validate_exact_str(&rule, "source", "trend")?;
                        validate_exact_str(&rule, "operator", "close_below_metric")?;
                        Ok(ExitRule::IndicatorStopLoss {
                            metric: read_str(&rule, "metric")?.to_string(),
                        })
                    }
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

fn read_str<'a>(rule: &'a Value, key: &str) -> RearviewResult<&'a str> {
    rule.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| RearviewError::Validation(format!("exit rule {key} is required")))
}

fn validate_exact_str(rule: &Value, key: &str, expected: &str) -> RearviewResult<()> {
    let value = read_str(rule, key)?;
    if value == expected {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!(
            "exit rule {key} must be {expected}"
        )))
    }
}

fn default_lot_size() -> u32 {
    100
}

fn default_min_trade_lots() -> u32 {
    1
}

fn load_default_catalog() -> RearviewResult<MetricCatalog> {
    let repo_root = find_repo_root()?;
    let policy_path = repo_root.join("engines/crates/rearview-core/config/metric_policy.yml");
    let dbt_marts_dir = repo_root.join("pipeline/elt/models/marts");
    let marts_database = std::env::var("REARVIEW_CLICKHOUSE_MARTS_DATABASE")
        .unwrap_or_else(|_| "fleur_marts".to_string());
    load_catalog_from_policy(policy_path, dbt_marts_dir, &marts_database)
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
    fn exit_rules_should_convert_indicator_stop_loss() {
        let policy = RiskExitPolicy {
            exit_rules: vec![serde_json::json!({
                "type": "indicator_stop_loss",
                "source": "trend",
                "metric": "price_ma_20",
                "operator": "close_below_metric"
            })],
        };

        let rules = policy.exit_rules().expect("indicator rule should convert");

        assert!(matches!(
            rules.as_slice(),
            [ExitRule::IndicatorStopLoss { metric }] if metric == "price_ma_20"
        ));
    }

    #[test]
    fn exit_rules_should_reject_non_trend_indicator_stop_loss() {
        let policy = RiskExitPolicy {
            exit_rules: vec![serde_json::json!({
                "type": "indicator_stop_loss",
                "source": "momentum",
                "metric": "rsi_6",
                "operator": "close_below_metric"
            })],
        };

        let error = policy
            .exit_rules()
            .expect_err("non-trend indicator stop loss should be rejected");

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn terminal_status_should_include_all_portfolio_terminal_states() {
        for status in [
            "succeeded",
            "failed_validation",
            "failed_compile",
            "failed_market_data",
            "failed_simulation",
            "failed_write",
            "cancelled",
        ] {
            assert!(is_terminal_status(status));
        }
    }
}
