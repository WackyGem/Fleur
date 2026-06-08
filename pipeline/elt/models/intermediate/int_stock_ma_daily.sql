{{ config(materialized='view') }}

select
    security_code,
    trade_date,
    ma_3,
    ma_5,
    ma_6,
    ma_10,
    ma_12,
    ma_14,
    ma_20,
    ma_24,
    ma_28,
    ma_57,
    ma_60,
    ma_114,
    ma_250,
    avg_ma_3_6_12_24,
    avg_ma_14_28_57_114,
    ema2_10
from {{ source('fleur_calculation', 'calc_stock_ma_daily') }}
