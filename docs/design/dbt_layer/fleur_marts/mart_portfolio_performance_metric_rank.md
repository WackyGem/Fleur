# mart_portfolio_performance_metric_rank 设计

状态：Design

依据：

- Intermediate model：`ref('int_portfolio_performance_metric')`
- Intermediate model：`ref('int_portfolio_performance_metric_rank_catalog')`
- Intermediate model：`ref('int_portfolio_performance_metric_status')`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_portfolio_performance_metric.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_portfolio_performance_metric_status.md`
- 目标 SQL：`pipeline/elt/models/marts/mart_portfolio_performance_metric_rank.sql`
- 目标 YAML：`pipeline/elt/models/marts/mart_portfolio_performance_metric_rank.yml`

## 1. 模型定位

跨 run 的组合绩效指标长表排名 mart。模型把 `int_portfolio_performance_metric` 宽表拆成长表，结合排名目录和指标状态，只对成功且非 NULL 的指标计算 dense rank。

本模型不重算绩效指标，也不在 mart 层决定指标可用性；可用性来自 `int_portfolio_performance_metric_status`。

## 2. 数据粒度与依赖

- 粒度：每个 run、attempt、benchmark、window、metric_name 一行排名结果。
- 候选键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `metric_name`。
- 物化：ClickHouse `MergeTree()` table。
- 排序键：`config_hash`, `security_code`, `window_key`, `metric_name`, `metric_rank`, `portfolio_run_id`, `result_attempt_id`。
- NULL 策略：只保留 `metric_value is not null` 且 `metric_status = 'succeeded'` 的指标。

## 3. 字段设计

| Mart 字段 | 来源/派生 | 类型建议 | 设计说明 |
|-----------|-----------|----------|----------|
| `portfolio_run_id` | `int_portfolio_performance_metric` | `String` | 组合 run 标识符。 |
| `result_attempt_id` | `int_portfolio_performance_metric` | `String` | 仅追加的 result attempt 标识符。 |
| `source_run_id` | `int_portfolio_performance_metric` | `String` | 产出组合输入的筛选 source run 标识符。 |
| `security_code` | `int_portfolio_performance_metric` | `String` | benchmark 证券代码。 |
| `window_key` | `int_portfolio_performance_metric` | `String` | 绩效指标窗口键。 |
| `window_start` | `int_portfolio_performance_metric` | `Nullable(Date)` | 绩效指标窗口起始日期；full_period 等无显式窗口边界的指标允许为 NULL。 |
| `window_end` | `int_portfolio_performance_metric` | `Nullable(Date)` | 绩效指标窗口结束日期；full_period 等无显式窗口边界的指标允许为 NULL。 |
| `config_hash` | `int_portfolio_performance_metric` | `String` | 规范化指标配置字段的 SHA-256 哈希。 |
| `metric_name` | array join | `String` | 绩效指标字段名。 |
| `metric_value` | array join | `Nullable(Float64)` | 该指标数值；已过滤 NULL。 |
| `rank_direction` | `int_portfolio_performance_metric_rank_catalog` | `String` | 排名方向，`asc` 或 `desc`。 |
| `metric_rank` | dense rank | `UInt64` | 在 `config_hash`, `security_code`, `window_key`, `metric_name` 分组内的 dense rank。 |
| `reason_code` | `int_portfolio_performance_metric_status` | `String` | 该指标状态的原因码。 |

## 4. 排名口径

- 排名目录来自 `int_portfolio_performance_metric_rank_catalog`。
- `rank_direction = 'none'` 的指标不进入本 mart。
- 只对 `metric_status = 'succeeded'` 且 `metric_value` 非空的行排名。
- 同分时按 `portfolio_run_id`, `result_attempt_id` 升序稳定排序。

## 5. 测试建议

- 组合键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `metric_name` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `metric_value`, `rank_direction`, `metric_rank`, `reason_code`: `not_null`。
- `rank_direction`: accepted values 为 `asc`, `desc`。
