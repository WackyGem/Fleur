{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, notice_date, dividend_plan_record_key)',
    partition_by='toYear(notice_date)'
) }}

with source as (
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
    from {{ ref('int_stock_dividend_plan') }}
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
from source
