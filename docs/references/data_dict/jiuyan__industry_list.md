# jiuyan__industry_list 字段校对

> 生成时间: 2026-05-30 10:31:26 UTC
> OpenAPI 文档: jiuyan__industry_list.yaml

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | industry_id | string | ✅ | string | LowCardinality(String) |
| 2 | title_red | integer | ✅ | bool | Bool |
| 3 | title_bold | integer | ✅ | bool | Bool |
| 4 | title | string | ✅ | string | String |
| 5 | author | string | ✅ | string | LowCardinality(String) |
| 6 | imgs | string | ✅ | string | String |
| 7 | keyword | string | ✅ | string | String |
| 8 | content | string | ✅ | string | String |
| 9 | is_top | integer | ✅ | bool | Bool |
| 10 | status | integer | ✅ | int64 | Int64 |
| 11 | sort_no | integer | ✅ | int64 | Int64 |
| 12 | forward_count | integer | ✅ | int64 | Int64 |
| 13 | browsers_count | integer | ✅ | int64 | Int64 |
| 14 | is_delete | string | ✅ | bool | Bool |
| 15 | delete_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 16 | create_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 17 | update_time | string | ✅ | timestamp[ns] | DateTime64(3) |

## 统计

- OpenAPI 字段总数: 17
- 资产使用字段数: 17
- 未使用字段数: 0
