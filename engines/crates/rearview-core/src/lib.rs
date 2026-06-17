//! Rearview 规则选股 HTTP 服务。
//!
//! 本 crate 提供规则 AST、metric catalog 校验、ClickHouse 查询规划、PostgreSQL 运行状态
//! 和 Axum HTTP API。它只消费 `fleur_marts`，不计算技术指标。

pub mod api;
pub mod clickhouse;
pub mod config;
pub mod domain;
pub mod error;
pub mod nats;
pub mod planner;
pub mod portfolio;
pub mod portfolio_performance;
pub mod portfolio_trade_metrics;
pub mod postgres;
pub mod service;

pub use config::{AppConfig, ClickHouseConfig};
pub use error::{RearviewError, RearviewResult};
