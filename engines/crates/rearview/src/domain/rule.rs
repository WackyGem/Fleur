use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::domain::metric::{MetricCatalog, MetricDefinition, Operator, ValueKind};
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleVersionSpec {
    pub universe: UniverseSpec,
    pub pool_filters: FilterExpr,
    pub scoring: ScoringSpec,
    pub top_n_default: u32,
    pub output_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UniverseSpec {
    pub base: String,
    #[serde(default)]
    pub exclude_st: bool,
    #[serde(default)]
    pub exclude_suspend: bool,
    #[serde(default)]
    pub include_security_codes: Vec<String>,
    #[serde(default)]
    pub exclude_security_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilterExpr {
    All {
        conditions: Vec<FilterExpr>,
    },
    Any {
        conditions: Vec<FilterExpr>,
    },
    Not {
        condition: Box<FilterExpr>,
    },
    Compare {
        left: Operand,
        op: Operator,
        #[serde(default)]
        right: Option<Operand>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operand {
    Metric {
        name: String,
    },
    Number {
        value: f64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    Range {
        min: Box<Operand>,
        max: Box<Operand>,
    },
    Binary {
        op: ArithmeticOp,
        left: Box<Operand>,
        right: Box<Operand>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOp {
    Multiply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoringSpec {
    #[serde(default)]
    pub rules: Vec<ScoringRule>,
    pub clamp: ScoreClamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScoringRule {
    ConditionalPoints {
        name: String,
        condition: FilterExpr,
        points: f64,
    },
    WeightedMetric {
        name: String,
        metric: String,
        weight: f64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ScoreClamp {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleDependencySnapshot {
    pub metrics: Vec<MetricDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricDependency {
    pub logical_metric: String,
    pub mart_database: String,
    pub mart_table: String,
    pub column_name: String,
    pub value_kind: ValueKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleHash(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleValidationReport {
    pub dependencies: RuleDependencySnapshot,
    pub rule_hash: RuleHash,
}

#[derive(Debug, Clone, Copy)]
enum ValidationContext {
    Filter,
    Scoring,
}

impl RuleVersionSpec {
    pub fn validate(&self, catalog: &MetricCatalog) -> RearviewResult<RuleValidationReport> {
        if self.top_n_default == 0 {
            return Err(RearviewError::Validation(
                "top_n_default must be greater than 0".to_string(),
            ));
        }
        if self.scoring.clamp.min < 0.0 || self.scoring.clamp.max > 99.0 {
            return Err(RearviewError::Validation(
                "score clamp must stay within [0, 99]".to_string(),
            ));
        }
        if self.scoring.clamp.min > self.scoring.clamp.max {
            return Err(RearviewError::Validation(
                "score clamp min must be <= max".to_string(),
            ));
        }

        self.pool_filters
            .validate(catalog, ValidationContext::Filter)?;
        self.scoring.validate(catalog)?;
        for metric in &self.output_metrics {
            catalog.require(metric)?;
        }

        let dependencies = self.dependency_snapshot(catalog)?;
        let rule_hash = self.rule_hash(&dependencies)?;
        Ok(RuleValidationReport {
            dependencies,
            rule_hash,
        })
    }

    pub fn dependency_snapshot(
        &self,
        catalog: &MetricCatalog,
    ) -> RearviewResult<RuleDependencySnapshot> {
        let mut metric_names = BTreeSet::new();
        self.pool_filters.collect_metrics(&mut metric_names);
        self.scoring.collect_metrics(&mut metric_names);
        metric_names.extend(self.output_metrics.iter().cloned());

        let mut metrics = Vec::with_capacity(metric_names.len());
        for logical_metric in metric_names {
            let metric = catalog.require(&logical_metric)?;
            metrics.push(metric.to_dependency());
        }
        Ok(RuleDependencySnapshot { metrics })
    }

    pub fn rule_hash(&self, dependencies: &RuleDependencySnapshot) -> RearviewResult<RuleHash> {
        let value = json!({
            "universe_snapshot": self.universe,
            "pool_filters": self.pool_filters,
            "scoring": self.scoring,
            "top_n_default": self.top_n_default,
            "output_metrics": self.output_metrics,
            "metric_dependency_snapshot": dependencies,
        });
        let canonical = canonicalize_json(value);
        let bytes = serde_json::to_vec(&canonical)?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Ok(RuleHash(hex::encode(hasher.finalize())))
    }
}

impl FilterExpr {
    fn validate(&self, catalog: &MetricCatalog, context: ValidationContext) -> RearviewResult<()> {
        match self {
            Self::All { conditions } | Self::Any { conditions } => {
                if conditions.is_empty() {
                    return Err(RearviewError::Validation(
                        "boolean expression must contain at least one condition".to_string(),
                    ));
                }
                for condition in conditions {
                    condition.validate(catalog, context)?;
                }
                Ok(())
            }
            Self::Not { condition } => condition.validate(catalog, context),
            Self::Compare { left, op, right } => {
                let left_metric = left.metric_name().ok_or_else(|| {
                    RearviewError::Validation("comparison left side must be a metric".to_string())
                })?;
                let metric = catalog.require(left_metric)?;
                match context {
                    ValidationContext::Filter => metric.assert_filter_allowed(*op)?,
                    ValidationContext::Scoring => metric.assert_scoring_allowed(*op)?,
                }

                if matches!(op, Operator::IsNull) {
                    if right.is_some() {
                        return Err(RearviewError::Validation(
                            "is_null comparison must not have a right side".to_string(),
                        ));
                    }
                    return Ok(());
                }

                let right = right.as_ref().ok_or_else(|| {
                    RearviewError::Validation("comparison right side is required".to_string())
                })?;
                validate_operand_pair(metric, *op, right, catalog, context)
            }
        }
    }

    pub fn collect_metrics(&self, output: &mut BTreeSet<String>) {
        match self {
            Self::All { conditions } | Self::Any { conditions } => {
                for condition in conditions {
                    condition.collect_metrics(output);
                }
            }
            Self::Not { condition } => condition.collect_metrics(output),
            Self::Compare { left, right, .. } => {
                left.collect_metrics(output);
                if let Some(right) = right {
                    right.collect_metrics(output);
                }
            }
        }
    }
}

impl Operand {
    pub fn metric(name: impl Into<String>) -> Self {
        Self::Metric { name: name.into() }
    }

    pub fn number(value: f64) -> Self {
        Self::Number { value }
    }

    pub fn bool(value: bool) -> Self {
        Self::Bool { value }
    }

    pub fn multiply(left: Operand, right: Operand) -> Self {
        Self::Binary {
            op: ArithmeticOp::Multiply,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn metric_name(&self) -> Option<&str> {
        match self {
            Self::Metric { name } => Some(name),
            Self::Number { .. }
            | Self::Bool { .. }
            | Self::String { .. }
            | Self::Range { .. }
            | Self::Binary { .. } => None,
        }
    }

    fn collect_metrics(&self, output: &mut BTreeSet<String>) {
        match self {
            Self::Metric { name } => {
                output.insert(name.clone());
            }
            Self::Range { min, max } => {
                min.collect_metrics(output);
                max.collect_metrics(output);
            }
            Self::Binary { left, right, .. } => {
                left.collect_metrics(output);
                right.collect_metrics(output);
            }
            Self::Number { .. } | Self::Bool { .. } | Self::String { .. } => {}
        }
    }

    fn infer_kind(
        &self,
        catalog: &MetricCatalog,
        context: ValidationContext,
    ) -> RearviewResult<ValueKind> {
        match self {
            Self::Metric { name } => {
                let metric = catalog.require(name)?;
                match context {
                    ValidationContext::Filter if !metric.allow_filter => {
                        return Err(RearviewError::Validation(format!(
                            "metric {name} is not allowed in filters"
                        )));
                    }
                    ValidationContext::Scoring if !metric.allow_scoring => {
                        return Err(RearviewError::Validation(format!(
                            "metric {name} is not allowed in scoring"
                        )));
                    }
                    ValidationContext::Filter | ValidationContext::Scoring => {}
                }
                Ok(metric.value_kind)
            }
            Self::Number { .. } => Ok(ValueKind::Numeric),
            Self::Bool { .. } => Ok(ValueKind::Boolean),
            Self::String { .. } => Ok(ValueKind::String),
            Self::Range { min, max } => {
                let min_kind = min.infer_kind(catalog, context)?;
                let max_kind = max.infer_kind(catalog, context)?;
                if min_kind != max_kind {
                    return Err(RearviewError::Validation(
                        "range operands must have the same type".to_string(),
                    ));
                }
                Ok(min_kind)
            }
            Self::Binary { op, left, right } => {
                let left_kind = left.infer_kind(catalog, context)?;
                let right_kind = right.infer_kind(catalog, context)?;
                if !is_numeric_kind(left_kind) || !is_numeric_kind(right_kind) {
                    return Err(RearviewError::Validation(format!(
                        "arithmetic operator {op:?} requires numeric operands"
                    )));
                }
                Ok(ValueKind::Numeric)
            }
        }
    }
}

impl ScoringSpec {
    fn validate(&self, catalog: &MetricCatalog) -> RearviewResult<()> {
        for rule in &self.rules {
            match rule {
                ScoringRule::ConditionalPoints { condition, .. } => {
                    condition.validate(catalog, ValidationContext::Scoring)?;
                }
                ScoringRule::WeightedMetric { metric, .. } => {
                    let definition = catalog.require(metric)?;
                    if !definition.allow_scoring {
                        return Err(RearviewError::Validation(format!(
                            "metric {metric} is not allowed in scoring"
                        )));
                    }
                    if !definition.is_numeric() {
                        return Err(RearviewError::Validation(format!(
                            "weighted metric {metric} must be numeric"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn collect_metrics(&self, output: &mut BTreeSet<String>) {
        for rule in &self.rules {
            match rule {
                ScoringRule::ConditionalPoints { condition, .. } => {
                    condition.collect_metrics(output);
                }
                ScoringRule::WeightedMetric { metric, .. } => {
                    output.insert(metric.clone());
                }
            }
        }
    }
}

impl MetricDefinition {
    fn to_dependency(&self) -> MetricDependency {
        MetricDependency {
            logical_metric: self.logical_metric.clone(),
            mart_database: self.mart_database.clone(),
            mart_table: self.mart_table.clone(),
            column_name: self.column_name.clone(),
            value_kind: self.value_kind,
        }
    }
}

pub fn representative_rule() -> RuleVersionSpec {
    let kdj_j = Operand::metric("kdj_j_value");
    let forward_close = Operand::metric("close_price_forward_adj");
    let price_avg_ma_3_6_12_24 = Operand::metric("price_avg_ma_3_6_12_24");
    let price_ma_60 = Operand::metric("price_ma_60");
    let price_ma_114 = Operand::metric("price_ma_114");
    let price_ma_250 = Operand::metric("price_ma_250");

    RuleVersionSpec {
        universe: UniverseSpec {
            base: "all_a_shares".to_string(),
            exclude_st: true,
            exclude_suspend: true,
            include_security_codes: Vec::new(),
            exclude_security_codes: Vec::new(),
        },
        pool_filters: FilterExpr::All {
            conditions: vec![
                compare(kdj_j.clone(), Operator::Lt, Operand::number(13.0)),
                compare(
                    Operand::metric("pct_amplitude"),
                    Operator::Lt,
                    Operand::number(4.0),
                ),
                compare(
                    Operand::metric("pct_change"),
                    Operator::Gt,
                    Operand::number(-2.0),
                ),
                compare(
                    Operand::metric("pct_change"),
                    Operator::Lt,
                    Operand::number(2.0),
                ),
                compare(
                    Operand::metric("close_down_streak_days"),
                    Operator::Lt,
                    Operand::number(4.0),
                ),
                compare(
                    Operand::metric("price_ema2_10"),
                    Operator::Gt,
                    Operand::metric("price_avg_ma_14_28_57_114"),
                ),
                compare(
                    forward_close.clone(),
                    Operator::Gt,
                    price_avg_ma_3_6_12_24.clone(),
                ),
                compare(price_ma_60.clone(), Operator::Gt, price_ma_114.clone()),
                compare(price_ma_114, Operator::Gt, price_ma_250),
                compare(
                    Operand::metric("volume"),
                    Operator::Lt,
                    Operand::multiply(Operand::metric("prev_volume"), Operand::number(0.8)),
                ),
            ],
        },
        scoring: ScoringSpec {
            rules: vec![
                points(
                    "kdj_j_below_minus_15",
                    compare(kdj_j.clone(), Operator::Lt, Operand::number(-15.0)),
                    25.0,
                ),
                points(
                    "kdj_j_between_minus_15_and_minus_10",
                    FilterExpr::All {
                        conditions: vec![
                            compare(kdj_j.clone(), Operator::Gte, Operand::number(-15.0)),
                            compare(kdj_j, Operator::Lt, Operand::number(-10.0)),
                        ],
                    },
                    15.0,
                ),
                points(
                    "volume_dry_up",
                    compare(
                        Operand::metric("volume"),
                        Operator::Lt,
                        Operand::multiply(Operand::metric("volume_ma_5"), Operand::number(0.6)),
                    ),
                    20.0,
                ),
                points(
                    "below_short_average",
                    compare(forward_close.clone(), Operator::Lt, price_avg_ma_3_6_12_24),
                    15.0,
                ),
                points(
                    "between_ma_20_and_ma_60",
                    FilterExpr::All {
                        conditions: vec![
                            compare(
                                Operand::metric("price_ma_20"),
                                Operator::Lt,
                                forward_close.clone(),
                            ),
                            compare(forward_close.clone(), Operator::Lt, price_ma_60),
                        ],
                    },
                    15.0,
                ),
                points(
                    "n_structure_20_second_low_ratio_above_1",
                    compare(
                        Operand::metric("n_structure_20_second_low_ratio"),
                        Operator::Gt,
                        Operand::number(1.0),
                    ),
                    15.0,
                ),
                points(
                    "below_boll_dn_20_2",
                    compare(forward_close, Operator::Lt, Operand::metric("boll_dn_20_2")),
                    15.0,
                ),
                points(
                    "rsi_6_below_25",
                    compare(
                        Operand::metric("rsi_6"),
                        Operator::Lt,
                        Operand::number(25.0),
                    ),
                    5.0,
                ),
            ],
            clamp: ScoreClamp {
                min: 0.0,
                max: 99.0,
            },
        },
        top_n_default: 10,
        output_metrics: vec![
            "close_price_forward_adj".to_string(),
            "kdj_j_value".to_string(),
            "pct_amplitude".to_string(),
            "pct_change".to_string(),
            "rsi_6".to_string(),
            "volume".to_string(),
            "prev_volume".to_string(),
            "volume_ma_5".to_string(),
            "price_ema2_10".to_string(),
            "price_avg_ma_14_28_57_114".to_string(),
            "price_avg_ma_3_6_12_24".to_string(),
            "price_ma_20".to_string(),
            "price_ma_60".to_string(),
            "price_ma_114".to_string(),
            "price_ma_250".to_string(),
            "boll_dn_20_2".to_string(),
            "close_down_streak_days".to_string(),
            "n_structure_20_second_low_ratio".to_string(),
        ],
    }
}

fn compare(left: Operand, op: Operator, right: Operand) -> FilterExpr {
    FilterExpr::Compare {
        left,
        op,
        right: Some(right),
    }
}

fn points(name: impl Into<String>, condition: FilterExpr, points: f64) -> ScoringRule {
    ScoringRule::ConditionalPoints {
        name: name.into(),
        condition,
        points,
    }
}

fn validate_operand_pair(
    metric: &MetricDefinition,
    operator: Operator,
    right: &Operand,
    catalog: &MetricCatalog,
    context: ValidationContext,
) -> RearviewResult<()> {
    let right_kind = right.infer_kind(catalog, context)?;
    match operator {
        Operator::Eq | Operator::Ne => {
            if !compatible_kind(metric.value_kind, right_kind) {
                return Err(type_mismatch(metric, right_kind));
            }
        }
        Operator::Lt | Operator::Lte | Operator::Gt | Operator::Gte => {
            if !is_orderable_kind(metric.value_kind)
                || !compatible_kind(metric.value_kind, right_kind)
            {
                return Err(type_mismatch(metric, right_kind));
            }
        }
        Operator::Between => {
            if !is_orderable_kind(metric.value_kind)
                || !compatible_kind(metric.value_kind, right_kind)
            {
                return Err(type_mismatch(metric, right_kind));
            }
        }
        Operator::IsNull => {}
    }
    Ok(())
}

fn type_mismatch(metric: &MetricDefinition, right_kind: ValueKind) -> RearviewError {
    RearviewError::Validation(format!(
        "metric {} has type {:?}, incompatible with right operand type {:?}",
        metric.logical_metric, metric.value_kind, right_kind
    ))
}

fn compatible_kind(left: ValueKind, right: ValueKind) -> bool {
    if is_numeric_kind(left) && is_numeric_kind(right) {
        return true;
    }
    left == right
}

fn is_numeric_kind(kind: ValueKind) -> bool {
    matches!(kind, ValueKind::Numeric | ValueKind::Integer)
}

fn is_orderable_kind(kind: ValueKind) -> bool {
    matches!(
        kind,
        ValueKind::Numeric | ValueKind::Integer | ValueKind::Date
    )
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize_json).collect()),
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            for key in keys {
                if let Some(value) = map.get(&key) {
                    sorted.insert(key, canonicalize_json(value.clone()));
                }
            }
            Value::Object(sorted)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::domain::metric::{NullPolicy, Operator};

    #[test]
    fn validate_should_accept_representative_rule() {
        let catalog = representative_catalog();
        let report = representative_rule().validate(&catalog).unwrap();

        assert_eq!(report.dependencies.metrics.len(), 18);
    }

    #[test]
    fn representative_rule_should_use_low_reversal_filters_and_clamp() {
        let rule = representative_rule();

        assert_eq!(
            rule.scoring.clamp,
            ScoreClamp {
                min: 0.0,
                max: 99.0
            }
        );
        assert_eq!(rule.scoring.rules.len(), 8);
        assert_eq!(
            match &rule.pool_filters {
                FilterExpr::All { conditions } => conditions.len(),
                _ => 0,
            },
            10
        );
        assert!(
            rule.output_metrics
                .contains(&"close_price_forward_adj".to_string())
        );
        assert!(
            rule.output_metrics
                .contains(&"n_structure_20_second_low_ratio".to_string())
        );
        assert!(rule.output_metrics.contains(&"price_ma_114".to_string()));
        assert!(rule.output_metrics.contains(&"price_ma_250".to_string()));
    }

    #[test]
    fn validate_should_reject_unregistered_rhs_metric() {
        let mut rule = representative_rule();
        rule.pool_filters = compare(
            Operand::metric("volume"),
            Operator::Gt,
            Operand::metric("missing_metric"),
        );
        let catalog = representative_catalog();

        let error = rule.validate(&catalog).unwrap_err();

        assert!(error.to_string().contains("missing_metric"));
    }

    #[test]
    fn rule_hash_should_be_stable_for_same_content() {
        let catalog = representative_catalog();
        let rule = representative_rule();
        let first = rule.validate(&catalog).unwrap().rule_hash;
        let second = rule.validate(&catalog).unwrap().rule_hash;

        assert_eq!(first, second);
    }

    fn representative_catalog() -> MetricCatalog {
        let mut metrics = Vec::new();
        for name in [
            "close_price_forward_adj",
            "prev_volume",
            "volume",
            "pct_amplitude",
            "pct_change",
            "kdj_j_value",
            "rsi_6",
            "price_ema2_10",
            "price_avg_ma_14_28_57_114",
            "price_avg_ma_3_6_12_24",
            "price_ma_20",
            "price_ma_60",
            "price_ma_114",
            "price_ma_250",
            "boll_dn_20_2",
            "volume_ma_5",
            "close_down_streak_days",
            "n_structure_20_second_low_ratio",
        ] {
            metrics.push(metric(name, ValueKind::Numeric));
        }
        MetricCatalog::new(metrics).unwrap()
    }

    fn metric(name: &str, value_kind: ValueKind) -> MetricDefinition {
        MetricDefinition {
            logical_metric: name.to_string(),
            mart_database: "fleur_marts".to_string(),
            mart_table: "mart_stock_quotes_daily".to_string(),
            column_name: name.to_string(),
            value_kind,
            allow_filter: true,
            allow_scoring: true,
            allowed_ops: BTreeSet::from([
                Operator::Eq,
                Operator::Ne,
                Operator::Lt,
                Operator::Lte,
                Operator::Gt,
                Operator::Gte,
                Operator::Between,
                Operator::IsNull,
            ]),
            null_policy: NullPolicy::NoMatch,
            default_output: false,
            description: None,
        }
    }
}
