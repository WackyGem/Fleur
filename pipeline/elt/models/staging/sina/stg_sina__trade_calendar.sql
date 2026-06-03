with source as (
    select
        trade_date
    from {{ source('raw', 'sina__trade_calendar') }}
)

select
    trade_date
from source
