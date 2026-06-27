# stg_eastmoney__dividend_main 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/eastmoney__dividend_main.md`
- Raw source：`source('raw', 'eastmoney__dividend_main')`
- 目标位置：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__dividend_main.sql`

## 1. 模型定位

EastMoney 分红送转主表的 source-local staging model。该表包含方案进度、报告期、公告日期、实施日期和分红金额等宽字段；staging 负责清洗字段名和基础格式，不做重复版本选择、方案文本解析或事件合并。

## 2. 数据特征

- 行数：151,606。
- 覆盖证券：`SECUCODE` 5,520 个。
- 候选粒度：profile 观察为 `SECUCODE`, `REPORT_DATE`，但该候选键存在 19 组重复，单键最大 2 行。
- `SECUCODE` 全部为 canonical 后缀格式；`SECURITY_CODE` 全部为 6 位本地代码。
- `NOTICE_DATE` 与 `REPORT_DATE` 无 NULL；多个实施日期字段存在大量 NULL，属于预期缺失。
- `REPORT_TIME` 在 raw contract 中为 `Nullable(Date)`；历史 S3/ClickHouse 观察曾为 `Nullable(String)`，当前 source-to-Parquet 转换会将可解析日期写为 `date32[day]`，历史非日期标签转 NULL。
- `LAST_TRADE_DATE` 全表 NULL，不应作为高价值 staging 字段。
- `ASSIGN_PROGRESS` 有 6 个观察值，其中低频值包括 `股东大会否决` 和 `董事会决议未通过`。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `security_code` | `SECUCODE` | `LowCardinality(String)` | 使用 `eastmoney_suffix` 标准化并测试格式。 |
| `security_name_abbr` | `SECURITY_NAME_ABBR` | `LowCardinality(String)` | source-local 简称。 |
| `notice_date` | `NOTICE_DATE` | `Date` | 公告日期，非 NULL。 |
| `report_period_label` | `REPORT_DATE` | `String` | 保留 `1990年报`、`2026重整计划` 等原始报告期标签，不强转 Date。 |
| `report_date` | `REPORT_TIME` | `Nullable(Date)` | raw contract 已收敛为 `Nullable(Date)`；1,621 行 NULL 保留，暴露为 glossary canonical `report_date`。 |
| `assign_progress` | `ASSIGN_PROGRESS` | `LowCardinality(String)` | 保留供应商方案进度枚举。 |
| `is_unassign` | `IS_UNASSIGN` | `Bool` | 保留 source-local 布尔。 |
| `impl_plan_profile` | `IMPL_PLAN_PROFILE` | `LowCardinality(Nullable(String))` | 方案简介；仅 trim/nullif。 |
| `impl_plan_newprofile` | `IMPL_PLAN_NEWPROFILE` | `LowCardinality(String)` | 方案简介加进度后缀；不解析结构。 |
| `new_profile` | `NEW_PROFILE` | `Nullable(String)` | 含税方案文本；不解析结构。 |
| `assign_object` | `ASSIGN_OBJECT` | `Nullable(String)` | 分配对象；NULL 是预期缺失。 |
| `equity_record_date` | `EQUITY_RECORD_DATE` | `Nullable(Date)` | 大量 NULL，保留。 |
| `ex_dividend_date` | `EX_DIVIDEND_DATE` | `Nullable(Date)` | 大量 NULL，保留。 |
| `pay_cash_date` | `PAY_CASH_DATE` | `Nullable(Date)` | 大量 NULL，保留。 |
| `gmdecision_notice_date` | `GMDECISION_NOTICE_DATE` | `Nullable(Date)` | 股东大会决议公告日。 |
| `annual_general_meeting_date` | `DAT_YAGGR` | `Nullable(Date)` | 年度股东大会日期。 |
| `info_code` | `INFO_CODE` | `Nullable(String)` | 公告编号；可作为后续版本识别候选字段。 |
| `total_dividend` | `TOTAL_DIVIDEND` | `Nullable(Float64)` | 金额单位按 raw 保留；0 值保留。 |
| `total_dividend_a` | `TOTAL_DIVIDEND_A` | `Nullable(Float64)` | A 股分红总额；0 值保留。 |

## 4. 标准化与 NULL 处理

- 不对 `SECUCODE`, `REPORT_DATE` 重复行做去重；staging 必须保留 raw 粒度。
- `REPORT_DATE` 不是稳定日期格式，第一版保留为报告期标签；`REPORT_TIME` 可稳定 cast 为 `Nullable(Date)`，并暴露为 glossary canonical `report_date`。
- `REPORT_TIME` 契约转换依据：ClickHouse 抽样确认历史原始值形如 `1990-12-31 00:00:00`；全表非 NULL 值不可解析数量为 0，非午夜时间数量为 0，DateTime distinct 数与 Date distinct 数均为 98。source-to-Parquet 转换对历史非日期标签使用 NULL。
- `EQUITY_RECORD_DATE`、`EX_DIVIDEND_DATE`、`PAY_CASH_DATE`、`GMDECISION_NOTICE_DATE`、`DAT_YAGGR` 的 NULL 是预期缺失，不转默认日期。
- `LAST_TRADE_DATE` 全表 NULL，第一版不建议暴露；如暴露，必须说明全 NULL 现状并免除测试。

## 5. 测试建议

- `security_code`: `not_null`，`cn_security_code_format`。
- `notice_date`: `not_null`。
- `report_period_label`: `not_null`。
- `report_date`: 不加 `not_null`；可加 schema/type 测试确认 raw source 暴露为 `Nullable(Date)`。
- 不对 `security_code`, `report_period_label` 加唯一测试，因为 profile 已发现重复。
- `assign_progress`: 可加 accepted-values，但需把低频失败状态也纳入当前取值集合。
- 所有可选实施日期字段不加 `not_null`。

## 6. 延后事项

- 19 组重复候选键的版本选择和公告优先级。
- 分红方案文本解析，例如 `10派1元(含税)` 的结构化拆解。
- 方案状态跨源统一枚举。
- 与配股、股本变动、公告维表或证券主数据合并。
