# eastmoney__dividend_main 数据字典

本文件由 `pipeline/contracts/datasets/eastmoney__dividend_main.yml` 生成。字段事实以 contract 为准。

- 数据集：`eastmoney__dividend_main`
- 版本：`1`
- 说明：东方财富分红主表 F10 年度 raw 分区
- 粒度：one row per security code per report date
- Source asset：`source/eastmoney__dividend_main`
- Raw asset：`clickhouse/raw/eastmoney__dividend_main`
- ClickHouse raw：`fleur_raw.eastmoney__dividend_main`
- 分区策略：`year`
- ORDER BY：`(SECUCODE, REPORT_DATE)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `SECUCODE` | `string` | `string` | `SECUCODE` | `LowCardinality(String)` | 证券代码（含市场后缀） |
| 2 | `SECURITY_CODE` | `string` | `string` | `SECURITY_CODE` | `LowCardinality(String)` | 证券代码（纯数字） |
| 3 | `SECURITY_NAME_ABBR` | `string` | `string` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | 证券简称 |
| 4 | `NOTICE_DATE` | `string` | `date32[day]` | `NOTICE_DATE` | `Date` | 公告日期 |
| 5 | `IMPL_PLAN_PROFILE` | `string` | `string` | `IMPL_PLAN_PROFILE` | `LowCardinality(String)` | 分红方案简述 |
| 6 | `ASSIGN_PROGRESS` | `string` | `string` | `ASSIGN_PROGRESS` | `LowCardinality(String)` | 分配进度 |
| 7 | `EQUITY_RECORD_DATE` | `string` | `date32[day]` | `EQUITY_RECORD_DATE` | `Date` | 股权登记日 |
| 8 | `EX_DIVIDEND_DATE` | `string` | `date32[day]` | `EX_DIVIDEND_DATE` | `Date` | 除权除息日 |
| 9 | `PAY_CASH_DATE` | `string` | `date32[day]` | `PAY_CASH_DATE` | `Date` | 派息日 |
| 10 | `IS_UNASSIGN` | `string` | `bool` | `IS_UNASSIGN` | `Bool` | 是否不分配："0" 否，"1" 是 |
| 11 | `REPORT_DATE` | `string` | `string` | `REPORT_DATE` | `LowCardinality(String)` | 报告期 |
| 12 | `ASSIGN_OBJECT` | `string` | `string` | `ASSIGN_OBJECT` | `LowCardinality(String)` | 分配对象 |
| 13 | `IMPL_PLAN_NEWPROFILE` | `string` | `string` | `IMPL_PLAN_NEWPROFILE` | `LowCardinality(String)` | 方案简介 + 进度后缀 |
| 14 | `NEW_PROFILE` | `string` | `string` | `NEW_PROFILE` | `LowCardinality(String)` | 分红方案（含税） |
| 15 | `GMDECISION_NOTICE_DATE` | `string` | `date32[day]` | `GMDECISION_NOTICE_DATE` | `Date` | 股东大会决议公告日 |
| 16 | `INFO_CODE` | `string` | `string` | `INFO_CODE` | `String` | 公告编号 |
| 17 | `DAT_YAGGR` | `string` | `date32[day]` | `DAT_YAGGR` | `Date` | 年度股东大会日期 |
| 18 | `TOTAL_DIVIDEND` | `number` | `double` | `TOTAL_DIVIDEND` | `Float64` | 分红总额（元） |
| 19 | `TOTAL_DIVIDEND_A` | `number` | `double` | `TOTAL_DIVIDEND_A` | `Float64` | A股分红总额（元） |
| 20 | `REPORT_TIME` | `string` | `string` | `REPORT_TIME` | `String` | 报告期截止日 |
| 21 | `DAT_YAGGR_TODAY` | `string` | `bool` | `DAT_YAGGR_TODAY` | `Bool` | 是否今日年度股东大会 |
| 22 | `NOTICE_TODAY` | `string` | `bool` | `NOTICE_TODAY` | `Bool` | 是否今日公告 |
| 23 | `GMDECISION_TODAY` | `string` | `bool` | `GMDECISION_TODAY` | `Bool` | 是否今日股东大会决议 |
| 24 | `DIRECTORSUPERVISOR_TODAY` | `string` | `bool` | `DIRECTORSUPERVISOR_TODAY` | `Bool` | 是否今日监事会决议 |
| 25 | `EQUITY_TODAY` | `string` | `bool` | `EQUITY_TODAY` | `Bool` | 是否今日股权登记 |
| 26 | `EX_DIVIDEND_TODAY` | `string` | `bool` | `EX_DIVIDEND_TODAY` | `Bool` | 是否今日除权除息 |
| 27 | `PAYCASH_TODAY` | `string` | `bool` | `PAYCASH_TODAY` | `Bool` | 是否今日派息 |
| 28 | `IS_PAYCASH` | `string` | `bool` | `IS_PAYCASH` | `Bool` | 是否派息 |
| 29 | `IS_EQUITY_RECENT` | `string` | `bool` | `IS_EQUITY_RECENT` | `Bool` | 是否近期股权登记 |
| 30 | `LAST_TRADE_DATE` | `string` | `date32[day]` | `LAST_TRADE_DATE` | `Date` | 最后交易日 |

## 数据集备注

东方财富分红主表 F10 年度 raw 分区

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
- REPORT_TIME intentionally remains string because historical responses can contain labels such as 1991年报 rather than ISO date strings.
- LowCardinality review from EastMoney dividend_main 2025 notice-date first page sample: INFO_CODE nonnull=500 uniq=497 unique_rate=0.994000. INFO_CODE is a high-cardinality announcement identifier and uses ClickHouse String. Parquet schema remains string.
