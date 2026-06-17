{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code)'
) }}

with benchmark_universe as (
    select '000903.SH' as security_code
    union all
    select '000300.SH'
    union all
    select '000905.SH'
    union all
    select '000906.SH'
    union all
    select '000852.SH'
    union all
    select '399311.SZ'
)

select
    index_basic.security_code,
    index_basic.security_local_code,
    index_basic.exchange_code,
    index_basic.index_name,
    index_basic.listing_status,
    index_basic.is_listed
from {{ ref('int_index_basic_snapshot') }} as index_basic
inner join benchmark_universe
    on index_basic.security_code = benchmark_universe.security_code
