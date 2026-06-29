{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
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
        prev_volume,
        volume,
        amount,
        turnover_rate_pct,
        turnover_rate_free_float_pct,
        amplitude_pct,
        change_pct,
        limit_up_price,
        limit_down_price,
        market_cap,
        float_market_cap,
        free_float_market_cap,
        shares,
        float_shares_a,
        free_float_shares,
        dy_static_pct,
        dy_ttm_pct,
        is_suspend,
        is_st
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

adjusted_quotes as (
    select
        security_code,
        trade_date,
        open_price_forward_adj,
        high_price_forward_adj,
        low_price_forward_adj,
        close_price_forward_adj,
        prev_close_price_forward_adj,
        open_price_backward_adj,
        high_price_backward_adj,
        low_price_backward_adj,
        close_price_backward_adj,
        prev_close_price_backward_adj,
        toNullable(forward_adjustment_factor) as forward_adjustment_factor,
        toNullable(forward_adjustment_ratio) as forward_adjustment_ratio,
        toNullable(backward_adjustment_factor) as backward_adjustment_factor,
        toNullable(backward_adjustment_ratio) as backward_adjustment_ratio
    from {{ ref('int_stock_quotes_daily_adj') }}
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
        roa,
        roaa,
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
        quotes.security_code as security_code,
        quotes.trade_date as trade_date,
        quotes.open_price as open_price,
        quotes.high_price as high_price,
        quotes.low_price as low_price,
        quotes.close_price as close_price,
        quotes.prev_close_price as prev_close_price,
        quotes.prev_close_price_unadj as prev_close_price_unadj,
        adjusted_quotes.open_price_forward_adj as open_price_forward_adj,
        adjusted_quotes.high_price_forward_adj as high_price_forward_adj,
        adjusted_quotes.low_price_forward_adj as low_price_forward_adj,
        adjusted_quotes.close_price_forward_adj as close_price_forward_adj,
        adjusted_quotes.prev_close_price_forward_adj as prev_close_price_forward_adj,
        adjusted_quotes.open_price_backward_adj as open_price_backward_adj,
        adjusted_quotes.high_price_backward_adj as high_price_backward_adj,
        adjusted_quotes.low_price_backward_adj as low_price_backward_adj,
        adjusted_quotes.close_price_backward_adj as close_price_backward_adj,
        adjusted_quotes.prev_close_price_backward_adj as prev_close_price_backward_adj,
        adjusted_quotes.forward_adjustment_factor as forward_adjustment_factor,
        adjusted_quotes.forward_adjustment_ratio as forward_adjustment_ratio,
        adjusted_quotes.backward_adjustment_factor as backward_adjustment_factor,
        adjusted_quotes.backward_adjustment_ratio as backward_adjustment_ratio,
        quotes.prev_volume as prev_volume,
        quotes.volume as volume,
        quotes.amount as amount,
        quotes.turnover_rate_pct as turnover_rate_pct,
        quotes.turnover_rate_free_float_pct as turnover_rate_free_float_pct,
        quotes.amplitude_pct as amplitude_pct,
        quotes.change_pct as change_pct,
        quotes.limit_up_price as limit_up_price,
        quotes.limit_down_price as limit_down_price,
        quotes.market_cap as market_cap,
        quotes.float_market_cap as float_market_cap,
        quotes.free_float_market_cap as free_float_market_cap,
        quotes.shares as shares,
        quotes.float_shares_a as float_shares_a,
        quotes.free_float_shares as free_float_shares,
        financial_valuation.pe_static as pe_static,
        financial_valuation.pe_ttm as pe_ttm,
        financial_valuation.pe_forecast as pe_forecast,
        financial_valuation.pb_mrq as pb_mrq,
        financial_valuation.book_value_per_share as book_value_per_share,
        financial_valuation.roe as roe,
        financial_valuation.roa as roa,
        financial_valuation.roaa as roaa,
        financial_valuation.roae as roae,
        quotes.dy_static_pct as dy_static_pct,
        quotes.dy_ttm_pct as dy_ttm_pct,
        quotes.is_suspend as is_suspend,
        quotes.is_st as is_st,
        kdj.kdj_rsv as kdj_rsv,
        kdj.kdj_k_value as kdj_k_value,
        kdj.kdj_d_value as kdj_d_value,
        kdj.kdj_j_value as kdj_j_value
    from quotes
    asof left join financial_valuation
        on quotes.security_code = financial_valuation.security_code
        and quotes.trade_date >= financial_valuation.report_date
    left any join adjusted_quotes
        on quotes.security_code = adjusted_quotes.security_code
        and quotes.trade_date = adjusted_quotes.trade_date
    left any join kdj
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
    open_price_forward_adj,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj,
    prev_close_price_forward_adj,
    open_price_backward_adj,
    high_price_backward_adj,
    low_price_backward_adj,
    close_price_backward_adj,
    prev_close_price_backward_adj,
    forward_adjustment_factor,
    forward_adjustment_ratio,
    backward_adjustment_factor,
    backward_adjustment_ratio,
    prev_volume,
    volume,
    amount,
    turnover_rate_pct,
    turnover_rate_free_float_pct,
    amplitude_pct,
    change_pct,
    limit_up_price,
    limit_down_price,
    market_cap,
    float_market_cap,
    free_float_market_cap,
    shares,
    float_shares_a,
    free_float_shares,
    pe_static,
    pe_ttm,
    pe_forecast,
    pb_mrq,
    book_value_per_share,
    roe,
    roa,
    roaa,
    roae,
    dy_static_pct,
    dy_ttm_pct,
    is_suspend,
    is_st,
    kdj_rsv,
    kdj_k_value,
    kdj_d_value,
    kdj_j_value
from quotes_with_financial_valuation
