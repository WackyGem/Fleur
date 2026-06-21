{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='trade_date'
) }}

with trade_calendar as (
    select
        trade_date
    from {{ ref('stg_sina__trade_calendar') }}
),

with_previous_trade_date as (
    select
        trade_date,
        lagInFrame(
            toNullable(trade_date),
            1,
            cast(null, 'Nullable(Date)')
        ) over (
            order by trade_date
            rows between unbounded preceding and unbounded following
        ) as prev_trade_date
    from trade_calendar
)

select
    trade_date,
    prev_trade_date
from with_previous_trade_date
