use std::path::Path;

use crate::domain::{MetricCatalog, MetricPolicyFile};
use crate::error::RearviewResult;

pub fn load_catalog_from_policy(
    policy_path: impl AsRef<Path>,
    dbt_marts_dir: impl AsRef<Path>,
    marts_database: &str,
) -> RearviewResult<MetricCatalog> {
    MetricPolicyFile::load(policy_path)?.into_catalog(dbt_marts_dir, marts_database)
}
