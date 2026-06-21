select
    security_code,
    trade_date
from {{ ref('mart_stock_trend_indicator_daily') }}
where
    (
        boll_upper_10_1p5 is not null
        and boll_mid_10_1p5 is not null
        and boll_lower_10_1p5 is not null
        and not (boll_upper_10_1p5 >= boll_mid_10_1p5 and boll_mid_10_1p5 >= boll_lower_10_1p5)
    )
    or (
        boll_upper_20_2 is not null
        and boll_mid_20_2 is not null
        and boll_lower_20_2 is not null
        and not (boll_upper_20_2 >= boll_mid_20_2 and boll_mid_20_2 >= boll_lower_20_2)
    )
    or (
        boll_upper_50_2p5 is not null
        and boll_mid_50_2p5 is not null
        and boll_lower_50_2p5 is not null
        and not (boll_upper_50_2p5 >= boll_mid_50_2p5 and boll_mid_50_2p5 >= boll_lower_50_2p5)
    )
