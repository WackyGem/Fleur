# int_stock_price_pattern_daily 设计

状态：Design

依据：

- Calculation source：`source('fleur_calculation', 'calc_stock_price_pattern_daily')`
- Furnace price-pattern 计划：`docs/plans/archive/0031-furnace-price-action-structure-technical-indicators-implementation-plan.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_stock_price_pattern_daily.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_stock_price_pattern_daily.yml`

## 1. 模型定位

A 股股票日频价格行为与 20 根 N 字结构 intermediate wrapper。模型包装 Furnace/Dagster 已物化的价格形态结果，作为 mart 层稳定消费接口的直接上游。

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
| `n_structure_20_is_valid` | calculation | `Bool` | 当 20 根有效 high/low 窗口内存在 L1 -> H1 -> L2，且当前有效 K 线已从 L2 重新上攻时为 true；仅形成 higher_low 阶段时为 false。 |
| `n_structure_20_stage` | calculation | `String` | N 字结构阶段：`none`、`higher_low`、`rebound` 或 `breakout`。 |
| `n_structure_20_higher_low_ratio` | calculation | `Nullable(Float64)` | L2 低点 / L1 低点；不存在完整 L1/H1/L2 候选结构时为 NULL。 |
| `n_structure_20_pullback_depth` | calculation | `Nullable(Float64)` | 回撤深度 `(H1 - L2) / (H1 - L1)`；不存在完整 L1/H1/L2 候选结构时为 NULL。 |
| `n_structure_20_rebound_ratio` | calculation | `Nullable(Float64)` | 当前有效 K 线最高价 / L2 低点；不存在完整 L1/H1/L2 候选结构时为 NULL。 |

## 4. 测试建议

- 组合键：`security_code`, `trade_date` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- `close_direction` 非空值只允许 `-1`, `0`, `1`。
- `n_structure_20_stage` 只允许 `none`, `higher_low`, `rebound`, `breakout`。
- mart 层行数应与本模型保持一致。
