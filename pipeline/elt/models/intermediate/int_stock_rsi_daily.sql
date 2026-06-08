{{ config(materialized='view') }}

select
    security_code,
    trade_date,
    rsi_6,
    rsi_12,
    rsi_14,
    rsi_24,
    rsi_25,
    rsi_50
from {{ source('fleur_calculation', 'calc_stock_rsi_daily') }}
