select
    security_code,
    trade_date,
    return_daily
from {{ ref('int_index_quotes_daily') }}
where return_daily is not null
    and not (return_daily > -1 and return_daily <= 1)
