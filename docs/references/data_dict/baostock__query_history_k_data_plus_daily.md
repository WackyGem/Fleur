# baostock__query_history_k_data_plus_daily 数据字典

本文件由 `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml` 生成。字段事实以 contract 为准。

- 数据集：`baostock__query_history_k_data_plus_daily`
- 版本：`1`
- 说明：BaoStock 日频行情数据
- 粒度：one row per stock code per trade date
- Source asset：`source/baostock__query_history_k_data_plus_daily`
- Raw asset：`clickhouse/raw/baostock__query_history_k_data_plus_daily`
- ClickHouse raw：`raw.baostock__query_history_k_data_plus_daily`
- 分区策略：`year`
- ORDER BY：`(code, date)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | stg 字段 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|----------|
| 1 | `date` | `string` | `date32[day]` | `date` | `Date` | `trade_date` | A 股市场交易日日期。 |
| 2 | `code` | `string` | `string` | `code` | `LowCardinality(String)` | `code` | 证券、行业或业务对象在来源系统中的编码。 |
| 3 | `open` | `string` | `double` | `open` | `Float64` | `open` | 交易标的在交易日开盘时的价格。 |
| 4 | `high` | `string` | `double` | `high` | `Float64` | `high` | 交易标的在交易日内达到的最高价格。 |
| 5 | `low` | `string` | `double` | `low` | `Float64` | `low` | 交易标的在交易日内达到的最低价格。 |
| 6 | `close` | `string` | `double` | `close` | `Float64` | `close` | 交易标的在交易日收盘时的价格。 |
| 7 | `preclose` | `string` | `double` | `preclose` | `Float64` | `preclose` | 交易标的上一交易日的收盘价格。 |
| 8 | `volume` | `string` | `int64` | `volume` | `Int64` | `volume` | 交易日内成交数量或成交量。 |
| 9 | `amount` | `string` | `double` | `amount` | `Float64` | `amount` | 交易日内成交金额，通常以人民币计价。 |
| 10 | `adjustflag` | `string` | `int8` | `adjustflag` | `Int8` | `adjustflag` | 行情价格的复权处理标记，用于区分不复权、前复权和后复权口径。 |
| 11 | `turn` | `string` | `double` | `turn` | `Float64` | `turn` | 交易日换手率。 |
| 12 | `tradestatus` | `string` | `int8` | `tradestatus` | `Int8` | `tradestatus` | 证券在交易日内的交易状态。 |
| 13 | `pctChg` | `string` | `double` | `pctChg` | `Float64` | `pct_chg` | 交易标的相对上一交易日的涨跌幅比例。 |
| 14 | `isST` | `string` | `int8` | `isST` | `Int8` | `is_st` | 证券是否处于 ST 或风险警示状态。 |

## 数据集备注

BaoStock 日频行情数据

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
