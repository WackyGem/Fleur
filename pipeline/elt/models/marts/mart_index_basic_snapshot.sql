{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='security_code'
) }}

select
    security_code,
    security_local_code,
    index_name,
    cast(exchange_code, 'LowCardinality(String)') as exchange_code,
    cast(listing_status, 'LowCardinality(String)') as listing_status,
    is_listed
from {{ ref('int_index_basic_snapshot') }}
