{{ config(materialized='view') }}

select
    security_code,
    trade_date,
    rsv_window,
    k_smoothing,
    d_smoothing,
    rsv,
    k_value,
    d_value,
    j_value
from {{ source('fleur_calculation', 'calc_stock_kdj_daily') }}
