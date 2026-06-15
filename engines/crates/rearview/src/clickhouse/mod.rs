use chrono::NaiveDate;
use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize};

use crate::config::ClickHouseConfig;
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Deserialize)]
struct TradeDateRow {
    trade_date: NaiveDate,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScreeningRow {
    pub security_code: String,
    pub trade_date: NaiveDate,
    pub raw_score: f64,
    pub score: f64,
    pub signal_rank: u32,
    #[serde(deserialize_with = "deserialize_clickhouse_bool")]
    pub is_buy_signal: bool,
    pub score_breakdown: String,
    pub selected_metrics: String,
    pub raw_values: String,
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
        let sql = format!(
            r#"
SELECT
    security_code,
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
    boll_up_20_2,
    boll_dn_20_2,
    macd_dif,
    macd_dea,
    macd_histogram
FROM {database}.`mart_stock_trend_indicator`
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
FROM {database}.`mart_stock_momentum_indicator`
WHERE trade_date BETWEEN toDate('{start_date}') AND toDate('{end_date}')
  AND security_code = {security_code}
ORDER BY trade_date ASC
FORMAT JSONEachRow"#
        );
        let body = self.execute_text(&sql, query_id).await?;
        parse_json_each_row::<MomentumIndicatorRow>(&body)
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
    turnover_rate,
    turnover_rate_actual,
    pct_amplitude,
    pct_change,
    limit_up_price,
    limit_down_price,
    a_market_cap,
    a_float_market_cap,
    a_free_float_market_cap,
    a_shares,
    a_float_shares,
    a_free_float_shares,
    pe_static,
    pe_ttm,
    pe_forecast,
    pb_mrq,
    book_value_per_share,
    roe,
    roa,
    roaa,
    roae,
    dy_static,
    dy_ttm,
    is_suspend,
    is_st,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value"#
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
    use super::{QuoteMartRow, ScreeningRow, validate_security_code};

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
    fn validate_security_code_rejects_sql_metacharacters() {
        let result = validate_security_code("sh.600000'; drop table x; --");

        assert!(result.is_err());
    }
}
