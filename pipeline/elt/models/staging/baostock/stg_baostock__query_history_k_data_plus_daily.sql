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
        tradestatus,
        isST
    from {{ source('raw', 'baostock__query_history_k_data_plus_daily_compacted') }}
)

select
    {{ normalize_cn_security_code('code', input_format='baostock_prefix') }} as security_code,
    date as trade_date,
    open as open_price,
    high as high_price,
    low as low_price,
    close as close_price,
    preclose as prev_close_price,
    volume as volume,
    amount as amount,
    cast(tradestatus = 0, 'Bool') as is_suspend,
    isST as is_st
from source
where adjustflag = 3
