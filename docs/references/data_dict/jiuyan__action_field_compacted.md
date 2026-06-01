# jiuyan__action_field_compacted 数据字典

本文件由 `pipeline/contracts/datasets/jiuyan__action_field_compacted.yml` 生成。字段事实以 contract 为准。

- 数据集：`jiuyan__action_field_compacted`
- 版本：`1`
- 说明：韭研题材异动每日数据年度合并 raw 分区
- 粒度：one row per stock code per source date/action field row
- Source asset：`source/jiuyan__action_field_compacted`
- Raw asset：`clickhouse/raw/jiuyan__action_field_compacted`
- ClickHouse raw：`raw.jiuyan__action_field_compacted`
- 分区策略：`year`
- ORDER BY：`(date, code)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `action_field_id` | `string` | `string` | `action_field_id` | `String` | 韭研题材异动记录唯一标识。 |
| 2 | `name` | `string` | `string` | `name` | `String` | 韭研题材异动名称。 |
| 3 | `date` | `string` | `date32[day]` | `date` | `Date` | 韭研题材异动对应的交易日期。 |
| 4 | `reason` | `string` | `string` | `reason` | `String` | 韭研题材异动形成或归类原因。 |
| 5 | `sort_no` | `integer` | `int64` | `sort_no` | `Int64` | 韭研题材异动展示排序号。 |
| 6 | `is_delete` | `boolean` | `bool` | `is_delete` | `Bool` | 韭研题材异动记录是否被标记为删除。 |
| 7 | `delete_time` | `string` | `timestamp[ns]` | `delete_time` | `DateTime64(3)` | 韭研题材异动记录删除时间。 |
| 8 | `create_time` | `string` | `timestamp[ns]` | `create_time` | `DateTime64(3)` | 韭研题材异动记录创建时间。 |
| 9 | `update_time` | `string` | `timestamp[ns]` | `update_time` | `DateTime64(3)` | 韭研题材异动记录更新时间。 |
| 10 | `count` | `integer` | `int64` | `count` | `Int64` | 韭研题材异动关联对象数量。 |
| 11 | `code` | `string` | `string` | `code` | `LowCardinality(String)` | 题材异动关联的证券代码。 |
| 12 | `time` | `string` | `time32[ms]` | `time` | `String` | 题材异动关联证券的事件时间。 |
| 13 | `num` | `string` | `string` | `num` | `LowCardinality(String)` | 题材异动关联证券的连板数量描述。 |
| 14 | `price` | `integer` | `int64` | `price` | `Int64` | 题材异动关联证券的价格数值。 |
| 15 | `day` | `integer` | `int64` | `day` | `Int64` | 题材异动关联证券的连板天数。 |
| 16 | `edition` | `integer` | `int64` | `edition` | `Int64` | 题材异动关联证券的连板板数。 |
| 17 | `shares_range` | `number` | `double` | `shares_range` | `Float64` | 题材异动关联证券的股份区间数值。 |
| 18 | `expound` | `string` | `string` | `expound` | `String` | 题材异动关联证券的补充说明。 |

## 数据集备注

韭研题材异动每日数据年度合并 raw 分区

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
- Downstream compacted contract consumes source-only asset source/jiuyan__action_field.
- String type decision on S3 parquet source/jiuyan__action_field_compacted: rows=5881; action_field_id nonnull=5881 uniq=740 unique_rate=0.125829, name nonnull=5881 uniq=2106 unique_rate=0.358102, reason nonnull=5881 uniq=336 unique_rate=0.057133, expound nonnull=5881 uniq=4257 unique_rate=0.723856. action_field_id, name and reason use ClickHouse String by explicit schema decision for this raw table; expound is descriptive high-uniqueness text and also uses ClickHouse String. Parquet schema remains string.
