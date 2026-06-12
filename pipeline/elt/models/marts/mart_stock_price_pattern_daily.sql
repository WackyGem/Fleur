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
    n_structure_20_valid_bars,
    n_structure_20_high_date,
    n_structure_20_high_price,
    n_structure_20_low_date,
    n_structure_20_low_price,
    n_structure_20_second_low_date,
    n_structure_20_second_low_price,
    n_structure_20_second_low_ratio,
    n_structure_20_is_valid
from {{ ref('int_stock_price_pattern_daily') }}
