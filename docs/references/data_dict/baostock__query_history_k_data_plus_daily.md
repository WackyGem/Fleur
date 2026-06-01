# baostock__query_history_k_data_plus_daily 字段校对

> 生成时间: 2026-06-01 00:00:00 UTC
> OpenAPI 文档: BaoStock query_history_k_data_plus

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | date | string | ✅ | date32[day] | Date |
| 2 | code | string | ✅ | string | LowCardinality(String) |
| 3 | open | string | ✅ | double | Float64 |
| 4 | high | string | ✅ | double | Float64 |
| 5 | low | string | ✅ | double | Float64 |
| 6 | close | string | ✅ | double | Float64 |
| 7 | preclose | string | ✅ | double | Float64 |
| 8 | volume | string | ✅ | int64 | Int64 |
| 9 | amount | string | ✅ | double | Float64 |
| 10 | adjustflag | string | ✅ | int8 | Int8 |
| 11 | turn | string | ✅ | double | Float64 |
| 12 | tradestatus | string | ✅ | int8 | Int8 |
| 13 | pctChg | string | ✅ | double | Float64 |
| 14 | isST | string | ✅ | int8 | Int8 |

## ClickHouse raw 设计记录

- 初始查询模式：dbt staging/marts 主要按证券代码 `code` 和交易日期 `date` 过滤。
- 初始 `ORDER BY`：`(code, date)`。
- `code` 预计为 A 股证券代码，基数低于 10,000，首批按 `LowCardinality(String)` 接入；首次环境 smoke test 需用 staging 表 `uniq(code)` 记录验证结果。

## 统计

- OpenAPI 字段总数: 14
- 资产使用字段数: 14
- 未使用字段数: 0

