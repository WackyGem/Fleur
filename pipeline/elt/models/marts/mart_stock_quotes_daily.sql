{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, trade_date)',
    partition_by='toYear(trade_date)'
) }}

with quotes as (
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
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

financial_valuation as (
    select
        security_code,
        report_date,
        pe_static,
        pe_ttm,
        pe_forecast,
        pb_mrq,
        book_value_per_share,
        roe,
        roae
    from {{ ref('int_stock_financial_valuation') }}
),

kdj as (
    select
        security_code,
        trade_date,
        rsv as kdj_rsv,
        k_value as kdj_k_value,
        d_value as kdj_d_value,
        j_value as kdj_j_value
    from {{ ref('int_stock_kdj_daily') }}
),

quotes_with_financial_valuation as (
    select
        quotes.security_code,
        quotes.trade_date,
        quotes.open_price,
        quotes.high_price,
        quotes.low_price,
        quotes.close_price,
        quotes.prev_close_price,
        quotes.prev_close_price_unadj,
        quotes.volume,
        quotes.amount,
        quotes.turnover_rate,
        quotes.turnover_rate_actual,
        quotes.pct_amplitude,
        quotes.pct_change,
        quotes.limit_up_price,
        quotes.limit_down_price,
        quotes.a_market_cap,
        quotes.a_float_market_cap,
        quotes.a_free_float_market_cap,
        quotes.a_shares,
        quotes.a_float_shares,
        quotes.a_free_float_shares,
        financial_valuation.pe_static,
        financial_valuation.pe_ttm,
        financial_valuation.pe_forecast,
        financial_valuation.pb_mrq,
        financial_valuation.book_value_per_share,
        financial_valuation.roe,
        financial_valuation.roae,
        quotes.dy_static,
        quotes.dy_ttm,
        quotes.is_suspend,
        quotes.is_st,
        kdj.kdj_rsv,
        kdj.kdj_k_value,
        kdj.kdj_d_value,
        kdj.kdj_j_value
    from quotes
    asof left join financial_valuation
        on quotes.security_code = financial_valuation.security_code
        and quotes.trade_date >= financial_valuation.report_date
    left join kdj
        on quotes.security_code = kdj.security_code
        and quotes.trade_date = kdj.trade_date
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
    pe_static,
    pe_ttm,
    pe_forecast,
    pb_mrq,
    book_value_per_share,
    roe,
    roae,
    dy_static,
    dy_ttm,
    is_suspend,
    is_st,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value
from quotes_with_financial_valuation
