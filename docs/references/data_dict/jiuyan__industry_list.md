# jiuyan__industry_list 数据字典

本文件由 `pipeline/contracts/datasets/jiuyan__industry_list.yml` 生成。字段事实以 contract 为准。

- 数据集：`jiuyan__industry_list`
- 版本：`1`
- 说明：韭研行业研究列表快照
- 粒度：one row per industry article
- Source asset：`source/jiuyan__industry_list`
- Raw asset：`clickhouse/raw/jiuyan__industry_list`
- ClickHouse raw：`fleur_raw.jiuyan__industry_list`
- 分区策略：`snapshot`
- ORDER BY：`(industry_id)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|
| 1 | `industry_id` | `string` | `string` | `industry_id` | `String` | 韭研行业研究记录唯一标识。 |
| 2 | `title_red` | `integer` | `bool` | `title_red` | `Bool` | 行业研究标题是否红色高亮展示。 |
| 3 | `title_bold` | `integer` | `bool` | `title_bold` | `Bool` | 行业研究标题是否加粗展示。 |
| 4 | `title` | `string` | `string` | `title` | `String` | 行业研究标题。 |
| 5 | `author` | `string` | `string` | `author` | `LowCardinality(Nullable(String))` | 行业研究内容作者。 |
| 6 | `imgs` | `string` | `string` | `imgs` | `String` | 行业研究内容关联图片列表。 |
| 7 | `keyword` | `string` | `string` | `keyword` | `String` | 行业研究内容关键词。 |
| 8 | `content` | `string` | `string` | `content` | `String` | 行业研究正文内容。 |
| 9 | `is_top` | `integer` | `bool` | `is_top` | `Bool` | 行业研究内容是否置顶。 |
| 10 | `status` | `integer` | `int64` | `status` | `Int64` | 行业研究内容发布状态。 |
| 11 | `sort_no` | `integer` | `int64` | `sort_no` | `Int64` | 行业研究内容展示排序号。 |
| 12 | `forward_count` | `integer` | `int64` | `forward_count` | `Int64` | 行业研究内容转发次数。 |
| 13 | `browsers_count` | `integer` | `int64` | `browsers_count` | `Int64` | 行业研究内容浏览次数。 |
| 14 | `is_delete` | `string` | `bool` | `is_delete` | `Bool` | 行业研究内容是否被标记为删除。 |
| 15 | `delete_time` | `string` | `timestamp[ns]` | `delete_time` | `Nullable(DateTime64(3))` | 行业研究内容删除时间。 |
| 16 | `create_time` | `string` | `timestamp[ns]` | `create_time` | `DateTime64(3)` | 行业研究内容创建时间。 |
| 17 | `update_time` | `string` | `timestamp[ns]` | `update_time` | `DateTime64(3)` | 行业研究内容更新时间。 |

## 数据集备注

韭研行业研究列表快照

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
- LowCardinality review on S3 parquet source/jiuyan__industry_list/000000_0.parquet: rows=956; industry_id nonnull=956 uniq=956 unique_rate=1.000000. industry_id is a source record identifier and uses ClickHouse String. Parquet schema remains string.
