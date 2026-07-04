{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, report_date, holder_rank, holder_identifier)',
    partition_by='toYear(report_date)'
) }}

with source as (
    select
        security_code,
        report_date,
        holder_rank,
        holder_identifier,
        holder_name,
        holder_type,
        shares_type,
        free_float_hold_shares,
        free_float_holdnum_ratio_pct,
        hold_num_change_text,
        change_ratio_pct
    from {{ ref('int_stock_free_float_shareholder_top10') }}
)

select
    security_code,
    report_date,
    holder_rank,
    holder_identifier,
    holder_name,
    holder_type,
    shares_type,
    free_float_hold_shares,
    free_float_holdnum_ratio_pct,
    hold_num_change_text,
    change_ratio_pct
from source
