# eastmoney__equity_history 字段校对

> 生成时间: 2026-05-30 10:31:25 UTC
> OpenAPI 文档: eastmoney__equity_history.yaml

## 字段对比

| # | 字段名 | OpenAPI 类型 | 资产使用 | PyArrow 类型 | ClickHouse 类型 |
|---|--------|-------------|---------|-------------|----------------|
| 1 | SECUCODE | string | ✅ | string | LowCardinality(String) |
| 2 | SECURITY_CODE | string | ✅ | string | LowCardinality(String) |
| 3 | ORG_CODE | string | ✅ | string | LowCardinality(String) |
| 4 | END_DATE | string | ✅ | date32[day] | Date |
| 5 | CHANGE_REASON | string | ✅ | string | LowCardinality(String) |
| 6 | LIMITED_SHARES | number | ✅ | double | Float64 |
| 7 | UNLIMITED_SHARES | number | ✅ | double | Float64 |
| 8 | TOTAL_SHARES | number | ✅ | double | Float64 |
| 9 | LIMITED_SHARES_RATIO | number | ✅ | double | Float64 |
| 10 | LISTED_SHARES_RATIO | number | ✅ | double | Float64 |
| 11 | TOTAL_SHARES_RATIO | string | ✅ | double | Float64 |
| 12 | LISTED_A_SHARES | number | ✅ | double | Float64 |
| 13 | LIMITED_A_SHARES | number | ✅ | double | Float64 |
| 14 | LISTED_A_SHARES_RATIO | number | ✅ | double | Float64 |
| 15 | LIMITED_A_SHARES_RATIO | number | ✅ | double | Float64 |
| 16 | B_FREE_SHARE | number | ✅ | double | Float64 |
| 17 | H_FREE_SHARE | number | ✅ | double | Float64 |
| 18 | B_FREE_SHARE_RATIO | number | ✅ | double | Float64 |
| 19 | H_FREE_SHARE_RATIO | number | ✅ | double | Float64 |
| 20 | SECURITY_TYPE_CODE | string | ✅ | string | LowCardinality(String) |
| 21 | NON_FREE_SHARES | number | ✅ | double | Float64 |
| 22 | NON_FREESHARES_RATIO | number | ✅ | double | Float64 |
| 23 | LIMITED_B_SHARES | number | ✅ | double | Float64 |
| 24 | LIMITED_BSHARES_RATIO | number | ✅ | double | Float64 |
| 25 | OTHER_FREE_SHARES | number | ✅ | double | Float64 |
| 26 | OTHER_FREESHARES_RATIO | number | ✅ | double | Float64 |
| 27 | LIMITED_STATE_SHARES | number | ✅ | double | Float64 |
| 28 | LIMITED_STATE_LEGAL | number | ✅ | double | Float64 |
| 29 | LIMITED_OTHARS | number | ✅ | double | Float64 |
| 30 | LIMITED_DOMESTIC_NOSTATE | number | ✅ | double | Float64 |
| 31 | LIMITED_DOMESTIC_NATURAL | number | ✅ | double | Float64 |
| 32 | LOCK_SHARES | number | ✅ | double | Float64 |
| 33 | LIMITED_FOREIGN_SHARES | number | ✅ | double | Float64 |
| 34 | LIMITED_OVERSEAS_NOSTATE | number | ✅ | double | Float64 |
| 35 | LIMITED_OVERSEAS_NATURAL | number | ✅ | double | Float64 |
| 36 | LIMITED_H_SHARES | number | ✅ | double | Float64 |
| 37 | SPONSOR_SHARES | number | ✅ | double | Float64 |
| 38 | STATE_SPONSOR_SHARES | number | ✅ | double | Float64 |
| 39 | SPONSOR_SOCIAL_SHARES | number | ✅ | double | Float64 |
| 40 | RAISE_SHARES | number | ✅ | double | Float64 |
| 41 | RAISE_STATE_SHARES | number | ✅ | double | Float64 |
| 42 | RAISE_DOMESTIC_SHARES | number | ✅ | double | Float64 |
| 43 | RAISE_OVERSEAS_SHARES | number | ✅ | double | Float64 |
| 44 | NOTICE_DATE | string | ✅ | date32[day] | Date |
| 45 | LISTING_DATE | string | ✅ | date32[day] | Date |
| 46 | LIMITED_SHARES_CHANGE | number | ✅ | double | Float64 |
| 47 | UNLIMITED_SHARES_CHANGE | number | ✅ | double | Float64 |
| 48 | TOTAL_SHARES_CHANGE | number | ✅ | double | Float64 |
| 49 | LISTED_ASHARES_CHANGE | number | ✅ | double | Float64 |
| 50 | LIMITED_ASHARES_CHANGE | number | ✅ | double | Float64 |
| 51 | B_FREESHARE_CHANGE | number | ✅ | double | Float64 |
| 52 | H_FREESHARE_CHANGE | number | ✅ | double | Float64 |
| 53 | LIMITED_BSHARES_CHANGE | number | ✅ | double | Float64 |
| 54 | NONFREE_SHARES_CHANGE | number | ✅ | double | Float64 |
| 55 | OTHERFREE_SHARES_CHANGE | number | ✅ | double | Float64 |
| 56 | FREE_SHARES | number | ✅ | double | Float64 |
| 57 | CHANGE_REASON_EXPLAIN | string | ✅ | string | LowCardinality(String) |
| 58 | LIMITED_H_SHARES_RATIO | number | ✅ | double | Float64 |
| 59 | LIMITED_H_SHARES_CHANGE | number | ✅ | double | Float64 |
| 60 | IS_FREE_WINDOW | string | ✅ | bool | Bool |
| 61 | IS_LIMITED_WINDOW | string | ✅ | bool | Bool |
| 62 | LISTED_A_RATIOPC | number | ✅ | double | Float64 |
| 63 | LISTED_B_RATIOPC | number | ✅ | double | Float64 |
| 64 | LISTED_H_RATIOPC | number | ✅ | double | Float64 |
| 65 | LISTED_OTHER_RATIOPC | number | ✅ | double | Float64 |
| 66 | LISTED_SUM_RATIOPC | number | ✅ | double | Float64 |
| 67 | MARKET_CODE | string | ✅ | string | LowCardinality(String) |
| 68 | IS_USE | string | ✅ | bool | Bool |
| 69 | SECURITY_NAME_ABBR | string | ✅ | string | LowCardinality(String) |

## 统计

- OpenAPI 字段总数: 69
- 资产使用字段数: 69
- 未使用字段数: 0
