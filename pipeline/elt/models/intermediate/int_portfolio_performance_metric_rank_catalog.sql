{{ config(materialized='ephemeral') }}

select
    tupleElement(metric_tuple, 1) as metric_name,
    tupleElement(metric_tuple, 2) as rank_direction,
    'exclude' as null_policy
from (
    select arrayJoin([
        ('holding_period_return', 'desc'),
        ('annualized_return', 'desc'),
        ('annualized_volatility', 'asc'),
        ('max_drawdown', 'asc'),
        ('calmar_ratio', 'desc'),
        ('downside_deviation', 'asc'),
        ('sortino_ratio', 'desc'),
        ('sharpe_ratio', 'desc'),
        ('information_ratio', 'desc'),
        ('beta', 'none'),
        ('alpha', 'desc'),
        ('treynor_ratio', 'desc')
    ]) as metric_tuple
)
