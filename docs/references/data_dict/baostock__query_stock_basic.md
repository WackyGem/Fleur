# baostock__query_stock_basic 数据字典

本文件由 `pipeline/contracts/datasets/baostock__query_stock_basic.yml` 生成。字段事实以 contract 为准。

- 数据集：`baostock__query_stock_basic`
- 版本：`1`
- 说明：BaoStock 证券基础信息快照
- 粒度：one row per security code
- Source asset：`source/baostock__query_stock_basic`
- Raw asset：`clickhouse/raw/baostock__query_stock_basic`
- ClickHouse raw：`fleur_raw.baostock__query_stock_basic`
- 分区策略：`snapshot`
- ORDER BY：`(code)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `code` | `string` | `string` | `code` | `String` | BaoStock 基础信息接口返回的证券代码。 |
| 2 | `code_name` | `string` | `string` | `code_name` | `String` | BaoStock 基础信息接口返回的证券简称。 |
| 3 | `ipoDate` | `string` | `date32[day]` | `ipoDate` | `Date` | 证券上市日期。 |
| 4 | `outDate` | `string` | `date32[day]` | `outDate` | `Date` | 证券退市日期；未退市时通常为空。 |
| 5 | `type` | `string` | `int8` | `type` | `Int8` | 证券类型代码。 |
| 6 | `status` | `string` | `int8` | `status` | `Int8` | 证券上市状态。 |

## 数据集备注

BaoStock 证券基础信息快照

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
