{% test baostock_security_board_consistency(model, column_name) %}

with validation as (
    select
        *,
        multiIf(
            security_type_code = 1 and exchange_code = 'SH' and startsWith(security_local_code, '68'), 'star_market',
            security_type_code = 1 and exchange_code = 'SH' and match(security_local_code, '^(600|601|603|605)'), 'sse_main_board',
            security_type_code = 1 and exchange_code = 'SZ' and startsWith(security_local_code, '30'), 'chinext',
            security_type_code = 1 and exchange_code = 'SZ' and match(security_local_code, '^(000|001|002|003)'), 'szse_main_board',
            null
        ) as expected_security_board
    from {{ model }}
)

select *
from validation
where
    (isNull(expected_security_board) and {{ column_name }} is not null)
    or (
        expected_security_board is not null
        and toString({{ column_name }}) != expected_security_board
    )

{% endtest %}
