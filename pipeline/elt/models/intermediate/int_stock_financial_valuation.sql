{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(security_code, report_date)',
    partition_by='toYear(report_date)'
) }}

with report_periods as (
    select
        security_code,
        report_date
    from {{ ref('stg_eastmoney__income_ytd') }}

    union distinct

    select
        security_code,
        report_date
    from {{ ref('stg_eastmoney__balance') }}
),

stock_quotes as (
    select
        security_code,
        trade_date,
        close_price
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

report_price as (
    select
        periods.security_code,
        periods.report_date,
        quotes.close_price
    from report_periods as periods
    asof left join stock_quotes as quotes
        on periods.security_code = quotes.security_code
        and periods.report_date >= quotes.trade_date
),

shares_history as (
    select
        security_code,
        effective_date,
        expiry_date,
        toNullable(total_shares) as total_shares
    from {{ ref('int_stock_shares_history') }}
),

report_shares as (
    select
        periods.security_code,
        periods.report_date,
        if(
            shares_history.expiry_date is null
            or periods.report_date <= shares_history.expiry_date,
            shares_history.total_shares,
            cast(null, 'Nullable(Float64)')
        ) as total_shares
    from report_periods as periods
    asof left join shares_history
        on periods.security_code = shares_history.security_code
        and periods.report_date >= shares_history.effective_date
),

market_cap as (
    select
        security_code,
        report_date,
        if(
            prices.close_price is null
            or prices.close_price <= 0
            or shares.total_shares is null
            or shares.total_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            prices.close_price * shares.total_shares
        ) as market_cap_at_report_date
    from report_periods as periods
    left any join report_price as prices
        using (security_code, report_date)
    left any join report_shares as shares
        using (security_code, report_date)
),

annual_income as (
    select
        security_code,
        report_date as annual_report_date,
        toNullable(parent_netprofit) as annual_parent_netprofit
    from {{ ref('stg_eastmoney__income_ytd') }}
    where report_type = '年报'
),

report_annual_income as (
    select
        periods.security_code,
        periods.report_date,
        annual_values.annual_parent_netprofit
    from report_periods as periods
    asof left join annual_income as annual_values
        on periods.security_code = annual_values.security_code
        and periods.report_date >= annual_values.annual_report_date
),

income_sq_windowed as (
    select
        security_code,
        report_date,
        count(parent_netprofit) over (
            partition by security_code
            order by report_date
            rows between 3 preceding and current row
        ) as ttm_quarter_count,
        sum(parent_netprofit) over (
            partition by security_code
            order by report_date
            rows between 3 preceding and current row
        ) as ttm_parent_netprofit_raw,
        lagInFrame(
            toNullable(report_date),
            3,
            cast(null, 'Nullable(Date)')
        ) over (
            partition by security_code
            order by report_date
            rows between unbounded preceding and unbounded following
        ) as earliest_ttm_report_date
    from {{ ref('stg_eastmoney__income_sq') }}
),

ttm_income as (
    select
        security_code,
        report_date,
        if(
            ttm_quarter_count = 4
            and earliest_ttm_report_date = addMonths(report_date, -9),
            toNullable(ttm_parent_netprofit_raw),
            cast(null, 'Nullable(Float64)')
        ) as ttm_parent_netprofit
    from income_sq_windowed
),

income_ytd_current as (
    select
        security_code,
        report_date,
        toNullable(parent_netprofit) as ytd_parent_netprofit,
        multiIf(
            report_type = '一季报',
            1,
            report_type = '中报',
            2,
            report_type = '三季报',
            3,
            report_type = '年报',
            4,
            cast(null, 'Nullable(UInt8)')
        ) as quarter_count
    from {{ ref('stg_eastmoney__income_ytd') }}
),

forecast_income as (
    select
        security_code,
        report_date,
        ytd_parent_netprofit,
        if(
            ytd_parent_netprofit is null
            or quarter_count is null,
            cast(null, 'Nullable(Float64)'),
            ytd_parent_netprofit * (4.0 / quarter_count)
        ) as forecast_full_year_parent_netprofit
    from income_ytd_current
),

balance_history as (
    select
        security_code,
        report_date,
        toNullable(report_date) as balance_report_date,
        toNullable(total_parent_equity) as total_parent_equity,
        toNullable(share_capital) as share_capital
    from {{ ref('stg_eastmoney__balance') }}
),

latest_mrq_equity as (
    select
        periods.security_code,
        periods.report_date,
        balances.balance_report_date,
        balances.total_parent_equity,
        balances.share_capital
    from report_periods as periods
    asof left join balance_history as balances
        on periods.security_code = balances.security_code
        and periods.report_date >= balances.report_date
),

report_periods_with_beginning as (
    select
        security_code,
        report_date,
        toDate(concat(toString(toYear(report_date) - 1), '-12-31')) as beginning_report_date
    from report_periods
),

beginning_parent_equity as (
    select
        periods.security_code,
        periods.report_date,
        balances.total_parent_equity as beginning_total_parent_equity
    from report_periods_with_beginning as periods
    left any join balance_history as balances
        on periods.security_code = balances.security_code
        and periods.beginning_report_date = balances.report_date
),

valuation_inputs as (
    select
        security_code,
        report_date,
        market_cap_values.market_cap_at_report_date,
        annual_values.annual_parent_netprofit,
        ttm_values.ttm_parent_netprofit,
        forecast_values.forecast_full_year_parent_netprofit,
        forecast_values.ytd_parent_netprofit,
        equity_values.total_parent_equity,
        equity_values.share_capital,
        beginning_equity_values.beginning_total_parent_equity
    from report_periods as periods
    left any join market_cap as market_cap_values
        using (security_code, report_date)
    left any join report_annual_income as annual_values
        using (security_code, report_date)
    left any join ttm_income as ttm_values
        using (security_code, report_date)
    left any join forecast_income as forecast_values
        using (security_code, report_date)
    left any join latest_mrq_equity as equity_values
        using (security_code, report_date)
    left any join beginning_parent_equity as beginning_equity_values
        using (security_code, report_date)
)

select
    security_code,
    report_date,
    if(
        market_cap_at_report_date is null
        or market_cap_at_report_date <= 0
        or annual_parent_netprofit is null
        or annual_parent_netprofit <= 0,
        cast(null, 'Nullable(Float64)'),
        market_cap_at_report_date / annual_parent_netprofit
    ) as pe_static,
    if(
        market_cap_at_report_date is null
        or market_cap_at_report_date <= 0
        or ttm_parent_netprofit is null
        or ttm_parent_netprofit <= 0,
        cast(null, 'Nullable(Float64)'),
        market_cap_at_report_date / ttm_parent_netprofit
    ) as pe_ttm,
    if(
        market_cap_at_report_date is null
        or market_cap_at_report_date <= 0
        or forecast_full_year_parent_netprofit is null
        or forecast_full_year_parent_netprofit <= 0,
        cast(null, 'Nullable(Float64)'),
        market_cap_at_report_date / forecast_full_year_parent_netprofit
    ) as pe_forecast,
    if(
        market_cap_at_report_date is null
        or market_cap_at_report_date <= 0
        or total_parent_equity is null
        or total_parent_equity <= 0,
        cast(null, 'Nullable(Float64)'),
        market_cap_at_report_date / total_parent_equity
    ) as pb_mrq,
    if(
        total_parent_equity is null
        or total_parent_equity <= 0
        or share_capital is null
        or share_capital <= 0,
        cast(null, 'Nullable(Float64)'),
        total_parent_equity / share_capital
    ) as book_value_per_share,
    if(
        ytd_parent_netprofit is null
        or total_parent_equity is null
        or total_parent_equity <= 0,
        cast(null, 'Nullable(Float64)'),
        ytd_parent_netprofit / total_parent_equity
    ) as roe,
    if(
        ytd_parent_netprofit is null
        or beginning_total_parent_equity is null
        or beginning_total_parent_equity <= 0
        or total_parent_equity is null
        or total_parent_equity <= 0
        or (beginning_total_parent_equity + total_parent_equity) / 2 <= 0,
        cast(null, 'Nullable(Float64)'),
        ytd_parent_netprofit / ((beginning_total_parent_equity + total_parent_equity) / 2)
    ) as roae
from valuation_inputs
