# baostock__query_history_k_data_plus_daily_compacted 数据字典

本文件由 `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily_compacted.yml` 生成。字段事实以 contract 为准。

- 数据集：`baostock__query_history_k_data_plus_daily_compacted`
- 版本：`1`
- 说明：BaoStock 日频行情每日数据年度合并 raw 分区
- 粒度：one row per stock code per trade date
- Source asset：`source/baostock__query_history_k_data_plus_daily_compacted`
- Raw asset：`clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`
- ClickHouse raw：`fleur_raw.baostock__query_history_k_data_plus_daily_compacted`
- 分区策略：`year`
- ORDER BY：`(date, code)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `date` | `string` | `date32[day]` | `date` | `Date` | BaoStock 行情接口返回的交易日期。 |
| 2 | `code` | `string` | `string` | `code` | `LowCardinality(String)` | BaoStock 行情接口返回的证券代码。 |
| 3 | `open` | `string` | `double` | `open` | `Nullable(Float64)` | 交易日开盘价。 |
| 4 | `high` | `string` | `double` | `high` | `Nullable(Float64)` | 交易日最高价。 |
| 5 | `low` | `string` | `double` | `low` | `Nullable(Float64)` | 交易日最低价。 |
| 6 | `close` | `string` | `double` | `close` | `Nullable(Float64)` | 交易日收盘价。 |
| 7 | `preclose` | `string` | `double` | `preclose` | `Nullable(Float64)` | 上一交易日收盘价。 |
| 8 | `volume` | `string` | `int64` | `volume` | `Nullable(Int64)` | 交易日成交量。 |
| 9 | `amount` | `string` | `double` | `amount` | `Nullable(Float64)` | 交易日成交金额。 |
| 10 | `adjustflag` | `string` | `int8` | `adjustflag` | `Int8` | 行情复权标记，用于区分不复权、前复权和后复权。 |
| 11 | `turn` | `string` | `double` | `turn` | `Nullable(Float64)` | 交易日换手率。 |
| 12 | `tradestatus` | `string` | `int8` | `tradestatus` | `Int8` | 交易日交易状态。 |
| 13 | `pctChg` | `string` | `double` | `pctChg` | `Nullable(Float64)` | 交易日涨跌幅。 |
| 14 | `isST` | `string` | `bool` | `isST` | `Nullable(Bool)` | 证券是否为 ST 或风险警示状态。 |

## 数据集备注

BaoStock 日频行情每日 source 分区年度合并结果，作为 ClickHouse raw 同步输入。

## 校验记录

- Split from baostock__query_history_k_data_plus_daily when daily source partitions moved to trade_date.
