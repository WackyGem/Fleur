# stg_jiuyan__action_field_compacted 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/jiuyan__action_field_compacted.md`
- Raw source：`source('raw', 'jiuyan__action_field_compacted')`
- 目标位置：`pipeline/elt/models/staging/jiuyan/stg_jiuyan__action_field_compacted.sql`

## 1. 模型定位

韭研题材异动明细的 source-local staging model。模型保留题材异动与关联股票的明细关系，完成字段命名、可解析时间字段拆分和轻量文本空值处理；不做题材实体归并、证券主数据匹配或连板语义解释。

## 2. 数据特征

- 行数：5,853。
- 粒度：一行代表一个 `action_field_id`, `code` 的题材异动关联证券。
- 候选键：`action_field_id`, `code`，profile 未发现重复。
- `date` 无 NULL，范围 2026-03-04 至 2026-06-01。
- `code` 不是现有 canonical / BaoStock dotted prefix / 纯数字格式；样例显示为 `bj920183`, `sh600108` 等紧凑供应商前缀格式。
- `reason` 空字符串 2,276 行；`num`, `day`, `edition` NULL 4,328 行，属于未连板或 source 未提供。
- `shares_range` 有 45 行负值，最小 -999；profile 未确认其业务含义。
- `delete_time`、`update_time` 全表 NULL；`is_delete` 全部为 false。
- `time` 是字符串，值形如 `1970-01-01 09:59:32.000`，表达日内事件时间而非真实日期。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `action_field_id` | `action_field_id` | `String` | 题材异动记录 ID。 |
| `security_code` | `code` | `String` | 使用 `normalize_cn_security_code(input_format='compact_prefix')` 将紧凑供应商代码转为 canonical 格式。 |
| `trade_date` | `date` | `Date` | 题材异动对应日期。 |
| `action_field_name` | `name` | `String` | 原始名称，source-local。 |
| `reason` | `reason` | `Nullable(String)` | `nullIf(reason, '')`；空字符串转 NULL。 |
| `sort_no` | `sort_no` | `Int64` | 展示排序，0 值保留。 |
| `is_delete` | `is_delete` | `Bool` | 删除标记。 |
| `create_time` | `create_time` | `DateTime` | 记录创建时间。 |
| `related_count` | `count` | `Int64` | 关联对象数量；避免使用保留词 `count`。 |
| `event_time` | `time` | `Nullable(Time)` | 保留 raw Time 类型，1970 日期占位不进入 staging。 |
| `limit_board_text` | `num` | `Nullable(String)` | 连板描述文本，不解析业务语义。 |
| `limit_days` | `day` | `Nullable(Int64)` | 连板天数，NULL 保留。 |
| `limit_boards` | `edition` | `Nullable(Int64)` | 连板板数，NULL 保留。 |
| `expound` | `expound` | `String` | 补充说明文本。 |

## 4. 标准化与 NULL 处理

- `code` 使用 `compact_prefix` 证券代码格式，例如 `sh600108`、`bj920183`；staging 通过项目 macro 转为 canonical `security_code`。
- `reason` 的空字符串可转 NULL。
- `time` 中的 `1970-01-01` 不应按缺失日期处理；它是日内时间承载方式，建议保留原文并派生 time-of-day。
- `shares_range = -999` 不能在 staging 静默转 NULL，需先确认供应商含义。

## 5. 测试建议

- `action_field_id`: `not_null`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- 组合键：`action_field_id`, `security_code` 唯一。
- `is_delete`: `accepted_values`，取值 `false`；如未来出现删除行再放宽。
- 不对 `num`, `day`, `edition`, `time` 加 `not_null`。

## 6. 延后事项

- 紧凑证券代码格式 macro 扩展和 canonical 证券匹配。
- 题材、个股、行业实体归并。
- 连板文本和 `shares_range` 业务含义解释。
- 删除/更新版本处理。
