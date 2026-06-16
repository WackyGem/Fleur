use async_nats::jetstream;
use async_nats::jetstream::consumer::Consumer;
use serde::{Deserialize, Serialize};

use crate::config::NatsConfig;
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTaskMessage {
    pub portfolio_run_id: String,
    pub source_run_id: String,
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
