{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='trade_date',
    partition_by='toYear(trade_date)'
) }}

with source as (
    select
        trade_date,
        three_month_yield_pct,
        six_month_yield_pct,
        one_year_yield_pct,
        two_year_yield_pct,
        three_year_yield_pct,
        five_year_yield_pct,
        seven_year_yield_pct,
        ten_year_yield_pct,
        fifteen_year_yield_pct,
        twenty_year_yield_pct,
        thirty_year_yield_pct
    from {{ ref('int_government_bond_yields_daily') }}
)

select
    trade_date,
    three_month_yield_pct,
    six_month_yield_pct,
    one_year_yield_pct,
    two_year_yield_pct,
    three_year_yield_pct,
    five_year_yield_pct,
    seven_year_yield_pct,
    ten_year_yield_pct,
    fifteen_year_yield_pct,
    twenty_year_yield_pct,
    thirty_year_yield_pct
from source
