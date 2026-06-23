use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::domain::metric::MetricCatalog;
use crate::domain::{RuleHash, RuleVersionSpec};
use crate::{RearviewError, RearviewResult};

const TREND_STOP_LOSS_METRICS: &[&str] = &[
    "price_ma_3",
    "price_ma_5",
    "price_ma_6",
    "price_ma_10",
    "price_ma_12",
    "price_ma_14",
    "price_ma_20",
    "price_ma_24",
    "price_ma_28",
    "price_ma_30",
    "price_ma_57",
    "price_ma_60",
    "price_ma_114",
    "price_ma_250",
    "price_avg_ma_3_6_12_24",
    "price_avg_ma_14_28_57_114",
    "price_ema2_10",
];

#[derive(Debug, Clone, Deserialize)]
pub struct StrategyBacktestValidateRequest {
    pub rule: RuleVersionSpec,
    #[serde(default)]
    pub preview_id: Option<String>,
    #[serde(default)]
    pub preview_range: Option<BacktestDateRange>,
    pub execution_config: BacktestExecutionConfig,
    #[serde(default)]
    pub range: Option<BacktestDateRange>,
    #[serde(default)]
    pub benchmark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BacktestDateRange {
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacktestExecutionConfig {
    pub market: String,
    pub account: BacktestAccountConfig,
    pub signal_policy: BacktestSignalPolicy,
    pub rebalance_policy: BacktestRebalancePolicy,
    pub fee_profile: BacktestFeeProfile,
    pub slippage_profile: BacktestSlippageProfile,
    pub risk_exit_policy: BacktestRiskExitPolicy,
    pub price_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacktestAccountConfig {
    pub initial_cash: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacktestSignalPolicy {
    pub buy_signal_top_n: u32,
    pub signal_timing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacktestRebalancePolicy {
    pub target_weighting: String,
    pub max_positions: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single_position_limit_pct: Option<f64>,
    #[serde(default)]
    pub cash_reserve_pct: f64,
    #[serde(default = "default_lot_size")]
    pub lot_size: u32,
    #[serde(default = "default_min_trade_lots")]
    pub min_trade_lots: u32,
    pub empty_signal_action: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BacktestFeeProfile {
    pub commission_rate: f64,
    pub commission_rate_max: f64,
    pub min_commission: f64,
    pub stamp_duty_rate_sell: f64,
    pub transfer_fee_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacktestSlippageProfile {
    #[serde(default = "default_slippage_mode")]
    pub mode: String,
    pub buy_bps: f64,
    pub sell_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacktestRiskExitPolicy {
    pub trigger_timing: String,
    #[serde(default)]
    pub exit_rules: Vec<ExitRuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExitRuleConfig {
    FixedStopLoss {
        loss_pct: f64,
    },
    TakeProfit {
        profit_pct: f64,
    },
    TimeStopLoss {
        holding_days: u32,
        max_return_pct: f64,
    },
    IndicatorStopLoss {
        #[serde(default)]
        source: String,
        #[serde(default)]
        metric: String,
        #[serde(default)]
        operator: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StrategyBacktestDraftResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_range: Option<BacktestDateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<BacktestDateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark: Option<String>,
    pub execution_config: BacktestExecutionConfig,
    pub rule_hash: String,
    pub execution_config_hash: String,
    pub summary: BacktestExecutionSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct BacktestExecutionSummary {
    pub buy_signal_top_n: u32,
    pub max_positions: usize,
    pub target_weight_per_position_pct: f64,
    pub implicit_cash_reserve_pct: f64,
    pub enabled_exit_rule_count: usize,
}

impl StrategyBacktestValidateRequest {
    pub fn validate(
        self,
        catalog: &MetricCatalog,
    ) -> RearviewResult<StrategyBacktestDraftResponse> {
        let rule_report = self.rule.validate(catalog)?;
        let execution_config = self.execution_config.canonicalized()?;
        let execution_config_hash = execution_config.compute_hash()?;
        let summary = execution_config.summary()?;

        Ok(StrategyBacktestDraftResponse {
            preview_id: self.preview_id,
            preview_range: self.preview_range,
            range: self.range,
            benchmark: self.benchmark,
            execution_config,
            rule_hash: rule_report.rule_hash.0,
            execution_config_hash,
            summary,
            warnings: Vec::new(),
        })
    }
}

impl BacktestExecutionConfig {
    pub fn canonicalized(mut self) -> RearviewResult<Self> {
        if self.signal_policy.signal_timing.trim().is_empty() {
            self.signal_policy.signal_timing = "close_confirm_next_open".to_string();
        }
        if self.rebalance_policy.target_weighting.trim().is_empty()
            || self.rebalance_policy.target_weighting == "equal_weight"
        {
            self.rebalance_policy.target_weighting = "equal_weight_capped".to_string();
        }
        if self.rebalance_policy.empty_signal_action.trim().is_empty() {
            self.rebalance_policy.empty_signal_action = "hold".to_string();
        }
        if self.slippage_profile.mode.trim().is_empty() {
            self.slippage_profile.mode = default_slippage_mode();
        }
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> RearviewResult<()> {
        validate_exact_string("execution_config.market", &self.market, "CN_A_SHARE")?;
        validate_exact_string(
            "execution_config.account.currency",
            &self.account.currency,
            "CNY",
        )?;
        validate_positive(
            "execution_config.account.initial_cash",
            self.account.initial_cash,
        )?;
        if self.signal_policy.buy_signal_top_n == 0 {
            return Err(RearviewError::Validation(
                "execution_config.signal_policy.buy_signal_top_n must be greater than 0"
                    .to_string(),
            ));
        }
        validate_exact_string(
            "execution_config.signal_policy.signal_timing",
            &self.signal_policy.signal_timing,
            "close_confirm_next_open",
        )?;
        validate_exact_string(
            "execution_config.rebalance_policy.target_weighting",
            &self.rebalance_policy.target_weighting,
            "equal_weight_capped",
        )?;
        if self.rebalance_policy.max_positions == 0 {
            return Err(RearviewError::Validation(
                "execution_config.rebalance_policy.max_positions must be greater than 0"
                    .to_string(),
            ));
        }
        let Some(single_position_limit_pct) = self.rebalance_policy.single_position_limit_pct
        else {
            return Err(RearviewError::Validation(
                "execution_config.rebalance_policy.single_position_limit_pct is required"
                    .to_string(),
            ));
        };
        validate_open_closed_pct(
            "execution_config.rebalance_policy.single_position_limit_pct",
            single_position_limit_pct,
        )?;
        validate_cash_reserve_pct(self.rebalance_policy.cash_reserve_pct)?;
        if self.rebalance_policy.lot_size == 0 || self.rebalance_policy.min_trade_lots == 0 {
            return Err(RearviewError::Validation(
                "execution_config.rebalance_policy.lot_size and min_trade_lots must be greater than 0"
                    .to_string(),
            ));
        }
        validate_exact_string(
            "execution_config.rebalance_policy.empty_signal_action",
            &self.rebalance_policy.empty_signal_action,
            "hold",
        )?;
        self.fee_profile.validate()?;
        self.slippage_profile.validate()?;
        self.risk_exit_policy.validate()?;
        validate_exact_string(
            "execution_config.price_basis",
            &self.price_basis,
            "backward_adjusted",
        )?;
        Ok(())
    }

    pub fn summary(&self) -> RearviewResult<BacktestExecutionSummary> {
        self.validate()?;
        let target_weight_per_position_pct = self.target_weight_per_position_pct();
        Ok(BacktestExecutionSummary {
            buy_signal_top_n: self.signal_policy.buy_signal_top_n,
            max_positions: self.rebalance_policy.max_positions,
            target_weight_per_position_pct,
            implicit_cash_reserve_pct: 1.0
                - target_weight_per_position_pct * self.rebalance_policy.max_positions as f64,
            enabled_exit_rule_count: self.risk_exit_policy.exit_rules.len(),
        })
    }

    pub fn target_weight_per_position_pct(&self) -> f64 {
        let equal_weight_after_cash_reserve = (1.0 - self.rebalance_policy.cash_reserve_pct)
            / self.rebalance_policy.max_positions as f64;
        self.rebalance_policy
            .single_position_limit_pct
            .map_or(equal_weight_after_cash_reserve, |cap| {
                equal_weight_after_cash_reserve.min(cap)
            })
    }

    pub fn compute_hash(&self) -> RearviewResult<String> {
        hash_json(self)
    }
}

impl BacktestFeeProfile {
    fn validate(&self) -> RearviewResult<()> {
        validate_non_negative(
            "execution_config.fee_profile.commission_rate",
            self.commission_rate,
        )?;
        validate_non_negative(
            "execution_config.fee_profile.commission_rate_max",
            self.commission_rate_max,
        )?;
        validate_non_negative(
            "execution_config.fee_profile.min_commission",
            self.min_commission,
        )?;
        validate_non_negative(
            "execution_config.fee_profile.stamp_duty_rate_sell",
            self.stamp_duty_rate_sell,
        )?;
        validate_non_negative(
            "execution_config.fee_profile.transfer_fee_rate",
            self.transfer_fee_rate,
        )?;
        Ok(())
    }
}

impl BacktestSlippageProfile {
    fn validate(&self) -> RearviewResult<()> {
        validate_exact_string("execution_config.slippage_profile.mode", &self.mode, "bps")?;
        validate_non_negative("execution_config.slippage_profile.buy_bps", self.buy_bps)?;
        validate_non_negative("execution_config.slippage_profile.sell_bps", self.sell_bps)?;
        Ok(())
    }
}

impl BacktestRiskExitPolicy {
    fn validate(&self) -> RearviewResult<()> {
        validate_exact_string(
            "execution_config.risk_exit_policy.trigger_timing",
            &self.trigger_timing,
            "close_confirm_next_open",
        )?;
        for rule in &self.exit_rules {
            rule.validate()?;
        }
        Ok(())
    }
}

impl ExitRuleConfig {
    fn validate(&self) -> RearviewResult<()> {
        match self {
            Self::FixedStopLoss { loss_pct } => {
                validate_open_closed_pct("execution_config.risk_exit_policy.loss_pct", *loss_pct)
            }
            Self::TakeProfit { profit_pct } => {
                validate_positive("execution_config.risk_exit_policy.profit_pct", *profit_pct)
            }
            Self::TimeStopLoss {
                holding_days,
                max_return_pct,
            } => {
                if *holding_days == 0 {
                    return Err(RearviewError::Validation(
                        "execution_config.risk_exit_policy.holding_days must be greater than 0"
                            .to_string(),
                    ));
                }
                validate_finite(
                    "execution_config.risk_exit_policy.max_return_pct",
                    *max_return_pct,
                )
            }
            Self::IndicatorStopLoss {
                source,
                metric,
                operator,
            } => {
                validate_exact_string(
                    "execution_config.risk_exit_policy.indicator_stop_loss.source",
                    source,
                    "trend",
                )?;
                validate_exact_string(
                    "execution_config.risk_exit_policy.indicator_stop_loss.operator",
                    operator,
                    "close_below_metric",
                )?;
                if TREND_STOP_LOSS_METRICS.contains(&metric.as_str()) {
                    Ok(())
                } else {
                    Err(RearviewError::Validation(format!(
                        "execution_config.risk_exit_policy.indicator_stop_loss.metric is not supported: {metric}"
                    )))
                }
            }
        }
    }
}

pub fn hash_rule_and_config(
    rule: &RuleVersionSpec,
    dependencies: &crate::domain::RuleDependencySnapshot,
    config: &BacktestExecutionConfig,
) -> RearviewResult<(RuleHash, String)> {
    Ok((rule.rule_hash(dependencies)?, config.compute_hash()?))
}

fn validate_exact_string(field: &str, value: &str, expected: &str) -> RearviewResult<()> {
    if value == expected {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!(
            "{field} must be {expected}"
        )))
    }
}

fn validate_positive(field: &str, value: f64) -> RearviewResult<()> {
    validate_finite(field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!(
            "{field} must be greater than 0"
        )))
    }
}

fn validate_non_negative(field: &str, value: f64) -> RearviewResult<()> {
    validate_finite(field, value)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!(
            "{field} must be greater than or equal to 0"
        )))
    }
}

fn validate_finite(field: &str, value: f64) -> RearviewResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!("{field} must be finite")))
    }
}

fn validate_open_closed_pct(field: &str, value: f64) -> RearviewResult<()> {
    validate_finite(field, value)?;
    if value > 0.0 && value <= 1.0 {
        Ok(())
    } else {
        Err(RearviewError::Validation(format!(
            "{field} must be within (0, 1]"
        )))
    }
}

fn validate_cash_reserve_pct(value: f64) -> RearviewResult<()> {
    validate_finite("execution_config.rebalance_policy.cash_reserve_pct", value)?;
    if (0.0..1.0).contains(&value) {
        Ok(())
    } else {
        Err(RearviewError::Validation(
            "execution_config.rebalance_policy.cash_reserve_pct must be within [0, 1)".to_string(),
        ))
    }
}

pub fn hash_json(value: &impl Serialize) -> RearviewResult<String> {
    let value = serde_json::to_value(value)?;
    let canonical = canonicalize_json(value);
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize_json).collect()),
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
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

fn default_lot_size() -> u32 {
    100
}

fn default_min_trade_lots() -> u32 {
    1
}

fn default_slippage_mode() -> String {
    "bps".to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::domain::metric::{MetricCatalog, MetricDefinition, NullPolicy, Operator, ValueKind};
    use crate::domain::{FilterExpr, Operand, ScoreClamp, ScoringRule, ScoringSpec, UniverseSpec};

    use super::*;

    #[test]
    fn summary_should_apply_single_position_cap_when_top_n_is_small() {
        let config = fixture_config(5, 0.10).canonicalized().unwrap();

        let summary = config.summary().unwrap();

        assert_eq!(summary.target_weight_per_position_pct, 0.10);
        assert_eq!(summary.implicit_cash_reserve_pct, 0.5);
    }

    #[test]
    fn summary_should_use_equal_weight_when_top_n_is_larger_than_cap() {
        let config = fixture_config(20, 0.10).canonicalized().unwrap();

        let summary = config.summary().unwrap();

        assert_eq!(summary.target_weight_per_position_pct, 0.05);
        assert_eq!(summary.implicit_cash_reserve_pct, 0.0);
    }

    #[test]
    fn canonicalized_should_preserve_independent_top_n_and_max_positions() {
        let mut config = fixture_config(3, 0.10);
        config.rebalance_policy.max_positions = 5;

        let canonical = config.canonicalized().unwrap();

        assert_eq!(canonical.signal_policy.buy_signal_top_n, 3);
        assert_eq!(canonical.rebalance_policy.max_positions, 5);
    }

    #[test]
    fn canonicalized_should_accept_trend_indicator_stop_loss() {
        let mut config = fixture_config(10, 0.10);
        config.risk_exit_policy.exit_rules = vec![ExitRuleConfig::IndicatorStopLoss {
            source: "trend".to_string(),
            metric: "price_ma_10".to_string(),
            operator: "close_below_metric".to_string(),
        }];

        let canonical = config.canonicalized().unwrap();

        assert_eq!(canonical.risk_exit_policy.exit_rules.len(), 1);
    }

    #[test]
    fn canonicalized_should_accept_main_chart_moving_average_stop_loss() {
        for metric in [
            "price_ma_3",
            "price_avg_ma_3_6_12_24",
            "price_avg_ma_14_28_57_114",
            "price_ema2_10",
        ] {
            let mut config = fixture_config(10, 0.10);
            config.risk_exit_policy.exit_rules = vec![ExitRuleConfig::IndicatorStopLoss {
                source: "trend".to_string(),
                metric: metric.to_string(),
                operator: "close_below_metric".to_string(),
            }];

            config
                .canonicalized()
                .unwrap_or_else(|error| panic!("{metric} should be accepted: {error}"));
        }
    }

    #[test]
    fn canonicalized_should_reject_non_trend_indicator_stop_loss() {
        let mut config = fixture_config(10, 0.10);
        config.risk_exit_policy.exit_rules = vec![ExitRuleConfig::IndicatorStopLoss {
            source: "momentum".to_string(),
            metric: "price_ma_10".to_string(),
            operator: "close_below_metric".to_string(),
        }];

        let error = config.canonicalized().unwrap_err();

        assert!(error.to_string().contains("source"));
    }

    #[test]
    fn canonicalized_should_reject_unknown_indicator_stop_loss_metric() {
        let mut config = fixture_config(10, 0.10);
        config.risk_exit_policy.exit_rules = vec![ExitRuleConfig::IndicatorStopLoss {
            source: "trend".to_string(),
            metric: "unknown_metric".to_string(),
            operator: "close_below_metric".to_string(),
        }];

        let error = config.canonicalized().unwrap_err();

        assert!(error.to_string().contains("metric"));
    }

    #[test]
    fn canonicalized_should_reject_boll_indicator_stop_loss() {
        let mut config = fixture_config(10, 0.10);
        config.risk_exit_policy.exit_rules = vec![ExitRuleConfig::IndicatorStopLoss {
            source: "trend".to_string(),
            metric: "boll_lower_20_2".to_string(),
            operator: "close_below_metric".to_string(),
        }];

        let error = config.canonicalized().unwrap_err();

        assert!(error.to_string().contains("metric"));
    }

    #[test]
    fn canonicalized_should_reject_unsupported_indicator_stop_loss_operator() {
        let mut config = fixture_config(10, 0.10);
        config.risk_exit_policy.exit_rules = vec![ExitRuleConfig::IndicatorStopLoss {
            source: "trend".to_string(),
            metric: "price_ma_10".to_string(),
            operator: "close_above_metric".to_string(),
        }];

        let error = config.canonicalized().unwrap_err();

        assert!(error.to_string().contains("operator"));
    }

    #[test]
    fn validate_request_should_return_stable_hashes_for_same_rule_and_config() {
        let request = fixture_request();
        let catalog = fixture_catalog();

        let first = request.clone().validate(&catalog).unwrap();
        let second = request.validate(&catalog).unwrap();

        assert_eq!(first.rule_hash, second.rule_hash);
        assert_eq!(first.execution_config_hash, second.execution_config_hash);
    }

    #[test]
    fn validate_request_should_change_only_execution_hash_when_top_n_changes() {
        let catalog = fixture_catalog();
        let first = fixture_request().validate(&catalog).unwrap();
        let mut changed = fixture_request();
        changed.execution_config.signal_policy.buy_signal_top_n = 8;

        let second = changed.validate(&catalog).unwrap();

        assert_eq!(first.rule_hash, second.rule_hash);
        assert_ne!(first.execution_config_hash, second.execution_config_hash);
    }

    #[test]
    fn validate_request_should_accept_trend_indicator_stop_loss() {
        let catalog = fixture_catalog();
        let mut request = fixture_request();
        request.execution_config.risk_exit_policy.exit_rules =
            vec![ExitRuleConfig::IndicatorStopLoss {
                source: "trend".to_string(),
                metric: "price_ma_10".to_string(),
                operator: "close_below_metric".to_string(),
            }];

        let response = request.validate(&catalog).unwrap();

        assert_eq!(response.summary.enabled_exit_rule_count, 1);
    }

    fn fixture_request() -> StrategyBacktestValidateRequest {
        StrategyBacktestValidateRequest {
            rule: fixture_rule(),
            preview_id: Some("preview-1".to_string()),
            preview_range: None,
            execution_config: fixture_config(10, 0.10),
            range: None,
            benchmark: None,
        }
    }

    fn fixture_config(
        buy_signal_top_n: u32,
        single_position_limit_pct: f64,
    ) -> BacktestExecutionConfig {
        BacktestExecutionConfig {
            market: "CN_A_SHARE".to_string(),
            account: BacktestAccountConfig {
                initial_cash: 1_000_000.0,
                currency: "CNY".to_string(),
            },
            signal_policy: BacktestSignalPolicy {
                buy_signal_top_n,
                signal_timing: "close_confirm_next_open".to_string(),
            },
            rebalance_policy: BacktestRebalancePolicy {
                target_weighting: "equal_weight_capped".to_string(),
                max_positions: buy_signal_top_n as usize,
                single_position_limit_pct: Some(single_position_limit_pct),
                cash_reserve_pct: 0.0,
                lot_size: 100,
                min_trade_lots: 1,
                empty_signal_action: "hold".to_string(),
            },
            fee_profile: BacktestFeeProfile {
                commission_rate: 0.0001,
                commission_rate_max: 0.003,
                min_commission: 5.0,
                stamp_duty_rate_sell: 0.0005,
                transfer_fee_rate: 0.00001,
            },
            slippage_profile: BacktestSlippageProfile {
                mode: "bps".to_string(),
                buy_bps: 10.0,
                sell_bps: 10.0,
            },
            risk_exit_policy: BacktestRiskExitPolicy {
                trigger_timing: "close_confirm_next_open".to_string(),
                exit_rules: vec![
                    ExitRuleConfig::FixedStopLoss { loss_pct: 0.08 },
                    ExitRuleConfig::TakeProfit { profit_pct: 0.20 },
                    ExitRuleConfig::TimeStopLoss {
                        holding_days: 20,
                        max_return_pct: 0.0,
                    },
                ],
            },
            price_basis: "backward_adjusted".to_string(),
        }
    }

    fn fixture_rule() -> RuleVersionSpec {
        RuleVersionSpec {
            universe: UniverseSpec {
                base: "all_a_shares".to_string(),
                exclude_st: true,
                exclude_suspend: true,
                include_security_codes: Vec::new(),
                exclude_security_codes: Vec::new(),
            },
            pool_filters: FilterExpr::Compare {
                left: Operand::Metric {
                    name: "close_price".to_string(),
                },
                op: Operator::Gt,
                right: Some(Operand::Number { value: 0.0 }),
            },
            scoring: ScoringSpec {
                rules: vec![ScoringRule::ConditionalPoints {
                    name: "positive close".to_string(),
                    condition: FilterExpr::Compare {
                        left: Operand::Metric {
                            name: "close_price".to_string(),
                        },
                        op: Operator::Gt,
                        right: Some(Operand::Number { value: 0.0 }),
                    },
                    points: 10.0,
                }],
                clamp: ScoreClamp {
                    min: 0.0,
                    max: 100.0,
                },
            },
            top_n_default: 20,
            output_metrics: vec!["close_price".to_string()],
        }
    }

    fn fixture_catalog() -> MetricCatalog {
        MetricCatalog::new(vec![MetricDefinition {
            logical_metric: "close_price".to_string(),
            mart_database: "fleur_marts".to_string(),
            mart_table: "mart_stock_quotes_daily".to_string(),
            column_name: "close_price".to_string(),
            value_kind: ValueKind::Numeric,
            allow_filter: true,
            allow_scoring: true,
            allowed_ops: BTreeSet::from([Operator::Gt]),
            null_policy: NullPolicy::NoMatch,
            default_output: true,
            description: None,
            cross: None,
            display: None,
        }])
        .unwrap()
    }
}
