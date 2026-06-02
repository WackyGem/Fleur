with source as (
    select *
    from {{ source('raw', 'sina__trade_calendar') }}
)

select
    trade_date as trade_date
from source
