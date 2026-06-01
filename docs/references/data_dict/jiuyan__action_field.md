# jiuyan__action_field 字段校对

> 生成时间: 2026-05-30 10:31:25 UTC
> OpenAPI 文档: jiuyan__action_field.yaml

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | action_field_id | string | ✅ | string | LowCardinality(String) |
| 2 | name | string | ✅ | string | LowCardinality(String) |
| 3 | date | string | ✅ | date32[day] | Date |
| 4 | reason | string | ✅ | string | LowCardinality(String) |
| 5 | status | integer | ❌ | - | - |
| 6 | sort_no | integer | ✅ | int64 | Int64 |
| 7 | is_delete | string | ✅ | bool | Bool |
| 8 | delete_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 9 | create_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 10 | update_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 11 | count | integer | ✅ | int64 | Int64 |
| 12 | list | array | ❌ | - | - |
| 13 | list.code | string | ✅ | string | LowCardinality(String) |
| 14 | list.name | string | ✅ | string | LowCardinality(String) |
| 15 | list.article | object | ❌ | - | - |
| 16 | list.article.article_id | string | ❌ | - | - |
| 17 | list.article.comment_count | integer | ❌ | - | - |
| 18 | list.article.like_count | integer | ❌ | - | - |
| 19 | list.article.create_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 20 | list.article.user_id | string | ❌ | - | - |
| 21 | list.article.is_like | integer | ❌ | - | - |
| 22 | list.article.action_info | object | ❌ | - | - |
| 23 | list.article.action_info.article_id | string | ❌ | - | - |
| 24 | list.article.action_info.action_info_id | string | ❌ | - | - |
| 25 | list.article.action_info.stock_id | string | ❌ | - | - |
| 26 | list.article.action_info.action_field_id | string | ✅ | string | LowCardinality(String) |
| 27 | list.article.action_info.time | string | ✅ | time32[ms] | String |
| 28 | list.article.action_info.num | string | ✅ | string | LowCardinality(String) |
| 29 | list.article.action_info.price | integer | ✅ | int64 | Int64 |
| 30 | list.article.action_info.day | integer | ✅ | int64 | Int64 |
| 31 | list.article.action_info.edition | integer | ✅ | int64 | Int64 |
| 32 | list.article.action_info.shares_range | number | ✅ | float64 | - |
| 33 | list.article.action_info.reason | string | ✅ | string | LowCardinality(String) |
| 34 | list.article.action_info.expound | string | ✅ | string | LowCardinality(String) |
| 35 | list.article.action_info.is_crawl | integer | ❌ | - | - |
| 36 | list.article.action_info.is_recommend | integer | ❌ | - | - |
| 37 | list.article.action_info.is_delete | string | ✅ | bool | Bool |
| 38 | list.article.action_info.delete_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 39 | list.article.action_info.create_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 40 | list.article.action_info.update_time | string | ✅ | timestamp[ns] | DateTime64(3) |
| 41 | list.article.action_info.sort_no | integer | ✅ | int64 | Int64 |
| 42 | list.article.forward_count | integer | ❌ | - | - |
| 43 | list.article.step_count | integer | ❌ | - | - |
| 44 | list.article.title | string | ❌ | - | - |
| 45 | list.article.is_step | integer | ❌ | - | - |
| 46 | list.article.user | object | ❌ | - | - |
| 47 | list.article.user.user_id | string | ❌ | - | - |
| 48 | list.article.user.avatar | string | ❌ | - | - |
| 49 | list.article.user.nickname | string | ❌ | - | - |
| 50 | code | string | ✅ | string | LowCardinality(String) |
| 51 | time | string | ✅ | time32[ms] | String |
| 52 | num | string | ✅ | string | LowCardinality(String) |
| 53 | price | integer | ✅ | int64 | Int64 |
| 54 | day | integer | ✅ | int64 | Int64 |
| 55 | edition | integer | ✅ | int64 | Int64 |
| 56 | shares_range | number | ✅ | float64 | - |
| 57 | expound | string | ✅ | string | String |

## 统计

- OpenAPI 字段总数: 49
- 资产使用字段数: 35
- 未使用字段数: 22
