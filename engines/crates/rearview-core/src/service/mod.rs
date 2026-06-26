pub mod catalog;
pub mod runner;

use std::sync::Arc;

use crate::clickhouse::ClickHouseClient;
use crate::config::AppConfig;
use crate::domain::MetricCatalog;
use crate::postgres::RearviewPg;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

const DEFAULT_SERVICE_COMPONENT: &str = "rearview-core";
const DEFAULT_SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub postgres: RearviewPg,
    pub catalog: MetricCatalog,
    pub clickhouse: ClickHouseClient,
    pub run_semaphore: Arc<Semaphore>,
    pub outbox_notifier: Arc<Notify>,
    pub service_component: &'static str,
    pub service_version: &'static str,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        postgres: RearviewPg,
        catalog: MetricCatalog,
        clickhouse: ClickHouseClient,
    ) -> Self {
        Self::new_with_outbox_notifier(
            config,
            postgres,
            catalog,
            clickhouse,
            Arc::new(Notify::new()),
        )
    }

    pub fn new_with_outbox_notifier(
        config: AppConfig,
        postgres: RearviewPg,
        catalog: MetricCatalog,
        clickhouse: ClickHouseClient,
        outbox_notifier: Arc<Notify>,
    ) -> Self {
        Self::new_with_service_identity(
            config,
            postgres,
            catalog,
            clickhouse,
            outbox_notifier,
            DEFAULT_SERVICE_COMPONENT,
            DEFAULT_SERVICE_VERSION,
        )
    }

    pub fn new_with_service_identity(
        config: AppConfig,
        postgres: RearviewPg,
        catalog: MetricCatalog,
        clickhouse: ClickHouseClient,
        outbox_notifier: Arc<Notify>,
        service_component: &'static str,
        service_version: &'static str,
    ) -> Self {
        let run_semaphore = Arc::new(Semaphore::new(config.max_concurrent_runs));
        Self {
            config,
            postgres,
            catalog,
            clickhouse,
            run_semaphore,
            outbox_notifier,
            service_component,
            service_version,
        }
    }
}
