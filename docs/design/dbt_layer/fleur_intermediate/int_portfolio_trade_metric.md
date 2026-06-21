# int_portfolio_trade_metric 设计

状态：Design

依据：

- Portfolio source：`source('fleur_portfolio', 'portfolio_run_snapshot')`
- Calculation source：`source('fleur_calculation', 'calc_portfolio_trade_metric')`
- 组合 RFC：`docs/RFC/0021-racingline-virtual-account-portfolio-rebalancing.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_portfolio_trade_metric.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_portfolio_trade_metric.yml`

## 1. 模型定位

交易质量指标 intermediate wrapper。模型透传 Rust worker 输出的交易质量计算行，供跨 run 排名和组合评估消费。

本模型不从已平仓交易明细重算胜率、盈亏比或持仓天数统计。dbt 只负责过滤完整 attempt、暴露字段契约和测试。

## 2. 数据粒度与依赖

- 粒度：每个 `portfolio_run_id` + `result_attempt_id` + `window_key` 一行。
- 候选键：`portfolio_run_id`, `result_attempt_id`, `window_key`。
- 物化：dbt view。
- Join 策略：`calc_portfolio_trade_metric` inner join `portfolio_run_snapshot`。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `portfolio_run_id` | calculation | `String` | 组合 run 标识符。 |
| `result_attempt_id` | calculation | `String` | 仅追加的 result attempt 标识符。 |
| `window_key` | calculation | `String` | 交易指标窗口键。 |
| `window_start` | calculation | `Nullable(Date)` | 指标窗口起始日期；无窗口边界时为 NULL。 |
| `window_end` | calculation | `Nullable(Date)` | 指标窗口结束日期；无窗口边界时为 NULL。 |
| `closed_trade_count` | calculation | `UInt32` | 窗口内已平仓交易笔数。 |
| `winning_trade_count` | calculation | `UInt32` | 窗口内盈利已平仓交易笔数。 |
| `losing_trade_count` | calculation | `UInt32` | 窗口内亏损已平仓交易笔数。 |
| `breakeven_trade_count` | calculation | `UInt32` | 窗口内持平已平仓交易笔数。 |
| `win_rate_closed_trades` | calculation | `Nullable(Float64)` | 盈利已平仓交易占比；分母为 0 时为 NULL。 |
| `average_win_return` | calculation | `Nullable(Float64)` | 盈利已平仓交易平均收益率；无盈利交易时为 NULL。 |
| `average_loss_return` | calculation | `Nullable(Float64)` | 亏损已平仓交易平均收益率；无亏损交易时为 NULL。 |
| `profit_loss_ratio` | calculation | `Nullable(Float64)` | 盈亏比，平均盈利收益率除以平均亏损收益率绝对值；无亏损交易时为 NULL。 |
| `average_holding_days` | calculation | `Nullable(Float64)` | 已平仓交易平均持仓天数；无已平仓交易时为 NULL。 |
| `largest_win_return` | calculation | `Nullable(Float64)` | 窗口内单笔最大盈利收益率；无盈利交易时为 NULL。 |
| `largest_loss_return` | calculation | `Nullable(Float64)` | 窗口内单笔最大亏损收益率；无亏损交易时为 NULL。 |
| `computed_at` | calculation | `DateTime` | 指标计算完成时间。 |

## 4. 测试建议

- 组合键：`portfolio_run_id`, `result_attempt_id`, `window_key` 唯一。
- `portfolio_run_id`, `result_attempt_id`, `window_key`: `not_null`。
- `closed_trade_count = winning_trade_count + losing_trade_count + breakeven_trade_count` 可作为定向一致性测试。
