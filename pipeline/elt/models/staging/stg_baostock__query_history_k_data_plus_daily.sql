{{ config(materialized='view') }}

with source as (
    select *
    from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
)

select
    date as trade_date,
    code,
    open,
    high,
    low,
    close,
    preclose,
    volume,
    amount,
    adjustflag,
    turn,
    tradestatus,
    pctChg as pct_chg,
    isST as is_st,
    year
from source
