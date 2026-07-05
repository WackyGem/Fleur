{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

select
    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    prev_close_price,
    return_daily,
    volume,
    amount
from {{ ref('int_index_quotes_daily') }}
