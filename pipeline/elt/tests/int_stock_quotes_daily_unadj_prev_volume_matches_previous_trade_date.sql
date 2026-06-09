select
    current_quotes.security_code,
    current_quotes.trade_date,
    current_quotes.prev_volume,
    previous_quotes.volume as expected_prev_volume
from {{ ref('int_stock_quotes_daily_unadj') }} as current_quotes
left join {{ ref('int_trade_calendar') }} as trade_calendar
    on current_quotes.trade_date = trade_calendar.trade_date
left join {{ ref('int_stock_quotes_daily_unadj') }} as previous_quotes
    on current_quotes.security_code = previous_quotes.security_code
    and trade_calendar.prev_trade_date = previous_quotes.trade_date
where not (current_quotes.prev_volume is not distinct from previous_quotes.volume)
