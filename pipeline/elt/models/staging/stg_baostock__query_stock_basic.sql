{{ config(materialized='view') }}

with source as (
    select *
    from {{ source('raw', 'baostock__query_stock_basic') }}
)

select
    code as security_code,
    code_name as security_name,
    ipoDate as ipo_date,
    outDate as out_date,
    type as stock_type,
    case
        when status = 1 then 'listed'
        when status = 0 then 'delisted'
        else 'unknown'
    end as stock_status
from source
