{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, trade_date)',
    partition_by='toYear(trade_date)'
) }}

select
    security_code,
    trade_date,
    return_daily
from {{ ref('int_benchmark_returns_daily') }}
