# stg_sina__trade_calendar 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/sina__trade_calendar.md`
- Raw source：`source('raw', 'sina__trade_calendar')`
- 目标位置：`pipeline/elt/models/staging/sina/stg_sina__trade_calendar.sql`

## 1. 模型定位

新浪 A 股交易日历的 source-local staging model。模型只暴露规范交易日期，不派生是否月末、周几、交易周序号等日期维度字段。

## 2. 数据特征

- 行数：8,797。
- 粒度：一行一个 `trade_date`。
- 候选键：`trade_date`，profile 未发现重复。
- 覆盖范围：1990-12-19 至 2026-12-31。
- `trade_date` 无 NULL。
- 无证券代码、枚举字段或数值字段。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `trade_date` | `trade_date` | `Date` | 使用 glossary canonical 字段，保留 raw 日期。 |

## 4. 标准化与 NULL 处理

- `trade_date` 已是 Date 类型，不需要 cast。
- 不填补非交易日；本表只表示已有交易日记录。
- 不派生自然日期维度字段。

## 5. 测试建议

- `trade_date`: `not_null`，唯一。
- 可增加日期范围监控，但不作为 staging 阻断测试，避免每年更新后误报。

## 6. 延后事项

- 自然日历补全。
- 交易周、交易月、上一个/下一个交易日等日期维度派生。
- 与其他交易日历来源对账。

