# ths__limit_up_pool 数据字典

本文件由 `pipeline/contracts/datasets/ths__limit_up_pool.yml` 生成。字段事实以 contract 为准。

- 数据集：`ths__limit_up_pool`
- 版本：`1`
- 说明：同花顺涨停池每日 source 分区
- 粒度：one row per stock code per source trade date
- Source asset：`source/ths__limit_up_pool`
- Raw asset：不适用
- ClickHouse raw：不适用

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | 中文描述 |
|---|----------|----------|--------------|----------|
| 1 | `date` | `string` | `date32[day]` | 同花顺涨停池对应的交易日期。 |
| 2 | `open_num` | `number` | `int64` | 股票当日涨停后开板次数。 |
| 3 | `first_limit_up_time` | `string` | `timestamp[s, tz=UTC]` | 股票当日首次涨停时间。 |
| 4 | `last_limit_up_time` | `string` | `timestamp[s, tz=UTC]` | 股票当日最后一次涨停时间。 |
| 5 | `code` | `string` | `string` | 同花顺涨停池中的证券代码。 |
| 6 | `limit_up_type` | `string` | `string` | 同花顺涨停类型分类。 |
| 7 | `order_volume` | `number` | `double` | 涨停封单量。 |
| 8 | `is_new` | `number` | `bool` | 是否为当日新进入涨停池的股票。 |
| 9 | `limit_up_suc_rate` | `number` | `double` | 涨停成功率。 |
| 10 | `currency_value` | `number` | `double` | 股票流通市值。 |
| 11 | `market_id` | `number` | `int64` | 同花顺市场标识。 |
| 12 | `is_again_limit` | `number` | `bool` | 是否再次涨停。 |
| 13 | `change_rate` | `number` | `double` | 当日涨跌幅。 |
| 14 | `turnover_rate` | `number` | `double` | 当日换手率。 |
| 15 | `reason_type` | `string` | `string` | 涨停原因类型。 |
| 16 | `order_amount` | `number` | `double` | 涨停封单金额。 |
| 17 | `high_days` | `string` | `string` | 连板或高度天数文本。 |
| 18 | `name` | `string` | `string` | 股票名称。 |
| 19 | `high_days_value` | `number` | `int64` | 连板或高度天数数值。 |
| 20 | `change_tag` | `string` | `string` | 涨跌幅标签。 |
| 21 | `market_type` | `string` | `string` | 市场类型。 |
| 22 | `latest` | `number` | `double` | 最新成交价格。 |

## 数据集备注

同花顺涨停池每日 source 分区；该 source-only asset 不直接同步 ClickHouse raw。

## 校验记录

- Source-only contract added by Plan 0020 Phase 3 from Dagster THS_LIMIT_UP_POOL_SCHEMA.
