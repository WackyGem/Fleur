{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(config_hash, security_code, window_key, metric_name, metric_rank, portfolio_run_id, result_attempt_id)'
) }}

with metric_long as (
    select
        portfolio_run_id,
        result_attempt_id,
        source_run_id,
        security_code,
        window_key,
        window_start,
        window_end,
        config_hash,
        tupleElement(metric_tuple, 1) as metric_name,
        tupleElement(metric_tuple, 2) as metric_value
    from {{ ref('int_portfolio_performance_metric') }}
    array join [
        ('holding_period_return', holding_period_return),
        ('annualized_return', annualized_return),
        ('annualized_volatility', annualized_volatility),
        ('max_drawdown', max_drawdown),
        ('calmar_ratio', calmar_ratio),
        ('downside_deviation', downside_deviation),
        ('sortino_ratio', sortino_ratio),
        ('sharpe_ratio', sharpe_ratio),
        ('information_ratio', information_ratio),
        ('beta', beta),
        ('alpha', alpha),
        ('treynor_ratio', treynor_ratio)
    ] as metric_tuple
),

rankable as (
    select
        metric_long.portfolio_run_id as portfolio_run_id,
        metric_long.result_attempt_id as result_attempt_id,
        metric_long.source_run_id as source_run_id,
        metric_long.security_code as security_code,
        metric_long.window_key as window_key,
        metric_long.window_start as window_start,
        metric_long.window_end as window_end,
        metric_long.config_hash as config_hash,
        metric_long.metric_name as metric_name,
        metric_long.metric_value as metric_value,
        catalog.rank_direction as rank_direction,
        status.reason_code as reason_code
    from metric_long
    inner join {{ ref('int_portfolio_performance_metric_rank_catalog') }} as catalog
        on metric_long.metric_name = catalog.metric_name
    inner join {{ ref('int_portfolio_performance_metric_status') }} as status
        on metric_long.portfolio_run_id = status.portfolio_run_id
        and metric_long.result_attempt_id = status.result_attempt_id
        and metric_long.security_code = status.security_code
        and metric_long.window_key = status.window_key
        and metric_long.metric_name = status.metric_name
    where catalog.rank_direction != 'none'
        and status.metric_status = 'succeeded'
        and metric_long.metric_value is not null
)

select
    r.portfolio_run_id,
    r.result_attempt_id,
    r.source_run_id,
    r.security_code,
    r.window_key,
    r.window_start,
    r.window_end,
    r.config_hash,
    r.metric_name,
    r.metric_value,
    r.rank_direction,
    dense_rank() over (
        partition by r.config_hash, r.security_code, r.window_key, r.metric_name
        order by
            if(r.rank_direction = 'desc', r.metric_value, -r.metric_value) desc,
            r.portfolio_run_id asc,
            r.result_attempt_id asc
    ) as metric_rank,
    r.reason_code
from rankable as r
