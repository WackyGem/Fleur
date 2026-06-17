use std::collections::{BTreeMap, VecDeque};

use chrono::NaiveDate;

use crate::clickhouse::calculation_write::{ClosedTradeRow, TradeMetricRow};
use crate::portfolio::{OrderSide, PortfolioSimulationOutput, PortfolioTradeRow, order_reason_str};

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone)]
struct OpenLot {
    position_lot_id: String,
    entry_trade_seq: u32,
    entry_date: NaiveDate,
    remaining_quantity: f64,
    entry_gross_remaining: f64,
    entry_fee_remaining: f64,
}

pub fn compute_trade_calculation_outputs(
    portfolio_run_id: &str,
    result_attempt_id: &str,
    output: &PortfolioSimulationOutput,
) -> (Vec<ClosedTradeRow>, Vec<TradeMetricRow>) {
    let trade_day_index = output
        .nav
        .iter()
        .enumerate()
        .map(|(index, row)| (row.trade_date, index))
        .collect::<BTreeMap<_, _>>();
    let closed_trades = compute_closed_trades(
        portfolio_run_id,
        result_attempt_id,
        output,
        &trade_day_index,
    );
    let trade_metrics = vec![compute_trade_metric(
        portfolio_run_id,
        result_attempt_id,
        &closed_trades,
    )];
    (closed_trades, trade_metrics)
}

fn compute_closed_trades(
    portfolio_run_id: &str,
    result_attempt_id: &str,
    output: &PortfolioSimulationOutput,
    trade_day_index: &BTreeMap<NaiveDate, usize>,
) -> Vec<ClosedTradeRow> {
    let mut open_lots: BTreeMap<String, VecDeque<OpenLot>> = BTreeMap::new();
    let mut closed_rows = Vec::new();
    let mut closed_trade_seq = 0_u32;

    for trade in &output.trades {
        match trade.side {
            OrderSide::Buy => open_lots
                .entry(trade.security_code.clone())
                .or_default()
                .push_back(OpenLot {
                    position_lot_id: format!("{result_attempt_id}-lot-{}", trade.trade_seq),
                    entry_trade_seq: trade.trade_seq,
                    entry_date: trade.trade_date,
                    remaining_quantity: trade.quantity,
                    entry_gross_remaining: trade.gross_amount,
                    entry_fee_remaining: trade.total_fee,
                }),
            OrderSide::Sell => consume_sell_trade(
                portfolio_run_id,
                result_attempt_id,
                trade,
                trade_day_index,
                &mut open_lots,
                &mut closed_rows,
                &mut closed_trade_seq,
            ),
        }
    }

    closed_rows
}

fn consume_sell_trade(
    portfolio_run_id: &str,
    result_attempt_id: &str,
    sell_trade: &PortfolioTradeRow,
    trade_day_index: &BTreeMap<NaiveDate, usize>,
    open_lots: &mut BTreeMap<String, VecDeque<OpenLot>>,
    closed_rows: &mut Vec<ClosedTradeRow>,
    closed_trade_seq: &mut u32,
) {
    let Some(lots) = open_lots.get_mut(&sell_trade.security_code) else {
        return;
    };
    let mut sell_quantity_remaining = sell_trade.quantity;
    let mut exit_gross_remaining = sell_trade.gross_amount;
    let mut exit_fee_remaining = sell_trade.total_fee;

    while sell_quantity_remaining > EPSILON {
        let Some(mut lot) = lots.pop_front() else {
            break;
        };
        let quantity = lot.remaining_quantity.min(sell_quantity_remaining);
        let entry_fraction = quantity / lot.remaining_quantity;
        let exit_fraction = quantity / sell_quantity_remaining;
        let entry_gross_amount = lot.entry_gross_remaining * entry_fraction;
        let entry_fee = lot.entry_fee_remaining * entry_fraction;
        let exit_gross_amount = exit_gross_remaining * exit_fraction;
        let exit_fee = exit_fee_remaining * exit_fraction;
        let realized_pnl = exit_gross_amount - entry_gross_amount - entry_fee - exit_fee;

        *closed_trade_seq += 1;
        closed_rows.push(ClosedTradeRow {
            portfolio_run_id: portfolio_run_id.to_string(),
            result_attempt_id: result_attempt_id.to_string(),
            closed_trade_id: format!("{result_attempt_id}-closed-{closed_trade_seq}"),
            closed_trade_seq: *closed_trade_seq,
            position_lot_id: lot.position_lot_id.clone(),
            entry_trade_seq: lot.entry_trade_seq,
            exit_trade_seq: sell_trade.trade_seq,
            security_code: sell_trade.security_code.clone(),
            entry_date: lot.entry_date,
            exit_date: sell_trade.trade_date,
            quantity,
            entry_gross_amount,
            exit_gross_amount,
            entry_fee,
            exit_fee,
            realized_pnl,
            holding_days: holding_days(trade_day_index, lot.entry_date, sell_trade.trade_date),
            exit_reason: order_reason_str(sell_trade.reason).to_string(),
        });

        lot.remaining_quantity -= quantity;
        lot.entry_gross_remaining -= entry_gross_amount;
        lot.entry_fee_remaining -= entry_fee;
        sell_quantity_remaining -= quantity;
        exit_gross_remaining -= exit_gross_amount;
        exit_fee_remaining -= exit_fee;

        if lot.remaining_quantity > EPSILON {
            lots.push_front(lot);
            break;
        }
    }
}

fn compute_trade_metric(
    portfolio_run_id: &str,
    result_attempt_id: &str,
    closed_trades: &[ClosedTradeRow],
) -> TradeMetricRow {
    let returns = closed_trades
        .iter()
        .filter_map(realized_return)
        .collect::<Vec<_>>();
    let wins = returns
        .iter()
        .copied()
        .filter(|value| *value > EPSILON)
        .collect::<Vec<_>>();
    let losses = returns
        .iter()
        .copied()
        .filter(|value| *value < -EPSILON)
        .collect::<Vec<_>>();
    let breakeven_trade_count = closed_trades.len() - wins.len() - losses.len();
    let average_win_return = average(&wins);
    let average_loss_return = average(&losses);
    let profit_loss_ratio = match (average_win_return, average_loss_return) {
        (Some(win), Some(loss)) if loss != 0.0 => Some(win.abs() / loss.abs()),
        _ => None,
    };

    TradeMetricRow {
        portfolio_run_id: portfolio_run_id.to_string(),
        result_attempt_id: result_attempt_id.to_string(),
        window_key: "full_period".to_string(),
        window_start: None,
        window_end: None,
        closed_trade_count: closed_trades.len() as u32,
        winning_trade_count: wins.len() as u32,
        losing_trade_count: losses.len() as u32,
        breakeven_trade_count: breakeven_trade_count as u32,
        win_rate_closed_trades: if closed_trades.is_empty() {
            None
        } else {
            Some(wins.len() as f64 / closed_trades.len() as f64)
        },
        average_win_return,
        average_loss_return,
        profit_loss_ratio,
        average_holding_days: if closed_trades.is_empty() {
            None
        } else {
            Some(
                closed_trades
                    .iter()
                    .map(|row| f64::from(row.holding_days))
                    .sum::<f64>()
                    / closed_trades.len() as f64,
            )
        },
        largest_win_return: wins.iter().copied().reduce(f64::max),
        largest_loss_return: losses.iter().copied().reduce(f64::min),
    }
}

fn realized_return(row: &ClosedTradeRow) -> Option<f64> {
    let denominator = row.entry_gross_amount + row.entry_fee;
    if denominator == 0.0 {
        None
    } else {
        Some(row.realized_pnl / denominator)
    }
}

fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn holding_days(
    trade_day_index: &BTreeMap<NaiveDate, usize>,
    entry_date: NaiveDate,
    exit_date: NaiveDate,
) -> u32 {
    match (
        trade_day_index.get(&entry_date).copied(),
        trade_day_index.get(&exit_date).copied(),
    ) {
        (Some(entry), Some(exit)) => exit.saturating_sub(entry) as u32,
        _ => exit_date
            .signed_duration_since(entry_date)
            .num_days()
            .max(0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::portfolio::{
        OrderReason, PortfolioEventRow, PortfolioNavRow, PortfolioPositionDayRow, PortfolioSummary,
        PortfolioTargetRow,
    };

    #[test]
    fn compute_trade_calculation_outputs_should_fifo_match_multiple_lots() {
        let output = output_with_trades(vec![
            trade(1, "2024-01-02", OrderSide::Buy, 100.0, 1_000.0, 10.0),
            trade(2, "2024-01-03", OrderSide::Buy, 100.0, 2_000.0, 20.0),
            trade(3, "2024-01-05", OrderSide::Sell, 150.0, 2_400.0, 24.0),
        ]);

        let (closed, _) = compute_trade_calculation_outputs("run-1", "attempt-1", &output);

        assert_eq!(closed.len(), 2);
        assert_eq!(closed[0].entry_trade_seq, 1);
        assert_eq!(closed[0].quantity, 100.0);
        assert_eq!(closed[1].entry_trade_seq, 2);
        assert_eq!(closed[1].quantity, 50.0);
        assert_eq!(closed[1].entry_gross_amount, 1_000.0);
        assert_eq!(closed[1].entry_fee, 10.0);
    }

    #[test]
    fn compute_trade_calculation_outputs_should_aggregate_trade_metrics() {
        let output = output_with_trades(vec![
            trade(1, "2024-01-02", OrderSide::Buy, 100.0, 1_000.0, 0.0),
            trade(2, "2024-01-03", OrderSide::Sell, 100.0, 1_100.0, 0.0),
            trade(3, "2024-01-04", OrderSide::Buy, 100.0, 1_000.0, 0.0),
            trade(4, "2024-01-05", OrderSide::Sell, 100.0, 900.0, 0.0),
        ]);

        let (_, metrics) = compute_trade_calculation_outputs("run-1", "attempt-1", &output);

        let metric = metrics.first().expect("trade metric row");
        assert_eq!(metric.closed_trade_count, 2);
        assert_eq!(metric.winning_trade_count, 1);
        assert_eq!(metric.losing_trade_count, 1);
        assert_eq!(metric.win_rate_closed_trades, Some(0.5));
        assert_eq!(metric.profit_loss_ratio, Some(1.0));
    }

    fn output_with_trades(trades: Vec<PortfolioTradeRow>) -> PortfolioSimulationOutput {
        let nav = ["2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05"]
            .into_iter()
            .map(|date| PortfolioNavRow {
                trade_date: parse_date(date),
                cash_balance: 0.0,
                position_market_value: 0.0,
                total_equity: 1.0,
                nav: 1.0,
                daily_return: Some(0.0),
                drawdown: 0.0,
                gross_exposure: 0.0,
                position_count: 0,
                turnover: 0.0,
                fee_amount: 0.0,
                warning_count: 0,
            })
            .collect();

        PortfolioSimulationOutput {
            targets: Vec::<PortfolioTargetRow>::new(),
            orders: Vec::new(),
            trades,
            positions: Vec::<PortfolioPositionDayRow>::new(),
            nav,
            events: Vec::<PortfolioEventRow>::new(),
            summary: PortfolioSummary {
                initial_cash: 1.0,
                ending_equity: 1.0,
                total_return: 0.0,
                max_drawdown: 0.0,
                trade_count: 0,
                total_fee: 0.0,
                warning_count: 0,
            },
        }
    }

    fn trade(
        trade_seq: u32,
        trade_date: &str,
        side: OrderSide,
        quantity: f64,
        gross_amount: f64,
        total_fee: f64,
    ) -> PortfolioTradeRow {
        PortfolioTradeRow {
            trade_seq,
            order_seq: trade_seq,
            trade_date: parse_date(trade_date),
            signal_date: None,
            security_code: "000001.SZ".to_string(),
            side,
            quantity,
            reference_price: gross_amount / quantity,
            execution_price: gross_amount / quantity,
            gross_amount,
            commission: total_fee,
            stamp_duty: 0.0,
            transfer_fee: 0.0,
            total_fee,
            slippage_cost: 0.0,
            reason: OrderReason::Rebalance,
        }
    }

    fn parse_date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("valid date")
    }
}
