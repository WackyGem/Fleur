use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub rearview_database_url: String,
    pub http_bind: SocketAddr,
    pub max_concurrent_runs: usize,
    pub chunk_small_range_trading_days: u32,
    pub clickhouse: ClickHouseConfig,
    pub nats: NatsConfig,
}

#[derive(Debug, Clone)]
pub struct ClickHouseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub secure: bool,
    pub marts_database: String,
    pub portfolio_database: String,
    pub calculation_database: String,
    pub connect_timeout: Duration,
    pub query_timeout: Duration,
    pub max_execution_time_seconds: u64,
    pub max_rows_to_read: u64,
    pub max_bytes_to_read: u64,
}

#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub portfolio_stream: String,
    pub portfolio_request_subject: String,
    pub portfolio_worker_durable: String,
    pub portfolio_worker_queue: String,
}

impl AppConfig {
    pub fn from_env() -> RearviewResult<Self> {
        let max_concurrent_runs = parse_env("REARVIEW_MAX_CONCURRENT_RUNS", "1")?;
        if max_concurrent_runs == 0 {
            return Err(RearviewError::Config(
                "REARVIEW_MAX_CONCURRENT_RUNS must be greater than 0".to_string(),
            ));
        }
        Ok(Self {
            rearview_database_url: required_env("REARVIEW_DATABASE_URL")?,
            http_bind: parse_env("REARVIEW_HTTP_BIND", "127.0.0.1:34057")?,
            max_concurrent_runs,
            chunk_small_range_trading_days: parse_env(
                "REARVIEW_CHUNK_SMALL_RANGE_TRADING_DAYS",
                "90",
            )?,
            clickhouse: ClickHouseConfig::from_env()?,
            nats: NatsConfig::from_env(),
        })
    }
}

impl NatsConfig {
    pub fn from_env() -> Self {
        Self {
            url: env_with_default("REARVIEW_NATS_URL", "nats://127.0.0.1:34055"),
            portfolio_stream: env_with_default("REARVIEW_PORTFOLIO_STREAM", "REARVIEW_PORTFOLIO"),
            portfolio_request_subject: env_with_default(
                "REARVIEW_PORTFOLIO_REQUEST_SUBJECT",
                "rearview.portfolio_run.requested",
            ),
            portfolio_worker_durable: env_with_default(
                "REARVIEW_PORTFOLIO_WORKER_DURABLE",
                "rearview-portfolio-worker",
            ),
            portfolio_worker_queue: env_with_default(
                "REARVIEW_PORTFOLIO_WORKER_QUEUE",
                "rearview-portfolio-workers",
            ),
        }
    }
}

impl ClickHouseConfig {
    pub fn from_env() -> RearviewResult<Self> {
        Ok(Self {
            host: env_with_default("CLICKHOUSE_HOST", "127.0.0.1"),
            port: parse_env("CLICKHOUSE_PORT", "34052")?,
            user: env_with_default("CLICKHOUSE_USER", "default"),
            password: env_with_default("CLICKHOUSE_PASSWORD", ""),
            secure: parse_env("CLICKHOUSE_SECURE", "false")?,
            marts_database: env_with_default("REARVIEW_CLICKHOUSE_MARTS_DATABASE", "fleur_marts"),
            portfolio_database: env_with_default(
                "REARVIEW_CLICKHOUSE_PORTFOLIO_DATABASE",
                "fleur_portfolio",
            ),
            calculation_database: env_with_default(
                "REARVIEW_CLICKHOUSE_CALCULATION_DATABASE",
                "fleur_calculation",
            ),
            connect_timeout: Duration::from_secs(parse_env(
                "CLICKHOUSE_CONNECT_TIMEOUT_SECONDS",
                "10",
            )?),
            query_timeout: Duration::from_secs(parse_env(
                "CLICKHOUSE_QUERY_TIMEOUT_SECONDS",
                "300",
            )?),
            max_execution_time_seconds: parse_env(
                "REARVIEW_CLICKHOUSE_MAX_EXECUTION_TIME_SECONDS",
                "300",
            )?,
            max_rows_to_read: parse_env("REARVIEW_CLICKHOUSE_MAX_ROWS_TO_READ", "1000000000")?,
            max_bytes_to_read: parse_env("REARVIEW_CLICKHOUSE_MAX_BYTES_TO_READ", "100000000000")?,
        })
    }

    pub fn base_url(&self) -> String {
        let scheme = if self.secure { "https" } else { "http" };
        format!("{scheme}://{}:{}", self.host, self.port)
    }
}

pub fn load_dotenv_if_present(path: impl AsRef<Path>) -> RearviewResult<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }

    dotenvy::from_path(path).map_err(|error| RearviewError::Config(error.to_string()))?;
    Ok(())
}

fn required_env(key: &str) -> RearviewResult<String> {
    let value = env::var(key).map_err(|_| RearviewError::Config(format!("{key} is required")))?;
    if value.trim().is_empty() {
        return Err(RearviewError::Config(format!("{key} must not be empty")));
    }
    Ok(value)
}

fn env_with_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn parse_env<T>(key: &str, default: &str) -> RearviewResult<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = env::var(key).unwrap_or_else(|_| default.to_string());
    value
        .parse::<T>()
        .map_err(|error| RearviewError::Config(format!("{key}={value:?} is invalid: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clickhouse_base_url_should_use_http_when_secure_is_false() {
        let config = ClickHouseConfig {
            host: "127.0.0.1".to_string(),
            port: 8123,
            user: "default".to_string(),
            password: String::new(),
            secure: false,
            marts_database: "fleur_marts".to_string(),
            portfolio_database: "fleur_portfolio".to_string(),
            calculation_database: "fleur_calculation".to_string(),
            connect_timeout: Duration::from_secs(1),
            query_timeout: Duration::from_secs(1),
            max_execution_time_seconds: 30,
            max_rows_to_read: 100,
            max_bytes_to_read: 1000,
        };

        assert_eq!(config.base_url(), "http://127.0.0.1:8123");
    }
}
