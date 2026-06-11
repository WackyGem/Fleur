select
    security_code,
    trade_date
from {{ ref('mart_stock_trend_indicator') }}
where
    (
        boll_up_10_1p5 is not null
        and boll_mid_10_1p5 is not null
        and boll_dn_10_1p5 is not null
        and not (boll_up_10_1p5 >= boll_mid_10_1p5 and boll_mid_10_1p5 >= boll_dn_10_1p5)
    )
    or (
        boll_up_20_2 is not null
        and boll_mid_20_2 is not null
        and boll_dn_20_2 is not null
        and not (boll_up_20_2 >= boll_mid_20_2 and boll_mid_20_2 >= boll_dn_20_2)
    )
    or (
        boll_up_50_2p5 is not null
        and boll_mid_50_2p5 is not null
        and boll_dn_50_2p5 is not null
        and not (boll_up_50_2p5 >= boll_mid_50_2p5 and boll_mid_50_2p5 >= boll_dn_50_2p5)
    )
