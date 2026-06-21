# int_stock_rsi_daily 设计

状态：Design

依据：

- Calculation source：`source('fleur_calculation', 'calc_stock_rsi_daily')`
- 上游计划：`docs/plans/archive/0030-furnace-rsi-technical-indicators-implementation-plan.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_stock_rsi_daily.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_stock_rsi_daily.yml`

## 1. 模型定位

A 股股票日频 RSI intermediate wrapper。模型包装 Furnace/Dagster 已物化的 RSI 结果，向 momentum mart 暴露固定窗口集合。

本模型不在 dbt 中重算 RSI 公式，不调整 warm-up 行，也不对 NULL 做填充。

## 2. 数据粒度与依赖

- 直接依赖：`fleur_calculation.calc_stock_rsi_daily`。
- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 输入口径：固定使用 `close_price_forward_adj`。
- 平滑口径：Wilder smoothing。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | calculation | `String` | 股票标准连接代码。 |
| `trade_date` | calculation | `Date` | 指标交易日。 |
| `rsi_6` | calculation | `Nullable(Float64)` | RSI(6)，非 NULL 值应在 `[0, 100]` 范围内。 |
| `rsi_12` | calculation | `Nullable(Float64)` | RSI(12)，非 NULL 值应在 `[0, 100]` 范围内。 |
| `rsi_14` | calculation | `Nullable(Float64)` | RSI(14)，非 NULL 值应在 `[0, 100]` 范围内。 |
| `rsi_24` | calculation | `Nullable(Float64)` | RSI(24)，非 NULL 值应在 `[0, 100]` 范围内。 |
| `rsi_25` | calculation | `Nullable(Float64)` | RSI(25)，非 NULL 值应在 `[0, 100]` 范围内。 |
| `rsi_50` | calculation | `Nullable(Float64)` | RSI(50)，第 51 个有效收盘价起首次非 NULL，非 NULL 值应在 `[0, 100]` 范围内。 |

## 4. 测试建议

- 组合键：`security_code`, `trade_date` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- RSI 非空值范围为 `[0, 100]`。
