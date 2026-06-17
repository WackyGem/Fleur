{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(benchmark_key, security_code)'
) }}

with benchmark_map as (
    select 'csi_a100' as benchmark_key, '中证A100' as benchmark_name, '000903.SH' as benchmark_security_code
    union all
    select 'csi_300' as benchmark_key, '沪深300' as benchmark_name, '000300.SH' as benchmark_security_code
    union all
    select 'csi_500' as benchmark_key, '中证500' as benchmark_name, '000905.SH' as benchmark_security_code
    union all
    select 'csi_800' as benchmark_key, '中证800' as benchmark_name, '000906.SH' as benchmark_security_code
    union all
    select 'csi_1000' as benchmark_key, '中证1000' as benchmark_name, '000852.SH' as benchmark_security_code
    union all
    select 'cnindex_1000' as benchmark_key, '国证1000' as benchmark_name, '399311.SZ' as benchmark_security_code
),

benchmarks as (
    select
        benchmark_map.benchmark_key,
        benchmark_map.benchmark_name,
        index_basic.security_code,
        index_basic.security_local_code,
        index_basic.exchange_code,
        index_basic.index_name,
        index_basic.listing_status,
        index_basic.is_listed
    from benchmark_map
    inner join {{ ref('int_index_basic_snapshot') }} as index_basic
        on benchmark_map.benchmark_security_code = index_basic.security_code
)

select
    benchmark_key,
    benchmark_name,
    security_code,
    security_local_code,
    exchange_code,
    index_name,
    listing_status,
    is_listed
from benchmarks
