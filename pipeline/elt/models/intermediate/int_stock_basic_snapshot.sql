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
        security_type_code,
        security_type,
        security_board,
        listing_status_code,
        listing_status,
        is_listed
    from {{ ref('stg_baostock__query_stock_basic') }}
    where security_type = 'stock'
)

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
    security_type,
    security_board
from source
