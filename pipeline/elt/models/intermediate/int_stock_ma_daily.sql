{{ config(materialized='view') }}

select
    security_code,
    trade_date,
    price_ma_3,
    price_ma_5,
    price_ma_6,
    price_ma_10,
    price_ma_12,
    price_ma_14,
    price_ma_20,
    price_ma_24,
    price_ma_28,
    price_ma_57,
    price_ma_60,
    price_ma_114,
    price_ma_250,
    price_avg_ma_3_6_12_24,
    price_avg_ma_14_28_57_114,
    price_ema2_10,
    volume_ma_5,
    volume_ma_10,
    volume_ma_20,
    volume_ma_60
from {{ source('fleur_calculation', 'calc_stock_ma_daily') }}
