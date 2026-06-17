with staging as (
    select
        trade_date
    from {{ ref('stg_chinabond__government_bond') }}
),

intermediate as (
    select
        trade_date
    from {{ ref('int_government_bond_yields_daily') }}
),

missing_from_intermediate as (
    select
        'missing_from_intermediate' as issue,
        staging.trade_date
    from staging
    left join intermediate
        on staging.trade_date = intermediate.trade_date
    where intermediate.trade_date is null
),

extra_in_intermediate as (
    select
        'extra_in_intermediate' as issue,
        intermediate.trade_date
    from intermediate
    left join staging
        on intermediate.trade_date = staging.trade_date
    where staging.trade_date is null
)

select
    issue,
    trade_date
from missing_from_intermediate

union all

select
    issue,
    trade_date
from extra_in_intermediate
