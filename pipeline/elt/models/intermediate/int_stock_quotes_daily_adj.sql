{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, trade_date)',
    partition_by='toYear(trade_date)'
) }}

with stock_quotes_unadj as (
    select
        security_code,
        trade_date,
        open_price,
        high_price,
        low_price,
        close_price,
        prev_close_price
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

adjustment_factors as (
    select
        security_code,
        trade_date,
        backward_adjustment_factor,
        backward_adjustment_ratio,
        forward_adjustment_factor,
        forward_adjustment_ratio
    from {{ ref('int_stock_adjustment_factor') }}
),

stock_quotes_adj as (
    select
        stock_quotes_unadj.security_code,
        stock_quotes_unadj.trade_date,
        stock_quotes_unadj.open_price
            * adjustment_factors.backward_adjustment_factor as open_price_backward_adj,
        stock_quotes_unadj.high_price
            * adjustment_factors.backward_adjustment_factor as high_price_backward_adj,
        stock_quotes_unadj.low_price
            * adjustment_factors.backward_adjustment_factor as low_price_backward_adj,
        stock_quotes_unadj.close_price
            * adjustment_factors.backward_adjustment_factor as close_price_backward_adj,
        stock_quotes_unadj.prev_close_price
            * adjustment_factors.backward_adjustment_factor as prev_close_price_backward_adj,
        stock_quotes_unadj.open_price
            * adjustment_factors.forward_adjustment_factor as open_price_forward_adj,
        stock_quotes_unadj.high_price
            * adjustment_factors.forward_adjustment_factor as high_price_forward_adj,
        stock_quotes_unadj.low_price
            * adjustment_factors.forward_adjustment_factor as low_price_forward_adj,
        stock_quotes_unadj.close_price
            * adjustment_factors.forward_adjustment_factor as close_price_forward_adj,
        stock_quotes_unadj.prev_close_price
            * adjustment_factors.forward_adjustment_factor as prev_close_price_forward_adj,
        adjustment_factors.backward_adjustment_factor,
        adjustment_factors.backward_adjustment_ratio,
        adjustment_factors.forward_adjustment_factor,
        adjustment_factors.forward_adjustment_ratio
    from stock_quotes_unadj
    inner join adjustment_factors
        on stock_quotes_unadj.security_code = adjustment_factors.security_code
        and stock_quotes_unadj.trade_date = adjustment_factors.trade_date
)

select
    security_code,
    trade_date,
    open_price_backward_adj,
    high_price_backward_adj,
    low_price_backward_adj,
    close_price_backward_adj,
    prev_close_price_backward_adj,
    open_price_forward_adj,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj,
    prev_close_price_forward_adj,
    backward_adjustment_factor,
    backward_adjustment_ratio,
    forward_adjustment_factor,
    forward_adjustment_ratio
from stock_quotes_adj
