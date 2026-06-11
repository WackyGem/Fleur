{{ config(materialized='view') }}

select
    security_code,
    trade_date,
    macd_dif,
    macd_dea,
    macd_histogram
from {{ source('fleur_calculation', 'calc_stock_macd_daily') }}
