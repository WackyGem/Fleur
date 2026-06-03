# stg_eastmoney__equity_history 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/eastmoney__equity_history.md`
- Raw source：`source('raw', 'eastmoney__equity_history')`
- 目标位置：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__equity_history.sql`

## 1. 模型定位

EastMoney 股本变动历史的 source-local staging model。模型将股本变动记录整理为一证券一截止日粒度的干净输入，完成证券代码、报告日期和字段命名标准化，不做股本口径重算、变动原因归并或自由流通股业务解释。

## 2. 数据特征

- 行数：146,365。
- 粒度：一行代表一个 `SECUCODE`, `END_DATE` 的股本变动记录。
- 候选键：`SECUCODE`, `END_DATE`，profile 未发现重复。
- `SECUCODE` 全部为 canonical 后缀格式；`SECURITY_CODE` 全部为 6 位本地代码。
- 日期字段 `END_DATE`, `NOTICE_DATE`, `LISTING_DATE` 无 NULL。
- 股数和比例字段未发现负值；多个有限售/流通字段存在业务合理 0 值。
- `TOTAL_SHARES_RATIO` 恒为 100。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `SECUCODE` | `LowCardinality(String)` | 使用 `normalize_cn_security_code(input_format='eastmoney_suffix')`。 |
| `end_date` | `END_DATE` | `Date` | 股本变动截止日，保留 source-local 字段命名。 |
| `notice_date` | `NOTICE_DATE` | `Date` | 公告披露日，source-local 字段。 |
| `listing_date` | `LISTING_DATE` | `Date` | 上市流通日期，source-local 字段。 |
| `change_reason` | `CHANGE_REASON` | `LowCardinality(String)` | 变动原因，不做跨源归并。 |
| `change_reason_explain` | `CHANGE_REASON_EXPLAIN` | `LowCardinality(String)` | 详细原因，不做同义归一。 |
| `total_shares` | `TOTAL_SHARES` | `Float64` | 总股本，单位按 raw 保留。 |
| `limited_shares` | `LIMITED_SHARES` | `Float64` | 有限售股份。 |
| `unlimited_shares` | `UNLIMITED_SHARES` | `Float64` | 无限售股份。 |
| `listed_a_shares` | `LISTED_A_SHARES` | `Float64` | A 股流通股。 |
| `limited_a_shares` | `LIMITED_A_SHARES` | `Float64` | A 股限售股。 |

## 4. 标准化与 NULL 处理

- `END_DATE` 在本模型中暴露为 source-local `end_date`，不使用 glossary canonical `report_date`。
- 0 股数、0 比例是业务事实，不转 NULL。
- 不对股本字段做加总校验或重算；这些属于业务规则验证，应放到 intermediate/mart 或专项测试。

## 5. 测试建议

- `security_code`: `not_null`，`cn_security_code_format`。
- `end_date`: `not_null`。
- 组合键：`security_code`, `end_date` 唯一。
- `total_shares`: 可加 `not_null`；大于 0 的断言待通用测试能力确认后增加。
- 比例字段不要加过窄阈值测试，除非后续 profile 全字段确认范围稳定。

## 6. 延后事项

- 股本字段之间的勾稽关系校验。
- `CHANGE_REASON` 和 `CHANGE_REASON_EXPLAIN` 的同义归并。
- 证券类型编码跨源映射。
- 与行情、市值或证券主数据合并。
