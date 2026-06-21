# int_government_bond_yields_daily 设计

状态：Design

依据：

- Staging model：`ref('stg_chinabond__government_bond')`
- Staging 设计：`docs/design/dbt_layer/fleur_staging/stg_chinabond__government_bond.md`
- 目标位置：`pipeline/elt/models/intermediate/int_government_bond_yields_daily.sql`
- 实施计划：`docs/plans/0042-chinabond-government-bond-s3-raw-implementation-plan.md`
- 关联 Q&A：`docs/Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md`

## 1. 模型定位

ChinaBond 中债国债收益率曲线日频 intermediate 模型。模型透传 staging 层的完整期限结构收益率，不做插值、单位换算、期限利差派生或交易日历对齐，作为后续 risk-free rate mart / analytics 层和组合绩效计算的稳定输入。

本模型不裁决无风险利率口径，也不输出特定期限别名（如 `risk_free_rate_1y`）。下游 mart / analytics / portfolio worker 在选择具体期限和折算口径时，自行从本模型读取所需字段并在自身层面声明语义。

## 2. 数据粒度与依赖

- 直接依赖：`stg_chinabond__government_bond`。
- 粒度：一行一个 ChinaBond 国债收益率曲线工作日，与 staging 一致。
- 候选键：`trade_date`。
- 收益率口径：百分比点（如 `2.85` 表示 2.85%），不转换为小数比例。
- 物化：ClickHouse `MergeTree()` table。
- 排序键：`trade_date`。
- 分区：`toYear(trade_date)`。

## 3. 字段设计

| 字段 | 来源/派生 | 类型建议 | 设计说明 |
|---|---|---|---|
| `trade_date` | staging | `Date` | ChinaBond 国债收益率曲线日期。 |
| `three_month_yield_pct` | staging | `Nullable(Float64)` | 3 个月期国债收益率，百分比点。 |
| `six_month_yield_pct` | staging | `Nullable(Float64)` | 6 个月期国债收益率，百分比点。 |
| `one_year_yield_pct` | staging | `Nullable(Float64)` | 1 年期国债收益率，百分比点；推荐作为默认 risk-free rate 候选期限，最终口径在 mart 层裁决。 |
| `two_year_yield_pct` | staging | `Nullable(Float64)` | 2 年期国债收益率，百分比点。 |
| `three_year_yield_pct` | staging | `Nullable(Float64)` | 3 年期国债收益率，百分比点。 |
| `five_year_yield_pct` | staging | `Nullable(Float64)` | 5 年期国债收益率，百分比点。 |
| `seven_year_yield_pct` | staging | `Nullable(Float64)` | 7 年期国债收益率，百分比点。 |
| `ten_year_yield_pct` | staging | `Nullable(Float64)` | 10 年期国债收益率，百分比点。 |
| `fifteen_year_yield_pct` | staging | `Nullable(Float64)` | 15 年期国债收益率；当前上游全 NULL，保留 nullable。 |
| `twenty_year_yield_pct` | staging | `Nullable(Float64)` | 20 年期国债收益率；当前上游全 NULL，保留 nullable。 |
| `thirty_year_yield_pct` | staging | `Nullable(Float64)` | 30 年期国债收益率，百分比点。 |

## 4. SQL 逻辑

```sql
with government_bond_yields as (
    select ...
    from {{ ref('stg_chinabond__government_bond') }}
)

select ...
from government_bond_yields
```

实现注意：

- 直接透传 staging 全部字段，不在 intermediate 层引入 raw 或 source 引用。
- 不做缺失日期 forward-fill、不做交易日历 join，缺口由下游决定补齐策略。
- 不在本模型构造期限利差（如 `ten_minus_two_year_spread`）或曲线形态指标。
- 不输出特定期限的语义别名（如 `risk_free_rate_pct`），避免在 intermediate 层固化无风险利率口径。

## 5. 测试建议

- `trade_date`: `not_null`，`unique`。
- 3M / 6M / 1Y / 2Y / 3Y / 5Y / 7Y / 10Y / 30Y 收益率：`not_null`。
- 15Y、20Y：不加 `not_null`，承认上游当前全 NULL。
- 模型对齐测试：`int_government_bond_yields_daily` 与 `stg_chinabond__government_bond` 的 `trade_date` 集合完全一致，由 `pipeline/elt/tests/intermediate/int_government_bond_yields_daily_matches_staging.sql` 覆盖。

## 6. 延后事项

- 默认 risk-free rate 期限选择和日频折算（如 `(1 + y_pct/100) ** (1/365) - 1`）口径，落到 mart / analytics 层。
- 交易日历对齐、forward-fill 或区间生效策略，落到 mart 或 portfolio worker 层。
- 期限利差、收益率曲线斜率 / 曲率派生指标。
- 多曲线扩展（如国开债、信用债收益率曲线）和跨曲线 mart 层归一。
- 与组合绩效指标（Sharpe、CAPM alpha 等）相关的最终 risk-free 口径，在 portfolio data plane 设计中决定。
