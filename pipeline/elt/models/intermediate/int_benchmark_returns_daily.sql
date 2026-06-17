{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, benchmark_key, security_code)',
    partition_by='toYear(trade_date)'
) }}

with benchmarks as (
    select
        benchmark_key,
        benchmark_name,
        security_code as benchmark_security_code
    from {{ ref('int_benchmark_basic_snapshot') }}
),

benchmark_returns as (
    select
        benchmarks.benchmark_key,
        benchmarks.benchmark_name,
        quotes.security_code,
        quotes.trade_date,
        quotes.close_price,
        quotes.prev_close_price,
        quotes.return_daily
    from benchmarks
    inner join {{ ref('int_index_quotes_daily') }} as quotes
        on benchmarks.benchmark_security_code = quotes.security_code
)

select
    benchmark_key,
    benchmark_name,
    security_code,
    trade_date,
    close_price,
    prev_close_price,
    return_daily
from benchmark_returns
