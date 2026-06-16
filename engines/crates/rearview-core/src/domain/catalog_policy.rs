use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::domain::metric::{MetricCatalog, MetricDefinition, Operator, ValueKind};
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Deserialize)]
pub struct MetricPolicyFile {
    pub metrics: Vec<MetricPolicyEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricPolicyEntry {
    pub logical_metric: String,
    pub source: MetricSourcePolicy,
    pub value_kind: ValueKind,
    pub allow_filter: bool,
    pub allow_scoring: bool,
    pub allowed_ops: std::collections::BTreeSet<Operator>,
    pub null_policy: crate::domain::metric::NullPolicy,
    #[serde(default)]
    pub default_output: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricSourcePolicy {
    #[serde(default)]
    pub mart_database: Option<String>,
    pub mart_table: String,
    pub column_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DbtSchemaFile {
    #[serde(default)]
    models: Vec<DbtModel>,
}

#[derive(Debug, Clone, Deserialize)]
struct DbtModel {
    name: String,
    #[serde(default)]
    columns: Vec<DbtColumn>,
}

#[derive(Debug, Clone, Deserialize)]
struct DbtColumn {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    data_type: Option<String>,
}

#[derive(Debug, Clone)]
struct DbtColumnFact {
    data_type: String,
    description: Option<String>,
}

type DbtFieldFacts = BTreeMap<(String, String), DbtColumnFact>;

impl MetricPolicyFile {
    pub fn load(path: impl AsRef<Path>) -> RearviewResult<Self> {
        let content = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn into_catalog(
        self,
        dbt_marts_dir: impl AsRef<Path>,
        default_mart_database: &str,
    ) -> RearviewResult<MetricCatalog> {
        let dbt_facts = load_dbt_field_facts(dbt_marts_dir)?;
        let mut metrics = Vec::with_capacity(self.metrics.len());
        for entry in self.metrics {
            metrics.push(entry.into_definition(&dbt_facts, default_mart_database)?);
        }
        MetricCatalog::new(metrics)
    }
}

impl MetricPolicyEntry {
    fn into_definition(
        self,
        dbt_facts: &DbtFieldFacts,
        default_mart_database: &str,
    ) -> RearviewResult<MetricDefinition> {
        let key = (
            self.source.mart_table.clone(),
            self.source.column_name.clone(),
        );
        let fact = dbt_facts.get(&key).ok_or_else(|| {
            RearviewError::MetricCatalog(format!(
                "policy metric {} references missing dbt field {}.{}",
                self.logical_metric, self.source.mart_table, self.source.column_name
            ))
        })?;
        let dbt_kind = value_kind_from_clickhouse_type(&fact.data_type)?;
        if !value_kind_compatible(self.value_kind, dbt_kind) {
            return Err(RearviewError::MetricCatalog(format!(
                "policy metric {} declares {:?}, but dbt field {}.{} has type {}",
                self.logical_metric,
                self.value_kind,
                self.source.mart_table,
                self.source.column_name,
                fact.data_type
            )));
        }

        Ok(MetricDefinition {
            logical_metric: self.logical_metric,
            mart_database: self
                .source
                .mart_database
                .unwrap_or_else(|| default_mart_database.to_string()),
            mart_table: self.source.mart_table,
            column_name: self.source.column_name,
            value_kind: self.value_kind,
            allow_filter: self.allow_filter,
            allow_scoring: self.allow_scoring,
            allowed_ops: self.allowed_ops,
            null_policy: self.null_policy,
            default_output: self.default_output,
            description: self.description.or_else(|| fact.description.clone()),
        })
    }
}

fn load_dbt_field_facts(dbt_marts_dir: impl AsRef<Path>) -> RearviewResult<DbtFieldFacts> {
    let mut facts = BTreeMap::new();
    for entry in fs::read_dir(dbt_marts_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yml") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        let schema: DbtSchemaFile = serde_yaml::from_str(&content)?;
        for model in schema.models {
            for column in model.columns {
                let Some(data_type) = column.data_type else {
                    continue;
                };
                facts.insert(
                    (model.name.clone(), column.name),
                    DbtColumnFact {
                        data_type,
                        description: column.description,
                    },
                );
            }
        }
    }
    Ok(facts)
}

fn value_kind_from_clickhouse_type(data_type: &str) -> RearviewResult<ValueKind> {
    let normalized = data_type
        .trim()
        .strip_prefix("Nullable(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(data_type)
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "float32" | "float64" | "decimal" | "decimal32" | "decimal64" | "decimal128" => {
            Ok(ValueKind::Numeric)
        }
        "int8" | "int16" | "int32" | "int64" | "uint8" | "uint16" | "uint32" | "uint64" => {
            Ok(ValueKind::Integer)
        }
        "bool" | "boolean" => Ok(ValueKind::Boolean),
        "string" | "lowcardinality(string)" => Ok(ValueKind::String),
        "date" | "datetime" | "datetime64" => Ok(ValueKind::Date),
        other => Err(RearviewError::MetricCatalog(format!(
            "unsupported dbt data_type for metric catalog: {other}"
        ))),
    }
}

fn value_kind_compatible(policy_kind: ValueKind, dbt_kind: ValueKind) -> bool {
    if matches!(policy_kind, ValueKind::Numeric | ValueKind::Integer)
        && matches!(dbt_kind, ValueKind::Numeric | ValueKind::Integer)
    {
        return true;
    }
    policy_kind == dbt_kind
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn policy_should_build_catalog_when_dbt_field_exists() {
        let root = create_temp_dir("policy_should_build_catalog_when_dbt_field_exists");
        let marts = root.join("marts");
        fs::create_dir_all(&marts).unwrap();
        fs::write(
            marts.join("mart_stock_quotes_daily.yml"),
            r#"
version: 2
models:
  - name: mart_stock_quotes_daily
    columns:
      - name: close_price
        data_type: Nullable(Float64)
"#,
        )
        .unwrap();
        let policy = MetricPolicyFile {
            metrics: vec![MetricPolicyEntry {
                logical_metric: "close_price".to_string(),
                source: MetricSourcePolicy {
                    mart_database: None,
                    mart_table: "mart_stock_quotes_daily".to_string(),
                    column_name: "close_price".to_string(),
                },
                value_kind: ValueKind::Numeric,
                allow_filter: true,
                allow_scoring: true,
                allowed_ops: std::collections::BTreeSet::from([Operator::Lt]),
                null_policy: crate::domain::metric::NullPolicy::NoMatch,
                default_output: true,
                description: None,
            }],
        };

        let catalog = policy.into_catalog(&marts, "fleur_marts").unwrap();

        assert!(catalog.get("close_price").is_some());
    }

    #[test]
    fn policy_should_fail_when_dbt_field_is_missing() {
        let root = create_temp_dir("policy_should_fail_when_dbt_field_is_missing");
        let marts = root.join("marts");
        fs::create_dir_all(&marts).unwrap();
        fs::write(
            marts.join("mart_stock_quotes_daily.yml"),
            r#"
version: 2
models:
  - name: mart_stock_quotes_daily
    columns: []
"#,
        )
        .unwrap();
        let policy = MetricPolicyFile {
            metrics: vec![MetricPolicyEntry {
                logical_metric: "close_price".to_string(),
                source: MetricSourcePolicy {
                    mart_database: None,
                    mart_table: "mart_stock_quotes_daily".to_string(),
                    column_name: "close_price".to_string(),
                },
                value_kind: ValueKind::Numeric,
                allow_filter: true,
                allow_scoring: true,
                allowed_ops: std::collections::BTreeSet::from([Operator::Lt]),
                null_policy: crate::domain::metric::NullPolicy::NoMatch,
                default_output: true,
                description: None,
            }],
        };

        let error = policy.into_catalog(&marts, "fleur_marts").unwrap_err();

        assert!(error.to_string().contains("missing dbt field"));
    }

    fn create_temp_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("rearview-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
