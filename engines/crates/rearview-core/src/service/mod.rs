pub mod catalog;
pub mod runner;

use std::sync::Arc;

use crate::clickhouse::ClickHouseClient;
use crate::config::AppConfig;
use crate::domain::MetricCatalog;
use crate::postgres::RearviewPg;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub postgres: RearviewPg,
    pub catalog: MetricCatalog,
    pub clickhouse: ClickHouseClient,
    pub run_semaphore: Arc<Semaphore>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        postgres: RearviewPg,
        catalog: MetricCatalog,
        clickhouse: ClickHouseClient,
    ) -> Self {
        let run_semaphore = Arc::new(Semaphore::new(config.max_concurrent_runs));
        Self {
            config,
            postgres,
            catalog,
            clickhouse,
            run_semaphore,
        }
    }
}
