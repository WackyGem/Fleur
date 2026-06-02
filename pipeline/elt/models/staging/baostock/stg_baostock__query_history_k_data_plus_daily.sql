with source as (
    select *
    from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
)

select
    {{ normalize_cn_security_code('code', input_format='baostock_prefix') }} as security_code,
    {{ cn_security_local_code('code', input_format='baostock_prefix') }} as security_local_code,
    {{ cn_exchange_code('code', input_format='baostock_prefix') }} as exchange_code,
    date as trade_date
from source
