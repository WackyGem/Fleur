{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='trade_date'
) }}

select
    trade_date,
    prev_trade_date
from {{ ref('int_trade_calendar') }}
