use crate::config::ClickHouseConfig;
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, serde::Deserialize)]
struct TradeDateRow {
    trade_date: chrono::NaiveDate,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ScreeningRow {
    pub security_code: String,
    pub trade_date: chrono::NaiveDate,
    pub score: f64,
    pub signal_rank: u32,
    #[serde(deserialize_with = "deserialize_clickhouse_bool")]
    pub is_buy_signal: bool,
    pub score_breakdown: String,
    pub selected_metrics: String,
    pub raw_values: String,
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
        let mut rows = Vec::new();
        for line in body.lines().filter(|line| !line.trim().is_empty()) {
            rows.push(serde_json::from_str::<ScreeningRow>(line)?);
        }
        Ok(rows)
    }

    pub async fn query_trade_dates(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
        query_id: &str,
    ) -> RearviewResult<Vec<chrono::NaiveDate>> {
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
        for line in body.lines().filter(|line| !line.trim().is_empty()) {
            let row = serde_json::from_str::<TradeDateRow>(line)?;
            trade_dates.push(row.trade_date);
        }
        Ok(trade_dates)
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

fn quote_identifier(identifier: &str) -> String {
    format!("`{identifier}`")
}

fn deserialize_clickhouse_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ClickHouseBoolVisitor;

    impl serde::de::Visitor<'_> for ClickHouseBoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a boolean, 0/1 integer, or true/false string")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(E::invalid_value(
                    serde::de::Unexpected::Signed(value),
                    &self,
                )),
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(E::invalid_value(
                    serde::de::Unexpected::Unsigned(value),
                    &self,
                )),
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "0" | "false" | "FALSE" => Ok(false),
                "1" | "true" | "TRUE" => Ok(true),
                _ => Err(E::invalid_value(serde::de::Unexpected::Str(value), &self)),
            }
        }
    }

    deserializer.deserialize_any(ClickHouseBoolVisitor)
}

#[cfg(test)]
mod tests {
    use super::ScreeningRow;

    fn row_json(is_buy_signal: &str) -> String {
        format!(
            r#"{{
                "security_code": "000001.SZ",
                "trade_date": "2026-05-20",
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
}
