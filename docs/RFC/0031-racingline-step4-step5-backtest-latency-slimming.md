# RFC 0031: Racingline Step 4 到 Step 5 回测跳转链路瘦身与延时治理

状态：Proposed（当前分析记录，2026-06-25）
领域：racingline, rearview
关联系统：racingline, rearview, clickhouse marts
代码根：app/racingline/, engines/crates/rearview-core/, engines/crates/rearview-server/, engines/crates/rearview-portfolio-worker/, pipeline/migrate/
系统地图：docs/systems/racingline.md, docs/systems/rearview.md
实测报告：docs/jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-baseline.md

## 摘要

`/strategies` 从 Step 4「模拟建仓」点击「策略回测」到进入 Step 5「策略回测」的当前路径，把页面跳转设计成了等待一次完整异步 backtest worker 终态：

```text
Step 4 activeStep = simulation
  -> Step 4 validate query 已经得到 BacktestExecutionDraft
  -> 点击「策略回测」
  -> openBacktest()
  -> POST /rearview/strategy-backtests
  -> PostgreSQL strategy_backtest_run + outbox
  -> rearview-server outbox dispatcher 发布 NATS typed task
  -> rearview-portfolio-worker 消费并完整计算
  -> worker 写 ClickHouse 结果和 PostgreSQL succeeded/current_result_attempt_id
  -> openBacktest() 手动轮询 GET /rearview/strategy-backtests/{id} 直到 terminal
  -> 额外等待 600ms
  -> setActiveStep("backtest")
  -> Step 5 挂载后再请求 nav / rebalance-records / performance
```

已确认的第一性原理问题不是“异步 worker 存在”本身，而是 UI 把“进入 Step 5 页面”错误绑定到“worker 已完成并有结果”。Step 5 页面的第一责任应是承接已创建的 backtest run，并展示 queued/running/succeeded/failed 状态；只有净值、调仓和绩效区域需要等结果。两点之间的直线是：

```text
Step 4 点击策略回测
  -> 创建 durable backtest run 返回 202 Accepted
  -> 立即进入 Step 5
  -> Step 5 页面内轮询 run 状态并按区域展示 loading/result/error
```

这样可以把用户感知的页面跳转等待从“完整 worker 计算时间 + 队列延迟 + 轮询粒度 + 额外 600ms”收缩为“create API 接受任务的同步耗时”。后端 worker 和 NATS 仍保留，因为回测本身可能覆盖多年、多 mart 和全 A 股，不适合同步 HTTP 完成。

## 当前事实

### 前端 Step 4 gate

Step 4 页面激活后，`app/racingline/src/routes/strategy-page.tsx` 会在 `activeStep === "simulation" || activeStep === "backtest"` 时启用 `useStrategyBacktestValidateQuery()`：

- `backtestValidateDraft` 由非 stale `PreviewSnapshot`、默认市场费率模板和 `SimulationSettings` 构造，见 `strategy-page.tsx:1771` 到 `1814`。
- `BacktestExecutionDraft` 由 validate response 转换，见 `strategy-page.tsx:1815` 到 `1831`。
- `canEnterBacktest` 要求 `backtestExecutionDraft` 存在、没有 validation error、没有 pending validate、create mutation 不在 pending，见 `strategy-page.tsx:2291` 到 `2297`。
- 点击 Step 4 底部「策略回测」按钮调用 `openBacktest()`，见 `strategy-page.tsx:2974` 到 `2992`。

因此，用户点击进入 Step 5 时，Step 4 draft validate 已经是前置 gate。再次进入后端 create API 时仍会重新 validate/hash，这是服务端可信边界需要，不是问题。

### 前端阻塞跳转

`openBacktest()` 是当前用户等待时间的直接来源：

1. 如果已有可复用 completed run，直接 `setActiveStep("backtest")`，见 `strategy-page.tsx:2193` 到 `2197`。
2. 否则打开不可关闭的「策略回测准备中」dialog，见 `strategy-page.tsx:2199` 到 `2202` 和 `2300` 到 `2320`。
3. 构造 create request 并调用 `initialBacktestMutation.mutateAsync()`，见 `strategy-page.tsx:2212` 到 `2223`。
4. create 返回后只把 phase 改为 `queued`，仍不进入 Step 5，见 `strategy-page.tsx:2225` 到 `2230`。
5. `waitForBacktestTerminalRun()` 每 1 秒手动 `GET /rearview/strategy-backtests/{id}`，直到 terminal status，见 `strategy-page.tsx:1617` 到 `1633` 和 `2232` 到 `2246`。
6. terminal 后再等待 `waitForBacktestCompletedMessage()` 的 600ms，见 `strategy-page.tsx:1613` 到 `1615` 和 `2250`。
7. 最后关闭 dialog 并 `setActiveStep("backtest")`，见 `strategy-page.tsx:2251` 到 `2252`。

这意味着 Step 4 到 Step 5 的页面跳转被完整 worker 结果绑定，严重放大用户感知延时。

### 前端 Step 5 内部还有第二套 run 创建路径

`BacktestPanel` 内部还有自己的 create mutation 和自动提交逻辑：

- `BacktestPanel` 内部创建 `useStrategyBacktestCreateMutation()`、`useStrategyBacktestQuery()`、`useStrategyBacktestNavQuery()`、`useStrategyBacktestRebalanceRecordsQuery()` 和 `useStrategyBacktestPerformanceQuery()`，见 `strategy-page.tsx:431` 到 `463`。
- `runBacktest()` 会再次构造 create request 并调用 create mutation，见 `strategy-page.tsx:599` 到 `623`。
- `useEffect()` 在没有 active run、签名未提交且 action 未禁用时自动 `runBacktest()`，见 `strategy-page.tsx:638` 到 `661`。

当前 Step 4 的 `openBacktest()` 会先创建并等到终态，再把 `initialRun` 传给 `BacktestPanel`，所以正常主路径不会触发第二次自动创建。但状态模型已经分裂成两套 mutation、两套 polling 和一套 auto-submit guard，维护成本高，后续功能容易产生重复 run 或状态竞态。

### Rearview create API 同步工作

`POST /rearview/strategy-backtests` 的 HTTP 阶段不是单纯插入 outbox：

1. 校验 `period_key` 和 benchmark allowlist，见 `engines/crates/rearview-core/src/api/mod.rs:525` 到 `530`。
2. 调用 `resolve_strategy_backtest_range()`，查询交易日和 benchmark returns，动态解析 `1y/2y/3y`，见 `api/mod.rs:531` 到 `544`、`4409` 到 `4540`。
3. 重新执行 `StrategyBacktestValidateRequest::validate()`，并校验前端带来的 hash/top_n，见 `api/mod.rs:546` 到 `578`。
4. 计算 catalog hash，并同步查询 risk-free rates 作为 preflight snapshot，见 `api/mod.rs:587` 到 `616`。
5. 计算 request hash，按 `client_request_id` 做幂等冲突判断，见 `api/mod.rs:617` 到 `643`。
6. 事务内写 `strategy_backtest_run(status='queued', dispatch_status='pending')` 和 `strategy_backtest_task_outbox(status='pending')`，见 `postgres/mod.rs:703` 到 `798`。
7. 返回 `202 Accepted` 和 run record，见 `api/mod.rs:645` 到 `672`。

这些同步工作会影响点击后第一段等待，但它们仍远小于完整 worker 计算时不应阻塞页面跳转。

### Outbox 与 NATS 分发

Rearview HTTP 服务启动时在进程内 spawn outbox dispatcher，见 `engines/crates/rearview-server/src/main.rs:57` 到 `63`。

当前 dispatcher 行为：

- 每轮先扫 portfolio outbox，再扫 strategy backtest outbox，再扫 strategy portfolio daily outbox，见 `main.rs:117` 到 `211`。
- 没有待发布记录时 sleep 2 秒，见 `main.rs:81` 到 `86`。
- NATS 连接或发布失败时 sleep 5 秒，见 `main.rs:88` 到 `106`。
- strategy backtest outbox 发布 `{"kind":"strategy_backtest","run_id":...}`，见 `main.rs:153` 到 `178` 和 `nats.rs:16` 到 `29`。
- 发布成功后把 outbox 标记为 `published`，并把 run 的 `dispatch_status` 改成 `published`，见 `postgres/mod.rs:1143` 到 `1179`。

因此，create API 返回后到 worker 可消费之间存在最多约 2 秒的常规空闲扫描延迟。这是可优化点，但不应成为页面跳转门禁。

### Worker 执行流

`rearview-portfolio-worker` 当前是单 consumer 主循环：收到一条消息后完整处理、ack，然后再处理下一条，见 `engines/crates/rearview-portfolio-worker/src/main.rs:63` 到 `96`。

Strategy backtest 任务处理链路：

1. 读取 run；terminal 状态直接返回；否则 claim run，lease 900 秒，状态改为 `compiling_signals`，见 `main.rs:134` 到 `153` 和 `postgres/mod.rs:863` 到 `909`。
2. `materialize_strategy_backtest_signals()`：
   - 更新进度为 `compiling_signals`。
   - 再次查询交易日列表，用于 T+1 execution date 映射。
   - 按 `REARVIEW_CHUNK_SMALL_RANGE_TRADING_DAYS` 判断是否拆 chunk；默认 90 天以上按自然年 chunk。
   - 每个 chunk 都重新 `planner.compile()` 并 `query_screening_rows()`。
   - 从完整候选 rows 中筛 `is_buy_signal && signal_rank <= top_n`，映射到下一交易日，生成内存态 `BuySignalInput[]`。
   - 写 compiled SQL hash、required metrics/marts 和 signal summary。
   见 `main.rs:582` 到 `733`。
3. 查询所有入选证券在完整回测区间内的 price bars，并固定 LEFT JOIN trend indicator 多列，见 `main.rs:230` 到 `260` 和 `clickhouse/mod.rs:896` 到 `956`。
4. 在 Rust 内存中执行 `simulate_portfolio()`，见 `main.rs:261` 到 `287`。
5. 查询 benchmark returns 和 risk-free rates，计算 performance、closed trades 和 trade metrics，见 `main.rs:289` 到 `338`。
6. 写结果：
   - `fleur_portfolio`：target、order、trade、position_day、nav_daily、event。
   - PostgreSQL：strategy backtest metric config。
   - `fleur_calculation`：performance metric、metric statuses、closed trades、trade metrics。
   - `fleur_portfolio.portfolio_run_snapshot`。
   - PostgreSQL finalize run 为 `succeeded` 并写 `current_result_attempt_id`。
   见 `main.rs:340` 到 `390`、`clickhouse/mod.rs:328` 到 `368`、`432` 到 `467`。

历史验收报告 `docs/jobs/reports/2026-06-23-racingline-strategy-step5-backtest.md` 记录了真实 run 的规模：默认 1y run 产生 527 条 executable signals、112,772 条 price bars；2y rerun 产生 836 条 executable signals、344,232 条 price bars。报告没有记录各阶段耗时。

### Step 5 结果读取

进入 Step 5 后，页面还会发起结果查询：

- run status：`useStrategyBacktestQuery()` 每 1 秒 refetch 非 terminal run，见 `app/racingline/src/api/hooks.ts:273` 到 `293`。
- nav：`GET /nav` 读取 ClickHouse nav，再查询 benchmark returns 并归一化，见 `api/mod.rs:686` 到 `710`。
- rebalance records：`GET /rebalance-records` 读取 nav、trade counts、选中日期 trades、positions、closed trades，再查证券 display，见 `api/mod.rs:713` 到 `852`。
- performance：`GET /performance` 读取 performance metric，再读取 nav 计算 daily win rate，见 `api/mod.rs:988` 到 `1018`。

这些请求在当前主路径里发生在 worker terminal 之后；如果改为立即进入 Step 5，它们应按 `isResultReady` gate 留在 succeeded 后触发，或按页面区域懒加载。

## 等待时间构成

当前用户点击 Step 4「策略回测」后的等待可以拆成：

```text
T_total_before_step5 =
  T_create_sync
  + T_outbox_idle_or_publish
  + T_queue_wait_for_worker
  + T_worker_signal_materialization
  + T_worker_market_data
  + T_worker_simulation
  + T_worker_performance
  + T_worker_write_results
  + T_frontend_polling_granularity
  + 600ms cosmetic delay
```

其中 `T_worker_*` 是回测业务工作，可能天然较长；但它们不应该计入“进入 Step 5 页面”的等待。Step 5 页面的正确体验是显示 run 已创建、正在排队或正在计算，而不是用 modal 把用户锁在 Step 4。

## 实测基线（2026-06-25）

运行报告见 `docs/jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-baseline.md`。本次只采集指标，不调整代码。

### 受控样本

| Period | run id | HTTP create | outbox publish | worker pickup | worker elapsed | backend total |
|---|---|---:|---:|---:|---:|---:|
| 1y | `145ceb26-b7e3-4581-a7f4-1aa5769b0789` | 0.138225s | 0.807s | 0.096s | 9.791s | 10.694s |
| 2y | `a1d49988-c1d3-48a0-b1b1-f2fd9963052d` | 0.130941s | 1.454s | 0.014s | 22.170s | 23.638s |

当前前端在 Step 4 等待 worker terminal 后才 `setActiveStep("backtest")`，所以用户感知跳转等待至少包含 10.7s（1y）和 23.6s（2y）的后端完成时间，还会叠加 1s 轮询粒度和 600ms cosmetic delay。create API 已在约 0.13s 返回 `202 Accepted`；若 create accepted 后立即进入 Step 5，页面 shell 等待可以降到百毫秒量级。

### 瓶颈拆解

| Period | screening chunks | price bars | price bars 后无 ClickHouse 日志间隙 | worker elapsed |
|---|---:|---:|---:|---:|
| 1y | 2.316s | 0.673s | 5.787s | 9.791s |
| 2y | 4.040s | 1.375s | 15.392s | 22.170s |

ClickHouse heavy reads 不是全部后端耗时。2y 样本中 screening + price bars 约 5.4s，但 worker 总耗时 22.2s；price bars 完成到下一条 benchmark 查询开始之间有 15.4s 无 ClickHouse query log。下一步要在 Rust worker 内部给 simulation、performance preparation、JSON serialization 和 write preparation 增加阶段计时。

### ClickHouse 查询评估

`query_portfolio_price_bars()` 的 `EXPLAIN indexes = 1` 显示 quotes 和 trend 两侧都命中 `Min-Max`、`Partition` 和 `PrimaryKey`，过滤条件包含 `trade_date` 与证券集合。Per `schema-pk-filter-on-orderby`、`schema-pk-prioritize-filters` 和 `query-join-filter-before`，当前不是“JOIN 后再过滤”的形态，优先级高于 JOIN 结构重写的是减少列读取。

2y 样本 `FORMAT Null` 对照：

| Query | duration | read_rows | read_bytes | memory |
|---|---:|---:|---:|---:|
| current `LEFT JOIN`, all trend columns | 551ms | 4,976,096 | 514.89 MiB | 277.88 MiB |
| `LEFT ANY JOIN`, all trend columns | 501ms | 4,976,096 | 514.89 MiB | 265.57 MiB |
| current `LEFT JOIN`, OHLC + `price_ma_10` | 269ms | 4,976,096 | 151.86 MiB | 98.15 MiB |
| `LEFT ANY JOIN`, OHLC + `price_ma_10` | 266ms | 4,976,096 | 151.86 MiB | 90.68 MiB |

Per `query-join-use-any`，样本区间 `mart_stock_trend_indicator_daily` 没有重复 `(security_code, trade_date)`，可以评估 `LEFT ANY JOIN`；但实测主要收益来自动态投影趋势列。当前样本只需要 `price_ma_10` 时，读字节减少约 70.5%，内存减少约 64.7%。

### 结果读取与写入状态

Step 5 succeeded 后 wrapper HTTP 耗时为 `/nav` 0.113s、`/rebalance-records` 0.167s、`/performance` 0.097s。重复 nav 读取存在，但不是当前 10-24s 跳转等待的主因。

ClickHouse result tables active parts 当前最高为 `portfolio_nav_daily` 99 parts、`calc_portfolio_closed_trade` 92 parts、`portfolio_position_day` 91 parts。Per `insert-batch-size` 和 `insert-async-small-batches`，暂未到风险线，但多表小批量写入会随使用量累积；按 `insert-mutation-avoid-update`，append-only `result_attempt_id` 写入模式保留。

### 数据卫生观察

近两天成功 run 中 1y outbox publish p95 为 1.975s，符合当前 dispatcher 2s idle sleep 的预期。但存在历史 active run `88194a48-948e-4122-89ff-e0739df55dc6` 卡在 `running_clickhouse`，heartbeat 已过期，会污染 pickup/queue 聚合指标。后续性能面板需要把 stale active run 单独标记。

## 字段链路审计（2026-06-25）

本节对齐 `/strategies` Step 5 当前真实展示字段、前端实际请求、后端实际返回和 ClickHouse 实际读取字段。结论是：存在不影响结果语义但会放大轮询、首屏 payload、ClickHouse 读字节和 worker 内存的脏字段；它们不是当前 10-24s 页面跳转等待的首因，但应进入下一轮瘦身计划。

### Step 5 真实展示字段

当前 Step 5 `BacktestPanel` 只启用三类结果查询，见 `app/racingline/src/routes/strategy-page.tsx`：

| 区域 | Query | 当前真实使用字段 |
|---|---|---|
| run status / gate | `useStrategyBacktestQuery(activeRunId)` | `strategy_backtest_run_id`、`status`、`current_result_attempt_id`、`period_key`、`benchmark_security_code`、`rule_hash`、`execution_config_hash`、`error_message` |
| 净值走势 | `/nav` | 每点 `trade_date`、`strategy_nav`、`benchmark_nav`；latest 点用于右侧摘要 |
| 持仓记录 | `/rebalance-records` | 每个调仓日 `trade_date`、`position_count`、`buy_count`、`hold_count`、`sell_count`；选中日 rows 的 `direction`、`security_code`、`security_name`、`holding_days`、`change_pct`、`cost_price`、`current_price`、`contribution_pct` |
| 策略业绩 | `/performance` | `metric` 中 12 个值：`holding_period_return`、`annualized_return`、`annualized_volatility`、`max_drawdown`、`calmar_ratio`、`downside_deviation`、`sortino_ratio`、`sharpe_ratio`、`information_ratio`、`beta`、`alpha`、`treynor_ratio`；`daily_win_rate.value` |

`targets`、`orders`、`trades`、`positions`、`events`、`closed-trades` 和 `trade-metrics` 的 API/hook/type 已存在，但当前 `/strategies` Step 5 首屏没有调用这些 hooks。

父页面在 `canPublishPortfolio` 为 true 时还会启用 `publishNavQuery` 和 `publishPerformanceQuery`，用于组合发布确认区的业绩摘要。它们与 Step 5 使用同一 TanStack Query key，通常复用缓存；但这个路径也说明当前系统会为了 latest summary 读取完整 nav 曲线。

### 实际响应字段差异

使用 2y 样本 run `a1d49988-c1d3-48a0-b1b1-f2fd9963052d` 读取真实 JSON：

| Endpoint | 当前大小 | 页面必需字段的等价大小 | 差异判断 |
|---|---:|---:|---|
| `GET /strategy-backtests/{id}` | 8,720 bytes | 549 bytes | 返回整条 PG run snapshot；页面状态轮询只需要 status/gate/hash/error/result attempt |
| `GET /strategy-backtests/{id}/nav` | 64,593 bytes | 46,726 bytes（删除 `excess_return`） | `excess_return` 未被前端读取，前端用 latest strategy/benchmark 自己计算超额收益 |
| `GET /strategy-backtests/{id}/rebalance-records` | 50,677 bytes | 50,464 bytes（删除 rows 中 `quantity/reason`） | `quantity` 和 `reason` 当前 Step 5 表格不展示；主要体积来自 485 个日期轴 record 本身 |
| `GET /strategy-backtests/{id}/performance` | 3,810 bytes | 873 bytes（删除 `statuses`）/ 540 bytes（只保留 UI metric） | `statuses` 和 metric 元数据当前 Step 5 不展示 |

字段级定位：

- `StrategyBacktestRunResponse` flatten 了完整 `StrategyBacktestRunRecord`，后端 `get_strategy_backtest_run()` 从 PostgreSQL 读取 `rule_snapshot`、`execution_config`、`required_metrics`、`required_marts`、`data_preflight_snapshot`、`range_resolution_snapshot`、`summary`、`signal_summary`、`data_coverage_summary` 等字段。当前 Step 5 状态展示不读取这些字段。2y 样本中 `rule_snapshot` 约 3.8KB，`execution_config` 约 0.9KB。
- `/nav` 后端返回 `excess_return`，但前端 `mapStrategyBacktestNavPoints()` 只读取 `trade_date`、`strategy_nav` 和 `benchmark_nav`。删除 `excess_return` 可把 2y nav payload 从 64.6KB 降到 46.7KB。
- `/rebalance-records` 的 row response 包含 `quantity` 和 `reason`，但 Step 5 表格不展示。它们在当前样本只节省约 213 bytes，不是主收益。`records[].rows=[]` 对每个非选中日期都存在，但保留同构结构有利于前端简单映射；若要进一步瘦身，可改成 `records` 只放日期轴计数，另放 `selected_rows`。
- `/performance` 的 `statuses`、`portfolio_run_id`、`result_attempt_id`、`security_code`、`window_key`、`window_start`、`window_end`、`config_hash`、`metric_status`、`observation_count` 当前 Step 5 不展示。发布确认区会展示 `daily_win_rate.observation_count` 和 `winning_day_count`，但也不需要 `statuses`。

### ClickHouse 读取字段差异

页面本身不展示 price bars 或趋势指标，但 worker 必须读取行情来模拟成交、持仓和退出规则。真实必需字段分两类：

1. 组合模拟基础字段：`security_code`、`trade_date`、`open_price_backward_adj`、`close_price_backward_adj`。
2. 指标止损启用时的补充字段：`close_price_forward_adj` 和 execution config 中 `risk_exit_policy.indicator_metrics()` 返回的具体 trend metric。

当前 `query_portfolio_price_bars()` 固定读取 `close_price_forward_adj` 和 17 个趋势列，包括所有 MA、MA combo、EMA 和 `boll_lower_20_2`。但 worker 已经能从 `execution_config.risk_exit_policy.indicator_metrics()` 得到实际需要的指标集合，且 data coverage summary 也记录了该集合。2y 样本只需要 `price_ma_10` 时，动态投影可把 price bars 读字节从 514.89 MiB 降到 151.86 MiB，内存从 277.88 MiB 降到 98.15 MiB。

按 `clickhouse-best-practices`：

- Per `schema-pk-filter-on-orderby` 和 `schema-pk-prioritize-filters`，price bars 查询已使用 `trade_date` 和 `security_code` 过滤，主键条件生效。
- Per `query-join-filter-before`，`EXPLAIN indexes = 1` 已确认 quotes 和 trend 两侧都被 date/security 过滤。
- Per `query-join-use-any` 和 `query-join-choose-algorithm`，`mart_stock_trend_indicator_daily` 在样本区间 `(security_code, trade_date)` 无重复，可在动态投影之后再评估 `LEFT ANY JOIN`，但首要收益来自少读列。

### 脏字段清理计划

| 优先级 | 清理项 | 预期收益 | 风险/约束 |
|---|---|---|---|
| P0 | 新增或改造 run status 轻量响应，只返回 Step 5 gate/status 需要字段；create 可保留 full run 或返回 accepted view | 轮询 payload 从 8.7KB 降到约 0.55KB，减少每秒状态轮询和 Step 4 handoff 噪声 | 发布、调试和幂等冲突仍可能需要 full snapshot；建议用独立 status view，不直接删 full record |
| P1 | `query_portfolio_price_bars()` 按 `indicator_metrics()` 动态投影趋势列；未启用指标止损时不 JOIN trend | 2y 样本 price bars 读字节下降约 70.5%，内存下降约 64.7% | `PriceBar` 结构可保留可选字段，但 SQL 只投影必要列；需补 worker 同结果 hash/summary 对比 |
| P2 | `/nav` 删除 `excess_return`，或把 excess 只放 latest summary | 2y nav payload 从 64.6KB 降到 46.7KB | 若后续页面直接展示逐日 excess curve，需要重新引入或前端计算 |
| P2 | `/performance` 增加 UI view，不返回 `statuses` 和 metric 元数据 | 2y performance payload 从 3.8KB 降到 0.54-0.87KB | 调试/诊断页面可能仍需 statuses；保留详细 endpoint 或 query param |
| P3 | `/rebalance-records` 拆成日期轴 counts + selected rows，删除当前 Step 5 未展示的 `quantity/reason` | 当前样本节省小；结构更清晰，便于懒加载选中日 rows | `strategy-detail-page` 使用 `row.reason`，需要区分 Step 5 compact view 和详情页 view |
| P3 | 父页面发布确认区只取 latest nav summary，而不是完整 nav 曲线 | 可减少非 Step 5 场景中为了最新点读取完整曲线 | 与 Step 5 同 key 缓存时收益有限；适合配合 overview/status endpoint |

结论：字段脏数据确实存在，但它们对“点击后卡 10-24s 才进入 Step 5”的直接贡献小于 terminal wait。字段瘦身的正确顺序是：先解耦页面跳转，再清理 run status 轮询和 worker price bars 动态投影，最后收敛 result wrapper 的 UI view。

## 第一性原理判断

一次 Step 5 回测只有两个不可混淆的问题：

1. 用户是否已经创建了一次可追踪、可重试、可解释的 backtest run？
2. 这次 run 的结果是否已经可展示？

第一个问题只要求 `strategy_backtest_run_id`、冻结后的 rule/config/range/benchmark/hash 和状态可查。它在 create API 返回 `202 Accepted` 时已经成立。

第二个问题才需要 worker 完成、ClickHouse 写入和 `current_result_attempt_id`。它应由 Step 5 页面内的局部 loading、progress、empty、failed 和 result view 承担。

因此，Step 4 到 Step 5 的直线最近路径是：

```text
create run accepted -> navigate/render Step 5 status shell -> wait result inside Step 5
```

而不是：

```text
create run accepted -> wait queue -> wait worker -> wait result write -> navigate/render Step 5
```

## 可删减和可合并流程

### 1. 删除 Step 4 里的 terminal wait

当前 `openBacktest()` 的 `waitForBacktestTerminalRun()` 和 `waitForBacktestCompletedMessage()` 是首要冗余。它们把异步 job 又同步回了页面跳转。

建议：

- create API 返回后立即缓存 run、关闭 launch dialog、`setActiveStep("backtest")`。
- Step 5 继续使用现有 `useStrategyBacktestQuery(activeRunId)` 的 `refetchInterval` 轮询。
- 删除或停用 `waitForBacktestTerminalRun()`、`waitForBacktestPollingInterval()` 和 600ms cosmetic delay。

预期收益：Step 4 到 Step 5 的用户等待不再包含 worker 全量计算时间。

### 2. 收敛两套 create/polling 状态

当前 Step 4 `initialBacktestMutation` 和 Step 5 `createBacktestMutation` 同时存在，且 Step 5 有自动提交 effect。建议选择一个所有者：

- 保守方案：Step 4 按钮负责创建第一条 run；Step 5 只负责展示和手动重新回测。
- Step 5 的 `autoSubmittedBacktestSignatureRef` 自动提交逻辑删除；只有用户点击「开始回测/重新回测」才创建新 run。
- `initialBacktestRun` 改名或收敛为 `activeBacktestRun` / `activeRunId`，避免“initial”在 rerun 后继续承载当前 run。

预期收益：减少重复 mutation、重复 query cache 写入和状态竞态。

### 3. 把 blocking modal 降级为 Step 5 页面状态

当前不可关闭 dialog 的文案是 `querying/queued/running/completed`，这些状态天然属于 Step 5 内容区。建议删除 blocking dialog，改为：

- Step 5 顶部 status alert 展示 queued/running/progress/error。
- 净值、持仓和业绩区域各自显示 skeleton/empty/result。
- Step 4 create 失败时仍留在 Step 4 显示 error；create 成功后无论 queued/running 都进入 Step 5。

预期收益：用户可看到配置、run id、阶段和失败原因，不再被“准备中”遮罩锁住。

### 4. 避免 create 后再由 Step 5 自动重建同一 run

如果 Step 5 自动提交逻辑保留，必须证明它不会在 `initialRun` 尚未同步到 `activeRunId` 前重复创建。更简单的减法是移除自动提交，只保留显式按钮。当前代码中 `client_request_id` 每次 `buildStrategyBacktestCreateRequest()` 都生成新 id，见 `strategy-page.tsx:354` 到 `391`，因此不能依赖 client id 自动去重。

### 5. 合并重复的状态轮询机制

当前有两种状态获取方式：

- `openBacktest()` 手动 `getStrategyBacktest()` loop。
- `useStrategyBacktestQuery()` React Query refetch interval。

建议只保留 React Query，所有状态展示从 query cache 读取。Step 4 create 成功后只 `queryClient.setQueryData()` 一次，再进入 Step 5。

### 6. 重审 create 阶段 risk-free preflight

Create API 同步查询 risk-free rates 只用于 `data_preflight_snapshot`，worker 后续计算 performance 时还会再次查询 risk-free rates。若风险免费率缺失不阻止 create，且最终权威仍在 worker，则 create 阶段的 risk-free 查询可以移到 worker coverage，或变为可选后台预检。

保留 create 阶段必须同步完成的工作：

- period/benchmark allowlist validation。
- range resolution。
- rule/config canonical validate 与 hash check。
- PG run + outbox transaction。

风险：删除 create preflight 前要确认当前 UI、报告和告警没有依赖 `risk_free_return_count` 作为 create 成败判断。

### 7. 复用或缓存动态区间解析

Step 5 options API 和 create API 都会解析交易日与 benchmark 覆盖：

- options：`GET /strategy-backtests/options` 调 `resolve_strategy_backtest_range()`。
- create：`POST /strategy-backtests` 再调一次 `resolve_strategy_backtest_range()`。

服务端必须在 create 时重新校验权威区间，但前端可在 Step 4 后台预取 options，让用户看到待执行区间；create request 可带 `range_hint` 辅助一致性检查。后端仍以自己解析为准。

## 性能优化空间

### 已符合的 ClickHouse 规则

按 `clickhouse-best-practices` 已核对以下规则：

- Per `schema-pk-prioritize-filters` 和 `schema-pk-filter-on-orderby`，portfolio result tables 的主要读取路径按 `portfolio_run_id + result_attempt_id` 过滤，现有 `ORDER BY (portfolio_run_id, result_attempt_id, date...)` 与 wrapper API 的访问模式基本一致，见 `portfolio_schema.rs:36` 到 `199` 和 `calculation_schema.rs:8` 到 `123`。
- Per `schema-partition-low-cardinality`，portfolio fact 表按 `toYYYYMM(date)` 分区，属于低基数时间分区，当前不需要改成按 run id 分区。
- Per `insert-mutation-avoid-update`，ClickHouse 结果用 append-only `result_attempt_id` 写入，PostgreSQL `current_result_attempt_id` 指向当前 attempt，没有用 ClickHouse mutation 更新历史结果。

### 已评估的 ClickHouse 优化方向

1. `query_portfolio_price_bars()` 当前固定查询大量趋势指标列，并 `LEFT JOIN mart_stock_trend_indicator_daily`，见 `clickhouse/mod.rs:923` 到 `953`。2y 隔离测试显示，只保留 OHLC 和 `price_ma_10` 可把 read_bytes 从 514.89 MiB 降到 151.86 MiB，memory 从 277.88 MiB 降到 98.15 MiB。应按 execution_config 的 `indicator_metrics()` 动态投影必需列。
2. Per `query-join-filter-before`，已用 `EXPLAIN indexes = 1` 确认 price bars 查询能把 date/security filters 作用到 quotes 和 trend 两侧；暂不优先改成双侧过滤子查询。
3. Per `query-join-use-any`，样本区间 trend 表 `(security_code, trade_date)` 无重复，`LEFT ANY JOIN` 语义可评估；单独收益小于动态投影，可作为后续小幅优化。
4. `query_trade_dates()` 在 create 和 worker 各查一次。实测 create/options trade dates 为 17-18ms，worker trade dates 为 12-17ms；当前不是主瓶颈，不建议为了去重增加 PG JSON 体积。
5. Per `insert-batch-size`，当前每次 worker 完成会对多个 ClickHouse 表分别 `INSERT JSONEachRow`，且 snapshot 是单行 insert，见 `clickhouse/mod.rs:328` 到 `368` 和 `432` 到 `495`。当前最高 active parts 约 99，暂未到风险线；高频运行时按 `insert-async-small-batches` 再评估 async insert、跨 run 合批或延迟写低优先级明细。

### Worker 计算优化

1. Worker 当前单进程主循环串行处理消息，见 `rearview-portfolio-worker/src/main.rs:63` 到 `96`。如果多个用户或多个 rerun 同时发生，后续任务会排队等待前一个完整回测。可以通过多个 worker 进程或 worker 内部并发处理提高吞吐，但必须配合 ClickHouse 并发上限。
2. Signal materialization 按 chunk 串行执行 planner compile 和 screening query，见 `main.rs:638` 到 `695`。3y 通常只有自然年级别少量 chunk，是否并行取决于 ClickHouse 压力；先记录每个 chunk elapsed 和 row count。
3. `query_portfolio_price_bars()` 一次拉取所有入选证券完整区间价格。对 2y run 验收已有 344,232 price bars。若 price bar 查询是 p95 主因，优化顺序应是减少列、确认索引、再考虑按 security/date chunk 分批并发。
4. Performance 需要 benchmark 和 risk-free rates；create 阶段已经查过 risk-free count，但 worker 仍要权威 rows。可以删除 create preflight 的重复查询，而不是跳过 worker 查询。

### Result wrapper 优化

Step 5 succeeded 后，当前首屏会读取 nav、rebalance-records 和 performance：

- `/nav` 查 nav + benchmark returns。
- `/rebalance-records` 再查 nav + trade counts + selected date trades/positions/closed trades + display。
- `/performance` 查 performance metric + 再查 nav 计算 daily win rate。

首屏重复读取 nav 三次。可选减法：

1. 新增 `GET /rearview/strategy-backtests/{id}/overview`，一次返回 Step 5 首屏需要的 status、nav、performance、rebalance rail 和 selected date rows。
2. 或保持 endpoint 分离，但前端先展示 nav/performance，rebalance records 在用户看到 Step 5 后懒加载。
3. daily win rate 可在 worker finalize summary 中写入，避免每次 performance wrapper 重读 nav；但要确认它属于稳定绩效口径。

## 分阶段建议

### Phase 0: 建立耗时基线

状态：已完成第一轮基线，见 `docs/jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-baseline.md`。后续仍需给 Rust worker 内部补阶段计时。

目标：把“很慢”拆成可量化阶段。

记录指标：

- 前端：click Step 4 button、create request start/end、Step 5 active step set、run terminal、nav/rebalance/performance loaded。
- Rearview create：range resolution、validate/hash、risk-free preflight、PG transaction elapsed。
- Outbox：outbox created_at -> published_at；run dispatch_status pending -> published。
- Worker：claim -> signal chunks -> price bars -> simulation -> performance -> write -> finalize，每阶段 elapsed、row count、query_id。
- ClickHouse：用现有 query_id 查 `system.query_log`，记录 read_rows、read_bytes、memory_usage、duration_ms。

完成标准：

- 已能区分页面等待策略、create 同步预检、outbox 空闲扫描、worker 队列、ClickHouse screening、price bars、写入和 result wrapper。
- 尚需补 worker 内部 simulation、performance preparation、serialization 和 write preparation 的细分计时。

### Phase 1: 立即降低用户感知延时

目标：不改变回测结果语义，先把 Step 5 页面跳转从 worker 终态中解耦。

实测依据：1y/2y create API 为 0.138s/0.131s，backend total 为 10.694s/23.638s。该阶段是最大用户体验收益点。

建议改动：

1. `openBacktest()` create 成功后立即 `setActiveStep("backtest")`。
2. 删除手动 terminal polling 和 600ms wait。
3. Step 5 顶部显示 run queued/running/progress；结果区域用现有 `isResultReady` gate。
4. 保留 create 失败时的 Step 4 error。
5. 若继续 1s 轮询 run 状态，优先使用轻量 status view，避免每次拉取完整 `rule_snapshot` 和 `execution_config`。

完成标准：

- 点击「策略回测」后，页面在 create API 返回后进入 Step 5。
- Worker 仍在后台运行；Step 5 status 自动刷新。
- 回测结果仍只在 `status = succeeded && current_result_attempt_id` 后展示。
- 状态轮询 response 不再返回当前页面不用的大型 snapshot 字段。

### Phase 2: 前端状态减法

目标：让 run 创建和轮询只有一个清晰 owner。

建议改动：

1. 移除 Step 5 auto-submit effect。
2. 统一 `initialBacktestRun` / `activeRunId` 命名和状态来源。
3. 保留 Step 5 手动「重新回测」作为唯一 rerun 入口。
4. launch dialog 改为 Step 5 内部 status alert 或删除。

完成标准：

- 不存在两套 create mutation 同时可能提交同一配置。
- 不存在手写 `getStrategyBacktest()` loop 和 React Query refetch interval 并存。

### Phase 3: Create/queue 延时收敛

目标：让 `POST /strategy-backtests` 真正接近“校验 + 入队”。

实测依据：create API 当前约 0.13s，不是主瓶颈；outbox publish 为 0.807s/1.454s，近两天 1y p95 为 1.975s，符合当前 2s idle sleep。

候选项：

1. 删除或后台化 create 阶段 risk-free preflight。
2. Step 4/5 预取 options，只作为 UI 展示和 `range_hint`，create 仍服务端权威解析。
3. outbox dispatcher 在 create 后被唤醒，或把 idle sleep 从固定 2 秒改为更短/可配置。保留 outbox 事务边界，不直接绕过 outbox 可靠性。

完成标准：

- create API p95 明确下降，且失败语义不回退。
- outbox pending 到 published 的常规延迟可观测、可解释。

### Phase 4: Worker 与 ClickHouse 查询瘦身

目标：缩短真实计算耗时，不牺牲结果可解释性。

候选项：

1. `query_portfolio_price_bars()` 按启用的 indicator stop-loss metric 动态投影趋势列；未启用指标止损时不 JOIN trend，这是已测到的 ClickHouse 层最高收益项。
2. 按 `query-join-use-any` 评估 `LEFT ANY JOIN`，但优先级低于动态投影。
3. 为 worker 每个内部阶段记录 elapsed、row count 和 query_id；尤其是 price bars 后到 benchmark 查询前的 simulation/performance gap。
4. 对 chunk screening 做并发实验，先限制最大并发，避免压垮 ClickHouse。
5. 监控 ClickHouse parts；若小 part 增长明显，再评估 async insert 或低优先级明细延迟写。

完成标准：

- 优化前后有同一策略、同一 period/benchmark 的 p50/p95 对比。
- 输出 row count、hash、summary 与优化前一致，或差异有明确业务原因。

### Phase 5: Step 5 首屏结果读取瘦身

目标：结果已经 succeeded 后，Step 5 首屏少发重复查询。

候选项：

1. 首屏懒加载 rebalance selected rows，只先展示 nav 和 status；或把 `records` 日期轴与 `selected_rows` 拆开。
2. 或新增 overview endpoint 合并 nav latest、performance UI metrics、rebalance rail 和 selected rows。
3. `/nav` 删除当前前端未使用的 `excess_return`，或只在 latest summary 中返回。
4. `/performance` 增加 UI view，不返回 `statuses` 和 metric 元数据；详细诊断保留原 endpoint 或 query param。
5. 把 daily win rate 放入 worker summary 或 performance result，避免 performance wrapper 重读 nav。

完成标准：

- Step 5 succeeded 后首屏请求数量、重复 nav 读取次数和 time-to-chart 可量化下降。
- 每个 wrapper endpoint 的 response 字段有对应 UI 消费方；诊断字段进入明确的 detail/debug view。

## 非目标

1. 本 RFC 不要求删除 NATS、outbox 或 worker；它们是长耗时回测的可靠异步边界。
2. 本 RFC 不要求把回测改成同步 HTTP。
3. 本 RFC 不改变 Step 1/2/3/4 的业务语义、hash、TopN、max positions、费用、滑点或止损规则。
4. RFC 撰写阶段不调整生产代码，只记录问题定位和后续减法方向；后续实施见 [Plan 0056](../plans/archive/0056-racingline-step4-step5-backtest-latency-optimization-plan.md) 和 [验收报告](../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md)。

## 实施状态（2026-06-25）

Plan 0056 已完成实施并通过 1y/2y dev smoke：

- Step 4 `openBacktest()` 已删除 terminal wait 和 600ms cosmetic delay，create 返回 `202 Accepted` 后立即进入 Step 5。
- Step 5 active run owner 已收敛到父页面 `activeBacktestRun`，Step 5 只保留显式重新回测入口。
- Rearview 新增 status view；Step 5 状态轮询不再拉取完整 run snapshot。
- `/nav`、`/performance`、`/rebalance-records` 新增 `view=ui` compact response，full/detail 字段保留。
- `query_portfolio_price_bars()` 已按 indicator stop-loss metrics 动态投影趋势列；未启用 indicator metrics 时不 JOIN trend 表。
- Worker summary 已写入 `worker_timing` 阶段耗时。
- Outbox dispatcher 已支持 create 后进程内 notify 唤醒，并记录 publish elapsed；stale active run 有只读诊断 endpoint。

验收样本中 create HTTP 为 0.105-0.113s，outbox publish 为 0.031-0.039s，status payload 为 578B，2y price bars read_bytes 为 151.86 MiB。完整数据见 [2026-06-25 优化验收报告](../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md)。

## 结论

当前严重用户体验问题的直接原因是前端 `openBacktest()` 在 Step 4 中等待完整 worker terminal status 后才进入 Step 5。第一性原理下，Step 5 是异步 run 的状态和结果页面，不是 worker 完成后的静态结果页。优先级最高的减法是删除 Step 4 的 terminal wait，把 create accepted 作为进入 Step 5 的门槛。

后端性能治理应在观测基线之后做：create 阶段可去掉重复 preflight，outbox 可减少空闲扫描延迟，worker 可按真实耗时收窄 price bars 列、验证 JOIN 过滤、控制并发和监控 ClickHouse 小 part。字段审计确认了额外脏字段：run status 轮询不需要完整 snapshot，nav 不需要逐点 `excess_return`，performance 首屏不需要 statuses，rebalance 首屏可以拆日期轴和选中日 rows。这样既不牺牲异步可靠性，也能把用户路径从“等待全部计算”收敛为“立即进入进度可见的回测页”，并把后续结果读取和 worker 数据面继续瘦身。
