{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='security_code'
) }}

with source as (
    select
        security_code,
        security_local_code,
        exchange_code,
        security_name,
        ipo_date,
        out_date,
        listing_status_code,
        listing_status,
        is_listed,
        security_type_code,
        security_type
    from {{ ref('stg_baostock__query_stock_basic') }}
    where security_type = 'index'
)

select
    security_code,
    security_local_code,
    exchange_code,
    security_name as index_name,
    ipo_date,
    out_date,
    listing_status_code,
    listing_status,
    is_listed,
    security_type_code,
    security_type
from source
