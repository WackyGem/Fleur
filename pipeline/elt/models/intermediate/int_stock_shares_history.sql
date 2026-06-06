{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, effective_date)'
) }}

with equity_history as (
    select
        security_code,
        report_date as end_date,
        total_shares,
        unlimited_shares,
        listed_a_shares,
        limited_a_shares
    from {{ ref('stg_eastmoney__equity_history') }}
),

freeholders_deduplicated as (
    select
        security_code,
        end_date,
        holder_eastmoney_code,
        holder_name,
        shares_type,
        max(free_float_hold_shares) as free_float_hold_shares,
        max(free_float_holdnum_ratio_pct) as free_float_holdnum_ratio_pct
    from {{ ref('stg_eastmoney__freeholders') }}
    where shares_type = 'A股'
    group by
        security_code,
        end_date,
        holder_eastmoney_code,
        holder_name,
        shares_type
),

freeholders_report_aggregates as (
    select
        security_code,
        end_date,
        sumIf(free_float_hold_shares, free_float_holdnum_ratio_pct > 5) as major_holder_a_float_shares,
        countIf(free_float_holdnum_ratio_pct > 5) as major_holder_count
    from freeholders_deduplicated
    group by
        security_code,
        end_date
),

change_points as (
    select
        security_code,
        end_date as effective_date
    from equity_history

    union distinct

    select
        security_code,
        end_date as effective_date
    from freeholders_report_aggregates
),

equity_as_of as (
    select
        change_points.security_code,
        change_points.effective_date,
        equity_history.end_date as source_equity_end_date,
        equity_history.total_shares,
        equity_history.unlimited_shares as float_shares,
        equity_history.listed_a_shares as a_float_shares,
        equity_history.limited_a_shares
    from change_points
    asof inner join equity_history
        on change_points.security_code = equity_history.security_code
        and change_points.effective_date >= equity_history.end_date
),

freeholders_as_of_source as (
    select
        security_code,
        end_date,
        toNullable(end_date) as source_freeholders_end_date,
        toNullable(major_holder_a_float_shares) as major_holder_a_float_shares,
        toNullable(major_holder_count) as major_holder_count
    from freeholders_report_aggregates
),

shares_with_next_effective_date as (
    select
        equity_as_of.security_code,
        equity_as_of.effective_date,
        leadInFrame(
            toNullable(equity_as_of.effective_date),
            1,
            cast(null, 'Nullable(Date)')
        ) over (
            partition by equity_as_of.security_code
            order by equity_as_of.effective_date
            rows between unbounded preceding and unbounded following
        ) as next_effective_date,
        equity_as_of.source_equity_end_date,
        freeholders_as_of_source.source_freeholders_end_date,
        equity_as_of.total_shares,
        equity_as_of.float_shares,
        if(
            equity_as_of.a_float_shares is null and equity_as_of.limited_a_shares is null,
            cast(null, 'Nullable(Float64)'),
            coalesce(equity_as_of.a_float_shares, 0) + coalesce(equity_as_of.limited_a_shares, 0)
        ) as a_shares,
        equity_as_of.a_float_shares,
        freeholders_as_of_source.major_holder_a_float_shares as major_holder_a_float_shares,
        freeholders_as_of_source.major_holder_count as major_holder_count
    from equity_as_of
    asof left join freeholders_as_of_source
        on equity_as_of.security_code = freeholders_as_of_source.security_code
        and equity_as_of.effective_date >= freeholders_as_of_source.end_date
)

select
    security_code,
    effective_date,
    if(
        next_effective_date is null,
        cast(null, 'Nullable(Date)'),
        next_effective_date - 1
    ) as expiry_date,
    source_equity_end_date,
    source_freeholders_end_date,
    total_shares,
    float_shares,
    a_shares,
    a_float_shares,
    if(
        a_float_shares is null,
        cast(null, 'Nullable(Float64)'),
        greatest(0, a_float_shares - coalesce(major_holder_a_float_shares, 0))
    ) as a_free_float_shares,
    coalesce(major_holder_a_float_shares, 0) as major_holder_a_float_shares,
    coalesce(major_holder_count, 0) as major_holder_count
from shares_with_next_effective_date
