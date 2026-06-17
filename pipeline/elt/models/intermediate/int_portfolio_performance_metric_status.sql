{{ config(materialized='view') }}

with complete_attempts as (
    select
        portfolio_run_id,
        result_attempt_id
    from {{ source('fleur_portfolio', 'portfolio_run_snapshot') }}
),

statuses as (
    select
        portfolio_run_id,
        result_attempt_id,
        security_code,
        window_key,
        metric_name,
        metric_status,
        reason_code,
        computed_at
    from {{ source('fleur_calculation', 'calc_portfolio_performance_metric_status') }}
)

select
    statuses.portfolio_run_id,
    statuses.result_attempt_id,
    statuses.security_code,
    statuses.window_key,
    statuses.metric_name,
    statuses.metric_status,
    statuses.reason_code,
    statuses.computed_at
from statuses
inner join complete_attempts
    on statuses.portfolio_run_id = complete_attempts.portfolio_run_id
    and statuses.result_attempt_id = complete_attempts.result_attempt_id
