# baostock__query_history_k_data_plus_daily 数据字典

本文件由 `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml` 生成。字段事实以 contract 为准。

- 数据集：`baostock__query_history_k_data_plus_daily`
- 版本：`1`
- 说明：BaoStock 日频行情每日 source 分区
- 粒度：one row per stock code per trade date
- Source asset：`source/baostock__query_history_k_data_plus_daily`
- Raw asset：不适用
- ClickHouse raw：不适用

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | 中文描述 |
|---|----------|----------|--------------|----------|
| 1 | `date` | `string` | `date32[day]` | BaoStock 行情接口返回的交易日期。 |
| 2 | `code` | `string` | `string` | BaoStock 行情接口返回的证券代码。 |
| 3 | `open` | `string` | `double` | 交易日开盘价。 |
| 4 | `high` | `string` | `double` | 交易日最高价。 |
| 5 | `low` | `string` | `double` | 交易日最低价。 |
| 6 | `close` | `string` | `double` | 交易日收盘价。 |
| 7 | `preclose` | `string` | `double` | 上一交易日收盘价。 |
| 8 | `volume` | `string` | `int64` | 交易日成交量。 |
| 9 | `amount` | `string` | `double` | 交易日成交金额。 |
| 10 | `adjustflag` | `string` | `int8` | 行情复权标记，用于区分不复权、前复权和后复权。 |
| 11 | `turn` | `string` | `double` | 交易日换手率。 |
| 12 | `tradestatus` | `string` | `int8` | 交易日交易状态。 |
| 13 | `pctChg` | `string` | `double` | 交易日涨跌幅。 |
| 14 | `isST` | `string` | `bool` | 证券是否为 ST 或风险警示状态。 |

## 数据集备注

BaoStock 日频行情每日增量 source 分区；ClickHouse raw 同步由年度 compacted 数据集承载。

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
