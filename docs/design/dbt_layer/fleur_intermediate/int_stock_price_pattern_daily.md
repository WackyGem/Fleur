# int_stock_price_pattern_daily 设计

状态：Design

依据：

- Calculation source：`source('fleur_calculation', 'calc_stock_price_pattern_daily')`
- Furnace price-pattern 计划：`docs/plans/archive/0031-furnace-price-action-structure-technical-indicators-implementation-plan.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_stock_price_pattern_daily.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_stock_price_pattern_daily.yml`

## 1. 模型定位

A 股股票日频价格行为与前低-次低结构 intermediate wrapper。模型包装 Furnace/Dagster 已物化的价格形态结果，作为 mart 层稳定消费接口的直接上游。

本模型不在 dbt 中重算连阳、连阴或 20 根窗口结构，不读取行情、raw 或 staging 表。

## 2. 数据粒度与依赖

- 直接依赖：`fleur_calculation.calc_stock_price_pattern_daily`。
- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 方向口径：未复权 `close_price` 相对 BaoStock `prev_close_price`。
- 结构口径：使用前复权 `high_price_forward_adj` 和 `low_price_forward_adj`。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | calculation | `String` | 股票标准连接代码。 |
| `trade_date` | calculation | `Date` | 指标交易日。 |
| `close_direction` | calculation | `Nullable(Int8)` | 未复权收盘价相对 BaoStock `prev_close_price` 的方向；`1` 上涨，`-1` 下跌，`0` 持平，NULL 表示输入缺失。 |
| `close_up_streak_days` | calculation | `Nullable(UInt16)` | 截至当前行的连续有效上涨天数；方向为 NULL 时为 NULL，当前持平或下跌时为 0。 |
| `close_down_streak_days` | calculation | `Nullable(UInt16)` | 截至当前行的连续有效下跌天数；方向为 NULL 时为 NULL，当前持平或上涨时为 0。 |
| `n_structure_20_valid_bars` | calculation | `UInt16` | 20 根结构窗口使用的近期有效前复权最高/最低价 K 线数量。 |
| `n_structure_20_high_date` | calculation | `Nullable(Date)` | 近期 20 根有效最高/最低价 K 线中第一个最高 `high_price_forward_adj` 的交易日。 |
| `n_structure_20_high_price` | calculation | `Nullable(Float64)` | 近期 20 根有效最高/最低价 K 线中的最高 `high_price_forward_adj`。 |
| `n_structure_20_low_date` | calculation | `Nullable(Date)` | 结构窗口左侧（含最高点 K 线）第一个最低 `low_price_forward_adj` 的交易日。 |
| `n_structure_20_low_price` | calculation | `Nullable(Float64)` | 结构窗口左侧（含最高点 K 线）的最低 `low_price_forward_adj`。 |
| `n_structure_20_second_low_date` | calculation | `Nullable(Date)` | 最高点 K 线右侧之后第一个最低 `low_price_forward_adj` 的交易日。 |
| `n_structure_20_second_low_price` | calculation | `Nullable(Float64)` | 最高点 K 线右侧之后的最低 `low_price_forward_adj`。 |
| `n_structure_20_second_low_ratio` | calculation | `Nullable(Float64)` | `n_structure_20_second_low_price / n_structure_20_low_price`；次低点不可用时为 NULL。 |
| `n_structure_20_is_valid` | calculation | `Bool` | 当 `n_structure_20_second_low_ratio > 1.0` 时为 true；比率为 NULL 或不大于 1.0 时为 false。 |

## 4. 测试建议

- 组合键：`security_code`, `trade_date` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- `close_direction` 非空值只允许 `-1`, `0`, `1`。
- mart 层行数应与本模型保持一致。
