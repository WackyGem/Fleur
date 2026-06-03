with source as (
    select
        industry_id,
        title,
        title_red,
        title_bold,
        author,
        imgs,
        keyword,
        content,
        is_top,
        status,
        sort_no,
        forward_count,
        browsers_count,
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
    author,
    imgs as images_raw,
    keyword,
    content,
    is_top,
    status as status_code,
    sort_no,
    forward_count,
    browsers_count,
    is_delete,
    create_time,
    update_time
from source
