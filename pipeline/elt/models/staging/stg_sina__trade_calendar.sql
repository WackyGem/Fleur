{{ config(materialized='view') }}

select
    trade_date
from {{ source('raw', 'sina__trade_calendar') }}
