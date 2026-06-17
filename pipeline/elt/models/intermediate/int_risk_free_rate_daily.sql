{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(source_tenor, trade_date)',
    partition_by='toYear(trade_date)'
) }}

with trade_calendar as (
    select
        trade_date
    from {{ ref('int_trade_calendar') }}
),

risk_free_tenors as (
    select '1y' as source_tenor
),

trade_date_grid as (
    select
        trade_calendar.trade_date,
        risk_free_tenors.source_tenor
    from trade_calendar
    cross join risk_free_tenors
),

risk_free_source as (
    select
        trade_date as source_date,
        '1y' as source_tenor,
        one_year_yield_pct / 100.0 as annual_rate
    from {{ ref('int_government_bond_yields_daily') }}
    where one_year_yield_pct is not null
),

forward_fill_candidates as (
    select
        trade_date_grid.trade_date,
        trade_date_grid.source_tenor,
        risk_free_source.source_date,
        risk_free_source.annual_rate,
        row_number() over (
            partition by trade_date_grid.trade_date, trade_date_grid.source_tenor
            order by risk_free_source.source_date desc
        ) as source_rank
    from trade_date_grid
    left join risk_free_source
        on trade_date_grid.source_tenor = risk_free_source.source_tenor
        and risk_free_source.source_date <= trade_date_grid.trade_date
)

select
    trade_date,
    if(
        annual_rate is null,
        cast(null, 'Nullable(Date)'),
        toNullable(source_date)
    ) as source_date,
    source_tenor,
    annual_rate,
    if(
        annual_rate is null,
        cast(null, 'Nullable(Float64)'),
        pow(1 + annual_rate, 1 / 252.0) - 1
    ) as daily_rate
from forward_fill_candidates
where source_rank = 1
