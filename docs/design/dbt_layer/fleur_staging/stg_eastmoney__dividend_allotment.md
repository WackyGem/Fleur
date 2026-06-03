# stg_eastmoney__dividend_allotment 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/eastmoney__dividend_allotment.md`
- Raw source：`source('raw', 'eastmoney__dividend_allotment')`
- 目标位置：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__dividend_allotment.sql`

## 1. 模型定位

EastMoney 配股事件的 source-local staging model。staging 保留每条配股公告/事件行，完成证券代码 canonical 化、日期类型保留、数值字段命名和文本轻清洗，不做分红配股事件合并、证券主数据修正或方案文本解析。

## 2. 数据特征

- 行数：1,156。
- 粒度：一行代表一个 `SECUCODE`, `NOTICE_DATE`, `EVENT_EXPLAIN` 配股事件。
- 候选键：`SECUCODE`, `NOTICE_DATE`, `EVENT_EXPLAIN`，profile 未发现重复。
- `SECUCODE` 全部为 `600000.SH` 类 canonical 后缀格式。
- `SECURITY_CODE` 全部为 6 位本地代码，只能作为 local code。
- 日期字段 `NOTICE_DATE`, `EQUITY_RECORD_DATE`, `EX_DIVIDEND_DATEE` 均无 NULL。
- `ISSUE_NUM`, `TOTAL_RAISE_FUNDS`, `ISSUE_PRICE` 无 NULL、无 0、无负值。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `SECUCODE` | `LowCardinality(String)` | 使用 `normalize_cn_security_code(input_format='eastmoney_suffix')`，也可直接 `upper` 后测试格式。 |
| `security_name_abbr` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | source-local 简称；不做简称历史归并。 |
| `notice_date` | `NOTICE_DATE` | `Date` | 公告日期。 |
| `equity_record_date` | `EQUITY_RECORD_DATE` | `Date` | 股权登记日。 |
| `ex_dividend_date` | `EX_DIVIDEND_DATEE` | `Date` | raw 字段名拼写为 `EX_DIVIDEND_DATEE`，staging 统一为 `ex_dividend_date`。 |
| `issue_num` | `ISSUE_NUM` | `Float64` | 配股数量；单位按 raw 保留。 |
| `total_raise_funds` | `TOTAL_RAISE_FUNDS` | `Float64` | 配股募集资金总额；单位按 raw 保留。 |
| `issue_price` | `ISSUE_PRICE` | `Float64` | 配股价格。 |
| `event_explain` | `EVENT_EXPLAIN` | `LowCardinality(String)` | 方案说明文本；仅 trim/nullif，不解析为结构化比例。 |

## 4. 标准化与 NULL 处理

- `SECUCODE` 是 canonical 格式，使用 `eastmoney_suffix` metadata 记录来源格式。
- `SECURITY_CODE` 不作为 canonical join key。
- `EX_DIVIDEND_DATEE` 只在 staging 字段名中修正拼写；source_columns metadata 必须保留原始字段名。
- 日期和数值字段 profile 未发现 NULL；staging 不引入填充值。

## 5. 测试建议

- `security_code`: `not_null`，`cn_security_code_format`。
- `notice_date`, `equity_record_date`, `ex_dividend_date`: `not_null`。
- 组合键：`security_code`, `notice_date`, `event_explain` 唯一。
- `issue_num`, `total_raise_funds`, `issue_price`: 可加 `not_null`；非负或大于 0 测试需在项目已有 generic test 支持后补充。
- 可增加 source-local 一致性测试：`security_local_code` 等于 raw `SECURITY_CODE`。

## 6. 延后事项

- `EVENT_EXPLAIN` 文本解析为每 10 股配股比例。
- 与分红主表、证券主数据或公告维表合并。
- 多公告版本优先级、事件时间线合理性裁决。

