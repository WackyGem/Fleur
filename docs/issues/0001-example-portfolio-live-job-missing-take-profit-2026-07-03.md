# Example Portfolio Live Job 缺少获利止盈条件（2026-07-03）

## 状态

Implemented

- Issue：[#1 — 用例策略缺少获利止盈条件导致收益过低](https://github.com/WackyGem/Fleur/issues/1)
- 实施报告：`docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`（v2→v8 完整迭代史）

## 摘要

`example__portfolio_live_job` 使用的 `racingline_0051_low_reversal` example portfolio fixture 在 v2 配置中仅启用 20 日时间止损（`TimeStopLoss { holding_days: 20, max_return_pct: 0.0 }`），不含任何止盈规则。持仓出现较大浮盈时只受时间止损约束，导致获利头寸无法在合理回报处了结，整体收益偏低。

issue #1 建议增加 8% 获利止盈条件。

## 影响范围

- 受影响 fixture：`racingline_0051_low_reversal` example portfolio（v2）
- 受影响入口：
  - Dagster job：`example__portfolio_live_job`（手动回归入口，不挂 schedule，不进生产 daily schedule）
  - Dagster asset：`rearview/example_0051_portfolio_live_run`
  - Rearview ensure API：`POST /rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure`
- 不受影响：
  - 生产实盘 `rearview/daily__portfolio_nav_liquidation`（独立代码路径）
  - 引擎止盈能力本身（`ExitRuleConfig::TakeProfit` 已存在且经过测试，本次只是启用）

## 已确认事实

1. v2 fixture 的风控配置位于 `engines/crates/rearview-core/src/examples.rs` 的 `racingline_0051_low_reversal_execution_config()`，`risk_exit_policy.exit_rules` 仅含一条 `TimeStopLoss`。
2. 引擎 `ExitRuleConfig` 枚举（`engines/crates/rearview-core/src/strategy_backtest.rs`）已支持四种退出规则：`FixedStopLoss`、`TakeProfit`、`TimeStopLoss`、`IndicatorStopLoss`。
3. `TakeProfit { profit_pct }` 的校验为 `profit_pct > 0`，运行时求值在 `engines/crates/rearview-core/src/portfolio/mod.rs` 的 `triggered_exit_reason`：当 `unrealized_return >= profit_pct` 触发卖出，其中 `unrealized_return = close_price / average_entry_price - 1.0`。
4. `exit_rules` 是 `BacktestExecutionConfig` 的一部分，追加规则会改变 `execution_config_hash`；ensure endpoint 对相同 `case_id`+`version` 但不同 fixture hash 的请求会拒绝，因此必须升级 fixture version。

## 处理结果

风控经多轮迭代，当前 example fixture 为 v8。

### v3：启用 8% 止盈（2026-08-02）

在保留 20 日时间止损的前提下追加 `TakeProfit { profit_pct: 0.08 }`。见迭代报告 `docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`。

### v4：追加 10% 止损与 MA30 趋势退出（2026-08-02）

追加 `FixedStopLoss { loss_pct: 0.10 }` 与 `IndicatorStopLoss { metric: "price_ma_30", operator: "close_below_metric" }`。见迭代报告 `docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`。

### v5：MA5 下穿重构（2026-08-02）

见迭代报告 `docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`。策略重写：移除时间止损与固定止盈，保留 10% 固定止损，MA 趋势退出由 MA30 单 bar 比较改为 MA5 下穿（crossover）。引擎新增 `cross_below_metric` operator。

### v6：MA10 下穿 + 20% 止盈调参（2026-08-02）

见迭代报告 `docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`。调参：保留 10% 固定止损，MA 趋势退出由 MA5 改为 MA10（仍用下穿语义），新增 20% 固定止盈。无引擎代码变更。

### v7：回退到 v4 策略（2026-08-02）

见迭代报告 `docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`。基于 prod 实盘收益对照（v2/v4/v5/v6 均有 live NAV 数据），v4（+17.83%、回撤 −30.19%）在总收益和最大回撤两项均最优。回退到 v4 的四条规则（时间止损 20 日 + 8% 止盈 + 10% 止损 + MA30 `close_below_metric`），但用新版本号 v7（避免与 archived v4 portfolio 混淆）。无引擎代码变更；部署后归档 active v6。

### v8：回退到 v3 策略（2026-08-02）

见迭代报告 `docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`。穷举 ClickHouse `live_nav_daily` 全部 26 个 attempt 后发现 v3（时间止损 20 日 + 8% 止盈）实盘收益 +42.04%、超额 +4.01%（唯一跑赢基准）、回撤 −21.46%（最小），五项指标全面领先。v4 追加的 10% 止损 + MA30 退出过早割肉，反而使收益从 +42% 降至 +17.83%。回退到 v3 的两条规则，用新版本号 v8。无引擎代码变更；部署后归档 active v7。

> 注：v3 的 portfolio 记录已从 Postgres 删除，但其 ClickHouse NAV/snapshot/performance 数据完整保留，策略定义已从 `live_run_snapshot.execution_snapshot` 核实。

### 当前行为（v8）

`exit_rules` 包含两条规则：

- `TimeStopLoss { holding_days: 20, max_return_pct: 0.0 }`：持仓满 20 个交易日强制卖出。
- `TakeProfit { profit_pct: 0.08 }`：单标的浮盈 ≥ 8% 止盈。

两条规则逐持仓独立求值，`trigger_timing = "close_confirm_next_open"` 收盘确认后次日开盘卖出（T+1），先触发者先执行。

fixture version 经 `v2 → v3 → v4 → v5 → v6 → v7 → v8`；`rule_hash` 始终不变（rule 未改），`execution_config_hash` v8 为 `7c28160c...`（与 v3 相同，因 exit_rules 内容相同且 hash 不含 version）。

部署影响：需重建 `rearview-server` 和 Dagster 镜像并重启容器，与 v2 部署同构；`release-manifest.yml` 无需更新（追踪组件 SemVer，不追踪 example fixture 内部版本号）。

## 关联文档

- 风控退出策略迭代报告（v2→v8 完整变更史 + 实盘收益对照 + 部署事故）：`docs/jobs/reports/2026-08-02-example-portfolio-live-job-risk-exit-iteration.md`
- v2 置换背景：`docs/jobs/reports/2026-07-03-example-portfolio-live-job-strategy-search-replacement.md`
- v2 部署：`docs/jobs/reports/2026-07-03-prod-docker-example-fixture-v2-deploy.md`
- Rearview 架构事实：`docs/architecture/rearview.md`
