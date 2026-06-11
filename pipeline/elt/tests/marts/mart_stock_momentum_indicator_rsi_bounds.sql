select
    security_code,
    trade_date
from {{ ref('mart_stock_momentum_indicator') }}
where
    (rsi_6 is not null and not (rsi_6 >= 0 and rsi_6 <= 100))
    or (rsi_12 is not null and not (rsi_12 >= 0 and rsi_12 <= 100))
    or (rsi_14 is not null and not (rsi_14 >= 0 and rsi_14 <= 100))
    or (rsi_24 is not null and not (rsi_24 >= 0 and rsi_24 <= 100))
    or (rsi_25 is not null and not (rsi_25 >= 0 and rsi_25 <= 100))
    or (rsi_50 is not null and not (rsi_50 >= 0 and rsi_50 <= 100))
