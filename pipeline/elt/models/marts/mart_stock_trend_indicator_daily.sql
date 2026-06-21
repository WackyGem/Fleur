{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

with ma as (
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
        price_ma_30,
        price_ma_57,
        price_ma_60,
        price_ma_114,
        price_ma_250,
        price_avg_ma_3_6_12_24,
        price_avg_ma_14_28_57_114,
        price_ema2_10
    from {{ ref('int_stock_ma_daily') }}
),

boll as (
    select
        security_code,
        trade_date,
        boll_mid_10_1p5,
        boll_upper_10_1p5,
        boll_lower_10_1p5,
        boll_mid_20_2,
        boll_upper_20_2,
        boll_lower_20_2,
        boll_mid_50_2p5,
        boll_upper_50_2p5,
        boll_lower_50_2p5
    from {{ ref('int_stock_boll_daily') }}
),

macd as (
    select
        security_code,
        trade_date,
        macd_dif,
        macd_dea,
        macd_histogram
    from {{ ref('int_stock_macd_daily') }}
)

select
    ma.security_code as security_code,
    ma.trade_date as trade_date,
    ma.price_ma_3 as price_ma_3,
    ma.price_ma_5 as price_ma_5,
    ma.price_ma_6 as price_ma_6,
    ma.price_ma_10 as price_ma_10,
    ma.price_ma_12 as price_ma_12,
    ma.price_ma_14 as price_ma_14,
    ma.price_ma_20 as price_ma_20,
    ma.price_ma_24 as price_ma_24,
    ma.price_ma_28 as price_ma_28,
    ma.price_ma_30 as price_ma_30,
    ma.price_ma_57 as price_ma_57,
    ma.price_ma_60 as price_ma_60,
    ma.price_ma_114 as price_ma_114,
    ma.price_ma_250 as price_ma_250,
    ma.price_avg_ma_3_6_12_24 as price_avg_ma_3_6_12_24,
    ma.price_avg_ma_14_28_57_114 as price_avg_ma_14_28_57_114,
    ma.price_ema2_10 as price_ema2_10,
    boll.boll_mid_10_1p5 as boll_mid_10_1p5,
    boll.boll_upper_10_1p5 as boll_upper_10_1p5,
    boll.boll_lower_10_1p5 as boll_lower_10_1p5,
    boll.boll_mid_20_2 as boll_mid_20_2,
    boll.boll_upper_20_2 as boll_upper_20_2,
    boll.boll_lower_20_2 as boll_lower_20_2,
    boll.boll_mid_50_2p5 as boll_mid_50_2p5,
    boll.boll_upper_50_2p5 as boll_upper_50_2p5,
    boll.boll_lower_50_2p5 as boll_lower_50_2p5,
    macd.macd_dif as macd_dif,
    macd.macd_dea as macd_dea,
    macd.macd_histogram as macd_histogram
from ma
left join boll
    on ma.security_code = boll.security_code
    and ma.trade_date = boll.trade_date
left join macd
    on ma.security_code = macd.security_code
    and ma.trade_date = macd.trade_date
