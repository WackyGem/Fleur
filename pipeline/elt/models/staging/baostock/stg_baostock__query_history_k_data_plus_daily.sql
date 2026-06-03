with source as (
    select
        date,
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
        pctChg,
        isST
    from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
)

select
    {{ normalize_cn_security_code('code', input_format='baostock_prefix') }} as security_code,
    date as trade_date,
    open as open_price,
    high as high_price,
    low as low_price,
    close as close_price,
    preclose as previous_close_price,
    volume as volume,
    amount as amount,
    adjustflag as adjust_flag,
    turn as turnover_rate,
    tradestatus as trade_status,
    isST as is_st,
    pctChg as pct_change
from source
