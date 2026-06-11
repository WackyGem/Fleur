{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

with stock_quotes as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

price_pairs as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price,
        prev_close_price_unadj is not null
            and prev_close_price is not null
            and prev_close_price_unadj > 0
            and prev_close_price > 0 as has_valid_adjustment_pair
    from stock_quotes
),

adjustment_ratios as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price,
        if(
            has_valid_adjustment_pair,
            prev_close_price_unadj / prev_close_price,
            1.0
        ) as backward_adjustment_ratio,
        if(
            has_valid_adjustment_pair,
            prev_close_price / prev_close_price_unadj,
            1.0
        ) as forward_adjustment_ratio
    from price_pairs
),

adjustment_factors as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price,
        backward_adjustment_ratio,
        exp(
            sum(log(backward_adjustment_ratio)) over (
                partition by security_code
                order by trade_date
                rows between unbounded preceding and current row
            )
        ) as backward_adjustment_factor,
        forward_adjustment_ratio,
        exp(
            coalesce(
                sum(log(forward_adjustment_ratio)) over (
                    partition by security_code
                    order by trade_date
                    rows between 1 following and unbounded following
                ),
                0.0
            )
        ) as forward_adjustment_factor
    from adjustment_ratios
)

select
    security_code,
    trade_date,
    prev_close_price_unadj,
    prev_close_price,
    backward_adjustment_ratio,
    backward_adjustment_factor,
    forward_adjustment_ratio,
    forward_adjustment_factor
from adjustment_factors
