# ths__limit_up_pool_compacted 数据字典

本文件由 `pipeline/contracts/datasets/ths__limit_up_pool_compacted.yml` 生成。字段事实以 contract 为准。

- 数据集：`ths__limit_up_pool_compacted`
- 版本：`1`
- 说明：同花顺涨停池每日数据年度合并 raw 分区
- 粒度：one row per stock code per trade date
- Source asset：`source/ths__limit_up_pool_compacted`
- Raw asset：`clickhouse/raw/ths__limit_up_pool_compacted`
- ClickHouse raw：`raw.ths__limit_up_pool_compacted`
- 分区策略：`year`
- ORDER BY：`(date, code)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | stg 字段 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|----------|
| 1 | `date` | `string` | `date32[day]` | `date` | `Date` | `-` | 同花顺涨停池对应的交易日期。 |
| 2 | `open_num` | `integer` | `int64` | `open_num` | `Int64` | `-` | 股票当日涨停后开板次数。 |
| 3 | `first_limit_up_time` | `string` | `timestamp[ns, tz=UTC]` | `first_limit_up_time` | `DateTime64(3, 'UTC')` | `-` | 股票当日首次涨停时间。 |
| 4 | `last_limit_up_time` | `string` | `timestamp[ns, tz=UTC]` | `last_limit_up_time` | `DateTime64(3, 'UTC')` | `-` | 股票当日最后一次涨停时间。 |
| 5 | `code` | `string` | `string` | `code` | `LowCardinality(String)` | `-` | 同花顺涨停池中的证券代码。 |
| 6 | `limit_up_type` | `string` | `string` | `limit_up_type` | `LowCardinality(String)` | `-` | 涨停类型分类。 |
| 7 | `order_volume` | `number` | `double` | `order_volume` | `Float64` | `-` | 涨停封单量。 |
| 8 | `is_new` | `boolean` | `bool` | `is_new` | `Bool` | `-` | 是否为当日新进入涨停池的股票。 |
| 9 | `limit_up_suc_rate` | `number` | `double` | `limit_up_suc_rate` | `Float64` | `-` | 涨停成功率。 |
| 10 | `currency_value` | `number` | `double` | `currency_value` | `Float64` | `-` | 股票流通市值。 |
| 11 | `market_id` | `integer` | `int64` | `market_id` | `Int64` | `-` | 同花顺市场标识。 |
| 12 | `is_again_limit` | `boolean` | `bool` | `is_again_limit` | `Bool` | `-` | 是否再次涨停。 |
| 13 | `change_rate` | `number` | `double` | `change_rate` | `Float64` | `-` | 当日涨跌幅。 |
| 14 | `turnover_rate` | `number` | `double` | `turnover_rate` | `Float64` | `-` | 当日换手率。 |
| 15 | `reason_type` | `string` | `string` | `reason_type` | `LowCardinality(String)` | `-` | 涨停原因类型。 |
| 16 | `order_amount` | `number` | `double` | `order_amount` | `Float64` | `-` | 涨停封单金额。 |
| 17 | `high_days` | `string` | `string` | `high_days` | `LowCardinality(String)` | `-` | 连板或高度天数文本。 |
| 18 | `name` | `string` | `string` | `name` | `LowCardinality(String)` | `-` | 股票名称。 |
| 19 | `high_days_value` | `integer` | `int64` | `high_days_value` | `Int64` | `-` | 连板或高度天数数值。 |
| 20 | `change_tag` | `string` | `string` | `change_tag` | `LowCardinality(String)` | `-` | 涨跌幅标签。 |
| 21 | `market_type` | `string` | `string` | `market_type` | `LowCardinality(String)` | `-` | 市场类型。 |
| 22 | `latest` | `number` | `double` | `latest` | `Float64` | `-` | 最新成交价格。 |

## 数据集备注

同花顺涨停池每日数据年度合并 raw 分区

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
