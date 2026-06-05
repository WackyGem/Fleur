# stg_eastmoney__cashflow_sq 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/eastmoney__cashflow_sq.md`
- 数据字典：`docs/references/data_dict/eastmoney__cashflow_sq.md`
- Raw source：`source('raw', 'eastmoney__cashflow_sq')`
- 目标位置：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__cashflow_sq.sql`

## 1. 模型定位

EastMoney 单季度现金流量表 F10 的 source-local staging model。模型保留一证券一报告期的 raw 粒度，并为 data dict 中 372 个 raw 字段提供完整 staging 输出。staging 只完成证券代码、报告日期、披露日期、币种和现金流科目字段命名标准化，不做财务科目重算、单季/累计口径互推、同比/环比重新计算、宽表拆长表或跨源财报合并。

## 2. 数据特征

- 行数：274,016。
- 覆盖证券：`SECUCODE` 5,408 个；`SECURITY_CODE` 5,408 个。
- 粒度：一行代表一个 `SECUCODE`, `REPORT_DATE` 的单季度现金流量表记录。
- 候选键：`SECUCODE`, `REPORT_DATE`，profile 未发现重复。
- 日期范围：`REPORT_DATE` 为 2000-12-31 至 2026-03-31；`NOTICE_DATE` 为 2001-03-15 至 2026-05-15；两者均无 NULL 和 `1970-01-01` 占位。
- `UPDATE_DATE` 为 2001-03-15 至 2026-06-02，NULL 449 行，应保留为可选更新日期。
- `SECUCODE` 全部为 `000001.SZ` 类 canonical 后缀格式；`SECURITY_CODE` 全部为 6 位本地代码。
- `REPORT_TYPE` 仅观察到 `一季度`、`二季度`、`三季度`、`四季度`；`ORG_TYPE` 全部为 `通用`。
- `CURRENCY` 为 `CNY` 273,610 行，NULL 406 行；NULL 不应在 staging 中填充为 `CNY`。
- 已画像 357 个数值字段，其中 220 个字段出现负值，178 个字段出现 0 值，254 个字段 NULL 数不低于 80%。负值、0 值和高 NULL 是财务科目、调整项和行业差异的正常可能性，staging 不过滤。
- 多个补充资料和增长率字段在当前 profile 中全表 NULL，例如 `NETPROFIT`、`ASSET_IMPAIRMENT`、`END_CASH` 及其部分 `_QOQ` / `_YOY` 镜像字段。staging 仍需完整暴露这些字段，但不应把它们标成高价值字段或添加 `not_null` 测试。

## 3. 字段设计

字段覆盖规则：

- staging 必须暴露 `docs/references/data_dict/eastmoney__cashflow_sq.md` 中列出的全部 372 个 raw 字段，每个 raw 字段至少对应一个 staging 输出字段。
- 本设计文档只列代表字段、字段组和命名规则；完整字段清单以 data dict、staging SQL 和 staging YAML 为准，避免在设计文档中重复 372 行机械映射。
- 除确定性派生字段（例如 `exchange_code`）外，staging 不新增业务派生指标；派生字段不替代 raw 字段完整覆盖要求。

### 3.1 标识与报告期字段

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `SECUCODE` | `LowCardinality(String)` | 使用 `normalize_cn_security_code(input_format='eastmoney_suffix')`；作为 canonical join key。 |
| `security_local_code` | `SECURITY_CODE` | `LowCardinality(String)` | 6 位本地代码，仅作为辅助字段，不单独推断交易所。 |
| `exchange_code` | `SECUCODE` | `LowCardinality(String)` | 从 canonical `SECUCODE` 确定性拆分出 `SH`、`SZ`、`BJ`。 |
| `security_name_abbr` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | source-local 证券简称；不做历史简称归并。 |
| `org_code` | `ORG_CODE` | `LowCardinality(String)` | EastMoney 机构代码，保留 source-local 语义。 |
| `org_type` | `ORG_TYPE` | `LowCardinality(String)` | 当前 profile 全部为 `通用`，保留原枚举。 |
| `security_type_code` | `SECURITY_TYPE_CODE` | `LowCardinality(String)` | 证券类型供应商编码，不映射跨源证券类型。 |
| `listing_state` | `LISTING_STATE` | `LowCardinality(String)` | 上市状态供应商枚举，不做主数据裁决。 |
| `report_date` | `REPORT_DATE` | `Date` | 报告期截止日，作为财务事实日期。 |
| `report_type` | `REPORT_TYPE` | `LowCardinality(String)` | 单季度报告类型，保留中文枚举。 |
| `report_date_name` | `REPORT_DATE_NAME` | `LowCardinality(String)` | 报告期名称，例如 `2025一季度`。 |
| `notice_date` | `NOTICE_DATE` | `Date` | 公告日期。 |
| `update_date` | `UPDATE_DATE` | `Nullable(Date)` | 更新日期，449 行 NULL 保留。 |
| `currency` | `CURRENCY` | `LowCardinality(Nullable(String))` | 金额币种；NULL 保留，不默认补 `CNY`。 |
| `opinion_type` | `OPINION_TYPE` | `LowCardinality(Nullable(String))` | 审计意见类型；大量 NULL，保留为可选字段。 |
| `osopinion_type` | `OSOPINION_TYPE` | `LowCardinality(Nullable(String))` | 内控审计意见类型；当前几乎全 NULL，不加 not_null。 |

### 3.2 现金流科目字段

staging 必须暴露全部现金流科目、补充资料、平衡项、其他项和附注金额字段。字段命名采用 raw 字段的小写 snake_case；只在 raw 字段存在明显拼写错误时修正 staging 字段名，并在 YAML `source_columns` 保留原始字段名。下表只列代表字段，不是完整字段清单。

| 字段组 | 代表 staging 字段 | 来源字段示例 | 设计说明 |
|--------|-------------------|--------------|----------|
| 经营活动流入 | `sales_services`, `receive_tax_refund`, `receive_other_operate`, `total_operate_inflow` | `SALES_SERVICES`, `RECEIVE_TAX_REFUND`, `TOTAL_OPERATE_INFLOW` | 保留金额原值；负值和 0 值不修正。 |
| 经营活动流出 | `buy_services`, `pay_staff_cash`, `pay_all_tax`, `pay_other_operate`, `total_operate_outflow` | `BUY_SERVICES`, `PAY_STAFF_CASH`, `PAY_ALL_TAX` | 保留 source-local 科目，不重算小计。 |
| 经营活动净额 | `netcash_operate` | `NETCASH_OPERATE` | 作为供应商提供的经营活动现金流净额，不由流入流出派生。 |
| 投资活动流入 | `withdraw_invest`, `receive_invest_income`, `disposal_long_asset`, `total_invest_inflow` | `WITHDRAW_INVEST`, `RECEIVE_INVEST_INCOME`, `TOTAL_INVEST_INFLOW` | 保留金额原值；行业特有字段可选暴露。 |
| 投资活动流出 | `construct_long_asset`, `invest_pay_cash`, `obtain_subsidiary_other`, `total_invest_outflow` | `CONSTRUCT_LONG_ASSET`, `INVEST_PAY_CASH`, `TOTAL_INVEST_OUTFLOW` | 不做科目归并或方向修正。 |
| 投资活动净额 | `netcash_invest` | `NETCASH_INVEST` | 负值是预期业务事实，不加非负测试。 |
| 筹资活动流入 | `accept_invest_cash`, `receive_loan_cash`, `issue_bond`, `total_finance_inflow` | `ACCEPT_INVEST_CASH`, `RECEIVE_LOAN_CASH`, `TOTAL_FINANCE_INFLOW` | 保留 source-local 科目。 |
| 筹资活动流出 | `pay_debt_cash`, `assign_dividend_profit`, `pay_other_finance`, `total_finance_outflow` | `PAY_DEBT_CASH`, `ASSIGN_DIVIDEND_PORFIT`, `TOTAL_FINANCE_OUTFLOW` | `ASSIGN_DIVIDEND_PORFIT` 在 staging 修正为 `assign_dividend_profit`。 |
| 筹资活动净额 | `netcash_finance` | `NETCASH_FINANCE` | 不重算、不平衡校验。 |
| 现金及等价物 | `rate_change_effect`, `cce_add`, `begin_cce`, `end_cce` | `RATE_CHANGE_EFFECT`, `CCE_ADD`, `BEGIN_CCE`, `END_CCE` | 保留 raw 金额单位；`CCE` 保持供应商缩写。 |

### 3.3 同比/环比字段

`_QOQ` 和 `_YOY` 字段是供应商给出的增长率，单位为百分比。第一版如暴露这些字段，建议使用 `<base_metric>_qoq_pct` 和 `<base_metric>_yoy_pct` 命名，例如：

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `sales_services_qoq_pct` | `SALES_SERVICES_QOQ` | `Nullable(Float64)` | 销售商品、提供劳务收到的现金环比增长率（%）。 |
| `netcash_operate_qoq_pct` | `NETCASH_OPERATE_QOQ` | `Nullable(Float64)` | 经营活动现金流净额环比增长率（%）。 |
| `netcash_invest_yoy_pct` | `NETCASH_INVEST_YOY` | `Nullable(Float64)` | 投资活动现金流净额同比增长率（%）。 |
| `cce_add_yoy_pct` | `CCE_ADD_YOY` | `Nullable(Float64)` | 现金及现金等价物净增加额同比增长率（%）。 |

增长率字段只做 rename/cast，不在 staging 中用相邻季度或上年同期重新计算。profile 观察到多项增长率存在极端值和高 NULL，测试不应设置窄阈值。

### 3.4 完整字段暴露与文档省略

- 全表 NULL 或接近全表 NULL 的补充资料字段仍然暴露，例如 `NETPROFIT`、`ASSET_IMPAIRMENT`、`END_CASH` 及其 `_QOQ` / `_YOY` 镜像字段。
- 供应商平衡项、其他项和附注字段仍然暴露，但只做 rename/cast/source metadata，不做勾稽解释。
- 文档省略逐列字段表；落地 SQL/YAML 时必须能从 data dict 的 372 个 raw 字段追溯到 staging 输出字段。
- 任何需要把宽表拆成长表的字段集合不在 staging 改变 grain；拆长表属于 intermediate/mart 建模。

## 4. 标准化与 NULL 处理

- `SECUCODE` 已是 canonical 后缀格式，仍通过项目 macro 记录 normalization metadata。
- `SECURITY_CODE` 只暴露为 `security_local_code`，不作为独立 join key。
- `REPORT_DATE`, `NOTICE_DATE` 保持 `Date`；本轮未发现 `1970-01-01` 占位，不引入占位修正逻辑。
- `UPDATE_DATE` 的 449 行 NULL、`CURRENCY` 的 406 行 NULL 和宽表数值 NULL 均保留。
- 空字符串如后续出现，只做 `nullif(trim(value), '')` 级别处理。
- 金额字段不做单位换算；同比/环比字段保留供应商百分比单位，并在字段描述中写明 `%`。
- 负值和 0 值不转 NULL、不过滤、不按正负方向修正。

## 5. 测试建议

- 模型级组合键：`security_code`, `report_date` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `security_local_code`: `not_null`；如已有 generic test，校验 6 位数字格式。
- `exchange_code`: `accepted_values`，取值 `SH`, `SZ`, `BJ`。
- `report_date`: `not_null`。
- `notice_date`: `not_null`。
- `report_type`: `accepted_values`，取值 `一季度`, `二季度`, `三季度`, `四季度`。
- `currency`: `accepted_values`，取值 `CNY`，允许 NULL。
- `update_date`: 不加 `not_null`。
- 财务金额、同比/环比和补充资料字段不加全字段 `not_null`、非负或窄范围测试；需要业务阈值时在 intermediate/mart 或专项 data test 中实现。

## 6. YAML 元数据要求

- `security_code`, `security_local_code`, `exchange_code`, `report_date` 优先使用 glossary 字段并记录 `glossary_key`。
- source-local 字段使用 `dictionary_scope: local`，并为每个输出字段记录 `config.meta.source_columns`。
- YAML 必须覆盖 data dict 中全部 372 个 raw 字段；文档省略字段清单不等于 SQL/YAML 可以省略字段。
- `security_code` 记录 normalization：`macro: normalize_cn_security_code`, `input_format: eastmoney_suffix`。
- 从 `SECUCODE` 派生的 `exchange_code` 记录 `derived_from` 或等价 metadata。
- 金额字段描述或 meta 中说明“金额单位按 raw 保留”；增长率字段说明“单位为 %，供应商已计算”。
- raw 字段拼写修正必须在 `source_columns` 中保留原字段，例如 `assign_dividend_profit` 来源为 `ASSIGN_DIVIDEND_PORFIT`。

## 7. 延后事项

- 单季度表与累计口径现金流表的合并、互推或差分。
- 财务科目重算、小计/净额勾稽、平衡项解释和异常阈值判断。
- 同比/环比增长率重新计算或供应商口径校验。
- 财报宽表拆长表、现金流科目层级维表和财务指标标准化。
- 证券主数据、上市状态、证券类型和交易所归属的跨源裁决。
- 审计意见、内控意见和上市状态枚举的跨源统一。
