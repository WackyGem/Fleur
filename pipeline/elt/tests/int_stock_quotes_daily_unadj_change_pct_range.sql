select
    security_code,
    trade_date,
    change_pct
from {{ ref('int_stock_quotes_daily_unadj') }}
where change_pct is not null
    and not (change_pct >= -100 and change_pct <= 10000)
