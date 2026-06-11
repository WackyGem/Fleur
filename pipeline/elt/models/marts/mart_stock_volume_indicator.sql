{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, trade_date)',
    partition_by='toYear(trade_date)'
) }}

select
    security_code,
    trade_date,
    volume_ma_5,
    volume_ma_10,
    volume_ma_20,
    volume_ma_60
from {{ ref('int_stock_ma_daily') }}
