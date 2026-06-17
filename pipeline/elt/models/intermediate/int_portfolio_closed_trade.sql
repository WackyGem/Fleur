{{ config(materialized='view') }}

with complete_attempts as (
    select
        portfolio_run_id,
        result_attempt_id
    from {{ source('fleur_portfolio', 'portfolio_run_snapshot') }}
),

closed_trades as (
    select
        portfolio_run_id,
        result_attempt_id,
        closed_trade_id,
        closed_trade_seq,
        position_lot_id,
        entry_trade_seq,
        exit_trade_seq,
        security_code,
        entry_date,
        exit_date,
        quantity,
        entry_gross_amount,
        exit_gross_amount,
        entry_fee,
        exit_fee,
        realized_pnl,
        holding_days,
        exit_reason,
        created_at
    from {{ source('fleur_calculation', 'calc_portfolio_closed_trade') }}
)

select
    closed_trades.portfolio_run_id,
    closed_trades.result_attempt_id,
    closed_trades.closed_trade_id,
    closed_trades.closed_trade_seq,
    closed_trades.position_lot_id,
    closed_trades.entry_trade_seq,
    closed_trades.exit_trade_seq,
    closed_trades.security_code,
    closed_trades.entry_date,
    closed_trades.exit_date,
    closed_trades.quantity,
    closed_trades.entry_gross_amount,
    closed_trades.exit_gross_amount,
    closed_trades.entry_fee,
    closed_trades.exit_fee,
    closed_trades.entry_fee + closed_trades.exit_fee as total_fee,
    closed_trades.realized_pnl,
    if(
        closed_trades.entry_gross_amount + closed_trades.entry_fee = 0,
        null,
        closed_trades.realized_pnl / (closed_trades.entry_gross_amount + closed_trades.entry_fee)
    ) as realized_return,
    closed_trades.holding_days,
    closed_trades.exit_reason,
    closed_trades.created_at
from closed_trades
inner join complete_attempts
    on closed_trades.portfolio_run_id = complete_attempts.portfolio_run_id
    and closed_trades.result_attempt_id = complete_attempts.result_attempt_id
