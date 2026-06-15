select
    mart.security_code,
    mart.trade_date
from {{ ref('mart_stock_quotes_daily') }} as mart
inner join {{ ref('int_stock_quotes_daily_adj') }} as adjusted
    using (security_code, trade_date)
where not (
    mart.open_price_forward_adj is not distinct
        from adjusted.open_price_forward_adj
    and mart.high_price_forward_adj is not distinct
        from adjusted.high_price_forward_adj
    and mart.low_price_forward_adj is not distinct
        from adjusted.low_price_forward_adj
    and mart.close_price_forward_adj is not distinct
        from adjusted.close_price_forward_adj
    and mart.prev_close_price_forward_adj is not distinct
        from adjusted.prev_close_price_forward_adj
    and mart.open_price_backward_adj is not distinct
        from adjusted.open_price_backward_adj
    and mart.high_price_backward_adj is not distinct
        from adjusted.high_price_backward_adj
    and mart.low_price_backward_adj is not distinct
        from adjusted.low_price_backward_adj
    and mart.close_price_backward_adj is not distinct
        from adjusted.close_price_backward_adj
    and mart.prev_close_price_backward_adj is not distinct
        from adjusted.prev_close_price_backward_adj
    and mart.forward_adjustment_factor is not distinct
        from adjusted.forward_adjustment_factor
    and mart.forward_adjustment_ratio is not distinct
        from adjusted.forward_adjustment_ratio
    and mart.backward_adjustment_factor is not distinct
        from adjusted.backward_adjustment_factor
    and mart.backward_adjustment_ratio is not distinct
        from adjusted.backward_adjustment_ratio
)
