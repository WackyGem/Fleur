{{ config(materialized='view') }}

with complete_attempts as (
    select
        portfolio_run_id,
        result_attempt_id,
        source_run_id,
        start_date,
        end_date
    from {{ source('fleur_portfolio', 'portfolio_run_snapshot') }}
),

metrics as (
    select
        portfolio_run_id,
        result_attempt_id,
        security_code,
        window_key,
        window_start,
        window_end,
        config_hash,
        metric_status,
        observation_count,
        holding_period_return,
        annualized_return,
        annualized_volatility,
        max_drawdown,
        calmar_ratio,
        downside_deviation,
        sortino_ratio,
        sharpe_ratio,
        information_ratio,
        beta,
        alpha,
        treynor_ratio,
        computed_at
    from {{ source('fleur_calculation', 'calc_portfolio_performance_metric') }}
)

select
    metrics.portfolio_run_id,
    metrics.result_attempt_id,
    complete_attempts.source_run_id,
    metrics.security_code,
    metrics.window_key,
    metrics.window_start,
    metrics.window_end,
    complete_attempts.start_date as run_start_date,
    complete_attempts.end_date as run_end_date,
    metrics.config_hash,
    metrics.metric_status,
    metrics.observation_count,
    metrics.holding_period_return,
    metrics.annualized_return,
    metrics.annualized_volatility,
    metrics.max_drawdown,
    metrics.calmar_ratio,
    metrics.downside_deviation,
    metrics.sortino_ratio,
    metrics.sharpe_ratio,
    metrics.information_ratio,
    metrics.beta,
    metrics.alpha,
    metrics.treynor_ratio,
    metrics.computed_at
from metrics
inner join complete_attempts
    on metrics.portfolio_run_id = complete_attempts.portfolio_run_id
    and metrics.result_attempt_id = complete_attempts.result_attempt_id
