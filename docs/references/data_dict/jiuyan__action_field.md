# jiuyan__action_field 数据字典

本文件由 `pipeline/contracts/datasets/jiuyan__action_field.yml` 生成。字段事实以 contract 为准。

- 数据集：`jiuyan__action_field`
- 版本：`1`
- 说明：韭研题材异动每日 source 分区
- 粒度：one row per stock code per source trade date/action field row
- Source asset：`source/jiuyan__action_field`
- Raw asset：不适用
- ClickHouse raw：不适用

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | 中文描述 |
|---|----------|----------|--------------|----------|
| 1 | `action_field_id` | `string` | `string` | 韭研题材异动记录唯一标识。 |
| 2 | `name` | `string` | `string` | 韭研题材异动名称。 |
| 3 | `date` | `string` | `date32[day]` | 韭研题材异动对应的交易日期。 |
| 4 | `reason` | `string` | `string` | 韭研题材异动形成或归类原因。 |
| 5 | `sort_no` | `integer` | `int64` | 韭研题材异动展示排序号。 |
| 6 | `is_delete` | `string` | `bool` | 韭研题材异动记录是否被标记为删除。 |
| 7 | `delete_time` | `string` | `timestamp[ms]` | 韭研题材异动记录删除时间。 |
| 8 | `create_time` | `string` | `timestamp[ms]` | 韭研题材异动记录创建时间。 |
| 9 | `update_time` | `string` | `timestamp[ms]` | 韭研题材异动记录更新时间。 |
| 10 | `count` | `integer` | `int64` | 韭研题材异动关联对象数量。 |
| 11 | `code` | `string` | `string` | 题材异动关联的证券代码。 |
| 12 | `time` | `string` | `time32[ms]` | 题材异动关联证券的事件时间。 |
| 13 | `num` | `string` | `string` | 题材异动关联证券的连板数量描述。 |
| 14 | `price` | `integer` | `int64` | 题材异动关联证券的价格数值。 |
| 15 | `day` | `integer` | `int64` | 题材异动关联证券的连板天数。 |
| 16 | `edition` | `integer` | `int64` | 题材异动关联证券的连板板数。 |
| 17 | `shares_range` | `number` | `double` | 题材异动关联证券的股份区间数值。 |
| 18 | `expound` | `string` | `string` | 题材异动关联证券的补充说明。 |

## 数据集备注

韭研题材异动每日 source 分区；该 source-only asset 不直接同步 ClickHouse raw。

## 校验记录

- Source-only contract added by Plan 0020 Phase 3 from Dagster JIUYAN_ACTION_FIELD_SCHEMA.
