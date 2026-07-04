{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, notice_date, dividend_plan_record_key)',
    partition_by='toYear(notice_date)'
) }}

with normalized_rows as (
    select distinct
        security_code,
        security_name_abbr,
        notice_date,
        report_period_label,
        report_date,
        assign_progress,
        is_unassign,
        impl_plan_profile,
        impl_plan_newprofile,
        new_profile,
        assign_object,
        equity_record_date,
        ex_dividend_date,
        pay_cash_date,
        gmdecision_notice_date,
        annual_general_meeting_date,
        announcement_identifier,
        total_dividend,
        total_dividend_a
    from {{ ref('stg_eastmoney__dividend_main') }}
),

keyed as (
    select
        hex(SHA256(concat('security_code=', ifNull(toString(security_code), '<NULL>'), '|', 'security_name_abbr=', ifNull(toString(security_name_abbr), '<NULL>'), '|', 'notice_date=', ifNull(toString(notice_date), '<NULL>'), '|', 'report_period_label=', ifNull(toString(report_period_label), '<NULL>'), '|', 'report_date=', ifNull(toString(report_date), '<NULL>'), '|', 'assign_progress=', ifNull(toString(assign_progress), '<NULL>'), '|', 'is_unassign=', ifNull(toString(is_unassign), '<NULL>'), '|', 'impl_plan_profile=', ifNull(toString(impl_plan_profile), '<NULL>'), '|', 'impl_plan_newprofile=', ifNull(toString(impl_plan_newprofile), '<NULL>'), '|', 'new_profile=', ifNull(toString(new_profile), '<NULL>'), '|', 'assign_object=', ifNull(toString(assign_object), '<NULL>'), '|', 'equity_record_date=', ifNull(toString(equity_record_date), '<NULL>'), '|', 'ex_dividend_date=', ifNull(toString(ex_dividend_date), '<NULL>'), '|', 'pay_cash_date=', ifNull(toString(pay_cash_date), '<NULL>'), '|', 'gmdecision_notice_date=', ifNull(toString(gmdecision_notice_date), '<NULL>'), '|', 'annual_general_meeting_date=', ifNull(toString(annual_general_meeting_date), '<NULL>'), '|', 'announcement_identifier=', ifNull(toString(announcement_identifier), '<NULL>'), '|', 'total_dividend=', ifNull(toString(total_dividend), '<NULL>'), '|', 'total_dividend_a=', ifNull(toString(total_dividend_a), '<NULL>')))) as dividend_plan_record_key,
        hex(SHA256(concat('security_code=', ifNull(toString(security_code), '<NULL>'), '|', 'report_period_label=', ifNull(toString(report_period_label), '<NULL>')))) as dividend_plan_group_key,
        security_code,
        security_name_abbr,
        notice_date,
        report_period_label,
        report_date,
        assign_progress,
        is_unassign,
        impl_plan_profile,
        impl_plan_newprofile,
        new_profile,
        assign_object,
        equity_record_date,
        ex_dividend_date,
        pay_cash_date,
        gmdecision_notice_date,
        annual_general_meeting_date,
        announcement_identifier,
        total_dividend,
        total_dividend_a
    from normalized_rows
)

select
    dividend_plan_record_key,
    dividend_plan_group_key,
    security_code,
    security_name_abbr,
    notice_date,
    report_period_label,
    report_date,
    assign_progress,
    is_unassign,
    impl_plan_profile,
    impl_plan_newprofile,
    new_profile,
    assign_object,
    equity_record_date,
    ex_dividend_date,
    pay_cash_date,
    gmdecision_notice_date,
    annual_general_meeting_date,
    announcement_identifier,
    total_dividend,
    total_dividend_a
from keyed
