# stg_jiuyan__industry_list 设计

状态：Implemented

依据：

- Raw profile：`docs/references/raw_profile/jiuyan__industry_list.md`
- Raw source：`source('raw', 'jiuyan__industry_list')`
- 目标位置：`pipeline/elt/models/staging/jiuyan/stg_jiuyan__industry_list.sql`

## 1. 模型定位

韭研行业研究列表快照的 source-local staging model。模型输出行业研究条目的一行一记录视图，完成字段命名、空字符串处理和基础状态暴露，不做正文解析、作者归并、图片 JSON 解析或主题实体标准化。

## 2. 数据特征

- 行数：957（2026-06-03 实施时复核）。
- 粒度：一行一个 `industry_id`。
- 候选键：`industry_id`，profile 未发现重复。
- `create_time` 无 NULL，范围 2024-03-16 至 2026-06-03。
- `update_time` 无 NULL，范围 2026-05-07 至 2026-06-03。
- `delete_time` 全表 NULL；`is_delete` 全部为 false。
- `author` NULL 749 行、空字符串 8 行；`content` 空字符串 17 行。
- `sort_no` 有 2 行负值，最小 -1；profile 未确认业务含义。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `industry_id` | `industry_id` | `String` | 行业研究记录 ID。 |
| `title` | `title` | `String` | 标题原文。 |
| `title_red` | `title_red` | `Bool` | 标题红色高亮标记。 |
| `title_bold` | `title_bold` | `Bool` | 当前全为 false，保留字段。 |
| `author` | `author` | `Nullable(String)` | NULL 保留。 |
| `images_raw` | `imgs` | `String` | 图片列表原文；不在 staging 解析数组。 |
| `keyword` | `keyword` | `String` | 关键词原文。 |
| `content` | `content` | `Nullable(String)` | 空字符串转 NULL。 |
| `is_top` | `is_top` | `Bool` | 置顶标记。 |
| `status_code` | `status` | `Int64` | 供应商状态码，当前全为 0。 |
| `sort_no` | `sort_no` | `Int64` | 展示排序，负值保留。 |
| `forward_count` | `forward_count` | `Int64` | 转发次数。 |
| `browsers_count` | `browsers_count` | `Int64` | 浏览次数，保留供应商字段拼写语义。 |
| `is_delete` | `is_delete` | `Bool` | 删除标记。 |
| `create_time` | `create_time` | `DateTime` | 创建时间。 |
| `update_time` | `update_time` | `Nullable(DateTime)` | 更新时间。 |

## 4. 标准化与 NULL 处理

- `author` 与 `content` 空字符串转 NULL；其余长文本不做无证据清洗。
- `imgs` 不在 staging 解析为结构化图片数组；保留原始字符串，解析延后。
- `sort_no = -1` 保留，不静默修正。
- 全表 NULL 的 `delete_time` 可以不进入第一版，避免低价值字段污染下游。

## 5. 测试建议

- `industry_id`: `not_null`，唯一。
- `title`: `not_null`。
- `create_time`, `update_time`: `not_null`。
- `is_delete`: `accepted_values`，当前取值 `false`。
- 不对 `author`, `content`, `delete_time` 加 `not_null`。

## 6. 延后事项

- 作者同义归一和来源可信度判断。
- `imgs` / `keyword` / `content` 的结构化解析。
- 行业主题实体匹配和正文 NLP。
- 删除版本和更新版本的 SCD 处理。
