{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(window_key, metric_name, metric_rank, portfolio_run_id, result_attempt_id)'
) }}

with metric_long as (
    select
        portfolio_run_id,
        result_attempt_id,
        window_key,
        window_start,
        window_end,
        tupleElement(metric_tuple, 1) as metric_name,
        tupleElement(metric_tuple, 2) as metric_value,
        tupleElement(metric_tuple, 3) as rank_direction
    from {{ ref('int_portfolio_trade_metric') }}
    array join [
        ('closed_trade_count', toNullable(toFloat64(closed_trade_count)), 'desc'),
        ('win_rate_closed_trades', win_rate_closed_trades, 'desc'),
        ('average_win_return', average_win_return, 'desc'),
        ('average_loss_return', average_loss_return, 'asc'),
        ('profit_loss_ratio', profit_loss_ratio, 'desc'),
        ('average_holding_days', average_holding_days, 'asc'),
        ('largest_win_return', largest_win_return, 'desc'),
        ('largest_loss_return', largest_loss_return, 'asc')
    ] as metric_tuple
)

select
    m.portfolio_run_id,
    m.result_attempt_id,
    m.window_key,
    m.window_start,
    m.window_end,
    m.metric_name,
    m.metric_value,
    m.rank_direction,
    dense_rank() over (
        partition by m.window_key, m.metric_name
        order by
            if(m.rank_direction = 'desc', m.metric_value, -m.metric_value) desc,
            m.portfolio_run_id asc,
            m.result_attempt_id asc
    ) as metric_rank
from metric_long as m
where m.metric_value is not null
