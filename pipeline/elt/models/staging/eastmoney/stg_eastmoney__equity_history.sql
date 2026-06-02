with source as (
    select *
    from {{ source('raw', 'eastmoney__equity_history') }}
)

select
    {{ normalize_cn_security_code('SECUCODE', input_format='eastmoney_suffix') }} as security_code,
    {{ cn_security_local_code('SECUCODE', input_format='eastmoney_suffix') }} as security_local_code,
    {{ cn_exchange_code('SECUCODE', input_format='eastmoney_suffix') }} as exchange_code,
    END_DATE as report_date
from source
