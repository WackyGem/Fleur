{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

select
    security_code,
    trade_date,
    close_direction,
    close_up_streak_days,
    close_down_streak_days,
    n_structure_20_is_valid,
    n_structure_20_stage,
    n_structure_20_higher_low_ratio,
    n_structure_20_pullback_depth,
    n_structure_20_rebound_ratio
from {{ ref('int_stock_price_pattern_daily') }}
