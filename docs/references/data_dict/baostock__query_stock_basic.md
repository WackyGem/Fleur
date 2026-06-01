# baostock__query_stock_basic 字段校对

> 生成时间: 2026-06-01 00:00:00 UTC
> OpenAPI 文档: BaoStock query_stock_basic

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | code | string | ✅ | string | LowCardinality(String) |
| 2 | code_name | string | ✅ | string | LowCardinality(String) |
| 3 | ipoDate | string | ✅ | date32[day] | Date |
| 4 | outDate | string | ✅ | date32[day] | Date |
| 5 | type | string | ✅ | int8 | Int8 |
| 6 | status | string | ✅ | int8 | Int8 |

## 统计

- OpenAPI 字段总数: 6
- 资产使用字段数: 6
- 未使用字段数: 0

