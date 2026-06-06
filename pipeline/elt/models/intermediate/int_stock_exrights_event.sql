{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, ex_dividend_date)',
    partition_by='toYear(ex_dividend_date)'
) }}

with dividend_main_source as (
    select
        security_code,
        ex_dividend_date,
        equity_record_date,
        notice_date,
        report_date,
        toNullable(report_period_label) as report_period_label,
        coalesce(
            nullIf(new_profile, ''),
            nullIf(impl_plan_profile, ''),
            nullIf(impl_plan_newprofile, '')
        ) as plan_text
    from {{ ref('stg_eastmoney__dividend_main') }}
    where assign_progress = '实施方案'
      and is_unassign = false
      and ex_dividend_date is not null
),

dividend_main_filtered as (
    select
        security_code,
        cast(ex_dividend_date, 'Date') as ex_dividend_date,
        equity_record_date,
        notice_date,
        report_date,
        report_period_label,
        plan_text
    from dividend_main_source
),

dividend_main_components as (
    select
        security_code,
        ex_dividend_date,
        min(equity_record_date) as equity_record_date,
        max(notice_date) as notice_date,
        max(report_date) as report_date,
        anyLast(report_period_label) as report_period_label,
        max(
            coalesce(
                toFloat64OrNull(
                    regexpExtract(
                        coalesce(plan_text, ''),
                        '10派([0-9]+(?:\\.[0-9]+)?)元',
                        1
                    )
                ) / 10,
                0
            )
        ) as cash_dividend_per_share,
        max(
            coalesce(
                toFloat64OrNull(
                    regexpExtract(
                        coalesce(plan_text, ''),
                        '10送([0-9]+(?:\\.[0-9]+)?)(?:股)?',
                        1
                    )
                ) / 10,
                0
            )
        ) as bonus_share_per_share,
        max(
            coalesce(
                toFloat64OrNull(
                    regexpExtract(
                        coalesce(plan_text, ''),
                        '10转([0-9]+(?:\\.[0-9]+)?)(?:股)?',
                        1
                    )
                ) / 10,
                0
            )
        ) as transfer_share_per_share,
        cast(0, 'Float64') as allotment_share_per_share,
        cast(null, 'Nullable(Float64)') as allotment_price_yuan,
        true as source_has_dividend_main,
        false as source_has_allotment,
        anyLast(plan_text) as source_plan_text,
        cast(null, 'Nullable(String)') as source_allotment_text
    from dividend_main_filtered
    group by
        security_code,
        ex_dividend_date
),

allotment_components as (
    select
        security_code,
        ex_dividend_date,
        min(equity_record_date) as equity_record_date,
        max(notice_date) as notice_date,
        cast(null, 'Nullable(Date)') as report_date,
        cast(null, 'Nullable(String)') as report_period_label,
        cast(0, 'Float64') as cash_dividend_per_share,
        cast(0, 'Float64') as bonus_share_per_share,
        cast(0, 'Float64') as transfer_share_per_share,
        max(
            coalesce(
                toFloat64OrNull(
                    regexpExtract(
                        coalesce(event_explain, ''),
                        '每?10股配([0-9]+(?:\\.[0-9]+)?)股',
                        1
                    )
                ) / 10,
                0
            )
        ) as allotment_share_per_share,
        max(issue_price) as allotment_price_yuan,
        false as source_has_dividend_main,
        true as source_has_allotment,
        cast(null, 'Nullable(String)') as source_plan_text,
        anyLast(toNullable(event_explain)) as source_allotment_text
    from {{ ref('stg_eastmoney__dividend_allotment') }}
    where ex_dividend_date is not null
    group by
        security_code,
        ex_dividend_date
),

source_components as (
    select
        security_code,
        ex_dividend_date,
        equity_record_date,
        notice_date,
        report_date,
        report_period_label,
        cash_dividend_per_share,
        bonus_share_per_share,
        transfer_share_per_share,
        allotment_share_per_share,
        allotment_price_yuan,
        source_has_dividend_main,
        source_has_allotment,
        source_plan_text,
        source_allotment_text
    from dividend_main_components

    union all

    select
        security_code,
        ex_dividend_date,
        equity_record_date,
        notice_date,
        report_date,
        report_period_label,
        cash_dividend_per_share,
        bonus_share_per_share,
        transfer_share_per_share,
        allotment_share_per_share,
        allotment_price_yuan,
        source_has_dividend_main,
        source_has_allotment,
        source_plan_text,
        source_allotment_text
    from allotment_components
),

event_components as (
    select
        security_code,
        ex_dividend_date,
        min(equity_record_date) as equity_record_date,
        max(notice_date) as notice_date,
        max(report_date) as report_date,
        anyIf(report_period_label, report_period_label is not null) as report_period_label,
        max(cash_dividend_per_share) as cash_dividend_per_share,
        max(bonus_share_per_share) as bonus_share_per_share,
        max(transfer_share_per_share) as transfer_share_per_share,
        max(allotment_share_per_share) as allotment_share_per_share,
        max(allotment_price_yuan) as allotment_price_yuan,
        max(source_has_dividend_main) as source_has_dividend_main,
        max(source_has_allotment) as source_has_allotment,
        anyIf(source_plan_text, source_plan_text is not null and source_plan_text != '')
            as source_plan_text,
        anyIf(
            source_allotment_text,
            source_allotment_text is not null and source_allotment_text != ''
        ) as source_allotment_text
    from source_components
    group by
        security_code,
        ex_dividend_date
),

events_with_flags as (
    select
        security_code,
        ex_dividend_date,
        equity_record_date,
        notice_date,
        report_date,
        report_period_label,
        cash_dividend_per_share,
        bonus_share_per_share,
        transfer_share_per_share,
        allotment_share_per_share,
        allotment_price_yuan,
        cash_dividend_per_share > 0 as has_cash_dividend,
        bonus_share_per_share > 0
            or transfer_share_per_share > 0
            or allotment_share_per_share > 0 as has_share_right,
        source_has_dividend_main,
        source_has_allotment,
        source_plan_text,
        source_allotment_text
    from event_components
)

select
    security_code,
    ex_dividend_date,
    equity_record_date,
    notice_date,
    report_date,
    report_period_label,
    cash_dividend_per_share,
    bonus_share_per_share,
    transfer_share_per_share,
    allotment_share_per_share,
    allotment_price_yuan,
    multiIf(
        has_cash_dividend and has_share_right,
        'DR',
        has_cash_dividend,
        'XD',
        has_share_right,
        'XR',
        cast(null, 'Nullable(String)')
    ) as event_tag,
    has_cash_dividend,
    has_share_right,
    source_has_dividend_main,
    source_has_allotment,
    source_plan_text,
    source_allotment_text
from events_with_flags
where has_cash_dividend or has_share_right
