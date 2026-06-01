# eastmoney__dividend_allotment 字段校对

> 生成时间: 2026-05-30 10:31:25 UTC
> OpenAPI 文档: eastmoney__dividend_allotment.yaml

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | SECUCODE | string | ✅ | string | LowCardinality(String) |
| 2 | SECURITY_CODE | string | ✅ | string | LowCardinality(String) |
| 3 | SECURITY_NAME_ABBR | string | ✅ | string | LowCardinality(String) |
| 4 | NOTICE_DATE | string | ✅ | date32[day] | Date |
| 5 | ISSUE_NUM | number | ✅ | double | Float64 |
| 6 | TOTAL_RAISE_FUNDS | number | ✅ | double | Float64 |
| 7 | ISSUE_PRICE | number | ✅ | double | Float64 |
| 8 | EQUITY_RECORD_DATE | string | ✅ | date32[day] | Date |
| 9 | EX_DIVIDEND_DATEE | string | ✅ | string | LowCardinality(String) |
| 10 | EVENT_EXPLAIN | string | ✅ | string | LowCardinality(String) |

## 统计

- OpenAPI 字段总数: 10
- 资产使用字段数: 10
- 未使用字段数: 0
