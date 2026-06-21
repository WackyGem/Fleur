# mart_portfolio_trade_metric_rank 设计

状态：Design

依据：

- Intermediate model：`ref('int_portfolio_trade_metric')`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_portfolio_trade_metric.md`
- 目标 SQL：`pipeline/elt/models/marts/mart_portfolio_trade_metric_rank.sql`
- 目标 YAML：`pipeline/elt/models/marts/mart_portfolio_trade_metric_rank.yml`

## 1. 模型定位

跨 run 的交易质量指标长表排名 mart。模型把 `int_portfolio_trade_metric` 宽表拆成长表，过滤 NULL 指标值，并按窗口和指标名计算 dense rank。

本模型不重算交易质量指标，只负责消费层长表化、排名方向和 ClickHouse table 物化。

## 2. 数据粒度与依赖

- 粒度：每个 run、attempt、window、metric_name 一行排名结果。
- 候选键：`portfolio_run_id`, `result_attempt_id`, `window_key`, `metric_name`。
- 物化：ClickHouse `MergeTree()` table。
- 排序键：`window_key`, `metric_name`, `metric_rank`, `portfolio_run_id`, `result_attempt_id`。
- NULL 策略：`metric_value is not null` 的指标才参与排名。

## 3. 字段设计

| Mart 字段 | 来源/派生 | 类型建议 | 设计说明 |
|-----------|-----------|----------|----------|
| `portfolio_run_id` | `int_portfolio_trade_metric` | `String` | 组合 run 标识符。 |
| `result_attempt_id` | `int_portfolio_trade_metric` | `String` | 仅追加的 result attempt 标识符。 |
| `window_key` | `int_portfolio_trade_metric` | `String` | 交易指标窗口键。 |
| `window_start` | `int_portfolio_trade_metric` | `Nullable(Date)` | 交易指标窗口起始日期；full_period 等无显式窗口边界的指标允许为 NULL。 |
| `window_end` | `int_portfolio_trade_metric` | `Nullable(Date)` | 交易指标窗口结束日期；full_period 等无显式窗口边界的指标允许为 NULL。 |
| `metric_name` | array join | `String` | 交易质量指标字段名。 |
| `metric_value` | array join | `Nullable(Float64)` | 该指标数值；已过滤 NULL。 |
| `rank_direction` | inline catalog | `String` | 排名方向，`asc` 或 `desc`。 |
| `metric_rank` | dense rank | `UInt64` | 在 `window_key`, `metric_name` 分组内的 dense rank。 |

## 4. 排名口径

第一版排名字段：

- `closed_trade_count`: `desc`
- `win_rate_closed_trades`: `desc`
- `average_win_return`: `desc`
- `average_loss_return`: `asc`
- `profit_loss_ratio`: `desc`
- `average_holding_days`: `asc`
- `largest_win_return`: `desc`
- `largest_loss_return`: `asc`

同分时按 `portfolio_run_id`, `result_attempt_id` 升序稳定排序。

## 5. 测试建议

- 组合键：`portfolio_run_id`, `result_attempt_id`, `window_key`, `metric_name` 唯一。
- `metric_value`, `rank_direction`, `metric_rank`: `not_null`。
- `rank_direction`: accepted values 为 `asc`, `desc`。
