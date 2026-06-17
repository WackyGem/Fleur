//! Write-side support for portfolio calculation output tables.

use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PerformanceMetricRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub security_code: String,
    pub window_key: String,
    pub window_start: Option<NaiveDate>,
    pub window_end: Option<NaiveDate>,
    pub config_hash: String,
    pub metric_status: String,
    pub observation_count: u32,
    pub holding_period_return: Option<f64>,
    pub annualized_return: Option<f64>,
    pub annualized_volatility: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub calmar_ratio: Option<f64>,
    pub downside_deviation: Option<f64>,
    pub sortino_ratio: Option<f64>,
    pub sharpe_ratio: Option<f64>,
    pub information_ratio: Option<f64>,
    pub beta: Option<f64>,
    pub alpha: Option<f64>,
    pub treynor_ratio: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PerformanceMetricStatusRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub security_code: String,
    pub window_key: String,
    pub metric_name: String,
    pub metric_status: String,
    pub reason_code: String,
}

#[derive(Debug, Serialize)]
pub struct ClosedTradeRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub closed_trade_id: String,
    pub closed_trade_seq: u32,
    pub position_lot_id: String,
    pub entry_trade_seq: u32,
    pub exit_trade_seq: u32,
    pub security_code: String,
    pub entry_date: NaiveDate,
    pub exit_date: NaiveDate,
    pub quantity: f64,
    pub entry_gross_amount: f64,
    pub exit_gross_amount: f64,
    pub entry_fee: f64,
    pub exit_fee: f64,
    pub realized_pnl: f64,
    pub holding_days: u32,
    pub exit_reason: String,
}

#[derive(Debug, Serialize)]
pub struct TradeMetricRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub window_key: String,
    pub window_start: Option<NaiveDate>,
    pub window_end: Option<NaiveDate>,
    pub closed_trade_count: u32,
    pub winning_trade_count: u32,
    pub losing_trade_count: u32,
    pub breakeven_trade_count: u32,
    pub win_rate_closed_trades: Option<f64>,
    pub average_win_return: Option<f64>,
    pub average_loss_return: Option<f64>,
    pub profit_loss_ratio: Option<f64>,
    pub average_holding_days: Option<f64>,
    pub largest_win_return: Option<f64>,
    pub largest_loss_return: Option<f64>,
}

#[derive(Debug, Default)]
pub struct CalculationWriteBatch {
    pub performance_metrics: Vec<PerformanceMetricRow>,
    pub performance_metric_statuses: Vec<PerformanceMetricStatusRow>,
    pub closed_trades: Vec<ClosedTradeRow>,
    pub trade_metrics: Vec<TradeMetricRow>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clickhouse::portfolio_write::to_json_each_row;

    #[test]
    fn performance_metric_row_serializes_nullable_metrics() {
        let rows = vec![PerformanceMetricRow {
            portfolio_run_id: "run-1".to_string(),
            result_attempt_id: "attempt-1".to_string(),
            security_code: "000300.SH".to_string(),
            window_key: "full_period".to_string(),
            window_start: None,
            window_end: None,
            config_hash: "hash".to_string(),
            metric_status: "succeeded".to_string(),
            observation_count: 252,
            holding_period_return: Some(0.12),
            annualized_return: Some(0.12),
            annualized_volatility: Some(0.2),
            max_drawdown: Some(0.08),
            calmar_ratio: Some(1.5),
            downside_deviation: Some(0.1),
            sortino_ratio: Some(1.2),
            sharpe_ratio: Some(0.5),
            information_ratio: None,
            beta: Some(0.9),
            alpha: Some(0.03),
            treynor_ratio: Some(0.1),
        }];

        let body = to_json_each_row(&rows).expect("serialize metric rows");

        assert!(body.contains(r#""information_ratio":null"#));
    }

    #[test]
    fn closed_trade_row_uses_gross_amount_fields() {
        let rows = vec![ClosedTradeRow {
            portfolio_run_id: "run-1".to_string(),
            result_attempt_id: "attempt-1".to_string(),
            closed_trade_id: "closed-1".to_string(),
            closed_trade_seq: 1,
            position_lot_id: "lot-1".to_string(),
            entry_trade_seq: 1,
            exit_trade_seq: 2,
            security_code: "000001.SZ".to_string(),
            entry_date: NaiveDate::from_ymd_opt(2024, 1, 2).expect("valid date"),
            exit_date: NaiveDate::from_ymd_opt(2024, 1, 5).expect("valid date"),
            quantity: 100.0,
            entry_gross_amount: 1000.0,
            exit_gross_amount: 1100.0,
            entry_fee: 1.0,
            exit_fee: 1.0,
            realized_pnl: 98.0,
            holding_days: 3,
            exit_reason: "rebalance".to_string(),
        }];

        let body = to_json_each_row(&rows).expect("serialize closed trade rows");

        assert!(body.contains("entry_gross_amount"));
        assert!(body.contains("exit_gross_amount"));
        assert!(!body.contains("entry_amount"));
    }
}
