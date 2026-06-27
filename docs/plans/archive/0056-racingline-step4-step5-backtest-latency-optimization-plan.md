# Plan 0056: Racingline Step 4 到 Step 5 回测延时优化实施计划

日期：2026-06-25

状态：Completed

## 背景

[RFC 0031](../../RFC/0031-racingline-step4-step5-backtest-latency-slimming.md) 已确认 `/strategies` 从 Step 4「模拟建仓」点击「策略回测」到 Step 5「策略回测」的当前路径存在设计层面的等待错误：前端 `openBacktest()` 在 Step 4 中创建 backtest run 后，手动轮询 `GET /rearview/strategy-backtests/{id}` 等到 worker terminal，再额外等待 600ms，最后才 `setActiveStep("backtest")`。

实测报告 [2026-06-25-racingline-step4-step5-backtest-latency-baseline](../../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-baseline.md) 的受控样本显示：

| Period | HTTP create | outbox publish | worker elapsed | backend total |
|---|---:|---:|---:|---:|
| 1y | 0.138s | 0.807s | 9.791s | 10.694s |
| 2y | 0.131s | 1.454s | 22.170s | 23.638s |

第一性原理下，Step 5 页面应该承接一个已创建、可追踪、可重试、可解释的异步 backtest run，并在页面内展示 queued/running/succeeded/failed 状态。进入 Step 5 不应等待 worker 完成；只有净值、调仓和绩效区域需要等待结果。

RFC 0031 还确认了额外字段和数据面问题：

- run status 轮询当前返回完整 PostgreSQL run snapshot，2y 样本 8.7KB，当前页面只需要约 0.55KB 的 status/gate 字段。
- `/nav` 返回前端未使用的 `excess_return`，2y 样本可从 64.6KB 降到 46.7KB。
- `/performance` 返回当前 Step 5 不展示的 `statuses` 和 metric 元数据，2y 样本可从 3.8KB 降到 0.54-0.87KB。
- worker `query_portfolio_price_bars()` 固定读取大量趋势列；2y 样本只需要 `price_ma_10` 时，动态投影可把 read bytes 从 514.89 MiB 降到 151.86 MiB，memory 从 277.88 MiB 降到 98.15 MiB。
- price bars 完成到下一条 benchmark 查询开始之间存在 5.8s（1y）/15.4s（2y）的无 ClickHouse 日志区间，需增加 worker 内部阶段计时。

本计划把 RFC 的减法方案拆成实施阶段。目标是先把用户感知跳转压到 1s 内，再逐步缩短结果 ready 时间和首屏 payload。

## 目标

- Step 4 点击「策略回测」后，create API 返回 `202 Accepted` 即进入 Step 5 状态页，不再等待 worker terminal。
- Step 5 页面内展示 queued/running/progress/failed/succeeded 状态；结果区域只在 `status = succeeded && current_result_attempt_id` 后读取。
- Step 4/Step 5 的首跑创建和状态轮询只有一个清晰 owner，不再保留两套自动首跑 create 路径、手写 terminal polling 和自动重复提交路径。
- run status 轮询使用轻量 view，不再每秒拉取完整 `rule_snapshot`、`execution_config`、summary 和 coverage snapshot。
- Step 5 result wrapper 返回字段与当前 UI 消费字段对齐，诊断字段进入明确的 detail/debug view。
- worker price bars 查询按 execution config 动态投影趋势列，未启用指标止损时不 JOIN trend 表。
- worker 增加阶段计时，能区分 screening、price bars、simulation、performance preparation、serialization、write preparation 和 ClickHouse writes。
- outbox publish 延迟可观测，并在不破坏 outbox 可靠性的前提下降低常规 idle 延迟。
- 每阶段都有可复现实测报告，至少覆盖 1y 和 2y 同策略样本。

## 非目标

- 不删除 PostgreSQL outbox、NATS JetStream 或 `rearview-portfolio-worker`；它们仍是长耗时回测的可靠异步边界。
- 不把回测改成同步 HTTP。
- 不改变 Step 1/2/3/4 的业务语义、RuleVersionSpec、hash、TopN、max positions、费用、滑点、止损或绩效口径。
- 不改变 ClickHouse 结果事实表的 append-only `result_attempt_id` 模式。
- 不在未建立 worker 内部阶段计时前承诺 simulation gap 的具体优化秒数。
- 不把调试/诊断信息直接删除；需要的详细字段通过 full/detail/debug endpoint 或 query param 保留。
- 不在本计划中处理鉴权、用户隔离、多租户限流和权限模型。

## 关联文档

| 文档 | 用途 |
|---|---|
| [RFC 0031](../../RFC/0031-racingline-step4-step5-backtest-latency-slimming.md) | 设计依据、执行流、字段审计和阶段建议 |
| [性能基线报告](../../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-baseline.md) | 1y/2y 受控样本、ClickHouse query log、parts 和 wrapper 响应实测 |
| [优化验收报告](../../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md) | 1y/2y live smoke、payload、worker timing、ClickHouse query log、outbox publish 和 stale active 诊断验收 |
| [Racingline 系统地图](../../architecture/racingline.md) | 前端职责、运行入口和质量门禁 |
| [Rearview 系统地图](../../architecture/rearview.md) | Rearview API、worker、ClickHouse 和 NATS 边界 |
| [Plan 0051](0051-racingline-strategy-backtest-step5-implementation-plan.md) | Step 5 原始实现计划 |
| [Step 5 验收报告](../../jobs/reports/2026-06-23-racingline-strategy-step5-backtest.md) | Step 5 原始 live smoke 和 rerun 验收 |

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| Step 4 handoff | [strategy-page.tsx](../../../app/racingline/src/routes/strategy-page.tsx) 的 `openBacktest()` create 成功后调用 `waitForBacktestTerminalRun()`，等 terminal 后再 `setActiveStep("backtest")` |
| 前端轮询 | [hooks.ts](../../../app/racingline/src/api/hooks.ts) 的 `useStrategyBacktestQuery()` 已支持非 terminal run 每 1s refetch |
| Step 5 create | `BacktestPanel` 内部还有 `useStrategyBacktestCreateMutation()` 和自动提交 effect，主路径由 Step 4 传 `initialRun` 时通常不会二次创建 |
| Query cache | [queryKeys.ts](../../../app/racingline/src/api/queryKeys.ts) 当前 `queryKeys.strategyBacktest(id)` 用于完整 `StrategyBacktestRunRecord`；Phase 3 status view 不能复用同一 key 写入子集类型 |
| Run response | [postgres/mod.rs](../../../engines/crates/rearview-core/src/postgres/mod.rs) `get_strategy_backtest_run()` 读取完整 run snapshot；[api/mod.rs](../../../engines/crates/rearview-core/src/api/mod.rs) `StrategyBacktestRunResponse` flatten 完整 record |
| Result wrappers | `/nav`、`/rebalance-records`、`/performance` 在 [api/mod.rs](../../../engines/crates/rearview-core/src/api/mod.rs) 中分别读取 ClickHouse nav、trade/position/closed trades 和 performance metric |
| 共享类型 | [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx) 使用 rebalance `row.reason`；`StrategyPortfolioPerformanceView` 复用 `StrategyBacktestPerformanceView`，compact view 必须与 full/detail 契约分离 |
| Price bars | [clickhouse/mod.rs](../../../engines/crates/rearview-core/src/clickhouse/mod.rs) `query_portfolio_price_bars()` 固定读取 OHLC、`close_price_forward_adj` 和 17 个趋势列 |
| Worker | [rearview-portfolio-worker/src/main.rs](../../../engines/crates/rearview-portfolio-worker/src/main.rs) 单 consumer 主循环，strategy backtest 在同一任务内完成 signal materialization、price bars、simulation、performance 和写入 |
| Outbox | [rearview-server/src/main.rs](../../../engines/crates/rearview-server/src/main.rs) 进程内 dispatcher 没有 pending 任务时 sleep 2s |
| 异常数据 | 存在历史 stale active run `88194a48-948e-4122-89ff-e0739df55dc6`，会污染 queue/pickup 聚合指标 |

## 目标调用图

短期目标（Phase 1 可独立交付，不等待 Phase 3 后端契约）：

```text
Step 4 click
  -> POST /rearview/strategy-backtests
  -> 202 Accepted + current full run response
  -> setActiveStep("backtest")

Step 5
  -> existing GET /rearview/strategy-backtests/{id} every 1s while non-terminal
  -> if succeeded: load nav / rebalance / performance
```

中期目标：

```text
Step 4 click
  -> POST /rearview/strategy-backtests
  -> 202 Accepted + accepted view
  -> Step 5 status shell

Step 5 status shell
  -> lightweight status polling
  -> optional overview endpoint after succeeded

Worker
  -> phase timing
  -> dynamic price bar projection
  -> same output hash / summary semantics
```

## 执行依赖和契约边界

- Phase 1 只改前端 handoff，不要求后端新增 status endpoint，也不要求 create response 瘦身；这样最大 UX 收益可以先发布和回滚。
- Phase 2 收敛首跑 owner 后，再做 Phase 3 status view；否则 status 子集、full run 和 rerun 状态容易混在同一 cache/state 模型里。
- Phase 3 的 compact/status/detail 都必须是显式契约：status view 使用独立类型和独立 query key，例如 `strategyBacktestStatus(id)`；full run 继续使用 `strategyBacktest(id)` 或明确命名的 full key。禁止把 status 子集写进 full run cache。
- 对共享 endpoint 和共享 TS 类型，默认新增 `view=ui/status/compact` 或独立 endpoint。只有 `rg` 确认没有其他消费方、且类型/单测同步更新后，才允许从既有 full response 删除字段。
- Phase 4 拆成 4a worker timing 和 4b price bars 动态投影。4a 可以先发；4b 的收益声明必须用 4a timing、ClickHouse query_log 和同配置结果一致性一起证明。
- 验收指标按阶段归属，不把 Phase 5 的 outbox p95 或 Phase 4 的 price bars 目标作为 Phase 1 发布门槛。

## 实施阶段

### Phase 0：实施前基线复核与测试夹具

目标：避免在旧异常 run 或缓存数据上做优化判断，先把本计划验收样本固定下来。

实施项：

1. 固定两个代表样本：
   - 1y 样本：沿用 RFC 0031 中 `perf-rfc0031-1y-*` 的请求形态。
   - 2y 样本：沿用 RFC 0031 中 `perf-rfc0031-2y-*` 的请求形态。
   - 保留源 payload run id 和新 run id 到验收报告。
2. 增加开发验收脚本或手动 runbook：
   - 记录 create HTTP time、Step 5 shell render time、run terminal time、nav/performance/rebalance loaded time。
   - 记录 PostgreSQL `created_at -> outbox.published_at -> started_at -> completed_at`。
   - 记录 ClickHouse query_id、duration、read_rows、read_bytes、memory。
3. 明确 stale active run 处理策略：
   - 本计划不直接修改历史 run 状态。
   - 性能报告聚合时排除 heartbeat 过期的 active run，或单独列为数据卫生问题。
4. 为后续阶段准备测试断言清单：
   - 前端 handoff 单测。
   - API response 字段 contract 单测。
   - worker dynamic projection 单测。
   - live smoke 报告模板。

测试策略：

- 本阶段以文档和运行报告为主，不要求修改生产代码。
- 如果补自动脚本，脚本必须只读查询或创建新的 perf run，不修改历史 run。

完成标准：

- 新增或更新 `docs/jobs/reports/YYYY-MM-DD-racingline-step4-step5-backtest-latency-before.md`。
- 报告包含 1y/2y 样本、命令、结果和环境。
- 后续阶段可以用同一策略、同一 period/benchmark 做前后对比。

### Phase 1：Step 4 handoff 立即进入 Step 5

目标：不改变回测结果语义，先把用户感知页面跳转从 worker terminal 中解耦。

实施项：

1. 修改 `openBacktest()`：
   - create 成功后立即缓存 run。
   - 使用当前 create 返回的完整 run response 写入 full run cache；本阶段不依赖 `/status` 或 compact response。
   - 关闭 launch dialog 或不再打开 blocking dialog。
   - 立即 `setActiveStep("backtest")`。
   - 不再调用 `waitForBacktestTerminalRun()`。
   - 删除或停用 `waitForBacktestCompletedMessage()` 的 600ms cosmetic delay。
2. Step 5 状态展示：
   - `BacktestPanel` 使用传入的 active run id。
   - `useStrategyBacktestQuery(activeRunId)` 继续用当前 full run endpoint 承担非 terminal 轮询。
   - 顶部 status alert 或 status shell 显示 queued/running/progress/failed。
   - nav/rebalance/performance 继续受 `isResultReady` gate 控制。
3. 错误语义：
   - create API 失败仍停留 Step 4 并展示 error。
   - create 成功后 worker 失败在 Step 5 显示 failed status 和 `error_message`。
4. 移除旧 blocking modal 状态：
   - `BacktestLaunchPhase`、launch dialog 文案和 terminal wait 逻辑不再参与主路径。
   - 若保留 transitional UI，必须不阻塞进入 Step 5。

测试策略：

- 前端单测覆盖 create promise resolve 后立即进入 Step 5，不等待 terminal polling。
- 前端单测覆盖 create reject 时不进入 Step 5。
- 前端单测覆盖 create resolve with `queued/running` 时 Step 5 显示状态，结果区域不请求 result wrappers。
- 浏览器 smoke 记录 click 到 Step 5 shell render 时间。

完成标准：

- 1y/2y 受控样本点击到 Step 5 shell render p50 <= 1s。
- 页面跳转不再包含 worker elapsed、outbox publish 和前端 terminal polling。
- run terminal 后 Step 5 自动展示结果。
- 回测结果 summary、nav、rebalance、performance 与优化前同配置一致。
- 不新增后端契约也可完成本阶段；如同时新增 status view，必须满足 Phase 3 的 cache/type 分离约束。

### Phase 2：前端状态 owner 收敛

目标：让 run 创建、轮询和 rerun 只有一个清晰 owner，降低重复 run 和竞态风险。

实施项：

1. 取消 Step 5 自动提交 effect：
   - 删除或停用 `autoSubmittedBacktestSignatureRef` 自动 `runBacktest()` 路径。
   - Step 5 内部只在用户显式点击「重新回测」时创建新 run。
2. 统一 active run 状态：
   - 将 `initialBacktestRun` / `initialRun` 语义收敛为 `activeBacktestRun` 或 `activeRunId`。
   - 父页面或一个 colocated run controller 是 active run 的唯一事实源；`BacktestPanel` 不再把 prop、mutation data 和内部 state 组成长期三路 fallback。
   - Step 4 create 成功后只设置一次 active run。
   - Step 5 rerun 成功后替换 active run。
3. 查询缓存规则：
   - create response 写入 `queryKeys.strategyBacktest(run_id)`。
   - 状态更新全部来自 React Query，不保留手写 `getStrategyBacktest()` loop。
   - Phase 3 之前该 key 只存 full run；Phase 3 后 status polling 必须切到 status 专用 key。
4. 按配置变更处理旧结果：
   - `hasStrategyBacktestConfigChanged()` 继续用于提示旧结果不匹配。
   - 配置变更后不自动 rerun，只提示用户显式重新回测。

测试策略：

- 前端单测覆盖 Step 5 挂载时不会自动创建 run。
- 前端单测覆盖用户点击重新回测时创建新 run，并替换 active run id。
- 前端单测覆盖配置变更只显示 pending config 提示，不自动提交。
- 使用 mock queryClient 验证 create response 的 cache key。

完成标准：

- 同一配置从 Step 4 到 Step 5 不存在两套自动首跑 create 路径同时可提交。
- 不存在手写 terminal polling 与 React Query interval 并存。
- `client_request_id` 新生成语义不会导致隐藏重复 run。

### Phase 3：轻量 status view 与 payload 瘦身

目标：减少 Step 5 非 terminal 轮询和 succeeded 首屏 wrapper 的无效字段。

实施项：

1. 新增 run status response：
   - 后端新增 `GET /rearview/strategy-backtests/{id}/status`，或给现有 `GET /{id}` 增加 `view=status` query param。
   - 返回字段只包含：`strategy_backtest_run_id`、`status`、`dispatch_status`、`progress`、`error_type`、`error_message`、`period_key`、`benchmark_security_code`、`start_date`、`end_date`、`rule_hash`、`execution_config_hash`、`current_result_attempt_id`。
   - 保留 full run endpoint 作为诊断和兼容接口。
   - 如果采用 `view=status` query param，前端 query key 必须包含 view；不能让 full/status 两种 response 共用 `queryKeys.strategyBacktest(id)`。
2. 前端轮询切到 status view：
   - 新增 `StrategyBacktestRunStatusView` 类型和 `useStrategyBacktestStatusQuery()`，主路径读取 status view。
   - `hasStrategyBacktestConfigChanged()` 只依赖 status view 中的 period、benchmark、rule hash 和 execution config hash。
   - 需要 full run snapshot 的调试、发布确认或兼容路径显式调用 full endpoint。
3. `/nav` response 瘦身：
   - 优先新增 compact nav view 或 `view=ui`；只有确认没有共享消费方后，才从既有 full `/nav` 删除 `excess_return`。
   - 前端继续用 `strategy_nav - benchmark_nav` 计算 latest excess。
4. `/performance` UI view：
   - 新增 compact performance view 和前端专用类型，返回 12 个 UI metric 和 `daily_win_rate`。
   - `statuses` 和 metric 元数据保留在 detail/debug view。
   - `StrategyPortfolioPerformanceView` 当前复用 backtest performance full 类型，不能被 Step 5 compact view 破坏。
5. `/rebalance-records` 结构拆分：
   - 不直接从既有共享 response 删除 `quantity/reason`。
   - 新增 Step 5 compact view，返回 `records` 日期轴 counts + `selected_rows`，避免每个 record 都带空 rows。
   - `strategy-detail-page` 当前使用 `row.reason`，详情页和 portfolio rebalance full view 必须继续保留 full row 字段。

测试策略：

- Rust API 单测覆盖 status view 不序列化 `rule_snapshot` 和 `execution_config`。
- Rust API 单测覆盖 compact performance view 不含 `statuses`。
- 前端 query key 单测或 hook 单测覆盖 status/full cache 不互相覆盖。
- 前端类型测试或组件单测覆盖 Step 5 只消费 compact fields。
- 浏览器/network smoke 记录 payload size：run status、nav、performance、rebalance。

完成标准：

- status polling response 2y 样本从约 8.7KB 降到约 1KB 内。
- `/nav` 2y 样本至少不再返回逐点 `excess_return`。
- `/performance` Step 5 UI view 2y 样本 <= 1KB。
- 详细诊断字段仍能通过 full/detail/debug view 获取。

### Phase 4：Worker 阶段计时与 ClickHouse price bars 动态投影

目标：缩短真实结果完成时间，并把 worker 内部不可见耗时拆开。

实施项：

1. Worker 阶段计时：
   - 在 strategy backtest worker 中记录以下阶段 elapsed：
     - claim / load run
     - signal materialization total
     - each screening chunk
     - price bars query
     - simulation
     - benchmark/risk-free query
     - performance calculation
     - output serialization / write preparation
     - ClickHouse writes by table
     - PostgreSQL finalize
   - 计时写入 run `summary` 或 `progress` 的 debug/timing 字段，避免覆盖业务 summary。
   - query_id 与 elapsed 同时记录，便于关联 ClickHouse `system.query_log`。
   - 该子阶段先独立落地一次 smoke，形成 4b 优化前 worker 内部基线。
2. `query_portfolio_price_bars()` 动态投影：
   - 修改函数签名，接收需要的 indicator metrics。
   - 基础列始终包含 `security_code`、`trade_date`、`open_price_backward_adj`、`close_price_backward_adj`。
   - 只有启用 indicator stop loss 时才读取 `close_price_forward_adj` 和所需 trend metric。
   - 未启用 indicator stop loss 时不 JOIN `mart_stock_trend_indicator_daily`。
   - 启用时仅投影 `indicator_metrics()` 返回的 allowlisted trend metrics。
   - SQL builder 层仍要用与 `TREND_STOP_LOSS_METRICS`/`trend_metric_value()` 对齐的 allowlist 做二次校验，不能信任任意字符串拼接列名。
3. `PriceBar` 结构兼容：
   - 保留可选字段，动态 SQL 未返回的字段通过 serde default 为 `None`。
   - `simulate_portfolio()` 只通过 `trend_metric_value()` 读取实际启用 metric。
4. JOIN 优化：
   - 在动态投影完成后，再评估 `LEFT ANY JOIN`。
   - 只有在 `(security_code, trade_date)` 唯一性有测试或查询验证时使用 ANY JOIN。
   - 继续用 `EXPLAIN indexes = 1` 验证 quotes/trend 两侧过滤。
   - Per `schema-pk-filter-on-orderby`、`schema-pk-prioritize-filters` 和 `query-join-filter-before`，任何 SQL 改写都必须保留 `trade_date` 与 `security_code` 过滤，并证明过滤作用到 quotes/trend 两侧。
   - Per `query-join-use-any` 和 `query-join-choose-algorithm`，`LEFT ANY JOIN` 只作为动态投影之后的小幅优化，不作为首要收益来源。
5. 结果一致性：
   - 同一策略、同一 period/benchmark、同一 seed/config 下，输出 row count、summary、hash 和主要指标应与优化前一致。
   - 若动态投影导致缺失指标错误，错误必须显式化，不允许 fallback 到全列查询掩盖问题。

测试策略：

- Rust 单测覆盖 no indicator stop loss 时 SQL 不含 trend JOIN。
- Rust 单测覆盖 `price_ma_10` 时 SQL 只投影 `price_ma_10`，不投影其他 MA/EMA/BOLL。
- Rust 单测覆盖 unsupported metric 被 validate 或 SQL builder allowlist 拒绝，不进入 SQL 拼接。
- Worker timing smoke 先于动态投影结果对比，报告 4a 基线和 4b 优化后指标。
- Worker integration smoke 覆盖 1y/2y run：summary、nav latest、performance metric 与优化前一致。
- ClickHouse `EXPLAIN indexes = 1` 和 query_log 对比 read_bytes/memory。

完成标准：

- 2y 样本 price bars read_bytes 接近 RFC 0031 的 slim 对照水平，目标 <= 180 MiB。
- 2y 样本 price bars memory 目标 <= 120 MiB。
- worker summary 中能看到 simulation/performance/serialization/write preparation 的阶段耗时。
- 未启用指标止损的 run 不访问 trend table。

### Phase 5：Outbox publish 延时和 stale run 可观测性

目标：降低 create accepted 后到 worker pickup 的常规延迟，并把异常 active run 从性能聚合中剥离。

实施项：

1. Outbox dispatcher 唤醒：
   - 保留 PG outbox 事务边界。
   - create 成功后通过进程内 notify、watch channel 或更短可配置 idle sleep 唤醒 dispatcher。
   - 不直接绕过 outbox 发布 NATS。
2. Dispatcher 观测：
   - 记录 outbox row `created_at -> published_at` elapsed。
   - 记录 pending scan batch size、publish success/fail 和 NATS stream sequence。
   - publish 失败仍保留 retry 和 error。
3. Stale run 标记：
   - 增加只读诊断 query 或 metrics，列出 heartbeat 过期 active run。
   - 性能报告聚合时默认排除 stale active run。
   - 是否自动 reclaim/cancel stale run 另行 RFC/plan，不在本阶段直接引入状态修复语义。

测试策略：

- Rust 单测覆盖 dispatcher wake signal 不破坏无 pending 时 sleep 行为。
- Rust 单测覆盖 publish 失败时 outbox `status='failed'`、run `dispatch_status='publish_failed'`，且下轮仍可按现有 pending/failed 扫描重试。
- Dev smoke 记录 outbox publish p50/p95。

完成标准：

- 1y/2y 受控样本 outbox publish p95 目标 <= 0.5s。
- 没有 pending 任务时 dispatcher 不 busy loop。
- stale active run 在报告中单独列出，不污染成功路径 pickup/worker 聚合。

### Phase 6：验收报告、文档收敛和归档

目标：用同一套样本证明优化效果，并把执行结果回写到项目记录系统。

实施项：

1. 新增验收报告：
   - `docs/jobs/reports/YYYY-MM-DD-racingline-step4-step5-backtest-latency-optimization.md`
   - 包含优化前后 1y/2y 对比。
   - 记录 click-to-Step5、create HTTP、outbox publish、worker elapsed、price bars read_bytes/memory、wrapper payload size。
2. 更新当前事实文档：
   - [Racingline 系统地图](../../architecture/racingline.md)：Step 5 handoff 行为改为 create accepted 后进入状态页。
   - [Rearview 系统地图](../../architecture/rearview.md)：如新增 status/compact endpoints 或 worker timing，需要更新职责和相关文档指针。
   - [RFC 0031](../../RFC/0031-racingline-step4-step5-backtest-latency-slimming.md)：标注 implemented phases 和验收报告。
3. 归档计划：
   - 完成后将本计划移入 `docs/plans/archive/`，状态改为 `Completed`。
   - 更新 `docs/plans/README.md` 的 active/recently completed 索引。

完成标准：

- 验收报告明确列出是否达到 click-to-Step5 <= 1s。
- 所有阶段的质量门禁通过。
- 文档入口没有断链。

## 阶段顺序和发布策略

推荐顺序：

1. Phase 0：基线复核。
2. Phase 1：立即进入 Step 5。这是最大用户体验收益，优先交付。
3. Phase 2：前端状态 owner 收敛。应紧跟 Phase 1，避免新旧状态模型并存过久。
4. Phase 3：轻量 status view 和 wrapper payload。降低轮询和首屏噪声。
5. Phase 4a：worker timing。先解释 price bars 后的未知 gap。
6. Phase 4b：price bars 动态投影。缩短真实结果 ready 时间。
7. Phase 5：outbox 和 stale run 可观测性。降低队列延迟和报告污染。
8. Phase 6：验收和归档。

允许 Phase 3 和 Phase 4a 并行开发；Phase 4b 不能跳过 4a timing 基线和结果一致性验证。

## 禁止模式

- 不允许 Step 4 在 create 成功后继续等待 worker terminal。
- 不允许在 Step 5 自动提交同一配置的新 run 来掩盖 active run 状态不清晰的问题。
- 不允许用多字段 fallback 或候选路径兼容不确定的 API 结构；字段归属必须沿类型和调用点确认。
- 不允许为了缩短 payload 直接删除诊断字段而不给 detail/debug view 留出口。
- 不允许 status/full/compact response 共用同一个前端 query key，导致子集对象覆盖 full run cache。
- 不允许在未确认共享消费方前从既有 full endpoint 删除 `reason`、`quantity`、`statuses` 等字段。
- 不允许动态 price bars SQL 在指标缺失时 fallback 到全列查询；缺失必须显式报错或由 validate 阶段拦截。
- 不允许绕过 PG outbox 直接从 HTTP create 发布 NATS。
- 不允许用 stale active run 混入成功路径 p50/p95。

## 最小验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

前端阶段：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

Rust/Rearview 阶段：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 PostgreSQL migration 时追加：

```bash
cd pipeline/migrate
uv run alembic upgrade head
```

Live smoke：

```bash
make racingline-dev
```

浏览器验证使用 Docker CDP：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## 验收指标

下表是全计划完成指标；每个阶段只承担本阶段相关指标，Phase 1 发布只要求 click-to-Step5、create HTTP 和结果一致性不回退。

| 指标 | 当前基线 | 目标 |
|---|---:|---:|
| click-to-Step5 shell | 1y >= 10.7s，2y >= 23.6s | p50 <= 1s，p95 <= 1.5s |
| create HTTP | 0.13s | 不回退，p95 <= 0.5s |
| outbox publish | 0.807s / 1.454s，1y p95 1.975s | p95 <= 0.5s |
| status polling payload | 2y 8.7KB | <= 1KB |
| nav payload | 2y 64.6KB | <= 47KB，或由 overview/latest view 替代 |
| performance payload | 2y 3.8KB | <= 1KB for UI view |
| price bars read_bytes | 2y 514.89 MiB | <= 180 MiB when only `price_ma_10` is needed |
| price bars memory | 2y 277.88 MiB isolated / 844.24 MiB live query_log | slim isolated <= 120 MiB；live query_log 明显下降 |
| worker internal gap visibility | 2y 15.4s unknown gap | summary 中可拆到 simulation/performance/serialization/write preparation |

## 完成标准

- Step 4 到 Step 5 页面跳转不再等待 worker terminal，1y/2y 受控样本 click-to-Step5 达到目标。
- 前端 create/active run/polling 状态只有一个 owner，自动重复提交路径移除。
- status polling、nav、performance 和 rebalance wrapper 的字段与 UI 消费方对齐；诊断字段保留明确入口。
- worker price bars 动态投影上线并通过 1y/2y 同配置结果一致性验证。
- worker 阶段计时能解释 price bars 后的无 ClickHouse 日志间隙。
- outbox publish 延迟和 stale active run 有可观测记录。
- 验收报告、系统地图、RFC 和 plans README 已同步。
