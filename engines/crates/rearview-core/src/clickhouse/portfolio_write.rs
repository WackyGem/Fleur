//! Write-side support for `fleur_portfolio` result fact tables.
//!
//! Each struct mirrors a ClickHouse table row and derives `Serialize` so it
//! can be emitted as JSONEachRow via the HTTP interface.

use chrono::NaiveDate;
use serde::Serialize;
use uuid::Uuid;

use crate::portfolio::{
    PortfolioSimulationOutput, order_reason_str, order_side_str, order_status_str,
    portfolio_event_type_str, target_reason_str,
};
use crate::postgres::PortfolioRunRecord;

#[derive(Debug, Serialize)]
pub struct RunSnapshotRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub source_run_id: String,
    pub rule_version_id: String,
    pub rule_hash: String,
    pub account_snapshot: String,
    pub execution_snapshot: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct NavDailyRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub trade_date: NaiveDate,
    pub cash_balance: f64,
    pub position_market_value: f64,
    pub total_equity: f64,
    pub nav: f64,
    pub daily_return: Option<f64>,
    pub drawdown: f64,
    pub gross_exposure: f64,
    pub position_count: u32,
    pub turnover: f64,
    pub fee_amount: f64,
    pub warning_count: u32,
}

#[derive(Debug, Serialize)]
pub struct PositionDayRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub trade_date: NaiveDate,
    pub security_code: String,
    pub quantity: f64,
    pub cost_basis: f64,
    pub average_entry_price: f64,
    pub close_price: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub unrealized_return: f64,
    pub holding_days: u32,
    pub is_stale_price: bool,
}

#[derive(Debug, Serialize)]
pub struct TradeRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub portfolio_trade_id: String,
    pub portfolio_order_id: Option<String>,
    pub trade_seq: u32,
    pub order_seq: u32,
    pub trade_date: NaiveDate,
    pub signal_date: Option<NaiveDate>,
    pub security_code: String,
    pub side: String,
    pub quantity: f64,
    pub reference_price: f64,
    pub execution_price: f64,
    pub gross_amount: f64,
    pub commission: f64,
    pub stamp_duty: f64,
    pub transfer_fee: f64,
    pub total_fee: f64,
    pub slippage_cost: f64,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct OrderRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub portfolio_order_id: String,
    pub order_seq: u32,
    pub signal_date: Option<NaiveDate>,
    pub execution_date: NaiveDate,
    pub security_code: String,
    pub side: String,
    pub order_quantity: f64,
    pub order_amount: f64,
    pub reference_price: Option<f64>,
    pub reason: String,
    pub status: String,
    pub event_ref: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TargetRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub signal_date: NaiveDate,
    pub execution_date: NaiveDate,
    pub security_code: String,
    pub source_rank: u32,
    pub source_score: f64,
    pub target_weight: f64,
    pub target_amount: f64,
    pub target_quantity: f64,
    pub target_reason: String,
}

#[derive(Debug, Serialize)]
pub struct EventRow {
    pub portfolio_run_id: String,
    pub result_attempt_id: String,
    pub portfolio_event_id: String,
    pub event_seq: u32,
    pub trade_date: NaiveDate,
    pub security_code: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub payload: String,
}

/// Context assembled while converting `PortfolioSimulationOutput` to ClickHouse
/// write rows. Carries generated IDs so trades can reference their orders.
pub struct WriteBatch {
    pub run_snapshot: RunSnapshotRow,
    pub targets: Vec<TargetRow>,
    pub orders: Vec<OrderRow>,
    pub trades: Vec<TradeRow>,
    pub positions: Vec<PositionDayRow>,
    pub nav: Vec<NavDailyRow>,
    pub events: Vec<EventRow>,
}

impl WriteBatch {
    /// Build a `WriteBatch` from the run record and simulation output.
    pub fn from_output(
        run: &PortfolioRunRecord,
        result_attempt_id: &str,
        output: &PortfolioSimulationOutput,
    ) -> Self {
        let portfolio_run_id = run.portfolio_run_id.clone();
        let result_attempt_id = result_attempt_id.to_string();

        let targets = output
            .targets
            .iter()
            .map(|t| TargetRow {
                portfolio_run_id: portfolio_run_id.clone(),
                result_attempt_id: result_attempt_id.clone(),
                signal_date: t.signal_date,
                execution_date: t.execution_date,
                security_code: t.security_code.clone(),
                source_rank: t.source_rank,
                source_score: t.source_score,
                target_weight: t.target_weight,
                target_amount: t.target_amount,
                target_quantity: t.target_quantity,
                target_reason: target_reason_str(t.target_reason).to_string(),
            })
            .collect();

        let mut order_id_map = std::collections::BTreeMap::new();
        let orders = output
            .orders
            .iter()
            .map(|o| {
                let id = Uuid::new_v4().to_string();
                order_id_map.insert(o.order_seq, id.clone());
                OrderRow {
                    portfolio_run_id: portfolio_run_id.clone(),
                    result_attempt_id: result_attempt_id.clone(),
                    portfolio_order_id: id,
                    order_seq: o.order_seq,
                    signal_date: o.signal_date,
                    execution_date: o.execution_date,
                    security_code: o.security_code.clone(),
                    side: order_side_str(o.side).to_string(),
                    order_quantity: o.order_quantity,
                    order_amount: o.order_amount,
                    reference_price: o.reference_price,
                    reason: order_reason_str(o.reason).to_string(),
                    status: order_status_str(o.status).to_string(),
                    event_ref: None,
                }
            })
            .collect();

        let trades = output
            .trades
            .iter()
            .map(|t| TradeRow {
                portfolio_run_id: portfolio_run_id.clone(),
                result_attempt_id: result_attempt_id.clone(),
                portfolio_trade_id: Uuid::new_v4().to_string(),
                portfolio_order_id: order_id_map.get(&t.order_seq).cloned(),
                trade_seq: t.trade_seq,
                order_seq: t.order_seq,
                trade_date: t.trade_date,
                signal_date: t.signal_date,
                security_code: t.security_code.clone(),
                side: order_side_str(t.side).to_string(),
                quantity: t.quantity,
                reference_price: t.reference_price,
                execution_price: t.execution_price,
                gross_amount: t.gross_amount,
                commission: t.commission,
                stamp_duty: t.stamp_duty,
                transfer_fee: t.transfer_fee,
                total_fee: t.total_fee,
                slippage_cost: t.slippage_cost,
                reason: order_reason_str(t.reason).to_string(),
            })
            .collect();

        let positions = output
            .positions
            .iter()
            .map(|p| PositionDayRow {
                portfolio_run_id: portfolio_run_id.clone(),
                result_attempt_id: result_attempt_id.clone(),
                trade_date: p.trade_date,
                security_code: p.security_code.clone(),
                quantity: p.quantity,
                cost_basis: p.cost_basis,
                average_entry_price: p.average_entry_price,
                close_price: p.close_price,
                market_value: p.market_value,
                unrealized_pnl: p.unrealized_pnl,
                unrealized_return: p.unrealized_return,
                holding_days: p.holding_days,
                is_stale_price: p.is_stale_price,
            })
            .collect();

        let nav = output
            .nav
            .iter()
            .map(|n| NavDailyRow {
                portfolio_run_id: portfolio_run_id.clone(),
                result_attempt_id: result_attempt_id.clone(),
                trade_date: n.trade_date,
                cash_balance: n.cash_balance,
                position_market_value: n.position_market_value,
                total_equity: n.total_equity,
                nav: n.nav,
                daily_return: n.daily_return,
                drawdown: n.drawdown,
                gross_exposure: n.gross_exposure,
                position_count: n.position_count as u32,
                turnover: n.turnover,
                fee_amount: n.fee_amount,
                warning_count: n.warning_count as u32,
            })
            .collect();

        let events = output
            .events
            .iter()
            .map(|e| EventRow {
                portfolio_run_id: portfolio_run_id.clone(),
                result_attempt_id: result_attempt_id.clone(),
                portfolio_event_id: Uuid::new_v4().to_string(),
                event_seq: e.event_seq,
                trade_date: e.trade_date,
                security_code: e.security_code.clone(),
                event_type: portfolio_event_type_str(e.event_type).to_string(),
                severity: "warning".to_string(),
                message: e.message.clone(),
                payload: "{}".to_string(),
            })
            .collect();

        let summary = serde_json::to_string(&output.summary).unwrap_or_else(|_| "{}".to_string());
        let run_snapshot = RunSnapshotRow {
            portfolio_run_id,
            result_attempt_id,
            source_run_id: run.source_run_id.clone(),
            rule_version_id: run.rule_version_id.clone(),
            rule_hash: run.rule_hash.clone(),
            account_snapshot: run.account_snapshot.to_string(),
            execution_snapshot: run.execution_snapshot.to_string(),
            start_date: run.start_date,
            end_date: run.end_date,
            summary,
        };

        Self {
            run_snapshot,
            targets,
            orders,
            trades,
            positions,
            nav,
            events,
        }
    }
}

/// Serialize a slice of `Serialize` items into JSONEachRow (one JSON object per line).
pub fn to_json_each_row<T: Serialize>(rows: &[T]) -> RearviewResult<String> {
    let mut body = String::new();
    for row in rows {
        let line = serde_json::to_string(row).map_err(RearviewError::Json)?;
        body.push_str(&line);
        body.push('\n');
    }
    Ok(body)
}

/// Serialize rows after replacing the legacy `portfolio_run_id` JSON field
/// with the concrete family identity field used by split result tables.
pub fn to_json_each_row_with_run_id_field<T: Serialize>(
    rows: &[T],
    run_id_field: &str,
) -> RearviewResult<String> {
    let mut body = String::new();
    for row in rows {
        let mut value = serde_json::to_value(row).map_err(RearviewError::Json)?;
        let object = value.as_object_mut().ok_or_else(|| {
            RearviewError::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ClickHouse row must serialize to a JSON object",
            )))
        })?;
        let Some(run_id) = object.remove("portfolio_run_id") else {
            return Err(RearviewError::Json(serde_json::Error::io(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "ClickHouse row is missing portfolio_run_id",
                ),
            )));
        };
        object.insert(run_id_field.to_string(), run_id);
        let line = serde_json::to_string(&value).map_err(RearviewError::Json)?;
        body.push_str(&line);
        body.push('\n');
    }
    Ok(body)
}

// Re-export for the macro-like usage in to_json_each_row
use crate::error::{RearviewError, RearviewResult};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::*;
    use serde_json::Value;

    fn sample_run() -> PortfolioRunRecord {
        PortfolioRunRecord {
            portfolio_run_id: "run-1".to_string(),
            source_run_id: "src-1".to_string(),
            rule_version_id: "rv-1".to_string(),
            rule_hash: "hash-1".to_string(),
            account_template_id: None,
            account_snapshot: serde_json::json!({"initial_cash": 1000000.0}),
            execution_snapshot: serde_json::json!({}),
            price_basis: "backward_adjusted".to_string(),
            start_date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(),
            status: "calculating_nav".to_string(),
            dispatch_status: "published".to_string(),
            nats_stream_sequence: None,
            summary: serde_json::json!({}),
            error_type: None,
            error_message: None,
            current_result_attempt_id: None,
        }
    }

    fn sample_output() -> PortfolioSimulationOutput {
        PortfolioSimulationOutput {
            targets: vec![],
            orders: vec![],
            trades: vec![],
            positions: vec![],
            nav: vec![PortfolioNavRow {
                trade_date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                cash_balance: 1_000_000.0,
                position_market_value: 0.0,
                total_equity: 1_000_000.0,
                nav: 1.0,
                daily_return: None,
                drawdown: 0.0,
                gross_exposure: 0.0,
                position_count: 0,
                turnover: 0.0,
                fee_amount: 0.0,
                warning_count: 0,
            }],
            events: vec![],
            summary: PortfolioSummary {
                initial_cash: 1_000_000.0,
                ending_equity: 1_000_000.0,
                total_return: 0.0,
                max_drawdown: 0.0,
                trade_count: 0,
                total_fee: 0.0,
                warning_count: 0,
            },
        }
    }

    #[test]
    fn from_output_includes_attempt_id_on_every_row() {
        let run = sample_run();
        let output = sample_output();
        let batch = WriteBatch::from_output(&run, "attempt-xyz", &output);

        assert_eq!(batch.run_snapshot.result_attempt_id, "attempt-xyz");
        assert_eq!(batch.nav.len(), 1);
        assert_eq!(batch.nav[0].result_attempt_id, "attempt-xyz");
        assert_eq!(batch.nav[0].portfolio_run_id, "run-1");
    }

    #[test]
    fn to_json_each_row_produces_one_line_per_row() {
        let run = sample_run();
        let output = sample_output();
        let batch = WriteBatch::from_output(&run, "a1", &output);
        let body = to_json_each_row(&batch.nav).unwrap();
        let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1);
        let parsed: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed["result_attempt_id"], "a1");
        assert_eq!(parsed["daily_return"], Value::Null);
    }

    #[test]
    fn to_json_each_row_with_run_id_field_rewrites_identity_column() {
        let run = sample_run();
        let output = sample_output();
        let batch = WriteBatch::from_output(&run, "a1", &output);
        let body =
            to_json_each_row_with_run_id_field(&batch.nav, "strategy_backtest_run_id").unwrap();
        let parsed: Value = serde_json::from_str(body.lines().next().unwrap()).unwrap();
        assert_eq!(parsed["strategy_backtest_run_id"], "run-1");
        assert!(parsed.get("portfolio_run_id").is_none());
    }
}
