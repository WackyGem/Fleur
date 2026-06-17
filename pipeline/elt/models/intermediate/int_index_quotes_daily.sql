{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

with index_universe as (
    select security_code
    from {{ ref('int_index_basic_snapshot') }}
),

index_quotes as (
    select
        quotes.security_code,
        quotes.trade_date,
        quotes.open_price,
        quotes.high_price,
        quotes.low_price,
        quotes.close_price,
        quotes.prev_close_price,
        if(
            isNotNull(quotes.close_price)
            and isNotNull(quotes.prev_close_price)
            and quotes.prev_close_price > 0,
            quotes.close_price / quotes.prev_close_price - 1,
            cast(null, 'Nullable(Float64)')
        ) as return_daily,
        quotes.volume,
        quotes.amount,
        quotes.is_suspend
    from {{ ref('stg_baostock__query_history_k_data_plus_daily') }} as quotes
    inner join index_universe
        on quotes.security_code = index_universe.security_code
)

select
    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    prev_close_price,
    return_daily,
    volume,
    amount,
    is_suspend
from index_quotes
