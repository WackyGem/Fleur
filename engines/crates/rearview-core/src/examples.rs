use chrono::NaiveDate;
use serde::Serialize;

use crate::domain::{
    FilterExpr, Operand, Operator, RuleVersionSpec, ScoreClamp, ScoringRule, ScoringSpec,
    UniverseSpec,
};
use crate::error::{RearviewError, RearviewResult};
use crate::strategy_backtest::{
    BacktestAccountConfig, BacktestExecutionConfig, BacktestFeeProfile, BacktestRebalancePolicy,
    BacktestRiskExitPolicy, BacktestSignalPolicy, BacktestSlippageProfile, ExitRuleConfig,
    hash_json,
};

pub const RACINGLINE_0051_LOW_REVERSAL_CASE_ID: &str = "racingline_0051_low_reversal";
pub const RACINGLINE_0051_LOW_REVERSAL_VERSION: &str = "v2";
pub const RACINGLINE_0051_LOW_REVERSAL_BENCHMARK: &str = "000905.SH";

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
    let rule = racingline_0051_low_reversal_rule();
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
            exit_rules: vec![ExitRuleConfig::TimeStopLoss {
                holding_days: 20,
                max_return_pct: 0.0,
            }],
        },
        price_basis: "backward_adjusted".to_string(),
    }
}

fn racingline_0051_low_reversal_rule() -> RuleVersionSpec {
    let kdj_j = Operand::metric("kdj_j_value");
    let forward_close = Operand::metric("close_price_forward_adj");
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
                compare(kdj_j.clone(), Operator::Lt, Operand::number(20.0)),
                compare(
                    Operand::metric("pct_amplitude"),
                    Operator::Lt,
                    Operand::number(5.0),
                ),
                compare(
                    Operand::metric("pct_change"),
                    Operator::Gt,
                    Operand::number(-3.0),
                ),
                compare(
                    Operand::metric("pct_change"),
                    Operator::Lt,
                    Operand::number(3.0),
                ),
                compare(
                    Operand::metric("close_down_streak_days"),
                    Operator::Lt,
                    Operand::number(5.0),
                ),
                compare(
                    Operand::metric("price_ema2_10"),
                    Operator::Gt,
                    Operand::metric("price_avg_ma_14_28_57_114"),
                ),
                compare(
                    forward_close.clone(),
                    Operator::Gt,
                    Operand::metric("price_avg_ma_3_6_12_24"),
                ),
                compare(
                    Operand::metric("volume"),
                    Operator::Lt,
                    Operand::multiply(Operand::metric("prev_volume"), Operand::number(1.0)),
                ),
                compare(price_ma_60.clone(), Operator::Gt, price_ma_114.clone()),
                compare(price_ma_114, Operator::Gt, price_ma_250),
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
                    "n_structure_20_rebound_valid",
                    compare(
                        Operand::metric("n_structure_20_is_valid"),
                        Operator::Eq,
                        Operand::bool(true),
                    ),
                    20.0,
                ),
                points(
                    "below_boll_lower_20_2",
                    compare(
                        forward_close,
                        Operator::Lt,
                        Operand::metric("boll_lower_20_2"),
                    ),
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
                max: 100.0,
            },
        },
        top_n_default: 10,
        output_metrics: vec![
            "boll_lower_20_2".to_string(),
            "close_down_streak_days".to_string(),
            "close_price_forward_adj".to_string(),
            "kdj_j_value".to_string(),
            "n_structure_20_is_valid".to_string(),
            "pct_amplitude".to_string(),
            "pct_change".to_string(),
            "prev_volume".to_string(),
            "price_avg_ma_14_28_57_114".to_string(),
            "price_avg_ma_3_6_12_24".to_string(),
            "price_ema2_10".to_string(),
            "price_ma_114".to_string(),
            "price_ma_20".to_string(),
            "price_ma_250".to_string(),
            "price_ma_60".to_string(),
            "rsi_6".to_string(),
            "volume".to_string(),
            "volume_ma_5".to_string(),
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

fn date_ymd(year: i32, month: u32, day: u32) -> RearviewResult<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        RearviewError::Config(format!(
            "invalid built-in 0051 example date: {year}-{month}-{day}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use crate::domain::{MetricPolicyFile, ScoringRule};
    use crate::strategy_backtest::ExitRuleConfig;

    use super::*;

    #[test]
    fn racingline_0051_config_should_fix_live_start_and_position_limits() {
        let config = racingline_0051_low_reversal_config().unwrap();

        assert_eq!(
            config.planned_live_start_date,
            date_ymd(2024, 1, 2).unwrap()
        );
        assert_eq!(config.version, "v2");
        assert_eq!(config.benchmark_security_code, "000905.SH");
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
    fn racingline_0051_config_should_only_enable_twenty_day_time_stop_loss() {
        let config = racingline_0051_low_reversal_config().unwrap();

        assert_eq!(
            config.execution_config.risk_exit_policy.exit_rules,
            vec![ExitRuleConfig::TimeStopLoss {
                holding_days: 20,
                max_return_pct: 0.0,
            }]
        );
    }

    #[test]
    fn racingline_0051_rule_should_use_strategy_search_loose_filters() {
        let config = racingline_0051_low_reversal_config().unwrap();
        let FilterExpr::All { conditions } = &config.rule.pool_filters else {
            panic!("expected all filters");
        };

        assert_eq!(conditions.len(), 10);
        assert_filter_number(conditions, 0, "kdj_j_value", Operator::Lt, 20.0);
        assert_filter_number(conditions, 1, "pct_amplitude", Operator::Lt, 5.0);
        assert_filter_number(conditions, 2, "pct_change", Operator::Gt, -3.0);
        assert_filter_number(conditions, 3, "pct_change", Operator::Lt, 3.0);
        assert_filter_number(conditions, 4, "close_down_streak_days", Operator::Lt, 5.0);
        assert_filter_binary_multiplier(conditions, 7, "volume", "prev_volume", 1.0);
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

    #[test]
    fn racingline_0051_hashes_should_match_strategy_search_report() {
        let config = racingline_0051_low_reversal_config().unwrap();
        let catalog = test_catalog();

        let rule_hash = config.rule.validate(&catalog).unwrap().rule_hash.0;
        let execution_config_hash = config.execution_config.compute_hash().unwrap();

        assert_eq!(
            rule_hash,
            "115a15f03f9946cebc5de4d5fedc7bd607a7536fd3a6b7b3fd0fd4eac0a8989a"
        );
        assert_eq!(
            execution_config_hash,
            "6cf814ca48e47c76dcde0beff203b003377e7a5ce94bf0430d8343a4490aa0b3"
        );
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

    fn assert_filter_number(
        conditions: &[FilterExpr],
        index: usize,
        expected_left: &str,
        expected_op: Operator,
        expected_right: f64,
    ) {
        assert!(matches!(
            &conditions[index],
            FilterExpr::Compare {
                left: Operand::Metric { name },
                op,
                right: Some(Operand::Number { value }),
            } if name == expected_left && *op == expected_op && *value == expected_right
        ));
    }

    fn assert_filter_binary_multiplier(
        conditions: &[FilterExpr],
        index: usize,
        expected_left: &str,
        expected_right_metric: &str,
        expected_multiplier: f64,
    ) {
        assert!(matches!(
            &conditions[index],
            FilterExpr::Compare {
                left: Operand::Metric { name },
                op: Operator::Lt,
                right: Some(Operand::Binary {
                    op: crate::domain::ArithmeticOp::Multiply,
                    left,
                    right,
                }),
            } if name == expected_left
                && matches!(left.as_ref(), Operand::Metric { name } if name == expected_right_metric)
                && matches!(right.as_ref(), Operand::Number { value } if *value == expected_multiplier)
        ));
    }

    fn test_catalog() -> crate::domain::MetricCatalog {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let policy = MetricPolicyFile::load(manifest_dir.join("config/metric_policy.yml")).unwrap();
        policy
            .into_catalog(
                manifest_dir.join("../../../pipeline/elt/models/marts"),
                "fleur_marts",
            )
            .unwrap()
    }
}
