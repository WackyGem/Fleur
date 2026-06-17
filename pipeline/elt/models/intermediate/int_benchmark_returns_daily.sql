{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

select
    benchmarks.security_code,
    quotes.trade_date,
    quotes.close_price,
    quotes.prev_close_price,
    quotes.return_daily
from {{ ref('int_benchmark_basic_snapshot') }} as benchmarks
inner join {{ ref('int_index_quotes_daily') }} as quotes
    on benchmarks.security_code = quotes.security_code
