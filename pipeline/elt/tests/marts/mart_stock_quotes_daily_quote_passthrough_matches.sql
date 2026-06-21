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
    and mart.turnover_rate_pct is not distinct from quotes.turnover_rate_pct
    and mart.turnover_rate_free_float_pct is not distinct from quotes.turnover_rate_free_float_pct
    and mart.amplitude_pct is not distinct from quotes.amplitude_pct
    and mart.change_pct is not distinct from quotes.change_pct
    and mart.limit_up_price is not distinct from quotes.limit_up_price
    and mart.limit_down_price is not distinct from quotes.limit_down_price
    and mart.market_cap is not distinct from quotes.market_cap
    and mart.float_market_cap is not distinct from quotes.float_market_cap
    and mart.free_float_market_cap is not distinct from quotes.free_float_market_cap
    and mart.shares is not distinct from quotes.shares
    and mart.float_shares_a is not distinct from quotes.float_shares_a
    and mart.free_float_shares is not distinct from quotes.free_float_shares
    and mart.dy_static_pct is not distinct from quotes.dy_static_pct
    and mart.dy_ttm_pct is not distinct from quotes.dy_ttm_pct
    and mart.is_suspend is not distinct from quotes.is_suspend
    and mart.is_st is not distinct from quotes.is_st
)
