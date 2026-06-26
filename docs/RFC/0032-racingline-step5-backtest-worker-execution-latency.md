# RFC 0032: Racingline Step 5 回测 worker 执行流程抽象与计算耗时优化

状态：Proposed（讨论稿，2026-06-25）
领域：racingline, rearview, clickhouse marts
关联系统：racingline, rearview
代码根：app/racingline/, engines/crates/rearview-core/, engines/crates/rearview-portfolio-worker/, engines/crates/rearview-server/
关联文档：docs/RFC/0031-racingline-step4-step5-backtest-latency-slimming.md, docs/jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md

## 摘要

RFC 0031 和 Plan 0056 已经把 Step 4 到 Step 5 的页面跳转从 worker 终态中解耦：点击「策略回测」后，`POST /rearview/strategy-backtests` 返回 `202 Accepted` 即进入 Step 5，浏览器实测 click-to-Step5 shell 为 439ms。当前用户仍感到 Step 5 慢，问题已经从“页面进不去”转移为“Step 5 run 结果完成慢”。

最新验收样本显示，1y/2y run 的 create 和 outbox 已不是主因：

| Period | create HTTP | outbox publish | worker pickup | worker elapsed | backend total |
|---|---:|---:|---:|---:|---:|
| 1y | 0.113s | 0.039s | 0.013s | 6.477s | 6.529s |
| 2y | 0.105s | 0.031s | 6.238s | 13.797s | 20.066s |

2y 的 6.238s pickup 等待来自单 worker 串行处理前一个 1y run；这是吞吐和排队问题。单次 run 的实际执行瓶颈集中在 worker 内部：

| Period | total | signal materialization | price bars | simulation | writes |
|---|---:|---:|---:|---:|---:|
| 1y | 6,471ms | 2,536ms | 2,096ms | 1,031ms | 767ms |
| 2y | 13,790ms | 4,385ms | 4,302ms | 4,212ms | 838ms |

第一性原理下，Step 5 回测 worker 只需要完成三件事：

1. 从冻结的规则和执行配置生成可执行买入信号。
2. 用这些信号和必要行情递推组合账本、净值和退出事件。
3. 写入权威结果事实，并把 run 终态指向当前 `result_attempt_id`。

任何不服务于这三件事的全量字段、重复查询、全范围数据读取、重复状态机制和低效内存索引，都应进入后续优化评估。

## 目标

1. 梳理 Step 5 当前真实回测执行流，区分必须流程和可删减流程。
2. 先给出不受历史实现约束的回测参数与执行流程基础模板，再用当前实现做映射和差距分析。
3. 把 `rearview-portfolio-worker` 的 strategy backtest 路径抽象成可复用执行模板，方便后续整理代码。
4. 基于已测 timing 和源码事实，列出下一步可讨论的性能优化方向。
5. 明确哪些优化已有指标支撑，哪些还只是待基准验证的假设。

## 非目标

1. 本 RFC 不调整生产代码。
2. 本 RFC 不改变回测业务口径：规则、TopN、最大持仓、费用、滑点、止损、benchmark、risk-free 和 result attempt 语义都不变。
3. 本 RFC 不删除 PostgreSQL outbox、NATS JetStream 或异步 worker；它们仍是长耗时回测的可靠边界。
4. 本 RFC 不把 Step 5 改成同步 HTTP 回测。

## 目标态参数模板 v0

本节先从回测本身出发定义最小参数模型，不以当前 `strategy_backtest_run`、worker 函数或前端组件为边界。一个回测任务应被拆成两层对象：

```text
BacktestSpec
  = 用户可理解、可复现、可 hash 的回测规格

BacktestExecutionPlan
  = 后端从 BacktestSpec 派生出的最小执行计划
```

`BacktestSpec` 只描述“要测什么”和“按什么规则测”。`BacktestExecutionPlan` 才描述“需要读哪些数据、跑哪些阶段、写哪些结果”。两者分开后，后续优化可以集中在 execution plan，不污染用户参数语义。

### BacktestSpec

| 参数组 | 必需/可选字段 | 用途 | 不应混入 |
|---|---|---|---|
| 身份与幂等 | `client_request_id`、派生 `request_hash` | 防重复提交、识别同一规格 | worker 临时状态、队列状态 |
| 规则输入 | `rule_snapshot`、`rule_hash` | 冻结选股和评分语义 | preview rows、展示用 explain JSON |
| 回测范围 | `start_date`、`end_date`、`period_key`、`range_as_of_date` | 冻结交易区间 | 浏览器当前日期推测 |
| Benchmark | `benchmark_security_code` | 计算超额收益、Alpha/Beta/IR | UI label |
| 账户 | `initial_cash`、`currency` | 组合初始状态 | 当前账户余额、真实交易账户 |
| 信号政策 | `buy_signal_top_n`、`signal_timing` | 每日候选信号和 T+1 执行语义 | Step 3 preview row limit |
| 调仓政策 | `max_positions`、`single_position_limit_pct`、`cash_reserve_pct`、`lot_size`、`min_trade_lots` | 仓位、现金和 A 股手数约束 | UI 默认值来源 |
| 成本模型 | commission、stamp duty、transfer fee、slippage | 成交价和费用计算 | 费率模板元数据 |
| 退出政策 | fixed stop、take profit、time stop、indicator stop metrics | 生成卖出事件和下一交易日卖出 | 未启用的指标字段 |
| 数据口径 | `price_basis`、calendar、risk-free tenor | 行情、benchmark 和 risk-free 对齐 | 页面展示格式 |
| 诊断等级 | `diagnostic_mode` 可选 | 控制是否产出完整候选统计、explain、raw values | 默认热路径必需参数 |

第一性原理约束：

1. `BacktestSpec` 必须能稳定 hash；同一 spec 应得到同一执行语义。
2. `BacktestSpec` 不保存可由后端确定性派生的中间结果，例如 price bars、signals、nav。
3. UI 展示字段只进入 `ui_display_snapshot` 或独立 display view，不进入执行语义。
4. 诊断字段必须可关闭。热路径默认只产生用户结果和必要审计字段。

### BacktestExecutionPlan

`BacktestExecutionPlan` 是 worker 真正需要的计划，建议拆成以下派生物：

| 派生物 | 来源 | 内容 | 性能原则 |
|---|---|---|---|
| `SignalQueryPlan` | rule + range + top_n | required metrics/marts、最小 signal SQL、query ids | 默认只返回 TopN 可执行信号；完整候选只在诊断模式开启 |
| `TradeCalendarPlan` | range + calendar | trade dates、`date -> next_trade_date` | 一次构造，多阶段复用 |
| `MarketDataDemand` | signals + exit policy | 每只证券需要的日期范围、OHLC 字段、indicator metrics | 先从信号推导最小数据需求，再查询行情 |
| `SimulationPlan` | execution config + calendar | 现金、仓位、费用、滑点、退出规则、索引策略 | 避免全量 clone；按日期流式推进 |
| `MetricPlan` | nav + benchmark config | benchmark/risk-free 范围、绩效配置 | 只读取计算指标所需序列 |
| `WritePlan` | output + result attempt | facts、calculation outputs、snapshot、finalize pointer | append-only，写完再切当前 attempt |
| `ObservationPlan` | all stages | stage timing、row counts、query ids、memory/read bytes | 默认轻量，可关联 query_log |

这层计划应尽量是纯数据结构。worker 执行只是解释这个 plan；优化就是替换某个 plan 的生成或执行策略。

## 目标态流程模板 v0

不受历史代码约束的理想流程如下：

```text
0. Accept
   BacktestSpec validate/hash -> durable run + outbox

1. Plan
   BacktestSpec -> BacktestExecutionPlan
   compile signal query, derive required metrics, build trade calendar

2. Signals
   run minimal signal query
   output SignalSet { signal_date, execution_date, security_code, rank, score }

3. Market Data Demand
   SignalSet + exit policy -> exact OHLC/indicator demand
   query only needed securities, dates and columns

4. Simulation
   stream trade dates
   apply pending sells, buys, valuation, risk exits
   output nav, positions, targets, orders, trades, events

5. Metrics
   nav + benchmark + risk-free -> performance metrics and trade metrics

6. Commit
   write append-only facts/calculation/snapshot
   finalize run with current_result_attempt_id

7. Serve
   status/result views read compact fields for UI
   diagnostic views read expanded snapshots only when needed
```

### 流程节点职责

这条流程的职责边界是：

```text
冻结输入 -> 派生计划 -> 生成信号 -> 精确取数 -> 组合递推 -> 绩效计算 -> 提交结果 -> 读取展示
```

| 节点 | 职责 | 不应承担 |
|---|---|---|
| `Accept` | 接收回测请求，校验用户提交的 `BacktestSpec`，生成 run id，写入 durable run 和 outbox。 | 不执行回测、不读大规模行情、不等待 worker。 |
| `Plan` | 把冻结参数派生成执行计划：信号 SQL、交易日历、需要的指标、query id、result attempt 策略。 | 不把 preview/explain 的展示字段带进热路径。 |
| `Signals` | 按规则和回测区间生成最小买入信号集：`signal_date`、`execution_date`、`security_code`、`rank`、`score`。 | 不返回全量候选股、不返回 `raw_values`/`score_breakdown`，除非诊断模式需要。 |
| `Market Data Demand` | 根据 signals 和退出规则，推导真正需要的行情数据：哪些证券、哪些日期、哪些 OHLC/指标列。 | 不默认读取所有证券完整区间和所有趋势字段。 |
| `Simulation` | 按交易日递推组合：处理卖出、买入、估值、费用、滑点、止盈止损和指标止损，生成 nav、持仓、订单、成交、事件。 | 不访问数据库、不计算展示层指标、不写结果。 |
| `Metrics` | 基于 simulation 输出的 nav/trades，再结合 benchmark 和 risk-free 计算绩效指标、交易级指标。 | 不重新跑信号、不重新模拟组合。 |
| `Commit` | 把本次 result attempt 的事实结果 append-only 写入 ClickHouse，最后更新 PostgreSQL `current_result_attempt_id`。 | 不在部分写入时暴露成功状态，不用 mutation 覆盖历史结果。 |
| `Serve` | 面向前端读取状态和结果：status、nav、rebalance、performance；默认 compact view，诊断字段走 detail/debug view。 | 不在读取接口里重复重算重活。 |

关键边界：

- `Accept` 只解决“任务是否成立”。
- `Plan` 只解决“怎么最小化执行”。
- `Signals` 只解决“什么时候买什么”。
- `Market Data Demand` 只解决“只取计算必需数据”。
- `Simulation` 只解决“组合每天怎么变化”。
- `Metrics` 只解决“结果表现如何”。
- `Commit` 只解决“结果如何成为权威事实”。
- `Serve` 只解决“页面如何快速读取”。

后续优化主要应该发生在 `Plan -> Signals -> Market Data Demand -> Simulation` 这四段，因为它们决定了 worker 的主要耗时和数据规模。

### 阶段输入输出产物

这里的“产物”不等同于都要落库。后续设计时应把产物分成三类：

- `Durable`：必须持久化，支持恢复、审计、重试和前端状态读取。
- `Ephemeral`：worker 内存态产物，只服务本次执行，不应默认落库。
- `Diagnostic`：诊断产物，只有摘要默认保留，完整明细应受 `diagnostic_mode` 控制。

| 阶段 | 输入 | 输出产物 | 产物类型 | 下游消费者 |
|---|---|---|---|---|
| `Accept` | `BacktestSpecRequest`：rule snapshot、execution config、period/range、benchmark、account、signal/rebalance/cost/exit policy、client_request_id。 | `BacktestRunAccepted`：`strategy_backtest_run_id`、canonical `BacktestSpec`、`rule_hash`、`execution_config_hash`、`request_hash`、`status=queued`、outbox pending task。 | Durable | 前端 Step 5 status shell、outbox dispatcher、worker claim。 |
| `Plan` | Canonical `BacktestSpec`、metric catalog、market calendar、benchmark availability、engine config。 | `BacktestExecutionPlan`：`SignalQueryPlan`、`TradeCalendarPlan`、result attempt policy、required metrics/marts、stage query id prefix、diagnostic mode。 | Ephemeral + Diagnostic summary | `Signals`、`Market Data Demand`、worker timing、progress view。 |
| `Signals` | `SignalQueryPlan`、`TradeCalendarPlan`、rule filter/scoring policy、`buy_signal_top_n`。 | `SignalSet`：`signal_date`、`execution_date`、`security_code`、`rank`、`score`；`SignalSummary`：signal date count、executable count、dropped count、compiled SQL hash。 | Ephemeral + Durable summary | `Market Data Demand`、`Simulation`、PG signal summary、diagnostics。 |
| `Market Data Demand` | `SignalSet`、exit policy indicator metrics、price basis、calendar、run date range。 | `MarketDataDemand`：security list、per-security date window、required OHLC columns、required indicator columns；`MarketDataSet`：price bars keyed by date/security。 | Ephemeral + Durable coverage summary | `Simulation`、data coverage summary、ClickHouse query log correlation。 |
| `Simulation` | `BacktestSpec` execution policy、`SignalSet`、`MarketDataSet`、`TradeCalendarPlan`。 | `PortfolioSimulationOutput`：nav rows、position-day rows、target rows、order rows、`TradeLedger`/`SettlementLedger` rows、event rows、portfolio summary、warning counts。 | Ephemeral until commit | `Metrics`、`Commit`、worker summary、交割单视图。 |
| `Metrics` | nav rows、trades、closed position context、benchmark returns、risk-free rates、metric config。 | `PerformanceOutput`：portfolio performance metric、metric statuses、daily win rate；`TradeMetricOutput`：closed trades、trade metrics。 | Ephemeral until commit | `Commit`、Step 5 performance view。 |
| `Commit` | result attempt id、simulation output、performance output、run snapshot payload。 | ClickHouse portfolio facts、ClickHouse calculation outputs、portfolio run snapshot、PostgreSQL metric config、PostgreSQL `current_result_attempt_id` finalize。 | Durable | `Serve`、publish flow、future rerun comparison、diagnostics。 |
| `Serve` | run id、current result attempt id、view mode、optional selected trade date/window filters。 | `StatusView`、`NavUiView`、`RebalanceUiView`、`PerformanceUiView`、diagnostic detail views。 | Read model | Racingline Step 5 UI、debug/diagnostic tools。 |

阶段之间只传必要产物：

```text
Accept -> Plan:
  canonical BacktestSpec + run id

Plan -> Signals:
  SignalQueryPlan + TradeCalendarPlan

Signals -> Market Data Demand:
  SignalSet + required exit indicator metrics

Market Data Demand -> Simulation:
  MarketDataSet + TradeCalendarPlan

Simulation -> Metrics:
  nav/trades/positions/events summary

Metrics -> Commit:
  performance metrics + trade metrics

Commit -> Serve:
  current_result_attempt_id + compact read models
```

最小产物原则：

1. `SignalSet` 不携带 preview 展示字段；完整候选、`raw_values`、`score_breakdown` 只属于诊断。
2. `MarketDataSet` 不携带未启用指标列；未启用 indicator stop loss 时不需要 trend indicator join。
3. `Simulation` 输出事实行，但不负责持久化；持久化由 `Commit` 统一保证顺序。
4. `Metrics` 只消费已经完成的 simulation output，不反向读取或修改信号。
5. `Serve` 只读已提交结果，不做 heavy recompute；需要诊断时显式请求 detail/debug view。

### 交割单产物

回测应把交割单作为一等输出产物。建议目标态命名为 `TradeLedger` 或 `SettlementLedger`，由 `Simulation` 阶段生成，`Commit` 阶段持久化，`Serve` 阶段提供查询视图。

单笔交割明细至少包含：

| 字段组 | 字段 | 说明 |
|---|---|---|
| 归属 | `portfolio_run_id`、`result_attempt_id`、`trade_seq`、`order_seq` | 绑定本次回测 attempt 和成交顺序 |
| 时间 | `trade_date`、`signal_date` | 成交日和触发信号日 |
| 标的 | `security_code` | 成交证券 |
| 方向与数量 | `side`、`quantity` | 买入/卖出和成交数量 |
| 价格 | `reference_price`、`execution_price` | 参考价和含滑点后的执行价 |
| 金额 | `gross_amount` | 成交金额，不含费用 |
| 费用金额 | `commission`、`stamp_duty`、`transfer_fee`、`total_fee`、`slippage_cost` | 佣金、印花税、过户费、总费用和滑点成本 |
| 原因 | `reason` | 调仓、固定止损、止盈、时间止损、指标止损等 |

买入笔数、卖出笔数、成交总金额、买入金额、卖出金额、总费用等汇总指标，不需要在明细行重复存储；它们应从 `TradeLedger` 聚合得到：

```text
buy_count        = count(side = Buy)
sell_count       = count(side = Sell)
buy_amount       = sum(gross_amount where side = Buy)
sell_amount      = sum(gross_amount where side = Sell)
turnover_amount  = sum(gross_amount)
fee_amount       = sum(total_fee)
slippage_amount  = sum(slippage_cost)
```

当前实现已经具备核心交割单事实表，对应 ClickHouse `fleur_portfolio.portfolio_trade`。该表记录成交数量、成交金额、费用金额和滑点成本；费率本身来自本次 run 的 `execution_config.fee_profile`，不在每笔 trade row 中重复存储。若后续交割单页面需要展示“费率”，应从冻结的 `BacktestSpec`/run snapshot 读取费率配置，或在服务层组装视图时附加，而不是把费率配置复制到每一笔成交明细。

现有结果事实关系：

| 目标态产物 | 当前事实表 | 作用 |
|---|---|---|
| `TradeLedger` / `SettlementLedger` | `portfolio_trade` | 成交交割明细：买卖方向、数量、价格、金额、费用、滑点、原因 |
| `OrderLedger` | `portfolio_order` | 委托/订单记录：成交、跳过、价格缺失、现金不足等状态 |
| `TargetLedger` | `portfolio_target` | 目标调仓记录：信号排名、目标权重、目标金额、目标数量 |
| `PositionLedger` | `portfolio_position_day` | 每日持仓快照和未实现盈亏 |
| `NavLedger` | `portfolio_nav_daily` | 每日净值、现金、仓位市值、换手、费用和 warning count |

阶段边界和可优化点：

| 阶段 | 输入 | 输出 | 可优化点 |
|---|---|---|---|
| Accept | user request | durable run id | 只做可信边界校验，不做重复 heavy preflight |
| Plan | frozen spec | execution plan | 缓存 catalog/range，分离 execution SQL 与 preview SQL |
| Signals | signal plan | `SignalSet` | TopN SQL、少字段、少 JSON、chunk 并行实验 |
| Market Data | signal set | price bars | 动态列、按证券最早执行日裁剪、Per `schema-pk-filter-on-orderby` 保持可用索引条件 |
| Simulation | config + bars + calendar | portfolio output | 低 clone 索引、`next_trade_date` map、按日期流式递推 |
| Metrics | nav + benchmark | performance output | 固化 daily win rate，避免 wrapper 重读 nav |
| Commit | output | result attempt | Per `insert-batch-size` 监控 batch/parts，保持 append-only |
| Serve | result attempt | UI view | compact view 默认，diagnostic view 显式请求 |

这个模板给后续讨论提供一个更干净的判断标准：如果某个流程不改变 `BacktestSpec`，也不减少 `BacktestExecutionPlan` 的必要数据需求，它就不应该进入热路径。

## 当前实现映射

| 目标态对象 | 当前实现位置 | 差距 |
|---|---|---|
| `BacktestSpec` | PostgreSQL `strategy_backtest_run` 的 `rule_snapshot`、`execution_config`、range、benchmark、hash 字段 | display、preflight、progress 和 execution 字段混在同一 record 中，语义上可接受但阅读成本高 |
| `SignalQueryPlan` | `QueryPlanner::compile()` | 使用 preview/explain 风格 SQL，返回全量 ranked candidates 和 JSON 解释字段；缺 worker 专用最小 SQL |
| `TradeCalendarPlan` | `query_trade_dates()` + worker 局部 `next_trade_date()` | worker signal 阶段有 trade dates，simulation 阶段又从 price map 推导下一交易日，没有统一 calendar plan |
| `MarketDataDemand` | `query_portfolio_price_bars(security_codes, start, end, indicator_metrics)` | 已支持动态指标列，但仍按所有入选证券读取完整 run 区间 |
| `SimulationPlan` | `PortfolioSimulationInput` | 输入清晰，但执行时 clone 全量 `PriceBar` 到 map，并反复构造 string key |
| `MetricPlan` | `PerformanceMetricConfig::full_period_with_benchmark()` | benchmark/risk-free 查询清晰；daily win rate 仍在 wrapper 里通过 nav 重算 |
| `WritePlan` | `write_portfolio_result_facts()` + calculation writes + snapshot + PG finalize | 顺序正确；后续只需监控 parts 和小批写 |
| `ObservationPlan` | `WorkerTiming` + query ids + summary | 已有总阶段 timing；simulation 内部和 signal SQL 输出规模还需要更细指标 |

## 当前用户可见流程

当前 Step 5 主路径已经是正确的异步页面模型：

```text
Step 4 点击「策略回测」
  -> openBacktest()
  -> POST /rearview/strategy-backtests
  -> create 返回 202 Accepted 和 full run
  -> 前端 setActiveBacktestRun + setActiveStep("backtest")
  -> Step 5 轮询 GET /rearview/strategy-backtests/{id}/status
  -> status = succeeded 且 current_result_attempt_id 存在
  -> Step 5 读取 /nav?view=ui、/rebalance-records?view=ui、/performance?view=ui
```

对应代码事实：

- `app/racingline/src/routes/strategy-page.tsx` 的 `openBacktest()` create 成功后调用 `acceptStrategyBacktestRunForStep5()` 并进入 Step 5。
- `BacktestPanel` 使用 `useStrategyBacktestStatusQuery(activeRunId)` 轮询轻量 status。
- 结果查询由 `isStrategyBacktestResultReady()` gate 控制，只有 `succeeded + current_result_attempt_id + config 未变化` 时启用。

因此，当前优化重点不是再改 Step 4 handoff，而是缩短 Step 5 中 `queued/running` 到 `succeeded` 的真实等待。

## 后端控制面流程

`POST /rearview/strategy-backtests` 当前同步阶段：

1. 校验 `period_key` 和 benchmark allowlist。
2. 动态解析 benchmark 可用区间。
3. 重新 validate `RuleVersionSpec + BacktestExecutionConfig` 并校验 hash/top_n。
4. 计算 catalog hash。
5. 查询 risk-free rows 作为 preflight snapshot。
6. 计算 request hash 和 `client_request_id` 幂等。
7. 在 PostgreSQL 事务中写 `strategy_backtest_run(status='queued')` 和 `strategy_backtest_task_outbox(status='pending')`。
8. 通过进程内 `Notify` 唤醒 outbox dispatcher，返回 `202 Accepted`。

必须保留：

- 服务端 canonical validate/hash，因为前端 draft 不是可信边界。
- PostgreSQL run + outbox 同事务，因为它是任务可恢复和可追踪的 durable 边界。
- `client_request_id` 幂等冲突判断。

可评估删减：

- Create 阶段 risk-free preflight 当前只记录 snapshot，worker 后续计算 performance 仍会重新读取 risk-free rows。若它不参与 create 成败判断，可考虑移到 worker coverage summary 或后台诊断，减少 create 同步职责。该项当前只有 7-8ms 实测收益，不是主瓶颈。

## Worker 执行流

当前 `rearview-portfolio-worker` 是单 consumer 主循环：收到一条 NATS message 后完整处理、ack，再处理下一条。strategy backtest 路径如下：

```text
NATS message { kind: "strategy_backtest", run_id }
  -> get run
  -> terminal 则跳过
  -> claim_strategy_backtest_run(run_id, lease=900s)
  -> process_strategy_backtest_run()
  -> 成功 ack NATS
```

`process_strategy_backtest_run()` 内部步骤：

| 步骤 | 当前状态 | 主要代码 | 必须性 | 备注 |
|---|---|---|---|---|
| S0 load execution config | `load_execution_config` | `serde_json::from_value::<StrategyExecutionConfig>` | 必须 | 需要冻结执行配置进入强类型执行路径 |
| S1 materialize signals | `compiling_signals` / `running_clickhouse` | `materialize_strategy_backtest_signals()` | 必须 | 2y 样本 4.385s，是第一梯队瓶颈 |
| S2 load market data | `loading_market_data` | `query_portfolio_price_bars()` | 必须 | 2y 样本 4.302s，已做动态趋势列投影 |
| S3 simulate portfolio | `calculating_nav` | `simulate_portfolio()` | 必须 | 2y 样本 4.212s，源码显示存在高价值数据结构优化点 |
| S4 compute performance | `computing_performance` | benchmark/risk-free queries + `compute_performance_metric()` | 必须 | 查询本身很小，计算和输出归入当前 timing |
| S5 prepare calculation outputs | `output_serialization_write_preparation` | `compute_trade_calculation_outputs()` | 必须 | 可继续拆分 closed trade 和 trade metric 耗时 |
| S6 write results | `writing_results` | ClickHouse facts/calculation/snapshot + PG metric config | 必须 | 2y 样本 0.838s，低于前三段 |
| S7 finalize run | `succeeded` | `finalize_strategy_backtest_run_to_clickhouse()` | 必须 | 唯一把 `current_result_attempt_id` 暴露给 Step 5 的动作 |

## Worker 编排抽象模板

后续整理 worker 代码时，可以把 strategy backtest、strategy portfolio daily run 和普通 portfolio run 收敛到同一类 durable task 模板：

```text
DurableWorkerTask
  identity:
    task_kind
    run_id
    result_attempt_id policy

  control_plane:
    load_run()
    is_terminal()
    claim(lease)
    update_progress(stage, payload)
    fail(status, error)
    finalize(summary, result_attempt_id)

  execution:
    load_config()
    materialize_signals()
    load_market_data()
    simulate()
    compute_metrics()
    write_outputs()

  observability:
    timing.mark(stage)
    query_id(stage)
    row_count(stage)
    summary_with_worker_timing()
```

这个模板不是为了增加抽象层，而是为了删除当前重复的编排代码。`materialize_strategy_backtest_signals()` 和 `materialize_strategy_portfolio_daily_run_signals()` 已经高度相似：都解析 frozen rule、查 trade dates、按 chunk 编译 SQL、查询 screening rows、筛 TopN、映射下一交易日、生成 `MaterializedSignals` 和 summary。差异主要是 run 字段名、progress updater、结束日期和 summary 写入目标。

建议后续抽象边界：

| 抽象 | 输入 | 输出 | 说明 |
|---|---|---|---|
| `SignalMaterializationJob` | rule snapshot、date range、top_n、progress sink、query id prefix | signals、security codes、compiled hash、required metrics/marts、summary | 合并 backtest 和 daily run 的重复逻辑 |
| `MarketDataLoadJob` | security/date demand、indicator metrics | price bars、coverage summary | 后续支持按证券最早执行日裁剪 |
| `SimulationJob` | execution config、signals、price bars | portfolio output、simulation timing detail | 方便对模拟器做基准和数据结构替换 |
| `ResultWriteJob` | run adapter、attempt id、portfolio output、calculation batch | write summary | 保持 ClickHouse append-only 和 PG finalize 顺序 |

## 必须流程与多余流程

### 必须保留

1. 异步 worker 边界：回测覆盖全市场、多年、多 mart，不能同步 HTTP 等待。
2. Frozen input：`rule_snapshot`、`execution_config`、period、benchmark、hash 和 catalog hash 必须随 run 固化。
3. Signal materialization：Step 5 不能复用 Step 3 preview rows；必须按回测区间重新执行规则。
4. T+1 execution mapping：买入信号必须从信号日映射到下一交易日执行，且不得越过 run `end_date`。
5. Price bars：组合递推必须读取执行价、估值价和启用的指标止损字段。
6. Append-only result attempt：ClickHouse 结果事实继续按 `portfolio_run_id + result_attempt_id` 写入，PostgreSQL 只切换当前 attempt 指针。
7. Status/progress：Step 5 必须能看到 queued/running/succeeded/failed 和 progress payload。

### 可删减或收窄

1. **回测信号不需要全量候选字段。** 当前 `QueryPlanner::compile()` 返回 `raw_score`、`score_breakdown`、`selected_metrics`、`raw_values` 等字段，worker 实际只使用 `security_code`、`trade_date`、`signal_rank`、`score` 和 `is_buy_signal`。这些 JSON 字段适合 preview/explain，不适合 worker 热路径。
2. **worker 不一定需要拉回全量 ranked candidates。** 当前 backtest SQL 没有 `WHERE signal_rank <= top_n`，worker 在 Rust 中二次过滤。后续可以增加专用 backtest signal SQL，只返回每个交易日 TopN 可执行候选；如果仍需要 `generated_candidate_count`，用独立聚合或把该统计降级为诊断字段。
3. **price bars 不一定需要每个入选证券的完整 run 区间。** 当前 `query_portfolio_price_bars()` 对所有入选证券读取 `run.start_date..run.end_date`。由于初始无持仓，某证券最早只从其第一次 execution date 开始需要价格；可以按 `security_code -> min_execution_date` 裁剪读取范围，但 SQL 形态必须用真实数据基准验证。
4. **模拟器不应 clone 全量 PriceBar 作为索引。** `simulate_portfolio()` 当前把 `input.prices` 每根 bar clone 到 `BTreeMap<(NaiveDate, String), PriceBar>`。2y 样本已有 374,548 bars；按 Rust 性能原则，热路径应避免大集合重复 clone。
5. **下一交易日查找不应扫描 price map。** `portfolio::next_trade_date()` 当前从 `BTreeMap` keys 开始找第一个大于当前日期的 key。它在每天每个持仓的退出检查中调用，应该由 trade_dates vector 或 date successor map 提供 O(1)/O(log n) 查询。
6. **result wrapper 仍有重复 nav 读取。** `/nav`、`/rebalance-records` 和 `/performance` 都会读取 nav；`/performance` 用 nav 计算 daily win rate。当前 HTTP 只有 0.10-0.17s，但可通过 overview endpoint、缓存或 worker finalize summary 收敛。
7. **signal materialization 编排重复。** Strategy backtest 和 strategy portfolio daily run 的 signal materialization 可抽象成同一模板，减少后续 bug 和优化重复实现。

## 性能优化方向

### P0: 拆清 simulation 阶段内部耗时

已测 2y simulation 为 4.212s，但当前 timing 没有拆出：

- price index build。
- trade_dates / signals index build。
- daily loop。
- exit rule evaluation。
- output rows allocation。

后续实施前应先增加局部 timing 或 criterion benchmark。按 Rust performance mindset，优化要以 release/profile 数据为准。

重点候选：

- 把 `BTreeMap<(NaiveDate, String), PriceBar>` 改成不 clone `PriceBar` 的索引，例如 `(date, security_code) -> price index`。
- 预先构造 `trade_date -> next_trade_date` map，替代每次从 price map keys 扫描。
- 如果仍使用 map key，避免在 `open_price()` / `close_price()` 查找时反复 `security_code.to_string()`。

预期讨论目标：如果 2y simulation 的 4.212s 主要来自索引 clone 和 next-date scan，优化后应争取降到 1s 级别；最终必须以同一 run 的 output hash/row count 对比确认。

### P1: 增加 backtest 专用 signal SQL

当前 `QueryPlanner::compile()` 是通用筛选结果 SQL，适合 preview/explain，但 worker 热路径只需要 TopN 可执行信号。

建议后续设计一个窄 contract：

```text
compile_backtest_signals(rule, start_date, end_date, top_n)
  -> SELECT security_code, trade_date, score, signal_rank
     FROM ranked
     WHERE signal_rank <= top_n
     ORDER BY trade_date, signal_rank, security_code
```

可选诊断统计：

- `generated_candidate_count`：独立 `count()` 聚合或只在 debug mode 计算。
- `signal_date_count`：可由 TopN rows 近似为有可执行候选的日期数；如果业务必须保留“全部命中日期”，需要独立聚合。
- `required_metrics` / `required_marts` / SQL hash：仍由 planner compile 产出。

按 ClickHouse `query-join-filter-before`，现有 CTE 已按 date filter 限制输入 mart；按 `query-join-choose-algorithm`，planner 当前使用 `ANY LEFT JOIN` 和 `join_algorithm='auto'`，大方向正确。下一步收益主要来自减少输出列、减少 JSONEachRow 序列化和减少 Rust 解析/过滤。

### P2: 裁剪 market data demand

Plan 0056 已完成动态趋势列投影，2y price bars read_bytes 从 baseline 514.89 MiB 降到 151.86 MiB。下一步不应再回到“全字段读取”，而应减少行数：

```text
signals -> security_code -> earliest_execution_date
market data demand = each security [earliest_execution_date, run.end_date]
```

需要谨慎验证：

- Per `schema-pk-filter-on-orderby`，查询仍应让 `trade_date` 和 `security_code` 命中排序键过滤。
- Per `query-join-filter-before`，quotes/trend 两侧都必须先过滤再 join。
- 如果用巨大 OR 条件表达 per-security date range，可能反而变慢；需要比较 inline demand table join、分 chunk 查询和当前单查询。

该项收益取决于信号分布。如果大多数证券很早出现信号，收益小；如果很多证券只在后半段第一次出现，收益大。

### P3: 解决 worker 排队吞吐

2y 验收样本 backend total 20.066s，其中 6.238s 是 pickup 等待，因为同一个 worker 先处理了 1y run。当前 worker 主循环天然串行。

候选方案：

1. 启动多个 `rearview-portfolio-worker run` 进程，用 PostgreSQL claim lease 保证同一 run 只被一个 worker 处理。
2. 在单进程内引入有限并发处理 NATS messages，但必须控制 ClickHouse 并发和内存。
3. 给 strategy backtest 与 daily run 分 durable consumer 或 subject，避免日运行和交互式回测互相排队。

该项优化的是多人/多 run 吞吐和排队时间，不一定缩短单个 run 的 `worker_elapsed`。

### P4: Wrapper 首屏合并或缓存

当前 Step 5 succeeded 后首屏读：

- `/nav?view=ui`
- `/rebalance-records?view=ui`
- `/performance?view=ui`

`/rebalance-records` 和 `/performance` 都会再读 nav。后续可选：

1. 新增 Step 5 overview endpoint，一次返回 status、nav UI、performance UI、rebalance rail 和 selected rows。
2. 把 `daily_win_rate` 写入 worker summary 或 performance calculation output，避免 performance wrapper 每次重读 nav。
3. 让 rebalance records 首屏只返回日期轴 summary，selected rows 延后到用户选择日期时读取。

该项当前优先级低于 worker 三段瓶颈，因为 HTTP 实测只有 0.10-0.17s。

### P5: 写入和 ClickHouse parts 监控

当前 writes 约 0.8s，active parts 最高约 99，未到风险线。按 `insert-batch-size`，后续只有在使用量上升导致小 part 增长明显时，再评估：

- async insert。
- 低优先级明细延迟写。
- 多 run 合批。

继续保留 append-only `result_attempt_id`，符合 `insert-mutation-avoid-update` 的方向。

## ClickHouse 规则核对

- Per `schema-pk-filter-on-orderby`，现有 result wrapper 按 `portfolio_run_id + result_attempt_id` 读取，portfolio result tables 的 `ORDER BY (portfolio_run_id, result_attempt_id, ...)` 与访问模式一致。
- Per `query-join-filter-before`，price bars 查询必须继续保证 quotes 和 trend 两侧先按 date/security 过滤，再 join。
- Per `query-join-use-any`，trend 表样本区间 `(security_code, trade_date)` 无重复，可继续评估 `LEFT ANY JOIN`，但 Plan 0056 已证明主要收益来自动态列投影。
- Per `query-join-choose-algorithm`，planner 使用 `ANY LEFT JOIN` 和 `join_algorithm='auto'`，后续只有在 EXPLAIN 或 query_log 显示 join 算法成为瓶颈时再手动指定。
- Per `insert-batch-size`，当前多表小批写尚未触发 parts 风险，但需要持续监控。

## 优化方案草案

本 RFC 的优化主线应从 `BacktestExecutionPlan` 入手，而不是改变 `BacktestSpec`。`BacktestSpec` 代表用户提交的稳定语义，必须可复现；优化只应该减少执行计划中的无效数据面、重复转换和等待时间。

第一阶段先建立可证明的优化闭环：

1. 固定 1y/2y/3y 代表性 run 输入，形成 before/after 基准样本。
2. 给 simulation 增加内部 timing，至少拆出 price index build、signal index build、daily loop、exit evaluation、output allocation。
3. 每轮优化同时记录 worker timing、ClickHouse query_log、read_rows/read_bytes/memory、结果 row count、summary 和关键指标。
4. 同一输入优化前后必须证明业务输出一致；如果存在浮点细微差异，需要在 RFC/plan 中定义可接受误差。

第二阶段做单 run 热路径减法，按收益确定性排序：

| 优化对象 | 当前问题 | 方案 | 验收口径 |
|---|---|---|---|
| `Signals` | worker 使用 `QueryPlanner::compile()`，输出全量 ranked candidates 和 `score_breakdown`/`raw_values` 等 JSON 字段，再在 Rust 中过滤 TopN。 | 新增 worker 专用 `compile_backtest_signals()`，默认只返回 `trade_date`、`security_code`、`score`、`signal_rank`，并在 SQL 层限制 `signal_rank <= top_n`；完整候选和 explain JSON 进入诊断模式。 | 2y `signal_materialization_total` 从 4.385s 降到 2-3s 区间；信号 row count、security set 和执行日期映射一致。 |
| `Market Data Demand` | price bars 已动态投影趋势列，但仍对所有入选证券读取完整 run 区间。 | 从 `SignalSet` 派生 `security_code -> earliest_execution_date`，实验 inline demand table join、分 chunk 查询和当前单查询三种 SQL 形态；保留必要 OHLC/indicator 列。 | 在遵守 `schema-pk-filter-on-orderby` 和 `query-join-filter-before` 的前提下，2y price bars worker 阶段耗时低于当前 4.302s，且 read_rows/read_bytes 同步下降。 |
| `Simulation` | 当前把所有 `PriceBar` clone 到 `BTreeMap<(NaiveDate, String), PriceBar>`，查价时构造 String key，下一交易日从 price map keys 扫描。 | 引入低 clone `PriceStore`/`TradeCalendar`：用价格数组索引或 borrowed/index key 替代全量 clone；预构造 `trade_date -> next_trade_date`；持仓估值和退出检查复用同一查价接口。 | 2y simulation 从 4.212s 降到 1s 级别；targets/orders/trades/positions/nav/events row count 和 summary 一致。 |

第三阶段处理结果提交和首屏读取：

1. `Commit` 继续保持 append-only result attempt 和 PG finalize 指针，不使用 mutation 覆盖历史结果。
2. `daily_win_rate`、交易汇总、交割单聚合这类稳定结果优先在 worker output 或 calculation outputs 中固化，减少 `/performance` wrapper 重读 nav。
3. Step 5 首屏若仍有明显迟滞，再新增 compact overview endpoint，一次返回 status、performance summary、nav summary 和 rebalance 日期轴；明细交割单、持仓和订单按用户选择分页读取。

第四阶段再做队列和并发：

1. 单 run 热路径降下来之后，再启动多 worker 进程或单进程 bounded concurrency。
2. 若交互式 backtest 与 daily run 互相影响明显，再拆 NATS subject/durable consumer。
3. 并发上限必须绑定 ClickHouse 内存和查询并发预算，避免把单次慢查询放大成集群抖动。

不建议优先做的方案：

1. 不建议把 Step 5 回测改回同步 HTTP；这会重新把页面体验绑定到长耗时计算。
2. 不建议复用 Step 3 preview rows 作为回测信号；preview 是展示抽样语义，不能替代全区间冻结回测。
3. 不建议先上 worker 并发掩盖单 run 慢路径；它只能降低 pickup wait，不能降低 `worker_elapsed`。
4. 不建议为了少写代码把诊断字段继续混在热路径默认输出中；诊断应是可开关能力。

按当前样本判断，完成第二阶段后，2y `worker_elapsed` 的合理讨论目标是 6-9s；若第三、四阶段同时完成，单 run 等待主要取决于 worker elapsed，排队 pickup p95 应争取低于 1s。这个目标不是承诺值，必须以同一输入样本的 before/after 数据确认。

## 建议后续实施顺序

1. **观测先行**：给 simulation 内部补 timing，或写独立 release benchmark，确认 4.212s 的具体来源。
2. **模拟器数据结构优化**：优先处理全量 `PriceBar` clone、字符串 key 构造和 next-trade-date 扫描。
3. **backtest signal SQL 窄化**：新增只返回 TopN 信号的 worker 专用 SQL，保留 preview/explain 的完整字段 SQL。
4. **market data 行数裁剪实验**：基于同一 1y/2y/3y run 比较当前单查询、per-security earliest execution 裁剪、分 chunk 查询的 read_rows/read_bytes/memory/duration。
5. **worker 并发/队列隔离**：在单 run 耗时下降后，再做多 worker 或 durable consumer 拆分，避免用并发掩盖单 run 热路径低效。
6. **wrapper overview**：如果 Step 5 succeeded 后 time-to-chart 仍慢，再合并或缓存首屏 wrapper。

## 讨论用目标

在不改变业务结果的前提下，下一轮优化可以设定讨论目标：

| 指标 | 当前样本 | 讨论目标 |
|---|---:|---:|
| 1y worker elapsed | 6.477s | 3-4s |
| 2y worker elapsed | 13.797s | 6-9s |
| 2y simulation | 4.212s | 1s 级别 |
| 2y signal materialization | 4.385s | 2-3s |
| 2y price bars | 4.302s | 2-3s |
| pickup wait | 单 worker 下可被前序 run 阻塞 | 多 worker/隔离后 p95 < 1s |

这些目标不是承诺值。它们依赖信号宽度、period、ClickHouse 当前负载、worker 并发和具体策略规则。实施计划必须用同一输入 run 做 before/after 对比，并验证 result row count、summary、hash 或关键业务指标一致。

## 开放问题

1. `generated_candidate_count` 和完整 `score_breakdown/raw_values` 是否必须作为每次 backtest 的持久诊断？如果不是，worker SQL 可以大幅收窄。
2. Step 5 是否需要展示更细的 worker stage 进度，例如 signal chunk、price bars、simulation、writing？如果需要，progress payload 应标准化。
3. 是否允许交互式 backtest 与 strategy portfolio daily run 使用不同 NATS subject/durable consumer，避免相互排队？
4. market data demand 是否可以按证券最早 execution date 裁剪，还是为了诊断和重算简单性保留全区间？
5. daily win rate 属于实时 wrapper 计算，还是应作为 worker performance output 固化？

## 结论

当前 Step 5 慢的主因已经不是页面跳转或 outbox，而是 worker 内部三段热路径：signal materialization、price bars 和 simulation。第一轮最值得做的减法是把 worker 热路径从 preview/explain 风格的“全量候选、全量字段、全区间行情、全量 clone 索引”收敛为回测真正需要的最小数据面：TopN 可执行信号、必要行情列和必要日期范围、低 clone 的价格索引、常数级下一交易日查找。并发 worker 可以解决排队，但不应替代单 run 热路径治理。
