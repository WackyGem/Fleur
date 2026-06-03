with source as (
    select
        SECUCODE,
        SECURITY_NAME_ABBR,
        NOTICE_DATE,
        ISSUE_NUM,
        TOTAL_RAISE_FUNDS,
        ISSUE_PRICE,
        EQUITY_RECORD_DATE,
        EX_DIVIDEND_DATEE,
        EVENT_EXPLAIN
    from {{ source('raw', 'eastmoney__dividend_allotment') }}
)

select
    {{ normalize_cn_security_code('SECUCODE', input_format='eastmoney_suffix') }} as security_code,
    SECURITY_NAME_ABBR as security_name_abbr,
    NOTICE_DATE as notice_date,
    EQUITY_RECORD_DATE as equity_record_date,
    EX_DIVIDEND_DATEE as ex_dividend_date,
    ISSUE_NUM as issue_num,
    TOTAL_RAISE_FUNDS as total_raise_funds,
    ISSUE_PRICE as issue_price,
    EVENT_EXPLAIN as event_explain
from source
