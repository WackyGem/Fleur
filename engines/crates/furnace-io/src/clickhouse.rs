use std::env;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};

use crate::FurnaceIoError;

/// 面向测试和 CLI 适配器的最小 ClickHouse 执行接口。
pub trait ClickHouseExecutor {
    /// 执行查询并返回标准输出。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError>;

    /// 执行查询并返回原始标准输出字节。
    ///
    /// RowBinary 等二进制格式可以避免大规模扫描时的文本解析开销。
    /// 测试执行器可以继续使用默认的 UTF-8 实现。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn query_bytes(&mut self, sql: &str) -> Result<Vec<u8>, FurnaceIoError> {
        self.query(sql).map(String::into_bytes)
    }

    /// 执行 INSERT 语句，并通过 stdin 提供 TSV 行。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError>;

    /// 执行 INSERT 语句，并通过 stdin 提供原始字节。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn insert_bytes(&mut self, sql: &str, bytes: &[u8]) -> Result<(), FurnaceIoError>;

    /// 执行语句并忽略其标准输出。
    ///
    /// # 错误
    ///
    /// 当 ClickHouse 执行失败时，返回 [`FurnaceIoError`]。
    fn execute(&mut self, sql: &str) -> Result<(), FurnaceIoError> {
        self.query(sql).map(|_| ())
    }

    /// 执行多条语句并忽略其标准输出。
    ///
    /// 默认实现会逐条执行语句。基于 CLI 的执行器可以覆盖该方法，
    /// 以减少子进程往返次数。
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
}

/// `clickhouse-client` 子进程执行器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseCliExecutor {
    command: String,
    command_args: Vec<String>,
    host: String,
    port: String,
    user: Option<String>,
    password: Option<String>,
    secure: bool,
    connect_timeout_seconds: Option<String>,
    query_timeout_seconds: Option<String>,
}

impl ClickHouseCliExecutor {
    /// 根据环境变量构造 CLI 执行器。
    ///
    /// 支持的变量包括：`FURNACE_CLICKHOUSE_CLIENT`、`CLICKHOUSE_HOST`、
    /// `FURNACE_CLICKHOUSE_CLIENT_ARGS`, `CLICKHOUSE_NATIVE_PORT`,
    /// `CLICKHOUSE_USER`, `CLICKHOUSE_PASSWORD`, `CLICKHOUSE_SECURE`,
    /// `CLICKHOUSE_CONNECT_TIMEOUT_SECONDS` 和 `CLICKHOUSE_QUERY_TIMEOUT_SECONDS`。
    pub fn from_env() -> Self {
        Self {
            command: env::var("FURNACE_CLICKHOUSE_CLIENT")
                .or_else(|_| env::var("CLICKHOUSE_CLIENT"))
                .unwrap_or_else(|_| "clickhouse-client".to_string()),
            command_args: env::var("FURNACE_CLICKHOUSE_CLIENT_ARGS")
                .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
                .unwrap_or_default(),
            host: env::var("CLICKHOUSE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("CLICKHOUSE_NATIVE_PORT").unwrap_or_else(|_| "9000".to_string()),
            user: env::var("CLICKHOUSE_USER").ok(),
            password: env::var("CLICKHOUSE_PASSWORD").ok(),
            secure: env::var("CLICKHOUSE_SECURE")
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false),
            connect_timeout_seconds: env::var("CLICKHOUSE_CONNECT_TIMEOUT_SECONDS").ok(),
            query_timeout_seconds: env::var("CLICKHOUSE_QUERY_TIMEOUT_SECONDS").ok(),
        }
    }

    fn base_command(&self) -> Command {
        let mut command = Command::new(&self.command);
        command.args(&self.command_args);
        command.arg("--host").arg(&self.host);
        command.arg("--port").arg(&self.port);
        if let Some(user) = &self.user {
            command.arg("--user").arg(user);
        }
        if let Some(password) = &self.password {
            command.arg("--password").arg(password);
        }
        if self.secure {
            command.arg("--secure");
        }
        if let Some(timeout) = &self.connect_timeout_seconds {
            command.arg("--connect_timeout").arg(timeout);
        }
        if let Some(timeout) = &self.query_timeout_seconds {
            command.arg("--receive_timeout").arg(timeout);
            command.arg("--send_timeout").arg(timeout);
        }
        command
    }
}

impl ClickHouseExecutor for ClickHouseCliExecutor {
    fn query(&mut self, sql: &str) -> Result<String, FurnaceIoError> {
        let output = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .output()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn query_bytes(&mut self, sql: &str) -> Result<Vec<u8>, FurnaceIoError> {
        let output = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .output()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(output.stdout)
    }

    fn insert_tsv(&mut self, sql: &str, tsv: &str) -> Result<(), FurnaceIoError> {
        self.insert_bytes(sql, tsv.as_bytes())
    }

    fn insert_bytes(&mut self, sql: &str, bytes: &[u8]) -> Result<(), FurnaceIoError> {
        let mut child = self
            .base_command()
            .arg("--query")
            .arg(sql)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;

        let Some(stdin) = child.stdin.as_mut() else {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: "failed to open clickhouse-client stdin".to_string(),
                source: None,
            });
        };
        stdin
            .write_all(bytes)
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: "failed to write insert bytes to clickhouse-client".to_string(),
                source: Some(source.to_string()),
            })?;

        let output =
            child
                .wait_with_output()
                .map_err(|source| FurnaceIoError::ClickHouseCommand {
                    message: format!("failed to wait for {}", self.command),
                    source: Some(source.to_string()),
                })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(())
    }

    fn execute_many(&mut self, sqls: &[String]) -> Result<(), FurnaceIoError> {
        if sqls.is_empty() {
            return Ok(());
        }
        let mut command = self.base_command();
        for sql in sqls {
            command.arg("--query").arg(sql);
        }
        let output = command
            .output()
            .map_err(|source| FurnaceIoError::ClickHouseCommand {
                message: format!("failed to run {}", self.command),
                source: Some(source.to_string()),
            })?;
        if !output.status.success() {
            return Err(FurnaceIoError::ClickHouseCommand {
                message: format!("clickhouse-client exited with {}", output.status),
                source: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            });
        }
        Ok(())
    }
}
