{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, assumeNotNull(security_code))',
    partition_by='toYear(trade_date)'
) }}

with source as (
    select
        trade_date,
        security_code,
        security_name,
        first_limit_up_time,
        last_limit_up_time,
        open_num,
        limit_up_type,
        order_volume,
        order_amount,
        is_new,
        is_again_limit,
        limit_up_success_rate,
        currency_value,
        market_id,
        market_type,
        change_rate,
        turnover_rate,
        reason_type,
        high_days,
        high_days_value_raw,
        change_tag,
        latest_price
    from {{ ref('stg_ths__limit_up_pool_compacted') }}
)

select
    trade_date,
    security_code,
    security_name,
    first_limit_up_time,
    last_limit_up_time,
    open_num,
    limit_up_type,
    order_volume,
    order_amount,
    is_new,
    is_again_limit,
    limit_up_success_rate,
    currency_value,
    market_id,
    market_type,
    change_rate,
    turnover_rate,
    reason_type,
    high_days,
    high_days_value_raw,
    change_tag,
    latest_price
from source
