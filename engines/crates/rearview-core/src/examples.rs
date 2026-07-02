use chrono::NaiveDate;
use serde::Serialize;

use crate::domain::{RuleVersionSpec, representative_rule};
use crate::error::{RearviewError, RearviewResult};
use crate::strategy_backtest::{
    BacktestAccountConfig, BacktestExecutionConfig, BacktestFeeProfile, BacktestRebalancePolicy,
    BacktestRiskExitPolicy, BacktestSignalPolicy, BacktestSlippageProfile, ExitRuleConfig,
    hash_json,
};

pub const RACINGLINE_0051_LOW_REVERSAL_CASE_ID: &str = "racingline_0051_low_reversal";
pub const RACINGLINE_0051_LOW_REVERSAL_VERSION: &str = "v1";
pub const RACINGLINE_0051_LOW_REVERSAL_BENCHMARK: &str = "000300.SH";

#[derive(Debug, Clone)]
pub struct Racingline0051LowReversalConfig {
    pub case_id: &'static str,
    pub version: &'static str,
    pub rule: RuleVersionSpec,
    pub execution_config: BacktestExecutionConfig,
    pub benchmark_security_code: &'static str,
    pub planned_live_start_date: NaiveDate,
    pub fixture_hash: String,
}

#[derive(Debug, Serialize)]
struct Racingline0051LowReversalFixture<'a> {
    case_id: &'a str,
    version: &'a str,
    rule_spec: &'a RuleVersionSpec,
    execution_config: &'a BacktestExecutionConfig,
    benchmark_security_code: &'a str,
    planned_live_start_date: NaiveDate,
}

pub fn racingline_0051_low_reversal_config() -> RearviewResult<Racingline0051LowReversalConfig> {
    let rule = representative_rule();
    let execution_config = racingline_0051_low_reversal_execution_config();
    let planned_live_start_date = date_ymd(2024, 1, 2)?;
    let fixture_hash = hash_json(&Racingline0051LowReversalFixture {
        case_id: RACINGLINE_0051_LOW_REVERSAL_CASE_ID,
        version: RACINGLINE_0051_LOW_REVERSAL_VERSION,
        rule_spec: &rule,
        execution_config: &execution_config,
        benchmark_security_code: RACINGLINE_0051_LOW_REVERSAL_BENCHMARK,
        planned_live_start_date,
    })?;
    Ok(Racingline0051LowReversalConfig {
        case_id: RACINGLINE_0051_LOW_REVERSAL_CASE_ID,
        version: RACINGLINE_0051_LOW_REVERSAL_VERSION,
        rule,
        execution_config,
        benchmark_security_code: RACINGLINE_0051_LOW_REVERSAL_BENCHMARK,
        planned_live_start_date,
        fixture_hash,
    })
}

fn racingline_0051_low_reversal_execution_config() -> BacktestExecutionConfig {
    BacktestExecutionConfig {
        market: "CN_A_SHARE".to_string(),
        account: BacktestAccountConfig {
            initial_cash: 1_000_000.0,
            currency: "CNY".to_string(),
        },
        signal_policy: BacktestSignalPolicy {
            buy_signal_top_n: 5,
            signal_timing: "close_confirm_next_open".to_string(),
        },
        rebalance_policy: BacktestRebalancePolicy {
            target_weighting: "equal_weight_capped".to_string(),
            max_positions: 5,
            single_position_limit_pct: Some(0.20),
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
                ExitRuleConfig::TakeProfit { profit_pct: 0.15 },
                ExitRuleConfig::IndicatorStopLoss {
                    source: "trend".to_string(),
                    metric: "price_ma_10".to_string(),
                    operator: "close_below_metric".to_string(),
                },
            ],
        },
        price_basis: "backward_adjusted".to_string(),
    }
}

fn date_ymd(year: i32, month: u32, day: u32) -> RearviewResult<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        RearviewError::Config(format!(
            "invalid built-in 0051 example date: {year}-{month}-{day}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use crate::domain::{FilterExpr, Operand, Operator, ScoringRule};
    use crate::strategy_backtest::ExitRuleConfig;

    use super::*;

    #[test]
    fn racingline_0051_config_should_fix_live_start_and_position_limits() {
        let config = racingline_0051_low_reversal_config().unwrap();

        assert_eq!(
            config.planned_live_start_date,
            date_ymd(2024, 1, 2).unwrap()
        );
        assert_eq!(config.execution_config.signal_policy.buy_signal_top_n, 5);
        assert_eq!(config.execution_config.rebalance_policy.max_positions, 5);
        assert_eq!(
            config
                .execution_config
                .rebalance_policy
                .single_position_limit_pct,
            Some(0.20)
        );
    }

    #[test]
    fn racingline_0051_config_should_only_enable_take_profit_and_ma10_stop_loss() {
        let config = racingline_0051_low_reversal_config().unwrap();

        assert_eq!(
            config.execution_config.risk_exit_policy.exit_rules,
            vec![
                ExitRuleConfig::TakeProfit { profit_pct: 0.15 },
                ExitRuleConfig::IndicatorStopLoss {
                    source: "trend".to_string(),
                    metric: "price_ma_10".to_string(),
                    operator: "close_below_metric".to_string(),
                },
            ]
        );
    }

    #[test]
    fn racingline_0051_rule_should_keep_n_structure_in_step2_at_twenty_points() {
        let config = racingline_0051_low_reversal_config().unwrap();
        let score_rule = config
            .rule
            .scoring
            .rules
            .iter()
            .find(|rule| match rule {
                ScoringRule::ConditionalPoints { name, .. } => {
                    name == "n_structure_20_rebound_valid"
                }
                ScoringRule::WeightedMetric { .. } => false,
            })
            .unwrap();

        match score_rule {
            ScoringRule::ConditionalPoints {
                condition, points, ..
            } => {
                assert_eq!(*points, 20.0);
                assert!(matches!(
                    condition,
                    FilterExpr::Compare {
                        left: Operand::Metric { name },
                        ..
                    } if name == "n_structure_20_is_valid"
                ));
            }
            ScoringRule::WeightedMetric { .. } => panic!("expected conditional points"),
        }
    }

    #[test]
    fn racingline_0051_rule_should_keep_kdj_score_bands_mutually_exclusive() {
        let config = racingline_0051_low_reversal_config().unwrap();

        let high_band = conditional_points_rule(&config.rule, "kdj_j_below_minus_15");
        assert_eq!(high_band.1, 25.0);
        assert!(matches!(
            high_band.0,
            FilterExpr::Compare {
                left: Operand::Metric { name },
                op: Operator::Lt,
                right: Some(Operand::Number { value }),
            } if name == "kdj_j_value" && *value == -15.0
        ));

        let mid_band = conditional_points_rule(&config.rule, "kdj_j_between_minus_15_and_minus_10");
        assert_eq!(mid_band.1, 15.0);
        assert!(matches!(
            mid_band.0,
            FilterExpr::All { conditions }
                if conditions.len() == 2
                    && matches!(
                        &conditions[0],
                        FilterExpr::Compare {
                            left: Operand::Metric { name },
                            op: Operator::Gte,
                            right: Some(Operand::Number { value }),
                        } if name == "kdj_j_value" && *value == -15.0
                    )
                    && matches!(
                        &conditions[1],
                        FilterExpr::Compare {
                            left: Operand::Metric { name },
                            op: Operator::Lt,
                            right: Some(Operand::Number { value }),
                        } if name == "kdj_j_value" && *value == -10.0
                    )
        ));
    }

    #[test]
    fn racingline_0051_fixture_hash_should_be_stable() {
        let first = racingline_0051_low_reversal_config().unwrap();
        let second = racingline_0051_low_reversal_config().unwrap();

        assert_eq!(first.fixture_hash, second.fixture_hash);
    }

    fn conditional_points_rule<'a>(
        rule: &'a RuleVersionSpec,
        expected_name: &str,
    ) -> (&'a FilterExpr, f64) {
        let Some(ScoringRule::ConditionalPoints {
            condition, points, ..
        }) = rule.scoring.rules.iter().find(|rule| match rule {
            ScoringRule::ConditionalPoints { name, .. } => name == expected_name,
            ScoringRule::WeightedMetric { .. } => false,
        })
        else {
            panic!("missing conditional points rule {expected_name}");
        };
        (condition, *points)
    }
}
