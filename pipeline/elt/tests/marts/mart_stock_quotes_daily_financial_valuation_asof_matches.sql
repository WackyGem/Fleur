with expected as (
    select
        mart.security_code,
        mart.trade_date,
        valuation.pe_static as expected_pe_static,
        valuation.pe_ttm as expected_pe_ttm,
        valuation.pe_forecast as expected_pe_forecast,
        valuation.pb_mrq as expected_pb_mrq,
        valuation.book_value_per_share as expected_book_value_per_share,
        valuation.roe as expected_roe,
        valuation.roa as expected_roa,
        valuation.roaa as expected_roaa,
        valuation.roae as expected_roae
    from {{ ref('mart_stock_quotes_daily') }} as mart
    asof left join {{ ref('int_stock_financial_valuation') }} as valuation
        on mart.security_code = valuation.security_code
        and mart.trade_date >= valuation.report_date
)

select
    mart.security_code,
    mart.trade_date
from {{ ref('mart_stock_quotes_daily') }} as mart
inner join expected
    using (security_code, trade_date)
where not (
    mart.pe_static is not distinct from expected.expected_pe_static
    and mart.pe_ttm is not distinct from expected.expected_pe_ttm
    and mart.pe_forecast is not distinct from expected.expected_pe_forecast
    and mart.pb_mrq is not distinct from expected.expected_pb_mrq
    and mart.book_value_per_share is not distinct
        from expected.expected_book_value_per_share
    and mart.roe is not distinct from expected.expected_roe
    and mart.roa is not distinct from expected.expected_roa
    and mart.roaa is not distinct from expected.expected_roaa
    and mart.roae is not distinct from expected.expected_roae
)
