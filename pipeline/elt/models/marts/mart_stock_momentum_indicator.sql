{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

with rsi as (
    select
        security_code,
        trade_date,
        rsi_6,
        rsi_12,
        rsi_14,
        rsi_24,
        rsi_25,
        rsi_50
    from {{ ref('int_stock_rsi_daily') }}
),

kdj as (
    select
        security_code,
        trade_date,
        rsv_window as kdj_rsv_window,
        k_smoothing as kdj_k_smoothing,
        d_smoothing as kdj_d_smoothing,
        rsv as kdj_rsv,
        k_value as kdj_k_value,
        d_value as kdj_d_value,
        j_value as kdj_j_value
    from {{ ref('int_stock_kdj_daily') }}
)

select
    rsi.security_code as security_code,
    rsi.trade_date as trade_date,
    rsi.rsi_6 as rsi_6,
    rsi.rsi_12 as rsi_12,
    rsi.rsi_14 as rsi_14,
    rsi.rsi_24 as rsi_24,
    rsi.rsi_25 as rsi_25,
    rsi.rsi_50 as rsi_50,
    kdj.kdj_rsv_window as kdj_rsv_window,
    kdj.kdj_k_smoothing as kdj_k_smoothing,
    kdj.kdj_d_smoothing as kdj_d_smoothing,
    kdj.kdj_rsv as kdj_rsv,
    kdj.kdj_k_value as kdj_k_value,
    kdj.kdj_d_value as kdj_d_value,
    kdj.kdj_j_value as kdj_j_value
from rsi
left join kdj
    on rsi.security_code = kdj.security_code
    and rsi.trade_date = kdj.trade_date
