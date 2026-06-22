use chrono::NaiveDate;
use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize};

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
pub struct SecurityDisplayRow {
    pub security_code: String,
    pub security_name: Option<String>,
    pub exchange_code: Option<String>,
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
    exchange_code
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
        let sql = format!(
            r#"
SELECT DISTINCT
    trade_date
FROM {database}.`mart_stock_quotes_daily`
WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
ORDER BY trade_date ASC
FORMAT JSONEachRow"#,
            database = quote_identifier(&self.config.marts_database),
        );
        let body = self.execute_text(&sql, query_id).await?;
        let mut trade_dates = Vec::new();
        for row in parse_json_each_row::<TradeDateRow>(&body)? {
            trade_dates.push(row.trade_date);
        }
        Ok(trade_dates)
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
WHERE security_code = {security_code}
  AND trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
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
WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
  AND security_code = {security_code}
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
WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
  AND security_code = {security_code}
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
        let sql = format!(
            r#"
SELECT
    security_code,
    trade_date,
    open_price_backward_adj,
    close_price_backward_adj
FROM {database}.`mart_stock_quotes_daily`
WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
  AND security_code IN ({securities})
ORDER BY trade_date ASC, security_code ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<PriceBar>(&body)
    }

    // ---- Portfolio result reads (Phase 5: API read source switch) ----

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
            .execute_text(&sql, "rearview-portfolio-read-nav")
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
            .execute_text(&sql, "rearview-portfolio-read-targets")
            .await?;
        let rows: Vec<crate::postgres::PortfolioTargetRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
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
            .execute_text(&sql, "rearview-portfolio-read-orders")
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
            .execute_text(&sql, "rearview-portfolio-read-trades")
            .await?;
        let rows: Vec<crate::postgres::PortfolioTradeRecord> = parse_json_each_row(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
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
                    .execute_text(&max_sql, "rearview-portfolio-read-max-date")
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
            .execute_text(&sql, "rearview-portfolio-read-positions")
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
            .execute_text(&sql, "rearview-portfolio-read-events")
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
            .execute_text(&sql, "rearview-portfolio-read-performance")
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
            .execute_text(&status_sql, "rearview-portfolio-read-performance-status")
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
            .execute_text(&sql, "rearview-portfolio-read-closed-trades")
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
            .execute_text(&sql, "rearview-portfolio-read-trade-metrics")
            .await?;
        let rows = parse_json_each_row::<crate::postgres::PortfolioTradeMetricRecord>(&body)?;
        Ok(crate::postgres::ListResult::from_rows(rows, filter.page))
    }

    async fn execute_text(&self, sql: &str, query_id: &str) -> RearviewResult<String> {
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
        if !status.is_success() {
            return Err(RearviewError::ClickHouse(format!(
                "ClickHouse HTTP status {status}: {body}"
            )));
        }
        Ok(body)
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

fn trend_select_columns() -> &'static str {
    r#"    security_code,
    trade_date,
    price_ma_5,
    price_ma_10,
    price_ma_20,
    price_ma_30,
    price_ma_60,
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
        QuoteMartRow, ScreeningRow, quote_select_columns, trend_select_columns,
        validate_security_code, validate_source_tenor, validate_window_key,
    };

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
