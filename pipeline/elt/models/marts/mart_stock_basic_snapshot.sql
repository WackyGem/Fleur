{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='security_code'
) }}

select
    security_code,
    security_name,
    cast(exchange_code, 'LowCardinality(String)') as exchange_code
from {{ ref('int_stock_basic_snapshot') }}
