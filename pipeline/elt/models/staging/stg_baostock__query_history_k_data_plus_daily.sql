{{ config(materialized='view') }}

with source as (
    select *
    from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
)

select
    date as trade_date,
    code as security_code,
    open,
    high,
    low,
    close,
    preclose,
    volume,
    amount,
    adjustflag,
    turn,
    case
        when tradestatus = 1 then 'trading'
        when tradestatus = 0 then 'suspended'
        else 'unknown'
    end as trading_status,
    pctChg as pct_chg,
    isST as is_st,
    year
from source
