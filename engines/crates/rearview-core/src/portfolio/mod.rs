use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSimulationInput {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub initial_cash: f64,
    pub max_positions: usize,
    pub single_position_limit_pct: Option<f64>,
    pub cash_reserve_pct: f64,
    pub lot_size: u32,
    pub min_trade_lots: u32,
    pub fee_profile: FeeProfile,
    pub slippage_profile: SlippageProfile,
    pub exit_rules: Vec<ExitRule>,
    pub signals: Vec<BuySignalInput>,
    pub prices: Vec<PriceBar>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeeProfile {
    pub commission_rate: f64,
    pub commission_rate_max: f64,
    pub min_commission: f64,
    pub stamp_duty_rate_sell: f64,
    pub transfer_fee_rate: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SlippageProfile {
    pub buy_bps: f64,
    pub sell_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExitRule {
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
        metric: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuySignalInput {
    pub signal_date: NaiveDate,
    pub execution_date: NaiveDate,
    pub security_code: String,
    pub rank: u32,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceBar {
    pub security_code: String,
    pub trade_date: NaiveDate,
    pub open_price_backward_adj: Option<f64>,
    pub close_price_backward_adj: Option<f64>,
    #[serde(default)]
    pub close_price_forward_adj: Option<f64>,
    #[serde(default)]
    pub price_ma_3: Option<f64>,
    #[serde(default)]
    pub price_ma_5: Option<f64>,
    #[serde(default)]
    pub price_ma_6: Option<f64>,
    #[serde(default)]
    pub price_ma_10: Option<f64>,
    #[serde(default)]
    pub price_ma_12: Option<f64>,
    #[serde(default)]
    pub price_ma_14: Option<f64>,
    #[serde(default)]
    pub price_ma_20: Option<f64>,
    #[serde(default)]
    pub price_ma_24: Option<f64>,
    #[serde(default)]
    pub price_ma_28: Option<f64>,
    #[serde(default)]
    pub price_ma_30: Option<f64>,
    #[serde(default)]
    pub price_ma_57: Option<f64>,
    #[serde(default)]
    pub price_ma_60: Option<f64>,
    #[serde(default)]
    pub price_ma_114: Option<f64>,
    #[serde(default)]
    pub price_ma_250: Option<f64>,
    #[serde(default)]
    pub price_avg_ma_3_6_12_24: Option<f64>,
    #[serde(default)]
    pub price_avg_ma_14_28_57_114: Option<f64>,
    #[serde(default)]
    pub price_ema2_10: Option<f64>,
    #[serde(default)]
    pub boll_lower_20_2: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSimulationOutput {
    pub targets: Vec<PortfolioTargetRow>,
    pub orders: Vec<PortfolioOrderRow>,
    pub trades: Vec<PortfolioTradeRow>,
    pub positions: Vec<PortfolioPositionDayRow>,
    pub nav: Vec<PortfolioNavRow>,
    pub events: Vec<PortfolioEventRow>,
    pub summary: PortfolioSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioSimulationWithDiagnostics {
    pub output: PortfolioSimulationOutput,
    pub diagnostics: SimulationDiagnostics,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationDiagnostics {
    pub version: u32,
    pub simulation_ms: BTreeMap<String, u128>,
    pub row_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTargetRow {
    pub signal_date: NaiveDate,
    pub execution_date: NaiveDate,
    pub security_code: String,
    pub source_rank: u32,
    pub source_score: f64,
    pub target_weight: f64,
    pub target_amount: f64,
    pub target_quantity: f64,
    pub target_reason: TargetReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetReason {
    BuySignal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioOrderRow {
    pub order_seq: u32,
    pub signal_date: Option<NaiveDate>,
    pub execution_date: NaiveDate,
    pub security_code: String,
    pub side: OrderSide,
    pub order_quantity: f64,
    pub order_amount: f64,
    pub reference_price: Option<f64>,
    pub reason: OrderReason,
    pub status: OrderStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderReason {
    Rebalance,
    FixedStopLoss,
    IndicatorStopLoss,
    TakeProfit,
    TimeStopLoss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Filled,
    SkippedPriceMissing,
    SkippedCashInsufficient,
    SkippedBelowMinLot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTradeRow {
    pub trade_seq: u32,
    pub order_seq: u32,
    pub trade_date: NaiveDate,
    pub signal_date: Option<NaiveDate>,
    pub security_code: String,
    pub side: OrderSide,
    pub quantity: f64,
    pub reference_price: f64,
    pub execution_price: f64,
    pub gross_amount: f64,
    pub commission: f64,
    pub stamp_duty: f64,
    pub transfer_fee: f64,
    pub total_fee: f64,
    pub slippage_cost: f64,
    pub reason: OrderReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPositionDayRow {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioNavRow {
    pub trade_date: NaiveDate,
    pub cash_balance: f64,
    pub position_market_value: f64,
    pub total_equity: f64,
    pub nav: f64,
    pub daily_return: Option<f64>,
    pub drawdown: f64,
    pub gross_exposure: f64,
    pub position_count: usize,
    pub turnover: f64,
    pub fee_amount: f64,
    pub warning_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioEventRow {
    pub event_seq: u32,
    pub trade_date: NaiveDate,
    pub security_code: Option<String>,
    pub event_type: PortfolioEventType,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortfolioEventType {
    PriceMissing,
    IndicatorMissing,
    CashInsufficientForMinLot,
    TargetAmountBelowMinLot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub initial_cash: f64,
    pub ending_equity: f64,
    pub total_return: f64,
    pub max_drawdown: f64,
    pub trade_count: usize,
    pub total_fee: f64,
    pub warning_count: usize,
}

#[derive(Debug, Clone)]
struct PositionState {
    quantity: f64,
    cost_basis: f64,
    average_entry_price: f64,
    entry_trade_index: usize,
}

#[derive(Debug, Clone)]
struct PendingSell {
    signal_date: NaiveDate,
    security_code: String,
    reason: OrderReason,
}

#[derive(Debug, Clone, Copy)]
struct FeeBreakdown {
    commission: f64,
    stamp_duty: f64,
    transfer_fee: f64,
    total: f64,
}

pub fn target_reason_str(reason: TargetReason) -> &'static str {
    match reason {
        TargetReason::BuySignal => "buy_signal",
    }
}

pub fn order_side_str(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    }
}

pub fn order_reason_str(reason: OrderReason) -> &'static str {
    match reason {
        OrderReason::Rebalance => "rebalance",
        OrderReason::FixedStopLoss => "fixed_stop_loss",
        OrderReason::IndicatorStopLoss => "indicator_stop_loss",
        OrderReason::TakeProfit => "take_profit",
        OrderReason::TimeStopLoss => "time_stop_loss",
    }
}

pub fn order_status_str(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Filled => "filled",
        OrderStatus::SkippedPriceMissing => "skipped_price_missing",
        OrderStatus::SkippedCashInsufficient => "skipped_cash_insufficient",
        OrderStatus::SkippedBelowMinLot => "skipped_below_min_lot",
    }
}

pub fn portfolio_event_type_str(event_type: PortfolioEventType) -> &'static str {
    match event_type {
        PortfolioEventType::PriceMissing => "price_missing",
        PortfolioEventType::IndicatorMissing => "indicator_missing",
        PortfolioEventType::CashInsufficientForMinLot => "cash_insufficient_for_min_lot",
        PortfolioEventType::TargetAmountBelowMinLot => "target_amount_below_min_lot",
    }
}

pub fn simulate_portfolio(
    input: &PortfolioSimulationInput,
) -> RearviewResult<PortfolioSimulationOutput> {
    Ok(simulate_portfolio_with_diagnostics(input)?.output)
}

pub fn simulate_portfolio_with_diagnostics(
    input: &PortfolioSimulationInput,
) -> RearviewResult<PortfolioSimulationWithDiagnostics> {
    validate_input(input)?;
    let mut diagnostics = SimulationDiagnostics {
        version: 1,
        simulation_ms: BTreeMap::new(),
        row_counts: BTreeMap::new(),
    };

    let stage_started = Instant::now();
    let prices = PriceStore::new(&input.prices);
    diagnostics.simulation_ms.insert(
        "price_store_build".to_string(),
        stage_started.elapsed().as_millis(),
    );
    diagnostics
        .row_counts
        .insert("price_bar_count".to_string(), input.prices.len());
    diagnostics
        .row_counts
        .insert("price_index_key_count".to_string(), prices.len());

    let stage_started = Instant::now();
    let calendar = TradeCalendarPlan::new(prices.trade_dates());
    diagnostics.simulation_ms.insert(
        "calendar_build".to_string(),
        stage_started.elapsed().as_millis(),
    );
    diagnostics
        .row_counts
        .insert("trade_date_count".to_string(), calendar.trade_dates().len());

    let stage_started = Instant::now();
    let mut signals_by_execution_date: BTreeMap<NaiveDate, Vec<&BuySignalInput>> = BTreeMap::new();
    for signal in &input.signals {
        signals_by_execution_date
            .entry(signal.execution_date)
            .or_default()
            .push(signal);
    }
    for signals in signals_by_execution_date.values_mut() {
        signals.sort_by_key(|signal| signal.rank);
    }
    diagnostics.simulation_ms.insert(
        "signal_index_build".to_string(),
        stage_started.elapsed().as_millis(),
    );
    diagnostics
        .row_counts
        .insert("signal_count".to_string(), input.signals.len());
    diagnostics.row_counts.insert(
        "signal_execution_date_count".to_string(),
        signals_by_execution_date.len(),
    );

    let mut cash = input.initial_cash;
    let mut positions: BTreeMap<String, PositionState> = BTreeMap::new();
    let mut pending_sells: BTreeMap<NaiveDate, Vec<PendingSell>> = BTreeMap::new();
    let mut previous_total_equity = Some(input.initial_cash);
    let mut max_equity = input.initial_cash;
    let mut order_seq = 0_u32;
    let mut trade_seq = 0_u32;
    let mut event_seq = 0_u32;

    let mut targets = Vec::new();
    let mut orders = Vec::new();
    let mut trades = Vec::new();
    let mut position_rows = Vec::new();
    let mut nav_rows = Vec::new();
    let mut events = Vec::new();

    nav_rows.push(PortfolioNavRow {
        trade_date: input.start_date,
        cash_balance: input.initial_cash,
        position_market_value: 0.0,
        total_equity: input.initial_cash,
        nav: 1.0,
        daily_return: None,
        drawdown: 0.0,
        gross_exposure: 0.0,
        position_count: 0,
        turnover: 0.0,
        fee_amount: 0.0,
        warning_count: 0,
    });

    let daily_loop_started = Instant::now();
    let mut sell_handling_ms = 0_u128;
    let mut buy_handling_ms = 0_u128;
    let mut valuation_ms = 0_u128;
    let mut exit_evaluation_ms = 0_u128;
    let mut pending_sell_dequeue_count = 0_usize;
    let mut pending_sell_enqueue_count = 0_usize;
    for (trade_day_index, trade_date) in calendar.trade_dates().iter().copied().enumerate() {
        if trade_date <= input.start_date {
            continue;
        }
        if trade_date > input.end_date {
            break;
        }
        let mut day_fee = 0.0;
        let mut day_turnover = 0.0;
        let mut day_warning_count = 0_usize;

        let stage_started = Instant::now();
        if let Some(sells) = pending_sells.remove(&trade_date) {
            pending_sell_dequeue_count += sells.len();
            for sell in sells {
                if let Some(position) = positions.remove(&sell.security_code) {
                    order_seq += 1;
                    let reference_price = match prices.open_price(trade_date, &sell.security_code) {
                        Some(price) => price,
                        None => {
                            day_warning_count += 1;
                            events.push(event(
                                &mut event_seq,
                                trade_date,
                                Some(sell.security_code.clone()),
                                PortfolioEventType::PriceMissing,
                                "sell skipped because execution open price is missing",
                            ));
                            orders.push(PortfolioOrderRow {
                                order_seq,
                                signal_date: Some(sell.signal_date),
                                execution_date: trade_date,
                                security_code: sell.security_code,
                                side: OrderSide::Sell,
                                order_quantity: position.quantity,
                                order_amount: 0.0,
                                reference_price: None,
                                reason: sell.reason,
                                status: OrderStatus::SkippedPriceMissing,
                            });
                            continue;
                        }
                    };
                    let execution_price =
                        reference_price * (1.0 - input.slippage_profile.sell_bps / 10_000.0);
                    let gross_amount = position.quantity * execution_price;
                    let fee = fee_breakdown(input.fee_profile, OrderSide::Sell, gross_amount);
                    cash += gross_amount - fee.total;
                    day_fee += fee.total;
                    day_turnover += gross_amount;
                    trades.push(trade(TradeInput {
                        trade_seq: &mut trade_seq,
                        order_seq,
                        trade_date,
                        signal_date: Some(sell.signal_date),
                        security_code: sell.security_code.clone(),
                        side: OrderSide::Sell,
                        quantity: position.quantity,
                        reference_price,
                        execution_price,
                        fee,
                        slippage_cost: reference_price * position.quantity - gross_amount,
                        reason: sell.reason,
                    }));
                    orders.push(PortfolioOrderRow {
                        order_seq,
                        signal_date: Some(sell.signal_date),
                        execution_date: trade_date,
                        security_code: sell.security_code,
                        side: OrderSide::Sell,
                        order_quantity: position.quantity,
                        order_amount: gross_amount,
                        reference_price: Some(reference_price),
                        reason: sell.reason,
                        status: OrderStatus::Filled,
                    });
                }
            }
        }
        sell_handling_ms += stage_started.elapsed().as_millis();

        let total_equity_after_sells = cash + prices.market_value(&positions, trade_date);
        let vacant_slots = input.max_positions.saturating_sub(positions.len());
        let stage_started = Instant::now();
        if vacant_slots > 0
            && let Some(signals) = signals_by_execution_date.get(&trade_date)
        {
            let mut filled_slots = 0_usize;
            for signal in signals {
                if filled_slots >= vacant_slots {
                    break;
                }
                if positions.contains_key(&signal.security_code) {
                    continue;
                }
                let target_weight = target_weight_per_position(input);
                let target_amount = total_equity_after_sells * target_weight;
                let reference_price = match prices.open_price(trade_date, &signal.security_code) {
                    Some(price) => price,
                    None => {
                        day_warning_count += 1;
                        events.push(event(
                            &mut event_seq,
                            trade_date,
                            Some(signal.security_code.clone()),
                            PortfolioEventType::PriceMissing,
                            "buy skipped because execution open price is missing",
                        ));
                        order_seq += 1;
                        orders.push(skipped_buy_order(
                            order_seq,
                            signal,
                            OrderStatus::SkippedPriceMissing,
                            None,
                        ));
                        continue;
                    }
                };
                let execution_price =
                    reference_price * (1.0 + input.slippage_profile.buy_bps / 10_000.0);
                let Some((quantity, fee)) =
                    affordable_buy_quantity(input, target_amount, cash, execution_price)
                else {
                    let raw_quantity = target_amount / execution_price;
                    let event_type =
                        if floor_to_lot(raw_quantity, input.lot_size) < min_trade_quantity(input) {
                            PortfolioEventType::TargetAmountBelowMinLot
                        } else {
                            PortfolioEventType::CashInsufficientForMinLot
                        };
                    day_warning_count += 1;
                    events.push(event(
                        &mut event_seq,
                        trade_date,
                        Some(signal.security_code.clone()),
                        event_type,
                        "buy skipped because target or cash cannot cover one lot",
                    ));
                    order_seq += 1;
                    orders.push(skipped_buy_order(
                        order_seq,
                        signal,
                        match event_type {
                            PortfolioEventType::CashInsufficientForMinLot => {
                                OrderStatus::SkippedCashInsufficient
                            }
                            PortfolioEventType::TargetAmountBelowMinLot => {
                                OrderStatus::SkippedBelowMinLot
                            }
                            PortfolioEventType::PriceMissing
                            | PortfolioEventType::IndicatorMissing => {
                                OrderStatus::SkippedPriceMissing
                            }
                        },
                        Some(reference_price),
                    ));
                    continue;
                };

                let gross_amount = quantity * execution_price;
                cash -= gross_amount + fee.total;
                day_fee += fee.total;
                day_turnover += gross_amount;
                order_seq += 1;
                trade_seq += 1;
                targets.push(PortfolioTargetRow {
                    signal_date: signal.signal_date,
                    execution_date: trade_date,
                    security_code: signal.security_code.clone(),
                    source_rank: signal.rank,
                    source_score: signal.score,
                    target_weight,
                    target_amount,
                    target_quantity: quantity,
                    target_reason: TargetReason::BuySignal,
                });
                orders.push(PortfolioOrderRow {
                    order_seq,
                    signal_date: Some(signal.signal_date),
                    execution_date: trade_date,
                    security_code: signal.security_code.clone(),
                    side: OrderSide::Buy,
                    order_quantity: quantity,
                    order_amount: gross_amount,
                    reference_price: Some(reference_price),
                    reason: OrderReason::Rebalance,
                    status: OrderStatus::Filled,
                });
                trades.push(PortfolioTradeRow {
                    trade_seq,
                    order_seq,
                    trade_date,
                    signal_date: Some(signal.signal_date),
                    security_code: signal.security_code.clone(),
                    side: OrderSide::Buy,
                    quantity,
                    reference_price,
                    execution_price,
                    gross_amount,
                    commission: fee.commission,
                    stamp_duty: fee.stamp_duty,
                    transfer_fee: fee.transfer_fee,
                    total_fee: fee.total,
                    slippage_cost: gross_amount - reference_price * quantity,
                    reason: OrderReason::Rebalance,
                });
                positions.insert(
                    signal.security_code.clone(),
                    PositionState {
                        quantity,
                        cost_basis: gross_amount + fee.total,
                        average_entry_price: execution_price,
                        entry_trade_index: trade_day_index,
                    },
                );
                filled_slots += 1;
            }
        }
        buy_handling_ms += stage_started.elapsed().as_millis();

        let stage_started = Instant::now();
        let mut position_market_value = 0.0;
        for (security_code, position) in &positions {
            if let Some(close_price) = prices.close_price(trade_date, security_code) {
                let market_value = position.quantity * close_price;
                let unrealized_pnl = market_value - position.cost_basis;
                let unrealized_return = unrealized_pnl / position.cost_basis;
                position_market_value += market_value;
                position_rows.push(PortfolioPositionDayRow {
                    trade_date,
                    security_code: security_code.clone(),
                    quantity: position.quantity,
                    cost_basis: position.cost_basis,
                    average_entry_price: position.average_entry_price,
                    close_price,
                    market_value,
                    unrealized_pnl,
                    unrealized_return,
                    holding_days: holding_days(position.entry_trade_index, trade_day_index),
                    is_stale_price: false,
                });
            } else {
                day_warning_count += 1;
                events.push(event(
                    &mut event_seq,
                    trade_date,
                    Some(security_code.clone()),
                    PortfolioEventType::PriceMissing,
                    "position valuation skipped because close price is missing",
                ));
            }
        }
        valuation_ms += stage_started.elapsed().as_millis();

        let total_equity = cash + position_market_value;
        max_equity = max_equity.max(total_equity);
        let daily_return = previous_total_equity.map(|previous| total_equity / previous - 1.0);
        previous_total_equity = Some(total_equity);
        let drawdown = if max_equity > 0.0 {
            total_equity / max_equity - 1.0
        } else {
            0.0
        };
        nav_rows.push(PortfolioNavRow {
            trade_date,
            cash_balance: cash,
            position_market_value,
            total_equity,
            nav: total_equity / input.initial_cash,
            daily_return,
            drawdown,
            gross_exposure: if total_equity > 0.0 {
                position_market_value / total_equity
            } else {
                0.0
            },
            position_count: positions.len(),
            turnover: if total_equity > 0.0 {
                day_turnover / total_equity
            } else {
                0.0
            },
            fee_amount: day_fee,
            warning_count: day_warning_count,
        });

        let stage_started = Instant::now();
        for (security_code, position) in &positions {
            if let Some(price_bar) = prices.price_bar(trade_date, security_code) {
                if let Some(metric) = missing_indicator_stop_loss_metric(input, price_bar) {
                    events.push(event(
                        &mut event_seq,
                        trade_date,
                        Some(security_code.clone()),
                        PortfolioEventType::IndicatorMissing,
                        &format!("indicator stop loss skipped because {metric} is missing"),
                    ));
                }
                if let Some(reason) =
                    triggered_exit_reason(input, position, price_bar, trade_day_index)
                    && let Some(next_trade_date) = calendar.next_trade_date(trade_date)
                {
                    pending_sells
                        .entry(next_trade_date)
                        .or_default()
                        .push(PendingSell {
                            signal_date: trade_date,
                            security_code: security_code.clone(),
                            reason,
                        });
                    pending_sell_enqueue_count += 1;
                }
            }
        }
        exit_evaluation_ms += stage_started.elapsed().as_millis();
    }
    diagnostics.simulation_ms.insert(
        "daily_loop".to_string(),
        daily_loop_started.elapsed().as_millis(),
    );
    diagnostics
        .simulation_ms
        .insert("sell_handling".to_string(), sell_handling_ms);
    diagnostics
        .simulation_ms
        .insert("buy_handling".to_string(), buy_handling_ms);
    diagnostics
        .simulation_ms
        .insert("valuation".to_string(), valuation_ms);
    diagnostics
        .simulation_ms
        .insert("exit_evaluation".to_string(), exit_evaluation_ms);
    diagnostics.row_counts.insert(
        "pending_sell_dequeue_count".to_string(),
        pending_sell_dequeue_count,
    );
    diagnostics.row_counts.insert(
        "pending_sell_enqueue_count".to_string(),
        pending_sell_enqueue_count,
    );

    let stage_started = Instant::now();
    let ending_equity = nav_rows
        .last()
        .map(|row| row.total_equity)
        .unwrap_or(input.initial_cash);
    let total_fee = trades.iter().map(|trade| trade.total_fee).sum();
    let max_drawdown = nav_rows
        .iter()
        .map(|row| row.drawdown)
        .fold(0.0_f64, f64::min);
    let output = PortfolioSimulationOutput {
        targets,
        orders,
        trades,
        positions: position_rows,
        nav: nav_rows,
        events,
        summary: PortfolioSummary {
            initial_cash: input.initial_cash,
            ending_equity,
            total_return: ending_equity / input.initial_cash - 1.0,
            max_drawdown,
            trade_count: trade_seq as usize,
            total_fee,
            warning_count: event_seq as usize,
        },
    };
    diagnostics.simulation_ms.insert(
        "output_finalize".to_string(),
        stage_started.elapsed().as_millis(),
    );
    diagnostics
        .row_counts
        .insert("target_count".to_string(), output.targets.len());
    diagnostics
        .row_counts
        .insert("order_count".to_string(), output.orders.len());
    diagnostics
        .row_counts
        .insert("trade_count".to_string(), output.trades.len());
    diagnostics
        .row_counts
        .insert("position_day_count".to_string(), output.positions.len());
    diagnostics
        .row_counts
        .insert("nav_count".to_string(), output.nav.len());
    diagnostics
        .row_counts
        .insert("event_count".to_string(), output.events.len());
    Ok(PortfolioSimulationWithDiagnostics {
        output,
        diagnostics,
    })
}

fn validate_input(input: &PortfolioSimulationInput) -> RearviewResult<()> {
    if input.initial_cash <= 0.0 {
        return Err(RearviewError::Validation(
            "initial_cash must be greater than 0".to_string(),
        ));
    }
    if input.max_positions == 0 {
        return Err(RearviewError::Validation(
            "max_positions must be greater than 0".to_string(),
        ));
    }
    if input.end_date < input.start_date {
        return Err(RearviewError::Validation(
            "end_date must be greater than or equal to start_date".to_string(),
        ));
    }
    if !(0.0..1.0).contains(&input.cash_reserve_pct) {
        return Err(RearviewError::Validation(
            "cash_reserve_pct must be within [0, 1)".to_string(),
        ));
    }
    if let Some(single_position_limit_pct) = input.single_position_limit_pct
        && (!(0.0..=1.0).contains(&single_position_limit_pct) || single_position_limit_pct == 0.0)
    {
        return Err(RearviewError::Validation(
            "single_position_limit_pct must be within (0, 1]".to_string(),
        ));
    }
    if input.lot_size == 0 || input.min_trade_lots == 0 {
        return Err(RearviewError::Validation(
            "lot_size and min_trade_lots must be greater than 0".to_string(),
        ));
    }
    for signal in &input.signals {
        if signal.execution_date <= signal.signal_date {
            return Err(RearviewError::Validation(format!(
                "signal execution_date must be after signal_date: {} {} -> {}",
                signal.security_code, signal.signal_date, signal.execution_date
            )));
        }
    }
    for rule in &input.exit_rules {
        if let ExitRule::IndicatorStopLoss { metric } = rule
            && !is_supported_indicator_stop_loss_metric(metric)
        {
            return Err(RearviewError::Validation(format!(
                "indicator stop loss metric is not supported: {metric}"
            )));
        }
    }
    Ok(())
}

fn target_weight_per_position(input: &PortfolioSimulationInput) -> f64 {
    let equal_weight_after_cash_reserve =
        (1.0 - input.cash_reserve_pct) / input.max_positions as f64;
    input
        .single_position_limit_pct
        .map_or(equal_weight_after_cash_reserve, |limit| {
            equal_weight_after_cash_reserve.min(limit)
        })
}

struct PriceStore<'a> {
    bars: &'a [PriceBar],
    by_date_security: BTreeMap<(NaiveDate, &'a str), usize>,
    trade_dates: Vec<NaiveDate>,
}

impl<'a> PriceStore<'a> {
    fn new(bars: &'a [PriceBar]) -> Self {
        let mut by_date_security = BTreeMap::new();
        let mut trade_dates = BTreeSet::new();
        for (index, price) in bars.iter().enumerate() {
            trade_dates.insert(price.trade_date);
            by_date_security.insert((price.trade_date, price.security_code.as_str()), index);
        }
        Self {
            bars,
            by_date_security,
            trade_dates: trade_dates.into_iter().collect(),
        }
    }

    fn len(&self) -> usize {
        self.by_date_security.len()
    }

    fn trade_dates(&self) -> &[NaiveDate] {
        &self.trade_dates
    }

    fn price_bar(&self, trade_date: NaiveDate, security_code: &str) -> Option<&'a PriceBar> {
        self.by_date_security
            .get(&(trade_date, security_code))
            .map(|index| &self.bars[*index])
    }

    fn open_price(&self, trade_date: NaiveDate, security_code: &str) -> Option<f64> {
        self.price_bar(trade_date, security_code)
            .and_then(|price| price.open_price_backward_adj)
    }

    fn close_price(&self, trade_date: NaiveDate, security_code: &str) -> Option<f64> {
        self.price_bar(trade_date, security_code)
            .and_then(|price| price.close_price_backward_adj)
    }

    fn market_value(
        &self,
        positions: &BTreeMap<String, PositionState>,
        trade_date: NaiveDate,
    ) -> f64 {
        positions
            .iter()
            .filter_map(|(security_code, position)| {
                self.close_price(trade_date, security_code)
                    .map(|price| position.quantity * price)
            })
            .sum()
    }
}

struct TradeCalendarPlan {
    trade_dates: Vec<NaiveDate>,
    next_by_date: BTreeMap<NaiveDate, NaiveDate>,
}

impl TradeCalendarPlan {
    fn new(trade_dates: &[NaiveDate]) -> Self {
        let next_by_date = trade_dates
            .windows(2)
            .map(|window| (window[0], window[1]))
            .collect();
        Self {
            trade_dates: trade_dates.to_vec(),
            next_by_date,
        }
    }

    fn trade_dates(&self) -> &[NaiveDate] {
        &self.trade_dates
    }

    fn next_trade_date(&self, trade_date: NaiveDate) -> Option<NaiveDate> {
        self.next_by_date.get(&trade_date).copied()
    }
}

fn affordable_buy_quantity(
    input: &PortfolioSimulationInput,
    target_amount: f64,
    cash: f64,
    execution_price: f64,
) -> Option<(f64, FeeBreakdown)> {
    let mut quantity = floor_to_lot(target_amount / execution_price, input.lot_size);
    let min_quantity = min_trade_quantity(input);
    while quantity >= min_quantity {
        let gross_amount = quantity * execution_price;
        let fee = fee_breakdown(input.fee_profile, OrderSide::Buy, gross_amount);
        if gross_amount + fee.total <= cash {
            return Some((quantity, fee));
        }
        quantity -= input.lot_size as f64;
    }
    None
}

fn floor_to_lot(quantity: f64, lot_size: u32) -> f64 {
    (quantity / lot_size as f64).floor() * lot_size as f64
}

fn min_trade_quantity(input: &PortfolioSimulationInput) -> f64 {
    (input.lot_size * input.min_trade_lots) as f64
}

fn fee_breakdown(profile: FeeProfile, side: OrderSide, gross_amount: f64) -> FeeBreakdown {
    let commission_rate = profile.commission_rate.min(profile.commission_rate_max);
    let commission = (gross_amount * commission_rate).max(profile.min_commission);
    let stamp_duty = if side == OrderSide::Sell {
        gross_amount * profile.stamp_duty_rate_sell
    } else {
        0.0
    };
    let transfer_fee = gross_amount * profile.transfer_fee_rate;
    let total = commission + stamp_duty + transfer_fee;
    FeeBreakdown {
        commission,
        stamp_duty,
        transfer_fee,
        total,
    }
}

struct TradeInput<'a> {
    trade_seq: &'a mut u32,
    order_seq: u32,
    trade_date: NaiveDate,
    signal_date: Option<NaiveDate>,
    security_code: String,
    side: OrderSide,
    quantity: f64,
    reference_price: f64,
    execution_price: f64,
    fee: FeeBreakdown,
    slippage_cost: f64,
    reason: OrderReason,
}

fn trade(input: TradeInput<'_>) -> PortfolioTradeRow {
    *input.trade_seq += 1;
    let gross_amount = input.quantity * input.execution_price;
    PortfolioTradeRow {
        trade_seq: *input.trade_seq,
        order_seq: input.order_seq,
        trade_date: input.trade_date,
        signal_date: input.signal_date,
        security_code: input.security_code,
        side: input.side,
        quantity: input.quantity,
        reference_price: input.reference_price,
        execution_price: input.execution_price,
        gross_amount,
        commission: input.fee.commission,
        stamp_duty: input.fee.stamp_duty,
        transfer_fee: input.fee.transfer_fee,
        total_fee: input.fee.total,
        slippage_cost: input.slippage_cost,
        reason: input.reason,
    }
}

fn skipped_buy_order(
    order_seq: u32,
    signal: &BuySignalInput,
    status: OrderStatus,
    reference_price: Option<f64>,
) -> PortfolioOrderRow {
    PortfolioOrderRow {
        order_seq,
        signal_date: Some(signal.signal_date),
        execution_date: signal.execution_date,
        security_code: signal.security_code.clone(),
        side: OrderSide::Buy,
        order_quantity: 0.0,
        order_amount: 0.0,
        reference_price,
        reason: OrderReason::Rebalance,
        status,
    }
}

fn event(
    event_seq: &mut u32,
    trade_date: NaiveDate,
    security_code: Option<String>,
    event_type: PortfolioEventType,
    message: &str,
) -> PortfolioEventRow {
    *event_seq += 1;
    PortfolioEventRow {
        event_seq: *event_seq,
        trade_date,
        security_code,
        event_type,
        message: message.to_string(),
    }
}

fn triggered_exit_reason(
    input: &PortfolioSimulationInput,
    position: &PositionState,
    price_bar: &PriceBar,
    trade_day_index: usize,
) -> Option<OrderReason> {
    let close_price = price_bar.close_price_backward_adj?;
    let unrealized_return = close_price / position.average_entry_price - 1.0;
    for rule in &input.exit_rules {
        match rule {
            ExitRule::FixedStopLoss { loss_pct } if unrealized_return <= -*loss_pct => {
                return Some(OrderReason::FixedStopLoss);
            }
            ExitRule::TakeProfit { profit_pct } if unrealized_return >= *profit_pct => {
                return Some(OrderReason::TakeProfit);
            }
            ExitRule::TimeStopLoss {
                holding_days: rule_holding_days,
                max_return_pct,
            } if holding_days(position.entry_trade_index, trade_day_index)
                >= *rule_holding_days
                && unrealized_return < *max_return_pct =>
            {
                return Some(OrderReason::TimeStopLoss);
            }
            ExitRule::IndicatorStopLoss { metric }
                if indicator_close_price(price_bar)
                    .zip(trend_metric_value(price_bar, metric))
                    .is_some_and(|(indicator_close, value)| indicator_close < value) =>
            {
                return Some(OrderReason::IndicatorStopLoss);
            }
            _ => {}
        }
    }
    None
}

fn indicator_close_price(price_bar: &PriceBar) -> Option<f64> {
    price_bar
        .close_price_forward_adj
        .or(price_bar.close_price_backward_adj)
}

fn missing_indicator_stop_loss_metric<'a>(
    input: &'a PortfolioSimulationInput,
    price_bar: &PriceBar,
) -> Option<&'a str> {
    indicator_close_price(price_bar)?;
    input.exit_rules.iter().find_map(|rule| {
        if let ExitRule::IndicatorStopLoss { metric } = rule
            && trend_metric_value(price_bar, metric).is_none()
        {
            Some(metric.as_str())
        } else {
            None
        }
    })
}

fn trend_metric_value(price_bar: &PriceBar, metric: &str) -> Option<f64> {
    match metric {
        "price_ma_3" => price_bar.price_ma_3,
        "price_ma_5" => price_bar.price_ma_5,
        "price_ma_6" => price_bar.price_ma_6,
        "price_ma_10" => price_bar.price_ma_10,
        "price_ma_12" => price_bar.price_ma_12,
        "price_ma_14" => price_bar.price_ma_14,
        "price_ma_20" => price_bar.price_ma_20,
        "price_ma_24" => price_bar.price_ma_24,
        "price_ma_28" => price_bar.price_ma_28,
        "price_ma_30" => price_bar.price_ma_30,
        "price_ma_57" => price_bar.price_ma_57,
        "price_ma_60" => price_bar.price_ma_60,
        "price_ma_114" => price_bar.price_ma_114,
        "price_ma_250" => price_bar.price_ma_250,
        "price_avg_ma_3_6_12_24" => price_bar.price_avg_ma_3_6_12_24,
        "price_avg_ma_14_28_57_114" => price_bar.price_avg_ma_14_28_57_114,
        "price_ema2_10" => price_bar.price_ema2_10,
        _ => None,
    }
}

fn is_supported_indicator_stop_loss_metric(metric: &str) -> bool {
    matches!(
        metric,
        "price_ma_3"
            | "price_ma_5"
            | "price_ma_6"
            | "price_ma_10"
            | "price_ma_12"
            | "price_ma_14"
            | "price_ma_20"
            | "price_ma_24"
            | "price_ma_28"
            | "price_ma_30"
            | "price_ma_57"
            | "price_ma_60"
            | "price_ma_114"
            | "price_ma_250"
            | "price_avg_ma_3_6_12_24"
            | "price_avg_ma_14_28_57_114"
            | "price_ema2_10"
    )
}

fn holding_days(entry_trade_index: usize, current_trade_index: usize) -> u32 {
    current_trade_index.saturating_sub(entry_trade_index) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_should_allow_reentry_after_security_is_sold() {
        let input = fixture_input();

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let bought_codes: Vec<_> = output
            .trades
            .iter()
            .filter(|trade| trade.side == OrderSide::Buy)
            .map(|trade| trade.security_code.as_str())
            .collect();
        assert_eq!(bought_codes, ["AAA", "BBB", "AAA"]);
    }

    #[test]
    fn simulation_should_not_repeat_current_positions() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let mut input = fixture_input();
        input.max_positions = 2;
        input.exit_rules = Vec::new();
        input.signals = vec![
            next_open_signal(d1, "AAA", 1),
            next_open_signal(d2, "AAA", 1),
        ];
        input.prices = vec![price(d1, "AAA", 10.0, 10.0), price(d2, "AAA", 10.0, 10.0)];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let aaa_buy_count = output
            .trades
            .iter()
            .filter(|trade| trade.security_code == "AAA" && trade.side == OrderSide::Buy)
            .count();
        assert_eq!(aaa_buy_count, 1);
    }

    #[test]
    fn simulation_should_cap_buys_by_vacant_slots_after_daily_top_n_candidates() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let mut input = fixture_input();
        input.initial_cash = 200_000.0;
        input.max_positions = 5;
        input.exit_rules = Vec::new();
        input.signals = vec![
            next_open_signal(d1, "AAA", 1),
            next_open_signal(d1, "BBB", 2),
            next_open_signal(d1, "CCC", 3),
            next_open_signal(d2, "DDD", 1),
            next_open_signal(d2, "EEE", 2),
            next_open_signal(d2, "FFF", 3),
            next_open_signal(d2, "GGG", 4),
            next_open_signal(d2, "HHH", 5),
            next_open_signal(d2, "III", 6),
        ];
        input.prices = [
            "AAA", "BBB", "CCC", "DDD", "EEE", "FFF", "GGG", "HHH", "III",
        ]
        .into_iter()
        .flat_map(|security_code| {
            [
                price(d1, security_code, 10.0, 10.0),
                price(d2, security_code, 10.0, 10.0),
            ]
        })
        .collect();

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let bought_codes: Vec<_> = output
            .trades
            .iter()
            .filter(|trade| trade.side == OrderSide::Buy)
            .map(|trade| trade.security_code.as_str())
            .collect();
        assert_eq!(bought_codes, ["AAA", "BBB", "CCC", "DDD", "EEE"]);
    }

    #[test]
    fn simulation_should_not_output_rows_after_end_date() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        let mut input = fixture_input();
        input.end_date = d2;
        input.exit_rules = Vec::new();
        input.signals = vec![next_open_signal(d1, "AAA", 1), signal(d2, d3, "BBB", 1)];
        input.prices = vec![
            price(d1, "AAA", 10.0, 10.0),
            price(d2, "AAA", 10.0, 10.0),
            price(d3, "AAA", 10.0, 10.0),
            price(d3, "BBB", 10.0, 10.0),
        ];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(
            output
                .nav
                .iter()
                .all(|row| row.trade_date <= input.end_date)
        );
        assert!(
            output
                .trades
                .iter()
                .all(|trade| trade.trade_date <= input.end_date)
        );
        assert!(
            !output
                .trades
                .iter()
                .any(|trade| trade.security_code == "BBB")
        );
    }

    #[test]
    fn trade_calendar_plan_should_not_return_next_date_for_last_trade_date() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let calendar = TradeCalendarPlan::new(&[d1, d2]);

        assert_eq!(calendar.next_trade_date(d1), Some(d2));
        assert_eq!(calendar.next_trade_date(d2), None);
    }

    #[test]
    fn price_store_should_read_prices_without_string_key_allocation_contract() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let prices = vec![
            price(d1, "AAA", 10.0, 11.0),
            price(d1, "AAA", 12.0, 13.0),
            price_with_metric(d2, "BBB", 20.0, 21.0, "price_ma_10", None),
        ];
        let store = PriceStore::new(&prices);

        assert_eq!(store.len(), 2);
        assert_eq!(store.open_price(d1, "AAA"), Some(12.0));
        assert_eq!(store.close_price(d1, "AAA"), Some(13.0));
        assert_eq!(store.open_price(d2, "AAA"), None);
        assert_eq!(
            store.price_bar(d2, "BBB").and_then(|bar| bar.price_ma_10),
            None
        );
        assert_eq!(store.trade_dates(), &[d1, d2]);
    }

    #[test]
    fn simulation_diagnostics_should_include_stable_timing_and_row_count_keys() {
        let input = fixture_input();

        let simulation = simulate_portfolio_with_diagnostics(&input)
            .expect("simulation with diagnostics should succeed");

        assert_eq!(simulation.diagnostics.version, 1);
        for key in [
            "price_store_build",
            "calendar_build",
            "signal_index_build",
            "daily_loop",
            "exit_evaluation",
            "output_finalize",
        ] {
            assert!(
                simulation.diagnostics.simulation_ms.contains_key(key),
                "missing timing key {key}"
            );
        }
        assert_eq!(
            simulation.diagnostics.row_counts.get("price_bar_count"),
            Some(&input.prices.len())
        );
        assert_eq!(
            simulation.diagnostics.row_counts.get("nav_count"),
            Some(&simulation.output.nav.len())
        );
    }

    #[test]
    fn simulation_should_not_enqueue_exit_on_missing_next_trade_date() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let mut input = fixture_input();
        input.end_date = d2;
        input.max_positions = 1;
        input.exit_rules = vec![ExitRule::FixedStopLoss { loss_pct: 0.01 }];
        input.signals = vec![next_open_signal(d1, "AAA", 1)];
        input.prices = vec![price(d1, "AAA", 10.0, 10.0), price(d2, "AAA", 10.0, 5.0)];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(
            output
                .trades
                .iter()
                .any(|trade| trade.side == OrderSide::Buy)
        );
        assert!(
            !output
                .trades
                .iter()
                .any(|trade| trade.side == OrderSide::Sell)
        );
        assert!(output.orders.iter().all(|order| order.execution_date <= d2));
    }

    #[test]
    fn fills_orders_in_lot_size_and_records_take_profit_sell() {
        let input = fixture_input();

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(
            output
                .trades
                .iter()
                .all(|trade| trade.quantity % 100.0 == 0.0)
        );
        assert!(
            output
                .trades
                .iter()
                .any(|trade| trade.security_code == "AAA"
                    && trade.side == OrderSide::Sell
                    && trade.reason == OrderReason::TakeProfit)
        );
    }

    #[test]
    fn skips_buy_when_cash_cannot_cover_one_lot() {
        let mut input = fixture_input();
        input.initial_cash = 500.0;

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(output.trades.is_empty());
        assert!(output.events.iter().any(|event| event.event_type
            == PortfolioEventType::CashInsufficientForMinLot
            || event.event_type == PortfolioEventType::TargetAmountBelowMinLot));
    }

    #[test]
    fn time_stop_loss_sells_when_holding_days_reaches_threshold() {
        let mut input = fixture_input();
        input.exit_rules = vec![ExitRule::TimeStopLoss {
            holding_days: 1,
            max_return_pct: 0.20,
        }];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(output.trades.iter().any(|trade| {
            trade.security_code == "AAA"
                && trade.side == OrderSide::Sell
                && trade.reason == OrderReason::TimeStopLoss
        }));
    }

    #[test]
    fn time_stop_loss_counts_trading_days_instead_of_calendar_days() {
        let d1 = date(2024, 1, 5);
        let d2 = date(2024, 1, 8);
        let d3 = date(2024, 1, 9);
        let d4 = date(2024, 1, 10);
        let mut input = fixture_input();
        input.max_positions = 1;
        input.end_date = d4;
        input.exit_rules = vec![ExitRule::TimeStopLoss {
            holding_days: 2,
            max_return_pct: 0.20,
        }];
        input.signals = vec![next_open_signal(d1, "AAA", 1)];
        input.prices = vec![
            price(d1, "AAA", 10.0, 10.0),
            price(d2, "AAA", 10.0, 10.0),
            price(d3, "AAA", 10.0, 10.0),
            price(d4, "AAA", 10.0, 10.0),
        ];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let sell_trade = output
            .trades
            .iter()
            .find(|trade| trade.side == OrderSide::Sell)
            .expect("time stop should sell after two trading holding days");
        assert_eq!(sell_trade.trade_date, d4);
    }

    #[test]
    fn indicator_stop_loss_sells_when_close_is_below_metric() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        let mut input = fixture_input();
        input.max_positions = 1;
        input.exit_rules = vec![ExitRule::IndicatorStopLoss {
            metric: "price_ma_10".to_string(),
        }];
        input.signals = vec![next_open_signal(d1, "AAA", 1)];
        input.prices = vec![
            price(d1, "AAA", 10.0, 10.0),
            price_with_metric(d2, "AAA", 10.0, 9.0, "price_ma_10", Some(10.0)),
            price(d3, "AAA", 9.0, 9.0),
        ];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(output.trades.iter().any(|trade| {
            trade.security_code == "AAA"
                && trade.side == OrderSide::Sell
                && trade.reason == OrderReason::IndicatorStopLoss
                && trade.trade_date == d3
        }));
    }

    #[test]
    fn indicator_stop_loss_sells_when_close_is_below_ma_combo() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        let mut input = fixture_input();
        input.max_positions = 1;
        input.exit_rules = vec![ExitRule::IndicatorStopLoss {
            metric: "price_avg_ma_3_6_12_24".to_string(),
        }];
        input.signals = vec![next_open_signal(d1, "AAA", 1)];
        input.prices = vec![
            price(d1, "AAA", 10.0, 10.0),
            price_with_metric(d2, "AAA", 10.0, 9.0, "price_avg_ma_3_6_12_24", Some(10.0)),
            price(d3, "AAA", 9.0, 9.0),
        ];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(output.trades.iter().any(|trade| {
            trade.security_code == "AAA"
                && trade.side == OrderSide::Sell
                && trade.reason == OrderReason::IndicatorStopLoss
                && trade.trade_date == d3
        }));
    }

    #[test]
    fn indicator_stop_loss_sells_when_close_is_below_ema() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        let mut input = fixture_input();
        input.max_positions = 1;
        input.exit_rules = vec![ExitRule::IndicatorStopLoss {
            metric: "price_ema2_10".to_string(),
        }];
        input.signals = vec![next_open_signal(d1, "AAA", 1)];
        input.prices = vec![
            price(d1, "AAA", 10.0, 10.0),
            price_with_metric(d2, "AAA", 10.0, 9.0, "price_ema2_10", Some(10.0)),
            price(d3, "AAA", 9.0, 9.0),
        ];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(output.trades.iter().any(|trade| {
            trade.security_code == "AAA"
                && trade.side == OrderSide::Sell
                && trade.reason == OrderReason::IndicatorStopLoss
                && trade.trade_date == d3
        }));
    }

    #[test]
    fn indicator_stop_loss_does_not_sell_when_metric_is_missing() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        let mut input = fixture_input();
        input.max_positions = 1;
        input.exit_rules = vec![ExitRule::IndicatorStopLoss {
            metric: "price_ma_10".to_string(),
        }];
        input.signals = vec![next_open_signal(d1, "AAA", 1)];
        input.prices = vec![
            price(d1, "AAA", 10.0, 10.0),
            price(d2, "AAA", 10.0, 9.0),
            price(d3, "AAA", 9.0, 9.0),
        ];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        assert!(!output.trades.iter().any(|trade| {
            trade.security_code == "AAA"
                && trade.side == OrderSide::Sell
                && trade.reason == OrderReason::IndicatorStopLoss
        }));
        assert!(output.events.iter().any(|event| {
            event.security_code.as_deref() == Some("AAA")
                && event.event_type == PortfolioEventType::IndicatorMissing
        }));
    }

    #[test]
    fn summary_max_drawdown_uses_lowest_nav_drawdown() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        let mut input = fixture_input();
        input.initial_cash = 2_000.0;
        input.max_positions = 1;
        input.exit_rules = Vec::new();
        input.signals = vec![next_open_signal(d1, "AAA", 1)];
        input.prices = vec![
            price(d1, "AAA", 10.0, 10.0),
            price(d2, "AAA", 10.0, 5.0),
            price(d3, "AAA", 10.0, 12.0),
        ];

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let lowest_nav_drawdown = output
            .nav
            .iter()
            .map(|row| row.drawdown)
            .fold(0.0_f64, f64::min);
        assert_eq!(output.summary.max_drawdown, lowest_nav_drawdown);
        assert!(output.summary.max_drawdown < -0.20);
    }

    #[test]
    fn nav_starts_from_initial_cash_before_first_execution() {
        let input = fixture_input();

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let first_nav = output.nav.first().expect("initial nav row should exist");
        assert_eq!(first_nav.trade_date, input.start_date);
        assert_eq!(first_nav.nav, 1.0);
        assert_eq!(first_nav.cash_balance, input.initial_cash);
        assert_eq!(first_nav.position_market_value, 0.0);
        assert_eq!(first_nav.position_count, 0);
    }

    #[test]
    fn simulation_should_reject_same_day_execution_signal() {
        let d1 = date(2024, 1, 2);
        let mut input = fixture_input();
        input.signals = vec![signal(d1, d1, "AAA", 1)];

        let error = simulate_portfolio(&input).expect_err("same-day signal should be rejected");

        assert!(
            error
                .to_string()
                .contains("execution_date must be after signal_date")
        );
    }

    #[test]
    fn single_position_limit_should_cap_target_weight_and_leave_cash_unallocated() {
        let mut input = fixture_input();
        input.max_positions = 5;
        input.single_position_limit_pct = Some(0.10);
        input.exit_rules = Vec::new();

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let first_target = output
            .targets
            .iter()
            .find(|target| target.security_code == "AAA")
            .expect("first ranked signal should create a target");
        assert_eq!(first_target.target_weight, 0.10);
        assert!(
            output
                .nav
                .iter()
                .any(|row| row.cash_balance > input.initial_cash * 0.75)
        );
    }

    #[test]
    fn missing_single_position_limit_should_keep_legacy_equal_weight_behavior() {
        let mut input = fixture_input();
        input.max_positions = 5;
        input.single_position_limit_pct = None;
        input.exit_rules = Vec::new();

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let first_target = output
            .targets
            .iter()
            .find(|target| target.security_code == "AAA")
            .expect("first ranked signal should create a target");
        assert_eq!(first_target.target_weight, 0.20);
    }

    fn fixture_input() -> PortfolioSimulationInput {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        PortfolioSimulationInput {
            start_date: date(2024, 1, 1),
            end_date: d3,
            initial_cash: 20_000.0,
            max_positions: 2,
            single_position_limit_pct: None,
            cash_reserve_pct: 0.0,
            lot_size: 100,
            min_trade_lots: 1,
            fee_profile: FeeProfile {
                commission_rate: 0.0001,
                commission_rate_max: 0.003,
                min_commission: 5.0,
                stamp_duty_rate_sell: 0.0005,
                transfer_fee_rate: 0.00001,
            },
            slippage_profile: SlippageProfile {
                buy_bps: 10.0,
                sell_bps: 10.0,
            },
            exit_rules: vec![ExitRule::TakeProfit { profit_pct: 0.10 }],
            signals: vec![
                next_open_signal(d1, "AAA", 1),
                next_open_signal(d1, "BBB", 2),
                next_open_signal(d2, "AAA", 1),
                next_open_signal(d3, "CCC", 1),
            ],
            prices: vec![
                price(d1, "AAA", 10.0, 12.0),
                price(d1, "BBB", 20.0, 20.0),
                price(d1, "CCC", 30.0, 30.0),
                price(d2, "AAA", 12.0, 12.0),
                price(d2, "BBB", 20.0, 20.0),
                price(d2, "CCC", 30.0, 30.0),
                price(d3, "AAA", 12.0, 12.0),
                price(d3, "BBB", 20.0, 20.0),
                price(d3, "CCC", 30.0, 30.0),
            ],
        }
    }

    fn next_open_signal(
        execution_date: NaiveDate,
        security_code: &str,
        rank: u32,
    ) -> BuySignalInput {
        signal(
            execution_date
                .pred_opt()
                .expect("fixture execution date should have a previous date"),
            execution_date,
            security_code,
            rank,
        )
    }

    fn signal(
        signal_date: NaiveDate,
        execution_date: NaiveDate,
        security_code: &str,
        rank: u32,
    ) -> BuySignalInput {
        BuySignalInput {
            signal_date,
            execution_date,
            security_code: security_code.to_string(),
            rank,
            score: 1.0,
        }
    }

    fn price(trade_date: NaiveDate, security_code: &str, open: f64, close: f64) -> PriceBar {
        PriceBar {
            security_code: security_code.to_string(),
            trade_date,
            open_price_backward_adj: Some(open),
            close_price_backward_adj: Some(close),
            close_price_forward_adj: None,
            price_ma_3: None,
            price_ma_5: None,
            price_ma_6: None,
            price_ma_10: None,
            price_ma_12: None,
            price_ma_14: None,
            price_ma_20: None,
            price_ma_24: None,
            price_ma_28: None,
            price_ma_30: None,
            price_ma_57: None,
            price_ma_60: None,
            price_ma_114: None,
            price_ma_250: None,
            price_avg_ma_3_6_12_24: None,
            price_avg_ma_14_28_57_114: None,
            price_ema2_10: None,
            boll_lower_20_2: None,
        }
    }

    fn price_with_metric(
        trade_date: NaiveDate,
        security_code: &str,
        open: f64,
        close: f64,
        metric: &str,
        value: Option<f64>,
    ) -> PriceBar {
        let mut bar = price(trade_date, security_code, open, close);
        match metric {
            "price_ma_10" => bar.price_ma_10 = value,
            "price_avg_ma_3_6_12_24" => bar.price_avg_ma_3_6_12_24 = value,
            "price_ema2_10" => bar.price_ema2_10 = value,
            other => panic!("unsupported test metric: {other}"),
        }
        bar
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }
}
