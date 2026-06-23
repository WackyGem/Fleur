use std::collections::BTreeMap;

use chrono::NaiveDate;
use sha2::{Digest, Sha256};

use crate::clickhouse::calculation_write::{PerformanceMetricRow, PerformanceMetricStatusRow};
use crate::portfolio::PortfolioNavRow;

pub const CORE_PERFORMANCE_METRICS: [&str; 12] = [
    "holding_period_return",
    "annualized_return",
    "annualized_volatility",
    "max_drawdown",
    "calmar_ratio",
    "downside_deviation",
    "sortino_ratio",
    "sharpe_ratio",
    "information_ratio",
    "beta",
    "alpha",
    "treynor_ratio",
];

#[derive(Debug, Clone)]
pub struct PerformanceMetricConfig {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub security_code: String,
    pub window_key: String,
    pub window_start: Option<NaiveDate>,
    pub window_end: Option<NaiveDate>,
    pub annualization_days: u32,
    pub min_observations: usize,
    pub portfolio_return_basis: String,
    pub benchmark_return_basis: String,
    pub risk_free_tenor: String,
    pub risk_free_daily_method: String,
    pub risk_free_fill_strategy: String,
    pub benchmark_fill_strategy: String,
    pub mar: f64,
    pub mar_basis: String,
    pub alignment_strategy: String,
    pub first_day_return_handling: String,
    pub zero_division_policy: String,
    pub config_version: u32,
    pub config_hash: String,
}

impl PerformanceMetricConfig {
    pub fn default_full_period(portfolio_run_id: &str, result_attempt_id: &str) -> Self {
        Self::full_period_with_benchmark(portfolio_run_id, result_attempt_id, "000300.SH")
    }

    pub fn full_period_with_benchmark(
        portfolio_run_id: &str,
        result_attempt_id: &str,
        benchmark_security_code: &str,
    ) -> Self {
        let mut config = Self {
            portfolio_run_id: portfolio_run_id.to_string(),
            result_attempt_id: result_attempt_id.to_string(),
            security_code: benchmark_security_code.to_string(),
            window_key: "full_period".to_string(),
            window_start: None,
            window_end: None,
            annualization_days: 252,
            min_observations: 20,
            portfolio_return_basis: "price_return".to_string(),
            benchmark_return_basis: "price_index".to_string(),
            risk_free_tenor: "1y".to_string(),
            risk_free_daily_method: "compound".to_string(),
            risk_free_fill_strategy: "forward_fill".to_string(),
            benchmark_fill_strategy: "skip".to_string(),
            mar: 0.0,
            mar_basis: "fixed".to_string(),
            alignment_strategy: "inner_join_trade_dates".to_string(),
            first_day_return_handling: "exclude".to_string(),
            zero_division_policy: "null".to_string(),
            config_version: 1,
            config_hash: String::new(),
        };
        config.config_hash = config.compute_hash();
        config
    }

    pub fn compute_hash(&self) -> String {
        let fields = vec![
            self.portfolio_run_id.clone(),
            self.result_attempt_id.clone(),
            self.security_code.clone(),
            self.window_key.clone(),
            format_optional_date(self.window_start),
            format_optional_date(self.window_end),
            self.annualization_days.to_string(),
            self.min_observations.to_string(),
            self.portfolio_return_basis.clone(),
            self.benchmark_return_basis.clone(),
            self.risk_free_tenor.clone(),
            self.risk_free_daily_method.clone(),
            self.risk_free_fill_strategy.clone(),
            self.benchmark_fill_strategy.clone(),
            format!("{:.10}", self.mar),
            self.mar_basis.clone(),
            self.alignment_strategy.clone(),
            self.first_day_return_handling.clone(),
            self.zero_division_policy.clone(),
            self.config_version.to_string(),
        ];
        let canonical = fields.join("|");
        hex::encode(Sha256::digest(canonical.as_bytes()))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BenchmarkReturn {
    pub trade_date: NaiveDate,
    pub return_daily: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct RiskFreeRate {
    pub trade_date: NaiveDate,
    pub daily_rate: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct AlignedReturn {
    portfolio_return: f64,
    benchmark_return: f64,
    risk_free_return: f64,
}

pub fn compute_performance_metric(
    config: &PerformanceMetricConfig,
    nav_rows: &[PortfolioNavRow],
    benchmark_returns: &[BenchmarkReturn],
    risk_free_rates: &[RiskFreeRate],
) -> (PerformanceMetricRow, Vec<PerformanceMetricStatusRow>) {
    let benchmark_by_date = benchmark_returns
        .iter()
        .filter_map(|row| row.return_daily.map(|value| (row.trade_date, value)))
        .collect::<BTreeMap<_, _>>();
    let risk_free_by_date = risk_free_rates
        .iter()
        .filter_map(|row| row.daily_rate.map(|value| (row.trade_date, value)))
        .collect::<BTreeMap<_, _>>();

    let mut missing_benchmark = false;
    let mut missing_risk_free = false;
    let mut aligned = Vec::new();
    for nav in nav_rows {
        let Some(portfolio_return) = nav.daily_return else {
            continue;
        };
        let Some(benchmark_return) = benchmark_by_date.get(&nav.trade_date).copied() else {
            missing_benchmark = true;
            continue;
        };
        let Some(risk_free_return) = risk_free_by_date.get(&nav.trade_date).copied() else {
            missing_risk_free = true;
            continue;
        };
        if !portfolio_return.is_finite()
            || !benchmark_return.is_finite()
            || !risk_free_return.is_finite()
        {
            return failed_metric_row(config, "invalid_input", "invalid_return_value", 0);
        }
        aligned.push(AlignedReturn {
            portfolio_return,
            benchmark_return,
            risk_free_return,
        });
    }

    if aligned.len() < config.min_observations {
        let (status, reason) = if missing_benchmark {
            ("missing_benchmark", "benchmark_series_missing")
        } else if missing_risk_free {
            ("missing_risk_free_rate", "risk_free_series_missing")
        } else {
            ("insufficient_observations", "n_below_min_observations")
        };
        return failed_metric_row(config, status, reason, aligned.len() as u32);
    }

    let observation_count = aligned.len() as u32;
    let annualization_days = f64::from(config.annualization_days);
    let n = aligned.len() as f64;
    let portfolio_returns = aligned
        .iter()
        .map(|row| row.portfolio_return)
        .collect::<Vec<_>>();
    let benchmark_returns = aligned
        .iter()
        .map(|row| row.benchmark_return)
        .collect::<Vec<_>>();
    let risk_free_returns = aligned
        .iter()
        .map(|row| row.risk_free_return)
        .collect::<Vec<_>>();
    let active_returns = aligned
        .iter()
        .map(|row| row.portfolio_return - row.benchmark_return)
        .collect::<Vec<_>>();

    let holding_period_return = compounded_return(&portfolio_returns);
    let annualized_return = annualize_return(holding_period_return, annualization_days, n);
    let benchmark_holding_period_return = compounded_return(&benchmark_returns);
    let benchmark_annualized_return =
        annualize_return(benchmark_holding_period_return, annualization_days, n);
    let annual_risk_free_rate =
        annualize_return(compounded_return(&risk_free_returns), annualization_days, n);
    let annualized_volatility =
        stddev_sample(&portfolio_returns).map(|v| v * annualization_days.sqrt());
    let downside_deviation = downside_deviation(&portfolio_returns, config.mar)
        .map(|value| value * annualization_days.sqrt());
    let max_drawdown = nav_rows
        .iter()
        .map(|row| row.drawdown)
        .fold(0.0_f64, f64::min)
        .abs();
    let tracking_error =
        stddev_sample(&active_returns).map(|value| value * annualization_days.sqrt());
    let benchmark_variance = variance_sample(&benchmark_returns);
    let beta = match (
        covariance_sample(&portfolio_returns, &benchmark_returns),
        benchmark_variance,
    ) {
        (Some(covariance), Some(variance)) if variance != 0.0 => Some(covariance / variance),
        _ => None,
    };

    let mut statuses = Vec::new();
    let calmar_ratio = ratio_or_status(
        annualized_return,
        Some(max_drawdown),
        "calmar_ratio",
        "max_drawdown_zero",
        &mut statuses,
    );
    let sharpe_ratio = ratio_or_status(
        annualized_return - annual_risk_free_rate,
        annualized_volatility,
        "sharpe_ratio",
        "annualized_volatility_zero",
        &mut statuses,
    );
    let sortino_ratio = ratio_or_status(
        annualized_return - annual_risk_free_rate,
        downside_deviation,
        "sortino_ratio",
        "downside_deviation_zero",
        &mut statuses,
    );
    let information_ratio = ratio_or_status(
        annualized_return - benchmark_annualized_return,
        tracking_error,
        "information_ratio",
        "tracking_error_zero",
        &mut statuses,
    );
    let treynor_ratio = ratio_or_status(
        annualized_return - annual_risk_free_rate,
        beta,
        "treynor_ratio",
        "beta_zero",
        &mut statuses,
    );
    let alpha = beta.map(|beta| {
        annualized_return
            - (annual_risk_free_rate + beta * (benchmark_annualized_return - annual_risk_free_rate))
    });
    if beta.is_none() {
        statuses.push(("beta", "zero_division", "benchmark_variance_zero"));
        statuses.push(("alpha", "zero_division", "benchmark_variance_zero"));
    }

    let row = PerformanceMetricRow {
        portfolio_run_id: config.portfolio_run_id.clone(),
        result_attempt_id: config.result_attempt_id.clone(),
        security_code: config.security_code.clone(),
        window_key: config.window_key.clone(),
        window_start: config.window_start,
        window_end: config.window_end,
        config_hash: config.config_hash.clone(),
        metric_status: "succeeded".to_string(),
        observation_count,
        holding_period_return: Some(holding_period_return),
        annualized_return: Some(annualized_return),
        annualized_volatility,
        max_drawdown: Some(max_drawdown),
        calmar_ratio,
        downside_deviation,
        sortino_ratio,
        sharpe_ratio,
        information_ratio,
        beta,
        alpha,
        treynor_ratio,
    };
    let status_rows = metric_status_rows(config, &statuses);
    (row, status_rows)
}

fn failed_metric_row(
    config: &PerformanceMetricConfig,
    metric_status: &str,
    reason_code: &str,
    observation_count: u32,
) -> (PerformanceMetricRow, Vec<PerformanceMetricStatusRow>) {
    let row = PerformanceMetricRow {
        portfolio_run_id: config.portfolio_run_id.clone(),
        result_attempt_id: config.result_attempt_id.clone(),
        security_code: config.security_code.clone(),
        window_key: config.window_key.clone(),
        window_start: config.window_start,
        window_end: config.window_end,
        config_hash: config.config_hash.clone(),
        metric_status: metric_status.to_string(),
        observation_count,
        holding_period_return: None,
        annualized_return: None,
        annualized_volatility: None,
        max_drawdown: None,
        calmar_ratio: None,
        downside_deviation: None,
        sortino_ratio: None,
        sharpe_ratio: None,
        information_ratio: None,
        beta: None,
        alpha: None,
        treynor_ratio: None,
    };
    let failures = CORE_PERFORMANCE_METRICS
        .iter()
        .map(|metric| (*metric, metric_status, reason_code))
        .collect::<Vec<_>>();
    let status_rows = metric_status_rows(config, &failures);
    (row, status_rows)
}

fn metric_status_rows(
    config: &PerformanceMetricConfig,
    failures: &[(&str, &str, &str)],
) -> Vec<PerformanceMetricStatusRow> {
    let failure_by_metric = failures
        .iter()
        .map(|(metric, status, reason)| (*metric, (*status, *reason)))
        .collect::<BTreeMap<_, _>>();
    CORE_PERFORMANCE_METRICS
        .iter()
        .map(|metric_name| {
            let (metric_status, reason_code) = failure_by_metric
                .get(metric_name)
                .copied()
                .unwrap_or(("succeeded", "none"));
            PerformanceMetricStatusRow {
                portfolio_run_id: config.portfolio_run_id.clone(),
                result_attempt_id: config.result_attempt_id.clone(),
                security_code: config.security_code.clone(),
                window_key: config.window_key.clone(),
                metric_name: (*metric_name).to_string(),
                metric_status: metric_status.to_string(),
                reason_code: reason_code.to_string(),
            }
        })
        .collect()
}

fn ratio_or_status(
    numerator: f64,
    denominator: Option<f64>,
    metric_name: &'static str,
    reason_code: &'static str,
    statuses: &mut Vec<(&'static str, &'static str, &'static str)>,
) -> Option<f64> {
    match denominator {
        Some(value) if value != 0.0 => Some(numerator / value),
        _ => {
            statuses.push((metric_name, "zero_division", reason_code));
            None
        }
    }
}

fn compounded_return(returns: &[f64]) -> f64 {
    returns.iter().fold(1.0, |acc, value| acc * (1.0 + value)) - 1.0
}

fn annualize_return(holding_period_return: f64, annualization_days: f64, n: f64) -> f64 {
    (1.0 + holding_period_return).powf(annualization_days / n) - 1.0
}

fn stddev_sample(values: &[f64]) -> Option<f64> {
    variance_sample(values).map(f64::sqrt)
}

fn variance_sample(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sum_squared = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>();
    Some(sum_squared / (values.len() as f64 - 1.0))
}

fn covariance_sample(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let sum = left
        .iter()
        .zip(right.iter())
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .sum::<f64>();
    Some(sum / (left.len() as f64 - 1.0))
}

fn downside_deviation(values: &[f64], mar_daily: f64) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let sum_squared = values
        .iter()
        .map(|value| (value - mar_daily).min(0.0).powi(2))
        .sum::<f64>();
    Some((sum_squared / (values.len() as f64 - 1.0)).sqrt())
}

fn format_optional_date(value: Option<NaiveDate>) -> String {
    value.map_or_else(String::new, |date| date.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PerformanceMetricConfig {
        PerformanceMetricConfig {
            portfolio_run_id: "run-1".to_string(),
            result_attempt_id: "attempt-1".to_string(),
            security_code: "000300.SH".to_string(),
            window_key: "full_period".to_string(),
            window_start: None,
            window_end: None,
            annualization_days: 252,
            min_observations: 3,
            portfolio_return_basis: "price_return".to_string(),
            benchmark_return_basis: "price_index".to_string(),
            risk_free_tenor: "1y".to_string(),
            risk_free_daily_method: "compound".to_string(),
            risk_free_fill_strategy: "forward_fill".to_string(),
            benchmark_fill_strategy: "skip".to_string(),
            mar: 0.0,
            mar_basis: "fixed".to_string(),
            alignment_strategy: "inner_join_trade_dates".to_string(),
            first_day_return_handling: "exclude".to_string(),
            zero_division_policy: "null".to_string(),
            config_version: 1,
            config_hash: "hash".to_string(),
        }
    }

    fn nav_row(day: u32, daily_return: Option<f64>, drawdown: f64) -> PortfolioNavRow {
        PortfolioNavRow {
            trade_date: NaiveDate::from_ymd_opt(2024, 1, day).expect("valid date"),
            cash_balance: 0.0,
            position_market_value: 0.0,
            total_equity: 100.0,
            nav: 1.0,
            daily_return,
            drawdown,
            gross_exposure: 1.0,
            position_count: 1,
            turnover: 0.0,
            fee_amount: 0.0,
            warning_count: 0,
        }
    }

    fn benchmark(day: u32, return_daily: f64) -> BenchmarkReturn {
        BenchmarkReturn {
            trade_date: NaiveDate::from_ymd_opt(2024, 1, day).expect("valid date"),
            return_daily: Some(return_daily),
        }
    }

    fn risk_free(day: u32, daily_rate: f64) -> RiskFreeRate {
        RiskFreeRate {
            trade_date: NaiveDate::from_ymd_opt(2024, 1, day).expect("valid date"),
            daily_rate: Some(daily_rate),
        }
    }

    #[test]
    fn performance_metric_config_hash_should_be_stable_for_canonical_fields() {
        let config = PerformanceMetricConfig::default_full_period("run-1", "attempt-1");
        let same_config = PerformanceMetricConfig::default_full_period("run-1", "attempt-1");
        let different_attempt = PerformanceMetricConfig::default_full_period("run-1", "attempt-2");

        assert_eq!(
            config.config_hash,
            "74e2930948c49b02157587878a66320d0e10a2fdd17d011c7d39e4c71e920c0a"
        );
        assert_eq!(config.config_hash, same_config.config_hash);
        assert_ne!(config.config_hash, different_attempt.config_hash);
    }

    #[test]
    fn compute_performance_metric_should_use_compounded_period_return() {
        let nav = vec![
            nav_row(1, None, 0.0),
            nav_row(2, Some(0.01), 0.0),
            nav_row(3, Some(-0.02), -0.02),
            nav_row(4, Some(0.03), 0.0),
        ];
        let benchmark = vec![benchmark(2, 0.005), benchmark(3, -0.01), benchmark(4, 0.02)];
        let risk_free = vec![risk_free(2, 0.0), risk_free(3, 0.0), risk_free(4, 0.0)];

        let (row, statuses) = compute_performance_metric(&config(), &nav, &benchmark, &risk_free);

        let expected = (1.01 * 0.98 * 1.03) - 1.0;
        assert_eq!(row.metric_status, "succeeded");
        assert!((row.holding_period_return.expect("return") - expected).abs() < 1e-12);
        assert_eq!(statuses.len(), CORE_PERFORMANCE_METRICS.len());
    }

    #[test]
    fn compute_performance_metric_should_store_positive_max_drawdown() {
        let nav = vec![
            nav_row(2, Some(0.01), 0.0),
            nav_row(3, Some(-0.02), -0.12),
            nav_row(4, Some(0.03), -0.04),
        ];
        let benchmark = vec![benchmark(2, 0.005), benchmark(3, -0.01), benchmark(4, 0.02)];
        let risk_free = vec![risk_free(2, 0.0), risk_free(3, 0.0), risk_free(4, 0.0)];

        let (row, _) = compute_performance_metric(&config(), &nav, &benchmark, &risk_free);

        assert_eq!(row.max_drawdown, Some(0.12));
    }

    #[test]
    fn compute_performance_metric_should_emit_metric_status_for_zero_beta() {
        let nav = vec![
            nav_row(2, Some(0.01), 0.0),
            nav_row(3, Some(-0.02), -0.02),
            nav_row(4, Some(0.03), 0.0),
        ];
        let benchmark = vec![benchmark(2, 0.01), benchmark(3, 0.01), benchmark(4, 0.01)];
        let risk_free = vec![risk_free(2, 0.0), risk_free(3, 0.0), risk_free(4, 0.0)];

        let (row, statuses) = compute_performance_metric(&config(), &nav, &benchmark, &risk_free);

        let beta_status = statuses
            .iter()
            .find(|status| status.metric_name == "beta")
            .expect("beta status");
        assert_eq!(row.beta, None);
        assert_eq!(beta_status.metric_status, "zero_division");
        assert_eq!(beta_status.reason_code, "benchmark_variance_zero");
    }

    #[test]
    fn compute_performance_metric_should_fail_when_observations_are_insufficient() {
        let cfg = PerformanceMetricConfig {
            min_observations: 4,
            ..config()
        };
        let nav = vec![
            nav_row(2, Some(0.01), 0.0),
            nav_row(3, Some(-0.02), -0.02),
            nav_row(4, Some(0.03), 0.0),
        ];
        let benchmark = vec![benchmark(2, 0.005), benchmark(3, -0.01), benchmark(4, 0.02)];
        let risk_free = vec![risk_free(2, 0.0), risk_free(3, 0.0), risk_free(4, 0.0)];

        let (row, statuses) = compute_performance_metric(&cfg, &nav, &benchmark, &risk_free);

        assert_eq!(row.metric_status, "insufficient_observations");
        assert!(statuses.iter().all(|status| {
            status.metric_status == "insufficient_observations"
                && status.reason_code == "n_below_min_observations"
        }));
    }
}
