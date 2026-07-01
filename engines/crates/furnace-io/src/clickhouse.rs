use std::env;
use std::time::Duration;

use clickhouse::{Client, RowOwned, RowRead, RowWrite};
use tokio::runtime::{Builder, Runtime};

use crate::FurnaceIoError;

const DEFAULT_HTTP_HOST: &str = "127.0.0.1";
const DEFAULT_HTTP_PORT: &str = "8123";

/// 面向 runner 和 CLI 适配器的 typed ClickHouse 执行接口。
pub trait ClickHouseExecutor {
    /// 执行 typed SELECT 并返回全部结果。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败或 row schema 不匹配时，返回 [`FurnaceIoError`]。
    fn fetch_all<T>(&mut self, sql: &str) -> Result<Vec<T>, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send;

    /// 执行 typed SELECT 并返回首行。
    ///
    /// # 错误
    ///
    /// 当查询没有返回行或 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn fetch_one<T>(&mut self, sql: &str) -> Result<T, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send,
    {
        self.fetch_all(sql)?
            .into_iter()
            .next()
            .ok_or_else(|| FurnaceIoError::Parse("ClickHouse query returned no rows".to_string()))
    }

    /// 执行 typed SELECT 并返回可选首行。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn fetch_optional<T>(&mut self, sql: &str) -> Result<Option<T>, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send,
    {
        Ok(self.fetch_all(sql)?.into_iter().next())
    }

    /// 执行 DDL 或其他无结果语句。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError>;

    /// 执行多条无结果语句。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn execute_many(&mut self, sqls: &[String]) -> Result<(), FurnaceIoError> {
        for sql in sqls {
            self.execute(sql)?;
        }
        Ok(())
    }

    /// 按 `batch_size` 将 typed rows 写入 ClickHouse。
    ///
    /// # 错误
    ///
    /// 当序列化或 ClickHouse 写入失败时，返回 [`FurnaceIoError`]。
    fn insert_rows<T>(
        &mut self,
        table: &str,
        rows: &[T],
        batch_size: usize,
    ) -> Result<(), FurnaceIoError>
    where
        T: RowOwned + RowWrite + Clone + Send + Sync;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseHttpConfig {
    pub url: String,
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub validate_schema: bool,
    pub query_timeout: Option<Duration>,
}

impl ClickHouseHttpConfig {
    /// 从进程环境读取 Furnace HTTP ClickHouse 配置。
    ///
    /// 只读取 HTTP client 相关变量；不会读取旧 native CLI 变量。
    pub fn from_env() -> Result<Self, FurnaceIoError> {
        Self::from_getter(|name| env::var(name).ok())
    }

    fn from_getter(mut get: impl FnMut(&str) -> Option<String>) -> Result<Self, FurnaceIoError> {
        let url = match get("FURNACE_CLICKHOUSE_URL") {
            Some(value) if !value.trim().is_empty() => value,
            _ => {
                let secure = parse_bool(get("CLICKHOUSE_SECURE").as_deref()).unwrap_or(false);
                let scheme = if secure { "https" } else { "http" };
                let host = get("CLICKHOUSE_HOST").unwrap_or_else(|| DEFAULT_HTTP_HOST.to_string());
                let port = get("CLICKHOUSE_PORT").unwrap_or_else(|| DEFAULT_HTTP_PORT.to_string());
                format!("{scheme}://{host}:{port}")
            }
        };

        let validate_schema =
            parse_bool(get("FURNACE_CLICKHOUSE_VALIDATE_SCHEMA").as_deref()).unwrap_or(true);
        let query_timeout = get("CLICKHOUSE_QUERY_TIMEOUT_SECONDS")
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                value.parse::<u64>().map(Duration::from_secs).map_err(|_| {
                    FurnaceIoError::Config(format!(
                        "CLICKHOUSE_QUERY_TIMEOUT_SECONDS must be a positive integer: {value}"
                    ))
                })
            })
            .transpose()?;

        Ok(Self {
            url,
            user: get("CLICKHOUSE_USER").filter(|value| !value.is_empty()),
            password: get("CLICKHOUSE_PASSWORD").filter(|value| !value.is_empty()),
            database: get("CLICKHOUSE_DB").filter(|value| !value.is_empty()),
            validate_schema,
            query_timeout,
        })
    }

    fn client(&self) -> Client {
        let mut client = Client::default()
            .with_url(&self.url)
            .with_validation(self.validate_schema)
            .with_product_info("fleur-furnace", env!("CARGO_PKG_VERSION"));
        if let Some(user) = &self.user {
            client = client.with_user(user);
        }
        if let Some(password) = &self.password {
            client = client.with_password(password);
        }
        if let Some(database) = &self.database {
            client = client.with_database(database);
        }
        if let Some(timeout) = self.query_timeout {
            let seconds = timeout.as_secs().to_string();
            client = client
                .with_setting("send_timeout", &seconds)
                .with_setting("receive_timeout", &seconds);
        }
        client
    }
}

pub struct ClickHouseHttpExecutor {
    client: Client,
    runtime: Runtime,
    query_timeout: Option<Duration>,
}

impl ClickHouseHttpExecutor {
    /// 根据环境变量构造 HTTP executor。
    ///
    /// # 错误
    ///
    /// 当配置无效或 Tokio runtime 无法创建时，返回 [`FurnaceIoError`]。
    pub fn from_env() -> Result<Self, FurnaceIoError> {
        Self::from_config(ClickHouseHttpConfig::from_env()?)
    }

    pub fn from_config(config: ClickHouseHttpConfig) -> Result<Self, FurnaceIoError> {
        let runtime = Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|source| {
                FurnaceIoError::Config(format!("failed to create Tokio runtime: {source}"))
            })?;
        let query_timeout = config.query_timeout;
        Ok(Self {
            client: config.client(),
            runtime,
            query_timeout,
        })
    }
}

impl ClickHouseExecutor for ClickHouseHttpExecutor {
    fn fetch_all<T>(&mut self, sql: &str) -> Result<Vec<T>, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send,
    {
        self.runtime
            .block_on(self.client.query(sql).fetch_all::<T>())
            .map_err(FurnaceIoError::from)
    }

    fn fetch_one<T>(&mut self, sql: &str) -> Result<T, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send,
    {
        self.runtime
            .block_on(self.client.query(sql).fetch_one::<T>())
            .map_err(FurnaceIoError::from)
    }

    fn fetch_optional<T>(&mut self, sql: &str) -> Result<Option<T>, FurnaceIoError>
    where
        T: RowOwned + RowRead + Send,
    {
        self.runtime
            .block_on(self.client.query(sql).fetch_optional::<T>())
            .map_err(FurnaceIoError::from)
    }

    fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError> {
        self.runtime
            .block_on(self.client.query(sql).execute())
            .map_err(FurnaceIoError::from)
    }

    fn execute_many(&mut self, sqls: &[String]) -> Result<(), FurnaceIoError> {
        for sql in sqls {
            self.execute(sql)?;
        }
        Ok(())
    }

    fn insert_rows<T>(
        &mut self,
        table: &str,
        rows: &[T],
        batch_size: usize,
    ) -> Result<(), FurnaceIoError>
    where
        T: RowOwned + RowWrite + Clone + Send + Sync,
    {
        if rows.is_empty() {
            return Ok(());
        }
        let timeout = self.query_timeout;
        self.runtime
            .block_on(async {
                for batch in rows.chunks(batch_size) {
                    let mut insert = self.client.insert_unescaped::<T>(table).await?;
                    if let Some(timeout) = timeout {
                        insert = insert.with_timeouts(Some(timeout), Some(timeout));
                    }
                    for row in batch {
                        insert.write(row).await?;
                    }
                    insert.end().await?;
                }
                Ok::<_, clickhouse::error::Error>(())
            })
            .map_err(FurnaceIoError::from)
    }
}

fn parse_bool(value: Option<&str>) -> Option<bool> {
    value.map(|value| matches!(value, "1" | "true" | "TRUE" | "yes" | "YES"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_from_pairs(pairs: &[(&str, &str)]) -> ClickHouseHttpConfig {
        ClickHouseHttpConfig::from_getter(|name| {
            pairs
                .iter()
                .find_map(|(key, value)| (*key == name).then(|| (*value).to_string()))
        })
        .unwrap()
    }

    #[test]
    fn config_prefers_furnace_clickhouse_url() {
        let config = config_from_pairs(&[
            ("FURNACE_CLICKHOUSE_URL", "http://127.0.0.1:34052"),
            ("CLICKHOUSE_HOST", "ignored"),
            ("CLICKHOUSE_PORT", "9000"),
        ]);

        assert_eq!(config.url, "http://127.0.0.1:34052");
    }

    #[test]
    fn config_builds_http_url_from_host_and_port_when_url_is_absent() {
        let config = config_from_pairs(&[
            ("CLICKHOUSE_HOST", "clickhouse"),
            ("CLICKHOUSE_PORT", "8123"),
        ]);

        assert_eq!(config.url, "http://clickhouse:8123");
    }

    #[test]
    fn config_reads_user_password_database_and_validation_flag() {
        let config = config_from_pairs(&[
            ("CLICKHOUSE_USER", "fleur"),
            ("CLICKHOUSE_PASSWORD", "secret"),
            ("CLICKHOUSE_DB", "fleur"),
            ("FURNACE_CLICKHOUSE_VALIDATE_SCHEMA", "false"),
        ]);

        assert_eq!(config.user.as_deref(), Some("fleur"));
        assert_eq!(config.password.as_deref(), Some("secret"));
        assert_eq!(config.database.as_deref(), Some("fleur"));
        assert!(!config.validate_schema);
    }

    #[test]
    fn config_reads_query_timeout() {
        let config = config_from_pairs(&[("CLICKHOUSE_QUERY_TIMEOUT_SECONDS", "300")]);

        assert_eq!(config.query_timeout, Some(Duration::from_secs(300)));
    }
}
