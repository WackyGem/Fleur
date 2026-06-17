{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(source_tenor, trade_date)',
    partition_by='toYear(trade_date)'
) }}

select
    trade_date,
    source_date,
    source_tenor,
    annual_rate,
    daily_rate
from {{ ref('int_risk_free_rate_daily') }}
