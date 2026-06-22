use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricDefinition {
    pub logical_metric: String,
    pub mart_database: String,
    pub mart_table: String,
    pub column_name: String,
    pub value_kind: ValueKind,
    pub allow_filter: bool,
    pub allow_scoring: bool,
    pub allowed_ops: BTreeSet<Operator>,
    pub null_policy: NullPolicy,
    #[serde(default)]
    pub default_output: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross: Option<MetricCross>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<MetricDisplay>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricCross {
    pub previous_metric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricDisplay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_zh: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    Numeric,
    Integer,
    Boolean,
    String,
    Date,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Between,
    IsNull,
    CrossesAbove,
    CrossesBelow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NullPolicy {
    NoMatch,
    Match,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct MetricCatalog {
    metrics: BTreeMap<String, MetricDefinition>,
}

impl MetricCatalog {
    pub fn new(metrics: Vec<MetricDefinition>) -> RearviewResult<Self> {
        let mut catalog = Self {
            metrics: BTreeMap::new(),
        };
        for metric in metrics {
            catalog.insert(metric)?;
        }
        Ok(catalog)
    }

    pub fn insert(&mut self, metric: MetricDefinition) -> RearviewResult<()> {
        if self.metrics.contains_key(&metric.logical_metric) {
            return Err(RearviewError::MetricCatalog(format!(
                "duplicate logical metric: {}",
                metric.logical_metric
            )));
        }
        if metric.logical_metric.trim().is_empty() {
            return Err(RearviewError::MetricCatalog(
                "logical metric must not be empty".to_string(),
            ));
        }
        self.metrics.insert(metric.logical_metric.clone(), metric);
        Ok(())
    }

    pub fn get(&self, logical_metric: &str) -> Option<&MetricDefinition> {
        self.metrics.get(logical_metric)
    }

    pub fn require(&self, logical_metric: &str) -> RearviewResult<&MetricDefinition> {
        self.get(logical_metric).ok_or_else(|| {
            RearviewError::Validation(format!("metric is not registered: {logical_metric}"))
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &MetricDefinition> {
        self.metrics.values()
    }
}

impl MetricDefinition {
    pub fn assert_filter_allowed(&self, operator: Operator) -> RearviewResult<()> {
        if !self.allow_filter {
            return Err(RearviewError::Validation(format!(
                "metric {} is not allowed in filters",
                self.logical_metric
            )));
        }
        self.assert_operator_allowed(operator)
    }

    pub fn assert_scoring_allowed(&self, operator: Operator) -> RearviewResult<()> {
        if !self.allow_scoring {
            return Err(RearviewError::Validation(format!(
                "metric {} is not allowed in scoring",
                self.logical_metric
            )));
        }
        self.assert_operator_allowed(operator)
    }

    pub fn assert_operator_allowed(&self, operator: Operator) -> RearviewResult<()> {
        if !self.allowed_ops.contains(&operator) {
            return Err(RearviewError::Validation(format!(
                "operator {:?} is not allowed for metric {}",
                operator, self.logical_metric
            )));
        }
        Ok(())
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self.value_kind, ValueKind::Numeric | ValueKind::Integer)
    }
}

impl ValueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Numeric => "numeric",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
            Self::String => "string",
            Self::Date => "date",
        }
    }
}

impl NullPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoMatch => "no_match",
            Self::Match => "match",
            Self::Error => "error",
        }
    }
}
