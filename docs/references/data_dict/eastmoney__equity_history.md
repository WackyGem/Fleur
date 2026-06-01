# eastmoney__equity_history 数据字典

本文件由 `pipeline/contracts/datasets/eastmoney__equity_history.yml` 生成。字段事实以 contract 为准。

- 数据集：`eastmoney__equity_history`
- 版本：`1`
- 说明：东方财富股本变动历史 F10 年度 raw 分区
- 粒度：one row per security code per report date
- Source asset：`source/eastmoney__equity_history`
- Raw asset：`clickhouse/raw/eastmoney__equity_history`
- ClickHouse raw：`raw.eastmoney__equity_history`
- 分区策略：`year`
- ORDER BY：`(SECUCODE, END_DATE)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | stg 字段 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|----------|
| 1 | `SECUCODE` | `string` | `string` | `SECUCODE` | `LowCardinality(String)` | `-` | 证券代码（含市场后缀） |
| 2 | `SECURITY_CODE` | `string` | `string` | `SECURITY_CODE` | `LowCardinality(String)` | `-` | 证券代码（纯数字） |
| 3 | `ORG_CODE` | `string` | `string` | `ORG_CODE` | `LowCardinality(String)` | `-` | 机构代码 |
| 4 | `END_DATE` | `string` | `date32[day]` | `END_DATE` | `Date` | `-` | 股本变动截止日 |
| 5 | `CHANGE_REASON` | `string` | `string` | `CHANGE_REASON` | `LowCardinality(String)` | `-` | 变动原因 |
| 6 | `LIMITED_SHARES` | `number` | `double` | `LIMITED_SHARES` | `Float64` | `-` | 有限售条件股份 |
| 7 | `UNLIMITED_SHARES` | `number` | `double` | `UNLIMITED_SHARES` | `Float64` | `-` | 无限售条件股份（已流通） |
| 8 | `TOTAL_SHARES` | `number` | `double` | `TOTAL_SHARES` | `Float64` | `-` | 总股本 |
| 9 | `LIMITED_SHARES_RATIO` | `number` | `double` | `LIMITED_SHARES_RATIO` | `Float64` | `-` | 限售股比例（%） |
| 10 | `LISTED_SHARES_RATIO` | `number` | `double` | `LISTED_SHARES_RATIO` | `Float64` | `-` | 已流通股比例（%） |
| 11 | `TOTAL_SHARES_RATIO` | `string` | `double` | `TOTAL_SHARES_RATIO` | `Float64` | `-` | 总股本比例（%） |
| 12 | `LISTED_A_SHARES` | `number` | `double` | `LISTED_A_SHARES` | `Float64` | `-` | 已上市流通 A 股 |
| 13 | `LIMITED_A_SHARES` | `number` | `double` | `LIMITED_A_SHARES` | `Float64` | `-` | 限售 A 股 |
| 14 | `LISTED_A_SHARES_RATIO` | `number` | `double` | `LISTED_A_SHARES_RATIO` | `Float64` | `-` | A 股流通比例（%） |
| 15 | `LIMITED_A_SHARES_RATIO` | `number` | `double` | `LIMITED_A_SHARES_RATIO` | `Float64` | `-` | 限售A股比例（%） |
| 16 | `B_FREE_SHARE` | `number` | `double` | `B_FREE_SHARE` | `Float64` | `-` | 已上市流通 B 股 |
| 17 | `H_FREE_SHARE` | `number` | `double` | `H_FREE_SHARE` | `Float64` | `-` | 已上市流通 H 股 |
| 18 | `B_FREE_SHARE_RATIO` | `number` | `double` | `B_FREE_SHARE_RATIO` | `Float64` | `-` | B股流通比例（%） |
| 19 | `H_FREE_SHARE_RATIO` | `number` | `double` | `H_FREE_SHARE_RATIO` | `Float64` | `-` | H 股流通比例（%） |
| 20 | `SECURITY_TYPE_CODE` | `string` | `string` | `SECURITY_TYPE_CODE` | `LowCardinality(String)` | `-` | 证券类型代码 |
| 21 | `NON_FREE_SHARES` | `number` | `double` | `NON_FREE_SHARES` | `Float64` | `-` | 非自由流通股 |
| 22 | `NON_FREESHARES_RATIO` | `number` | `double` | `NON_FREESHARES_RATIO` | `Float64` | `-` | 非流通股比例（%） |
| 23 | `LIMITED_B_SHARES` | `number` | `double` | `LIMITED_B_SHARES` | `Float64` | `-` | 限售 B 股 |
| 24 | `LIMITED_BSHARES_RATIO` | `number` | `double` | `LIMITED_BSHARES_RATIO` | `Float64` | `-` | 限售B股比例（%） |
| 25 | `OTHER_FREE_SHARES` | `number` | `double` | `OTHER_FREE_SHARES` | `Float64` | `-` | 其他已上市流通股 |
| 26 | `OTHER_FREESHARES_RATIO` | `number` | `double` | `OTHER_FREESHARES_RATIO` | `Float64` | `-` | 其他流通股比例（%） |
| 27 | `LIMITED_STATE_SHARES` | `number` | `double` | `LIMITED_STATE_SHARES` | `Float64` | `-` | 国家持股（限售） |
| 28 | `LIMITED_STATE_LEGAL` | `number` | `double` | `LIMITED_STATE_LEGAL` | `Float64` | `-` | 国有法人持股（限售） |
| 29 | `LIMITED_OTHARS` | `number` | `double` | `LIMITED_OTHARS` | `Float64` | `-` | 其他限售股份 |
| 30 | `LIMITED_DOMESTIC_NOSTATE` | `number` | `double` | `LIMITED_DOMESTIC_NOSTATE` | `Float64` | `-` | 境内非国有法人持股（限售） |
| 31 | `LIMITED_DOMESTIC_NATURAL` | `number` | `double` | `LIMITED_DOMESTIC_NATURAL` | `Float64` | `-` | 境内自然人持股（限售） |
| 32 | `LOCK_SHARES` | `number` | `double` | `LOCK_SHARES` | `Float64` | `-` | 锁定股份 |
| 33 | `LIMITED_FOREIGN_SHARES` | `number` | `double` | `LIMITED_FOREIGN_SHARES` | `Float64` | `-` | 外资持股（限售） |
| 34 | `LIMITED_OVERSEAS_NOSTATE` | `number` | `double` | `LIMITED_OVERSEAS_NOSTATE` | `Float64` | `-` | 境外非国有法人持股（限售） |
| 35 | `LIMITED_OVERSEAS_NATURAL` | `number` | `double` | `LIMITED_OVERSEAS_NATURAL` | `Float64` | `-` | 境外自然人持股（限售） |
| 36 | `LIMITED_H_SHARES` | `number` | `double` | `LIMITED_H_SHARES` | `Float64` | `-` | 限售 H 股 |
| 37 | `SPONSOR_SHARES` | `number` | `double` | `SPONSOR_SHARES` | `Float64` | `-` | 发起人股份 |
| 38 | `STATE_SPONSOR_SHARES` | `number` | `double` | `STATE_SPONSOR_SHARES` | `Float64` | `-` | 国家发起人股份 |
| 39 | `SPONSOR_SOCIAL_SHARES` | `number` | `double` | `SPONSOR_SOCIAL_SHARES` | `Float64` | `-` | 社会发起人股份 |
| 40 | `RAISE_SHARES` | `number` | `double` | `RAISE_SHARES` | `Float64` | `-` | 募集法人股份 |
| 41 | `RAISE_STATE_SHARES` | `number` | `double` | `RAISE_STATE_SHARES` | `Float64` | `-` | 国家募集法人股份 |
| 42 | `RAISE_DOMESTIC_SHARES` | `number` | `double` | `RAISE_DOMESTIC_SHARES` | `Float64` | `-` | 境内募集法人股份 |
| 43 | `RAISE_OVERSEAS_SHARES` | `number` | `double` | `RAISE_OVERSEAS_SHARES` | `Float64` | `-` | 境外募集法人股份 |
| 44 | `NOTICE_DATE` | `string` | `date32[day]` | `NOTICE_DATE` | `Date` | `-` | 公告披露日 |
| 45 | `LISTING_DATE` | `string` | `date32[day]` | `LISTING_DATE` | `Date` | `-` | 上市流通日期 |
| 46 | `LIMITED_SHARES_CHANGE` | `number` | `double` | `LIMITED_SHARES_CHANGE` | `Float64` | `-` | 限售股变动量 |
| 47 | `UNLIMITED_SHARES_CHANGE` | `number` | `double` | `UNLIMITED_SHARES_CHANGE` | `Float64` | `-` | 流通股变动量 |
| 48 | `TOTAL_SHARES_CHANGE` | `number` | `double` | `TOTAL_SHARES_CHANGE` | `Float64` | `-` | 总股本变动量 |
| 49 | `LISTED_ASHARES_CHANGE` | `number` | `double` | `LISTED_ASHARES_CHANGE` | `Float64` | `-` | 已上市流通A股变动量 |
| 50 | `LIMITED_ASHARES_CHANGE` | `number` | `double` | `LIMITED_ASHARES_CHANGE` | `Float64` | `-` | 限售A股变动量 |
| 51 | `B_FREESHARE_CHANGE` | `number` | `double` | `B_FREESHARE_CHANGE` | `Float64` | `-` | B股流通变动量 |
| 52 | `H_FREESHARE_CHANGE` | `number` | `double` | `H_FREESHARE_CHANGE` | `Float64` | `-` | H股流通变动量 |
| 53 | `LIMITED_BSHARES_CHANGE` | `number` | `double` | `LIMITED_BSHARES_CHANGE` | `Float64` | `-` | 限售B股变动量 |
| 54 | `NONFREE_SHARES_CHANGE` | `number` | `double` | `NONFREE_SHARES_CHANGE` | `Float64` | `-` | 非流通股变动量 |
| 55 | `OTHERFREE_SHARES_CHANGE` | `number` | `double` | `OTHERFREE_SHARES_CHANGE` | `Float64` | `-` | 其他流通股变动量 |
| 56 | `FREE_SHARES` | `number` | `double` | `FREE_SHARES` | `Float64` | `-` | 流通股（通常 = TOTAL_SHARES） |
| 57 | `CHANGE_REASON_EXPLAIN` | `string` | `string` | `CHANGE_REASON_EXPLAIN` | `LowCardinality(String)` | `-` | 变动原因详细说明 |
| 58 | `LIMITED_H_SHARES_RATIO` | `number` | `double` | `LIMITED_H_SHARES_RATIO` | `Float64` | `-` | 限售H股比例（%） |
| 59 | `LIMITED_H_SHARES_CHANGE` | `number` | `double` | `LIMITED_H_SHARES_CHANGE` | `Float64` | `-` | 限售H股变动量 |
| 60 | `IS_FREE_WINDOW` | `string` | `bool` | `IS_FREE_WINDOW` | `Bool` | `-` | 是否为自由流通窗口 |
| 61 | `IS_LIMITED_WINDOW` | `string` | `bool` | `IS_LIMITED_WINDOW` | `Bool` | `-` | 是否限售窗口 |
| 62 | `LISTED_A_RATIOPC` | `number` | `double` | `LISTED_A_RATIOPC` | `Float64` | `-` | A 股占已流通比例（%） |
| 63 | `LISTED_B_RATIOPC` | `number` | `double` | `LISTED_B_RATIOPC` | `Float64` | `-` | B股占已流通比例（%） |
| 64 | `LISTED_H_RATIOPC` | `number` | `double` | `LISTED_H_RATIOPC` | `Float64` | `-` | H 股占已流通比例（%） |
| 65 | `LISTED_OTHER_RATIOPC` | `number` | `double` | `LISTED_OTHER_RATIOPC` | `Float64` | `-` | 其他占已流通比例（%） |
| 66 | `LISTED_SUM_RATIOPC` | `number` | `double` | `LISTED_SUM_RATIOPC` | `Float64` | `-` | 合计占已流通比例（%） |
| 67 | `MARKET_CODE` | `string` | `string` | `MARKET_CODE` | `LowCardinality(String)` | `-` | 市场代码 |
| 68 | `IS_USE` | `string` | `bool` | `IS_USE` | `Bool` | `-` | 是否有效 |
| 69 | `SECURITY_NAME_ABBR` | `string` | `string` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | `-` | 证券简称 |

## 数据集备注

东方财富股本变动历史 F10 年度 raw 分区

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
