select
    mart.security_code,
    mart.trade_date
from {{ ref('mart_stock_quotes_daily') }} as mart
inner join {{ ref('int_stock_quotes_daily_unadj') }} as quotes
    using (security_code, trade_date)
where not (
    mart.open_price is not distinct from quotes.open_price
    and mart.high_price is not distinct from quotes.high_price
    and mart.low_price is not distinct from quotes.low_price
    and mart.close_price is not distinct from quotes.close_price
    and mart.prev_close_price is not distinct from quotes.prev_close_price
    and mart.prev_close_price_unadj is not distinct from quotes.prev_close_price_unadj
    and mart.prev_volume is not distinct from quotes.prev_volume
    and mart.volume is not distinct from quotes.volume
    and mart.amount is not distinct from quotes.amount
    and mart.turnover_rate is not distinct from quotes.turnover_rate
    and mart.turnover_rate_actual is not distinct from quotes.turnover_rate_actual
    and mart.pct_amplitude is not distinct from quotes.pct_amplitude
    and mart.pct_change is not distinct from quotes.pct_change
    and mart.limit_up_price is not distinct from quotes.limit_up_price
    and mart.limit_down_price is not distinct from quotes.limit_down_price
    and mart.a_market_cap is not distinct from quotes.a_market_cap
    and mart.a_float_market_cap is not distinct from quotes.a_float_market_cap
    and mart.a_free_float_market_cap is not distinct from quotes.a_free_float_market_cap
    and mart.a_shares is not distinct from quotes.a_shares
    and mart.a_float_shares is not distinct from quotes.a_float_shares
    and mart.a_free_float_shares is not distinct from quotes.a_free_float_shares
    and mart.dy_static is not distinct from quotes.dy_static
    and mart.dy_ttm is not distinct from quotes.dy_ttm
    and mart.is_suspend is not distinct from quotes.is_suspend
    and mart.is_st is not distinct from quotes.is_st
)
