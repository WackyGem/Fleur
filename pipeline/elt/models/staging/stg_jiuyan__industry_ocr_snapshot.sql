{{ config(materialized='view') }}

select
    industry_id,
    image_filename,
    image_index,
    ocr_row_index,
    stock_name,
    theme_path,
    relation,
    source
from {{ source('raw', 'jiuyan__industry_ocr_snapshot') }}
