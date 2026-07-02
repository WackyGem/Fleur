# mart_stock_price_pattern_daily 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_price_pattern_daily')`
- Furnace price-pattern 计划：`docs/plans/archive/0031-furnace-price-action-structure-technical-indicators-implementation-plan.md`
- Furnace 全市场验收：`docs/jobs/reports/2026-06-09-furnace-price-pattern-full-market-validation.md`
- ClickHouse 排序键优化：`docs/issues/archive/optimize/0001-clickhouse-date-first-order-by-optimization.md`
- 目标 SQL：`pipeline/elt/models/marts/mart_stock_price_pattern_daily.sql`
- 目标 YAML：`pipeline/elt/models/marts/mart_stock_price_pattern_daily.yml`

## 1. 模型定位

A 股股票日频价格行为与 20 根 N 字结构 mart。模型把 `int_stock_price_pattern_daily` 暴露的价格方向、连阳连阴和 N 字结构字段整理成面向消费层的稳定接口。

本模型不承载公式实现。价格行为和 L1 -> H1 -> L2 -> 当前重新上攻结构均由 Furnace calculation 层完成，并经 dbt intermediate wrapper 暴露。本 mart 只负责字段分组、物化为 `fleur_marts` 表和消费层文档。

非目标：

- 不读取行情、估值、财务、raw、staging 或 `fleur_calculation` 物理表。
- 不在 mart 层重算连阳、连阴或 N 字结构。
- 不把本模型并入 `mart_stock_trend_indicator_daily`、`mart_stock_momentum_indicator_daily` 或 `mart_stock_volume_indicator_daily`，避免价格形态类字段和趋势/动量/均量指标混杂。

## 2. 数据粒度与依赖

- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 唯一上游：`int_stock_price_pattern_daily`。
- Join 策略：无 join。
- 当前上游物理事实：`fleur_calculation.calc_stock_price_pattern_daily` 为 `MergeTree`，约 1799 万行，`ORDER BY (trade_date, security_code)`，`PARTITION BY toYear(trade_date)`。

## 3. 字段分组

| 字段组 | 来源 | 字段 |
|---|---|---|
| 主键 | `int_stock_price_pattern_daily` | `security_code`, `trade_date` |
| 价格方向 | `int_stock_price_pattern_daily` | `close_direction`, `close_up_streak_days`, `close_down_streak_days` |
| 20 根 N 字结构 | `int_stock_price_pattern_daily` | `n_structure_20_is_valid`, `n_structure_20_stage`, `n_structure_20_higher_low_ratio`, `n_structure_20_pullback_depth`, `n_structure_20_rebound_ratio` |

## 4. 指标口径

价格方向：

- `close_direction = 1` 表示当日未复权收盘价高于 BaoStock `prev_close_price`。
- `close_direction = -1` 表示当日未复权收盘价低于 BaoStock `prev_close_price`。
- `close_direction = 0` 表示当日未复权收盘价等于 BaoStock `prev_close_price`。
- `close_price` 或 `prev_close_price` 为空时方向不可判定，streak 字段为空并打断连续性。

20 根 N 字结构：

- 结构检测使用 `int_stock_quotes_daily_adj.high_price_forward_adj` 和 `low_price_forward_adj`。
- 窗口最多保留最近 20 根有效 high/low 价格柱。
- 结构顺序必须是 `L1 -> H1 -> L2 -> 当前有效 K 线`，其中 L1、H1、L2 均在当前 K 线之前。
- 第一段上攻要求 `H1 / L1 - 1 >= 8%`，L2 抬高要求 `L2 / L1 >= 1.01`。
- 回撤深度 `n_structure_20_pullback_depth = (H1 - L2) / (H1 - L1)`，有效候选要求在 `[0.25, 0.75]`。
- `n_structure_20_stage` 为 `none`、`higher_low`、`rebound` 或 `breakout`；`rebound` 要求当前有效最高价 / L2 >= 1.03，`breakout` 表示当前有效最高价突破 H1。
- `n_structure_20_is_valid` 仅在 `rebound` 或 `breakout` 阶段为 true。

## 5. NULL 语义

NULL 语义完全沿用 `int_stock_price_pattern_daily`：

- 方向不可判定时，`close_direction`、`close_up_streak_days` 和 `close_down_streak_days` 允许为 NULL。
- 不存在完整 L1/H1/L2 候选结构时，`n_structure_20_higher_low_ratio`、`n_structure_20_pullback_depth` 和 `n_structure_20_rebound_ratio` 为 NULL，`n_structure_20_stage` 为 `none`。
- mart 层不填 0，不使用上一日值，不重算公式。

## 6. 物化与排序

本模型物化为 ClickHouse `MergeTree()` table：

- `partition_by = toYear(trade_date)`，保持年度回填和生命周期边界。
- `order_by = (trade_date, security_code)`，对齐上游 calculation 表和现有技术指标 mart，服务日截面选股、日期范围回测和全市场批量读取。

## 7. 验证

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_price_pattern_daily --full-refresh
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_price_pattern_daily --limit 20
```

补充质量检查：

- `(security_code, trade_date)` 唯一。
- `security_code`、`trade_date` 非空。
- `security_code` 符合 A 股标准代码格式。
- `close_direction` 非空值只允许 `-1`, `0`, `1`。
- `n_structure_20_stage` 只允许 `none`, `higher_low`, `rebound`, `breakout`。
- mart 行数应等于 `int_stock_price_pattern_daily` 行数。
- 输出字段集合只包含价格方向、连阳连阴、20 根窗口结构字段和主键。
