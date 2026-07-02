pub mod catalog;
pub mod runner;
pub mod strategy_portfolio;

use std::sync::Arc;

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};

use crate::clickhouse::ClickHouseClient;
use crate::config::AppConfig;
use crate::domain::MetricCatalog;
use crate::postgres::RearviewPg;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

const DEFAULT_SERVICE_COMPONENT: &str = "rearview-core";
const DEFAULT_SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type MarketDateProvider = Arc<dyn Fn() -> NaiveDate + Send + Sync>;
pub type MarketNowProvider = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

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
    market_now_provider: MarketNowProvider,
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
            market_now_provider: Arc::new(Utc::now),
        }
    }

    pub fn with_market_date_provider(mut self, market_date_provider: MarketDateProvider) -> Self {
        self.market_now_provider = Arc::new(move || {
            let market_date = market_date_provider();
            let Some(market_midnight) = market_date.and_hms_opt(0, 0, 0) else {
                return Utc::now();
            };
            DateTime::from_naive_utc_and_offset(market_midnight, Utc)
        });
        self
    }

    pub fn with_market_now_provider(mut self, market_now_provider: MarketNowProvider) -> Self {
        self.market_now_provider = market_now_provider;
        self
    }

    pub fn current_market_datetime(&self) -> DateTime<FixedOffset> {
        current_cn_a_share_market_datetime((self.market_now_provider)())
    }

    pub fn current_market_date(&self) -> NaiveDate {
        self.current_market_datetime().date_naive()
    }
}

fn current_cn_a_share_market_datetime(now: DateTime<Utc>) -> DateTime<FixedOffset> {
    let Some(offset) = FixedOffset::east_opt(8 * 60 * 60) else {
        return now.fixed_offset();
    };
    now.with_timezone(&offset)
}
