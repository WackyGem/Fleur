{{ config(materialized='view') }}

select
    security_code,
    trade_date,
    boll_mid_10_1p5,
    boll_up_10_1p5,
    boll_dn_10_1p5,
    boll_mid_20_2,
    boll_up_20_2,
    boll_dn_20_2,
    boll_mid_50_2p5,
    boll_up_50_2p5,
    boll_dn_50_2p5
from {{ source('fleur_calculation', 'calc_stock_boll_daily') }}
