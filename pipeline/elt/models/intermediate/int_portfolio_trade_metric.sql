{{ config(materialized='view') }}

with complete_attempts as (
    select
        portfolio_run_id,
        result_attempt_id
    from {{ source('fleur_portfolio', 'portfolio_run_snapshot') }}
),

metrics as (
    select
        portfolio_run_id,
        result_attempt_id,
        window_key,
        window_start,
        window_end,
        closed_trade_count,
        winning_trade_count,
        losing_trade_count,
        breakeven_trade_count,
        win_rate_closed_trades,
        average_win_return,
        average_loss_return,
        profit_loss_ratio,
        average_holding_days,
        largest_win_return,
        largest_loss_return,
        computed_at
    from {{ source('fleur_calculation', 'calc_portfolio_trade_metric') }}
)

select
    metrics.portfolio_run_id,
    metrics.result_attempt_id,
    metrics.window_key,
    metrics.window_start,
    metrics.window_end,
    metrics.closed_trade_count,
    metrics.winning_trade_count,
    metrics.losing_trade_count,
    metrics.breakeven_trade_count,
    metrics.win_rate_closed_trades,
    metrics.average_win_return,
    metrics.average_loss_return,
    metrics.profit_loss_ratio,
    metrics.average_holding_days,
    metrics.largest_win_return,
    metrics.largest_loss_return,
    metrics.computed_at
from metrics
inner join complete_attempts
    on metrics.portfolio_run_id = complete_attempts.portfolio_run_id
    and metrics.result_attempt_id = complete_attempts.result_attempt_id
