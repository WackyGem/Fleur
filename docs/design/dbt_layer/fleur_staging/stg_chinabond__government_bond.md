# stg_chinabond__government_bond 设计

状态：Design

依据：

- Raw profile：`docs/references/raw_profile/chinabond__government_bond.md`
- Raw source：`source('raw', 'chinabond__government_bond')`
- 数据契约：`pipeline/contracts/datasets/chinabond__government_bond.yml`
- 目标位置：`pipeline/elt/models/staging/chinabond/stg_chinabond__government_bond.sql`
- 实施计划：`docs/plans/0042-chinabond-government-bond-s3-raw-implementation-plan.md`

## 1. 模型定位

ChinaBond 中债国债收益率曲线 source-local staging model。staging 仅做 raw 字段透传和列重命名，不做期限结构派生、单位换算、缺失值插值或跨源利率归一。

本模型只表达 ChinaBond source-local 国债收益率曲线，不裁决无风险利率口径，也不替代 mart / analytics 层的 risk-free rate 设计。

## 2. 数据特征

- 行数：5,075。
- 粒度：一行一个 ChinaBond 国债收益率曲线工作日。
- 候选键：`work_date`，profile 未发现重复。
- 数据范围：`work_date` 覆盖 2006-03-01 至 2026-06-16，无 NULL 也无 `1970-01-01` 占位日期。
- 当前 raw 仅包含一条曲线 `curve_name = 中债国债收益率曲线`，staging 不输出 `curve_name`。
- 各期限收益率单位为百分比点（如 `2.85` 表示 2.85%），非小数比例。
- 15Y、20Y 收益率字段在 5,075 行 raw 中全部为 NULL；3M / 6M / 1Y / 2Y / 3Y / 5Y / 7Y / 10Y / 30Y 无 NULL、无 0、无负值。

## 3. 字段设计

| Staging 字段 | 来源字段 | 类型建议 | 设计说明 |
|--------------|----------|----------|----------|
| `trade_date` | `work_date` | `Date` | ChinaBond 国债收益率曲线日期；staging 改名为 `trade_date` 以对齐项目内日历语义，不做交易日历对齐。 |
| `three_month_yield_pct` | `three_month_yield_pct` | `Nullable(Float64)` | 3 个月期国债收益率，百分比点。 |
| `six_month_yield_pct` | `six_month_yield_pct` | `Nullable(Float64)` | 6 个月期国债收益率，百分比点。 |
| `one_year_yield_pct` | `one_year_yield_pct` | `Nullable(Float64)` | 1 年期国债收益率，百分比点。 |
| `two_year_yield_pct` | `two_year_yield_pct` | `Nullable(Float64)` | 2 年期国债收益率，百分比点。 |
| `three_year_yield_pct` | `three_year_yield_pct` | `Nullable(Float64)` | 3 年期国债收益率，百分比点。 |
| `five_year_yield_pct` | `five_year_yield_pct` | `Nullable(Float64)` | 5 年期国债收益率，百分比点。 |
| `seven_year_yield_pct` | `seven_year_yield_pct` | `Nullable(Float64)` | 7 年期国债收益率，百分比点。 |
| `ten_year_yield_pct` | `ten_year_yield_pct` | `Nullable(Float64)` | 10 年期国债收益率，百分比点。 |
| `fifteen_year_yield_pct` | `fifteen_year_yield_pct` | `Nullable(Float64)` | 15 年期国债收益率，百分比点；当前 raw 全为 NULL，保留 nullable，不在 staging 填补。 |
| `twenty_year_yield_pct` | `twenty_year_yield_pct` | `Nullable(Float64)` | 20 年期国债收益率，百分比点；当前 raw 全为 NULL，保留 nullable，不在 staging 填补。 |
| `thirty_year_yield_pct` | `thirty_year_yield_pct` | `Nullable(Float64)` | 30 年期国债收益率，百分比点。 |

## 4. 标准化与 NULL 处理

- 列重命名：`work_date` → `trade_date`；其他字段保持 raw 命名，单位后缀 `_yield_pct` 保持不变。
- 单位口径：所有期限收益率保留 ChinaBond 原始百分比点口径，不在 staging 转换为小数比例（`/100`）。
- 缺失值：15Y、20Y 收益率全 NULL 是 raw source 当前事实，不在 staging 用 0、相邻期限插值或前向填充补齐。
- 不输出字段：`curve_name` 不进入 staging，因为当前 raw 只有单一曲线；如未来出现多条曲线，需要重新评估 grain、natural key 和字段集，再扩展 staging。
- 不在 staging 处理：交易日历对齐、前向填充到非工作日、期限利差派生、单位转换、跨源利率归一。

## 5. 测试建议

- `trade_date`: `not_null`，`unique`。
- `three_month_yield_pct` / `six_month_yield_pct` / `one_year_yield_pct` / `two_year_yield_pct` / `three_year_yield_pct` / `five_year_yield_pct` / `seven_year_yield_pct` / `ten_year_yield_pct` / `thirty_year_yield_pct`：`not_null`。
- `fifteen_year_yield_pct`、`twenty_year_yield_pct`：不加 `not_null`，承认 raw 当前全 NULL 的事实。
- 不需要 `curve_name` 相关测试；该字段不在 staging 输出。

## 6. 延后事项

- 风险无风险利率（risk-free rate）口径选择：使用哪个期限点（如 1Y）作为默认 risk-free，是否折算为日频 risk-free return，应在 mart / analytics 层决定。
- 交易日历对齐和缺失日期 forward-fill / 区间生效策略。
- 多曲线扩展（如国开债、信用债收益率曲线）和跨曲线归一。
- 期限结构指标（10Y-2Y 利差、收益率曲线斜率/曲率）派生。
