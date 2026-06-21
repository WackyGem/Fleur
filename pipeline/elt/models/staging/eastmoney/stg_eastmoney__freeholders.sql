with source as (
    select
        SECUCODE,
        END_DATE,
        HOLDER_RANK,
        HOLDER_NEW,
        HOLDER_NAME,
        HOLDER_TYPE,
        SHARES_TYPE,
        HOLD_NUM,
        FREE_HOLDNUM_RATIO,
        HOLD_NUM_CHANGE,
        CHANGE_RATIO
    from {{ source('raw', 'eastmoney__freeholders') }}
)

select
    {{ normalize_cn_security_code('SECUCODE', input_format='eastmoney_suffix') }} as security_code,
    END_DATE as report_date,
    HOLDER_RANK as holder_rank,
    HOLDER_NEW as holder_eastmoney_code,
    HOLDER_NAME as holder_name,
    HOLDER_TYPE as holder_type,
    SHARES_TYPE as shares_type,
    HOLD_NUM as free_float_hold_shares,
    FREE_HOLDNUM_RATIO as free_float_holdnum_ratio_pct,
    HOLD_NUM_CHANGE as hold_num_change_text,
    CHANGE_RATIO as change_ratio_pct
from source
