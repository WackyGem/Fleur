# RFC 0028: Racingline 策略回测 Step 5 异步执行方案

状态：Implemented
领域：racingline, rearview
关联系统：racingline, rearview, data-platform
代码根：app/racingline_new/, engines/crates/rearview-core/, engines/crates/rearview-server/, engines/crates/rearview-portfolio-worker/, pipeline/migrate/
需求入口：docs/intake/racingline.md

## 摘要

本文档定义 `/strategies` Step 5「策略回测」的第一版正式设计。Step 1 到 Step 4 已形成真实输入链路：

```text
Step 1 策略选股
  -> RuleVersionSpec.pool_filters

Step 2 权重配置
  -> RuleVersionSpec.scoring.rules

Step 3 股池预览
  -> 非 stale PreviewSnapshot.applied_rule_spec

Step 4 模拟建仓
  -> Rearview validate 后的 BacktestExecutionDraft

Step 5 策略回测
  -> 创建异步 backtest run
  -> 按回测区间重新执行选股、评分、TopN 信号和组合模拟
  -> 写入组合账本、净值和绩效指标
  -> Racingline 展示结果快照
```

Step 5 的核心不是在浏览器里把 Step 3 的 preview 数据拉长成曲线，而是把 Step 1 到 Step 4 的配置固化为一次可复现的回测任务，由 Rearview 异步执行并把结果写入现有 portfolio data plane。

由于回测可能覆盖多年、全 A 股和多个指标 mart，HTTP 请求不能同步等待。第一版必须复用现有 PostgreSQL outbox + NATS JetStream + `rearview-portfolio-worker` 的异步边界：HTTP 只创建控制面 run 并返回 `202 Accepted`，worker 消费轻量任务消息，执行计算并写回状态和结果。

## 背景

[Q&A 0004](../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md) 定义了从看板进入 `/strategies` 的完整业务闭环：选股、权重、股池预览、模拟建仓、策略回测和运行策略。

[RFC 0024](0024-racingline-strategy-selection-step1.md)、[RFC 0025](0025-racingline-strategy-weight-configuration-step2.md)、[RFC 0026](0026-racingline-strategy-pool-preview-step3.md) 和 [RFC 0027](0027-racingline-strategy-simulation-position-step4.md) 已把前四步边界收敛为：

1. Step 1 只记录候选池筛选规则。
2. Step 2 只记录候选池内评分规则。
3. Step 3 执行一次真实 preview，用于检查命中股票和分数解释，但不作为历史回测数据源。
4. Step 4 只生成并校验回测执行配置，不执行回测。

当前 `app/racingline_new` 的 Step 5 仍是本地静态净值、持仓和绩效样例；真实执行按钮禁用。Rearview 已有 `POST /rearview/strategy-backtests/validate` 校验 draft，但该接口不创建 run、不写结果、不发 NATS。

## 第一性原理

Step 5 必须先回答三个问题。

### P1: 回测的不可变输入是什么

一次回测必须由以下不可变输入唯一决定：

| 输入 | 来源 | 必须固化 |
|---|---|---|
| 选股和评分规则 | Step 3 `PreviewSnapshot.applied_rule_spec` | `rule_snapshot`, `rule_hash`, metric dependency snapshot |
| 建仓和风控规则 | Step 4 `BacktestExecutionDraft.execution_config` | `execution_config`, `execution_config_hash` |
| 回测范围 | Step 5 period/range | `start_date`, `end_date` |
| 基准 | Step 5 benchmark | `benchmark_security_code` |
| 数据口径 | Rearview canonical config | `price_basis = backward_adjusted`, signal timing, risk-free config |
| 执行代码版本 | Rearview worker | 后续可补 `engine_version`，第一版至少保留 config/hash |

如果这些输入相同，结果应可重算并解释；如果任一输入变化，必须创建新的 backtest run 或新的 `result_attempt_id`。

### P1.1: Step 4 配置必须逐项进入 execution_config

Step 4 页面里的配置不能只作为前端摘要，也不能在 Step 5 被重新解释。它们必须先由 Step 4 adapter 映射为 canonical `BacktestExecutionConfig`，再由 Step 5 create API 固化到 `strategy_backtest_run.execution_config`。

| Step 4 页面配置 | Canonical 字段 | Step 5 执行语义 |
|---|---|---|
| 初始资金 | `account.initial_cash`, `account.currency` | 回测首日现金和初始净值基准；第一版固定 `CNY` |
| 建仓价格口径 | `price_basis = backward_adjusted` | 组合净值、成交参考价和持仓估值统一使用后复权研究口径 |
| 买入 TopN | `signal_policy.buy_signal_top_n` | 每个信号日从完整候选池按 `score DESC, security_code ASC` 取前 N 个作为可调入候选；不读取 Step 3 preview row limit |
| 买入时点 | `signal_policy.signal_timing = close_confirm_next_open` | T 日收盘指标确认信号，T+1 交易日开盘执行；不允许默认同日收盘买入 |
| 最大持仓 | `rebalance_policy.max_positions` | 组合最多持仓槽位；worker 用它计算空闲仓位和是否可继续调入 |
| 单票仓位上限 | `rebalance_policy.single_position_limit_pct` | 单票目标权重为 `min((1 - cash_reserve_pct) / max_positions, single_position_limit_pct)` |
| 现金保留 | `rebalance_policy.cash_reserve_pct` | 第一版 UI 默认为 0；单票 cap 导致的未使用资金形成隐含现金保留 |
| A 股手数 | `rebalance_policy.lot_size`, `rebalance_policy.min_trade_lots` | 买入数量向下取整到 100 股整数倍，低于 1 手则跳过并记录 event |
| 调仓规则 | `rebalance_policy.target_weighting = equal_weight_capped`, `empty_signal_action = hold` | 第一版不是每日全量换仓；每日实际买入数量受可调入 TopN 候选和剩余持仓槽位共同约束，信号为空时继续持有 |
| 佣金 | `fee_profile.commission_rate`, `commission_rate_max`, `min_commission` | 每笔买卖成交计算佣金，并受市场模板上限和最低佣金约束 |
| 印花税 | `fee_profile.stamp_duty_rate_sell` | 卖出单边计费 |
| 过户费 | `fee_profile.transfer_fee_rate` | 买卖双边按成交金额计费 |
| 滑点 | `slippage_profile.mode = bps`, `buy_bps`, `sell_bps` | 买入执行价上调、卖出执行价下调，并写入 slippage cost |
| 固定止损 | `risk_exit_policy.exit_rules[].fixed_stop_loss` | 触发后按 `trigger_timing` 在下一交易日卖出 |
| 止盈 | `risk_exit_policy.exit_rules[].take_profit` | 触发后卖出并记录退出原因 |
| 时间止损 | `risk_exit_policy.exit_rules[].time_stop_loss` | 持仓达到指定天数且收益低于阈值时退出 |
| 指标止损 | `risk_exit_policy.exit_rules[].indicator_stop_loss` | 第一版只支持受控趋势主图指标，语义为收盘价跌破所选 metric 后下一交易日卖出 |

`buy_signal_top_n` 和 `max_positions` 是两个互不冲突的参数，不能互相覆盖：

```text
daily_candidate_limit = buy_signal_top_n
vacant_slots = max_positions - current_position_count_after_sells
actual_buy_count <= min(daily_candidate_limit, vacant_slots)
```

示例：如果 `max_positions = 5`，某日可调入 TopN 候选只有 3 只，且当前无持仓，则最多买入 3 只；第二天若可调入候选有 6 只，但已有 3 只持仓且没有卖出，剩余仓位只有 `5 - 3 = 2`，则只从 6 个候选中按分数优先买入前 2 只。已持仓证券不重复买入；若高分候选因价格缺失、现金不足或低于最小手数被跳过，可以继续检查 TopN 候选列表内的后续证券，但总买入数仍不得超过空闲仓位。

实现约束：Rearview canonicalization 必须保留用户提交的 `rebalance_policy.max_positions`，不得静默把它改成 `signal_policy.buy_signal_top_n`。如果现有实现会覆盖该字段，这是 Step 5 实施前必须修正的后端缺口。Step 5 UI 必须展示后端返回的 canonical `buy_signal_top_n` 和 `max_positions`，避免用户看到的买入规则和真实回测不一致。

### P2: 回测的权威结果是什么

Step 5 的权威结果不是前端图表状态，也不是 Step 3 preview response，而是 worker 写入的组合结果事实：

- 净值：`fleur_portfolio.portfolio_nav_daily`
- 持仓：`fleur_portfolio.portfolio_position_day`
- 目标：`fleur_portfolio.portfolio_target`
- 订单：`fleur_portfolio.portfolio_order`
- 成交：`fleur_portfolio.portfolio_trade`
- 事件：`fleur_portfolio.portfolio_event`
- 绩效指标：`fleur_calculation.calc_portfolio_performance_metric`
- 交易级指标：`fleur_calculation.calc_portfolio_closed_trade`、`calc_portfolio_trade_metric`

PostgreSQL 只保存控制面：run 状态、任务分发、输入快照、错误、summary 和当前有效 `result_attempt_id`。

### P3: 为什么必须异步

一次 Step 5 回测至少包含：

1. 按回测区间编译并执行 Step 1/2 规则。
2. 按每个交易日生成完整候选池、score、rank 和 TopN 买入信号。
3. 查询涉及证券的交易日、后复权行情和趋势指标止损字段。
4. 逐日递推组合持仓、现金、费用、滑点、止盈止损、指标止损和净值。
5. 读取 benchmark/risk-free 序列并计算核心绩效指标。
6. 写入 ClickHouse 结果事实和 PostgreSQL 终态。

这些工作可能超过普通 HTTP 超时，也需要失败重试和状态可见性。因此 Step 5 必须是异步 job，不允许在 `POST` 请求内同步完成。

## 当前资源盘点

### Racingline 前端资源

| 资源 | 路径 | 当前能力 | Step 5 复用方式 | 缺口 |
|---|---|---|---|---|
| Step 5 面板 | `app/racingline_new/src/routes/strategy-page.tsx` | 有周期、benchmark、净值、持仓、绩效样例 UI | 可保留信息结构，替换为真实 run 状态和结果查询 | 仍使用 `backtestNetValuePoints`、`backtestRebalanceRecords`、`backtestPerformanceGroups` 静态数据 |
| Backtest draft adapter | `app/racingline_new/src/features/strategy/execution.ts` | 已能生成 validate request、`BacktestExecutionDraft` 和 Step 5 request draft | `buildBacktestExecutionRequestDraft()` 可升级为 create backtest request | 目前不调用 create API，不处理 status/poll/result |
| API client | `app/racingline_new/src/api/rearview.ts` | 已有 `validateStrategyBacktest()` | 增加 create/get/list result hooks | 缺 `POST /rearview/strategy-backtests` 和结果查询类型 |
| Step gate | `strategy-page.tsx` + `preview.ts` | 已有非 stale preview gate、Step 4 fee validation | Step 5 创建 run 前继续校验 draft stale | Step 4 config 变化后未形成独立 run invalidation |
| 图表 | `BacktestNetValueChart` | 已用 Lightweight Charts 展示净值和基准 | 绑定真实 nav + benchmark normalized series | 目前没有 loading/error/empty/succeeded 分态 |

### Rearview 后端资源

| 资源 | 路径 | 当前能力 | Step 5 复用方式 | 缺口 |
|---|---|---|---|---|
| Draft validate | `engines/crates/rearview-core/src/strategy_backtest.rs` | 校验 transient `RuleVersionSpec + BacktestExecutionConfig`，返回 `rule_hash` 和 `execution_config_hash` | Step 5 create API 的第一道 canonical validation | 只 validate，不持久化、不排队、不执行 |
| Preview planner | `engines/crates/rearview-core/src/planner/sql.rs` | 可从 transient rule 编译 preview/timeline/pool-page SQL | Step 5 worker 可复用 planner 编译全区间信号 | 当前只有 preview 返回和正式 run 两条路径 |
| 正式 run runner | `engines/crates/rearview-core/src/service/runner.rs` | 对持久化 `rule_version_id` 执行区间选股并写 `pool_member/buy_signal` | 算法和 chunk 规划可复用 | 依赖正式 `rule_version_id`，不适合 Step 5 未发布草稿 |
| Portfolio engine | `engines/crates/rearview-core/src/portfolio/mod.rs` | 已实现组合递推、费用、滑点、TopN 空闲仓位递补、单票上限、固定止损、止盈、时间止损、指标止损 | Step 5 组合模拟核心 | 输入目前来自 `source_run_id` 的 `buy_signal` |
| Performance engine | `engines/crates/rearview-core/src/portfolio_performance.rs` | 计算 12 个核心绩效指标 | Step 5 绩效指标事实来源 | 默认 benchmark 当前固定为 `000300.SH`，需由 Step 5 request 覆盖 |
| Portfolio worker | `engines/crates/rearview-portfolio-worker/src/main.rs` | 消费 NATS、读取 PostgreSQL `portfolio_run`、写 ClickHouse 结果和 calculation 输出 | 作为 Step 5 异步执行进程继续复用 | 消息和控制面只识别 `portfolio_run_id/source_run_id` |
| NATS/outbox | `rearview-core/src/nats.rs`、`rearview-server/src/main.rs`、`postgres/mod.rs` | PostgreSQL outbox 发布 NATS JetStream，worker at-least-once 消费 | Step 5 继续用同一 stream/consumer 或扩展 message kind | 当前 message payload 只有 portfolio run 语义 |
| Result API | `api/mod.rs` | `/rearview/portfolio-runs/{id}/nav|targets|orders|trades|positions|events|performance|closed-trades|trade-metrics` | Step 5 可用 wrapper API 复用查询实现 | URL 和类型名是 portfolio-run，不适合直接暴露给策略创建流 |

### 数据和存储资源

| 资源 | 当前能力 | Step 5 价值 | 缺口 |
|---|---|---|---|
| `fleur_portfolio` ClickHouse tables | Rust worker 创建和写入 portfolio result facts | 可承载 Step 5 回测账本和净值 | 需要能区分 `strategy_backtest` source 或在 snapshot 中保留 backtest metadata |
| `fleur_calculation` ClickHouse tables | 保存 performance metric、metric status、closed trade 和 trade metric | 可承载 Step 5 业绩指标 | benchmark 需要从 Step 5 request 进入 config |
| `portfolio_metric_config` PostgreSQL table | 保存绩效计算参数和 config hash，当前外键指向 `portfolio_run` | Step 5 可复用字段语义 | 不能直接写独立 `strategy_backtest_run`；需要泛化为 simulation metric config 或新增 sibling table |
| `mart_benchmark_returns_daily`、`mart_risk_free_rate_daily` | worker-ready benchmark/risk-free 输入 | Step 5 绩效计算输入 | 前端 benchmark 选项必须只提供 Rearview 可读取 benchmark |

### ClickHouse 设计护栏

本 RFC 不新增一套 Step 5 专用 ClickHouse 结果表，原因是现有 portfolio data plane 已符合当前访问模式：

1. 查询通常按 run id 和 attempt id 拉净值/明细，符合 `schema-pk-prioritize-filters` 和 `schema-pk-filter-on-orderby`：现有 `ORDER BY (portfolio_run_id, result_attempt_id, trade_date...)` 命中高选择性前缀。
2. 时间序列表按 `toYYYYMM(trade_date)` 分区，符合 `schema-partition-low-cardinality`，避免按 run id 高基数字段分区。
3. worker 结果使用 append-only `result_attempt_id`，符合 `insert-mutation-avoid-update`，不依赖高频 `ALTER UPDATE`。
4. worker 应继续批量写入结果，遵守 `insert-batch-size` 的 10K-100K 行理想区间；小结果可合并表级 batch，避免每行一次 insert。

## 缺口与填充方案

| 缺口 ID | 缺口 | 影响 | 填充方案 |
|---|---|---|---|
| G1 | Step 5 没有 first-class backtest run。 | 前端无法创建、轮询或复现一次回测。 | 新增 `strategy_backtest_run` control-plane 表和 API。 |
| G2 | 现有 `portfolio_run` 强依赖 `source_run_id` 和正式 `rule_version_id`。 | Step 5 未发布策略时无法直接复用。 | Step 5 control plane 保存 transient `rule_snapshot`，worker 直接 materialize signals。 |
| G3 | NATS message 只有 portfolio run 语义。 | worker 无法区分正式组合运行和 Step 5 回测。 | 扩展为 typed task message：`kind = portfolio_run | strategy_backtest`，payload 只带 run id。 |
| G4 | Worker 当前从 PostgreSQL `buy_signal` 读取 source signals。 | Step 5 无正式 source run 时没有信号输入。 | 在 worker 内新增 transient signal materialization：planner 编译 Step 1/2 rule，按 range/chunk 查询 ClickHouse，生成 `BuySignalInput`。 |
| G5 | Step 5 benchmark 未进入 performance config。 | UI 选择的 benchmark 不影响 Alpha/Beta/IR 等指标。 | `PerformanceMetricConfig` 支持 `security_code` 参数，并由 backtest metric config 固化该 benchmark。 |
| G6 | Step 5 前端展示 mock 成功态。 | 用户可能误以为完成真实回测。 | 移除 Step 5 mock 成功路径；只展示 idle/queued/running/succeeded/failed/empty。 |
| G7 | Step 4 draft 与 Step 5 run 没有绑定。 | 回测结果无法证明使用了哪版建仓参数。 | Create request 必须带 `rule_hash` 和 `execution_config_hash`，后端重新计算并校验一致。 |
| G8 | 进度粒度不足。 | 长时间运行时用户无法判断卡在哪一阶段。 | Backtest run 状态包含 signal chunks、market data、portfolio simulation、performance、write result 阶段。 |
| G9 | Step 5 结果与“运行策略”边界未定义。 | 成功回测可能被误认为已经创建运行策略。 | Step 5 只产出 research backtest result；运行策略另起保存/发布流程。 |
| G10 | 直接创建隐藏 rule set/version 会污染策略版本空间。 | 回测草稿会被误当成用户正式策略。 | 第一版不创建正式 `rule_set` 或 `rule_version`；只保存 backtest run 的 immutable `rule_snapshot`。 |
| G11 | `buy_signal_top_n` 和 `max_positions` 可能在 canonicalization 中被合并。 | 用户配置“每日调入候选数”和“最大持仓数”后，实际回测仓位会被错误改写。 | 后端保留两者独立语义：TopN 限制每日可调入候选列表，`max_positions` 限制组合最大持仓和空闲槽位。 |

## 设计

### D1: Step 5 是 backtest run，不是 preview 延长

Step 5 创建的对象是 durable research backtest run：

```text
StrategyBacktestRun {
  strategy_backtest_run_id
  rule_snapshot
  rule_hash
  execution_config
  execution_config_hash
  start_date
  end_date
  benchmark_security_code
  preview_id
  preview_range
  status
  dispatch_status
  current_result_attempt_id
}
```

它不创建正式策略，也不发布规则版本。后续“运行策略”可以从成功的 backtest run 读取同一套快照，再进入正式保存/发布流程；该流程不属于本 RFC。

### D2: Create API contract

新增：

```http
POST /rearview/strategy-backtests
```

请求：

```json
{
  "rule": {},
  "preview_id": "01J...",
  "preview_range": {
    "start_date": "2025-06-23",
    "end_date": "2026-06-23"
  },
  "range": {
    "start_date": "2023-06-23",
    "end_date": "2026-06-23"
  },
  "benchmark": "000300.SH",
  "execution_config": {},
  "client_request_id": "optional-idempotency-key"
}
```

响应 `202 Accepted`：

```json
{
  "strategy_backtest_run_id": "01J...",
  "status": "queued",
  "dispatch_status": "pending",
  "rule_hash": "sha256...",
  "execution_config_hash": "sha256...",
  "current_result_attempt_id": null,
  "created_at": "2026-06-23T00:00:00Z"
}
```

后端必须重新执行 `StrategyBacktestValidateRequest::validate()`，不信任前端提交的 hash。若请求携带 `rule_hash` 或 `execution_config_hash`，后端只用于一致性校验，不作为权威值。

### D3: Query API contract

新增 wrapper API，避免 Step 5 前端直接使用 portfolio-run URL：

```http
GET /rearview/strategy-backtests/{strategy_backtest_run_id}
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/nav
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/targets
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/orders
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/trades
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/positions
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/events
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/performance
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/closed-trades
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/trade-metrics
```

这些 endpoint 内部可复用现有 ClickHouse 查询函数，第一版不复制数据面。

### D4: Control-plane schema

新增 PostgreSQL 表：

```text
strategy_backtest_run
strategy_backtest_task_outbox
strategy_backtest_metric_config
```

`strategy_backtest_run` 第一版字段：

| 字段 | 说明 |
|---|---|
| `strategy_backtest_run_id` | 主键；同时作为 ClickHouse result fact 的 run id 使用 |
| `rule_snapshot` | canonical `RuleVersionSpec` JSON |
| `rule_hash` | 后端计算的 hash |
| `execution_config` | canonical `BacktestExecutionConfig` JSON |
| `execution_config_hash` | 后端计算的 hash |
| `preview_id` | Step 3 applied preview id，可为空但前端应提交 |
| `preview_range` | Step 3 preview context |
| `start_date`, `end_date` | Step 5 回测区间 |
| `benchmark_security_code` | 绩效指标 benchmark |
| `price_basis` | 固定 `backward_adjusted` |
| `status` | 任务状态 |
| `dispatch_status` | outbox 发布状态 |
| `nats_stream_sequence` | NATS publish ack |
| `summary` | 轻量 summary |
| `error_type`, `error_message` | 错误 |
| `current_result_attempt_id` | 当前有效 ClickHouse 结果版本 |
| `created_at`, `updated_at`, `started_at`, `completed_at` | 审计 |

状态枚举：

```text
created
queued
compiling_signals
running_clickhouse
loading_market_data
calculating_nav
computing_performance
writing_results
succeeded
failed_validation
failed_compile
failed_market_data
failed_simulation
failed_write
cancelled
```

`strategy_backtest_task_outbox` 与现有 `portfolio_task_outbox` 同构，第一版也可以复用一张 generic outbox，但不能把完整 rule/config 放进 NATS message。

`strategy_backtest_metric_config` 复用 `portfolio_metric_config` 的字段语义，但外键指向 `strategy_backtest_run`。不直接写现有 `portfolio_metric_config`，因为该表当前有 `portfolio_run` 外键。后续若 portfolio run 和 strategy backtest 都稳定为同一类 simulation run，可以另起 migration 将两张 metric config 表泛化为 `simulation_metric_config`。

### D5: NATS task message

当前消息：

```json
{
  "portfolio_run_id": "...",
  "source_run_id": "..."
}
```

扩展为 typed message：

```json
{
  "kind": "strategy_backtest",
  "run_id": "01J..."
}
```

兼容策略：

1. 没有 `kind` 但有 `portfolio_run_id` 的旧消息按 `portfolio_run` 处理。
2. `strategy_backtest` 消息只带 run id，worker 从 PostgreSQL 读取完整快照。
3. 同一 NATS stream 可以继续使用 `rearview.portfolio_run.requested` subject；若需要隔离吞吐，再新增 `rearview.strategy_backtest.requested` subject，但仍用同一 JetStream 基础设施。

### D6: Worker 执行流程

Step 5 worker 流程：

```text
consume strategy_backtest task
  -> claim strategy_backtest_run
  -> validate immutable snapshot
  -> materialize backtest signals from rule_snapshot
  -> query trade calendar and price bars
  -> simulate_portfolio()
  -> compute_performance_metric()
  -> write fleur_portfolio result facts
  -> write fleur_calculation outputs
  -> insert strategy_backtest_metric_config
  -> update strategy_backtest_run.current_result_attempt_id
  -> status = succeeded
  -> ack message
```

worker 构造 `PortfolioSimulationInput` 时必须从固化的 `execution_config` 读取 `initial_cash`、`buy_signal_top_n`、`max_positions`、`single_position_limit_pct`、`cash_reserve_pct`、`lot_size`、`min_trade_lots`、`fee_profile`、`slippage_profile` 和 `risk_exit_policy.exit_rules`。这些字段不得从 Step 5 UI 当前草稿、Step 3 preview rows 或市场默认模板重新推导。

信号 materialization 复用现有 planner：

```text
RuleVersionSpec + backtest_range + top_n
  -> QueryPlanner::compile(...)
  -> ClickHouse screening rows
  -> BuySignalInput[]
```

这里的 `top_n` 来自 Step 4 `execution_config.signal_policy.buy_signal_top_n`，不是 Step 3 preview row limit，也不是 `rebalance_policy.max_positions`。`max_positions` 只在 portfolio simulation 阶段限制最大持仓槽位和空闲仓位调入数量。

### D7: 不创建隐藏 rule set/version

第一版不为了回测创建隐藏 `rule_set`、`rule_version` 或正式 `run`。理由：

1. Step 5 是研究型回测，不是策略发布。
2. 当前 `run` schema 强依赖 `rule_version_id`，强行创建隐藏版本会污染策略版本列表和 account template 关系。
3. Backtest run 的不可变 `rule_snapshot + rule_hash` 已足够复现。
4. 后续“运行策略”阶段才应创建正式 rule set/version 和 account template。

### D8: 结果数据面复用

ClickHouse 结果表继续使用 `portfolio_run_id` 字段。第一版约定：

```text
portfolio_run_id = strategy_backtest_run_id
source_kind = strategy_backtest
```

为了让分析快照可解释，`portfolio_run_snapshot` 需要补充或在 `execution_snapshot` 中记录：

- `source_kind = strategy_backtest`
- `strategy_backtest_run_id`
- `preview_id`
- `rule_hash`
- `execution_config_hash`
- `benchmark_security_code`

如果修改 ClickHouse schema 风险过大，第一版可先把这些字段放入 `execution_snapshot` JSON string，并把现有非空字符串字段写成可解释占位：

| 字段 | Step 5 第一版值 |
|---|---|
| `source_run_id` | `strategy_backtest_run_id` |
| `rule_version_id` | `transient` |

第二阶段再补 typed columns，例如 `source_kind`、`source_id` 和 nullable `rule_version_id`。

### D9: 绩效指标

第一版 Step 5 展示以下权威指标：

| 指标 | 来源 |
|---|---|
| 区间收益 | `calc_portfolio_performance_metric.holding_period_return` |
| 年化收益 | `annualized_return` |
| 年化波动 | `annualized_volatility` |
| 最大回撤 | `max_drawdown` |
| Calmar | `calmar_ratio` |
| 下行波动 | `downside_deviation` |
| Sortino | `sortino_ratio` |
| Sharpe | `sharpe_ratio` |
| 信息比率 | `information_ratio` |
| Beta | `beta` |
| Alpha | `alpha` |
| Treynor | `treynor_ratio` |

此外展示净值 summary：

- 当前净值
- 日收益
- 现金
- 持仓市值
- 仓位
- 持仓数
- 费用
- warning count

交易级指标可作为次级区域或后续增强，但 API 应复用已存在的 `closed-trades` 和 `trade-metrics`。

### D10: Benchmark 语义

Step 5 的 benchmark 必须进入后端权威计算：

```text
benchmark_security_code -> PerformanceMetricConfig.security_code
```

`PerformanceMetricConfig::default_full_period()` 需要改为接受 benchmark 参数，不能继续硬编码 `000300.SH`。Backtest 的 metric config 写入 `strategy_backtest_metric_config`，并让 `calc_portfolio_performance_metric.config_hash` 指向这套配置。若 benchmark 缺失或 mart 无数据：

1. NAV 和账本仍可成功写入。
2. `performance.metric_status` 应为 `missing_benchmark` 或类似状态。
3. 前端展示绩效缺失原因，而不是显示 0 或 mock 值。

### D11: Idempotency 和重算

Create API 支持可选 `client_request_id`。同一 `client_request_id` 在同一用户/会话范围内重复提交，应返回同一个 `strategy_backtest_run_id`，避免用户双击创建重复任务。第一版没有用户系统时，可按 request hash + client id 做弱幂等。

重算策略：

1. 同一 backtest run 可生成新的 `result_attempt_id`。
2. ClickHouse 结果 append-only，不覆盖旧 attempt。
3. PostgreSQL `current_result_attempt_id` 指向当前有效 attempt。
4. `GET` 默认读当前 attempt，可通过 query 参数指定历史 attempt。

### D12: Racingline Step 5 UI

Step 5 页面状态：

| 状态 | 行为 |
|---|---|
| `idle` | 展示 Step 4 draft 摘要、range 和 benchmark 选择，主按钮为“开始回测” |
| `queued/running` | 展示状态、阶段、已完成 chunk/day、禁用重复提交 |
| `succeeded` | 展示真实净值、绩效、持仓、成交和事件 |
| `failed_*` | 展示错误类型、错误消息、可重试按钮 |
| `stale` | Step 1/2/4 改动后，已有结果作为历史快照保留，但当前草稿需要重新创建回测 |

必须移除或隔离 Step 5 用户成功路径中的静态 mock：

- `backtestNetValuePoints`
- `backtestRebalanceRecords`
- `backtestPerformanceGroups`
- 本地 `buildBacktestTrade()` 生成的持仓样例

测试 fixture 可以保留，但用户可操作路径不能把 mock 当作真实成功态。

### D13: Progress contract

`GET /rearview/strategy-backtests/{id}` 返回轻量进度：

```json
{
  "status": "running_clickhouse",
  "progress": {
    "signal_chunks_total": 6,
    "signal_chunks_completed": 2,
    "trade_dates_total": 720,
    "trade_dates_materialized": 241,
    "current_stage": "materialize_signals"
  }
}
```

第一版可以先用 stage + chunks 粗粒度；不要求实时百分比精确到每条证券。

### D14: 错误语义

错误类型：

| 类型 | 场景 |
|---|---|
| `failed_validation` | rule/config/range/benchmark 不合法 |
| `failed_compile` | planner 编译失败 |
| `failed_market_data` | ClickHouse 查询、行情、benchmark 或 risk-free 输入失败 |
| `failed_simulation` | portfolio engine 递推失败 |
| `failed_write` | PostgreSQL 或 ClickHouse 结果写入失败 |
| `cancelled` | 用户或系统取消 |

前端不解析底层堆栈，只展示错误类型、用户可读消息和重试入口。

## API 草案

### Create

```http
POST /rearview/strategy-backtests
```

返回 `StrategyBacktestRunRecord`。

### Get

```http
GET /rearview/strategy-backtests/{id}
```

返回 run status、hash、range、benchmark、summary、error 和 progress。

### Results

结果 API 第一版与 portfolio result API 结构对齐：

```http
GET /rearview/strategy-backtests/{id}/nav?result_attempt_id=
GET /rearview/strategy-backtests/{id}/performance?security_code=000300.SH&window_key=full_period
GET /rearview/strategy-backtests/{id}/positions?trade_date=&limit=&offset=
GET /rearview/strategy-backtests/{id}/trades?trade_date=&security_code=&limit=&offset=
GET /rearview/strategy-backtests/{id}/orders?execution_date=&security_code=&limit=&offset=
GET /rearview/strategy-backtests/{id}/targets?signal_date=&limit=&offset=
GET /rearview/strategy-backtests/{id}/events?trade_date=&event_type=&limit=&offset=
```

## 实施阶段

### Phase 0: 契约冻结和 fixture

1. 固定 `StrategyBacktestCreateRequest/RunRecord/Progress/Result` 类型。
2. 准备一个短区间 deterministic fixture，覆盖至少 2 只证券、3 至 5 个交易日、买入、卖出、费用、滑点、指标止损和 benchmark 缺失分支。
3. 固定 Step 5 不创建 rule set/version 的边界。

验证：

```bash
make docs-check
git diff --check
```

### Phase 1: PostgreSQL control plane 和 NATS message

1. 新增 migration：`strategy_backtest_run`、`strategy_backtest_task_outbox`、`strategy_backtest_metric_config`。
2. 新增 repository create/get/list/claim/finalize/fail。
3. 扩展 NATS task message，兼容旧 portfolio message。
4. 后端 create API 返回 `202 Accepted`。

验证：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
cd ../engines
cargo test -p rearview-core strategy_backtest
```

### Phase 2: Worker signal materialization

1. Worker 支持 `kind = strategy_backtest`。
2. 从 `rule_snapshot + range + top_n` 编译 chunk。
3. 查询 ClickHouse screening rows 并生成 `BuySignalInput[]`。
4. 复用 `simulate_portfolio()` 计算账本。
5. benchmark 参数进入 `PerformanceMetricConfig`。

验证：

```bash
cd engines
cargo test -p rearview-core portfolio
cargo test -p rearview-core portfolio_performance
cargo test -p rearview-portfolio-worker
```

### Phase 3: Result API wrapper

1. 新增 `/rearview/strategy-backtests/{id}/...` 查询 API。
2. 内部复用 ClickHouse portfolio/calculation query 函数。
3. `resolve_result_attempt` 支持 strategy backtest run。
4. 增加 API 测试覆盖 missing attempt、running 状态和 succeeded 状态。

验证：

```bash
cd engines
cargo test -p rearview-core strategy_backtest
cargo clippy -p rearview-core --all-targets --all-features -- -D warnings
```

### Phase 4: Racingline Step 5 接入

1. `BacktestPanel` 使用真实 create/get/result hooks。
2. 移除 mock 成功路径。
3. Step 5 idle 状态展示 draft 摘要、range 和 benchmark。
4. Running 状态轮询 progress。
5. Succeeded 状态展示 nav、performance、positions/trades/events。
6. Failed 状态展示错误和重试。
7. Step 1/2/4 改动后标记 active backtest draft stale。

验证：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

### Phase 5: Live smoke 和报告

1. 启动 `make racingline-dev`，确保 Rearview server、portfolio worker、NATS 和 ClickHouse 可用。
2. 在 `/strategies` 完成 Step 1 到 Step 5 的短区间回测。
3. 验证 Network 没有 mock 成功路径。
4. 验证 ClickHouse 写入 `fleur_portfolio` 和 `fleur_calculation`。
5. 写入 `docs/jobs/reports/` 验收报告。

验证：

```bash
make racingline-dev
cd engines
cargo run -p rearview-portfolio-worker -- run --once
```

## 禁止模式

1. 不在浏览器内计算权威净值、持仓、成交、费用、滑点或绩效指标。
2. 不把 Step 3 preview response 作为 Step 5 历史回测结果。
3. 不在 `POST /rearview/strategy-backtests` 中同步等待完整回测完成。
4. 不把完整 rule/config/行情/结果塞进 NATS message。
5. 不为每次回测创建用户可见的隐藏 rule set 或 rule version。
6. 不新增一套 Step 5 专用 ClickHouse 结果表，除非后续证明现有 portfolio data plane 无法承载。
7. 不用 mock 数据作为 Step 5 接口失败后的 success fallback。
8. 不把 benchmark 缺失时的绩效指标显示为 0。

## 验收标准

第一版完成后应满足：

1. 用户在 Step 5 点击“开始回测”后得到一个真实 `strategy_backtest_run_id`。
2. HTTP create 返回 `202 Accepted`，后续通过轮询获得 queued/running/succeeded/failed 状态。
3. worker 通过 NATS 异步执行，不在 HTTP 请求内计算完整回测。
4. 回测按 Step 3 applied rule 和 Step 4 canonical execution config 重新执行全区间信号，不复用 Step 3 preview rows。
5. ClickHouse 写入净值、目标、订单、成交、持仓、事件和绩效指标，PostgreSQL 保存当前有效 `result_attempt_id`。
6. Racingline Step 5 不再展示静态 mock 成功结果。
7. 用户选择的 benchmark 进入绩效计算配置。
8. Step 1/2/4 改动后，已有 Step 5 结果只作为历史快照，当前草稿必须重新回测。
9. 失败状态可解释，并保留可重试入口。

## 后续问题

1. “运行策略”是否从成功 backtest run 一键创建正式 rule set/version、account template 和生产组合运行，需要另起 RFC。
2. 是否需要取消任务和 worker 中断，需要在长区间回测落地后评估。
3. 是否需要跨 backtest run 排名和批量参数扫描，应基于 `fleur_calculation` 结果另行设计。
4. `portfolio_run_id` 字段是否长期泛化为 `simulation_run_id`，需要在多个 run kind 稳定后再做 ADR。
