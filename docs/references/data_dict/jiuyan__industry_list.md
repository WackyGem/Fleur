# jiuyan__industry_list 数据字典

本文件由 `pipeline/contracts/datasets/jiuyan__industry_list.yml` 生成。字段事实以 contract 为准。

- 数据集：`jiuyan__industry_list`
- 版本：`1`
- 说明：韭研行业研究列表快照
- 粒度：one row per industry article
- Source asset：`source/jiuyan__industry_list`
- Raw asset：`clickhouse/raw/jiuyan__industry_list`
- ClickHouse raw：`raw.jiuyan__industry_list`
- 分区策略：`snapshot`
- ORDER BY：`(industry_id)`

## 字段链路

| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | stg 字段 | 中文描述 |
|---|----------|----------|--------------|---------------------|-----------------|----------|----------|
| 1 | `industry_id` | `string` | `string` | `industry_id` | `LowCardinality(String)` | `industry_id` | 行业研究记录在来源系统中的唯一标识。 |
| 2 | `title_red` | `integer` | `bool` | `title_red` | `Bool` | `title_red` | 标题是否在来源系统中红色高亮展示。 |
| 3 | `title_bold` | `integer` | `bool` | `title_bold` | `Bool` | `title_bold` | 标题是否在来源系统中加粗展示。 |
| 4 | `title` | `string` | `string` | `title` | `String` | `title` | 内容标题。 |
| 5 | `author` | `string` | `string` | `author` | `LowCardinality(String)` | `author` | 内容发布或维护人员名称。 |
| 6 | `imgs` | `string` | `string` | `imgs` | `String` | `imgs` | 内容关联的图片列表或图片地址集合。 |
| 7 | `keyword` | `string` | `string` | `keyword` | `String` | `keyword` | 内容或主题关联的关键词。 |
| 8 | `content` | `string` | `string` | `content` | `String` | `content` | 正文内容或说明文本。 |
| 9 | `is_top` | `integer` | `bool` | `is_top` | `Bool` | `is_top` | 内容是否被来源系统置顶展示。 |
| 10 | `status` | `integer` | `int64` | `status` | `Int64` | `status` | 记录或业务对象在来源系统中的状态。 |
| 11 | `sort_no` | `integer` | `int64` | `sort_no` | `Int64` | `sort_no` | 来源系统用于展示或处理的排序序号。 |
| 12 | `forward_count` | `integer` | `int64` | `forward_count` | `Int64` | `forward_count` | 内容被转发或分享的次数。 |
| 13 | `browsers_count` | `integer` | `int64` | `browsers_count` | `Int64` | `browsers_count` | 内容被浏览或阅读的次数。 |
| 14 | `is_delete` | `string` | `bool` | `is_delete` | `Bool` | `is_delete` | 记录是否已被来源系统标记为删除。 |
| 15 | `delete_time` | `string` | `timestamp[ns]` | `delete_time` | `DateTime64(3)` | `delete_time` | 记录在来源系统中的删除时间；未删除时通常为空。 |
| 16 | `create_time` | `string` | `timestamp[ns]` | `create_time` | `DateTime64(3)` | `create_time` | 记录在来源系统中的创建时间。 |
| 17 | `update_time` | `string` | `timestamp[ns]` | `update_time` | `DateTime64(3)` | `update_time` | 记录在来源系统中的最后更新时间。 |

## 数据集备注

韭研行业研究列表快照

## 校验记录

- Initial contract migrated from docs/references/data_dict and current raw sync specs.
