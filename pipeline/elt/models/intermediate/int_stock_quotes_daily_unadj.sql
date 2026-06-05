with stock_universe as (
    select
        security_code
    from {{ ref('int_stock_basic_snapshot') }}
),

stock_quotes as (
    select
        quotes.security_code,
        quotes.trade_date,
        quotes.open_price,
        quotes.high_price,
        quotes.low_price,
        quotes.close_price,
        quotes.prev_close_price,
        quotes.volume,
        quotes.amount,
        quotes.is_suspend,
        quotes.is_st
    from {{ ref('stg_baostock__query_history_k_data_plus_daily') }} as quotes
    inner join stock_universe
        on quotes.security_code = stock_universe.security_code
    where quotes.trade_date > toDate('1995-01-01')
),

quotes_with_prev_trade_date as (
    select
        stock_quotes.security_code,
        stock_quotes.trade_date,
        stock_quotes.open_price,
        stock_quotes.high_price,
        stock_quotes.low_price,
        stock_quotes.close_price,
        stock_quotes.prev_close_price,
        stock_quotes.volume,
        stock_quotes.amount,
        stock_quotes.is_suspend,
        stock_quotes.is_st,
        trade_calendar.prev_trade_date
    from stock_quotes
    left join {{ ref('int_trade_calendar') }} as trade_calendar
        on stock_quotes.trade_date = trade_calendar.trade_date
),

quotes_with_prev_close_unadj as (
    select
        current_quotes.security_code,
        current_quotes.trade_date,
        current_quotes.open_price,
        current_quotes.high_price,
        current_quotes.low_price,
        current_quotes.close_price,
        current_quotes.prev_close_price,
        previous_quotes.close_price as prev_close_price_unadj,
        current_quotes.volume,
        current_quotes.amount,
        current_quotes.is_suspend,
        current_quotes.is_st
    from quotes_with_prev_trade_date as current_quotes
    left join stock_quotes as previous_quotes
        on current_quotes.security_code = previous_quotes.security_code
        and current_quotes.prev_trade_date = previous_quotes.trade_date
)

select
    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    prev_close_price,
    prev_close_price_unadj,
    volume,
    amount,
    is_suspend,
    is_st
from quotes_with_prev_close_unadj
