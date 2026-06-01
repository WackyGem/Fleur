{{ config(materialized='view') }}

select
    industry_id,
    title_red,
    title_bold,
    title,
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
    delete_time,
    create_time,
    update_time
from {{ source('raw', 'jiuyan__industry_list') }}
