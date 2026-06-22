use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::domain::metric::{
    MetricCatalog, MetricCross, MetricDefinition, MetricDisplay, NullPolicy, Operator, ValueKind,
};
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Deserialize)]
pub struct MetricPolicyFile {
    #[serde(default)]
    pub op_profiles: BTreeMap<String, std::collections::BTreeSet<Operator>>,
    #[serde(default)]
    pub defaults: BTreeMap<String, MetricPolicyDefaults>,
    pub metrics: Vec<MetricPolicyEntry>,
    #[serde(default)]
    pub ignored_fields: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricPolicyEntry {
    pub logical_metric: String,
    pub source: MetricSourcePolicy,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub value_kind: Option<ValueKind>,
    #[serde(default)]
    pub allow_filter: Option<bool>,
    #[serde(default)]
    pub allow_scoring: Option<bool>,
    #[serde(default)]
    pub allowed_ops: Option<std::collections::BTreeSet<Operator>>,
    #[serde(default)]
    pub allowed_ops_profile: Option<String>,
    #[serde(default)]
    pub null_policy: Option<NullPolicy>,
    #[serde(default)]
    pub default_output: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cross: Option<MetricCrossPolicy>,
    #[serde(default)]
    pub display: Option<MetricDisplay>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricPolicyDefaults {
    #[serde(default)]
    pub value_kind: Option<ValueKind>,
    #[serde(default)]
    pub allow_filter: Option<bool>,
    #[serde(default)]
    pub allow_scoring: Option<bool>,
    #[serde(default)]
    pub allowed_ops: Option<std::collections::BTreeSet<Operator>>,
    #[serde(default)]
    pub allowed_ops_profile: Option<String>,
    #[serde(default)]
    pub null_policy: Option<NullPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricCrossPolicy {
    pub previous_metric: String,
    #[serde(default)]
    pub allowed_ops: std::collections::BTreeSet<Operator>,
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
        for entry in &self.metrics {
            metrics.push(entry.clone().into_definition(
                &dbt_facts,
                default_mart_database,
                &self.defaults,
                &self.op_profiles,
            )?);
        }
        let catalog = MetricCatalog::new(metrics)?;
        validate_cross_references(&catalog)?;
        Ok(catalog)
    }

    pub fn check_coverage(&self, dbt_marts_dir: impl AsRef<Path>) -> RearviewResult<usize> {
        let dbt_facts = load_dbt_field_facts(dbt_marts_dir)?;
        let mut source_fields = BTreeMap::<String, std::collections::BTreeSet<String>>::new();
        for metric in &self.metrics {
            source_fields
                .entry(metric.source.mart_table.clone())
                .or_default()
                .insert(metric.source.column_name.clone());
        }

        let mut checked_fields = 0;
        let covered_tables = source_fields
            .keys()
            .chain(self.ignored_fields.keys())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();

        for table in covered_tables {
            let ignored = self.ignored_fields.get(&table);
            if let Some(ignored) = ignored {
                for column in ignored.keys() {
                    if !dbt_facts.contains_key(&(table.clone(), column.clone())) {
                        return Err(RearviewError::MetricCatalog(format!(
                            "ignored field references missing dbt field {table}.{column}"
                        )));
                    }
                }
            }

            for (model_name, column_name) in dbt_facts.keys() {
                if model_name != &table || is_key_column(column_name) {
                    continue;
                }
                checked_fields += 1;
                let is_metric = source_fields
                    .get(&table)
                    .is_some_and(|columns| columns.contains(column_name));
                let is_ignored = ignored.is_some_and(|fields| fields.contains_key(column_name));
                if !is_metric && !is_ignored {
                    return Err(RearviewError::MetricCatalog(format!(
                        "dbt field {table}.{column_name} is not covered by metric policy metrics or ignored_fields"
                    )));
                }
            }
        }

        Ok(checked_fields)
    }
}

impl MetricPolicyEntry {
    fn into_definition(
        self,
        dbt_facts: &DbtFieldFacts,
        default_mart_database: &str,
        defaults: &BTreeMap<String, MetricPolicyDefaults>,
        op_profiles: &BTreeMap<String, std::collections::BTreeSet<Operator>>,
    ) -> RearviewResult<MetricDefinition> {
        let defaults = self
            .extends
            .as_ref()
            .map(|name| {
                defaults.get(name).ok_or_else(|| {
                    RearviewError::MetricCatalog(format!(
                        "policy metric {} extends unknown default profile {name}",
                        self.logical_metric
                    ))
                })
            })
            .transpose()?;
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
        let value_kind = self
            .value_kind
            .or_else(|| defaults.and_then(|value| value.value_kind))
            .ok_or_else(|| {
                RearviewError::MetricCatalog(format!(
                    "policy metric {} is missing value_kind",
                    self.logical_metric
                ))
            })?;
        let dbt_kind = value_kind_from_clickhouse_type(&fact.data_type)?;
        if !value_kind_compatible(value_kind, dbt_kind) {
            return Err(RearviewError::MetricCatalog(format!(
                "policy metric {} declares {:?}, but dbt field {}.{} has type {}",
                self.logical_metric,
                value_kind,
                self.source.mart_table,
                self.source.column_name,
                fact.data_type
            )));
        }
        let mut allowed_ops = resolve_allowed_ops(
            &self.logical_metric,
            self.allowed_ops,
            self.allowed_ops_profile,
            defaults,
            op_profiles,
        )?;
        if let Some(cross) = &self.cross {
            allowed_ops.extend(cross.allowed_ops.iter().copied());
        }
        let null_policy = self
            .null_policy
            .or_else(|| defaults.and_then(|value| value.null_policy))
            .unwrap_or(NullPolicy::NoMatch);

        Ok(MetricDefinition {
            logical_metric: self.logical_metric,
            mart_database: self
                .source
                .mart_database
                .unwrap_or_else(|| default_mart_database.to_string()),
            mart_table: self.source.mart_table,
            column_name: self.source.column_name,
            value_kind,
            allow_filter: self
                .allow_filter
                .or_else(|| defaults.and_then(|value| value.allow_filter))
                .unwrap_or(false),
            allow_scoring: self
                .allow_scoring
                .or_else(|| defaults.and_then(|value| value.allow_scoring))
                .unwrap_or(false),
            allowed_ops,
            null_policy,
            default_output: self.default_output,
            description: self.description.or_else(|| fact.description.clone()),
            cross: self.cross.map(|cross| MetricCross {
                previous_metric: cross.previous_metric,
            }),
            display: self.display,
        })
    }
}

fn resolve_allowed_ops(
    logical_metric: &str,
    explicit_ops: Option<std::collections::BTreeSet<Operator>>,
    explicit_profile: Option<String>,
    defaults: Option<&MetricPolicyDefaults>,
    op_profiles: &BTreeMap<String, std::collections::BTreeSet<Operator>>,
) -> RearviewResult<std::collections::BTreeSet<Operator>> {
    if let Some(ops) = explicit_ops {
        return Ok(ops);
    }
    let profile =
        explicit_profile.or_else(|| defaults.and_then(|value| value.allowed_ops_profile.clone()));
    if let Some(profile) = profile {
        return op_profiles.get(&profile).cloned().ok_or_else(|| {
            RearviewError::MetricCatalog(format!(
                "policy metric {logical_metric} references unknown operator profile {profile}"
            ))
        });
    }
    Ok(defaults
        .and_then(|value| value.allowed_ops.clone())
        .unwrap_or_default())
}

fn validate_cross_references(catalog: &MetricCatalog) -> RearviewResult<()> {
    for metric in catalog.iter() {
        let Some(cross) = &metric.cross else {
            continue;
        };
        let previous = catalog.require(&cross.previous_metric).map_err(|_| {
            RearviewError::MetricCatalog(format!(
                "policy metric {} references missing previous_metric {}",
                metric.logical_metric, cross.previous_metric
            ))
        })?;
        if !previous.is_numeric() {
            return Err(RearviewError::MetricCatalog(format!(
                "policy metric {} references non-numeric previous_metric {}",
                metric.logical_metric, previous.logical_metric
            )));
        }
    }
    Ok(())
}

fn is_key_column(column_name: &str) -> bool {
    matches!(column_name, "security_code" | "trade_date")
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
    fn policy_should_build_catalog_from_defaults_and_profiles() {
        let root = create_temp_dir("policy_should_build_catalog_from_defaults_and_profiles");
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
        let policy = policy_from_yaml(
            r#"
op_profiles:
  numeric_filter: [lt, is_null]
defaults:
  numeric_metric:
    value_kind: numeric
    allow_filter: true
    allow_scoring: true
    allowed_ops_profile: numeric_filter
    null_policy: no_match
metrics:
  - logical_metric: close_price
    source: {mart_table: mart_stock_quotes_daily, column_name: close_price}
    extends: numeric_metric
    default_output: true
"#,
        );

        let catalog = policy.into_catalog(&marts, "fleur_marts").unwrap();

        let metric = catalog.get("close_price").unwrap();
        assert!(metric.allowed_ops.contains(&Operator::Lt));
        assert!(metric.default_output);
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
        let policy = policy_from_yaml(
            r#"
metrics:
  - logical_metric: close_price
    source: {mart_table: mart_stock_quotes_daily, column_name: close_price}
    value_kind: numeric
    allow_filter: true
    allow_scoring: true
    allowed_ops: [lt]
    null_policy: no_match
"#,
        );

        let error = policy.into_catalog(&marts, "fleur_marts").unwrap_err();

        assert!(error.to_string().contains("missing dbt field"));
    }

    #[test]
    fn policy_should_fail_when_previous_metric_is_missing() {
        let root = create_temp_dir("policy_should_fail_when_previous_metric_is_missing");
        let marts = root.join("marts");
        fs::create_dir_all(&marts).unwrap();
        fs::write(
            marts.join("mart_stock_trend_indicator_daily.yml"),
            r#"
version: 2
models:
  - name: mart_stock_trend_indicator_daily
    columns:
      - name: price_ma_5
        data_type: Nullable(Float64)
"#,
        )
        .unwrap();
        let policy = policy_from_yaml(
            r#"
metrics:
  - logical_metric: price_ma_5
    source: {mart_table: mart_stock_trend_indicator_daily, column_name: price_ma_5}
    value_kind: numeric
    allow_filter: true
    allow_scoring: true
    allowed_ops: [lt]
    null_policy: no_match
    cross: {previous_metric: prev_price_ma_5, allowed_ops: [crosses_above]}
"#,
        );

        let error = policy.into_catalog(&marts, "fleur_marts").unwrap_err();

        assert!(error.to_string().contains("missing previous_metric"));
    }

    #[test]
    fn coverage_should_fail_when_dbt_field_is_not_metric_or_ignored() {
        let root = create_temp_dir("coverage_should_fail_when_dbt_field_is_not_metric_or_ignored");
        let marts = root.join("marts");
        fs::create_dir_all(&marts).unwrap();
        fs::write(
            marts.join("mart_stock_quotes_daily.yml"),
            r#"
version: 2
models:
  - name: mart_stock_quotes_daily
    columns:
      - name: security_code
        data_type: String
      - name: trade_date
        data_type: Date
      - name: close_price
        data_type: Nullable(Float64)
      - name: open_price
        data_type: Nullable(Float64)
"#,
        )
        .unwrap();
        let policy = policy_from_yaml(
            r#"
metrics:
  - logical_metric: close_price
    source: {mart_table: mart_stock_quotes_daily, column_name: close_price}
    value_kind: numeric
    allow_filter: true
    allow_scoring: true
    allowed_ops: [lt]
    null_policy: no_match
"#,
        );

        let error = policy.check_coverage(&marts).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("mart_stock_quotes_daily.open_price")
        );
    }

    fn policy_from_yaml(content: &str) -> MetricPolicyFile {
        serde_yaml::from_str(content).unwrap()
    }

    fn create_temp_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("rearview-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
