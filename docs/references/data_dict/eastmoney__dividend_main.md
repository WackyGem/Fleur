# eastmoney__dividend_main 字段校对

> 生成时间: 2026-05-30 10:31:25 UTC
> OpenAPI 文档: eastmoney__dividend_main.yaml

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | SECUCODE | string | ✅ | string | LowCardinality(String) |
| 2 | SECURITY_CODE | string | ✅ | string | LowCardinality(String) |
| 3 | SECURITY_NAME_ABBR | string | ✅ | string | LowCardinality(String) |
| 4 | NOTICE_DATE | string | ✅ | date32[day] | Date |
| 5 | IMPL_PLAN_PROFILE | string | ✅ | string | LowCardinality(String) |
| 6 | ASSIGN_PROGRESS | string | ✅ | string | LowCardinality(String) |
| 7 | EQUITY_RECORD_DATE | number | ✅ | date32[day] | Date |
| 8 | EX_DIVIDEND_DATE | number | ✅ | date32[day] | Date |
| 9 | PAY_CASH_DATE | number | ✅ | date32[day] | Date |
| 10 | IS_UNASSIGN | string | ✅ | bool | Bool |
| 11 | REPORT_DATE | string | ✅ | string | LowCardinality(String) |
| 12 | ASSIGN_OBJECT | string | ✅ | string | LowCardinality(String) |
| 13 | IMPL_PLAN_NEWPROFILE | string | ✅ | string | LowCardinality(String) |
| 14 | NEW_PROFILE | string | ✅ | string | LowCardinality(String) |
| 15 | GMDECISION_NOTICE_DATE | number | ✅ | date32[day] | Date |
| 16 | INFO_CODE | string | ✅ | string | LowCardinality(String) |
| 17 | DAT_YAGGR | string | ✅ | date32[day] | Date |
| 18 | TOTAL_DIVIDEND | number | ✅ | double | Float64 |
| 19 | TOTAL_DIVIDEND_A | number | ✅ | double | Float64 |
| 20 | REPORT_TIME | string | ✅ | date32[day] | Date |
| 21 | DAT_YAGGR_TODAY | string | ✅ | bool | Bool |
| 22 | NOTICE_TODAY | string | ✅ | bool | Bool |
| 23 | GMDECISION_TODAY | string | ✅ | bool | Bool |
| 24 | DIRECTORSUPERVISOR_TODAY | string | ✅ | bool | Bool |
| 25 | EQUITY_TODAY | string | ✅ | bool | Bool |
| 26 | EX_DIVIDEND_TODAY | string | ✅ | bool | Bool |
| 27 | PAYCASH_TODAY | string | ✅ | bool | Bool |
| 28 | IS_PAYCASH | string | ✅ | bool | Bool |
| 29 | IS_EQUITY_RECENT | string | ✅ | bool | Bool |
| 30 | LAST_TRADE_DATE | number | ✅ | date32[day] | Date |

## 统计

- OpenAPI 字段总数: 30
- 资产使用字段数: 30
- 未使用字段数: 0
