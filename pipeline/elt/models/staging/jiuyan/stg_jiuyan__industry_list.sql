with source as (
    select
        industry_id,
        title,
        title_red,
        title_bold,
        content,
        is_top,
        sort_no,
        is_delete,
        create_time,
        update_time
    from {{ source('raw', 'jiuyan__industry_list') }}
)

select
    industry_id,
    title,
    title_red,
    title_bold,
    content,
    is_top,
    sort_no,
    is_delete,
    create_time,
    update_time
from source
