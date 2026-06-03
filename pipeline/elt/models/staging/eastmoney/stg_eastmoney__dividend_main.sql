with source as (
    select
        SECUCODE,
        SECURITY_NAME_ABBR,
        NOTICE_DATE,
        IMPL_PLAN_PROFILE,
        ASSIGN_PROGRESS,
        EQUITY_RECORD_DATE,
        EX_DIVIDEND_DATE,
        PAY_CASH_DATE,
        IS_UNASSIGN,
        REPORT_DATE,
        ASSIGN_OBJECT,
        IMPL_PLAN_NEWPROFILE,
        NEW_PROFILE,
        GMDECISION_NOTICE_DATE,
        INFO_CODE,
        DAT_YAGGR,
        TOTAL_DIVIDEND,
        TOTAL_DIVIDEND_A,
        REPORT_TIME
    from {{ source('raw', 'eastmoney__dividend_main') }}
)

select
    {{ normalize_cn_security_code('SECUCODE', input_format='eastmoney_suffix') }} as security_code,
    SECURITY_NAME_ABBR as security_name_abbr,
    NOTICE_DATE as notice_date,
    REPORT_DATE as report_period_label,
    REPORT_TIME as report_time,
    ASSIGN_PROGRESS as assign_progress,
    IS_UNASSIGN as is_unassign,
    IMPL_PLAN_PROFILE as impl_plan_profile,
    IMPL_PLAN_NEWPROFILE as impl_plan_newprofile,
    NEW_PROFILE as new_profile,
    ASSIGN_OBJECT as assign_object,
    EQUITY_RECORD_DATE as equity_record_date,
    EX_DIVIDEND_DATE as ex_dividend_date,
    PAY_CASH_DATE as pay_cash_date,
    GMDECISION_NOTICE_DATE as gmdecision_notice_date,
    DAT_YAGGR as annual_general_meeting_date,
    INFO_CODE as info_code,
    TOTAL_DIVIDEND as total_dividend,
    TOTAL_DIVIDEND_A as total_dividend_a
from source
