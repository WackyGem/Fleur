with mart_keys as (
    select
        security_code,
        trade_date
    from {{ ref('mart_stock_quotes_daily') }}
),

quote_keys as (
    select
        security_code,
        trade_date
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

missing_from_mart as (
    select
        'missing_from_mart' as issue_type,
        security_code,
        trade_date
    from quote_keys
    except distinct
    select
        'missing_from_mart' as issue_type,
        security_code,
        trade_date
    from mart_keys
),

extra_in_mart as (
    select
        'extra_in_mart' as issue_type,
        security_code,
        trade_date
    from mart_keys
    except distinct
    select
        'extra_in_mart' as issue_type,
        security_code,
        trade_date
    from quote_keys
)

select *
from missing_from_mart

union all

select *
from extra_in_mart
