with mart_keys as (
    select
        security_code,
        trade_date
    from {{ ref('mart_stock_quotes_daily') }}
),

adjusted_keys as (
    select
        security_code,
        trade_date
    from {{ ref('int_stock_quotes_daily_adj') }}
),

missing_from_adjusted as (
    select
        'missing_from_adjusted' as issue_type,
        security_code,
        trade_date
    from mart_keys
    except distinct
    select
        'missing_from_adjusted' as issue_type,
        security_code,
        trade_date
    from adjusted_keys
),

missing_adjustment_factor as (
    select
        'missing_adjustment_factor' as issue_type,
        security_code,
        trade_date
    from {{ ref('int_stock_quotes_daily_adj') }}
    where forward_adjustment_factor is null
        or forward_adjustment_ratio is null
        or backward_adjustment_factor is null
        or backward_adjustment_ratio is null
)

select *
from missing_from_adjusted

union all

select *
from missing_adjustment_factor
