# RFC 0048: 同花顺、中债和东方财富数据透传到 marts 层

状态：Implemented With Runtime Verification Follow-Up
日期：2026-07-04
领域：dbt, Dagster, ClickHouse, data-platform
关联系统：pipeline/elt, pipeline/scheduler, pipeline/contracts
相关文档：
- docs/architecture/data-platform.md
- docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md
- docs/ADR/0007-dbt-staging-cleaning-boundary.md
- docs/ADR/0008-raw-source-profiling-before-dbt-staging.md
- docs/RFC/archive/0040-dagster-stg-to-mart-asset-inventory.md
- docs/jobs/reports/2026-07-01-backfill-source-to-marts-controller-dry-run.md
- docs/jobs/reports/2026-07-04-ths-chinabond-f10-marts-passthrough.md
- pipeline/elt/models/staging/ths/stg_ths__limit_up_pool_compacted.sql
- pipeline/elt/models/staging/chinabond/stg_chinabond__government_bond.sql
- pipeline/elt/models/staging/eastmoney/
- pipeline/elt/models/intermediate/int_government_bond_yields_daily.sql
- pipeline/elt/models/intermediate/int_risk_free_rate_daily.sql
- pipeline/elt/models/marts/mart_risk_free_rate_daily.sql
- pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py

## 摘要

当前同花顺涨停池、中债国债收益率曲线和东方财富 9 个 F10 数据集都已进入 raw 和 staging 层，但 marts 暴露不完整：

1. 同花顺只到 `stg_ths__limit_up_pool_compacted`，没有 intermediate 或 mart 下游。
2. 中债已有 `int_government_bond_yields_daily` 透传完整期限收益率曲线，但 mart 只暴露由 1 年期收益率派生的 `mart_risk_free_rate_daily`。
3. 东方财富 9 个 staging 中，7 个被股票股本、除权除息或财务估值 intermediate 间接消费，`cashflow_sq` 和 `cashflow_ytd` 当前没有下游；marts 层只有 `mart_stock_quotes_daily` 间接暴露部分估值结果，没有 9 张 F10 源表的透传入口。

本 RFC 建议新增 thin passthrough 链路：

```text
stg_ths__limit_up_pool_compacted
  -> int_stock_limit_up_pool_daily
  -> mart_stock_limit_up_pool_daily

stg_chinabond__government_bond
  -> int_government_bond_yields_daily
  -> mart_government_bond_yields_daily

stg_eastmoney__{balance,cashflow_sq,cashflow_ytd,dividend_allotment,dividend_main,equity_history,freeholders,income_sq,income_ytd}
  -> int_stock_{business_semantic_name}
  -> mart_stock_{business_semantic_name}
```

`mart_risk_free_rate_daily` 保持现有职责，继续作为 worker-ready 无风险日收益率入口，不承载完整中债收益率曲线。

## 目标

1. 在 marts 层提供同花顺涨停池日频明细稳定入口。
2. 在 marts 层提供中债国债收益率曲线完整期限字段稳定入口。
3. 在 marts 层提供东方财富 9 个 F10 数据集的稳定透传入口。
4. EastMoney 下游 intermediate 和 mart 模型不继续携带数据源名，使用业务语义命名；`_eastmoney` 只保留在 raw/source/staging 层。
5. 保持透传语义：不新增策略打分、题材拆分、期限插值、收益率单位换算、风险利率折算、财务指标重算、分红配股合并或股东身份归并。
6. 东方财富 9 个 staging 的全部字段必须完整传递到对应 marts；资产负债表、利润表、现金流量表等宽表不得裁剪字段。
7. 让 `daily__fetch_history_sources_to_marts_schedule_job` 和 backfill controller 的 THS / ChinaBond / EastMoney scope 覆盖新增 intermediate 和 mart 资产。
8. 为新增模型补齐 YAML 文档、粒度说明和最小数据测试。

## 非目标

1. 不新增同花顺远端字段或修改 `ths__limit_up_pool` / `ths__limit_up_pool_compacted` 数据契约。
2. 不改变中债 raw、staging 和 `int_risk_free_rate_daily` 的现有字段口径。
3. 不在 dbt 中解析 `reason_type` 的题材文本、连板语义或市场标签业务规则。
4. 不把中债 15 年、20 年当前为空的字段填补、插值或删除。
5. 不修改东方财富 9 个数据契约，不新增 raw 字段，不合并财报口径，不从单季度/YTD 表互相推导。
6. 不在 intermediate 或 mart 模型名中保留 `_eastmoney` 数据源标识。
7. 不替代现有 `int_stock_financial_valuation`、`int_stock_exrights_event`、`int_stock_shares_history` 或 `mart_stock_quotes_daily` 的业务派生职责。
8. 不调整 source/raw 回填策略、HTTP client、ClickHouse raw sync 或 Furnace 计算逻辑。

## 当前事实

### dbt 项目分层

[pipeline/elt/dbt_project.yml](../../pipeline/elt/dbt_project.yml) 当前配置：

| 层 | schema | 默认 materialization |
|---|---|---|
| staging | `fleur_staging` | view |
| intermediate | `fleur_intermediate` | view |
| marts | `fleur_marts` | table |

多个 intermediate 模型在 SQL 内显式配置为 table；所有当前 mart 模型均为 table。

### 相关 source 和 contract

| 数据集 | contract 粒度 | dbt source | ClickHouse raw |
|---|---|---|---|
| `ths__limit_up_pool_compacted` | one row per stock code per trade date | `source('raw', 'ths__limit_up_pool_compacted')` | `fleur_raw.ths__limit_up_pool_compacted` |
| `chinabond__government_bond` | one row per ChinaBond government bond yield curve work date | `source('raw', 'chinabond__government_bond')` | `fleur_raw.chinabond__government_bond` |
| `eastmoney__balance` | one row per security code per report date | `source('raw', 'eastmoney__balance')` | `fleur_raw.eastmoney__balance` |
| `eastmoney__cashflow_sq` | one row per security code per report date | `source('raw', 'eastmoney__cashflow_sq')` | `fleur_raw.eastmoney__cashflow_sq` |
| `eastmoney__cashflow_ytd` | one row per security code per report date | `source('raw', 'eastmoney__cashflow_ytd')` | `fleur_raw.eastmoney__cashflow_ytd` |
| `eastmoney__dividend_allotment` | one row per security code per report date | `source('raw', 'eastmoney__dividend_allotment')` | `fleur_raw.eastmoney__dividend_allotment` |
| `eastmoney__dividend_main` | one row per security code per report date | `source('raw', 'eastmoney__dividend_main')` | `fleur_raw.eastmoney__dividend_main` |
| `eastmoney__equity_history` | one row per security code per report date | `source('raw', 'eastmoney__equity_history')` | `fleur_raw.eastmoney__equity_history` |
| `eastmoney__freeholders` | one row per security code per report end date per free-float shareholder rank | `source('raw', 'eastmoney__freeholders')` | `fleur_raw.eastmoney__freeholders` |
| `eastmoney__income_sq` | one row per security code per report date | `source('raw', 'eastmoney__income_sq')` | `fleur_raw.eastmoney__income_sq` |
| `eastmoney__income_ytd` | one row per security code per report date | `source('raw', 'eastmoney__income_ytd')` | `fleur_raw.eastmoney__income_ytd` |

对 THS，`pipeline/elt/models/sources.yml` 暴露 `ths__limit_up_pool_compacted` 给 dbt；日分区稀疏 source `ths__limit_up_pool` 作为 compacted 上游，不直接进入 dbt。EastMoney 9 个年度分区 raw 表已全部暴露给 dbt staging。

### 同花顺当前链路

当前模型：

```text
source('raw', 'ths__limit_up_pool_compacted')
  -> stg_ths__limit_up_pool_compacted
```

`stg_ths__limit_up_pool_compacted` 直接从 raw 选择 22 个字段，将 `date` 规范为 `trade_date`，将本地 A 股代码通过 `normalize_cn_security_code` 规范为 `security_code`，并保留 source-local 字段：

| 字段组 | 字段 |
|---|---|
| 主键 | `trade_date`, `security_code` |
| 证券标识 | `security_name`, `market_id`, `market_type` |
| 涨停状态 | `first_limit_up_time`, `last_limit_up_time`, `open_num`, `limit_up_type`, `is_new`, `is_again_limit` |
| 封单和价格 | `order_volume`, `order_amount`, `latest_price`, `currency_value` |
| 行情表现 | `change_rate`, `turnover_rate`, `limit_up_success_rate` |
| 文本标签 | `reason_type`, `high_days`, `high_days_value_raw`, `change_tag` |

当前没有任何 `ref('stg_ths__limit_up_pool_compacted')` 下游。编排层中 `MARKET_EVENTS_SCOPE` 当前只包含 staging：

```text
THS_STAGING_ASSET_KEYS = ("stg_ths__limit_up_pool_compacted",)
STAGE_DBT_INTERMEDIATE = ()
STAGE_DBT_MARTS = ()
```

### 中债当前链路

当前模型：

```text
source('raw', 'chinabond__government_bond')
  -> stg_chinabond__government_bond
  -> int_government_bond_yields_daily
  -> int_risk_free_rate_daily
  -> mart_risk_free_rate_daily
```

`stg_chinabond__government_bond` 将 raw `work_date` 规范为 `trade_date`，并保留完整期限收益率字段：

```text
three_month_yield_pct
six_month_yield_pct
one_year_yield_pct
two_year_yield_pct
three_year_yield_pct
five_year_yield_pct
seven_year_yield_pct
ten_year_yield_pct
fifteen_year_yield_pct
twenty_year_yield_pct
thirty_year_yield_pct
```

`int_government_bond_yields_daily` 是 table，按 `toYear(trade_date)` 分区、`trade_date` 排序，直接透传 staging 完整期限曲线，不做期限结构派生、插值或单位换算。

`int_risk_free_rate_daily` 是业务派生模型：只使用 `one_year_yield_pct`，将百分比点转换为小数比例 `annual_rate`，按 A 股交易日 forward-fill，并派生 `daily_rate`。

`mart_risk_free_rate_daily` 是 `int_risk_free_rate_daily` 的 thin mart，字段为：

```text
trade_date, source_date, source_tenor, annual_rate, daily_rate
```

因此当前 marts 层没有完整中债收益率曲线入口。

编排层中 `CHINABOND_SCOPE` 当前 marts 只包含 risk-free：

```text
CHINABOND_INTERMEDIATE_ASSET_KEYS = (
    "int_government_bond_yields_daily",
    "int_risk_free_rate_daily",
)
CHINABOND_MART_ASSET_KEYS = ("mart_risk_free_rate_daily",)
```

### 东方财富当前链路

当前 9 个 EastMoney F10 staging 模型都直接读取 `source('raw', 'eastmoney__*')`，并将 `SECUCODE` 规范为 canonical `security_code`。其中现金流量表、利润表和资产负债表还保留 `security_local_code`、`report_date`、`notice_date`、`update_date` 等字段；分红配股、股本历史和前十大流通股东模型保留各自 source-local 字段。

当前链路：

```text
source('raw', 'eastmoney__balance')
  -> stg_eastmoney__balance
  -> int_stock_financial_valuation
  -> mart_stock_quotes_daily

source('raw', 'eastmoney__income_sq')
  -> stg_eastmoney__income_sq
  -> int_stock_financial_valuation
  -> mart_stock_quotes_daily

source('raw', 'eastmoney__income_ytd')
  -> stg_eastmoney__income_ytd
  -> int_stock_financial_valuation
  -> mart_stock_quotes_daily

source('raw', 'eastmoney__dividend_main')
  -> stg_eastmoney__dividend_main
  -> int_stock_exrights_event
  -> int_stock_quotes_daily_unadj / int_stock_quotes_daily_adj
  -> mart_stock_quotes_daily

source('raw', 'eastmoney__dividend_allotment')
  -> stg_eastmoney__dividend_allotment
  -> int_stock_exrights_event
  -> int_stock_quotes_daily_unadj / int_stock_quotes_daily_adj
  -> mart_stock_quotes_daily

source('raw', 'eastmoney__equity_history')
  -> stg_eastmoney__equity_history
  -> int_stock_shares_history
  -> int_stock_quotes_daily_unadj / int_stock_financial_valuation
  -> mart_stock_quotes_daily

source('raw', 'eastmoney__freeholders')
  -> stg_eastmoney__freeholders
  -> int_stock_shares_history
  -> int_stock_quotes_daily_unadj / int_stock_financial_valuation
  -> mart_stock_quotes_daily

source('raw', 'eastmoney__cashflow_sq')
  -> stg_eastmoney__cashflow_sq

source('raw', 'eastmoney__cashflow_ytd')
  -> stg_eastmoney__cashflow_ytd
```

这说明当前 EastMoney 只通过业务派生链路进入 `mart_stock_quotes_daily`，没有逐源表、逐粒度的 marts 透传入口。现金流量表两张 staging 当前没有 intermediate 或 mart 下游。

编排层中 `EASTMONEY_F10_SCOPE` 当前包含 9 个 staging、6 个股票业务 intermediate 和 1 个 mart：

```text
EASTMONEY_STAGING_ASSET_KEYS = (
    "stg_eastmoney__balance",
    "stg_eastmoney__cashflow_sq",
    "stg_eastmoney__cashflow_ytd",
    "stg_eastmoney__dividend_allotment",
    "stg_eastmoney__dividend_main",
    "stg_eastmoney__equity_history",
    "stg_eastmoney__freeholders",
    "stg_eastmoney__income_sq",
    "stg_eastmoney__income_ytd",
)
EASTMONEY_INTERMEDIATE_ASSET_KEYS = (
    "int_stock_shares_history",
    "int_stock_exrights_event",
    "int_stock_quotes_daily_unadj",
    "int_stock_adjustment_factor",
    "int_stock_quotes_daily_adj",
    "int_stock_financial_valuation",
)
EASTMONEY_MART_ASSET_KEYS = ("mart_stock_quotes_daily",)
```

## stg 和 int 层资源盘点

### Staging

当前 `pipeline/elt/models/staging/` 有 17 个 SQL 模型，均属于 dbt staging 层。

| 模型 | 来源域 | 当前下游情况 |
|---|---|---|
| `stg_baostock__query_history_k_data_plus_daily` | BaoStock | 股票/指数行情 intermediate |
| `stg_baostock__query_stock_basic` | BaoStock | 股票/指数基础信息 intermediate |
| `stg_chinabond__government_bond` | ChinaBond | `int_government_bond_yields_daily` |
| `stg_eastmoney__balance` | EastMoney | `int_stock_financial_valuation` |
| `stg_eastmoney__cashflow_sq` | EastMoney | 当前无 intermediate 下游 |
| `stg_eastmoney__cashflow_ytd` | EastMoney | 当前无 intermediate 下游 |
| `stg_eastmoney__dividend_allotment` | EastMoney | `int_stock_exrights_event` |
| `stg_eastmoney__dividend_main` | EastMoney | `int_stock_exrights_event` |
| `stg_eastmoney__equity_history` | EastMoney | `int_stock_shares_history` |
| `stg_eastmoney__freeholders` | EastMoney | `int_stock_shares_history` |
| `stg_eastmoney__income_sq` | EastMoney | `int_stock_financial_valuation` |
| `stg_eastmoney__income_ytd` | EastMoney | `int_stock_financial_valuation` |
| `stg_jiuyan__action_field_compacted` | JiuYan | 当前无 intermediate 下游 |
| `stg_jiuyan__industry_list` | JiuYan | 当前无 intermediate 下游 |
| `stg_jiuyan__industry_ocr_snapshot` | JiuYan | 当前无 intermediate 下游 |
| `stg_sina__trade_calendar` | Sina | `int_trade_calendar` |
| `stg_ths__limit_up_pool_compacted` | THS | 当前无 intermediate 下游 |

### EastMoney 9 个 staging 盘点

| staging 模型 | staging 粒度或自然键 | 当前业务下游 | 本 RFC 目标 mart |
|---|---|---|---|
| `stg_eastmoney__balance` | `security_code`, `report_date` | `int_stock_financial_valuation` | `mart_stock_balance_sheet` |
| `stg_eastmoney__cashflow_sq` | `security_code`, `report_date` | 无 | `mart_stock_cashflow_statement_quarterly` |
| `stg_eastmoney__cashflow_ytd` | `security_code`, `report_date` | 无 | `mart_stock_cashflow_statement_ytd` |
| `stg_eastmoney__dividend_allotment` | `security_code`, `notice_date`, `event_explain` | `int_stock_exrights_event` | `mart_stock_allotment_event` |
| `stg_eastmoney__dividend_main` | source-local event rows，包含 `security_code`, `notice_date`, `report_date`, `announcement_identifier` | `int_stock_exrights_event` | `mart_stock_dividend_plan` |
| `stg_eastmoney__equity_history` | `security_code`, `end_date` | `int_stock_shares_history` | `mart_stock_share_capital_history` |
| `stg_eastmoney__freeholders` | `security_code`, `report_date`, `holder_rank`, `holder_identifier`, `holder_name`, `shares_type` | `int_stock_shares_history` | `mart_stock_free_float_shareholder_top10` |
| `stg_eastmoney__income_sq` | `security_code`, `report_date` | `int_stock_financial_valuation` | `mart_stock_income_statement_quarterly` |
| `stg_eastmoney__income_ytd` | `security_code`, `report_date` | `int_stock_financial_valuation` | `mart_stock_income_statement_ytd` |

这些目标 mart 是 F10 业务语义透传入口，不替代现有股票业务 intermediate；现有业务模型继续承担合并、派生和跨源建模职责。

### Intermediate

当前 `pipeline/elt/models/intermediate/` 有 25 个 SQL 模型，其中 `int_portfolio_performance_metric_rank_catalog` 是 ephemeral catalog，不注册为 Dagster asset；其余 24 个是 intermediate asset。

| 模型 | materialization | 主要来源或职责 |
|---|---|---|
| `int_benchmark_basic_snapshot` | table | 从指数基础信息筛选 benchmark |
| `int_benchmark_returns_daily` | table | benchmark 指数日收益 |
| `int_government_bond_yields_daily` | table | ChinaBond 完整期限收益率曲线透传 |
| `int_index_basic_snapshot` | table | BaoStock 指数基础信息 |
| `int_index_quotes_daily` | table | BaoStock 指数日频行情和收益 |
| `int_portfolio_closed_trade` | view | worker 已平仓交易薄封装 |
| `int_portfolio_performance_metric` | view | worker 组合绩效指标薄封装 |
| `int_portfolio_performance_metric_rank_catalog` | ephemeral | portfolio performance ranking direction catalog |
| `int_portfolio_performance_metric_status` | view | portfolio metric NULL 状态说明 |
| `int_portfolio_trade_metric` | view | worker 交易质量指标薄封装 |
| `int_risk_free_rate_daily` | table | ChinaBond 1y -> worker-ready 无风险日收益 |
| `int_stock_adjustment_factor` | table | 股票复权因子 |
| `int_stock_basic_snapshot` | table | 股票基础信息当前快照 |
| `int_stock_boll_daily` | view | Furnace BOLL wrapper |
| `int_stock_exrights_event` | table | 分红配股合并后的除权除息事件 |
| `int_stock_financial_valuation` | table | 报告期估值和盈利能力指标 |
| `int_stock_kdj_daily` | view | Furnace KDJ wrapper |
| `int_stock_ma_daily` | view | Furnace MA/EMA wrapper |
| `int_stock_macd_daily` | view | Furnace MACD wrapper |
| `int_stock_price_pattern_daily` | view | Furnace 价格形态 wrapper |
| `int_stock_quotes_daily_adj` | table | 复权日行情 |
| `int_stock_quotes_daily_unadj` | table | 未复权日行情、涨跌停、市值和股本口径 |
| `int_stock_rsi_daily` | view | Furnace RSI wrapper |
| `int_stock_shares_history` | table | 股本有效区间和自由流通股本估算 |
| `int_trade_calendar` | table | A 股交易日历和前一交易日 |

## 设计

### 新增 `int_stock_limit_up_pool_daily`

新增 intermediate table：

```text
pipeline/elt/models/intermediate/int_stock_limit_up_pool_daily.sql
pipeline/elt/models/intermediate/int_stock_limit_up_pool_daily.yml
```

建议配置：

```jinja
{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}
```

字段从 `stg_ths__limit_up_pool_compacted` 直接选择，不解析 `reason_type`，不重新命名已在 staging 中规范好的 canonical 字段：

```text
trade_date
security_code
security_name
first_limit_up_time
last_limit_up_time
open_num
limit_up_type
order_volume
order_amount
is_new
is_again_limit
limit_up_success_rate
currency_value
market_id
market_type
change_rate
turnover_rate
reason_type
high_days
high_days_value_raw
change_tag
latest_price
```

粒度：每个交易日、每只证券一行。

最小测试：

1. `unique_combination_of_columns(trade_date, security_code)`。
2. `trade_date` not null。
3. `security_code` not null + `cn_security_code_format`。
4. `mart_stock_limit_up_pool_daily` 复用同一组 grain 和 not null 测试，避免 thin mart 降低约束。
5. 对 staging 已有 accepted_values 的枚举字段继续保留或下推到 mart，避免新增模型降低约束。

### 新增 `mart_stock_limit_up_pool_daily`

新增 mart table：

```text
pipeline/elt/models/marts/mart_stock_limit_up_pool_daily.sql
pipeline/elt/models/marts/mart_stock_limit_up_pool_daily.yml
```

建议配置与 intermediate 相同：

```jinja
{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='(trade_date, security_code)',
    partition_by='toYear(trade_date)'
) }}
```

职责：从 `int_stock_limit_up_pool_daily` 选择稳定字段，作为消费层读取同花顺涨停池的唯一 mart 入口。

命名不采用 `mart_ths__limit_up_pool_compacted`，原因是 marts 层不暴露 raw/compacted 物理细节；同花顺来源在 YAML 描述和字段 meta 中保留。

### 新增 `mart_government_bond_yields_daily`

新增 mart table：

```text
pipeline/elt/models/marts/mart_government_bond_yields_daily.sql
pipeline/elt/models/marts/mart_government_bond_yields_daily.yml
```

建议配置：

```jinja
{{ config(
    materialized='table',
    engine='MergeTree()',
    order_by='trade_date',
    partition_by='toYear(trade_date)'
) }}
```

字段从 `int_government_bond_yields_daily` 直接选择：

```text
trade_date
three_month_yield_pct
six_month_yield_pct
one_year_yield_pct
two_year_yield_pct
three_year_yield_pct
five_year_yield_pct
seven_year_yield_pct
ten_year_yield_pct
fifteen_year_yield_pct
twenty_year_yield_pct
thirty_year_yield_pct
```

粒度：每个 ChinaBond 国债收益率曲线日期一行。

最小测试：

1. `trade_date` not null + unique。
2. 对当前上游已 not null 的期限字段保留 not null：3m、6m、1y、2y、3y、5y、7y、10y、30y。
3. 15y 和 20y 继续 nullable，不做 not null。

### 新增 F10 9 个 thin intermediate

新增 9 个 intermediate table：

```text
pipeline/elt/models/intermediate/int_stock_balance_sheet.sql
pipeline/elt/models/intermediate/int_stock_cashflow_statement_quarterly.sql
pipeline/elt/models/intermediate/int_stock_cashflow_statement_ytd.sql
pipeline/elt/models/intermediate/int_stock_allotment_event.sql
pipeline/elt/models/intermediate/int_stock_dividend_plan.sql
pipeline/elt/models/intermediate/int_stock_share_capital_history.sql
pipeline/elt/models/intermediate/int_stock_free_float_shareholder_top10.sql
pipeline/elt/models/intermediate/int_stock_income_statement_quarterly.sql
pipeline/elt/models/intermediate/int_stock_income_statement_ytd.sql
```

这些模型只从对应 `stg_eastmoney__*` 显式选择字段，不使用 `select *`，不新增跨表 join，不合并单季度/YTD，不从分红主表和配股表重建除权事件，不归并前十大流通股东身份。模型名使用业务语义，不携带 `_eastmoney` 数据源标识。

建议物化：

| 模型 | order_by | partition_by |
|---|---|---|
| `int_stock_balance_sheet` | `(security_code, report_date)` | `toYear(report_date)` |
| `int_stock_cashflow_statement_quarterly` | `(security_code, report_date)` | `toYear(report_date)` |
| `int_stock_cashflow_statement_ytd` | `(security_code, report_date)` | `toYear(report_date)` |
| `int_stock_allotment_event` | `(security_code, notice_date)` | `toYear(notice_date)` |
| `int_stock_dividend_plan` | `(security_code, notice_date)` | `toYear(notice_date)` |
| `int_stock_share_capital_history` | `(security_code, end_date)` | `toYear(end_date)` |
| `int_stock_free_float_shareholder_top10` | `(security_code, report_date, holder_rank)` | `toYear(report_date)` |
| `int_stock_income_statement_quarterly` | `(security_code, report_date)` | `toYear(report_date)` |
| `int_stock_income_statement_ytd` | `(security_code, report_date)` | `toYear(report_date)` |

字段策略：

1. 全量传递 staging 已规范字段，保留 canonical 字段名、单位后缀和 `_pct` 口径。
2. intermediate 和 mart 模型名不得携带数据源标识；字段名也不得新增数据源标识。
3. staging 层先完成 source-local canonical 命名；`HOLDER_NEW -> holder_identifier`、`INFO_CODE -> announcement_identifier` 这类确定性字段重命名不下推到 intermediate/mart。
4. 不把 `security_name_abbr`、`report_date_name`、`assign_progress`、`shares_type` 等 source-local 文本提升为跨源维表。
5. 9 个 intermediate 必须覆盖对应 staging 模型的全部字段；允许受控业务重命名，但必须在 YAML 中保留字段级 lineage。
6. 对现金流量表、利润表和资产负债表这类宽表，SQL 必须显式列出 staging YAML 中的全部字段，不得裁剪任何财务字段、同比/环比字段或说明字段。
7. 如果后续 contract 新增 raw 字段，先更新 staging，再受控新增 int/mart 字段；不得通过 `select *` 自动扩散到 marts。
8. 实施时必须补一个机械字段覆盖校验，建议路径为 `pipeline/elt/scripts/validate_f10_passthrough_coverage.py`：读取 dbt manifest 和 9 组 staging/int/mart YAML，确认每个 staging 字段都有一个 downstream base 字段或明确受控重命名，新增派生键只允许出现在白名单中，模型名和字段名不含 `_eastmoney`。
9. 受控重命名前移到 staging 后，int/mart 直接透传 canonical staging 字段；staging YAML 继续作为 raw `source_columns` lineage 的权威位置。

最小测试：

| 模型 | 唯一性测试 |
|---|---|
| `int_stock_balance_sheet` | `security_code`, `report_date` |
| `int_stock_cashflow_statement_quarterly` | `security_code`, `report_date` |
| `int_stock_cashflow_statement_ytd` | `security_code`, `report_date` |
| `int_stock_allotment_event` | `security_code`, `notice_date`, `event_explain` |
| `int_stock_dividend_plan` | `dividend_plan_record_key`，见下方唯一性设计 |
| `int_stock_share_capital_history` | `security_code`, `end_date` |
| `int_stock_free_float_shareholder_top10` | `security_code`, `report_date`, `holder_rank`, `holder_identifier`, `holder_name`, `shares_type` |
| `int_stock_income_statement_quarterly` | `security_code`, `report_date` |
| `int_stock_income_statement_ytd` | `security_code`, `report_date` |

所有 9 个模型至少保留 `security_code` not null + `cn_security_code_format`；日期主键字段按 staging 当前约束保留 not null。
对应 9 个 mart 至少复用 intermediate 的 grain 唯一性测试、`security_code` not null/格式测试和日期主键 not null 测试；thin mart 不应只依赖 intermediate 测试。

### 新增 F10 9 个 marts

新增 9 个 mart table：

```text
pipeline/elt/models/marts/mart_stock_balance_sheet.sql
pipeline/elt/models/marts/mart_stock_cashflow_statement_quarterly.sql
pipeline/elt/models/marts/mart_stock_cashflow_statement_ytd.sql
pipeline/elt/models/marts/mart_stock_allotment_event.sql
pipeline/elt/models/marts/mart_stock_dividend_plan.sql
pipeline/elt/models/marts/mart_stock_share_capital_history.sql
pipeline/elt/models/marts/mart_stock_free_float_shareholder_top10.sql
pipeline/elt/models/marts/mart_stock_income_statement_quarterly.sql
pipeline/elt/models/marts/mart_stock_income_statement_ytd.sql
```

每个 mart 从对应 `int_stock_*` 选择稳定字段，作为消费层读取 9 张 F10 数据的入口。mart YAML 必须说明：

1. 来源是 F10 source-local 数据；数据源事实放在 YAML meta/source lineage，不放进模型名。
2. 粒度继承对应 staging/int。
3. 不重算财务指标、不合并报表口径、不合并分红配股事件、不裁剪前十大流通股东名次。
4. 单位、同比/环比字段和百分比点口径继承 staging 字段说明。
5. mart 必须包含对应 intermediate 的全部字段；也就是 staging -> int -> mart 三层字段覆盖无缺失，受控重命名字段必须有 lineage。

这些 mart 采用 `mart_stock_*` 业务语义命名，但仍是 F10 透传入口；跨源业务口径、跨源合并或指标派生仍由现有或后续独立模型承担。

### `int_stock_dividend_plan` 唯一性设计

`stg_eastmoney__dividend_main` 的历史记录不能用单一业务自然键静默去重。现有 raw profile 已确认：

1. `SECUCODE, REPORT_DATE` 有 19 组重复键，单键最大 2 行。
2. `INFO_CODE` 是高基数字段，但 nullable；历史 profile 中 `INFO_CODE` NULL 70,499 行，不能作为全表唯一键。
3. 同一证券、同一报告期可能存在不同公告进度或历史版本；历史记录也可能包含错误版本，透传层不能判断哪个版本是正确值。

因此 `int_stock_dividend_plan` 采用两层键：

| 字段 | 生成依据 | 语义 | 唯一性 |
|---|---|---|---|
| `dividend_plan_record_key` | 对 staging 全部字段在 downstream 的 base 表达做确定性 fingerprint；受控重命名使用下游字段名，字段顺序固定，NULL 显式编码，不包含本键、`dividend_plan_group_key` 或其他派生键 | 一条规范化分红方案记录的内容身份 | `unique` |
| `dividend_plan_group_key` | `security_code`, `report_period_label` | 同一证券同一报告期的分红方案版本组 | 不唯一 |
| `announcement_identifier` | staging `announcement_identifier`，来源 raw `INFO_CODE` | 公告业务标识；字段名不携带数据源标识 | 不唯一、nullable |

实现约束：

1. `int_stock_dividend_plan` 不按 `assign_progress`、`notice_date`、`gmdecision_notice_date` 或 `ex_dividend_date` 选择最新或最可信记录。
2. `int_stock_dividend_plan` 可以对完全相同的 normalized row 使用 `select distinct` 去重，再生成 `dividend_plan_record_key`；这只去除重复物理行，不修正历史业务记录。
3. 如果 profile 发现完全相同 normalized row 需要保留重复次数，则当前 raw/staging 缺少稳定 ingestion row id，无法构造可复现的物理行唯一键；应先补 source/raw ingestion metadata，而不是在 dbt 中用不稳定排序生成序号。
4. `mart_stock_dividend_plan` 透传 `dividend_plan_record_key`、`dividend_plan_group_key`、`announcement_identifier` 和全部 staging 字段，供消费者自行决定是否按版本组取最新。
5. `announcement_identifier` 已在 staging 暴露；字段覆盖校验要求 int/mart 直接透传该 canonical 字段，不再保留 `info_code`。

实施前必须补跑 profile：

```sql
select
    count() as rows,
    countIf(announcement_identifier is null) as announcement_identifier_null_rows,
    uniqExactIf(announcement_identifier, announcement_identifier is not null) as distinct_announcement_identifier_nonnull
from {{ ref('stg_eastmoney__dividend_main') }};
```

```sql
select
    count() as duplicate_group_count,
    max(row_count) as max_rows_per_group
from (
    select security_code, report_period_label, count() as row_count
    from {{ ref('stg_eastmoney__dividend_main') }}
    group by security_code, report_period_label
    having row_count > 1
);
```

```sql
select
    count() as duplicate_record_fingerprint_count,
    max(row_count) as max_rows_per_fingerprint
from (
    select
        security_code,
        security_name_abbr,
        notice_date,
        report_period_label,
        report_date,
        assign_progress,
        is_unassign,
        impl_plan_profile,
        impl_plan_newprofile,
        new_profile,
        assign_object,
        equity_record_date,
        ex_dividend_date,
        pay_cash_date,
        gmdecision_notice_date,
        annual_general_meeting_date,
        announcement_identifier,
        total_dividend,
        total_dividend_a,
        count() as row_count
    from {{ ref('stg_eastmoney__dividend_main') }}
    group by
        security_code,
        security_name_abbr,
        notice_date,
        report_period_label,
        report_date,
        assign_progress,
        is_unassign,
        impl_plan_profile,
        impl_plan_newprofile,
        new_profile,
        assign_object,
        equity_record_date,
        ex_dividend_date,
        pay_cash_date,
        gmdecision_notice_date,
        annual_general_meeting_date,
        announcement_identifier,
        total_dividend,
        total_dividend_a
    having row_count > 1
);
```

### 保留 `mart_risk_free_rate_daily`

`mart_risk_free_rate_daily` 不改名、不并入 `mart_government_bond_yields_daily`。两者职责不同：

| mart | 职责 | 单位 |
|---|---|---|
| `mart_government_bond_yields_daily` | 完整中债国债收益率曲线透传 | 百分比点 |
| `mart_risk_free_rate_daily` | worker-ready 无风险日收益率 | 小数比例 |

后续扩展多期限 risk-free 时，应先扩展 `int_risk_free_rate_daily` 的期限选择逻辑，再受控扩展 `mart_risk_free_rate_daily`，不从完整收益率 mart 反推业务口径。

## 编排影响

需要同步更新 [source_to_marts_backfill.py](../../pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py)：

```text
THS_INTERMEDIATE_ASSET_KEYS = ("int_stock_limit_up_pool_daily",)
THS_MART_ASSET_KEYS = ("mart_stock_limit_up_pool_daily",)

CHINABOND_MART_ASSET_KEYS = (
    "mart_government_bond_yields_daily",
    "mart_risk_free_rate_daily",
)

F10_PASSTHROUGH_INTERMEDIATE_ASSET_KEYS = (
    "int_stock_balance_sheet",
    "int_stock_cashflow_statement_quarterly",
    "int_stock_cashflow_statement_ytd",
    "int_stock_allotment_event",
    "int_stock_dividend_plan",
    "int_stock_share_capital_history",
    "int_stock_free_float_shareholder_top10",
    "int_stock_income_statement_quarterly",
    "int_stock_income_statement_ytd",
)

F10_PASSTHROUGH_MART_ASSET_KEYS = (
    "mart_stock_balance_sheet",
    "mart_stock_cashflow_statement_quarterly",
    "mart_stock_cashflow_statement_ytd",
    "mart_stock_allotment_event",
    "mart_stock_dividend_plan",
    "mart_stock_share_capital_history",
    "mart_stock_free_float_shareholder_top10",
    "mart_stock_income_statement_quarterly",
    "mart_stock_income_statement_ytd",
)
```

并将 `MARKET_EVENTS_SCOPE` 从 only-staging 改为：

```text
STAGE_DBT_STAGING: THS_STAGING_ASSET_KEYS
STAGE_DBT_INTERMEDIATE: THS_INTERMEDIATE_ASSET_KEYS
STAGE_DBT_MARTS: THS_MART_ASSET_KEYS
```

并将 `EASTMONEY_F10_SCOPE` 扩展为：

```text
STAGE_DBT_STAGING: EASTMONEY_STAGING_ASSET_KEYS
STAGE_DBT_INTERMEDIATE: EASTMONEY_INTERMEDIATE_ASSET_KEYS + F10_PASSTHROUGH_INTERMEDIATE_ASSET_KEYS
STAGE_DBT_MARTS: EASTMONEY_MART_ASSET_KEYS + F10_PASSTHROUGH_MART_ASSET_KEYS
```

这样 daily controller 和 history source-to-marts backfill 展开时，THS、ChinaBond 和 EastMoney 新 mart 会与各自 raw/stg/int 链路保持一致。

## 实施计划

1. 新增 `int_stock_limit_up_pool_daily` SQL/YAML。
2. 新增 `mart_stock_limit_up_pool_daily` SQL/YAML。
3. 新增 `mart_government_bond_yields_daily` SQL/YAML。
4. 新增 9 个 F10 `int_stock_*` SQL/YAML，模型名不带 `_eastmoney`。
5. 新增 9 个 F10 `mart_stock_*` SQL/YAML，模型名不带 `_eastmoney`。
6. 更新 `source_to_marts_backfill.py` 的 THS / ChinaBond / EastMoney scope 常量。
7. 使用定向 dbt selector 验证新增链路和字段完整性。
8. 使用 controller dry-run 验证 source-to-marts 展开包含新增 THS、ChinaBond 和 F10 marts。

## 验证

dbt 最小验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_limit_up_pool_daily mart_stock_limit_up_pool_daily mart_government_bond_yields_daily int_stock_balance_sheet int_stock_cashflow_statement_quarterly int_stock_cashflow_statement_ytd int_stock_allotment_event int_stock_dividend_plan int_stock_share_capital_history int_stock_free_float_shareholder_top10 int_stock_income_statement_quarterly int_stock_income_statement_ytd mart_stock_balance_sheet mart_stock_cashflow_statement_quarterly mart_stock_cashflow_statement_ytd mart_stock_allotment_event mart_stock_dividend_plan mart_stock_share_capital_history mart_stock_free_float_shareholder_top10 mart_stock_income_statement_quarterly mart_stock_income_statement_ytd
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_limit_up_pool_daily --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_government_bond_yields_daily --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_balance_sheet --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_cashflow_statement_quarterly --limit 10
```

字段完整性验收：

1. 对 9 条 F10 透传链路逐一核对 staging 字段在 intermediate 和 mart 中都有一一对应字段；受控重命名必须有 YAML `meta.source_columns` 指向上游 staging model column。
2. 对 `stg_eastmoney__balance`、`stg_eastmoney__cashflow_sq`、`stg_eastmoney__cashflow_ytd`、`stg_eastmoney__income_sq`、`stg_eastmoney__income_ytd` 这 5 张宽表，额外核对列数一致，确保没有遗漏任何财务宽字段。
3. 字段完整性必须通过机械校验脚本，而不是只靠人工 review；人工核对只作为补充说明。
4. 任一字段无法透传时，必须在实施报告中说明字段名、原因和修复计划；不得默默裁剪。

`int_stock_dividend_plan` 唯一性验收：

1. `dividend_plan_record_key` 必须通过 unique test。
2. `dividend_plan_group_key` 允许重复；重复表示同一证券同一报告期存在多个公告版本或历史版本。
3. `announcement_identifier` 不做 unique / not_null 测试；它只作为可观察公告编号字段。
4. 若 normalized row fingerprint 发现完全重复，第一版允许对完全相同记录 `select distinct` 后再生成 `dividend_plan_record_key`；不得按业务字段选择所谓正确历史版本。

编排最小验证：

```bash
cd pipeline
uv run ruff check scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py scheduler/tests
uv run pytest scheduler/tests -k source_to_marts
cd scheduler
uv run dg check defs
```

文档和格式验证：

```bash
make docs-check
git diff --check
```

如果 ClickHouse 或上游 raw 数据不可用，实施报告必须记录无法执行 `dbt build/show` 的原因，并至少保留 `dbt parse`、相关 Python tests 和 controller dry-run 输出。

## 待决问题

1. `mart_stock_limit_up_pool_daily` 是否应补充消费端索引字段，例如 `limit_up_rank` 或 `first_limit_up_time` 排序序号；本 RFC 第一版不新增派生字段。
2. THS `reason_type` 的文本规范化是否进入后续 JiuYan/THS 市场事件统一 mart；需要另起 RFC，不能在本次透传中隐式实现。
3. ChinaBond 完整曲线是否需要后续增加长表形态 `mart_government_bond_yield_tenor_daily`；第一版先提供宽表透传，避免破坏现有字段事实。
4. `int_stock_dividend_plan` 是否需要后续派生 latest/effective 版本视图；本 RFC 的透传 mart 不做历史纠错。
5. F10 9 张 passthrough mart 后续是否需要再抽象为跨源 `mart_stock_financial_statement_*`；本 RFC 不做跨源业务口径。
