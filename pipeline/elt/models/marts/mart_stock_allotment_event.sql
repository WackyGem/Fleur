{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, notice_date, event_explain)',
    partition_by='toYear(notice_date)'
) }}

with source as (
    select
        security_code,
        security_name_abbr,
        notice_date,
        equity_record_date,
        ex_dividend_date,
        issue_num,
        total_raise_funds,
        issue_price,
        event_explain
    from {{ ref('int_stock_allotment_event') }}
)

select
    security_code,
    security_name_abbr,
    notice_date,
    equity_record_date,
    ex_dividend_date,
    issue_num,
    total_raise_funds,
    issue_price,
    event_explain
from source
