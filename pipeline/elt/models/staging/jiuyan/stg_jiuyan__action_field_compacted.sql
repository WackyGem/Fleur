with source as (
    select
        action_field_id,
        name,
        date,
        reason,
        sort_no,
        is_delete,
        create_time,
        `count` as raw_count,
        code,
        time,
        num,
        day,
        edition,
        expound
    from {{ source('raw', 'jiuyan__action_field_compacted') }}
)

select
    action_field_id,
    {{ normalize_cn_security_code('code', input_format='compact_prefix') }} as security_code,
    date as trade_date,
    name as action_field_name,
    reason,
    sort_no,
    is_delete,
    create_time,
    raw_count as related_count,
    time as event_time,
    num as limit_board_text,
    day as limit_days,
    edition as limit_boards,
    expound
from source
