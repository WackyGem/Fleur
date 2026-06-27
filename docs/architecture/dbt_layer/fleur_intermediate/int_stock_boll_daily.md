# int_stock_boll_daily 设计

状态：Design

依据：

- Calculation source：`source('fleur_calculation', 'calc_stock_boll_daily')`
- 上游计划：`docs/plans/archive/0030-furnace-bollinger-bands-technical-indicators-implementation-plan.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_stock_boll_daily.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_stock_boll_daily.yml`

## 1. 模型定位

A 股股票日频 Bollinger Bands intermediate wrapper。模型包装 Furnace/Dagster 已物化的 calculation 层结果，为趋势 mart 暴露固定参数组的布林带字段。

本模型不在 dbt 中重算布林带公式。字段命名使用 `boll_mid_*`, `boll_upper_*`, `boll_lower_*`，避免旧式 `up/dn` 缩写在消费侧产生歧义。

## 2. 数据粒度与依赖

- 直接依赖：`fleur_calculation.calc_stock_boll_daily`。
- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 输入口径：第一版固定使用 `close_price_forward_adj` 前复权收盘价。
- 标准差口径：总体标准差，`ddof = 0`。
- NULL 语义：有效 close 不足窗口、当前 close 为空或状态无法推进时，布林带字段为 `NULL`。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | calculation | `String` | 股票标准连接代码。 |
| `trade_date` | calculation | `Date` | 指标交易日。 |
| `boll_mid_10_1p5` | calculation | `Nullable(Float64)` | BOLL(10, 1.5) 中轨。 |
| `boll_upper_10_1p5` | calculation | `Nullable(Float64)` | BOLL(10, 1.5) 上轨，`mid + 1.5 * population_std`。 |
| `boll_lower_10_1p5` | calculation | `Nullable(Float64)` | BOLL(10, 1.5) 下轨，`mid - 1.5 * population_std`。 |
| `boll_mid_20_2` | calculation | `Nullable(Float64)` | BOLL(20, 2) 中轨。 |
| `boll_upper_20_2` | calculation | `Nullable(Float64)` | BOLL(20, 2) 上轨，`mid + 2 * population_std`。 |
| `boll_lower_20_2` | calculation | `Nullable(Float64)` | BOLL(20, 2) 下轨，`mid - 2 * population_std`。 |
| `boll_mid_50_2p5` | calculation | `Nullable(Float64)` | BOLL(50, 2.5) 中轨。 |
| `boll_upper_50_2p5` | calculation | `Nullable(Float64)` | BOLL(50, 2.5) 上轨，`mid + 2.5 * population_std`。 |
| `boll_lower_50_2p5` | calculation | `Nullable(Float64)` | BOLL(50, 2.5) 下轨，`mid - 2.5 * population_std`。 |

## 4. 测试建议

- 组合键：`security_code`, `trade_date` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- BOLL 三元组非空时应满足 `upper >= mid >= lower`。
