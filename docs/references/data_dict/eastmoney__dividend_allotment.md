# eastmoney__dividend_allotment 数据字典

本文件由 `pipeline/contracts/datasets/eastmoney__dividend_allotment.yml` 生成。字段事实以 contract 为准。

- 数据集：`eastmoney__dividend_allotment`
- 版本：`1`
- 说明：东方财富分红配股 F10 年度 raw 分区
- 粒度：one row per security code per report date
- Source asset：`source/eastmoney__dividend_allotment`
- Raw asset：`clickhouse/raw/eastmoney__dividend_allotment`
- ClickHouse raw：`fleur_raw.eastmoney__dividend_allotment`
- 分区策略：`year`
- ORDER BY：`(SECUCODE, NOTICE_DATE)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `SECUCODE` | `string` | `string` | `SECUCODE` | `LowCardinality(String)` | 证券代码（含市场后缀） |
| 2 | `SECURITY_CODE` | `string` | `string` | `SECURITY_CODE` | `LowCardinality(String)` | 证券代码（纯数字） |
| 3 | `SECURITY_NAME_ABBR` | `string` | `string` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | 证券简称 |
| 4 | `NOTICE_DATE` | `string` | `date32[day]` | `NOTICE_DATE` | `Date` | 公告日期 |
| 5 | `ISSUE_NUM` | `number` | `double` | `ISSUE_NUM` | `Float64` | 配股数量 |
| 6 | `TOTAL_RAISE_FUNDS` | `number` | `double` | `TOTAL_RAISE_FUNDS` | `Float64` | 配股募集资金总额 |
| 7 | `ISSUE_PRICE` | `number` | `double` | `ISSUE_PRICE` | `Float64` | 配股价格 |
| 8 | `EQUITY_RECORD_DATE` | `string` | `date32[day]` | `EQUITY_RECORD_DATE` | `Date` | 股权登记日 |
| 9 | `EX_DIVIDEND_DATEE` | `string` | `date32[day]` | `EX_DIVIDEND_DATEE` | `Date` | 除权除息日 |
| 10 | `EVENT_EXPLAIN` | `string` | `string` | `EVENT_EXPLAIN` | `LowCardinality(String)` | 配股方案说明（如 "每10股配3股"） |

## 数据集备注

东方财富分红配股 F10 年度 raw 分区

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
