{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='trade_date',
    partition_by='toYear(trade_date)'
) }}

with government_bond_yields as (
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
    from {{ ref('stg_chinabond__government_bond') }}
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
from government_bond_yields
