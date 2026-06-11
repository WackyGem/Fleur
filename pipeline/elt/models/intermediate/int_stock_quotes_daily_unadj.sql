{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}

with stock_universe as (
    select
        security_code,
        security_board
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
        quotes.is_st,
        stock_universe.security_board
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
        stock_quotes.security_board,
        trade_calendar.prev_trade_date
    from stock_quotes
    left any join {{ ref('int_trade_calendar') }} as trade_calendar
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
        previous_quotes.volume as prev_volume,
        current_quotes.volume,
        current_quotes.amount,
        current_quotes.is_suspend,
        current_quotes.is_st,
        current_quotes.security_board
    from quotes_with_prev_trade_date as current_quotes
    left any join stock_quotes as previous_quotes
        on current_quotes.security_code = previous_quotes.security_code
        and current_quotes.prev_trade_date = previous_quotes.trade_date
),

shares_history as (
    select
        security_code,
        effective_date,
        expiry_date,
        toNullable(a_shares) as a_shares,
        toNullable(a_float_shares) as a_float_shares,
        toNullable(a_free_float_shares) as a_free_float_shares
    from {{ ref('int_stock_shares_history') }}
),

quotes_with_shares as (
    select
        quotes_with_prev_close_unadj.security_code,
        quotes_with_prev_close_unadj.trade_date,
        quotes_with_prev_close_unadj.open_price,
        quotes_with_prev_close_unadj.high_price,
        quotes_with_prev_close_unadj.low_price,
        quotes_with_prev_close_unadj.close_price,
        quotes_with_prev_close_unadj.prev_close_price,
        quotes_with_prev_close_unadj.prev_close_price_unadj,
        quotes_with_prev_close_unadj.prev_volume,
        quotes_with_prev_close_unadj.volume,
        quotes_with_prev_close_unadj.amount,
        quotes_with_prev_close_unadj.is_suspend,
        quotes_with_prev_close_unadj.is_st,
        quotes_with_prev_close_unadj.security_board,
        if(
            shares_history.expiry_date is null
            or quotes_with_prev_close_unadj.trade_date <= shares_history.expiry_date,
            shares_history.a_shares,
            cast(null, 'Nullable(Float64)')
        ) as a_shares,
        if(
            shares_history.expiry_date is null
            or quotes_with_prev_close_unadj.trade_date <= shares_history.expiry_date,
            shares_history.a_float_shares,
            cast(null, 'Nullable(Float64)')
        ) as a_float_shares,
        if(
            shares_history.expiry_date is null
            or quotes_with_prev_close_unadj.trade_date <= shares_history.expiry_date,
            shares_history.a_free_float_shares,
            cast(null, 'Nullable(Float64)')
        ) as a_free_float_shares
    from quotes_with_prev_close_unadj
    asof left join shares_history
        on quotes_with_prev_close_unadj.security_code = shares_history.security_code
        and quotes_with_prev_close_unadj.trade_date >= shares_history.effective_date
),

cash_dividend_events as (
    select
        security_code,
        ex_dividend_date,
        sum(exrights_events.cash_dividend_per_share) as cash_dividend_per_share
    from {{ ref('int_stock_exrights_event') }} as exrights_events
    where exrights_events.has_cash_dividend = true
      and exrights_events.cash_dividend_per_share > 0
    group by
        security_code,
        ex_dividend_date
),

annual_cash_dividends as (
    select
        security_code,
        latest_ex_dividend_date,
        toNullable(annual_cash_dividend_per_share) as latest_annual_cash_dividend_per_share
    from (
        select
            security_code,
            report_date,
            sum(exrights_events.cash_dividend_per_share) as annual_cash_dividend_per_share,
            max(ex_dividend_date) as latest_ex_dividend_date
        from {{ ref('int_stock_exrights_event') }} as exrights_events
        where exrights_events.has_cash_dividend = true
          and exrights_events.cash_dividend_per_share > 0
          and exrights_events.report_date is not null
          and toMonth(exrights_events.report_date) = 12
          and toDayOfMonth(exrights_events.report_date) = 31
        group by
            security_code,
            report_date
    )
),

cash_dividend_cumulative as (
    select
        security_code,
        ex_dividend_date,
        toNullable(
            sum(cash_dividend_per_share) over (
                partition by security_code
                order by ex_dividend_date
                rows between unbounded preceding and current row
            )
        ) as cumulative_cash_dividend_per_share
    from cash_dividend_events
),

quotes_with_static_dividend as (
    select
        quotes_with_shares.*,
        annual_cash_dividends.latest_annual_cash_dividend_per_share
    from quotes_with_shares
    asof left join annual_cash_dividends
        on quotes_with_shares.security_code = annual_cash_dividends.security_code
        and quotes_with_shares.trade_date >= annual_cash_dividends.latest_ex_dividend_date
),

quotes_with_current_dividend_cumulative as (
    select
        quotes_with_static_dividend.*,
        addYears(quotes_with_static_dividend.trade_date, -1) as ttm_start_date,
        current_cumulative.cumulative_cash_dividend_per_share
            as current_cumulative_cash_dividend_per_share
    from quotes_with_static_dividend
    asof left join cash_dividend_cumulative as current_cumulative
        on quotes_with_static_dividend.security_code = current_cumulative.security_code
        and quotes_with_static_dividend.trade_date >= current_cumulative.ex_dividend_date
),

quotes_with_dividends as (
    select
        quotes_with_current_dividend_cumulative.*,
        previous_cumulative.cumulative_cash_dividend_per_share
            as previous_cumulative_cash_dividend_per_share
    from quotes_with_current_dividend_cumulative
    asof left join cash_dividend_cumulative as previous_cumulative
        on quotes_with_current_dividend_cumulative.security_code
            = previous_cumulative.security_code
        and quotes_with_current_dividend_cumulative.ttm_start_date
            >= previous_cumulative.ex_dividend_date
),

quotes_with_metrics as (
    select
        security_code,
        trade_date,
        open_price,
        high_price,
        low_price,
        close_price,
        prev_close_price,
        prev_close_price_unadj,
        prev_volume,
        volume,
        amount,
        if(
            volume is null
            or a_float_shares is null
            or a_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            volume / a_float_shares * 100
        ) as turnover_rate,
        if(
            volume is null
            or a_free_float_shares is null
            or a_free_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            volume / a_free_float_shares * 100
        ) as turnover_rate_actual,
        if(
            high_price is null
            or low_price is null
            or prev_close_price is null
            or prev_close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            (high_price - low_price) / prev_close_price * 100
        ) as pct_amplitude,
        if(
            close_price is null
            or prev_close_price is null
            or prev_close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            (close_price - prev_close_price) / prev_close_price * 100
        ) as pct_change,
        multiIf(
            security_board in ('sse_main_board', 'szse_main_board')
                and coalesce(is_st, false) = true
                and trade_date < toDate('2026-07-06'),
            0.05,
            security_board in ('sse_main_board', 'szse_main_board')
                and coalesce(is_st, false) = true
                and trade_date >= toDate('2026-07-06'),
            0.10,
            security_board in ('sse_main_board', 'szse_main_board'),
            0.10,
            security_board in ('star_market', 'chinext'),
            0.20,
            cast(null, 'Nullable(Float64)')
        ) as price_limit_ratio,
        if(
            close_price is null
            or close_price < 0
            or a_shares is null
            or a_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            close_price * a_shares
        ) as a_market_cap,
        if(
            close_price is null
            or close_price < 0
            or a_float_shares is null
            or a_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            close_price * a_float_shares
        ) as a_float_market_cap,
        if(
            close_price is null
            or close_price < 0
            or a_free_float_shares is null
            or a_free_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            close_price * a_free_float_shares
        ) as a_free_float_market_cap,
        a_shares,
        a_float_shares,
        a_free_float_shares,
        if(
            latest_annual_cash_dividend_per_share is null
            or close_price is null
            or close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            latest_annual_cash_dividend_per_share / close_price * 100
        ) as dy_static,
        if(
            current_cumulative_cash_dividend_per_share is null
            or close_price is null
            or close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            (
                current_cumulative_cash_dividend_per_share
                - coalesce(previous_cumulative_cash_dividend_per_share, 0)
            ) / close_price * 100
        ) as dy_ttm,
        is_suspend,
        is_st
    from quotes_with_dividends
),

quotes_with_limit_prices as (
    select
        security_code,
        trade_date,
        open_price,
        high_price,
        low_price,
        close_price,
        prev_close_price,
        prev_close_price_unadj,
        prev_volume,
        volume,
        amount,
        turnover_rate,
        turnover_rate_actual,
        pct_amplitude,
        pct_change,
        if(
            prev_close_price is null
            or prev_close_price <= 0
            or price_limit_ratio is null,
            cast(null, 'Nullable(Float64)'),
            round(prev_close_price * (1 + price_limit_ratio), 2)
        ) as limit_up_price,
        if(
            prev_close_price is null
            or prev_close_price <= 0
            or price_limit_ratio is null,
            cast(null, 'Nullable(Float64)'),
            round(prev_close_price * (1 - price_limit_ratio), 2)
        ) as limit_down_price,
        a_market_cap,
        a_float_market_cap,
        a_free_float_market_cap,
        a_shares,
        a_float_shares,
        a_free_float_shares,
        dy_static,
        dy_ttm,
        is_suspend,
        is_st
    from quotes_with_metrics
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
    prev_volume,
    volume,
    amount,
    turnover_rate,
    turnover_rate_actual,
    pct_amplitude,
    pct_change,
    limit_up_price,
    limit_down_price,
    a_market_cap,
    a_float_market_cap,
    a_free_float_market_cap,
    a_shares,
    a_float_shares,
    a_free_float_shares,
    dy_static,
    dy_ttm,
    is_suspend,
    is_st
from quotes_with_limit_prices
