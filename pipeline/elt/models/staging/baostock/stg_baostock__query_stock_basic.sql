with source as (
    select
        code,
        code_name,
        ipoDate,
        outDate,
        type,
        status
    from {{ source('raw', 'baostock__query_stock_basic') }}
),

normalized as (
    select
        {{ normalize_cn_security_code('code', input_format='baostock_prefix') }} as security_code,
        {{ cn_security_local_code('code', input_format='baostock_prefix') }} as security_local_code,
        {{ cn_exchange_code('code', input_format='baostock_prefix') }} as exchange_code,
        trim(code_name) as security_name,
        ipoDate as ipo_date,
        outDate as out_date,
        type as security_type_code,
        status as listing_status_code
    from source
)

select
    security_code,
    security_local_code,
    exchange_code,
    security_name,
    ipo_date,
    out_date,
    security_type_code,
    cast(
        multiIf(
            security_type_code = 1, 'stock',
            security_type_code = 2, 'index',
            security_type_code = 3, 'other',
            security_type_code = 4, 'convertible_bond',
            security_type_code = 5, 'etf',
            'unknown'
        ),
        'Enum8(\'stock\' = 1, \'index\' = 2, \'other\' = 3, \'convertible_bond\' = 4, \'etf\' = 5)'
    ) as security_type,
    cast(
        multiIf(
            security_type_code = 1 and exchange_code = 'SH' and startsWith(security_local_code, '68'), 'star_market',
            security_type_code = 1 and exchange_code = 'SH' and match(security_local_code, '^(600|601|603|605)'), 'sse_main_board',
            security_type_code = 1 and exchange_code = 'SZ' and startsWith(security_local_code, '30'), 'chinext',
            security_type_code = 1 and exchange_code = 'SZ' and match(security_local_code, '^(000|001|002|003)'), 'szse_main_board',
            null
        ),
        'Nullable(Enum8(\'sse_main_board\' = 1, \'szse_main_board\' = 2, \'chinext\' = 3, \'star_market\' = 4))'
    ) as security_board,
    listing_status_code,
    cast(
        multiIf(
            listing_status_code = 1, 'listed',
            listing_status_code = 0, 'delisted',
            'unknown'
        ),
        'Enum8(\'delisted\' = 0, \'listed\' = 1)'
    ) as listing_status
from normalized
