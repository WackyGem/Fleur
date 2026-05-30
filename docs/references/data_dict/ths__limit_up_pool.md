# ths__limit_up_pool 字段校对

> 生成时间: 2026-05-30 10:31:25 UTC
> OpenAPI 文档: ths__limit_up_pool.yaml

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 |
|---|--------|-------------|---------|-------------|
| 1 | open_num | number | ✅ | int64 |
| 2 | first_limit_up_time | string | ✅ | timestamp[ns, tz=UTC] |
| 3 | last_limit_up_time | string | ✅ | timestamp[ns, tz=UTC] |
| 4 | code | string | ✅ | string |
| 5 | limit_up_type | string | ✅ | string |
| 6 | order_volume | number | ✅ | double |
| 7 | is_new | number | ✅ | bool |
| 8 | limit_up_suc_rate | number | ✅ | double |
| 9 | currency_value | number | ✅ | double |
| 10 | market_id | number | ✅ | int64 |
| 11 | is_again_limit | number | ✅ | bool |
| 12 | change_rate | number | ✅ | double |
| 13 | turnover_rate | number | ✅ | double |
| 14 | reason_type | string | ✅ | string |
| 15 | order_amount | number | ✅ | double |
| 16 | high_days | string | ✅ | string |
| 17 | name | string | ✅ | string |
| 18 | high_days_value | number | ✅ | int64 |
| 19 | change_tag | string | ✅ | string |
| 20 | market_type | string | ✅ | string |
| 21 | latest | number | ✅ | double |
| 22 | time_preview | array | ❌ | - |
| 23 | date | N/A | ✅ | date32[day] |

## 统计

- OpenAPI 字段总数: 22
- 资产使用字段数: 22
- 未使用字段数: 1
