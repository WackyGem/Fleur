# stg_eastmoney__equity_history 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/eastmoney__equity_history.md`
- Raw source：`source('raw', 'eastmoney__equity_history')`
- 目标位置：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__equity_history.sql`

## 1. 模型定位

EastMoney 股本变动历史的 source-local staging model。模型将股本变动记录整理为一证券一报告日粒度的干净输入，完成证券代码、本地代码和字段命名标准化，并完整透传 raw 层 69 个股本结构字段。staging 不做股本口径重算、变动原因归并、跨源交易日对齐或自由流通股业务规则再解释。

## 2. 数据特征

- 行数：146,365。
- 粒度：一行代表一个 `SECUCODE`, `END_DATE` 的股本变动记录。
- 候选键：`SECUCODE`, `END_DATE`，profile 未发现重复。
- `SECUCODE` 全部为 canonical 后缀格式；`SECURITY_CODE` 全部为 6 位本地代码。
- 日期字段 `END_DATE`, `NOTICE_DATE`, `LISTING_DATE` 无 NULL。
- 股数和比例字段未发现负值；多个有限售/流通字段存在业务合理 0 值。
- `TOTAL_SHARES_RATIO` 恒为 100。
- `FREE_SHARES` 为供应商给出的自由流通股本，和 `TOTAL_SHARES` 是独立 raw 字段；实际换手率和自由流通市值应在 intermediate/mart 按交易日 as-of join 使用，不在 staging 中重算。
- `NON_FREE_SHARES`、`NON_FREESHARES_RATIO` 和 `IS_FREE_WINDOW` 用于追溯自由流通口径相关字段状态，staging 仅透传。
- raw 层 69 个字段全部进入 staging；除证券代码标准化和少数命名整理外，不丢弃 source-local 股本结构字段。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `SECUCODE` | `String` | 使用 `normalize_cn_security_code(input_format='eastmoney_suffix')` 标准化为 canonical 证券代码。 |
| `security_local_code` | `SECURITY_CODE` | `String` | 使用 `cn_security_local_code(input_format='a_share_local_code')` 保留 6 位本地代码。 |
| `org_code` | `ORG_CODE` | `LowCardinality(String)` | 机构代码。 |
| `report_date` | `END_DATE` | `Date` | 股本变动报告日。 |
| `change_reason` | `CHANGE_REASON` | `LowCardinality(String)` | 变动原因。 |
| `limited_shares` | `LIMITED_SHARES` | `Nullable(Float64)` | 有限售条件股份。 |
| `unlimited_shares` | `UNLIMITED_SHARES` | `Nullable(Float64)` | 无限售条件股份（已流通）。 |
| `total_shares` | `TOTAL_SHARES` | `Nullable(Float64)` | 总股本。 |
| `limited_shares_ratio` | `LIMITED_SHARES_RATIO` | `Nullable(Float64)` | 限售股比例（%），保留供应商百分数口径。 |
| `listed_shares_ratio` | `LISTED_SHARES_RATIO` | `Nullable(Float64)` | 已流通股比例（%），保留供应商百分数口径。 |
| `total_shares_ratio` | `TOTAL_SHARES_RATIO` | `Float64` | 总股本比例（%），保留供应商百分数口径。 |
| `listed_a_shares` | `LISTED_A_SHARES` | `Nullable(Float64)` | 已上市流通 A 股。 |
| `limited_a_shares` | `LIMITED_A_SHARES` | `Nullable(Float64)` | 限售 A 股。 |
| `listed_a_shares_ratio` | `LISTED_A_SHARES_RATIO` | `Nullable(Float64)` | A 股流通比例（%），保留供应商百分数口径。 |
| `limited_a_shares_ratio` | `LIMITED_A_SHARES_RATIO` | `Nullable(Float64)` | 限售 A 股比例（%），保留供应商百分数口径。 |
| `b_free_share` | `B_FREE_SHARE` | `Nullable(Float64)` | 已上市流通 B 股。 |
| `h_free_share` | `H_FREE_SHARE` | `Nullable(Float64)` | 已上市流通 H 股。 |
| `b_free_share_ratio` | `B_FREE_SHARE_RATIO` | `Nullable(Float64)` | B 股流通比例（%），保留供应商百分数口径。 |
| `h_free_share_ratio` | `H_FREE_SHARE_RATIO` | `Nullable(Float64)` | H 股流通比例（%），保留供应商百分数口径。 |
| `security_type_code` | `SECURITY_TYPE_CODE` | `LowCardinality(String)` | 证券类型代码。 |
| `non_free_shares` | `NON_FREE_SHARES` | `Nullable(Float64)` | 非自由流通股。 |
| `non_free_shares_ratio` | `NON_FREESHARES_RATIO` | `Nullable(Float64)` | 非自由流通股比例（%），保留供应商百分数口径。 |
| `limited_b_shares` | `LIMITED_B_SHARES` | `Nullable(Float64)` | 限售 B 股。 |
| `limited_b_shares_ratio` | `LIMITED_BSHARES_RATIO` | `Nullable(Float64)` | 限售 B 股比例（%），保留供应商百分数口径。 |
| `other_free_shares` | `OTHER_FREE_SHARES` | `Nullable(Float64)` | 其他已上市流通股。 |
| `other_free_shares_ratio` | `OTHER_FREESHARES_RATIO` | `Nullable(Float64)` | 其他流通股比例（%），保留供应商百分数口径。 |
| `limited_state_shares` | `LIMITED_STATE_SHARES` | `Nullable(Float64)` | 国家持股（限售）。 |
| `limited_state_legal` | `LIMITED_STATE_LEGAL` | `Nullable(Float64)` | 国有法人持股（限售）。 |
| `limited_othars` | `LIMITED_OTHARS` | `Nullable(Float64)` | 其他限售股份；字段名保留供应商拼写。 |
| `limited_domestic_nostate` | `LIMITED_DOMESTIC_NOSTATE` | `Nullable(Float64)` | 境内非国有法人持股（限售）。 |
| `limited_domestic_natural` | `LIMITED_DOMESTIC_NATURAL` | `Nullable(Float64)` | 境内自然人持股（限售）。 |
| `lock_shares` | `LOCK_SHARES` | `Nullable(Float64)` | 锁定股份。 |
| `limited_foreign_shares` | `LIMITED_FOREIGN_SHARES` | `Nullable(Float64)` | 外资持股（限售）。 |
| `limited_overseas_nostate` | `LIMITED_OVERSEAS_NOSTATE` | `Nullable(Float64)` | 境外非国有法人持股（限售）。 |
| `limited_overseas_natural` | `LIMITED_OVERSEAS_NATURAL` | `Nullable(Float64)` | 境外自然人持股（限售）。 |
| `limited_h_shares` | `LIMITED_H_SHARES` | `Nullable(Float64)` | 限售 H 股。 |
| `sponsor_shares` | `SPONSOR_SHARES` | `Nullable(Float64)` | 发起人股份。 |
| `state_sponsor_shares` | `STATE_SPONSOR_SHARES` | `Nullable(Float64)` | 国家发起人股份。 |
| `sponsor_social_shares` | `SPONSOR_SOCIAL_SHARES` | `Nullable(Float64)` | 社会发起人股份。 |
| `raise_shares` | `RAISE_SHARES` | `Nullable(Float64)` | 募集法人股份。 |
| `raise_state_shares` | `RAISE_STATE_SHARES` | `Nullable(Float64)` | 国家募集法人股份。 |
| `raise_domestic_shares` | `RAISE_DOMESTIC_SHARES` | `Nullable(Float64)` | 境内募集法人股份。 |
| `raise_overseas_shares` | `RAISE_OVERSEAS_SHARES` | `Nullable(Float64)` | 境外募集法人股份。 |
| `notice_date` | `NOTICE_DATE` | `Date` | 公告披露日。 |
| `listing_date` | `LISTING_DATE` | `Date` | 上市流通日期。 |
| `limited_shares_change` | `LIMITED_SHARES_CHANGE` | `Nullable(Float64)` | 限售股变动量。 |
| `unlimited_shares_change` | `UNLIMITED_SHARES_CHANGE` | `Nullable(Float64)` | 流通股变动量。 |
| `total_shares_change` | `TOTAL_SHARES_CHANGE` | `Nullable(Float64)` | 总股本变动量。 |
| `listed_a_shares_change` | `LISTED_ASHARES_CHANGE` | `Nullable(Float64)` | 已上市流通 A 股变动量。 |
| `limited_a_shares_change` | `LIMITED_ASHARES_CHANGE` | `Nullable(Float64)` | 限售 A 股变动量。 |
| `b_free_share_change` | `B_FREESHARE_CHANGE` | `Nullable(Float64)` | B 股流通变动量。 |
| `h_free_share_change` | `H_FREESHARE_CHANGE` | `Nullable(Float64)` | H 股流通变动量。 |
| `limited_b_shares_change` | `LIMITED_BSHARES_CHANGE` | `Nullable(Float64)` | 限售 B 股变动量。 |
| `non_free_shares_change` | `NONFREE_SHARES_CHANGE` | `Nullable(Float64)` | 非自由流通股变动量。 |
| `other_free_shares_change` | `OTHERFREE_SHARES_CHANGE` | `Nullable(Float64)` | 其他流通股变动量。 |
| `free_shares` | `FREE_SHARES` | `Nullable(Float64)` | 自由流通股本，供应商字段，不能用 `total_shares` 替代。 |
| `change_reason_explain` | `CHANGE_REASON_EXPLAIN` | `LowCardinality(String)` | 变动原因详细说明，不做同义归一。 |
| `limited_h_shares_ratio` | `LIMITED_H_SHARES_RATIO` | `Nullable(Float64)` | 限售 H 股比例（%），保留供应商百分数口径。 |
| `limited_h_shares_change` | `LIMITED_H_SHARES_CHANGE` | `Nullable(Float64)` | 限售 H 股变动量。 |
| `is_free_window` | `IS_FREE_WINDOW` | `Bool` | 是否为自由流通窗口，保留 source-local 布尔语义。 |
| `is_limited_window` | `IS_LIMITED_WINDOW` | `Bool` | 是否限售窗口，保留 source-local 布尔语义。 |
| `listed_a_ratio_pc` | `LISTED_A_RATIOPC` | `Nullable(Float64)` | A 股占已流通比例（%），保留供应商百分数口径。 |
| `listed_b_ratio_pc` | `LISTED_B_RATIOPC` | `Nullable(Float64)` | B 股占已流通比例（%），保留供应商百分数口径。 |
| `listed_h_ratio_pc` | `LISTED_H_RATIOPC` | `Nullable(Float64)` | H 股占已流通比例（%），保留供应商百分数口径。 |
| `listed_other_ratio_pc` | `LISTED_OTHER_RATIOPC` | `Nullable(Float64)` | 其他占已流通比例（%），保留供应商百分数口径。 |
| `listed_sum_ratio_pc` | `LISTED_SUM_RATIOPC` | `Nullable(Float64)` | 合计占已流通比例（%），保留供应商百分数口径。 |
| `market_code` | `MARKET_CODE` | `LowCardinality(String)` | 市场代码。 |
| `is_use` | `IS_USE` | `Bool` | 是否有效。 |
| `security_name_abbr` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | 证券简称，不做历史简称归并。 |

## 4. 标准化与 NULL 处理

- `END_DATE` 在本模型中暴露为 glossary canonical `report_date`。
- `SECURITY_CODE` 暴露为 `security_local_code`；`SECUCODE` 暴露为 canonical `security_code`。
- raw 字段 `NON_FREESHARES_RATIO`、`LISTED_ASHARES_CHANGE`、`B_FREESHARE_CHANGE` 等供应商压缩拼写会整理为可读 snake_case，但 YAML `source_columns` 必须保留原 raw 字段。
- 0 股数、0 比例是业务事实，不转 NULL。
- 不对股本字段做加总校验或重算；这些属于业务规则验证，应放到 intermediate/mart 或专项测试。
- 不用 `total_shares - non_free_shares` 派生覆盖 `free_shares`；`FREE_SHARES` 作为供应商自由流通股本事实透传。

## 5. 测试建议

- `security_code`: `not_null`，`cn_security_code_format`。
- `security_local_code`: `not_null`。
- `report_date`: `not_null`。
- 组合键：`security_code`, `report_date` 唯一。
- `notice_date`, `listing_date`, `total_shares`: 可加 `not_null`；股本大于 0 的断言待通用测试能力确认后增加。
- 比例字段不要加过窄阈值测试，除非后续 profile 全字段确认范围稳定。
- `free_shares` 可保留为可空数值字段，不加 `not_null`；实际换手率计算时由 intermediate 决定缺失处理。

## 6. 延后事项

- 股本字段之间的勾稽关系校验。
- `CHANGE_REASON` 和 `CHANGE_REASON_EXPLAIN` 的同义归并。
- 证券类型编码跨源映射。
- 与行情、市值或证券主数据合并。
- 实际换手率：`volume / free_shares * 100`，需要与行情交易日做 as-of join。
- 自由流通市值：`close_price * free_shares`，需要与行情交易日做 as-of join。
