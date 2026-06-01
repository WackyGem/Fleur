# sina__trade_calendar 数据字典

本文件由 `pipeline/contracts/datasets/sina__trade_calendar.yml` 生成。字段事实以 contract 为准。

- 数据集：`sina__trade_calendar`
- 版本：`1`
- 说明：新浪 A 股交易日历快照
- 粒度：one row per trade date
- Source asset：`source/sina__trade_calendar`
- Raw asset：`clickhouse/raw/sina__trade_calendar`
- ClickHouse raw：`raw.sina__trade_calendar`
- 分区策略：`snapshot`
- ORDER BY：`(trade_date)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `trade_date` | `N/A` | `date32[day]` | `trade_date` | `Date` | 新浪交易日历中的 A 股交易日期。 |

## 数据集备注

新浪 A 股交易日历快照

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
