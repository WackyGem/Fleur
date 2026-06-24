use async_nats::jetstream;
use async_nats::jetstream::consumer::Consumer;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::NatsConfig;
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTaskMessage {
    pub portfolio_run_id: String,
    pub source_run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyBacktestTaskMessage {
    pub kind: String,
    pub run_id: String,
}

impl StrategyBacktestTaskMessage {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            kind: "strategy_backtest".to_string(),
            run_id: run_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyPortfolioDailyRunTaskMessage {
    pub kind: String,
    pub daily_run_id: String,
}

impl StrategyPortfolioDailyRunTaskMessage {
    pub fn new(daily_run_id: impl Into<String>) -> Self {
        Self {
            kind: "strategy_portfolio_daily_run".to_string(),
            daily_run_id: daily_run_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RearviewTaskMessage {
    PortfolioRun {
        portfolio_run_id: String,
        source_run_id: String,
    },
    StrategyBacktest {
        run_id: String,
    },
    StrategyPortfolioDailyRun {
        daily_run_id: String,
    },
}

impl<'de> Deserialize<'de> for RearviewTaskMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let Some(object) = value.as_object() else {
            return Err(de::Error::custom("task message must be a JSON object"));
        };
        match object.get("kind").and_then(Value::as_str) {
            Some("strategy_backtest") => {
                let run_id = object
                    .get("run_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| de::Error::custom("strategy_backtest run_id is required"))?;
                Ok(Self::StrategyBacktest {
                    run_id: run_id.to_string(),
                })
            }
            Some("strategy_portfolio_daily_run") => {
                let daily_run_id = object
                    .get("daily_run_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        de::Error::custom("strategy_portfolio_daily_run daily_run_id is required")
                    })?;
                Ok(Self::StrategyPortfolioDailyRun {
                    daily_run_id: daily_run_id.to_string(),
                })
            }
            Some("portfolio_run") => {
                let portfolio_run_id = object
                    .get("portfolio_run_id")
                    .or_else(|| object.get("run_id"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| de::Error::custom("portfolio_run_id is required"))?;
                let source_run_id = object
                    .get("source_run_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| de::Error::custom("source_run_id is required"))?;
                Ok(Self::PortfolioRun {
                    portfolio_run_id: portfolio_run_id.to_string(),
                    source_run_id: source_run_id.to_string(),
                })
            }
            Some(other) => Err(de::Error::custom(format!("unsupported task kind: {other}"))),
            None => {
                let legacy = serde_json::from_value::<PortfolioTaskMessage>(value)
                    .map_err(de::Error::custom)?;
                Ok(Self::PortfolioRun {
                    portfolio_run_id: legacy.portfolio_run_id,
                    source_run_id: legacy.source_run_id,
                })
            }
        }
    }
}

pub async fn connect_jetstream(config: &NatsConfig) -> RearviewResult<jetstream::Context> {
    let client = async_nats::connect(&config.url)
        .await
        .map_err(|error| RearviewError::Nats(error.to_string()))?;
    Ok(jetstream::new(client))
}

pub async fn ensure_portfolio_stream(
    jetstream: &jetstream::Context,
    config: &NatsConfig,
) -> RearviewResult<jetstream::stream::Stream> {
    jetstream
        .get_or_create_stream(jetstream::stream::Config {
            name: config.portfolio_stream.clone(),
            subjects: vec![config.portfolio_request_subject.clone()],
            ..Default::default()
        })
        .await
        .map_err(|error| RearviewError::Nats(error.to_string()))
}

pub async fn ensure_portfolio_consumer(
    stream: &jetstream::stream::Stream,
    config: &NatsConfig,
) -> RearviewResult<Consumer<jetstream::consumer::pull::Config>> {
    stream
        .get_or_create_consumer(
            &config.portfolio_worker_durable,
            jetstream::consumer::pull::Config {
                durable_name: Some(config.portfolio_worker_durable.clone()),
                ..Default::default()
            },
        )
        .await
        .map_err(|error| RearviewError::Nats(error.to_string()))
}

pub async fn publish_portfolio_task(
    jetstream: &jetstream::Context,
    config: &NatsConfig,
    payload: &PortfolioTaskMessage,
) -> RearviewResult<u64> {
    publish_task_payload(jetstream, config, payload).await
}

pub async fn publish_strategy_backtest_task(
    jetstream: &jetstream::Context,
    config: &NatsConfig,
    payload: &StrategyBacktestTaskMessage,
) -> RearviewResult<u64> {
    publish_task_payload(jetstream, config, payload).await
}

pub async fn publish_strategy_portfolio_daily_run_task(
    jetstream: &jetstream::Context,
    config: &NatsConfig,
    payload: &StrategyPortfolioDailyRunTaskMessage,
) -> RearviewResult<u64> {
    publish_task_payload(jetstream, config, payload).await
}

async fn publish_task_payload<T: Serialize>(
    jetstream: &jetstream::Context,
    config: &NatsConfig,
    payload: &T,
) -> RearviewResult<u64> {
    let body = serde_json::to_vec(payload)?;
    let ack = jetstream
        .publish(config.portfolio_request_subject.clone(), body.into())
        .await
        .map_err(|error| RearviewError::Nats(error.to_string()))?;
    let ack = ack
        .await
        .map_err(|error| RearviewError::Nats(error.to_string()))?;
    Ok(ack.sequence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_message_should_parse_legacy_portfolio_payload() {
        let payload = serde_json::json!({
            "portfolio_run_id": "portfolio-1",
            "source_run_id": "run-1"
        });

        let message = serde_json::from_value::<RearviewTaskMessage>(payload).unwrap();

        assert_eq!(
            message,
            RearviewTaskMessage::PortfolioRun {
                portfolio_run_id: "portfolio-1".to_string(),
                source_run_id: "run-1".to_string()
            }
        );
    }

    #[test]
    fn task_message_should_parse_strategy_backtest_payload() {
        let payload = serde_json::json!({
            "kind": "strategy_backtest",
            "run_id": "backtest-1"
        });

        let message = serde_json::from_value::<RearviewTaskMessage>(payload).unwrap();

        assert_eq!(
            message,
            RearviewTaskMessage::StrategyBacktest {
                run_id: "backtest-1".to_string()
            }
        );
    }

    #[test]
    fn task_message_should_parse_strategy_portfolio_daily_run_payload() {
        let payload = serde_json::json!({
            "kind": "strategy_portfolio_daily_run",
            "daily_run_id": "daily-1"
        });

        let message = serde_json::from_value::<RearviewTaskMessage>(payload).unwrap();

        assert_eq!(
            message,
            RearviewTaskMessage::StrategyPortfolioDailyRun {
                daily_run_id: "daily-1".to_string()
            }
        );
    }
}
