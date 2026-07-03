# Plan 0058: Racingline Step 5 回测 worker 执行耗时优化实施计划

日期：2026-06-26

状态：Completed

## 背景

[Plan 0056](0056-racingline-step4-step5-backtest-latency-optimization-plan.md) 已经把 Step 4 到 Step 5 的页面进入路径从 worker terminal 状态中解耦。当前浏览器实测 click-to-Step5 shell 为 439ms，说明用户已能快速进入 Step 5 状态页。剩余慢点已经转移到 Step 5 run 从 `queued/running` 到 `succeeded` 的真实等待。

[RFC 0032](../../RFC/archive/0032-racingline-step5-backtest-worker-execution-latency.md) 将回测流程抽象为：

```text
Accept -> Plan -> Signals -> Market Data Demand -> Simulation -> Metrics -> Commit -> Serve
```

当前实测瓶颈集中在 worker 内部三段热路径：

| Period | total | signal materialization | price bars | simulation | writes |
|---|---:|---:|---:|---:|---:|
| 1y | 6,471ms | 2,536ms | 2,096ms | 1,031ms | 767ms |
| 2y | 13,790ms | 4,385ms | 4,302ms | 4,212ms | 838ms |

另有排队问题：2y 样本 backend total 为 20.066s，其中 6.238s 是单 worker 先处理前一个 1y run 导致的 pickup wait。这个问题需要队列/并发治理，但它不能替代单 run 热路径优化。

第一性原理下，本计划只优化 `BacktestExecutionPlan` 的执行成本，不改变 `BacktestSpec` 的业务语义。目标是把 worker 热路径从“全量候选、全量字段、全区间行情、全量 clone 索引”收敛为“TopN 可执行信号、必要行情列、必要日期范围、低 clone 价格索引和可证明的结果一致性”。

## 目标

1. 给 Step 5 worker 建立可复现的性能基线、阶段 timing 和结果等价护栏。
2. 优先优化 `Simulation`：去除全量 `PriceBar` clone、字符串 key 反复构造和下一交易日扫描。
3. 新增 worker 专用 backtest signal SQL，默认只返回 TopN 可执行信号所需字段。
4. 实验并落地 `MarketDataDemand` 行数裁剪，按每只证券最早 execution date 读取必要行情。
5. 收敛 Step 5 succeeded 后的首屏重复读取，固化可由 worker 产出的稳定指标。
6. 在单 run 热路径下降后，再治理 worker pickup wait：多 worker、bounded concurrency 或 subject/durable consumer 隔离。
7. 每阶段都形成 job report，记录 before/after 指标、命令、结果一致性和残余风险。

## 非目标

1. 不重新处理 Step 4 到 Step 5 页面进入问题；该问题已由 Plan 0056 完成。
2. 不把 Step 5 回测改成同步 HTTP。
3. 不改变回测业务口径：rule、TopN、max positions、费用、滑点、止损、benchmark、risk-free、T+1 execution 和 result attempt 语义都不变。
4. 不删除 PostgreSQL outbox、NATS JetStream 或异步 worker 边界。
5. 不复用 Step 3 preview rows 作为 Step 5 回测信号。
6. 不为了性能使用模糊 fallback、候选字段兼容或静默降级到全量数据读取。
7. 不在没有 before/after 数据和结果一致性证明前承诺最终秒数。

## 关联文档

| 文档 | 用途 |
|---|---|
| [RFC 0032](../../RFC/archive/0032-racingline-step5-backtest-worker-execution-latency.md) | Step 5 worker 执行流、参数模板、优化方案草案和讨论目标 |
| [Plan 0056](0056-racingline-step4-step5-backtest-latency-optimization-plan.md) | Step 4/5 handoff、status/compact API、worker timing、动态 price bars 和 outbox 唤醒已完成计划 |
| [0056 验收报告](../../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md) | 当前性能样本、payload、worker timing、ClickHouse query log 和 stale active 诊断 |
| [0056 前基线报告](../../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-baseline.md) | 历史 worker elapsed、price bars 全字段读取和 wrapper 重复 nav 读取基线 |
| [Racingline 系统地图](../../architecture/racingline.md) | Step 5 前端当前事实、运行入口和质量门禁 |
| [Rearview 系统地图](../../architecture/rearview.md) | Rearview API、worker、ClickHouse、NATS 和质量门禁 |

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| 页面进入 | Plan 0056 后，`POST /rearview/strategy-backtests` 返回 `202 Accepted` 即进入 Step 5，浏览器 click-to-Step5 shell 为 439ms |
| 状态轮询 | `GET /rearview/strategy-backtests/{id}/status` 返回 compact status，样本 payload 为 578 bytes |
| 结果 wrapper | `/nav`、`/performance`、`/rebalance-records` 支持 `view=ui` compact response |
| 信号生成 | `rearview-portfolio-worker` 的 `materialize_strategy_backtest_signals()` 调用 `QueryPlanner::compile()`，返回全量 ranked candidates 和 preview/explain JSON 字段，再在 Rust 中过滤 TopN |
| 行情读取 | `query_portfolio_price_bars(security_codes, start, end, indicator_metrics)` 已支持动态趋势列投影，但仍对所有入选证券读取完整 run 区间 |
| 模拟器 | `simulate_portfolio()` 把 `input.prices` clone 到 `BTreeMap<(NaiveDate, String), PriceBar>`，查价时构造 `String` key，退出检查中从 price map keys 推导下一交易日 |
| 写入 | ClickHouse portfolio/calculation facts 继续 append-only 写入 `result_attempt_id`，PostgreSQL finalize 切 `current_result_attempt_id` |
| 队列 | `rearview-portfolio-worker` 当前单 consumer 串行处理 task；pickup wait 可被前序 run 阻塞 |

## 目标指标

这些目标用于讨论和验收分层，不是无条件承诺值。每项都必须用同一输入样本 before/after 验证。

| 指标 | 当前样本 | 阶段目标 |
|---|---:|---:|
| 1y worker elapsed | 6.477s | 3-4s |
| 2y worker elapsed | 13.797s | 6-9s |
| 2y signal materialization | 4.385s | 2-3s |
| 2y price bars worker stage | 4.302s | 2-3s |
| 2y simulation | 4.212s | 1s 级别 |
| pickup wait | 单 worker 下可被前序 run 阻塞 | 并发/隔离后 p95 < 1s |
| Step 5 succeeded 后首屏 wrapper | 0.10-0.17s 级别 | 不作为 P0，若用户仍感知迟滞再优化 |

## 设计原则

1. `BacktestSpec` 是用户语义合同，不为性能优化改写。优化只发生在 `BacktestExecutionPlan` 和执行策略。
2. 先测量，再优化。Rust 热路径必须用 release 级别 timing、benchmark 或 live smoke 判断。
3. 每轮改动都要证明输出一致：row count、summary、关键指标、必要时输出 hash。
4. 热路径默认只产出用户结果和必要审计字段；完整候选、raw values、score breakdown 和 explain JSON 属于诊断模式。
5. ClickHouse 查询必须保留可用过滤条件。Per `schema-pk-filter-on-orderby`，查询应使用排序键相关过滤；Per `query-join-filter-before`，JOIN 两侧必须先过滤再 join。
6. Per `query-join-use-any`，只有右表键唯一且只需要一条匹配时才使用 `ANY JOIN`。
7. Per `query-join-choose-algorithm`，join algorithm 只有在 EXPLAIN/query_log 证明必要时才手动指定。
8. Per `insert-batch-size`，结果写入继续监控 parts 和 batch size；Per `insert-mutation-avoid-update`，继续避免 mutation 覆盖历史结果。
9. Rust 热路径遵守 borrowing over cloning：大集合默认传引用或索引，不在循环中 clone 大结构，不用无证据的 `#[inline]`。
10. worker 并发是排队优化，不是单 run 热路径优化；必须在单 run 热路径下降后再扩大并发。

## Review 补充的实现缺口

本计划初稿已经覆盖了方向，但还需要补清以下实现缺口，防止后续执行时出现“目标正确、落点不清”的问题：

| 缺口 | 风险 | 补充要求 |
|---|---|---|
| Simulation timing schema 不清 | 当前 `WorkerTiming::mark()` 只记录相邻阶段耗时；如果直接把细粒度 timing 塞进 summary，容易污染业务 summary 或覆盖既有字段。 | 定义 `worker_timing.version`、`stages_ms`、`simulation_ms`、`row_counts`、`query_ids` 的稳定结构；细粒度 timing 只进入诊断 summary，不进入前端 compact status 必需字段。 |
| Simulation 重构边界不清 | 直接改 `PortfolioSimulationInput` 或输出结构，可能扩大到 API/ClickHouse 写入层。 | Phase 1 默认先在 `simulate_portfolio()` 内部引入 `PriceStore`/`TradeCalendar`，保持 `PortfolioSimulationInput` 和 `PortfolioSimulationOutput` 外部 contract 不变。 |
| Simulation 只提到 price clone，遗漏其他热路径候选 | 当前代码还存在每日 `held` 集合 clone、`events.iter()` 计算 warning count、退出检查重复查价等潜在成本。 | Phase 1 timing 必须覆盖这些候选；是否优化以 release/profile 数据为准。 |
| Signal 窄化缺少 row type 边界 | 当前 `query_screening_rows()` 解析 `ScreeningRow`，该类型服务 preview/persisted pool，强行删字段会破坏 Step 3 和历史接口。 | 新增 `BacktestSignalRow` 和 `query_backtest_signal_rows()`；不要收缩 `ScreeningRow`。 |
| Backtest 与 daily run 信号路径重复 | 当前 `materialize_strategy_backtest_signals()` 与 `materialize_strategy_portfolio_daily_run_signals()` 高度相似；只改 backtest 会留下重复慢路径和语义漂移。 | Phase 2 先抽 `SignalMaterializationJob` 的最小共享内核，或至少在计划中明确 daily run 是否同步迁移；不能留下两个不同 TopN 语义。 |
| `generated_candidate_count` 语义可能漂移 | SQL 层 TopN 过滤后，`rows.len()` 不再等于全量候选数。 | Phase 2 必须把该指标改名、降级为 diagnostic，或用独立聚合保留原语义；不得用 TopN row count 冒充全量候选数。 |
| MarketDataDemand 直接进 worker 风险高 | per-security date window SQL 可能破坏排序键过滤，或因巨大 OR/inline table 反而变慢。 | Phase 3 必须先做离线 SQL 对比和 `EXPLAIN indexes = 1`，只有真实样本胜出才接入 worker。 |
| 结果等价缺少可执行比对方法 | 只说 row count/summary 一致不足以发现 orders/trades/nav 顺序或金额差异。 | Phase 0 增加 ClickHouse facts 快照比对口径：排除 run id/result_attempt_id/写入时间等 volatile 字段后，按业务键排序比较。 |
| Worker 并发缺少 NATS consumer 语义检查 | 当前是 JetStream pull durable consumer 单循环；多进程/并发处理需要确认 durable、ack、claim lease 和 max in-flight 组合。 | Phase 5 先审查 consumer 配置和 ack 行为，再决定多进程、bounded concurrency 或 subject 拆分。 |
| 计划缺少分支/PR 切分 | 多阶段混在一次实现里会难以定位性能回退或结果漂移。 | 每个 Phase 默认单独 PR/commit 和 job report；Phase 1-3 不应在没有基线报告时合并到同一批变更。 |

## 实施阶段

### Phase 0：基准样本与结果等价护栏

目标：让后续每个优化阶段有稳定可复现的对照。

实施项：

1. 固定 1y/2y/3y 代表样本：
   - 继续以已成功 run `0eeb7f71-028a-43fb-af91-e3ec609e4e4b` 的 stored `rule_snapshot` 和 `execution_config` 为源。
   - 每次测试生成新的 `client_request_id`，记录新 run id。
2. 定义 before/after 采集项：
   - PostgreSQL：create、outbox publish、worker pickup、worker elapsed、backend total。
   - Worker summary：`worker_timing.stages_ms`。
   - ClickHouse：query_id、duration、read_rows、read_bytes、memory。
   - 输出规模：signals、security_count、price_bar_count、nav/orders/trades/positions/events row count。
   - 业务结果：summary、performance metric、trade metrics、关键净值点。
3. 准备结果等价检查：
   - 同一 run spec 下，优化前后 row count 必须一致，除非该阶段明确改变诊断输出而不改变交易结果。
   - 浮点指标比较需要定义 tolerance；默认使用关键指标绝对/相对误差，并在 job report 记录。
   - ClickHouse facts 需要按表导出并排序比较：`portfolio_nav_daily` 按 `trade_date`，`portfolio_target` 按 `signal_date/source_rank/security_code`，`portfolio_order` 按 `order_seq`，`portfolio_trade` 按 `trade_seq`，`portfolio_position_day` 按 `trade_date/security_code`，`portfolio_event` 按 `event_seq`。
   - 比对时排除 `portfolio_run_id`、`result_attempt_id`、ClickHouse 写入时间、query id 等 volatile 字段；保留业务键、金额、数量、原因、状态和指标值。
   - 需要给每张事实表生成 row count、stable hash 和关键汇总，例如 nav 最后一日、trade gross/fee/slippage sum、order status counts。
4. 新增或更新 job report 模板：
   - 建议路径：`docs/jobs/reports/YYYY-MM-DD-racingline-step5-backtest-worker-latency-optimization.md`。
5. 明确交付切分：
   - Phase 1、Phase 2、Phase 3 默认分别实施和验收。
   - 没有 Phase 0 baseline report 时，不合并 Phase 1-3 的生产代码变更。
   - 每个阶段的 job report 必须能独立回答“耗时是否下降、输出是否一致、剩余瓶颈在哪里”。

测试策略：

- 本阶段允许只新增报告和脚本计划；如果新增自动脚本，脚本必须只创建新 perf run 或只读查询历史 run。
- 文档-only 变更运行 `make docs-check` 和 `git diff --check`。

完成标准：

- 后续阶段可以用同一输入生成 1y/2y/3y before/after 对比。
- 报告能同时解释 worker timing 与 ClickHouse query_log。
- stale active run 不混入成功路径 p50/p95。

### Phase 1：Simulation 内部 timing 与低 clone 价格索引

目标：优先处理当前 2y 样本 4.212s 的 simulation 热路径，并证明结果一致。

实施项：

1. 给 `simulate_portfolio()` 增加局部 timing 或可测试 benchmark harness：
   - price index build。
   - trade date / next trade date map build。
   - signals index build。
   - daily loop。
   - buy/sell handling。
   - valuation。
   - exit rule evaluation。
   - output row allocation/preparation。
   - 每日 held set 构造。
   - warning_count 统计。
   - pending sells enqueue/dequeue。
2. 引入 `TradeCalendarPlan`：
   - 从价格交易日或 worker trade dates 构造有序 trade dates。
   - 预构造 `trade_date -> next_trade_date`，替代从 price map keys 扫描。
   - 明确当没有下一交易日时的行为，不能静默卖出到当前日期。
3. 引入低 clone `PriceStore`：
   - 用 `Vec<PriceBar>` + `(date, security_code) -> index`，或等价低 clone 结构替代 `BTreeMap<(NaiveDate, String), PriceBar>` 全量 clone。
   - `open_price()`、`close_price()` 和 indicator metric 读取统一走 `PriceStore` 查询接口。
   - 避免在持仓循环中反复 `security_code.to_string()`。
   - 如果使用 map key，优先让 key 持有 index 或 interned security id，而不是重复分配 `String`。
   - `PriceStore` 应保持模块内部私有；外部仍传入 `PortfolioSimulationInput { prices: Vec<PriceBar>, ... }`。
4. 保持输出语义：
   - targets、orders、trades、position_day、nav、events 的生成顺序保持稳定。
   - 交割单字段、fee/slippage、reason、order_seq/trade_seq 不改变。
5. 稳定 timing 输出：
   - worker summary 中新增 `worker_timing.version = 2`。
   - 顶层 `stages_ms.simulation` 保持兼容。
   - simulation 子阶段写入 `worker_timing.simulation_ms`，例如 `price_store_build`、`calendar_build`、`signal_index_build`、`daily_loop`、`exit_evaluation`、`output_finalize`。
   - row count 写入 `worker_timing.row_counts`，不和业务 `PortfolioSummary` 字段混名。

测试策略：

- Rust 单测覆盖：
  - same-day signal 仍被拒绝。
  - 缺失 open/close price 的 skipped order/event 语义不变。
  - indicator stop loss 触发和 missing indicator 事件不变。
  - no-next-trade-date 行为明确。
  - `PriceStore` 在重复证券、多日期、缺失 open/close、缺失 indicator 字段时返回一致结果。
  - `TradeCalendarPlan` 对最后一个交易日不产生非法下一交易日。
- 增加一个中等规模 simulation benchmark 或 release-mode smoke。
- 运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

完成标准：

- 2y simulation 从 4.212s 降到 1s 级别，或 timing 清楚证明剩余耗时来自非索引构造环节。
- 同一输入样本下，simulation output row count、summary 和关键净值指标一致。
- 没有引入大集合 clone、循环内字符串分配或无界临时 collection。

### Phase 2：Backtest signal SQL 窄化

目标：把 worker 的 signal materialization 从 preview/explain 风格 SQL 收敛为回测执行专用 SQL。

实施项：

1. 在 planner 中新增 worker 专用 contract：

```text
compile_backtest_signals(rule, start_date, end_date, top_n)
  -> SELECT security_code, trade_date, score, signal_rank
     FROM ranked
     WHERE signal_rank <= top_n
     ORDER BY trade_date, signal_rank, security_code
```

2. 默认输出字段只保留：
   - `security_code`
   - `trade_date`
   - `score`
   - `signal_rank`
3. 不默认输出：
   - `raw_score`
   - `is_buy_signal`
   - `score_breakdown`
   - `selected_metrics`
   - `raw_values`
4. 诊断统计拆分：
   - `generated_candidate_count` 若仍需要，改为独立聚合或仅诊断模式计算。
   - `required_metrics`、`required_marts` 和 SQL hash 仍由 planner 产出。
5. worker 改用窄化 query result：
   - `materialize_strategy_backtest_signals()` 不再在 Rust 中二次过滤全量 ranked candidates。
   - `signal_date_count`、`top_n_candidate_count`、`dropped_signal_count` 的定义写清楚，避免换 SQL 后指标语义漂移。
6. 新增明确的 ClickHouse row 类型和查询方法：
   - 在 `rearview-core::clickhouse` 中新增 `BacktestSignalRow`，字段仅为 `security_code`、`trade_date`、`score`、`signal_rank`。
   - 新增 `query_backtest_signal_rows(sql, query_id)`；不要复用或收缩 `query_screening_rows()` 与 `ScreeningRow`。
   - `ScreeningRow` 继续服务 Step 3 preview、pool persistence 和 API explain，不因 worker 瘦身删字段。
7. 抽取或对齐 signal materialization 内核：
   - `materialize_strategy_backtest_signals()` 与 `materialize_strategy_portfolio_daily_run_signals()` 共享 `SignalMaterializationJob` 的日期 chunk、TopN、T+1 mapping、summary 计算和 progress 回调。
   - 如果 Phase 2 只迁移 backtest，必须在 job report 中明确 daily run 仍使用旧路径及后续处理计划。
8. 统计字段命名：
   - `generated_candidate_count` 保留原义时必须来自全量聚合。
   - 如果不再计算全量候选，改名为 `top_n_row_count` 或 `diagnostic_generated_candidate_count_unavailable`，避免对外语义漂移。
   - `signal_date_count` 要说明是“有 TopN 可执行信号的日期数”还是“全量 pool 命中的日期数”。

测试策略：

- Planner 单测覆盖 SQL 包含 `WHERE signal_rank <= top_n`。
- Planner 单测覆盖 worker SQL 不包含 `score_breakdown`、`selected_metrics`、`raw_values`。
- ClickHouse parser 单测覆盖 `BacktestSignalRow` 只需要窄字段即可解析。
- Worker 单测或 integration smoke 覆盖同一 rule 下 signal set、execution date 映射和 security set 一致。
- Daily run 路径若同步迁移，增加对应 smoke；若不同步迁移，明确 skipped reason。
- ClickHouse query_log 对比 read_rows/read_bytes/memory/duration。

完成标准：

- 2y `signal_materialization_total` 从 4.385s 降到 2-3s 区间，或 query_log 证明主要瓶颈在输入 mart 读取而非输出/JSON/Rust 过滤。
- SQL 输出行数等于 TopN 可执行候选行数加必要诊断行数，不再拉回全量候选。
- Step 3 preview/explain API 行为不受影响。

### Phase 3：MarketDataDemand 行数裁剪

目标：在 Plan 0056 已完成动态列投影后，进一步减少 price bars 行数。

实施项：

1. 先做离线 SQL 实验，不直接接入 worker：
   - 从 Phase 0 的 1y/2y/3y run 提取 `SignalSet` 和 security/date 分布。
   - 在 job report 中记录每只证券 earliest execution date 的分布，例如 min/p50/p90/max 和“首 20% 日期内已出现信号的证券占比”。
   - 若大多数证券都在前段出现，预期收益需要下调，避免投入高风险 SQL 改写。
2. 从 `SignalSet` 派生 `MarketDataDemand`：

```text
security_code -> earliest_execution_date
required_date_window = [earliest_execution_date, run.end_date]
required_columns = OHLC + enabled indicator metrics
```

3. 比较三种 SQL 形态：
   - 当前单查询：所有证券读取完整 run range。
   - inline demand table join：把 `(security_code, start_date)` 作为 demand 表 join 到 quotes/trend。
   - 分 chunk 查询：按 date window 或 security chunk 拆分，再合并结果。
4. 每种 SQL 必须验证：
   - Per `schema-pk-filter-on-orderby`，保留 `trade_date` 与 `security_code` 过滤能力。
   - Per `query-join-filter-before`，quotes/trend 两侧先过滤再 join。
   - Per `query-join-use-any`，只有趋势表唯一性已验证时才使用 `LEFT ANY JOIN`。
   - Per `query-join-choose-algorithm`，只有 EXPLAIN/query_log 证明 `auto` 选择不佳时才手动指定 join algorithm。
5. 覆盖风险：
   - 对持仓估值，某证券只要被买入，就需要从最早 execution date 到 end_date 的价格。
   - 对指标止损，只需要启用的 indicator metric；未启用指标不能拉全 trend 列。
   - 不允许因缺少价格数据自动 fallback 到完整区间查询；缺口要进入 data coverage summary 或显式失败/警告。
   - 如果 inline demand table 导致 query text 过大或 parse 时间升高，必须记录 query text size 和 ClickHouse parse/execute 总耗时。
6. 接入 worker 的条件：
   - 只有至少 2y 样本在 duration、read_rows、read_bytes、memory 中多数指标胜出，且结果一致，才把裁剪 SQL 接入默认 worker。
   - 若只有特定分布收益明显，可作为 feature flag 或 diagnostic experiment 保留，不进入默认路径。

测试策略：

- SQL builder 单测覆盖 demand 输入生成预期 SQL。
- ClickHouse `EXPLAIN indexes = 1` 对比三种 SQL。
- `FORMAT Null` 隔离测试和 live worker stage timing 都要记录；不能只看其中一种。
- 使用 1y/2y/3y 同样本记录 query_log。
- Worker smoke 验证 price_bar_count 变化不改变交易结果。

完成标准：

- 采用的 SQL 形态在 2y 样本中 price bars worker stage 低于当前 4.302s，且 read_rows/read_bytes 同步下降。
- 如果所有裁剪 SQL 都慢于当前单查询，记录为“不采用”，保留动态列投影，不为了理论减法牺牲实际性能。
- data coverage summary 明确记录 demand security count、date windows 和 price bar count。

### Phase 4：Metrics/Serve 首屏收敛

目标：降低 Step 5 succeeded 后结果首屏重复读取，但不抢占 worker 热路径优化优先级。

实施项：

1. 固化可由 worker 稳定产出的指标：
   - `daily_win_rate`。
   - trade ledger 聚合：buy_count、sell_count、turnover_amount、fee_amount、slippage_amount。
   - closed trade / trade metric 汇总。
   - 指标产物需要明确落点：优先进入 calculation outputs 或 run snapshot summary；不要复制到每笔 trade row。
   - 对历史 result attempt 不做 ClickHouse mutation 回填；历史结果按旧 contract 读取或在服务层兼容。
2. 避免 wrapper 重复读取 nav：
   - `/performance?view=ui` 优先读取已提交 performance/calculation output。
   - `/rebalance-records?view=ui` 首屏只返回日期轴 summary 和 selected rows。
3. 评估 Step 5 overview endpoint：

```text
GET /rearview/strategy-backtests/{id}/overview?view=ui
  -> status
  -> nav summary / latest point
  -> performance summary
  -> rebalance date rail
```

4. 保留 detail/debug view：
   - full nav、orders、trades、positions、events、closed trades 和 trade metrics 仍按分页/detail endpoint 查询。
   - 交割单明细不复制费率配置；费率来自 frozen `execution_config.fee_profile`。
5. 前端 cache 边界：
   - overview/compact endpoint 使用独立 query key。
   - full/detail endpoint 不与 overview cache 混写。
   - Step 5 首屏不能因为 overview 缺少 detail 字段而写多路 fallback。

测试策略：

- API 单测覆盖 compact response 不含诊断字段。
- 前端类型/组件测试覆盖 Step 5 UI 只依赖 compact view。
- Browser network smoke 记录 succeeded 后 time-to-chart 和 payload。
- 如果触碰前端，运行：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

完成标准：

- 首屏 wrapper 不再重复重算或重复读取 nav 来计算稳定指标。
- 诊断/detail 能力保留。
- 如果 HTTP 仍只有 0.10-0.17s 且用户无明显感知，允许只完成指标固化，不新增 overview endpoint。

### Phase 5：Worker 排队与吞吐治理

目标：降低 pickup wait，避免交互式 backtest 被前序 run 或 daily run 长时间阻塞。

实施项：

1. 先确认单 run 热路径已下降，再启用并发治理。
2. 先审查当前 NATS consumer 语义：
   - 当前 worker 使用 JetStream pull durable consumer，`ensure_portfolio_consumer()` 按 `portfolio_worker_durable` 创建 durable consumer。
   - 计划实施前必须确认多进程共享同一 durable pull consumer 时的 message delivery、ack 和 redelivery 行为。
   - `portfolio_worker_queue` 当前配置存在，但是否实际用于 queue group 需要源码确认；不能假设已经启用 queue group。
3. 候选方案按风险递增：
   - 启动多个 `rearview-portfolio-worker run` 进程，依赖 PostgreSQL claim lease 保证同一 run 只被一个 worker 处理。
   - 单进程内 bounded concurrency，显式限制 ClickHouse 并发和内存。
   - strategy backtest 与 strategy portfolio daily run 拆分 NATS subject/durable consumer。
4. 并发预算：
   - 给 interactive backtest 设置较低且稳定的并发上限。
   - 记录 ClickHouse query concurrency、memory 和 timeout。
   - 遇到内存或 query timeout 时降并发，不通过扩大 timeout 掩盖问题。
   - 不同 worker 进程必须使用同一代码版本，避免相同 run 的结果因版本不同而不可比较。
5. 任务语义：
   - 保留 at-least-once delivery。
   - claim lease、heartbeat、terminal skip 和 ack/nack 语义必须有测试或 smoke 覆盖。
   - 如果 worker 在处理过程中失败，应验证 message redelivery 或 outbox 重投递不会造成双 finalize。
6. 配置与部署：
   - 新增并发参数时必须有明确 env 名、默认值和系统地图更新。
   - dev `make racingline-dev` 启动几个 worker 必须明确；不能在开发环境无意中启动多实例影响性能基线。

测试策略：

- 使用两个 1y/2y run 并发提交，记录 pickup wait。
- 验证同一 run 不会被两个 worker finalize。
- 验证同一 run 在 worker crash/restart 后能够被重新处理或明确失败。
- 验证 stale active run 仍能被诊断 endpoint 标出。
- 记录 ClickHouse query_log，确认并发不会造成 memory 激增。

完成标准：

- pickup wait p95 在交互式样本中低于 1s，或有明确容量原因说明。
- worker_elapsed 不因并发治理显著回退。
- 没有破坏 PG outbox + NATS + claim lease 的可靠边界。

### Phase 6：验收、归档和后续治理

目标：把实施结果沉淀为报告和当前事实文档。

实施项：

1. 新增最终验收报告：
   - 建议路径：`docs/jobs/reports/YYYY-MM-DD-racingline-step5-backtest-worker-latency-optimization.md`。
   - 覆盖 1y/2y/3y before/after。
   - 记录所有命令、环境、run id、query_id、性能指标、结果一致性和残余风险。
2. 更新当前事实文档：
   - `docs/architecture/rearview.md`：worker 热路径、并发模式、result wrapper 现状。
   - `docs/architecture/racingline.md`：若 Step 5 overview 或首屏契约变化，同步更新。
   - `docs/RFC/archive/0032`：若最终设计与 RFC 草案不同，补充实施结论或后续开放问题。
3. 完成后归档本 plan：
   - 移动到 `docs/plans/archive/0058-...md`。
   - 更新 `docs/plans/README.md`。

完成标准：

- job report 能解释每个目标指标是否达成。
- active plan 列表不残留已完成计划。
- 残余风险进入新的 RFC、plan 或 debt 文档，而不是留在聊天上下文。

## 禁止模式

1. 禁止把 Step 5 回测改成同步 HTTP。
2. 禁止复用 Step 3 preview rows 作为回测信号。
3. 禁止在字段来源不确定时写多路 fallback 或候选字段兼容。
4. 禁止为了避免 SQL 设计，把 price bars fallback 到全字段、全区间、全证券读取。
5. 禁止把 `score_breakdown`、`raw_values`、完整候选池作为 worker 默认热路径输出。
6. 禁止用 ClickHouse mutation 覆盖历史 result attempt。
7. 禁止未限制并发就增加 worker consumer。
8. 禁止只看 wall time，不记录 ClickHouse read_rows/read_bytes/memory 和 worker stage timing。

## 允许保留的例外

1. 如果 `generated_candidate_count` 被产品或诊断明确要求保留，可以用独立聚合或 diagnostic mode 计算；不得让它阻塞默认 TopN 热路径。
2. 如果 per-security date demand SQL 在真实样本中慢于当前单查询，应保留当前单查询和动态列投影，并把该结论写入 job report。
3. 如果 simulation 低 clone 重构风险高，可以先只落 `TradeCalendarPlan` 和 next-date map，再分阶段替换 price store。
4. 如果 Step 5 首屏 wrapper 已经低于用户感知阈值，overview endpoint 可延期。

## 最小验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

Rust/Rearview 阶段：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Racingline 前端阶段：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

Live smoke 阶段：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
```

ClickHouse/PG 性能采样命令应写入 job report，至少包含：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml exec -T clickhouse clickhouse-client --format PrettyCompact --query "SELECT event_time_microseconds, query_id, query_duration_ms, read_rows, read_bytes, memory_usage FROM system.query_log WHERE type = 'QueryFinish' AND query_id LIKE 'strategy-backtest-%' ORDER BY event_time_microseconds"
docker compose --env-file .env -f deploy/docker-compose.yml exec -T postgres sh -lc 'psql -U "$POSTGRES_USER" -d rearview -P pager=off -c "SELECT strategy_backtest_run_id, period_key, status, created_at, started_at, completed_at FROM strategy_backtest_run ORDER BY created_at DESC LIMIT 20;"'
```

## 交付物清单

| 阶段 | 主要交付物 |
|---|---|
| Phase 0 | 基准样本、等价检查口径、job report 模板 |
| Phase 1 | simulation timing、`TradeCalendarPlan`、低 clone `PriceStore`、结果一致性报告 |
| Phase 2 | `compile_backtest_signals()`、窄化 query row 类型、worker signal path 切换 |
| Phase 3 | `MarketDataDemand`、SQL 形态对比、采用或不采用的性能结论 |
| Phase 4 | 固化指标、compact/overview read model、Step 5 首屏 payload 报告 |
| Phase 5 | worker 并发/隔离方案、pickup wait 报告、可靠性 smoke |
| Phase 6 | 最终验收报告、architecture/RFC 更新、plan 归档 |

## 完成标准

本计划完成时应满足：

1. 1y/2y/3y 样本有完整 before/after 报告。
2. 2y worker elapsed 达到 6-9s 讨论目标，或报告明确说明未达到目标的剩余瓶颈和下一步方案。
3. 2y simulation、signal materialization、price bars 三段至少两段有明确下降；未下降的阶段有数据支撑的“不采用/延期”结论。
4. pickup wait 有独立治理结论，不再与 worker_elapsed 混在一起解释。
5. 输出结果业务语义保持一致，差异有明确原因和验收口径。
6. 所有生产代码变更通过对应 Rust/前端质量门禁。
7. 文档索引、systems 当前事实和 job report 已同步。
