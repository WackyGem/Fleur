# stg_eastmoney__income_sq 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/eastmoney__income_sq.md`
- 数据字典：`docs/references/data_dict/eastmoney__income_sq.md`
- Raw source：`source('raw', 'eastmoney__income_sq')`
- 目标位置：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__income_sq.sql`

## 1. 模型定位

EastMoney F10 利润表单季度口径的 source-local staging model。模型保留东方财富披露的一证券一报告期宽表记录，把证券代码、报告期、公告日期、利润表科目金额、环比增长率、同比增长率和审计意见字段整理为 dbt 可复用的字段名与类型。

staging 只做确定性的字段命名、证券代码标准化、基础类型保留和 source-local NULL 语义保留，不做财务科目重算、单季度与累计口径互推、报表勾稽校验、跨源财报合并、公告版本选择或宽表拆长表。

## 2. 数据特征

- 行数：279,918。
- 覆盖证券：`SECUCODE` 5,418 个；`SECURITY_CODE` 5,418 个。
- 粒度：一行代表一个 `SECUCODE`, `REPORT_DATE` 单季度利润表记录。
- 候选键：`SECUCODE`, `REPORT_DATE`，profile 未发现重复。
- `SECUCODE` 全部为 `600000.SH` 类 canonical 后缀格式；`SECURITY_CODE` 全部为 6 位本地代码。
- 日期范围：`REPORT_DATE` 为 1993-06-30 至 2026-03-31；`NOTICE_DATE` 为 1993-08-14 至 2026-05-15；`UPDATE_DATE` 为 1993-08-14 至 2026-06-02。
- `REPORT_DATE`, `NOTICE_DATE` 无 NULL；`UPDATE_DATE` 有 193 行 NULL；日期字段未发现 `1970-01-01` 占位值。
- `REPORT_TYPE` 仅观察到 `一季度`、`二季度`、`三季度`、`四季度`；`ORG_TYPE` 全部为 `通用`。
- `CURRENCY` 仅观察到 `CNY` 和 NULL，其中 NULL 494 行；staging 不用 `CNY` 回填。
- raw 共 299 个字段，其中 14 个元数据字段、285 个数值字段。数值字段分为 95 个单季度绝对值字段、95 个 `_QOQ` 环比增长率字段、95 个 `_YOY` 同比增长率字段。
- 285 个数值字段中 247 个出现负值、195 个出现 0 值、169 个 NULL 数不低于 80%。这些值符合利润表科目、调整项、行业适用性和增长率口径的 source-local 特征，staging 不过滤、不置零、不转 NULL。

## 3. 字段设计

第一版建议输出 raw 中 299 个字段的信息：`SECUCODE` 标准化为 `security_code`，`SECURITY_CODE` 暴露为 `security_local_code`，其余字段按规则改为 lower snake case。不要额外输出 `exchange_code`；下游如需要可从 `security_code` 派生。

| 字段组 | Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------|--------------|----------|----------|----------|
| 主键 | `security_code` | `SECUCODE` | `String` | 使用 `normalize_cn_security_code(input_format='eastmoney_suffix')`。 |
| 主键辅助 | `security_local_code` | `SECURITY_CODE` | `String` | 6 位本地代码，只用于追溯和一致性检查，不作为 join key。 |
| 证券属性 | `security_name_abbr` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | source-local 简称，不做历史简称归并。 |
| 证券属性 | `org_code` | `ORG_CODE` | `LowCardinality(String)` | 东方财富机构代码。 |
| 证券属性 | `org_type` | `ORG_TYPE` | `LowCardinality(String)` | profile 当前全为 `通用`，保留原值。 |
| 报告期 | `report_date` | `REPORT_DATE` | `Date` | 财报报告期截止日，进入候选键。 |
| 报告期 | `report_type` | `REPORT_TYPE` | `LowCardinality(String)` | 单季度报告类型，保留 `一季度`、`二季度`、`三季度`、`四季度`。 |
| 报告期 | `report_date_name` | `REPORT_DATE_NAME` | `LowCardinality(String)` | 报告期展示标签，例如 `2026一季度`。 |
| 证券属性 | `security_type_code` | `SECURITY_TYPE_CODE` | `LowCardinality(String)` | 东方财富证券类型代码，不在 staging 做跨源映射。 |
| 披露时间 | `notice_date` | `NOTICE_DATE` | `Date` | 公告日期，profile 无 NULL。 |
| 披露时间 | `update_date` | `UPDATE_DATE` | `Nullable(Date)` | 更新日期，193 行 NULL 保留。 |
| 币种 | `currency` | `CURRENCY` | `LowCardinality(Nullable(String))` | 仅观察到 `CNY` 和 NULL；NULL 不回填。 |
| 审计意见 | `opinion_type` | `OPINION_TYPE` | `LowCardinality(Nullable(String))` | 审计意见类型，NULL 多，保留原始枚举。 |
| 审计意见 | `osopinion_type` | `OSOPINION_TYPE` | `LowCardinality(Nullable(String))` | 内控审计意见类型，基本为空，保留原始枚举。 |
| 单季度金额/每股值 | `<line_item>` | 非 `_QOQ`/`_YOY` 数值字段 | `Nullable(Float64)` | 95 个单季度利润表绝对值字段，金额单位按 raw 保留；EPS 字段按 raw 元/股保留。 |
| 环比增长率 | `<line_item>_qoq` | `_QOQ` 数值字段 | `Nullable(Float64)` | 95 个环比增长率字段，保留供应商百分数口径，不除以 100。 |
| 同比增长率 | `<line_item>_yoy` | `_YOY` 数值字段 | `Nullable(Float64)` | 95 个同比增长率字段，保留供应商百分数口径，不除以 100。 |

数值字段命名规则：

- 常规字段采用确定性 lower snake case，例如 `TOTAL_OPERATE_INCOME` -> `total_operate_income`，`TOTAL_OPERATE_INCOME_QOQ` -> `total_operate_income_qoq`，`TOTAL_OPERATE_INCOME_YOY` -> `total_operate_income_yoy`。
- 保留 source-local 科目粒度，不把宽表拆成长表，不把相近科目合并。
- 金额、EPS、环比和同比字段必须在 YAML `config.meta.source_columns` 中记录原始字段；`_QOQ` 和 `_YOY` metadata 需标记 `unit: percent` 或等价说明。
- `BASIC_EPS`、`DILUTED_EPS` 为每股收益，metadata 需单独标记 `unit: CNY_per_share` 或等价说明，不归入普通金额字段单位。
- 仅修正明确的供应商拼写或分词问题，source column metadata 仍保留原始字段名。

已知命名建议：

| 来源字段 | Staging 字段 | 说明 |
|----------|--------------|------|
| `SECUCODE` | `security_code` | canonical 证券连接键。 |
| `SECURITY_CODE` | `security_local_code` | 6 位本地代码，避免和 canonical key 混淆。 |
| `OSOPINION_TYPE` | `osopinion_type` | 保持当前 source-local 缩写，不臆造 `os_opinion_type` 语义。 |
| `TOTAL_OPERATE_INCOME` | `total_operate_income` | 利润表科目按 lower snake case。 |
| `TOTAL_OPERATE_INCOME_QOQ` | `total_operate_income_qoq` | 环比增长率字段保留 `_qoq` 后缀。 |
| `TOTAL_OPERATE_INCOME_YOY` | `total_operate_income_yoy` | 同比增长率字段保留 `_yoy` 后缀。 |
| `BASIC_EPS` | `basic_eps` | EPS 缩写保留为 `eps`。 |
| `DEDUCT_PARENT_NETPROFIT` | `deduct_parent_netprofit` | 不在 staging 中改写为业务解释性长名。 |
| `ACF_END_INCOME` | `acf_end_income` | source-local 缩写保留，后续标准科目映射延后。 |
| `HMI_AFA` | `hmi_afa` | source-local 缩写保留，避免无依据扩写。 |

## 4. 标准化与 NULL 处理

- `security_code` 只从 `SECUCODE` 标准化；`SECURITY_CODE` 不单独推断交易所。
- 不对 `security_code`, `report_date` 做去重；profile 已确认候选键唯一，后续如出现重复应先回到 raw profile 分析原因。
- 日期字段已经是 ClickHouse `Date` / `Nullable(Date)`；不需要从字符串重新解析。
- `UPDATE_DATE`、`CURRENCY`、审计意见字段和大量利润表科目 NULL 是 source-local 预期缺失，staging 保留 NULL。
- 0 值是披露事实，不转 NULL；负值可能来自费用冲回、减值转回、亏损、少数股东损益、综合收益调整或增长率下降，不在 staging 过滤。
- `_QOQ` 与 `_YOY` 字段保留供应商百分数值，例如 raw 值 `5` 表示 5%，不转换为 `0.05`。
- 不对 `TOTAL_OPERATE_INCOME`、`TOTAL_OPERATE_COST`、`OPERATE_PROFIT`、`TOTAL_PROFIT`、`NETPROFIT` 等总计字段做等式校验或重算；这类勾稽关系属于 intermediate/mart 或专项数据质量测试。
- 不用单季度表反推累计利润表，也不用累计表反推单季度利润表；`income_sq` 和 `income_ytd` 的口径对齐延后到 intermediate。

## 5. SQL 逻辑建议

实现时可按以下顺序组织输出字段：

1. 主键与证券字段：`security_code`, `security_local_code`, `security_name_abbr`, `org_code`, `org_type`, `security_type_code`。
2. 报告与披露字段：`report_date`, `report_type`, `report_date_name`, `notice_date`, `update_date`, `currency`。
3. 利润表单季度绝对值字段：所有非 `_QOQ`/`_YOY` 数值字段，按 data_dict 原始顺序输出。
4. 环比增长率字段：所有 `_QOQ` 数值字段，按 data_dict 原始顺序输出。
5. 同比增长率字段：所有 `_YOY` 数值字段，按 data_dict 原始顺序输出。
6. 审计意见字段：`opinion_type`, `osopinion_type`。

示例片段：

```sql
with source as (
    select
        SECUCODE,
        SECURITY_CODE,
        SECURITY_NAME_ABBR,
        ORG_CODE,
        ORG_TYPE,
        REPORT_DATE,
        REPORT_TYPE,
        REPORT_DATE_NAME,
        SECURITY_TYPE_CODE,
        NOTICE_DATE,
        UPDATE_DATE,
        CURRENCY,
        TOTAL_OPERATE_INCOME,
        TOTAL_OPERATE_INCOME_QOQ,
        TOTAL_OPERATE_INCOME_YOY,
        OPERATE_PROFIT,
        NETPROFIT,
        PARENT_NETPROFIT,
        BASIC_EPS,
        OPINION_TYPE,
        OSOPINION_TYPE
    from {{ source('raw', 'eastmoney__income_sq') }}
)

select
    {{ normalize_cn_security_code('SECUCODE', input_format='eastmoney_suffix') }} as security_code,
    SECURITY_CODE as security_local_code,
    SECURITY_NAME_ABBR as security_name_abbr,
    ORG_CODE as org_code,
    ORG_TYPE as org_type,
    REPORT_DATE as report_date,
    REPORT_TYPE as report_type,
    REPORT_DATE_NAME as report_date_name,
    SECURITY_TYPE_CODE as security_type_code,
    NOTICE_DATE as notice_date,
    UPDATE_DATE as update_date,
    CURRENCY as currency,
    TOTAL_OPERATE_INCOME as total_operate_income,
    TOTAL_OPERATE_INCOME_QOQ as total_operate_income_qoq,
    TOTAL_OPERATE_INCOME_YOY as total_operate_income_yoy,
    OPERATE_PROFIT as operate_profit,
    NETPROFIT as netprofit,
    PARENT_NETPROFIT as parent_netprofit,
    BASIC_EPS as basic_eps,
    OPINION_TYPE as opinion_type,
    OSOPINION_TYPE as osopinion_type
from source
```

宽表实现不应在 SQL 中手写复杂表达式；字段 alias 可以按 data_dict 机械生成后 review，业务口径转换不要混入 staging。

## 6. 测试建议

- 模型级组合唯一：`security_code`, `report_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `security_local_code`: `not_null`；可增加与 `security_code` 前 6 位一致的 source-local 测试。
- `report_date`: `not_null`。
- `notice_date`: `not_null`。
- `report_type`: `not_null`，`accepted_values` 取 `一季度`, `二季度`, `三季度`, `四季度`。
- `currency`: 可加 `accepted_values` 取 `CNY`，但不加 `not_null`。
- `org_type`: 可加 `accepted_values` 取 `通用`；如果后续 source 出现新机构类型，应先更新 raw profile 和设计。
- `security_type_code`: 不建议第一版加 accepted-values；profile 观察到 `058001001` 和 `058001008`，枚举语义需等证券类型维度设计确认。
- `opinion_type`, `osopinion_type`: 不加 `not_null`；accepted-values 需先补充完整枚举画像。
- 利润表科目、EPS、`_QOQ` 和 `_YOY` 字段不加全字段 `not_null`、非负或阈值测试；高 NULL、0、负值和极端增长率均已由 profile 证明需要保留。

## 7. YAML metadata 建议

- 每个输出字段都记录 `config.meta.source_columns`，source 为 `raw`，table 为 `eastmoney__income_sq`。
- `security_code` 记录 `glossary_key: security_code` 和 normalization metadata：`macro: normalize_cn_security_code`，`input_format: eastmoney_suffix`。
- `security_local_code` 可记录 `glossary_key: security_local_code`；如直接透传 raw `SECURITY_CODE`，normalization metadata 可省略或记录 `input_format: eastmoney_local_code`。
- 其余 source-local 字段记录 `dictionary_scope: local`。
- 单季度金额字段记录 source-local 单位说明，例如 `unit: raw_currency_amount`；不要在 metadata 中声称已做元/万元/亿元转换，除非后续 profile 明确验证。远端接口文档说明绝对值字段为金额元，但当前 staging 仍以 raw currency amount 表达，避免与多币种或供应商变更耦合。
- `basic_eps`、`diluted_eps` 记录 `unit: CNY_per_share` 或等价说明。
- `_QOQ` 字段记录百分数口径，例如 `period_comparison: quarter_over_quarter`，`unit: percent`，`scale: percent_value_not_fraction`。
- `_YOY` 字段记录百分数口径，例如 `period_comparison: year_over_year`，`unit: percent`，`scale: percent_value_not_fraction`。
- 命名例外字段在 metadata 中保留原始 `source_columns`，必要时增加 `normalization.reason: spelling_or_abbreviation_preserved` 或等价说明。

## 8. 延后事项

- 单季度利润表与年初至报告期末利润表的口径对齐、互推和差异解释。
- 利润表科目之间的会计恒等式和总分项勾稽校验。
- 财务科目跨源映射、同义科目归并和报表标准科目树。
- 资产负债表、利润表、现金流量表之间的财报实体合并。
- 宽表拆长表或财务指标事实表建模。
- 环比/同比增长率重算、异常阈值判断和极端值裁决。
- 审计意见、机构类型、证券类型代码的跨源枚举映射。
- `CURRENCY` NULL 回填和多币种折算。
