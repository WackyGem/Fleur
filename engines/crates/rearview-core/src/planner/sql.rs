use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::Serialize;

use crate::domain::{
    ArithmeticOp, FilterExpr, MetricCatalog, MetricDefinition, Operand, Operator, RuleVersionSpec,
    ScoringRule,
};
use crate::error::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Serialize)]
pub struct CompiledQuery {
    pub sql: String,
    pub sql_hash: String,
    pub required_metrics: Vec<String>,
    pub required_marts: Vec<String>,
    pub required_columns: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Copy)]
pub struct QuerySettings {
    pub max_execution_time_seconds: u64,
    pub max_rows_to_read: u64,
    pub max_bytes_to_read: u64,
}

impl Default for QuerySettings {
    fn default() -> Self {
        Self {
            max_execution_time_seconds: 300,
            max_rows_to_read: 1_000_000_000,
            max_bytes_to_read: 100_000_000_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryPlanner {
    catalog: MetricCatalog,
}

impl QueryPlanner {
    pub fn new(catalog: MetricCatalog) -> Self {
        Self { catalog }
    }

    pub fn compile_explain(&self, rule: &RuleVersionSpec) -> RearviewResult<CompiledQuery> {
        self.compile(
            rule,
            None,
            None,
            rule.top_n_default,
            QuerySettings::default(),
        )
    }

    pub fn compile(
        &self,
        rule: &RuleVersionSpec,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        top_n: u32,
        settings: QuerySettings,
    ) -> RearviewResult<CompiledQuery> {
        let report = rule.validate(&self.catalog)?;
        let required_metrics = report
            .dependencies
            .metrics
            .iter()
            .map(|metric| metric.logical_metric.clone())
            .collect::<Vec<_>>();
        let metric_lookup = required_metrics
            .iter()
            .map(|metric| {
                self.catalog
                    .require(metric)
                    .map(|definition| (metric.clone(), definition.clone()))
            })
            .collect::<RearviewResult<BTreeMap<_, _>>>()?;
        let marts = group_metrics_by_mart(&metric_lookup, &rule.universe)?;
        let required_marts = marts.keys().cloned().collect::<Vec<_>>();
        let required_columns = required_columns_by_mart(&marts);
        let ctes = compile_mart_ctes(&marts, start_date, end_date)?;
        let from_sql = compile_join_sql(&marts)?;
        let filter_sql = compile_filter(&rule.pool_filters, &metric_lookup)?;
        let score_sql = compile_score(rule, &metric_lookup)?;
        let score_breakdown_sql = compile_score_breakdown(rule, &metric_lookup)?;
        let selected_metrics_sql =
            compile_selected_metric_json(&rule.output_metrics, &metric_lookup)?;
        let raw_values_sql = compile_raw_values_json(&required_metrics, &metric_lookup)?;
        let universe_sql = compile_universe_filter(rule);
        let where_sql = if universe_sql.is_empty() {
            filter_sql
        } else {
            format!("({universe_sql}) AND ({filter_sql})")
        };
        let sql = format!(
            r#"WITH
{ctes},
pool AS (
    SELECT
        security_code,
        trade_date,
        {score_sql} AS raw_score,
        {score_breakdown_sql} AS score_breakdown,
        {selected_metrics_sql} AS selected_metrics,
        {raw_values_sql} AS raw_values
    FROM {from_sql}
    WHERE {where_sql}
),
scored AS (
    SELECT
        security_code,
        trade_date,
        raw_score,
        greatest({clamp_min}, least({clamp_max}, raw_score)) AS score,
        score_breakdown,
        selected_metrics,
        raw_values
    FROM pool
),
ranked AS (
    SELECT
        security_code,
        trade_date,
        raw_score,
        score,
        row_number() OVER (PARTITION BY trade_date ORDER BY score DESC, security_code ASC) AS signal_rank,
        score_breakdown,
        selected_metrics,
        raw_values
    FROM scored
)
SELECT
    security_code,
    trade_date,
    raw_score,
    score,
    signal_rank,
    signal_rank <= {top_n} AS is_buy_signal,
    score_breakdown,
    selected_metrics,
    raw_values
FROM ranked
ORDER BY trade_date ASC, signal_rank ASC, security_code ASC
SETTINGS
    max_execution_time = {max_execution_time},
    max_rows_to_read = {max_rows_to_read},
    max_bytes_to_read = {max_bytes_to_read},
    timeout_before_checking_execution_speed = 0,
    join_algorithm = 'auto'"#,
            max_execution_time = settings.max_execution_time_seconds,
            max_rows_to_read = settings.max_rows_to_read,
            max_bytes_to_read = settings.max_bytes_to_read,
            clamp_min = rule.scoring.clamp.min,
            clamp_max = rule.scoring.clamp.max,
        );
        let sql_hash = sql_hash(&sql);
        Ok(CompiledQuery {
            sql,
            sql_hash,
            required_metrics,
            required_marts,
            required_columns,
        })
    }
}

#[derive(Debug, Clone)]
struct MartPlan {
    alias: String,
    database: String,
    table: String,
    metrics: Vec<MetricDefinition>,
    extra_columns: BTreeSet<String>,
}

fn group_metrics_by_mart(
    metrics: &BTreeMap<String, MetricDefinition>,
    universe: &crate::domain::UniverseSpec,
) -> RearviewResult<BTreeMap<String, MartPlan>> {
    let mut plans = BTreeMap::<String, MartPlan>::new();
    for metric in metrics.values() {
        validate_identifier(&metric.mart_database)?;
        validate_identifier(&metric.mart_table)?;
        validate_identifier(&metric.column_name)?;
        validate_identifier(&metric.logical_metric)?;
        let key = format!("{}.{}", metric.mart_database, metric.mart_table);
        let alias = format!("m{}", plans.len());
        let plan = plans.entry(key).or_insert_with(|| MartPlan {
            alias,
            database: metric.mart_database.clone(),
            table: metric.mart_table.clone(),
            metrics: Vec::new(),
            extra_columns: BTreeSet::new(),
        });
        plan.metrics.push(metric.clone());
    }

    let quotes_key = "fleur_marts.mart_stock_quotes_daily".to_string();
    if universe.exclude_st || universe.exclude_suspend {
        let plan = plans.entry(quotes_key).or_insert_with(|| MartPlan {
            alias: "m0".to_string(),
            database: "fleur_marts".to_string(),
            table: "mart_stock_quotes_daily".to_string(),
            metrics: Vec::new(),
            extra_columns: BTreeSet::new(),
        });
        if universe.exclude_st {
            plan.extra_columns.insert("is_st".to_string());
        }
        if universe.exclude_suspend {
            plan.extra_columns.insert("is_suspend".to_string());
        }
    }

    for (index, plan) in plans.values_mut().enumerate() {
        plan.alias = format!("m{index}");
    }
    Ok(plans)
}

fn required_columns_by_mart(marts: &BTreeMap<String, MartPlan>) -> BTreeMap<String, Vec<String>> {
    marts
        .iter()
        .map(|(mart, plan)| {
            let mut columns =
                BTreeSet::from(["security_code".to_string(), "trade_date".to_string()]);
            columns.extend(plan.metrics.iter().map(|metric| metric.column_name.clone()));
            columns.extend(plan.extra_columns.iter().cloned());
            (mart.clone(), columns.into_iter().collect())
        })
        .collect()
}

fn compile_mart_ctes(
    marts: &BTreeMap<String, MartPlan>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> RearviewResult<String> {
    marts
        .values()
        .map(|plan| {
            let columns = compile_cte_columns(plan)?;
            let date_filter = compile_date_filter(start_date, end_date);
            Ok(format!(
                r#"{alias} AS (
    SELECT
        {columns}
    FROM {database}.{table}
    WHERE {date_filter}
)"#,
                alias = quote_identifier(&plan.alias),
                columns = columns,
                database = quote_identifier(&plan.database),
                table = quote_identifier(&plan.table),
            ))
        })
        .collect::<RearviewResult<Vec<_>>>()
        .map(|ctes| ctes.join(",\n"))
}

fn compile_cte_columns(plan: &MartPlan) -> RearviewResult<String> {
    let mut columns = vec![
        quote_identifier("security_code"),
        quote_identifier("trade_date"),
    ];
    for metric in &plan.metrics {
        columns.push(format!(
            "{} AS {}",
            quote_identifier(&metric.column_name),
            quote_identifier(&metric.logical_metric)
        ));
    }
    for column in &plan.extra_columns {
        validate_identifier(column)?;
        columns.push(quote_identifier(column));
    }
    Ok(columns.join(",\n        "))
}

fn compile_date_filter(start_date: Option<NaiveDate>, end_date: Option<NaiveDate>) -> String {
    let start = start_date
        .map(|date| format!("toDate('{}')", date.format("%Y-%m-%d")))
        .unwrap_or_else(|| "{start_date:Date}".to_string());
    let end = end_date
        .map(|date| format!("toDate('{}')", date.format("%Y-%m-%d")))
        .unwrap_or_else(|| "{end_date:Date}".to_string());
    format!("trade_date BETWEEN {start} AND {end}")
}

fn compile_join_sql(marts: &BTreeMap<String, MartPlan>) -> RearviewResult<String> {
    let base = marts
        .values()
        .find(|plan| plan.table == "mart_stock_quotes_daily")
        .or_else(|| marts.values().next())
        .ok_or_else(|| RearviewError::Planner("query requires at least one mart".to_string()))?;
    let mut sql = quote_identifier(&base.alias);
    for plan in marts.values().filter(|plan| plan.alias != base.alias) {
        sql.push_str(&format!(
            "\n    ANY LEFT JOIN {} USING (security_code, trade_date)",
            quote_identifier(&plan.alias)
        ));
    }
    Ok(sql)
}

fn compile_filter(
    filter: &FilterExpr,
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    match filter {
        FilterExpr::All { conditions } => conditions
            .iter()
            .map(|condition| compile_filter(condition, metrics).map(|sql| format!("({sql})")))
            .collect::<RearviewResult<Vec<_>>>()
            .map(|items| items.join(" AND ")),
        FilterExpr::Any { conditions } => conditions
            .iter()
            .map(|condition| compile_filter(condition, metrics).map(|sql| format!("({sql})")))
            .collect::<RearviewResult<Vec<_>>>()
            .map(|items| items.join(" OR ")),
        FilterExpr::Not { condition } => {
            Ok(format!("NOT ({})", compile_filter(condition, metrics)?))
        }
        FilterExpr::Compare { left, op, right } => {
            compile_compare(left, *op, right.as_ref(), metrics)
        }
    }
}

fn compile_compare(
    left: &Operand,
    op: Operator,
    right: Option<&Operand>,
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    let left_sql = compile_operand(left, metrics)?;
    if matches!(op, Operator::IsNull) {
        return Ok(format!("{left_sql} IS NULL"));
    }
    let right = right.ok_or_else(|| {
        RearviewError::Planner("comparison right side is required for SQL compilation".to_string())
    })?;
    if matches!(op, Operator::CrossesAbove | Operator::CrossesBelow) {
        return compile_crossing_compare(left, op, right, metrics);
    }
    if matches!(op, Operator::Between) {
        let Operand::Range { min, max } = right else {
            return Err(RearviewError::Planner(
                "between comparison requires range operand".to_string(),
            ));
        };
        return Ok(format!(
            "{left_sql} BETWEEN {} AND {}",
            compile_operand(min, metrics)?,
            compile_operand(max, metrics)?
        ));
    }
    Ok(format!(
        "{left_sql} {} {}",
        operator_sql(op)?,
        compile_operand(right, metrics)?
    ))
}

fn compile_crossing_compare(
    left: &Operand,
    op: Operator,
    right: &Operand,
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    let current_left = compile_operand(left, metrics)?;
    let current_right = compile_operand(right, metrics)?;
    let previous_left = compile_previous_operand(left, metrics)?;
    let previous_right = match right {
        Operand::Metric { .. } => compile_previous_operand(right, metrics)?,
        Operand::Number { .. } => current_right.clone(),
        Operand::Bool { .. }
        | Operand::String { .. }
        | Operand::Range { .. }
        | Operand::Binary { .. } => {
            return Err(RearviewError::Planner(
                "crossing right operand must be a metric or numeric constant".to_string(),
            ));
        }
    };
    let (current_operator, previous_operator) = match op {
        Operator::CrossesAbove => (">", "<="),
        Operator::CrossesBelow => ("<", ">="),
        _ => {
            return Err(RearviewError::Planner(format!(
                "operator {op:?} is not a crossing operator"
            )));
        }
    };
    Ok(format!(
        "({current_left} {current_operator} {current_right} AND {previous_left} {previous_operator} {previous_right})"
    ))
}

fn compile_previous_operand(
    operand: &Operand,
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    let Operand::Metric { name } = operand else {
        return Err(RearviewError::Planner(
            "crossing previous operand must be a metric".to_string(),
        ));
    };
    let metric = metrics
        .get(name)
        .ok_or_else(|| RearviewError::Planner(format!("missing metric in plan: {name}")))?;
    let cross = metric.cross.as_ref().ok_or_else(|| {
        RearviewError::Planner(format!(
            "metric {name} cannot use crossing operators because previous_metric is not configured"
        ))
    })?;
    let previous_metric = metrics.get(&cross.previous_metric).ok_or_else(|| {
        RearviewError::Planner(format!(
            "missing previous metric in plan: {}",
            cross.previous_metric
        ))
    })?;
    Ok(quote_identifier(&previous_metric.logical_metric))
}

fn compile_operand(
    operand: &Operand,
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    match operand {
        Operand::Metric { name } => {
            let metric = metrics
                .get(name)
                .ok_or_else(|| RearviewError::Planner(format!("missing metric in plan: {name}")))?;
            Ok(quote_identifier(&metric.logical_metric))
        }
        Operand::Number { value } => {
            if !value.is_finite() {
                return Err(RearviewError::Planner(
                    "numeric literal must be finite".to_string(),
                ));
            }
            Ok(value.to_string())
        }
        Operand::Bool { value } => Ok(if *value { "true" } else { "false" }.to_string()),
        Operand::String { value } => Ok(format!("'{}'", value.replace('\'', "''"))),
        Operand::Range { .. } => Err(RearviewError::Planner(
            "range operand can only be used with between".to_string(),
        )),
        Operand::Binary { op, left, right } => {
            let operator = match op {
                ArithmeticOp::Multiply => "*",
            };
            Ok(format!(
                "({} {operator} {})",
                compile_operand(left, metrics)?,
                compile_operand(right, metrics)?
            ))
        }
    }
}

fn compile_score(
    rule: &RuleVersionSpec,
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    if rule.scoring.rules.is_empty() {
        return Ok("0".to_string());
    }
    rule.scoring
        .rules
        .iter()
        .map(|scoring_rule| match scoring_rule {
            ScoringRule::ConditionalPoints {
                condition, points, ..
            } => Ok(format!(
                "if({}, {}, 0)",
                compile_filter(condition, metrics)?,
                points
            )),
            ScoringRule::WeightedMetric { metric, weight, .. } => Ok(format!(
                "coalesce({}, 0) * {weight}",
                quote_identifier(metric)
            )),
        })
        .collect::<RearviewResult<Vec<_>>>()
        .map(|items| items.join(" + "))
}

fn compile_score_breakdown(
    rule: &RuleVersionSpec,
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    let mut entries = Vec::new();
    for scoring_rule in &rule.scoring.rules {
        match scoring_rule {
            ScoringRule::ConditionalPoints {
                name,
                condition,
                points,
            } => {
                entries.push(format!(
                    "'{}', if({}, {}, 0)",
                    escape_string(name),
                    compile_filter(condition, metrics)?,
                    points
                ));
            }
            ScoringRule::WeightedMetric {
                name,
                metric,
                weight,
            } => {
                entries.push(format!(
                    "'{}', coalesce({}, 0) * {weight}",
                    escape_string(name),
                    quote_identifier(metric)
                ));
            }
        }
    }
    Ok(format!("toJSONString(map({}))", entries.join(", ")))
}

fn compile_selected_metric_json(
    output_metrics: &[String],
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    compile_metric_json(output_metrics, metrics)
}

fn compile_raw_values_json(
    required_metrics: &[String],
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    compile_metric_json(required_metrics, metrics)
}

fn compile_metric_json(
    metric_names: &[String],
    metrics: &BTreeMap<String, MetricDefinition>,
) -> RearviewResult<String> {
    let mut entries = Vec::new();
    for metric in metric_names {
        let definition = metrics
            .get(metric)
            .ok_or_else(|| RearviewError::Planner(format!("missing metric in plan: {metric}")))?;
        entries.push(format!(
            "'{}', toString({})",
            escape_string(&definition.logical_metric),
            quote_identifier(&definition.logical_metric)
        ));
    }
    Ok(format!("toJSONString(map({}))", entries.join(", ")))
}

fn compile_universe_filter(rule: &RuleVersionSpec) -> String {
    let mut conditions = Vec::new();
    if rule.universe.exclude_st {
        conditions.push("coalesce(is_st, false) = false".to_string());
    }
    if rule.universe.exclude_suspend {
        conditions.push("coalesce(is_suspend, false) = false".to_string());
    }
    conditions.join(" AND ")
}

fn operator_sql(operator: Operator) -> RearviewResult<&'static str> {
    match operator {
        Operator::Eq => Ok("="),
        Operator::Ne => Ok("!="),
        Operator::Lt => Ok("<"),
        Operator::Lte => Ok("<="),
        Operator::Gt => Ok(">"),
        Operator::Gte => Ok(">="),
        Operator::Between | Operator::IsNull | Operator::CrossesAbove | Operator::CrossesBelow => {
            Err(RearviewError::Planner(format!(
                "operator {operator:?} is not binary"
            )))
        }
    }
}

fn validate_identifier(identifier: &str) -> RearviewResult<()> {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return Err(RearviewError::Planner(
            "identifier must not be empty".to_string(),
        ));
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(RearviewError::Planner(format!(
            "invalid identifier: {identifier}"
        )));
    }
    if !chars.all(|char| char == '_' || char.is_ascii_alphanumeric()) {
        return Err(RearviewError::Planner(format!(
            "invalid identifier: {identifier}"
        )));
    }
    Ok(())
}

fn quote_identifier(identifier: &str) -> String {
    format!("`{identifier}`")
}

fn escape_string(value: &str) -> String {
    value.replace('\'', "''")
}

fn sql_hash(sql: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(sql.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::catalog_policy::MetricPolicyFile;
    use crate::domain::representative_rule;

    #[test]
    fn compile_should_filter_each_mart_before_joining() {
        let catalog = test_catalog();
        let planner = QueryPlanner::new(catalog);
        let compiled = planner
            .compile(
                &representative_rule(),
                Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()),
                10,
                QuerySettings::default(),
            )
            .unwrap();

        assert!(
            compiled
                .sql
                .contains("WHERE trade_date BETWEEN toDate('2024-01-01')")
        );
    }

    #[test]
    fn compile_should_emit_safety_settings() {
        let catalog = test_catalog();
        let planner = QueryPlanner::new(catalog);
        let compiled = planner.compile_explain(&representative_rule()).unwrap();

        assert!(compiled.sql.contains("max_rows_to_read"));
    }

    #[test]
    fn compile_should_use_rule_score_clamp() {
        let catalog = test_catalog();
        let planner = QueryPlanner::new(catalog);
        let mut rule = representative_rule();
        rule.scoring.clamp.max = 50.0;
        let compiled = planner.compile_explain(&rule).unwrap();

        assert!(
            compiled
                .sql
                .contains("greatest(0, least(50, raw_score)) AS score")
        );
    }

    #[test]
    fn compile_should_expand_crosses_above_between_metrics() {
        let catalog = test_catalog();
        let planner = QueryPlanner::new(catalog);
        let rule = crossing_rule(
            Operand::metric("price_ma_5"),
            Operator::CrossesAbove,
            Operand::metric("price_ma_20"),
        );

        let compiled = planner.compile_explain(&rule).unwrap();

        assert!(compiled.sql.contains(
            "(`price_ma_5` > `price_ma_20` AND `prev_price_ma_5` <= `prev_price_ma_20`)"
        ));
        assert!(
            compiled
                .required_metrics
                .contains(&"prev_price_ma_5".to_string())
        );
        assert!(
            compiled
                .required_columns
                .get("fleur_marts.mart_stock_trend_indicator_daily")
                .is_some_and(|columns| columns.contains(&"prev_price_ma_20".to_string()))
        );
    }

    #[test]
    fn compile_should_expand_crosses_below_against_constant() {
        let catalog = test_catalog();
        let planner = QueryPlanner::new(catalog);
        let rule = crossing_rule(
            Operand::metric("macd_dif"),
            Operator::CrossesBelow,
            Operand::number(0.0),
        );

        let compiled = planner.compile_explain(&rule).unwrap();

        assert!(
            compiled
                .sql
                .contains("(`macd_dif` < 0 AND `prev_macd_dif` >= 0)")
        );
    }

    #[test]
    fn compile_should_reject_crossing_when_metric_has_no_previous_metric() {
        let catalog = test_catalog();
        let planner = QueryPlanner::new(catalog);
        let rule = crossing_rule(
            Operand::metric("close_price"),
            Operator::CrossesAbove,
            Operand::metric("price_ma_20"),
        );

        let error = planner.compile_explain(&rule).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("operator CrossesAbove is not allowed for metric close_price")
        );
    }

    fn test_catalog() -> MetricCatalog {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let policy = MetricPolicyFile::load(manifest_dir.join("config/metric_policy.yml")).unwrap();
        policy
            .into_catalog(
                manifest_dir.join("../../../pipeline/elt/models/marts"),
                "fleur_marts",
            )
            .unwrap()
    }

    fn crossing_rule(left: Operand, op: Operator, right: Operand) -> RuleVersionSpec {
        RuleVersionSpec {
            universe: crate::domain::UniverseSpec {
                base: "all_a_shares".to_string(),
                exclude_st: false,
                exclude_suspend: false,
                include_security_codes: Vec::new(),
                exclude_security_codes: Vec::new(),
            },
            pool_filters: FilterExpr::Compare {
                left,
                op,
                right: Some(right),
            },
            scoring: crate::domain::ScoringSpec {
                rules: Vec::new(),
                clamp: crate::domain::ScoreClamp {
                    min: 0.0,
                    max: 100.0,
                },
            },
            top_n_default: 20,
            output_metrics: vec!["price_ma_20".to_string()],
        }
    }
}
