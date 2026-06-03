with source as (
    select
        SECUCODE,
        END_DATE,
        NOTICE_DATE,
        LISTING_DATE,
        CHANGE_REASON,
        CHANGE_REASON_EXPLAIN,
        TOTAL_SHARES,
        LIMITED_SHARES,
        UNLIMITED_SHARES,
        LISTED_A_SHARES,
        LIMITED_A_SHARES
    from {{ source('raw', 'eastmoney__equity_history') }}
)

select
    {{ normalize_cn_security_code('SECUCODE', input_format='eastmoney_suffix') }} as security_code,
    END_DATE as end_date,
    NOTICE_DATE as notice_date,
    LISTING_DATE as listing_date,
    CHANGE_REASON as change_reason,
    CHANGE_REASON_EXPLAIN as change_reason_explain,
    TOTAL_SHARES as total_shares,
    LIMITED_SHARES as limited_shares,
    UNLIMITED_SHARES as unlimited_shares,
    LISTED_A_SHARES as listed_a_shares,
    LIMITED_A_SHARES as limited_a_shares
from source
