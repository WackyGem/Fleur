# baostock__query_stock_basic 数据字典

本文件由 `pipeline/contracts/datasets/baostock__query_stock_basic.yml` 生成。字段事实以 contract 为准。

- 数据集：`baostock__query_stock_basic`
- 版本：`1`
- 说明：BaoStock 证券基础信息快照
- 粒度：one row per security code
- Source asset：`source/baostock__query_stock_basic`
- Raw asset：`clickhouse/raw/baostock__query_stock_basic`
- ClickHouse raw：`raw.baostock__query_stock_basic`
- 分区策略：`snapshot`
- ORDER BY：`(code)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | stg 字段 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|----------|
| 1 | `code` | `string` | `string` | `code` | `LowCardinality(String)` | `code` | 证券、行业或业务对象在来源系统中的编码。 |
| 2 | `code_name` | `string` | `string` | `code_name` | `LowCardinality(String)` | `code_name` | 证券、行业或业务对象的显示名称。 |
| 3 | `ipoDate` | `string` | `date32[day]` | `ipoDate` | `Date` | `ipo_date` | 证券首次上市交易日期。 |
| 4 | `outDate` | `string` | `date32[day]` | `outDate` | `Date` | `out_date` | 证券退市或终止上市日期。 |
| 5 | `type` | `string` | `int8` | `type` | `Int8` | `stock_type` | 证券品种类型，例如股票、指数或其他市场品种。 |
| 6 | `status` | `string` | `int8` | `status` | `Int8` | `stock_status` | 证券上市、退市或暂停交易等状态。 |

## 数据集备注

BaoStock 证券基础信息快照

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
