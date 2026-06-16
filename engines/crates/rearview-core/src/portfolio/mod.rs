use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{RearviewError, RearviewResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSimulationInput {
    pub start_date: NaiveDate,
    pub initial_cash: f64,
    pub max_positions: usize,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

pub fn simulate_portfolio(
    input: &PortfolioSimulationInput,
) -> RearviewResult<PortfolioSimulationOutput> {
    validate_input(input)?;

    let mut prices: BTreeMap<(NaiveDate, String), PriceBar> = BTreeMap::new();
    let mut trade_dates = BTreeSet::new();
    for price in &input.prices {
        trade_dates.insert(price.trade_date);
        prices.insert(
            (price.trade_date, price.security_code.clone()),
            price.clone(),
        );
    }

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

    let mut cash = input.initial_cash;
    let mut positions: BTreeMap<String, PositionState> = BTreeMap::new();
    let mut bought_history: BTreeSet<String> = BTreeSet::new();
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

    for (trade_day_index, trade_date) in trade_dates.into_iter().enumerate() {
        if trade_date <= input.start_date {
            continue;
        }
        let mut day_fee = 0.0;
        let mut day_turnover = 0.0;

        if let Some(sells) = pending_sells.remove(&trade_date) {
            for sell in sells {
                if let Some(position) = positions.remove(&sell.security_code) {
                    order_seq += 1;
                    let reference_price = match open_price(&prices, trade_date, &sell.security_code)
                    {
                        Some(price) => price,
                        None => {
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

        let total_equity_after_sells = cash + market_value(&positions, &prices, trade_date);
        let vacant_slots = input.max_positions.saturating_sub(positions.len());
        if vacant_slots > 0
            && let Some(signals) = signals_by_execution_date.get(&trade_date)
        {
            let held: BTreeSet<String> = positions.keys().cloned().collect();
            let mut filled_slots = 0_usize;
            for signal in signals {
                if filled_slots >= vacant_slots {
                    break;
                }
                if held.contains(&signal.security_code)
                    || positions.contains_key(&signal.security_code)
                    || bought_history.contains(&signal.security_code)
                {
                    continue;
                }
                let target_weight = (1.0 - input.cash_reserve_pct) / input.max_positions as f64;
                let target_amount = total_equity_after_sells * target_weight;
                let reference_price = match open_price(&prices, trade_date, &signal.security_code) {
                    Some(price) => price,
                    None => {
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
                            PortfolioEventType::PriceMissing => OrderStatus::SkippedPriceMissing,
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
                bought_history.insert(signal.security_code.clone());
                filled_slots += 1;
            }
        }

        let mut position_market_value = 0.0;
        for (security_code, position) in &positions {
            if let Some(close_price) = close_price(&prices, trade_date, security_code) {
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
                events.push(event(
                    &mut event_seq,
                    trade_date,
                    Some(security_code.clone()),
                    PortfolioEventType::PriceMissing,
                    "position valuation skipped because close price is missing",
                ));
            }
        }

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
            warning_count: events
                .iter()
                .filter(|event| event.trade_date == trade_date)
                .count(),
        });

        for (security_code, position) in &positions {
            if let Some(close_price) = close_price(&prices, trade_date, security_code)
                && let Some(reason) =
                    triggered_exit_reason(input, position, close_price, trade_day_index)
            {
                pending_sells
                    .entry(next_trade_date(&prices, trade_date))
                    .or_default()
                    .push(PendingSell {
                        signal_date: trade_date,
                        security_code: security_code.clone(),
                        reason,
                    });
            }
        }
    }

    let ending_equity = nav_rows
        .last()
        .map(|row| row.total_equity)
        .unwrap_or(input.initial_cash);
    let total_fee = trades.iter().map(|trade| trade.total_fee).sum();
    let max_drawdown = nav_rows
        .iter()
        .map(|row| row.drawdown)
        .fold(0.0_f64, f64::min);
    Ok(PortfolioSimulationOutput {
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
    if input.lot_size == 0 || input.min_trade_lots == 0 {
        return Err(RearviewError::Validation(
            "lot_size and min_trade_lots must be greater than 0".to_string(),
        ));
    }
    Ok(())
}

fn open_price(
    prices: &BTreeMap<(NaiveDate, String), PriceBar>,
    trade_date: NaiveDate,
    security_code: &str,
) -> Option<f64> {
    prices
        .get(&(trade_date, security_code.to_string()))
        .and_then(|price| price.open_price_backward_adj)
}

fn close_price(
    prices: &BTreeMap<(NaiveDate, String), PriceBar>,
    trade_date: NaiveDate,
    security_code: &str,
) -> Option<f64> {
    prices
        .get(&(trade_date, security_code.to_string()))
        .and_then(|price| price.close_price_backward_adj)
}

fn market_value(
    positions: &BTreeMap<String, PositionState>,
    prices: &BTreeMap<(NaiveDate, String), PriceBar>,
    trade_date: NaiveDate,
) -> f64 {
    positions
        .iter()
        .filter_map(|(security_code, position)| {
            close_price(prices, trade_date, security_code).map(|price| position.quantity * price)
        })
        .sum()
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
    close_price: f64,
    trade_day_index: usize,
) -> Option<OrderReason> {
    let unrealized_return = close_price / position.average_entry_price - 1.0;
    for rule in &input.exit_rules {
        match *rule {
            ExitRule::FixedStopLoss { loss_pct } if unrealized_return <= -loss_pct => {
                return Some(OrderReason::FixedStopLoss);
            }
            ExitRule::TakeProfit { profit_pct } if unrealized_return >= profit_pct => {
                return Some(OrderReason::TakeProfit);
            }
            ExitRule::TimeStopLoss {
                holding_days: rule_holding_days,
                max_return_pct,
            } if holding_days(position.entry_trade_index, trade_day_index) >= rule_holding_days
                && unrealized_return < max_return_pct =>
            {
                return Some(OrderReason::TimeStopLoss);
            }
            _ => {}
        }
    }
    None
}

fn next_trade_date(
    prices: &BTreeMap<(NaiveDate, String), PriceBar>,
    trade_date: NaiveDate,
) -> NaiveDate {
    prices
        .keys()
        .map(|(date, _)| *date)
        .find(|date| *date > trade_date)
        .unwrap_or(trade_date)
}

fn holding_days(entry_trade_index: usize, current_trade_index: usize) -> u32 {
    current_trade_index.saturating_sub(entry_trade_index) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_vacant_slots_by_rank_and_does_not_repeat_existing_positions() {
        let input = fixture_input();

        let output = simulate_portfolio(&input).expect("simulation should succeed");

        let bought_codes: Vec<_> = output
            .trades
            .iter()
            .filter(|trade| trade.side == OrderSide::Buy)
            .map(|trade| trade.security_code.as_str())
            .collect();
        assert_eq!(bought_codes, ["AAA", "BBB", "CCC"]);
        assert!(
            output
                .trades
                .iter()
                .all(|trade| trade.quantity % 100.0 == 0.0)
        );
        assert_eq!(
            output
                .orders
                .iter()
                .filter(|order| order.security_code == "AAA" && order.side == OrderSide::Buy)
                .count(),
            1
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
        input.exit_rules = vec![ExitRule::TimeStopLoss {
            holding_days: 2,
            max_return_pct: 0.20,
        }];
        input.signals = vec![signal(d1, d1, "AAA", 1)];
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
    fn summary_max_drawdown_uses_lowest_nav_drawdown() {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        let mut input = fixture_input();
        input.initial_cash = 2_000.0;
        input.max_positions = 1;
        input.exit_rules = Vec::new();
        input.signals = vec![signal(d1, d1, "AAA", 1)];
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

    fn fixture_input() -> PortfolioSimulationInput {
        let d1 = date(2024, 1, 2);
        let d2 = date(2024, 1, 3);
        let d3 = date(2024, 1, 4);
        PortfolioSimulationInput {
            start_date: date(2024, 1, 1),
            initial_cash: 20_000.0,
            max_positions: 2,
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
                signal(d1, d1, "AAA", 1),
                signal(d1, d1, "BBB", 2),
                signal(d2, d2, "AAA", 1),
                signal(d3, d3, "CCC", 1),
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
        }
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }
}
