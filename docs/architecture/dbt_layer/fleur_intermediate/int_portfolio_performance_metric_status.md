# int_portfolio_performance_metric_status 设计

状态：Design

依据：

- Portfolio source：`source('fleur_portfolio', 'portfolio_run_snapshot')`
- Calculation source：`source('fleur_calculation', 'calc_portfolio_performance_metric_status')`
- 组合数据平面 RFC：`docs/RFC/archive/0022-portfolio-data-plane-clickhouse-and-metrics.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_portfolio_performance_metric_status.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_portfolio_performance_metric_status.yml`

## 1. 模型定位

组合绩效指标状态 intermediate wrapper。模型透传 worker 输出的指标级状态，用来解释 `int_portfolio_performance_metric` 中各指标的 NULL 值和排名可用性。

本模型不从绩效宽表反推状态，不在 dbt 中裁决指标是否成功。

## 2. 数据粒度与依赖

- 粒度：每个 portfolio run、attempt、benchmark、window、metric_name 一行。
- 候选键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `metric_name`。
- 物化：dbt view。
- Join 策略：状态源 inner join `portfolio_run_snapshot`。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `portfolio_run_id` | calculation | `String` | 组合 run 标识符。 |
| `result_attempt_id` | calculation | `String` | 仅追加的 result attempt 标识符。 |
| `security_code` | calculation | `String` | benchmark 证券代码。 |
| `window_key` | calculation | `String` | 指标窗口键，第一版固定 `full_period`。 |
| `metric_name` | calculation | `String` | 绩效指标字段名。 |
| `metric_status` | calculation | `String` | 指标级计算状态。 |
| `reason_code` | calculation | `String` | 指标状态的原因码。 |
| `computed_at` | calculation | `DateTime` | 状态计算完成时间。 |

## 4. 测试建议

- 组合键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `metric_name` 唯一。
- `portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `metric_name`, `metric_status`, `reason_code`: `not_null`。
- `security_code`: `cn_security_code_format`。
- `metric_name`: accepted values 应覆盖绩效宽表可排名和可解释指标。
- `metric_status`: accepted values 为 `succeeded`, `insufficient_observations`, `invalid_input`。
