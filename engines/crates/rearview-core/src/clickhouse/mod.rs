use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use chrono::NaiveDate;
use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize};
use tracing::info;

use crate::config::ClickHouseConfig;
use crate::error::{RearviewError, RearviewResult};
use crate::portfolio::PriceBar;
use crate::portfolio_performance::{BenchmarkReturn, RiskFreeRate};

pub mod calculation_schema;
pub mod calculation_write;
pub mod portfolio_schema;
pub mod portfolio_write;

use portfolio_schema::all_table_sqls;
use portfolio_write::WriteBatch;

const BACKTEST_RESULT_DATABASE: &str = "fleur_backtest";
const BACKTEST_RUN_ID_FIELD: &str = "strategy_backtest_run_id";
const LIVE_RUN_ID_FIELD: &str = "strategy_portfolio_daily_run_id";

#[derive(Debug, Clone, Copy)]
struct SplitTargetFamily<'a> {
    database: &'a str,
    table: &'a str,
    run_id_field: &'a str,
    query_prefix: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct SplitPerformanceFamily<'a> {
    database: &'a str,
    prefix: &'a str,
    run_id_field: &'a str,
    query_prefix: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
struct TradeDateRow {
    trade_date: NaiveDate,
}

#[derive(Debug, Clone, Deserialize)]
struct BenchmarkReturnRow {
    trade_date: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    return_daily: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct VirtualAccountNavRow {
    trade_date: NaiveDate,
    cash_balance: f64,
    position_market_value: f64,
    total_equity: f64,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    daily_return: Option<f64>,
    position_count: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct VirtualAccountHoldingRow {
    holding_unrealized_pnl: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrategyPortfolioVirtualAccountRecord {
    pub account_date: NaiveDate,
    pub cash_balance: f64,
    pub position_market_value: f64,
    pub total_equity: f64,
    pub holding_unrealized_pnl: f64,
    pub daily_pnl: Option<f64>,
    pub daily_return: Option<f64>,
    pub position_count: i32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrategyPortfolioStatementSummaryRecord {
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub average_position_pct: Option<f64>,
    pub traded_security_count: u64,
    pub trade_count: u64,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub trade_win_rate: Option<f64>,
    pub winning_security_count: u64,
    pub losing_security_count: u64,
    pub holding_days: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrategyPortfolioStatementOperationRecord {
    pub portfolio_trade_id: String,
    pub trade_seq: i32,
    pub trade_date: NaiveDate,
    pub security_code: String,
    pub side: String,
    pub execution_price: f64,
    pub quantity: f64,
    pub lot_size: u32,
    pub lot_count: f64,
    pub gross_amount: f64,
    pub commission: f64,
    pub stamp_duty: f64,
    pub transfer_fee: f64,
    pub total_fee: f64,
    pub position_balance_quantity: f64,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub realized_pnl: Option<f64>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RiskFreeRateRow {
    trade_date: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    daily_rate: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScreeningRow {
    pub security_code: String,
    pub trade_date: NaiveDate,
    pub raw_score: f64,
    pub score: f64,
    pub signal_rank: u32,
    #[serde(default)]
    pub pool_count: Option<usize>,
    #[serde(deserialize_with = "deserialize_clickhouse_bool")]
    pub is_buy_signal: bool,
    pub score_breakdown: String,
    pub selected_metrics: String,
    pub raw_values: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BacktestSignalRow {
    pub security_code: String,
    pub trade_date: NaiveDate,
    pub score: f64,
    pub signal_rank: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketDataDemandEntry {
    pub security_code: String,
    pub start_date: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketDataDemand {
    pub entries: Vec<MarketDataDemandEntry>,
    pub end_date: NaiveDate,
}

impl MarketDataDemand {
    pub fn from_security_start_dates<I>(items: I, end_date: NaiveDate) -> RearviewResult<Self>
    where
        I: IntoIterator<Item = (String, NaiveDate)>,
    {
        let mut start_by_security = BTreeMap::<String, NaiveDate>::new();
        for (security_code, start_date) in items {
            validate_security_code(&security_code)?;
            if start_date > end_date {
                return Err(RearviewError::Validation(format!(
                    "market data demand start_date must be <= end_date: {security_code} {start_date} > {end_date}"
                )));
            }
            start_by_security
                .entry(security_code)
                .and_modify(|existing| *existing = (*existing).min(start_date))
                .or_insert(start_date);
        }
        Ok(Self {
            entries: start_by_security
                .into_iter()
                .map(|(security_code, start_date)| MarketDataDemandEntry {
                    security_code,
                    start_date,
                })
                .collect(),
            end_date,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn earliest_start_date(&self) -> Option<NaiveDate> {
        self.entries.iter().map(|entry| entry.start_date).min()
    }

    pub fn security_codes(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.security_code.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewTimelineRow {
    pub trade_date: NaiveDate,
    pub pool_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityDisplayRow {
    pub security_code: String,
    pub security_name: Option<String>,
    pub exchange_code: Option<String>,
    pub security_board: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuoteMartRow {
    pub security_code: String,
    pub trade_date: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub open_price: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub high_price: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub low_price: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub close_price: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub prev_close_price: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub prev_close_price_unadj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub open_price_forward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub high_price_forward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub low_price_forward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub close_price_forward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub prev_close_price_forward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub open_price_backward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub high_price_backward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub low_price_backward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub close_price_backward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub prev_close_price_backward_adj: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub forward_adjustment_factor: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub forward_adjustment_ratio: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub backward_adjustment_factor: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub backward_adjustment_ratio: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub prev_volume: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub volume: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub amount: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub turnover_rate: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub turnover_rate_actual: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub pct_amplitude: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub pct_change: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub limit_up_price: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub limit_down_price: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub a_market_cap: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub a_float_market_cap: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub a_free_float_market_cap: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub a_shares: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub a_float_shares: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub a_free_float_shares: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub pe_static: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub pe_ttm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub pe_forecast: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub pb_mrq: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub book_value_per_share: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub roe: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub roa: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub roaa: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub roae: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub dy_static: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub dy_ttm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool")]
    pub is_suspend: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool")]
    pub is_st: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_rsv: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_k_value: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_d_value: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_j_value: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum AnalysisQuoteAdjustment {
    ForwardAdjusted,
    BackwardAdjusted,
    Unadjusted,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrendIndicatorRow {
    pub security_code: String,
    pub trade_date: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_ma_5: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_ma_10: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_ma_20: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_ma_30: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_ma_60: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_ma_250: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_avg_ma_3_6_12_24: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_avg_ma_14_28_57_114: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub price_ema2_10: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub boll_mid_20_2: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub boll_up_20_2: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub boll_dn_20_2: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub macd_dif: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub macd_dea: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub macd_histogram: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MomentumIndicatorRow {
    pub security_code: String,
    pub trade_date: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub rsi_6: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub rsi_12: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub rsi_24: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_rsv: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_k_value: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_d_value: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub kdj_j_value: Option<f64>,
}

#[derive(Clone)]
pub struct ClickHouseClient {
    config: ClickHouseConfig,
    client: reqwest::Client,
}

impl ClickHouseClient {
    pub fn new(config: ClickHouseConfig) -> RearviewResult<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.query_timeout)
            .build()?;
        Ok(Self { config, client })
    }

    pub fn config(&self) -> &ClickHouseConfig {
        &self.config
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub async fn check_readiness(&self) -> RearviewResult<()> {
        let body = self.execute_text("SELECT 1", "rearview-readiness").await?;
        if body.trim() != "1" {
            return Err(RearviewError::ClickHouse(format!(
                "unexpected readiness response: {body}"
            )));
        }
        let database_check_sql = format!(
            "SELECT count() FROM system.databases WHERE name = '{}'",
            self.config.marts_database.replace('\'', "''")
        );
        let body = self
            .execute_text(&database_check_sql, "rearview-mart-database-readiness")
            .await?;
        if body.trim() != "1" {
            return Err(RearviewError::ClickHouse(format!(
                "ClickHouse mart database does not exist: {}",
                self.config.marts_database
            )));
        }
        Ok(())
    }

    pub async fn ensure_portfolio_schema(&self) -> RearviewResult<()> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let db_sql = portfolio_schema::create_database_sql(database);
        self.execute_text(&db_sql, "rearview-portfolio-schema-db")
            .await?;
        for table_sql in all_table_sqls(database) {
            self.execute_text(&table_sql, "rearview-portfolio-schema-table")
                .await?;
        }
        for table_sql in portfolio_schema::all_live_table_sqls(database) {
            self.execute_text(&table_sql, "rearview-live-schema-table")
                .await?;
        }
        for table_sql in calculation_schema::all_live_table_sqls(database) {
            self.execute_text(&table_sql, "rearview-live-calculation-schema-table")
                .await?;
        }

        validate_identifier(BACKTEST_RESULT_DATABASE)?;
        let db_sql = portfolio_schema::create_database_sql(BACKTEST_RESULT_DATABASE);
        self.execute_text(&db_sql, "rearview-backtest-schema-db")
            .await?;
        for table_sql in portfolio_schema::all_backtest_table_sqls(BACKTEST_RESULT_DATABASE) {
            self.execute_text(&table_sql, "rearview-backtest-schema-table")
                .await?;
        }
        for table_sql in calculation_schema::all_backtest_table_sqls(BACKTEST_RESULT_DATABASE) {
            self.execute_text(&table_sql, "rearview-backtest-calculation-schema-table")
                .await?;
        }

        let calculation_database = &self.config.calculation_database;
        validate_identifier(calculation_database)?;
        let db_sql = calculation_schema::create_database_sql(calculation_database);
        self.execute_text(&db_sql, "rearview-calculation-schema-db")
            .await?;
        for table_sql in calculation_schema::all_table_sqls(calculation_database) {
            self.execute_text(&table_sql, "rearview-calculation-schema-table")
                .await?;
        }
        Ok(())
    }

    /// Write a complete portfolio run's results to ClickHouse, append-only
    /// under `result_attempt_id`. Tables are written in dependency order;
    /// `portfolio_run_snapshot` is written last to mark a complete attempt.
    pub async fn write_portfolio_results(
        &self,
        run: &crate::postgres::PortfolioRunRecord,
        result_attempt_id: &str,
        output: &crate::portfolio::PortfolioSimulationOutput,
    ) -> RearviewResult<()> {
        let batch = self
            .write_portfolio_result_facts(run, result_attempt_id, output)
            .await?;
        self.write_portfolio_run_snapshot(result_attempt_id, &batch.run_snapshot)
            .await?;
        Ok(())
    }

    pub async fn write_portfolio_result_facts(
        &self,
        run: &crate::postgres::PortfolioRunRecord,
        result_attempt_id: &str,
        output: &crate::portfolio::PortfolioSimulationOutput,
    ) -> RearviewResult<WriteBatch> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let batch = WriteBatch::from_output(run, result_attempt_id, output);
        let query_id = format!("rearview-portfolio-write-{result_attempt_id}");

        self.insert_rows(database, "portfolio_target", &batch.targets, &query_id)
            .await?;
        self.insert_rows(database, "portfolio_order", &batch.orders, &query_id)
            .await?;
        self.insert_rows(database, "portfolio_trade", &batch.trades, &query_id)
            .await?;
        self.insert_rows(
            database,
            "portfolio_position_day",
            &batch.positions,
            &query_id,
        )
        .await?;
        self.insert_rows(database, "portfolio_nav_daily", &batch.nav, &query_id)
            .await?;
        self.insert_rows(database, "portfolio_event", &batch.events, &query_id)
            .await?;
        Ok(batch)
    }

    pub async fn write_portfolio_run_snapshot(
        &self,
        result_attempt_id: &str,
        run_snapshot: &portfolio_write::RunSnapshotRow,
    ) -> RearviewResult<()> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let query_id = format!("rearview-portfolio-write-{result_attempt_id}");
        self.insert_single(database, "portfolio_run_snapshot", run_snapshot, &query_id)
            .await?;
        Ok(())
    }

    pub async fn write_strategy_backtest_result_facts(
        &self,
        run: &crate::postgres::PortfolioRunRecord,
        result_attempt_id: &str,
        output: &crate::portfolio::PortfolioSimulationOutput,
    ) -> RearviewResult<WriteBatch> {
        validate_identifier(BACKTEST_RESULT_DATABASE)?;
        let batch = WriteBatch::from_output(run, result_attempt_id, output);
        let query_id = format!("rearview-backtest-write-{result_attempt_id}");
        self.write_split_result_fact_batch(
            BACKTEST_RESULT_DATABASE,
            "backtest",
            BACKTEST_RUN_ID_FIELD,
            &query_id,
            &batch,
        )
        .await?;
        Ok(batch)
    }

    pub async fn write_strategy_backtest_calculation_outputs(
        &self,
        result_attempt_id: &str,
        batch: &calculation_write::CalculationWriteBatch,
    ) -> RearviewResult<()> {
        validate_identifier(BACKTEST_RESULT_DATABASE)?;
        let query_id = format!("rearview-backtest-calculation-write-{result_attempt_id}");
        self.write_split_calculation_batch(
            BACKTEST_RESULT_DATABASE,
            "backtest",
            BACKTEST_RUN_ID_FIELD,
            &query_id,
            batch,
        )
        .await
    }

    pub async fn write_strategy_backtest_run_snapshot(
        &self,
        result_attempt_id: &str,
        run_snapshot: &portfolio_write::RunSnapshotRow,
    ) -> RearviewResult<()> {
        validate_identifier(BACKTEST_RESULT_DATABASE)?;
        let query_id = format!("rearview-backtest-write-{result_attempt_id}");
        self.insert_single_with_run_id_field(
            BACKTEST_RESULT_DATABASE,
            "backtest_run_snapshot",
            run_snapshot,
            &query_id,
            BACKTEST_RUN_ID_FIELD,
        )
        .await
    }

    pub async fn write_strategy_portfolio_live_result_facts(
        &self,
        run: &crate::postgres::PortfolioRunRecord,
        result_attempt_id: &str,
        output: &crate::portfolio::PortfolioSimulationOutput,
    ) -> RearviewResult<WriteBatch> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let batch = WriteBatch::from_output(run, result_attempt_id, output);
        let query_id = format!("rearview-live-write-{result_attempt_id}");
        self.write_split_result_fact_batch(database, "live", LIVE_RUN_ID_FIELD, &query_id, &batch)
            .await?;
        Ok(batch)
    }

    pub async fn write_strategy_portfolio_live_calculation_outputs(
        &self,
        result_attempt_id: &str,
        batch: &calculation_write::CalculationWriteBatch,
    ) -> RearviewResult<()> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let query_id = format!("rearview-live-calculation-write-{result_attempt_id}");
        self.write_split_calculation_batch(database, "live", LIVE_RUN_ID_FIELD, &query_id, batch)
            .await
    }

    pub async fn write_strategy_portfolio_live_run_snapshot(
        &self,
        result_attempt_id: &str,
        run_snapshot: &portfolio_write::RunSnapshotRow,
    ) -> RearviewResult<()> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let query_id = format!("rearview-live-write-{result_attempt_id}");
        self.insert_single_with_run_id_field(
            database,
            "live_run_snapshot",
            run_snapshot,
            &query_id,
            LIVE_RUN_ID_FIELD,
        )
        .await
    }

    async fn write_split_result_fact_batch(
        &self,
        database: &str,
        prefix: &str,
        run_id_field: &str,
        query_id: &str,
        batch: &WriteBatch,
    ) -> RearviewResult<()> {
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_target"),
            &batch.targets,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_order"),
            &batch.orders,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_trade"),
            &batch.trades,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_position_day"),
            &batch.positions,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_nav_daily"),
            &batch.nav,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_event"),
            &batch.events,
            query_id,
            run_id_field,
        )
        .await?;
        Ok(())
    }

    async fn write_split_calculation_batch(
        &self,
        database: &str,
        prefix: &str,
        run_id_field: &str,
        query_id: &str,
        batch: &calculation_write::CalculationWriteBatch,
    ) -> RearviewResult<()> {
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_performance_metric"),
            &batch.performance_metrics,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_performance_metric_status"),
            &batch.performance_metric_statuses,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_closed_trade"),
            &batch.closed_trades,
            query_id,
            run_id_field,
        )
        .await?;
        self.insert_rows_with_run_id_field(
            database,
            &format!("{prefix}_trade_metric"),
            &batch.trade_metrics,
            query_id,
            run_id_field,
        )
        .await?;
        Ok(())
    }

    pub async fn query_mart_benchmark_returns(
        &self,
        security_code: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<BenchmarkReturn>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;
        let database = quote_identifier(&self.config.marts_database);
        let security_code = quote_string_literal(security_code);
        let sql = format!(
            r#"
SELECT trade_date, return_daily
FROM {database}.`mart_benchmark_returns_daily`
WHERE security_code = {security_code}
  AND trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        Ok(parse_json_each_row::<BenchmarkReturnRow>(&body)?
            .into_iter()
            .map(|row| BenchmarkReturn {
                trade_date: row.trade_date,
                return_daily: row.return_daily,
            })
            .collect())
    }

    pub async fn query_mart_risk_free_rates(
        &self,
        source_tenor: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<RiskFreeRate>> {
        validate_identifier(&self.config.marts_database)?;
        validate_source_tenor(source_tenor)?;
        let database = quote_identifier(&self.config.marts_database);
        let source_tenor = quote_string_literal(source_tenor);
        let sql = format!(
            r#"
SELECT trade_date, daily_rate
FROM {database}.`mart_risk_free_rate_daily`
WHERE source_tenor = {source_tenor}
  AND trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        Ok(parse_json_each_row::<RiskFreeRateRow>(&body)?
            .into_iter()
            .map(|row| RiskFreeRate {
                trade_date: row.trade_date,
                daily_rate: row.daily_rate,
            })
            .collect())
    }

    pub async fn write_portfolio_calculation_outputs(
        &self,
        result_attempt_id: &str,
        batch: &calculation_write::CalculationWriteBatch,
    ) -> RearviewResult<()> {
        let database = &self.config.calculation_database;
        validate_identifier(database)?;
        let query_id = format!("rearview-portfolio-calculation-write-{result_attempt_id}");
        self.insert_rows(
            database,
            "calc_portfolio_performance_metric",
            &batch.performance_metrics,
            &query_id,
        )
        .await?;
        self.insert_rows(
            database,
            "calc_portfolio_performance_metric_status",
            &batch.performance_metric_statuses,
            &query_id,
        )
        .await?;
        self.insert_rows(
            database,
            "calc_portfolio_closed_trade",
            &batch.closed_trades,
            &query_id,
        )
        .await?;
        self.insert_rows(
            database,
            "calc_portfolio_trade_metric",
            &batch.trade_metrics,
            &query_id,
        )
        .await?;
        Ok(())
    }

    async fn insert_rows<T: serde::Serialize>(
        &self,
        database: &str,
        table: &str,
        rows: &[T],
        query_id: &str,
    ) -> RearviewResult<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let body = portfolio_write::to_json_each_row(rows)?;
        let sql = format!("INSERT INTO {database}.{table} FORMAT JSONEachRow");
        self.execute_insert(&sql, &body, query_id).await
    }

    async fn insert_rows_with_run_id_field<T: serde::Serialize>(
        &self,
        database: &str,
        table: &str,
        rows: &[T],
        query_id: &str,
        run_id_field: &str,
    ) -> RearviewResult<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let body = portfolio_write::to_json_each_row_with_run_id_field(rows, run_id_field)?;
        let sql = format!("INSERT INTO {database}.{table} FORMAT JSONEachRow");
        self.execute_insert(&sql, &body, query_id).await
    }

    async fn insert_single<T: serde::Serialize>(
        &self,
        database: &str,
        table: &str,
        row: &T,
        query_id: &str,
    ) -> RearviewResult<()> {
        let body = portfolio_write::to_json_each_row(std::slice::from_ref(row))?;
        let sql = format!("INSERT INTO {database}.{table} FORMAT JSONEachRow");
        self.execute_insert(&sql, &body, query_id).await
    }

    async fn insert_single_with_run_id_field<T: serde::Serialize>(
        &self,
        database: &str,
        table: &str,
        row: &T,
        query_id: &str,
        run_id_field: &str,
    ) -> RearviewResult<()> {
        let body = portfolio_write::to_json_each_row_with_run_id_field(
            std::slice::from_ref(row),
            run_id_field,
        )?;
        let sql = format!("INSERT INTO {database}.{table} FORMAT JSONEachRow");
        self.execute_insert(&sql, &body, query_id).await
    }

    async fn execute_insert(&self, sql: &str, body: &str, query_id: &str) -> RearviewResult<()> {
        let response = self
            .client
            .post(self.config.base_url())
            .query(&[("query_id", query_id), ("query", sql)])
            .basic_auth(&self.config.user, Some(&self.config.password))
            .body(body.to_string())
            .send()
            .await?;
        let status = response.status();
        let resp_body = response.text().await?;
        if !status.is_success() {
            return Err(RearviewError::ClickHouse(format!(
                "ClickHouse insert HTTP status {status}: {resp_body}"
            )));
        }
        Ok(())
    }

    pub async fn query_screening_rows(
        &self,
        sql: &str,
        query_id: &str,
    ) -> RearviewResult<Vec<ScreeningRow>> {
        let body = self
            .execute_text(&format!("{sql}\nFORMAT JSONEachRow"), query_id)
            .await?;
        parse_json_each_row::<ScreeningRow>(&body)
    }

    pub async fn query_backtest_signal_rows(
        &self,
        sql: &str,
        query_id: &str,
    ) -> RearviewResult<Vec<BacktestSignalRow>> {
        let body = self
            .execute_text(&format!("{sql}\nFORMAT JSONEachRow"), query_id)
            .await?;
        parse_json_each_row::<BacktestSignalRow>(&body)
    }

    pub async fn query_preview_timeline_rows(
        &self,
        sql: &str,
        query_id: &str,
    ) -> RearviewResult<Vec<PreviewTimelineRow>> {
        let body = self
            .execute_text(&format!("{sql}\nFORMAT JSONEachRow"), query_id)
            .await?;
        parse_json_each_row::<PreviewTimelineRow>(&body)
    }

    pub async fn query_security_display_rows(
        &self,
        security_codes: &[String],
        query_id: &str,
    ) -> RearviewResult<Vec<SecurityDisplayRow>> {
        validate_identifier(&self.config.marts_database)?;
        if security_codes.is_empty() {
            return Ok(Vec::new());
        }
        for security_code in security_codes {
            validate_security_code(security_code)?;
        }
        let securities = security_codes
            .iter()
            .map(|security_code| quote_string_literal(security_code))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            r#"
SELECT
    security_code,
    security_name,
    exchange_code,
    security_board
FROM {database}.`mart_stock_basic_snapshot`
WHERE security_code IN ({securities})
ORDER BY security_code ASC
FORMAT JSONEachRow"#,
            database = quote_identifier(&self.config.marts_database),
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<SecurityDisplayRow>(&body)
    }

    pub async fn query_trade_dates(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<NaiveDate>> {
        validate_identifier(&self.config.marts_database)?;
        let sql = trade_dates_sql(
            &quote_identifier(&self.config.marts_database),
            "mart_stock_quotes_daily",
            start_date,
            end_date,
        );
        let body = self.execute_text(&sql, query_id).await?;
        let mut trade_dates = Vec::new();
        for row in parse_json_each_row::<TradeDateRow>(&body)? {
            trade_dates.push(row.trade_date);
        }
        Ok(trade_dates)
    }

    pub async fn query_trade_calendar_dates(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<NaiveDate>> {
        validate_identifier(&self.config.marts_database)?;
        let sql = trade_dates_sql(
            &quote_identifier(&self.config.marts_database),
            "mart_trade_calendar",
            start_date,
            end_date,
        );
        let body = self.execute_text(&sql, query_id).await?;
        let mut trade_dates = Vec::new();
        for row in parse_json_each_row::<TradeDateRow>(&body)? {
            trade_dates.push(row.trade_date);
        }
        Ok(trade_dates)
    }

    pub async fn query_trade_date_lookback_start(
        &self,
        end_date: NaiveDate,
        lookback_trading_days: u32,
        query_id: &str,
    ) -> RearviewResult<Option<NaiveDate>> {
        validate_identifier(&self.config.marts_database)?;
        if lookback_trading_days == 0 {
            return Err(RearviewError::Validation(
                "lookback_trading_days must be greater than 0".to_string(),
            ));
        }

        let database = quote_identifier(&self.config.marts_database);
        let sql = format!(
            r#"
SELECT
    trade_date
FROM
(
    SELECT DISTINCT
        trade_date
    FROM {database}.`mart_stock_quotes_daily`
    PREWHERE trade_date <= toDate('{end_date}')
    ORDER BY trade_date DESC
    LIMIT {lookback_trading_days}
)
ORDER BY trade_date ASC
LIMIT 1
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        let mut rows = parse_json_each_row::<TradeDateRow>(&body)?;
        Ok(rows.pop().map(|row| row.trade_date))
    }

    pub async fn query_analysis_quote_rows(
        &self,
        security_code: &str,
        start_date: Option<NaiveDate>,
        end_date: NaiveDate,
        lookback_trading_days: u32,
        query_id: &str,
    ) -> RearviewResult<Vec<QuoteMartRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;
        if let Some(start_date) = start_date
            && start_date > end_date
        {
            return Err(RearviewError::Validation(
                "quote_start_date must be <= quote_end_date".to_string(),
            ));
        }
        if lookback_trading_days == 0 {
            return Err(RearviewError::Validation(
                "lookback_trading_days must be greater than 0".to_string(),
            ));
        }

        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let select = quote_select_columns();
        let sql = match start_date {
            Some(start_date) => format!(
                r#"
SELECT
{select}
FROM {database}.`mart_stock_quotes_daily`
PREWHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
WHERE security_code = {security_code}
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
            ),
            None => format!(
                r#"
SELECT
{select}
FROM {database}.`mart_stock_quotes_daily`
WHERE security_code = {security_code}
  AND trade_date <= toDate('{end_date}')
ORDER BY trade_date DESC
LIMIT {lookback_trading_days}
FORMAT JSONEachRow"#
            ),
        };
        let body = self.execute_text(&sql, query_id).await?;
        let mut rows = parse_json_each_row::<QuoteMartRow>(&body)?;
        if start_date.is_none() {
            rows.reverse();
        }
        Ok(rows)
    }

    pub async fn query_analysis_chart_quote_rows(
        &self,
        security_code: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        adjustment: AnalysisQuoteAdjustment,
        query_id: &str,
    ) -> RearviewResult<Vec<QuoteMartRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;
        if start_date > end_date {
            return Err(RearviewError::Validation(
                "quote_start_date must be <= quote_end_date".to_string(),
            ));
        }

        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let select = chart_quote_select_columns(adjustment);
        let sql = format!(
            r#"
SELECT
{select}
FROM {database}.`mart_stock_quotes_daily`
PREWHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
WHERE security_code = {security_code}
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<QuoteMartRow>(&body)
    }

    pub async fn query_chart_context_chart_quote_rows(
        &self,
        security_code: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        adjustment: AnalysisQuoteAdjustment,
        query_id: &str,
    ) -> RearviewResult<Vec<QuoteMartRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;
        if start_date > end_date {
            return Err(RearviewError::Validation(
                "quote_start_date must be <= quote_end_date".to_string(),
            ));
        }

        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let select = chart_context_chart_quote_select_columns(adjustment);
        let sql = format!(
            r#"
SELECT
{select}
FROM {database}.`mart_stock_quotes_daily`
PREWHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
WHERE security_code = {security_code}
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<QuoteMartRow>(&body)
    }

    pub async fn query_analysis_selected_quote_row(
        &self,
        security_code: &str,
        trade_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Option<QuoteMartRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;

        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let select = quote_select_columns();
        let sql = format!(
            r#"
SELECT
{select}
FROM {database}.`mart_stock_quotes_daily`
PREWHERE trade_date = toDate('{trade_date}')
WHERE security_code = {security_code}
LIMIT 1
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        let mut rows = parse_json_each_row::<QuoteMartRow>(&body)?;
        Ok(rows.pop())
    }

    pub async fn query_chart_context_selected_quote_row(
        &self,
        security_code: &str,
        trade_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Option<QuoteMartRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;

        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let select = chart_context_selected_quote_select_columns();
        let sql = format!(
            r#"
SELECT
{select}
FROM {database}.`mart_stock_quotes_daily`
PREWHERE trade_date = toDate('{trade_date}')
WHERE security_code = {security_code}
LIMIT 1
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        let mut rows = parse_json_each_row::<QuoteMartRow>(&body)?;
        Ok(rows.pop())
    }

    pub async fn query_analysis_trend_rows(
        &self,
        security_code: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<TrendIndicatorRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;
        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let select = trend_select_columns();
        let sql = format!(
            r#"
SELECT
{select}
FROM {database}.`mart_stock_trend_indicator_daily`
PREWHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
WHERE security_code = {security_code}
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<TrendIndicatorRow>(&body)
    }

    pub async fn query_chart_context_trend_rows(
        &self,
        security_code: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<TrendIndicatorRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;
        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let select = chart_context_trend_select_columns();
        let sql = format!(
            r#"
SELECT
{select}
FROM {database}.`mart_stock_trend_indicator_daily`
PREWHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
WHERE security_code = {security_code}
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<TrendIndicatorRow>(&body)
    }

    pub async fn query_analysis_momentum_rows(
        &self,
        security_code: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<MomentumIndicatorRow>> {
        validate_identifier(&self.config.marts_database)?;
        validate_security_code(security_code)?;
        let security_code = quote_string_literal(security_code);
        let database = quote_identifier(&self.config.marts_database);
        let sql = format!(
            r#"
SELECT
    security_code,
    trade_date,
    rsi_6,
    rsi_12,
    rsi_24,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value
FROM {database}.`mart_stock_momentum_indicator_daily`
PREWHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
WHERE security_code = {security_code}
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<MomentumIndicatorRow>(&body)
    }

    pub async fn query_portfolio_price_bars(
        &self,
        security_codes: &[String],
        start_date: NaiveDate,
        end_date: NaiveDate,
        indicator_metrics: &[String],
        query_id: &str,
    ) -> RearviewResult<Vec<PriceBar>> {
        validate_identifier(&self.config.marts_database)?;
        if start_date > end_date {
            return Err(RearviewError::Validation(
                "start_date must be <= end_date".to_string(),
            ));
        }
        if security_codes.is_empty() {
            return Ok(Vec::new());
        }
        for security_code in security_codes {
            validate_security_code(security_code)?;
        }
        let database = quote_identifier(&self.config.marts_database);
        let securities = security_codes
            .iter()
            .map(|security_code| quote_string_literal(security_code))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = portfolio_price_bars_sql(
            &database,
            &securities,
            start_date,
            end_date,
            indicator_metrics,
        )?;
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<PriceBar>(&body)
    }

    pub async fn query_portfolio_price_bars_for_demand(
        &self,
        demand: &MarketDataDemand,
        indicator_metrics: &[String],
        query_id: &str,
    ) -> RearviewResult<Vec<PriceBar>> {
        validate_identifier(&self.config.marts_database)?;
        if demand.is_empty() {
            return Ok(Vec::new());
        }
        for entry in &demand.entries {
            validate_security_code(&entry.security_code)?;
        }
        let database = quote_identifier(&self.config.marts_database);
        let sql = portfolio_price_bars_demand_join_sql(&database, demand, indicator_metrics)?;
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<PriceBar>(&body)
    }

    // ---- Portfolio result reads (Phase 5: API read source switch) ----

    pub async fn query_strategy_backtest_nav(
        &self,
        strategy_backtest_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioNavRecord>> {
        self.query_split_nav(
            BACKTEST_RESULT_DATABASE,
            "backtest_nav_daily",
            BACKTEST_RUN_ID_FIELD,
            strategy_backtest_run_id,
            result_attempt_id,
            "rearview-backtest-read-nav",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_nav(
        &self,
        strategy_portfolio_daily_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioNavRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_nav(
            database,
            "live_nav_daily",
            LIVE_RUN_ID_FIELD,
            strategy_portfolio_daily_run_id,
            result_attempt_id,
            "rearview-live-read-nav",
        )
        .await
    }

    async fn query_split_nav(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        portfolio_run_id: &str,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioNavRecord>> {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = format!(
            r#"
SELECT {run_id_field} AS portfolio_run_id, trade_date, cash_balance, position_market_value,
       total_equity, nav, daily_return, drawdown, gross_exposure,
       position_count, turnover, fee_amount, warning_count
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
ORDER BY trade_date
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, portfolio_run_id, result_attempt_id),
            )
            .await?;
        parse_json_each_row(&body)
    }

    pub async fn query_strategy_portfolio_virtual_account(
        &self,
        strategy_portfolio_daily_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<StrategyPortfolioVirtualAccountRecord> {
        let database = &self.config.portfolio_database;
        self.query_split_virtual_account(
            database,
            LIVE_RUN_ID_FIELD,
            strategy_portfolio_daily_run_id,
            result_attempt_id,
        )
        .await
    }

    async fn query_split_virtual_account(
        &self,
        database: &str,
        run_id_field: &str,
        portfolio_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<StrategyPortfolioVirtualAccountRecord> {
        validate_identifier(database)?;
        validate_identifier(run_id_field)?;
        let quoted_database = quote_identifier(database);
        let quoted_run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let nav_sql =
            virtual_account_nav_sql(&quoted_database, &quoted_run_id_field, &run_id, &attempt);
        let nav_body = self
            .execute_text(
                &nav_sql,
                &portfolio_read_query_id(
                    "rearview-live-read-virtual-account-nav",
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let nav_rows: Vec<VirtualAccountNavRow> = parse_json_each_row(&nav_body)?;
        let Some(latest_nav) = nav_rows.first() else {
            return Err(RearviewError::NotFound(format!(
                "no live nav rows found for strategy portfolio daily run {portfolio_run_id}"
            )));
        };
        let previous_nav = nav_rows.get(1);
        let holding_sql = virtual_account_holding_sql(
            &quoted_database,
            &quoted_run_id_field,
            &run_id,
            &attempt,
            latest_nav.trade_date,
        );
        let holding_body = self
            .execute_text(
                &holding_sql,
                &portfolio_read_query_id(
                    "rearview-live-read-virtual-account-holding",
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let holding_unrealized_pnl =
            parse_json_each_row::<VirtualAccountHoldingRow>(&holding_body)?
                .into_iter()
                .next()
                .map_or(0.0, |row| row.holding_unrealized_pnl);

        Ok(virtual_account_record(
            latest_nav,
            previous_nav,
            holding_unrealized_pnl,
        ))
    }

    pub async fn query_strategy_portfolio_statement_summary(
        &self,
        strategy_portfolio_daily_run_id: &str,
        result_attempt_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> RearviewResult<StrategyPortfolioStatementSummaryRecord> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let quoted_database = quote_identifier(database);
        let quoted_run_id_field = quote_identifier(LIVE_RUN_ID_FIELD);
        let run_id = quote_string_literal(strategy_portfolio_daily_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = statement_summary_sql(
            &quoted_database,
            &quoted_run_id_field,
            &run_id,
            &attempt,
            start_date,
            end_date,
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-live-read-statement-summary",
                    strategy_portfolio_daily_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        parse_json_each_row::<StrategyPortfolioStatementSummaryRecord>(&body)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                RearviewError::NotFound(format!(
                    "no statement summary row found for strategy portfolio daily run {strategy_portfolio_daily_run_id}"
                ))
            })
    }

    pub async fn query_strategy_portfolio_statement_operations(
        &self,
        strategy_portfolio_daily_run_id: &str,
        result_attempt_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        page: crate::postgres::Page,
    ) -> RearviewResult<crate::postgres::ListResult<StrategyPortfolioStatementOperationRecord>>
    {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let quoted_database = quote_identifier(database);
        let quoted_run_id_field = quote_identifier(LIVE_RUN_ID_FIELD);
        let run_id = quote_string_literal(strategy_portfolio_daily_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = statement_operations_sql(
            &quoted_database,
            &quoted_run_id_field,
            &run_id,
            &attempt,
            start_date,
            end_date,
            page,
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-live-read-statement-operations",
                    strategy_portfolio_daily_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let rows = parse_json_each_row::<StrategyPortfolioStatementOperationRecord>(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, page))
    }

    pub async fn query_strategy_backtest_targets(
        &self,
        filter: &crate::postgres::PortfolioTargetFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTargetRecord>> {
        self.query_split_targets(
            BACKTEST_RESULT_DATABASE,
            "backtest_target",
            BACKTEST_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-backtest-read-targets",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_targets(
        &self,
        filter: &crate::postgres::PortfolioTargetFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTargetRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_targets(
            database,
            "live_target",
            LIVE_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-live-read-targets",
        )
        .await
    }

    async fn query_split_targets(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        filter: &crate::postgres::PortfolioTargetFilter,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTargetRecord>> {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let signal_filter = match filter.signal_date {
            Some(date) => format!("AND signal_date = toDate('{date}')"),
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT {run_id_field} AS portfolio_run_id, signal_date, execution_date, security_code,
       source_rank, source_score, target_weight, target_amount,
       target_quantity, target_reason
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  {signal_filter}
ORDER BY signal_date, source_rank, security_code
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, &filter.portfolio_run_id, result_attempt_id),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioTargetRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_strategy_backtest_latest_targets(
        &self,
        strategy_backtest_run_id: &str,
        result_attempt_id: &str,
        limit: usize,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioTargetRecord>> {
        self.query_split_latest_targets(
            SplitTargetFamily {
                database: BACKTEST_RESULT_DATABASE,
                table: "backtest_target",
                run_id_field: BACKTEST_RUN_ID_FIELD,
                query_prefix: "rearview-backtest-read-latest-targets",
            },
            strategy_backtest_run_id,
            result_attempt_id,
            limit,
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_latest_targets(
        &self,
        strategy_portfolio_daily_run_id: &str,
        result_attempt_id: &str,
        limit: usize,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioTargetRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_latest_targets(
            SplitTargetFamily {
                database,
                table: "live_target",
                run_id_field: LIVE_RUN_ID_FIELD,
                query_prefix: "rearview-live-read-latest-targets",
            },
            strategy_portfolio_daily_run_id,
            result_attempt_id,
            limit,
        )
        .await
    }

    async fn query_split_latest_targets(
        &self,
        family: SplitTargetFamily<'_>,
        portfolio_run_id: &str,
        result_attempt_id: &str,
        limit: usize,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioTargetRecord>> {
        validate_identifier(family.database)?;
        validate_identifier(family.table)?;
        validate_identifier(family.run_id_field)?;
        let database = quote_identifier(family.database);
        let table = quote_identifier(family.table);
        let run_id_field = quote_identifier(family.run_id_field);
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = format!(
            r#"
WITH latest_signal_date AS (
    SELECT max(signal_date) AS signal_date
    FROM {database}.{table}
    WHERE {run_id_field} = {run_id}
      AND result_attempt_id = {attempt}
)
SELECT {run_id_field} AS portfolio_run_id, signal_date, execution_date, security_code,
       source_rank, source_score, target_weight, target_amount,
       target_quantity, target_reason
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  AND signal_date = (SELECT signal_date FROM latest_signal_date)
ORDER BY source_rank, security_code
LIMIT {limit}
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(family.query_prefix, portfolio_run_id, result_attempt_id),
            )
            .await?;
        parse_json_each_row(&body)
    }

    pub async fn query_strategy_backtest_orders(
        &self,
        filter: &crate::postgres::PortfolioOrderFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioOrderRecord>> {
        self.query_split_orders(
            BACKTEST_RESULT_DATABASE,
            "backtest_order",
            BACKTEST_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-backtest-read-orders",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_orders(
        &self,
        filter: &crate::postgres::PortfolioOrderFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioOrderRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_orders(
            database,
            "live_order",
            LIVE_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-live-read-orders",
        )
        .await
    }

    async fn query_split_orders(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        filter: &crate::postgres::PortfolioOrderFilter,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioOrderRecord>> {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let exec_filter = match filter.execution_date {
            Some(date) => format!("AND execution_date = toDate('{date}')"),
            None => String::new(),
        };
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_order_id, {run_id_field} AS portfolio_run_id, order_seq, signal_date,
       execution_date, security_code, side, order_quantity, order_amount,
       reference_price, reason, status, event_ref
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  {exec_filter}
  {code_filter}
ORDER BY execution_date, order_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, &filter.portfolio_run_id, result_attempt_id),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioOrderRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_strategy_backtest_trades(
        &self,
        filter: &crate::postgres::PortfolioTradeFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeRecord>> {
        self.query_split_trades(
            BACKTEST_RESULT_DATABASE,
            "backtest_trade",
            BACKTEST_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-backtest-read-trades",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_trades(
        &self,
        filter: &crate::postgres::PortfolioTradeFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_trades(
            database,
            "live_trade",
            LIVE_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-live-read-trades",
        )
        .await
    }

    async fn query_split_trades(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        filter: &crate::postgres::PortfolioTradeFilter,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeRecord>> {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let date_filter = match filter.trade_date {
            Some(date) => format!("AND trade_date = toDate('{date}')"),
            None => String::new(),
        };
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_trade_id, {run_id_field} AS portfolio_run_id, trade_seq, portfolio_order_id,
       trade_date, signal_date, security_code, side, quantity, reference_price,
       execution_price, gross_amount, commission, stamp_duty, transfer_fee,
       total_fee, slippage_cost, reason
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  {date_filter}
  {code_filter}
ORDER BY trade_date, trade_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, &filter.portfolio_run_id, result_attempt_id),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioTradeRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_strategy_backtest_rebalance_trade_counts(
        &self,
        strategy_backtest_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioRebalanceTradeCountRecord>> {
        self.query_split_rebalance_trade_counts(
            BACKTEST_RESULT_DATABASE,
            "backtest_trade",
            BACKTEST_RUN_ID_FIELD,
            strategy_backtest_run_id,
            result_attempt_id,
            "rearview-backtest-read-rebalance-counts",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_rebalance_trade_counts(
        &self,
        strategy_portfolio_daily_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioRebalanceTradeCountRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_rebalance_trade_counts(
            database,
            "live_trade",
            LIVE_RUN_ID_FIELD,
            strategy_portfolio_daily_run_id,
            result_attempt_id,
            "rearview-live-read-rebalance-counts",
        )
        .await
    }

    async fn query_split_rebalance_trade_counts(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        portfolio_run_id: &str,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioRebalanceTradeCountRecord>> {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = format!(
            r#"
SELECT
    trade_date,
    toInt32(countDistinctIf(security_code, lower(side) = 'buy')) AS buy_count,
    toInt32(countDistinctIf(security_code, lower(side) = 'sell')) AS sell_count
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
GROUP BY trade_date
ORDER BY trade_date
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, portfolio_run_id, result_attempt_id),
            )
            .await?;
        parse_json_each_row(&body)
    }

    pub async fn query_strategy_backtest_positions(
        &self,
        filter: &crate::postgres::PortfolioPositionFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioPositionRecord>> {
        self.query_split_positions(
            BACKTEST_RESULT_DATABASE,
            "backtest_position_day",
            BACKTEST_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-backtest-read-positions",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_positions(
        &self,
        filter: &crate::postgres::PortfolioPositionFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioPositionRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_positions(
            database,
            "live_position_day",
            LIVE_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-live-read-positions",
        )
        .await
    }

    async fn query_split_positions(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        filter: &crate::postgres::PortfolioPositionFilter,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioPositionRecord>> {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let quoted_database = quote_identifier(database);
        let quoted_table = quote_identifier(table);
        let quoted_run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);

        let trade_date = match filter.trade_date {
            Some(date) => Some(date),
            None => {
                let max_sql = format!(
                    r#"
SELECT max(trade_date) AS max_date
FROM {quoted_database}.{quoted_table}
WHERE {quoted_run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
FORMAT JSONEachRow"#
                );
                let body = self
                    .execute_text(
                        &max_sql,
                        &portfolio_read_query_id(
                            query_prefix,
                            &filter.portfolio_run_id,
                            result_attempt_id,
                        ),
                    )
                    .await?;
                #[derive(Deserialize)]
                struct MaxDateRow {
                    max_date: Option<NaiveDate>,
                }
                let rows: Vec<MaxDateRow> = parse_json_each_row(&body)?;
                rows.into_iter().next().and_then(|r| r.max_date)
            }
        };

        let date_filter = match trade_date {
            Some(date) => format!("AND trade_date = toDate('{date}')"),
            None => String::new(),
        };
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT {quoted_run_id_field} AS portfolio_run_id, trade_date, security_code, quantity, cost_basis,
       average_entry_price, close_price, market_value, unrealized_pnl,
       unrealized_return, holding_days, is_stale_price
FROM {quoted_database}.{quoted_table}
WHERE {quoted_run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  {date_filter}
  {code_filter}
ORDER BY trade_date, security_code
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, &filter.portfolio_run_id, result_attempt_id),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioPositionRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_strategy_backtest_events(
        &self,
        filter: &crate::postgres::PortfolioEventFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioEventRecord>> {
        self.query_split_events(
            BACKTEST_RESULT_DATABASE,
            "backtest_event",
            BACKTEST_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-backtest-read-events",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_events(
        &self,
        filter: &crate::postgres::PortfolioEventFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioEventRecord>> {
        let database = &self.config.portfolio_database;
        self.query_split_events(
            database,
            "live_event",
            LIVE_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-live-read-events",
        )
        .await
    }

    async fn query_split_events(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        filter: &crate::postgres::PortfolioEventFilter,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioEventRecord>> {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let date_filter = match filter.trade_date {
            Some(date) => format!("AND trade_date = toDate('{date}')"),
            None => String::new(),
        };
        let type_filter = match &filter.event_type {
            Some(et) => format!("AND event_type = {}", quote_string_literal(et)),
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_event_id, {run_id_field} AS portfolio_run_id, event_seq, trade_date, security_code,
       event_type, severity, message, payload
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  {date_filter}
  {type_filter}
ORDER BY event_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, &filter.portfolio_run_id, result_attempt_id),
            )
            .await?;
        #[derive(Deserialize)]
        struct ChEventRow {
            portfolio_event_id: String,
            portfolio_run_id: String,
            event_seq: i32,
            trade_date: Option<NaiveDate>,
            security_code: Option<String>,
            event_type: String,
            severity: String,
            message: String,
            payload: String,
        }
        let ch_rows: Vec<ChEventRow> = parse_json_each_row(&body)?;
        let rows: Vec<crate::postgres::PortfolioEventRecord> = ch_rows
            .into_iter()
            .map(|r| crate::postgres::PortfolioEventRecord {
                portfolio_event_id: r.portfolio_event_id,
                portfolio_run_id: r.portfolio_run_id,
                event_seq: r.event_seq,
                trade_date: r.trade_date,
                security_code: r.security_code,
                event_type: r.event_type,
                severity: r.severity,
                message: r.message,
                payload: serde_json::from_str(&r.payload).unwrap_or_else(|_| serde_json::json!({})),
            })
            .collect();
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_strategy_backtest_performance(
        &self,
        strategy_backtest_run_id: &str,
        result_attempt_id: &str,
        security_code: &str,
        window_key: &str,
    ) -> RearviewResult<crate::postgres::PortfolioPerformanceResponse> {
        self.query_split_performance(
            SplitPerformanceFamily {
                database: BACKTEST_RESULT_DATABASE,
                prefix: "backtest",
                run_id_field: BACKTEST_RUN_ID_FIELD,
                query_prefix: "rearview-backtest-read-performance",
            },
            strategy_backtest_run_id,
            result_attempt_id,
            security_code,
            window_key,
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_performance(
        &self,
        strategy_portfolio_daily_run_id: &str,
        result_attempt_id: &str,
        security_code: &str,
        window_key: &str,
    ) -> RearviewResult<crate::postgres::PortfolioPerformanceResponse> {
        let database = &self.config.portfolio_database;
        self.query_split_performance(
            SplitPerformanceFamily {
                database,
                prefix: "live",
                run_id_field: LIVE_RUN_ID_FIELD,
                query_prefix: "rearview-live-read-performance",
            },
            strategy_portfolio_daily_run_id,
            result_attempt_id,
            security_code,
            window_key,
        )
        .await
    }

    async fn query_split_performance(
        &self,
        family: SplitPerformanceFamily<'_>,
        portfolio_run_id: &str,
        result_attempt_id: &str,
        security_code: &str,
        window_key: &str,
    ) -> RearviewResult<crate::postgres::PortfolioPerformanceResponse> {
        validate_identifier(family.database)?;
        validate_identifier(family.prefix)?;
        validate_identifier(family.run_id_field)?;
        validate_security_code(security_code)?;
        validate_window_key(window_key)?;
        let database = quote_identifier(family.database);
        let run_id_field = quote_identifier(family.run_id_field);
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let security_code = quote_string_literal(security_code);
        let window_key = quote_string_literal(window_key);
        let metric_table = quote_identifier(&format!("{}_performance_metric", family.prefix));
        let status_table =
            quote_identifier(&format!("{}_performance_metric_status", family.prefix));
        let sql = format!(
            r#"
SELECT {run_id_field} AS portfolio_run_id, result_attempt_id, security_code, window_key,
       window_start, window_end, config_hash, metric_status, observation_count,
       holding_period_return, annualized_return, annualized_volatility,
       max_drawdown, calmar_ratio, downside_deviation, sortino_ratio,
       sharpe_ratio, information_ratio, beta, alpha, treynor_ratio
FROM {database}.{metric_table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  AND security_code = {security_code}
  AND window_key = {window_key}
LIMIT 1
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(family.query_prefix, portfolio_run_id, result_attempt_id),
            )
            .await?;
        let metric =
            parse_json_each_row::<crate::postgres::PortfolioPerformanceMetricRecord>(&body)?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    RearviewError::NotFound(format!(
                        "portfolio performance metric not found for run {portfolio_run_id} attempt {result_attempt_id}"
                    ))
                })?;

        let status_sql = format!(
            r#"
SELECT {run_id_field} AS portfolio_run_id, result_attempt_id, security_code, window_key,
       metric_name, metric_status, reason_code
FROM {database}.{status_table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  AND security_code = {security_code}
  AND window_key = {window_key}
ORDER BY metric_name
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &status_sql,
                &portfolio_read_query_id(
                    &format!("{}-status", family.query_prefix),
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let statuses =
            parse_json_each_row::<crate::postgres::PortfolioPerformanceMetricStatusRecord>(&body)?;
        Ok(crate::postgres::PortfolioPerformanceResponse { metric, statuses })
    }

    pub async fn query_strategy_backtest_closed_trades(
        &self,
        filter: &crate::postgres::PortfolioClosedTradeFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioClosedTradeRecord>>
    {
        self.query_split_closed_trades(
            BACKTEST_RESULT_DATABASE,
            "backtest_closed_trade",
            BACKTEST_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-backtest-read-closed-trades",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_closed_trades(
        &self,
        filter: &crate::postgres::PortfolioClosedTradeFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioClosedTradeRecord>>
    {
        let database = &self.config.portfolio_database;
        self.query_split_closed_trades(
            database,
            "live_closed_trade",
            LIVE_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-live-read-closed-trades",
        )
        .await
    }

    async fn query_split_closed_trades(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        filter: &crate::postgres::PortfolioClosedTradeFilter,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioClosedTradeRecord>>
    {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let exit_date_filter = match filter.exit_date {
            Some(date) => format!("AND exit_date = toDate('{date}')"),
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT {run_id_field} AS portfolio_run_id, result_attempt_id, closed_trade_id, closed_trade_seq,
       position_lot_id, entry_trade_seq, exit_trade_seq, security_code,
       entry_date, exit_date, quantity, entry_gross_amount, exit_gross_amount,
       entry_fee, exit_fee, entry_fee + exit_fee AS total_fee, realized_pnl,
       if(entry_gross_amount + entry_fee = 0, null, realized_pnl / (entry_gross_amount + entry_fee)) AS realized_return,
       holding_days, exit_reason
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  {exit_date_filter}
  {code_filter}
ORDER BY exit_date, security_code, closed_trade_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, &filter.portfolio_run_id, result_attempt_id),
            )
            .await?;
        let rows = parse_json_each_row::<crate::postgres::PortfolioClosedTradeRecord>(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_strategy_backtest_trade_metrics(
        &self,
        filter: &crate::postgres::PortfolioTradeMetricFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeMetricRecord>>
    {
        self.query_split_trade_metrics(
            BACKTEST_RESULT_DATABASE,
            "backtest_trade_metric",
            BACKTEST_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-backtest-read-trade-metrics",
        )
        .await
    }

    pub async fn query_strategy_portfolio_live_trade_metrics(
        &self,
        filter: &crate::postgres::PortfolioTradeMetricFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeMetricRecord>>
    {
        let database = &self.config.portfolio_database;
        self.query_split_trade_metrics(
            database,
            "live_trade_metric",
            LIVE_RUN_ID_FIELD,
            filter,
            result_attempt_id,
            "rearview-live-read-trade-metrics",
        )
        .await
    }

    async fn query_split_trade_metrics(
        &self,
        database: &str,
        table: &str,
        run_id_field: &str,
        filter: &crate::postgres::PortfolioTradeMetricFilter,
        result_attempt_id: &str,
        query_prefix: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeMetricRecord>>
    {
        validate_identifier(database)?;
        validate_identifier(table)?;
        validate_identifier(run_id_field)?;
        let database = quote_identifier(database);
        let table = quote_identifier(table);
        let run_id_field = quote_identifier(run_id_field);
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let window_filter = match &filter.window_key {
            Some(window_key) => {
                validate_window_key(window_key)?;
                format!("AND window_key = {}", quote_string_literal(window_key))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT {run_id_field} AS portfolio_run_id, result_attempt_id, window_key, window_start, window_end,
       closed_trade_count, winning_trade_count, losing_trade_count,
       breakeven_trade_count, win_rate_closed_trades, average_win_return,
       average_loss_return, profit_loss_ratio, average_holding_days,
       largest_win_return, largest_loss_return
FROM {database}.{table}
WHERE {run_id_field} = {run_id}
  AND result_attempt_id = {attempt}
  {window_filter}
ORDER BY window_key
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(query_prefix, &filter.portfolio_run_id, result_attempt_id),
            )
            .await?;
        let rows = parse_json_each_row::<crate::postgres::PortfolioTradeMetricRecord>(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_portfolio_nav(
        &self,
        portfolio_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioNavRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = format!(
            r#"
SELECT portfolio_run_id, trade_date, cash_balance, position_market_value,
       total_equity, nav, daily_return, drawdown, gross_exposure,
       position_count, turnover, fee_amount, warning_count
FROM {database}.portfolio_nav_daily
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
ORDER BY trade_date
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-nav",
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        parse_json_each_row(&body)
    }

    pub async fn query_portfolio_targets(
        &self,
        filter: &crate::postgres::PortfolioTargetFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTargetRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let signal_filter = match filter.signal_date {
            Some(date) => format!("AND signal_date = toDate('{date}')"),
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_run_id, signal_date, execution_date, security_code,
       source_rank, source_score, target_weight, target_amount,
       target_quantity, target_reason
FROM {database}.portfolio_target
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  {signal_filter}
ORDER BY signal_date, source_rank, security_code
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-targets",
                    &filter.portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioTargetRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_portfolio_latest_targets(
        &self,
        portfolio_run_id: &str,
        result_attempt_id: &str,
        limit: usize,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioTargetRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = format!(
            r#"
WITH latest_signal_date AS (
    SELECT max(signal_date) AS signal_date
    FROM {database}.portfolio_target
    WHERE portfolio_run_id = {run_id}
      AND result_attempt_id = {attempt}
)
SELECT portfolio_run_id, signal_date, execution_date, security_code,
       source_rank, source_score, target_weight, target_amount,
       target_quantity, target_reason
FROM {database}.portfolio_target
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  AND signal_date = (SELECT signal_date FROM latest_signal_date)
ORDER BY source_rank, security_code
LIMIT {limit}
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-latest-targets",
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        parse_json_each_row(&body)
    }

    pub async fn query_portfolio_orders(
        &self,
        filter: &crate::postgres::PortfolioOrderFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioOrderRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let exec_filter = match filter.execution_date {
            Some(date) => format!("AND execution_date = toDate('{date}')"),
            None => String::new(),
        };
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_order_id, portfolio_run_id, order_seq, signal_date,
       execution_date, security_code, side, order_quantity, order_amount,
       reference_price, reason, status, event_ref
FROM {database}.portfolio_order
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  {exec_filter}
  {code_filter}
ORDER BY execution_date, order_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-orders",
                    &filter.portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioOrderRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_portfolio_trades(
        &self,
        filter: &crate::postgres::PortfolioTradeFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let date_filter = match filter.trade_date {
            Some(date) => format!("AND trade_date = toDate('{date}')"),
            None => String::new(),
        };
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_trade_id, portfolio_run_id, trade_seq, portfolio_order_id,
       trade_date, signal_date, security_code, side, quantity, reference_price,
       execution_price, gross_amount, commission, stamp_duty, transfer_fee,
       total_fee, slippage_cost, reason
FROM {database}.portfolio_trade
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  {date_filter}
  {code_filter}
ORDER BY trade_date, trade_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-trades",
                    &filter.portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioTradeRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_portfolio_rebalance_trade_counts(
        &self,
        portfolio_run_id: &str,
        result_attempt_id: &str,
    ) -> RearviewResult<Vec<crate::postgres::PortfolioRebalanceTradeCountRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let sql = format!(
            r#"
SELECT
    trade_date,
    toInt32(countDistinctIf(security_code, lower(side) = 'buy')) AS buy_count,
    toInt32(countDistinctIf(security_code, lower(side) = 'sell')) AS sell_count
FROM {database}.portfolio_trade
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
GROUP BY trade_date
ORDER BY trade_date
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-rebalance-counts",
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        parse_json_each_row(&body)
    }

    pub async fn query_portfolio_positions(
        &self,
        filter: &crate::postgres::PortfolioPositionFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioPositionRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);

        let trade_date = match filter.trade_date {
            Some(date) => Some(date),
            None => {
                let max_sql = format!(
                    r#"
SELECT max(trade_date) AS max_date
FROM {database}.portfolio_position_day
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
FORMAT JSONEachRow"#
                );
                let body = self
                    .execute_text(
                        &max_sql,
                        &portfolio_read_query_id(
                            "rearview-portfolio-read-max-date",
                            &filter.portfolio_run_id,
                            result_attempt_id,
                        ),
                    )
                    .await?;
                #[derive(Deserialize)]
                struct MaxDateRow {
                    max_date: Option<NaiveDate>,
                }
                let rows: Vec<MaxDateRow> = parse_json_each_row(&body)?;
                rows.into_iter().next().and_then(|r| r.max_date)
            }
        };

        let date_filter = match trade_date {
            Some(date) => format!("AND trade_date = toDate('{date}')"),
            None => String::new(),
        };
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_run_id, trade_date, security_code, quantity, cost_basis,
       average_entry_price, close_price, market_value, unrealized_pnl,
       unrealized_return, holding_days, is_stale_price
FROM {database}.portfolio_position_day
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  {date_filter}
  {code_filter}
ORDER BY trade_date, security_code
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-positions",
                    &filter.portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let rows: Vec<crate::postgres::PortfolioPositionRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_portfolio_events(
        &self,
        filter: &crate::postgres::PortfolioEventFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioEventRecord>> {
        let database = &self.config.portfolio_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let date_filter = match filter.trade_date {
            Some(date) => format!("AND trade_date = toDate('{date}')"),
            None => String::new(),
        };
        let type_filter = match &filter.event_type {
            Some(et) => format!("AND event_type = {}", quote_string_literal(et)),
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_event_id, portfolio_run_id, event_seq, trade_date, security_code,
       event_type, severity, message, payload
FROM {database}.portfolio_event
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  {date_filter}
  {type_filter}
ORDER BY event_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-events",
                    &filter.portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        // payload is stored as String in ClickHouse; parse it back to Value.
        #[derive(Deserialize)]
        struct ChEventRow {
            portfolio_event_id: String,
            portfolio_run_id: String,
            event_seq: i32,
            trade_date: Option<NaiveDate>,
            security_code: Option<String>,
            event_type: String,
            severity: String,
            message: String,
            payload: String,
        }
        let ch_rows: Vec<ChEventRow> = parse_json_each_row(&body)?;
        let rows: Vec<crate::postgres::PortfolioEventRecord> = ch_rows
            .into_iter()
            .map(|r| crate::postgres::PortfolioEventRecord {
                portfolio_event_id: r.portfolio_event_id,
                portfolio_run_id: r.portfolio_run_id,
                event_seq: r.event_seq,
                trade_date: r.trade_date,
                security_code: r.security_code,
                event_type: r.event_type,
                severity: r.severity,
                message: r.message,
                payload: serde_json::from_str(&r.payload).unwrap_or_else(|_| serde_json::json!({})),
            })
            .collect();
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_portfolio_performance(
        &self,
        portfolio_run_id: &str,
        result_attempt_id: &str,
        security_code: &str,
        window_key: &str,
    ) -> RearviewResult<crate::postgres::PortfolioPerformanceResponse> {
        let database = &self.config.calculation_database;
        validate_identifier(database)?;
        validate_security_code(security_code)?;
        validate_window_key(window_key)?;
        let run_id = quote_string_literal(portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let security_code = quote_string_literal(security_code);
        let window_key = quote_string_literal(window_key);
        let sql = format!(
            r#"
SELECT portfolio_run_id, result_attempt_id, security_code, window_key,
       window_start, window_end, config_hash, metric_status, observation_count,
       holding_period_return, annualized_return, annualized_volatility,
       max_drawdown, calmar_ratio, downside_deviation, sortino_ratio,
       sharpe_ratio, information_ratio, beta, alpha, treynor_ratio
FROM {database}.calc_portfolio_performance_metric
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  AND security_code = {security_code}
  AND window_key = {window_key}
LIMIT 1
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-performance",
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let metric =
            parse_json_each_row::<crate::postgres::PortfolioPerformanceMetricRecord>(&body)?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    RearviewError::NotFound(format!(
                        "portfolio performance metric not found for run {portfolio_run_id} attempt {result_attempt_id}"
                    ))
                })?;

        let status_sql = format!(
            r#"
SELECT portfolio_run_id, result_attempt_id, security_code, window_key,
       metric_name, metric_status, reason_code
FROM {database}.calc_portfolio_performance_metric_status
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  AND security_code = {security_code}
  AND window_key = {window_key}
ORDER BY metric_name
FORMAT JSONEachRow"#
        );
        let body = self
            .execute_text(
                &status_sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-performance-status",
                    portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let statuses =
            parse_json_each_row::<crate::postgres::PortfolioPerformanceMetricStatusRecord>(&body)?;
        Ok(crate::postgres::PortfolioPerformanceResponse { metric, statuses })
    }

    pub async fn query_portfolio_closed_trades(
        &self,
        filter: &crate::postgres::PortfolioClosedTradeFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioClosedTradeRecord>>
    {
        let database = &self.config.calculation_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let code_filter = match &filter.security_code {
            Some(code) => {
                validate_security_code(code)?;
                format!("AND security_code = {}", quote_string_literal(code))
            }
            None => String::new(),
        };
        let exit_date_filter = match filter.exit_date {
            Some(date) => format!("AND exit_date = toDate('{date}')"),
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_run_id, result_attempt_id, closed_trade_id, closed_trade_seq,
       position_lot_id, entry_trade_seq, exit_trade_seq, security_code,
       entry_date, exit_date, quantity, entry_gross_amount, exit_gross_amount,
       entry_fee, exit_fee, entry_fee + exit_fee AS total_fee, realized_pnl,
       if(entry_gross_amount + entry_fee = 0, null, realized_pnl / (entry_gross_amount + entry_fee)) AS realized_return,
       holding_days, exit_reason
FROM {database}.calc_portfolio_closed_trade
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  {exit_date_filter}
  {code_filter}
ORDER BY exit_date, security_code, closed_trade_seq
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-closed-trades",
                    &filter.portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let rows = parse_json_each_row::<crate::postgres::PortfolioClosedTradeRecord>(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    pub async fn query_portfolio_trade_metrics(
        &self,
        filter: &crate::postgres::PortfolioTradeMetricFilter,
        result_attempt_id: &str,
    ) -> RearviewResult<crate::postgres::ListResult<crate::postgres::PortfolioTradeMetricRecord>>
    {
        let database = &self.config.calculation_database;
        validate_identifier(database)?;
        let run_id = quote_string_literal(&filter.portfolio_run_id);
        let attempt = quote_string_literal(result_attempt_id);
        let window_filter = match &filter.window_key {
            Some(window_key) => {
                validate_window_key(window_key)?;
                format!("AND window_key = {}", quote_string_literal(window_key))
            }
            None => String::new(),
        };
        let fetch_limit = filter.page.fetch_limit();
        let sql = format!(
            r#"
SELECT portfolio_run_id, result_attempt_id, window_key, window_start, window_end,
       closed_trade_count, winning_trade_count, losing_trade_count,
       breakeven_trade_count, win_rate_closed_trades, average_win_return,
       average_loss_return, profit_loss_ratio, average_holding_days,
       largest_win_return, largest_loss_return
FROM {database}.calc_portfolio_trade_metric
WHERE portfolio_run_id = {run_id}
  AND result_attempt_id = {attempt}
  {window_filter}
ORDER BY window_key
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#,
            offset = filter.page.offset
        );
        let body = self
            .execute_text(
                &sql,
                &portfolio_read_query_id(
                    "rearview-portfolio-read-trade-metrics",
                    &filter.portfolio_run_id,
                    result_attempt_id,
                ),
            )
            .await?;
        let rows = parse_json_each_row::<crate::postgres::PortfolioTradeMetricRecord>(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    async fn execute_text(&self, sql: &str, query_id: &str) -> RearviewResult<String> {
        let started_at = Instant::now();
        let response = self
            .client
            .post(self.config.base_url())
            .query(&[("query_id", query_id)])
            .basic_auth(&self.config.user, Some(&self.config.password))
            .body(sql.to_string())
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        info!(
            query_id,
            status = status.as_u16(),
            elapsed_ms = started_at.elapsed().as_millis(),
            response_bytes = body.len(),
            "clickhouse query"
        );
        if !status.is_success() {
            return Err(RearviewError::ClickHouse(format!(
                "ClickHouse HTTP status {status}: {body}"
            )));
        }
        Ok(body)
    }
}

fn portfolio_read_query_id(
    prefix: &str,
    portfolio_run_id: &str,
    result_attempt_id: &str,
) -> String {
    format!(
        "{prefix}-{portfolio_run_id}-{result_attempt_id}-{}",
        ulid::Ulid::new()
    )
}

fn portfolio_price_bars_sql(
    database: &str,
    securities: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    indicator_metrics: &[String],
) -> RearviewResult<String> {
    let trend_columns = indicator_metrics
        .iter()
        .map(|metric| price_bar_trend_column(metric))
        .collect::<RearviewResult<BTreeSet<_>>>()?;
    let mut select_columns = vec![
        "    q.security_code AS security_code".to_string(),
        "    q.trade_date AS trade_date".to_string(),
        "    q.open_price_backward_adj AS open_price_backward_adj".to_string(),
        "    q.close_price_backward_adj AS close_price_backward_adj".to_string(),
    ];
    let join = if trend_columns.is_empty() {
        String::new()
    } else {
        select_columns.push("    q.close_price_forward_adj AS close_price_forward_adj".to_string());
        for column in &trend_columns {
            select_columns.push(format!("    t.{column} AS {column}"));
        }
        let trend_select_columns = trend_columns
            .iter()
            .map(|column| format!("        {column}"))
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            r#"
LEFT JOIN (
    SELECT
        security_code,
        trade_date,
{trend_select_columns}
    FROM {database}.`mart_stock_trend_indicator_daily`
    WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
      AND security_code IN ({securities})
) AS t
    ON q.security_code = t.security_code AND q.trade_date = t.trade_date"#
        )
    };
    let select_columns = select_columns.join(",\n");
    Ok(format!(
        r#"
SELECT
{select_columns}
FROM {database}.`mart_stock_quotes_daily` AS q{join}
WHERE q.trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
  AND q.security_code IN ({securities})
ORDER BY q.trade_date ASC, q.security_code ASC
FORMAT JSONEachRow"#
    ))
}

fn portfolio_price_bars_demand_join_sql(
    database: &str,
    demand: &MarketDataDemand,
    indicator_metrics: &[String],
) -> RearviewResult<String> {
    let start_date = demand.earliest_start_date().ok_or_else(|| {
        RearviewError::Validation("market data demand must not be empty".to_string())
    })?;
    let end_date = demand.end_date;
    let securities = demand
        .entries
        .iter()
        .map(|entry| quote_string_literal(&entry.security_code))
        .collect::<Vec<_>>()
        .join(", ");
    let demand_rows = demand
        .entries
        .iter()
        .map(|entry| {
            Ok(format!(
                "    SELECT {} AS security_code, toDate('{}') AS start_date",
                quote_string_literal(&entry.security_code),
                entry.start_date
            ))
        })
        .collect::<RearviewResult<Vec<_>>>()?
        .join("\n    UNION ALL\n");
    let trend_columns = indicator_metrics
        .iter()
        .map(|metric| price_bar_trend_column(metric))
        .collect::<RearviewResult<BTreeSet<_>>>()?;
    let mut select_columns = vec![
        "    q.security_code AS security_code".to_string(),
        "    q.trade_date AS trade_date".to_string(),
        "    q.open_price_backward_adj AS open_price_backward_adj".to_string(),
        "    q.close_price_backward_adj AS close_price_backward_adj".to_string(),
    ];
    let join = if trend_columns.is_empty() {
        String::new()
    } else {
        select_columns.push("    q.close_price_forward_adj AS close_price_forward_adj".to_string());
        for column in &trend_columns {
            select_columns.push(format!("    t.{column} AS {column}"));
        }
        let trend_select_columns = trend_columns
            .iter()
            .map(|column| format!("        {column}"))
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            r#"
LEFT JOIN (
    SELECT
        security_code,
        trade_date,
{trend_select_columns}
    FROM {database}.`mart_stock_trend_indicator_daily`
    WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
      AND security_code IN ({securities})
) AS t
    ON q.security_code = t.security_code AND q.trade_date = t.trade_date"#
        )
    };
    let select_columns = select_columns.join(",\n");
    Ok(format!(
        r#"
WITH demand AS (
{demand_rows}
),
quotes AS (
    SELECT
        security_code,
        trade_date,
        open_price_backward_adj,
        close_price_backward_adj,
        close_price_forward_adj
    FROM {database}.`mart_stock_quotes_daily`
    WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
      AND security_code IN ({securities})
)
SELECT
{select_columns}
FROM quotes AS q
INNER JOIN demand AS d
    ON q.security_code = d.security_code AND q.trade_date >= d.start_date{join}
ORDER BY q.trade_date ASC, q.security_code ASC
FORMAT JSONEachRow"#
    ))
}

fn price_bar_trend_column(metric: &str) -> RearviewResult<&'static str> {
    match metric {
        "price_ma_3" => Ok("price_ma_3"),
        "price_ma_5" => Ok("price_ma_5"),
        "price_ma_6" => Ok("price_ma_6"),
        "price_ma_10" => Ok("price_ma_10"),
        "price_ma_12" => Ok("price_ma_12"),
        "price_ma_14" => Ok("price_ma_14"),
        "price_ma_20" => Ok("price_ma_20"),
        "price_ma_24" => Ok("price_ma_24"),
        "price_ma_28" => Ok("price_ma_28"),
        "price_ma_30" => Ok("price_ma_30"),
        "price_ma_57" => Ok("price_ma_57"),
        "price_ma_60" => Ok("price_ma_60"),
        "price_ma_114" => Ok("price_ma_114"),
        "price_ma_250" => Ok("price_ma_250"),
        "price_avg_ma_3_6_12_24" => Ok("price_avg_ma_3_6_12_24"),
        "price_avg_ma_14_28_57_114" => Ok("price_avg_ma_14_28_57_114"),
        "price_ema2_10" => Ok("price_ema2_10"),
        other => Err(RearviewError::Validation(format!(
            "indicator stop loss metric is not supported for price bars: {other}"
        ))),
    }
}

fn validate_identifier(identifier: &str) -> RearviewResult<()> {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return Err(RearviewError::ClickHouse(
            "identifier must not be empty".to_string(),
        ));
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(RearviewError::ClickHouse(format!(
            "invalid identifier: {identifier}"
        )));
    }
    if !chars.all(|char| char == '_' || char.is_ascii_alphanumeric()) {
        return Err(RearviewError::ClickHouse(format!(
            "invalid identifier: {identifier}"
        )));
    }
    Ok(())
}

fn validate_security_code(security_code: &str) -> RearviewResult<()> {
    let trimmed = security_code.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(RearviewError::Validation(format!(
            "invalid security_code: {security_code}"
        )));
    }
    if !trimmed
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '_' | '-'))
    {
        return Err(RearviewError::Validation(format!(
            "invalid security_code: {security_code}"
        )));
    }
    Ok(())
}

fn validate_source_tenor(source_tenor: &str) -> RearviewResult<()> {
    let trimmed = source_tenor.trim();
    if trimmed.is_empty() || trimmed.len() > 16 {
        return Err(RearviewError::Validation(format!(
            "invalid source_tenor: {source_tenor}"
        )));
    }
    if !trimmed
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '-'))
    {
        return Err(RearviewError::Validation(format!(
            "invalid source_tenor: {source_tenor}"
        )));
    }
    Ok(())
}

fn validate_window_key(window_key: &str) -> RearviewResult<()> {
    let trimmed = window_key.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(RearviewError::Validation(format!(
            "invalid window_key: {window_key}"
        )));
    }
    if !trimmed
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '-'))
    {
        return Err(RearviewError::Validation(format!(
            "invalid window_key: {window_key}"
        )));
    }
    Ok(())
}

fn quote_identifier(identifier: &str) -> String {
    format!("`{identifier}`")
}

fn quote_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn virtual_account_nav_sql(
    quoted_database: &str,
    quoted_run_id_field: &str,
    quoted_run_id: &str,
    quoted_attempt: &str,
) -> String {
    format!(
        r#"
SELECT trade_date, cash_balance, position_market_value, total_equity,
       daily_return, position_count
FROM {quoted_database}.live_nav_daily
WHERE {quoted_run_id_field} = {quoted_run_id}
  AND result_attempt_id = {quoted_attempt}
ORDER BY trade_date DESC
LIMIT 2
FORMAT JSONEachRow"#
    )
}

fn virtual_account_holding_sql(
    quoted_database: &str,
    quoted_run_id_field: &str,
    quoted_run_id: &str,
    quoted_attempt: &str,
    account_date: NaiveDate,
) -> String {
    format!(
        r#"
SELECT coalesce(sum(unrealized_pnl), 0) AS holding_unrealized_pnl
FROM {quoted_database}.live_position_day
WHERE {quoted_run_id_field} = {quoted_run_id}
  AND result_attempt_id = {quoted_attempt}
  AND trade_date = toDate('{account_date}')
FORMAT JSONEachRow"#
    )
}

fn virtual_account_record(
    latest_nav: &VirtualAccountNavRow,
    previous_nav: Option<&VirtualAccountNavRow>,
    holding_unrealized_pnl: f64,
) -> StrategyPortfolioVirtualAccountRecord {
    StrategyPortfolioVirtualAccountRecord {
        account_date: latest_nav.trade_date,
        cash_balance: latest_nav.cash_balance,
        position_market_value: latest_nav.position_market_value,
        total_equity: latest_nav.total_equity,
        holding_unrealized_pnl,
        daily_pnl: previous_nav.map(|row| latest_nav.total_equity - row.total_equity),
        daily_return: latest_nav.daily_return,
        position_count: latest_nav.position_count,
    }
}

fn statement_summary_sql(
    quoted_database: &str,
    quoted_run_id_field: &str,
    quoted_run_id: &str,
    quoted_attempt: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> String {
    format!(
        r#"
SELECT
  (
    SELECT avg(gross_exposure)
    FROM {quoted_database}.live_nav_daily
    WHERE {quoted_run_id_field} = {quoted_run_id}
      AND result_attempt_id = {quoted_attempt}
      AND trade_date >= toDate('{start_date}')
      AND trade_date <= toDate('{end_date}')
  ) AS average_position_pct,
  (
    SELECT countDistinct(security_code)
    FROM {quoted_database}.live_trade
    WHERE {quoted_run_id_field} = {quoted_run_id}
      AND result_attempt_id = {quoted_attempt}
      AND trade_date >= toDate('{start_date}')
      AND trade_date <= toDate('{end_date}')
  ) AS traded_security_count,
  (
    SELECT count()
    FROM {quoted_database}.live_trade
    WHERE {quoted_run_id_field} = {quoted_run_id}
      AND result_attempt_id = {quoted_attempt}
      AND trade_date >= toDate('{start_date}')
      AND trade_date <= toDate('{end_date}')
  ) AS trade_count,
  (
    SELECT if(count() = 0, null, countIf(realized_pnl > 0) / toFloat64(count()))
    FROM (
      SELECT
        trade.trade_seq,
        trade.security_code,
        sum(coalesce(closed.realized_pnl, 0.0)) AS realized_pnl
      FROM {quoted_database}.live_trade AS trade
      LEFT JOIN {quoted_database}.live_closed_trade AS closed
        ON closed.strategy_portfolio_daily_run_id = trade.strategy_portfolio_daily_run_id
       AND closed.result_attempt_id = trade.result_attempt_id
       AND closed.exit_trade_seq = trade.trade_seq
       AND closed.security_code = trade.security_code
      WHERE trade.strategy_portfolio_daily_run_id = {quoted_run_id}
        AND trade.result_attempt_id = {quoted_attempt}
        AND lower(trade.side) = 'sell'
        AND trade.trade_date >= toDate('{start_date}')
        AND trade.trade_date <= toDate('{end_date}')
      GROUP BY trade.trade_seq, trade.security_code
    )
  ) AS trade_win_rate,
  (
    SELECT countIf(realized_pnl_by_security > 0)
    FROM (
      SELECT security_code, sum(realized_pnl) AS realized_pnl_by_security
      FROM {quoted_database}.live_closed_trade
      WHERE {quoted_run_id_field} = {quoted_run_id}
        AND result_attempt_id = {quoted_attempt}
        AND exit_date >= toDate('{start_date}')
        AND exit_date <= toDate('{end_date}')
      GROUP BY security_code
    )
  ) AS winning_security_count,
  (
    SELECT countIf(realized_pnl_by_security < 0)
    FROM (
      SELECT security_code, sum(realized_pnl) AS realized_pnl_by_security
      FROM {quoted_database}.live_closed_trade
      WHERE {quoted_run_id_field} = {quoted_run_id}
        AND result_attempt_id = {quoted_attempt}
        AND exit_date >= toDate('{start_date}')
        AND exit_date <= toDate('{end_date}')
      GROUP BY security_code
    )
  ) AS losing_security_count,
  (
    SELECT countIf(position_count > 0)
    FROM {quoted_database}.live_nav_daily
    WHERE {quoted_run_id_field} = {quoted_run_id}
      AND result_attempt_id = {quoted_attempt}
      AND trade_date >= toDate('{start_date}')
      AND trade_date <= toDate('{end_date}')
  ) AS holding_days
FORMAT JSONEachRow"#
    )
}

fn statement_operations_sql(
    quoted_database: &str,
    quoted_run_id_field: &str,
    quoted_run_id: &str,
    quoted_attempt: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    page: crate::postgres::Page,
) -> String {
    let fetch_limit = page.fetch_limit();
    let offset = page.offset;
    format!(
        r#"
WITH trade_with_balance AS (
  SELECT
    portfolio_trade_id,
    trade_seq,
    trade_date,
    security_code,
    side,
    quantity,
    execution_price,
    gross_amount,
    commission,
    stamp_duty,
    transfer_fee,
    total_fee,
    reason,
    sum(if(lower(side) = 'buy', quantity, -quantity))
      OVER (
        PARTITION BY security_code
        ORDER BY trade_date, trade_seq
        ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
      ) AS position_balance_quantity
  FROM {quoted_database}.live_trade
  WHERE {quoted_run_id_field} = {quoted_run_id}
    AND result_attempt_id = {quoted_attempt}
),
realized_by_exit_trade AS (
  SELECT
    exit_trade_seq,
    security_code,
    sum(realized_pnl) AS realized_pnl
  FROM {quoted_database}.live_closed_trade
  WHERE {quoted_run_id_field} = {quoted_run_id}
    AND result_attempt_id = {quoted_attempt}
  GROUP BY exit_trade_seq, security_code
)
SELECT
  trade_with_balance.portfolio_trade_id,
  trade_with_balance.trade_seq,
  trade_with_balance.trade_date,
  trade_with_balance.security_code,
  trade_with_balance.side,
  trade_with_balance.execution_price,
  trade_with_balance.quantity,
  toUInt32(100) AS lot_size,
  trade_with_balance.quantity / 100.0 AS lot_count,
  trade_with_balance.gross_amount,
  trade_with_balance.commission,
  trade_with_balance.stamp_duty,
  trade_with_balance.transfer_fee,
  trade_with_balance.total_fee,
  trade_with_balance.position_balance_quantity,
  realized_by_exit_trade.realized_pnl,
  trade_with_balance.reason
FROM trade_with_balance
LEFT JOIN realized_by_exit_trade
  ON realized_by_exit_trade.exit_trade_seq = trade_with_balance.trade_seq
 AND realized_by_exit_trade.security_code = trade_with_balance.security_code
WHERE trade_with_balance.trade_date >= toDate('{start_date}')
  AND trade_with_balance.trade_date <= toDate('{end_date}')
ORDER BY trade_with_balance.trade_date DESC, trade_with_balance.trade_seq DESC
LIMIT {fetch_limit} OFFSET {offset}
FORMAT JSONEachRow"#
    )
}

fn trade_dates_sql(
    quoted_database: &str,
    table: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> String {
    format!(
        r#"
SELECT DISTINCT
    trade_date
FROM {quoted_database}.{}
WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
ORDER BY trade_date ASC
FORMAT JSONEachRow"#,
        quote_identifier(table),
    )
}

fn quote_select_columns() -> &'static str {
    r#"    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    prev_close_price,
    prev_close_price_unadj,
    open_price_forward_adj,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj,
    prev_close_price_forward_adj,
    open_price_backward_adj,
    high_price_backward_adj,
    low_price_backward_adj,
    close_price_backward_adj,
    prev_close_price_backward_adj,
    forward_adjustment_factor,
    forward_adjustment_ratio,
    backward_adjustment_factor,
    backward_adjustment_ratio,
    prev_volume,
    volume,
    amount,
    turnover_rate_pct AS turnover_rate,
    turnover_rate_free_float_pct AS turnover_rate_actual,
    amplitude_pct AS pct_amplitude,
    change_pct AS pct_change,
    limit_up_price,
    limit_down_price,
    market_cap AS a_market_cap,
    float_market_cap AS a_float_market_cap,
    free_float_market_cap AS a_free_float_market_cap,
    shares AS a_shares,
    float_shares_a AS a_float_shares,
    free_float_shares AS a_free_float_shares,
    pe_static,
    pe_ttm,
    pe_forecast,
    pb_mrq,
    book_value_per_share,
    roe,
    roa,
    roaa,
    roae,
    dy_static_pct AS dy_static,
    dy_ttm_pct AS dy_ttm,
    is_suspend,
    is_st,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value"#
}

fn chart_quote_select_columns(adjustment: AnalysisQuoteAdjustment) -> &'static str {
    match adjustment {
        AnalysisQuoteAdjustment::ForwardAdjusted => {
            r#"    security_code,
    trade_date,
    open_price_forward_adj,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj,
    volume,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value"#
        }
        AnalysisQuoteAdjustment::BackwardAdjusted => {
            r#"    security_code,
    trade_date,
    open_price_backward_adj,
    high_price_backward_adj,
    low_price_backward_adj,
    close_price_backward_adj,
    volume,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value"#
        }
        AnalysisQuoteAdjustment::Unadjusted => {
            r#"    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    volume,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value"#
        }
    }
}

fn chart_context_chart_quote_select_columns(adjustment: AnalysisQuoteAdjustment) -> &'static str {
    match adjustment {
        AnalysisQuoteAdjustment::ForwardAdjusted => {
            r#"    security_code,
    trade_date,
    open_price_forward_adj,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj,
    volume"#
        }
        AnalysisQuoteAdjustment::BackwardAdjusted => {
            r#"    security_code,
    trade_date,
    open_price_backward_adj,
    high_price_backward_adj,
    low_price_backward_adj,
    close_price_backward_adj,
    volume"#
        }
        AnalysisQuoteAdjustment::Unadjusted => {
            r#"    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    volume"#
        }
    }
}

fn chart_context_selected_quote_select_columns() -> &'static str {
    r#"    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    prev_close_price,
    volume,
    amount,
    amplitude_pct AS pct_amplitude,
    change_pct AS pct_change,
    limit_up_price,
    limit_down_price,
    market_cap AS a_market_cap,
    pe_ttm,
    roe"#
}

fn trend_select_columns() -> &'static str {
    r#"    security_code,
    trade_date,
    price_ma_5,
    price_ma_10,
    price_ma_20,
    price_ma_30,
    price_ma_60,
    price_ma_250,
    price_avg_ma_3_6_12_24,
    price_avg_ma_14_28_57_114,
    price_ema2_10,
    boll_mid_20_2,
    boll_upper_20_2 AS boll_up_20_2,
    boll_lower_20_2 AS boll_dn_20_2,
    macd_dif,
    macd_dea,
    macd_histogram"#
}

fn chart_context_trend_select_columns() -> &'static str {
    r#"    security_code,
    trade_date,
    price_ma_5,
    price_ma_10,
    price_ma_30"#
}

fn parse_json_each_row<T>(body: &str) -> RearviewResult<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        rows.push(serde_json::from_str::<T>(line)?);
    }
    Ok(rows)
}

fn deserialize_optional_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalF64Visitor;

    impl<'de> de::Visitor<'de> for OptionalF64Visitor {
        type Value = Option<f64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("null, numeric, or numeric string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as f64))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as f64))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.trim().is_empty() {
                return Ok(None);
            }
            value.parse::<f64>().map(Some).map_err(|_| {
                E::invalid_value(Unexpected::Str(value), &"a numeric string or empty string")
            })
        }
    }

    deserializer.deserialize_option(OptionalF64Visitor)
}

fn deserialize_optional_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalBoolVisitor;

    impl<'de> de::Visitor<'de> for OptionalBoolVisitor {
        type Value = Option<bool>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("null, boolean, 0/1 integer, or true/false string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                0 => Ok(Some(false)),
                1 => Ok(Some(true)),
                _ => Err(E::invalid_value(Unexpected::Signed(value), &self)),
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                0 => Ok(Some(false)),
                1 => Ok(Some(true)),
                _ => Err(E::invalid_value(Unexpected::Unsigned(value), &self)),
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                "" => Ok(None),
                "0" | "false" | "FALSE" => Ok(Some(false)),
                "1" | "true" | "TRUE" => Ok(Some(true)),
                _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
            }
        }
    }

    deserializer.deserialize_option(OptionalBoolVisitor)
}

fn deserialize_clickhouse_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct ClickHouseBoolVisitor;

    impl de::Visitor<'_> for ClickHouseBoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a boolean, 0/1 integer, or true/false string")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(E::invalid_value(Unexpected::Signed(value), &self)),
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(E::invalid_value(Unexpected::Unsigned(value), &self)),
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                "0" | "false" | "FALSE" => Ok(false),
                "1" | "true" | "TRUE" => Ok(true),
                _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
            }
        }
    }

    deserializer.deserialize_any(ClickHouseBoolVisitor)
}

#[cfg(test)]
mod tests {
    use super::{
        AnalysisQuoteAdjustment, BacktestSignalRow, MarketDataDemand, QuoteMartRow, ScreeningRow,
        chart_context_chart_quote_select_columns, chart_context_selected_quote_select_columns,
        chart_context_trend_select_columns, portfolio_price_bars_demand_join_sql,
        portfolio_price_bars_sql, quote_select_columns, trade_dates_sql, trend_select_columns,
        validate_security_code, validate_source_tenor, validate_window_key,
        virtual_account_holding_sql, virtual_account_nav_sql, virtual_account_record,
    };
    use chrono::NaiveDate;

    use crate::RearviewError;
    use crate::postgres::Page;

    fn row_json(is_buy_signal: &str) -> String {
        format!(
            r#"{{
                "security_code": "000001.SZ",
                "trade_date": "2026-05-20",
                "raw_score": 42.5,
                "score": 42.5,
                "signal_rank": 1,
                "is_buy_signal": {is_buy_signal},
                "score_breakdown": "{{}}",
                "selected_metrics": "{{}}",
                "raw_values": "{{}}"
            }}"#
        )
    }

    #[test]
    fn screening_row_accepts_clickhouse_integer_true() {
        let row = serde_json::from_str::<ScreeningRow>(&row_json("1"))
            .expect("integer true should deserialize");

        assert!(row.is_buy_signal);
        assert_eq!(row.raw_score, 42.5);
    }

    #[test]
    fn screening_row_accepts_clickhouse_integer_false() {
        let row = serde_json::from_str::<ScreeningRow>(&row_json("0"))
            .expect("integer false should deserialize");

        assert!(!row.is_buy_signal);
    }

    #[test]
    fn screening_row_accepts_json_boolean() {
        let row = serde_json::from_str::<ScreeningRow>(&row_json("true"))
            .expect("boolean true should deserialize");

        assert!(row.is_buy_signal);
    }

    #[test]
    fn screening_row_accepts_clickhouse_string_boolean() {
        let row = serde_json::from_str::<ScreeningRow>(&row_json(r#""false""#))
            .expect("string false should deserialize");

        assert!(!row.is_buy_signal);
    }

    #[test]
    fn backtest_signal_row_accepts_only_worker_hot_path_fields() {
        let json = r#"{
            "security_code": "000001.SZ",
            "trade_date": "2026-05-20",
            "score": 42.5,
            "signal_rank": 1
        }"#;

        let row = serde_json::from_str::<BacktestSignalRow>(json)
            .expect("backtest signal row should deserialize narrow fields");

        assert_eq!(row.security_code, "000001.SZ");
        assert_eq!(
            row.trade_date,
            NaiveDate::from_ymd_opt(2026, 5, 20).unwrap()
        );
        assert_eq!(row.score, 42.5);
        assert_eq!(row.signal_rank, 1);
    }

    #[test]
    fn virtual_account_record_should_derive_daily_pnl_from_total_equity_delta() {
        let latest = virtual_account_nav_row("2026-06-27", 101_000.0, Some(0.01));
        let previous = virtual_account_nav_row("2026-06-26", 100_000.0, None);

        let record = virtual_account_record(&latest, Some(&previous), 2_500.0);

        assert_eq!(record.daily_pnl, Some(1_000.0));
    }

    #[test]
    fn virtual_account_record_should_leave_daily_pnl_empty_without_previous_nav() {
        let latest = virtual_account_nav_row("2026-06-27", 101_000.0, None);

        let record = virtual_account_record(&latest, None, 0.0);

        assert_eq!(record.daily_pnl, None);
    }

    #[test]
    fn virtual_account_nav_sql_should_filter_live_attempt_and_limit_latest_two_rows() {
        let sql = virtual_account_nav_sql(
            "`fleur_portfolio`",
            "`strategy_portfolio_daily_run_id`",
            "'daily-run-1'",
            "'attempt-1'",
        );

        assert!(sql.contains("FROM `fleur_portfolio`.live_nav_daily"));
        assert!(sql.contains("`strategy_portfolio_daily_run_id` = 'daily-run-1'"));
        assert!(sql.contains("result_attempt_id = 'attempt-1'"));
        assert!(sql.contains("ORDER BY trade_date DESC"));
        assert!(sql.contains("LIMIT 2"));
    }

    #[test]
    fn virtual_account_holding_sql_should_aggregate_unrealized_pnl_for_account_date() {
        let sql = virtual_account_holding_sql(
            "`fleur_portfolio`",
            "`strategy_portfolio_daily_run_id`",
            "'daily-run-1'",
            "'attempt-1'",
            NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(),
        );

        assert!(sql.contains("coalesce(sum(unrealized_pnl), 0)"));
        assert!(sql.contains("FROM `fleur_portfolio`.live_position_day"));
        assert!(sql.contains("`strategy_portfolio_daily_run_id` = 'daily-run-1'"));
        assert!(sql.contains("result_attempt_id = 'attempt-1'"));
        assert!(sql.contains("trade_date = toDate('2026-06-27')"));
    }

    #[test]
    fn statement_summary_sql_should_bind_attempt_and_compute_sell_trade_win_rate() {
        let sql = super::statement_summary_sql(
            "`fleur_portfolio`",
            "`strategy_portfolio_daily_run_id`",
            "'daily-run-1'",
            "'attempt-1'",
            NaiveDate::from_ymd_opt(2026, 3, 26).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 26).unwrap(),
        );

        assert!(sql.contains("FROM `fleur_portfolio`.live_nav_daily"));
        assert!(sql.contains("FROM `fleur_portfolio`.live_trade"));
        assert!(sql.contains("FROM `fleur_portfolio`.live_closed_trade"));
        assert!(sql.contains("`strategy_portfolio_daily_run_id` = 'daily-run-1'"));
        assert!(sql.contains("result_attempt_id = 'attempt-1'"));
        assert!(sql.contains("lower(trade.side) = 'sell'"));
        assert!(sql.contains("closed.exit_trade_seq = trade.trade_seq"));
        assert!(sql.contains("countIf(realized_pnl > 0) / toFloat64(count())"));
        assert!(sql.contains("countIf(position_count > 0)"));
        assert!(sql.contains("trade_date >= toDate('2026-03-26')"));
        assert!(sql.contains("trade_date <= toDate('2026-06-26')"));
    }

    #[test]
    fn statement_operations_sql_should_compute_balance_before_period_filter() {
        let sql = super::statement_operations_sql(
            "`fleur_portfolio`",
            "`strategy_portfolio_daily_run_id`",
            "'daily-run-1'",
            "'attempt-1'",
            NaiveDate::from_ymd_opt(2026, 3, 26).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 26).unwrap(),
            Page {
                limit: 100,
                offset: 20,
            },
        );

        assert!(sql.contains("WITH trade_with_balance AS"));
        assert!(sql.contains("ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW"));
        assert!(sql.contains("WHERE `strategy_portfolio_daily_run_id` = 'daily-run-1'"));
        assert!(sql.contains("result_attempt_id = 'attempt-1'"));
        assert!(sql.contains("LEFT JOIN realized_by_exit_trade"));
        assert!(
            sql.contains("realized_by_exit_trade.exit_trade_seq = trade_with_balance.trade_seq")
        );
        assert!(sql.contains("WHERE trade_with_balance.trade_date >= toDate('2026-03-26')"));
        assert!(sql.contains("AND trade_with_balance.trade_date <= toDate('2026-06-26')"));
        assert!(sql.contains(
            "ORDER BY trade_with_balance.trade_date DESC, trade_with_balance.trade_seq DESC"
        ));
        assert!(sql.contains("LIMIT 101 OFFSET 20"));
    }

    #[test]
    fn trade_dates_sql_reads_quote_mart_when_database_is_marts() {
        let sql = trade_dates_sql(
            "`fleur_marts`",
            "mart_stock_quotes_daily",
            NaiveDate::from_ymd_opt(2026, 6, 26).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 29).unwrap(),
        );

        assert!(sql.contains("FROM `fleur_marts`.`mart_stock_quotes_daily`"));
        assert!(!sql.contains("int_trade_calendar"));
    }

    #[test]
    fn trade_dates_sql_reads_trade_calendar_mart_when_database_is_marts() {
        let sql = trade_dates_sql(
            "`fleur_marts`",
            "mart_trade_calendar",
            NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(),
            NaiveDate::from_ymd_opt(2026, 8, 10).unwrap(),
        );

        assert!(sql.contains("FROM `fleur_marts`.`mart_trade_calendar`"));
        assert!(!sql.contains("mart_stock_quotes_daily"));
    }

    fn virtual_account_nav_row(
        trade_date: &str,
        total_equity: f64,
        daily_return: Option<f64>,
    ) -> super::VirtualAccountNavRow {
        super::VirtualAccountNavRow {
            trade_date: NaiveDate::parse_from_str(trade_date, "%Y-%m-%d").unwrap(),
            cash_balance: 10_000.0,
            position_market_value: total_equity - 10_000.0,
            total_equity,
            daily_return,
            position_count: 3,
        }
    }

    #[test]
    fn quote_row_accepts_quoted_numeric_and_integer_bool() {
        let json = r#"{
            "security_code": "sh.600000",
            "trade_date": "2026-06-12",
            "open_price": "10.1",
            "high_price": 10.5,
            "low_price": null,
            "close_price": 10,
            "is_suspend": 0,
            "is_st": "1"
        }"#;

        let row = serde_json::from_str::<QuoteMartRow>(json)
            .expect("quote row should deserialize mixed ClickHouse JSON values");

        assert_eq!(row.open_price, Some(10.1));
        assert_eq!(row.is_suspend, Some(false));
        assert_eq!(row.is_st, Some(true));
    }

    #[test]
    fn quote_select_columns_alias_current_mart_fields_to_analysis_contract() {
        let columns = quote_select_columns();

        assert!(columns.contains("turnover_rate_pct AS turnover_rate"));
        assert!(columns.contains("turnover_rate_free_float_pct AS turnover_rate_actual"));
        assert!(columns.contains("amplitude_pct AS pct_amplitude"));
        assert!(columns.contains("change_pct AS pct_change"));
        assert!(columns.contains("market_cap AS a_market_cap"));
        assert!(columns.contains("dy_static_pct AS dy_static"));
    }

    #[test]
    fn trend_select_columns_alias_current_boll_fields_to_analysis_contract() {
        let columns = trend_select_columns();

        assert!(columns.contains("boll_upper_20_2 AS boll_up_20_2"));
        assert!(columns.contains("boll_lower_20_2 AS boll_dn_20_2"));
    }

    #[test]
    fn chart_context_quote_columns_exclude_kdj_fields() {
        let columns =
            chart_context_chart_quote_select_columns(AnalysisQuoteAdjustment::ForwardAdjusted);

        assert!(!columns.contains("kdj_"));
    }

    #[test]
    fn chart_context_selected_quote_columns_include_only_step3_panel_fields() {
        let columns = chart_context_selected_quote_select_columns();

        assert!(columns.contains("market_cap AS a_market_cap"));
        assert!(!columns.contains("kdj_"));
        assert!(!columns.contains("turnover_rate"));
        assert!(!columns.contains("pe_static"));
        assert!(!columns.contains("pb_mrq"));
    }

    #[test]
    fn chart_context_trend_columns_exclude_macd_and_boll_fields() {
        let columns = chart_context_trend_select_columns();

        assert!(columns.contains("price_ma_5"));
        assert!(columns.contains("price_ma_10"));
        assert!(columns.contains("price_ma_30"));
        assert!(!columns.contains("macd"));
        assert!(!columns.contains("boll"));
        assert!(!columns.contains("price_ma_20"));
    }

    #[test]
    fn portfolio_price_bars_sql_omits_trend_join_without_indicator_metrics() {
        let sql = portfolio_price_bars_sql(
            "`fleur_marts`",
            "'600000.SH'",
            NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
            &[],
        )
        .unwrap();

        assert!(!sql.contains("mart_stock_trend_indicator_daily"));
        assert!(!sql.contains("close_price_forward_adj"));
    }

    #[test]
    fn portfolio_price_bars_sql_projects_only_requested_indicator_metric() {
        let sql = portfolio_price_bars_sql(
            "`fleur_marts`",
            "'600000.SH'",
            NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
            &["price_ma_10".to_string()],
        )
        .unwrap();

        assert!(sql.contains("mart_stock_trend_indicator_daily"));
        assert!(sql.contains("t.price_ma_10 AS price_ma_10"));
        assert!(sql.contains("WHERE trade_date BETWEEN"));
        assert!(sql.contains("AND security_code IN ('600000.SH')"));
        assert!(!sql.contains("price_ma_20"));
        assert!(!sql.contains("price_ema2_10"));
    }

    #[test]
    fn portfolio_price_bars_sql_rejects_unsupported_indicator_metric() {
        let error = portfolio_price_bars_sql(
            "`fleur_marts`",
            "'600000.SH'",
            NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
            &["unknown_metric".to_string()],
        )
        .unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn market_data_demand_keeps_earliest_start_per_security() {
        let demand = MarketDataDemand::from_security_start_dates(
            [
                (
                    "600000.SH".to_string(),
                    NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
                ),
                (
                    "600000.SH".to_string(),
                    NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                ),
                (
                    "000001.SZ".to_string(),
                    NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                ),
            ],
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
        )
        .unwrap();

        assert_eq!(demand.entries.len(), 2);
        assert_eq!(
            demand.earliest_start_date(),
            Some(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap())
        );
        assert_eq!(
            demand.entries[1].start_date,
            NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()
        );
    }

    #[test]
    fn portfolio_price_bars_demand_join_sql_filters_before_joining() {
        let demand = MarketDataDemand::from_security_start_dates(
            [
                (
                    "600000.SH".to_string(),
                    NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                ),
                (
                    "000001.SZ".to_string(),
                    NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
                ),
            ],
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
        )
        .unwrap();

        let sql = portfolio_price_bars_demand_join_sql(
            "`fleur_marts`",
            &demand,
            &["price_ma_10".to_string()],
        )
        .unwrap();

        assert!(sql.contains("WITH demand AS"));
        assert!(sql.contains("UNION ALL"));
        assert!(sql.contains("FROM `fleur_marts`.`mart_stock_quotes_daily`"));
        assert!(
            sql.contains("WHERE trade_date BETWEEN toDate('2025-01-02') AND toDate('2025-01-31')")
        );
        assert!(sql.contains("AND security_code IN ('000001.SZ', '600000.SH')"));
        assert!(sql.contains("q.trade_date >= d.start_date"));
        assert!(sql.contains("FROM `fleur_marts`.`mart_stock_trend_indicator_daily`"));
        assert!(sql.contains("t.price_ma_10 AS price_ma_10"));
        assert!(!sql.contains("price_ma_20"));
    }

    #[test]
    fn portfolio_price_bars_demand_join_sql_rejects_unsupported_indicator_metric() {
        let demand = MarketDataDemand::from_security_start_dates(
            [(
                "600000.SH".to_string(),
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            )],
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
        )
        .unwrap();

        let error = portfolio_price_bars_demand_join_sql(
            "`fleur_marts`",
            &demand,
            &["unknown_metric".to_string()],
        )
        .unwrap_err();

        assert!(matches!(error, RearviewError::Validation(_)));
    }

    #[test]
    fn validate_security_code_rejects_sql_metacharacters() {
        let result = validate_security_code("sh.600000'; drop table x; --");

        assert!(result.is_err());
    }

    #[test]
    fn validate_source_tenor_rejects_sql_metacharacters() {
        let result = validate_source_tenor("1y'; drop table x; --");

        assert!(result.is_err());
    }

    #[test]
    fn validate_window_key_rejects_sql_metacharacters() {
        let result = validate_window_key("full_period'; drop table x; --");

        assert!(result.is_err());
    }
}
