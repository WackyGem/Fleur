# int_portfolio_performance_metric 设计

状态：Design

依据：

- Portfolio source：`source('fleur_portfolio', 'portfolio_run_snapshot')`
- Calculation source：`source('fleur_calculation', 'calc_portfolio_performance_metric')`
- 组合数据平面 RFC：`docs/RFC/0022-portfolio-data-plane-clickhouse-and-metrics.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_portfolio_performance_metric.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_portfolio_performance_metric.yml`

## 1. 模型定位

组合绩效指标 intermediate wrapper。模型透传 Rust worker 输出的绩效计算行，并从 `portfolio_run_snapshot` 补充 `source_run_id`、组合起止日期，供排名 mart 和应用查询消费。

本模型不在 dbt 中重算持有期收益、年化收益、波动率、最大回撤、Sharpe、Sortino、beta、alpha 或 Treynor 等公式。

## 2. 数据粒度与依赖

- 粒度：每个 portfolio run、attempt、benchmark、window 一行。
- 候选键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`。
- 物化：dbt view。
- Join 策略：`calc_portfolio_performance_metric` inner join `portfolio_run_snapshot`。
- `security_code` 表示 benchmark 证券代码。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `portfolio_run_id` | calculation | `String` | 组合 run 标识符。 |
| `result_attempt_id` | calculation | `String` | 仅追加的 result attempt 标识符。 |
| `source_run_id` | `portfolio_run_snapshot` | `String` | 产出组合输入的选股源 run 标识。 |
| `security_code` | calculation | `String` | benchmark 证券代码。 |
| `window_key` | calculation | `String` | 绩效指标窗口键，第一版固定 `full_period`。 |
| `window_start` | calculation | `Nullable(Date)` | 指标窗口起始日期；无窗口边界时为 NULL。 |
| `window_end` | calculation | `Nullable(Date)` | 指标窗口结束日期；无窗口边界时为 NULL。 |
| `run_start_date` | `portfolio_run_snapshot.start_date` | `Date` | 组合回测起始日期。 |
| `run_end_date` | `portfolio_run_snapshot.end_date` | `Date` | 组合回测结束日期。 |
| `config_hash` | calculation | `String` | 规范化指标配置字段的 SHA-256 哈希。 |
| `metric_status` | calculation | `String` | 整行输入状态。 |
| `observation_count` | calculation | `UInt32` | worker 对齐使用的组合、benchmark 和无风险利率日频观测数。 |
| `holding_period_return` | calculation | `Nullable(Float64)` | 区间持有期收益率。 |
| `annualized_return` | calculation | `Nullable(Float64)` | 年化收益率。 |
| `annualized_volatility` | calculation | `Nullable(Float64)` | 年化波动率。 |
| `max_drawdown` | calculation | `Nullable(Float64)` | 区间最大回撤。 |
| `calmar_ratio` | calculation | `Nullable(Float64)` | 卡玛比率。 |
| `downside_deviation` | calculation | `Nullable(Float64)` | 下行标准差。 |
| `sortino_ratio` | calculation | `Nullable(Float64)` | 索提诺比率。 |
| `sharpe_ratio` | calculation | `Nullable(Float64)` | 夏普比率。 |
| `information_ratio` | calculation | `Nullable(Float64)` | 信息比率。 |
| `beta` | calculation | `Nullable(Float64)` | 相对基准的贝塔系数。 |
| `alpha` | calculation | `Nullable(Float64)` | 相对基准的詹森阿尔法。 |
| `treynor_ratio` | calculation | `Nullable(Float64)` | 特雷诺比率。 |
| `computed_at` | calculation | `DateTime` | 指标计算完成时间。 |

## 4. 测试建议

- 组合键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key` 唯一。
- `portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `config_hash`, `metric_status`, `observation_count`: `not_null`。
- `security_code`: `cn_security_code_format`。
- `window_key`: accepted values 第一版为 `full_period`。
- `metric_status`: accepted values 为 `succeeded`, `insufficient_observations`, `invalid_input`。
