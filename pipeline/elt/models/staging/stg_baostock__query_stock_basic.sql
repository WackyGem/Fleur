{{ config(materialized='view') }}

with source as (
    select *
    from {{ source('raw', 'baostock__query_stock_basic') }}
)

select
    code,
    code_name,
    ipoDate as ipo_date,
    outDate as out_date,
    type as stock_type,
    status as stock_status
from source
