select *
from {{ ref('int_stock_quotes_daily_unadj') }}
where trade_date <= toDate('1995-01-01')
