# ths__limit_up_pool 字段校对

> 生成时间: 2026-05-30 10:31:25 UTC
> OpenAPI 文档: ths__limit_up_pool.yaml

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | open_num | number | ✅ | int64 | Int64 |
| 2 | first_limit_up_time | string | ✅ | timestamp[ns, tz=UTC] | DateTime64(3, 'UTC') |
| 3 | last_limit_up_time | string | ✅ | timestamp[ns, tz=UTC] | DateTime64(3, 'UTC') |
| 4 | code | string | ✅ | string | LowCardinality(String) |
| 5 | limit_up_type | string | ✅ | string | LowCardinality(String) |
| 6 | order_volume | number | ✅ | double | Float64 |
| 7 | is_new | number | ✅ | bool | Bool |
| 8 | limit_up_suc_rate | number | ✅ | double | Float64 |
| 9 | currency_value | number | ✅ | double | Float64 |
| 10 | market_id | number | ✅ | int64 | Int64 |
| 11 | is_again_limit | number | ✅ | bool | Bool |
| 12 | change_rate | number | ✅ | double | Float64 |
| 13 | turnover_rate | number | ✅ | double | Float64 |
| 14 | reason_type | string | ✅ | string | LowCardinality(String) |
| 15 | order_amount | number | ✅ | double | Float64 |
| 16 | high_days | string | ✅ | string | LowCardinality(String) |
| 17 | name | string | ✅ | string | LowCardinality(String) |
| 18 | high_days_value | number | ✅ | int64 | Int64 |
| 19 | change_tag | string | ✅ | string | LowCardinality(String) |
| 20 | market_type | string | ✅ | string | LowCardinality(String) |
| 21 | latest | number | ✅ | double | Float64 |
| 22 | time_preview | array | ❌ | - | - |
| 23 | date | N/A | ✅ | date32[day] | Date |

## 统计

- OpenAPI 字段总数: 22
- 资产使用字段数: 22
- 未使用字段数: 1
