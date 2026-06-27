# int_stock_ma_daily 设计

状态：Design

依据：

- Calculation source：`source('fleur_calculation', 'calc_stock_ma_daily')`
- 上游计划：`docs/plans/archive/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_stock_ma_daily.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_stock_ma_daily.yml`

## 1. 模型定位

A 股股票日频 Moving Average intermediate wrapper。模型包装 Furnace/Dagster 已物化的 calculation 层结果，提供价格均线、组合均线、双重 EMA 和均量字段的 dbt 契约。

本模型不在 dbt 中重算 MA/EMA 公式，不暴露 Furnace 内部状态列 `price_ema1_10_state` 和 `price_ema2_10_state`。下游 mart 负责按趋势和均量消费场景拆分字段。

## 2. 数据粒度与依赖

- 直接依赖：`fleur_calculation.calc_stock_ma_daily`。
- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 输入口径：价格指标使用 `close_price_forward_adj`，均量指标使用 `int_stock_quotes_daily_unadj.volume`。
- NULL 语义：窗口不足、当前输入缺失或状态无法推进时，指标字段允许为 `NULL`。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | calculation | `String` | 股票标准连接代码。 |
| `trade_date` | calculation | `Date` | 指标交易日。 |
| `price_ma_3` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 3 个有效收盘价简单移动平均。 |
| `price_ma_5` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 5 个有效收盘价简单移动平均。 |
| `price_ma_6` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 6 个有效收盘价简单移动平均。 |
| `price_ma_10` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 10 个有效收盘价简单移动平均。 |
| `price_ma_12` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 12 个有效收盘价简单移动平均。 |
| `price_ma_14` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 14 个有效收盘价简单移动平均。 |
| `price_ma_20` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 20 个有效收盘价简单移动平均。 |
| `price_ma_24` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 24 个有效收盘价简单移动平均。 |
| `price_ma_28` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 28 个有效收盘价简单移动平均。 |
| `price_ma_30` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 30 个有效收盘价简单移动平均。 |
| `price_ma_57` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 57 个有效收盘价简单移动平均。 |
| `price_ma_60` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 60 个有效收盘价简单移动平均。 |
| `price_ma_114` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 114 个有效收盘价简单移动平均。 |
| `price_ma_250` | calculation | `Nullable(Float64)` | 基于前复权收盘价的 250 个有效收盘价简单移动平均。 |
| `price_avg_ma_3_6_12_24` | calculation | `Nullable(Float64)` | `price_ma_3`, `price_ma_6`, `price_ma_12`, `price_ma_24` 的均值；任一组件为 NULL 时为 NULL。 |
| `price_avg_ma_14_28_57_114` | calculation | `Nullable(Float64)` | `price_ma_14`, `price_ma_28`, `price_ma_57`, `price_ma_114` 的均值；任一组件为 NULL 时为 NULL。 |
| `price_ema2_10` | calculation | `Nullable(Float64)` | 双重 EMA `EMA(EMA(close_price_forward_adj, 10), 10)`，使用 SMA 启动；第 19 个有效收盘价起首次非 NULL。 |
| `volume_ma_5` | calculation | `Nullable(Float64)` | 基于未复权成交量的 5 个有效成交量简单移动平均；0 成交量是有效输入。 |
| `volume_ma_10` | calculation | `Nullable(Float64)` | 基于未复权成交量的 10 个有效成交量简单移动平均；0 成交量是有效输入。 |
| `volume_ma_20` | calculation | `Nullable(Float64)` | 基于未复权成交量的 20 个有效成交量简单移动平均；0 成交量是有效输入。 |
| `volume_ma_60` | calculation | `Nullable(Float64)` | 基于未复权成交量的 60 个有效成交量简单移动平均；0 成交量是有效输入。 |

## 4. 测试建议

- 组合键：`security_code`, `trade_date` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- 下游趋势 mart 只消费价格 MA、组合 MA 和 `price_ema2_10`。
- 下游均量 mart 只消费 `volume_ma_*` 字段。
