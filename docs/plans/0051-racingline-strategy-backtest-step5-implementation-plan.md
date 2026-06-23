# Plan 0051: Racingline 策略回测 Step 5 异步执行实施计划

日期：2026-06-23

状态：Proposed

领域：racingline, rearview

关联系统：racingline, rearview, data-platform, deploy-ops

代码根：

- `app/racingline_new/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/migrate/`

关联文档：

- [RFC 0028: Racingline 策略回测 Step 5 异步执行方案](../RFC/0028-racingline-strategy-backtest-step5.md)
- [RFC 0027: Racingline 模拟建仓 Step 4 实现方案](../RFC/0027-racingline-strategy-simulation-position-step4.md)
- [RFC 0026: Racingline 股池预览 Step 3 实现方案](../RFC/0026-racingline-strategy-pool-preview-step3.md)
- [RFC 0022: 组合数据面迁移 ClickHouse 与绩效指标分层](../RFC/0022-portfolio-data-plane-clickhouse-and-metrics.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md)
- [ADR 0012: 组合净值递推与绩效指标权威计算留在 Rust](../ADR/0012-portfolio-nav-recursion-stays-in-rust.md)
- [System: Racingline](../systems/racingline.md)
- [System: Rearview](../systems/rearview.md)

相关规则：

- `fleur-harness`：计划必须承接当前代码事实、列出阶段、验证命令和完成标准。
- `clickhouse-best-practices`：本计划复用现有 ClickHouse portfolio data plane，不新增高基数分区；查询必须按 `portfolio_run_id/result_attempt_id` 命中 ORDER BY 前缀；结果写入必须批量 insert，避免 `ALTER UPDATE` mutation；调仓聚合和证券展示 join 必须先过滤再 join，单值展示用 `ANY JOIN` 或独立 display lookup。对应规则：`schema-pk-prioritize-filters`、`schema-pk-filter-on-orderby`、`schema-partition-low-cardinality`、`insert-batch-size`、`insert-mutation-avoid-update`、`query-join-filter-before`、`query-join-use-any`。

## 目标

1. 完成 `/strategies` Step 5 第一版真实回测闭环：创建异步 backtest run、排队、worker 执行、写入结果、前端轮询和展示。
2. 将 Step 1/2 的 `RuleVersionSpec`、Step 3 非 stale applied preview snapshot、Step 4 canonical `BacktestExecutionConfig`、Step 5 回测区间和 benchmark 固化为一次可复现 backtest run。
3. 复用现有 PostgreSQL outbox + NATS JetStream + `rearview-portfolio-worker` 异步边界，HTTP create 只返回 `202 Accepted`。
4. 复用现有 `fleur_portfolio` 和 `fleur_calculation` data plane，避免新增 Step 5 专用 ClickHouse 结果表。
5. 修正 Step 4/5 执行语义缺口：`buy_signal_top_n` 表示每日按分数优先取出的可调入候选数量，`max_positions` 表示组合最大持仓股数，两者必须独立生效，不能互相覆盖。
6. 让 worker 从 transient `rule_snapshot` 按回测区间重新生成信号，不依赖正式 `rule_version_id`、正式 `run` 或 Step 3 preview rows。
7. Racingline Step 5 移除 mock 成功路径，只展示真实 backtest run 状态和结果。
8. Step 5 UI 以 `app/racingline_new/src/routes/strategy-page.tsx` 现有 `BacktestPanel` 原型为准，补齐该页面需要的配置、净值、调仓、持仓、绩效和状态数据。
9. 完成短区间 live smoke 和验收报告，证明 Step 1 到 Step 5 的真实执行流程连贯。

## 非目标

1. 不实现“运行策略”发布流程；成功 backtest run 到正式 rule set/version/account template 的创建另起 RFC 或 plan。
2. 不创建隐藏的用户可见 `rule_set`、`rule_version` 或正式 `run` 来承载 Step 5 草稿。
3. 不在浏览器内计算权威净值、持仓、成交、费用、滑点或绩效指标。
4. 不把 Step 3 preview response 当作 Step 5 历史回测数据源。
5. 不新增独立 backtester 服务；第一版继续复用 `rearview-portfolio-worker`。
6. 不新增 Step 5 专用 ClickHouse 结果事实表；结果事实继续写入 `fleur_portfolio`，绩效和交易级指标继续写入 `fleur_calculation`。
7. 不在第一版实现取消运行、暂停恢复、批量参数扫描、跨 backtest 排名或归因分析。
8. 不允许任意 SQL、Python、Rust 或前端公式作为回测逻辑或卖出规则。

## 当前事实基线

1. `app/racingline_new` Step 1/2/3 已使用 Rearview 真实接口形成 `RuleVersionSpec` 和 `PreviewSnapshot`，Step 3 preview 不是持久 run。
2. Step 4 已有 `SimulationSettings -> BacktestExecutionConfig` adapter，并通过 `POST /rearview/strategy-backtests/validate` 获取 canonical config、`rule_hash` 和 `execution_config_hash`。
3. `strategy_backtest.rs` 当前只提供 validate contract；它不创建 run、不写 PostgreSQL、不发 NATS、不写 ClickHouse。
4. `BacktestExecutionConfig::canonicalized()` 当前存在语义风险：它会把 `rebalance_policy.max_positions` 设为 `signal_policy.buy_signal_top_n`，这与 Step 4 页面业务语义冲突。
5. `PortfolioSimulationInput` 和 `simulate_portfolio()` 已支持 `max_positions`、`single_position_limit_pct`、费用、滑点、固定止损、止盈、时间止损和指标止损。
6. 现有 portfolio worker 从 `portfolio_run.source_run_id` 读取 PostgreSQL `buy_signal`，不能直接消费 transient `RuleVersionSpec`。
7. 现有 `portfolio_run` schema 强依赖 `source_run_id` 和 `rule_version_id`，不适合直接保存未发布策略草稿。
8. `portfolio_task_outbox` 和 NATS message 当前只表达 portfolio run：消息包含 `portfolio_run_id` 和 `source_run_id`。
9. `fleur_portfolio` ClickHouse 表使用 `portfolio_run_id + result_attempt_id` append-only 结果版本，符合 Step 5 结果复用需求。
10. `portfolio_metric_config` 当前外键指向 `portfolio_run`，独立 `strategy_backtest_run` 不能直接写入该表。
11. `PerformanceMetricConfig::default_full_period()` 当前默认 benchmark 为 `000300.SH`，Step 5 用户选择的 benchmark 还没有进入 worker 权威计算配置。
12. `app/racingline_new/src/routes/strategy-page.tsx` 的 Step 5 仍使用静态 `backtestNetValuePoints`、`backtestRebalanceRecords` 和 `backtestPerformanceGroups`，真实执行按钮仍未接入。
13. `BacktestPanel` 当前原型已经固定第一版页面形态：回测配置、净值走势、持仓记录、调仓日横向列表、调入/持有/卖出分组表和策略业绩侧栏。
14. 现有 portfolio result API 能返回 `nav`、`targets`、`orders`、`trades`、`positions`、`events`、`performance`、`closed-trades` 和 `trade-metrics`，但没有直接返回 Step 5 原型需要的 benchmark 净值曲线、调仓记录聚合行、证券名称和日胜率。
15. `PortfolioSimulationInput` 当前只有 `start_date`，没有 `end_date`；现有 portfolio worker 会把行情查询延后 14 天，Step 5 必须避免净值、成交和绩效越过用户选择的回测结束日。
16. `simulate_portfolio()` 当前会用 lifetime `bought_history` 跳过曾经买入过的证券；真实策略回测只应禁止重复买入当前持仓，卖出后的证券后续再次入选时必须允许重新买入。
17. 现有 `query_portfolio_price_bars()` 只投影固定趋势指标列；Step 4 指标止损支持 MA、MA 组合和 EMA 后，Step 5 必须保证所选指标止损字段进入 worker 行情输入。
18. NATS JetStream 是 at-least-once 交付；Step 5 control plane 必须能在 worker 崩溃、消息重投递或 ClickHouse 已写但 PostgreSQL 未 finalize 时恢复到可解释状态。

## 设计缺口补充

| 缺口 ID | 缺口 | 必须补齐的设计口径 | 落地阶段 |
|---|---|---|---|
| G1 | `TopN` 与 `maxPositions` 语义被合并。 | `buy_signal_top_n` 只限制每日可调入候选数；`max_positions` 只限制组合最大持仓和空闲仓位。实际买入数 `<= min(buy_signal_top_n, vacant_slots)`。 | Phase 0, Phase 3 |
| G2 | Step 5 没有 first-class control plane。 | 新增 `strategy_backtest_run`，固化 rule/config/range/benchmark/hash/status/current attempt。 | Phase 1 |
| G3 | NATS message 缺少 task kind。 | 扩展 typed message：`kind = portfolio_run | strategy_backtest`；旧消息兼容为 `portfolio_run`。 | Phase 2 |
| G4 | Worker 只能从正式 source run 读 signals。 | 为 `strategy_backtest` 新增 transient signal materialization：planner 编译 rule，按 chunk 查询 ClickHouse，生成内存态 `BuySignalInput[]`。 | Phase 3 |
| G5 | Step 5 benchmark 没进入绩效配置。 | `PerformanceMetricConfig` 支持 benchmark 参数，写入 `strategy_backtest_metric_config`，结果表通过 `config_hash` 指向该配置。 | Phase 4 |
| G6 | `portfolio_metric_config` FK 只支持 `portfolio_run`。 | 新增 `strategy_backtest_metric_config` sibling table；后续再考虑泛化为 `simulation_metric_config`。 | Phase 1, Phase 4 |
| G7 | ClickHouse snapshot 字段仍是 portfolio 命名。 | 第一版 `portfolio_run_id = strategy_backtest_run_id`；`execution_snapshot` 记录 `source_kind = strategy_backtest`、`strategy_backtest_run_id`、hash 和 benchmark。 | Phase 4 |
| G8 | Step 5 UI mock 会伪造成成功结果。 | 移除用户成功路径中的静态净值、持仓和绩效样例；接口失败只能显示 error/empty。 | Phase 5 |
| G9 | 进度不可解释。 | `strategy_backtest_run` 返回阶段状态和粗粒度 progress：signal chunks、market data、simulation、performance、write result。 | Phase 2, Phase 5 |
| G10 | 重复点击可能创建重复 run。 | Create API 支持 `client_request_id`；无用户系统时按 request hash + client id 做弱幂等。 | Phase 2 |
| G11 | Step 5 UI 所需数据没有 contract。 | 以现有 `BacktestPanel` 为准定义 `StrategyBacktestConfigSummary`、`StrategyBacktestNavPoint`、`StrategyBacktestRebalanceRecord`、`StrategyBacktestPerformanceView`。 | Phase 0, Phase 5 |
| G12 | 净值走势需要 benchmark normalized nav。 | `/strategy-backtests/{id}/nav` 返回 `strategy_nav`、`benchmark_nav` 和 `excess_return`；benchmark 从 run snapshot 读取并在后端按相同交易日归一化，不由前端计算。 | Phase 4 |
| G13 | 持仓记录原型需要调仓日聚合和证券名称。 | 新增 `/strategy-backtests/{id}/rebalance-records` wrapper，后端聚合 nav/trades/positions/closed-trades 并用 `mart_stock_basic_snapshot` 补 `security_name`。 | Phase 4, Phase 5 |
| G14 | 策略业绩原型包含 `日胜率`，现有 performance metric 没有该字段。 | 第一版不扩 ClickHouse 表字段；后端 Rust wrapper 基于 nav rows 计算 `daily_win_rate` 并随 performance view 返回，前端不得自行计算权威绩效。 | Phase 4 |
| G15 | Step 4 配置摘要进入 Step 5 后会丢展示语义。 | `strategy_backtest_run` 保存非权威 `ui_display_snapshot`，GET 返回 canonical config summary；计算只使用 `execution_config`。 | Phase 1, Phase 2, Phase 5 |
| G16 | 回测区间和 T+1 执行窗口没有闭合语义。 | `execution_date` 必须落在 `[start_date, end_date]`；`signal_date = end_date` 的买入信号不执行；输出 nav/trades/performance 不得越过 `end_date`。 | Phase 0, Phase 3 |
| G17 | 缺少轻量数据预检和 worker 数据覆盖口径。 | Create 阶段做 calendar/benchmark/range 预检；worker 阶段记录 signals、prices、benchmark、risk-free、indicator columns 覆盖摘要，并通过 succeeded/failed status + coverage warnings 表达结果。 | Phase 2, Phase 3, Phase 4 |
| G18 | at-least-once 消息缺少 attempt/lease 语义。 | `strategy_backtest_run` 增加 `worker_attempt_no`、`claimed_at`、`heartbeat_at`、`claim_expires_at`；重投递只能 claim 过期或可重算状态，新 result attempt append-only。 | Phase 1, Phase 3 |
| G19 | 曾经买入过的证券被永久禁止再买。 | 修正 portfolio simulation：只禁止当前持仓重复买入；卖出后再次进入 TopN 且有空仓时允许重新买入。 | Phase 0, Phase 3 |
| G20 | 指标止损可能选到行情输入没有的字段。 | indicator stop-loss create validation 与 worker price loader 必须按 selected metric 读取 MA、MA 组合、EMA 主图指标；缺列是 validation/compile 失败，不是 silent skip。 | Phase 0, Phase 3 |
| G21 | no-lookahead contract 没有被测试固定。 | 策略信号只使用 `signal_date` 收盘已知数据，买入/卖出都在下一交易日开盘执行；任何同日成交或未来行情引用都必须由测试阻断。 | Phase 0, Phase 3 |
| G22 | 结果可追溯性缺少数据和 SQL provenance。 | 保存 `catalog_hash`、`compiled_sql_hash`、`required_metrics`、`required_marts`、`data_preflight_snapshot` 和 result attempt provenance；严格 immutable mart snapshot 另起后续议题。 | Phase 1, Phase 3, Phase 6 |

## 第一性原理闭环审视

Step 5 做完后要称为“策略回测闭环”，必须同时满足下面六个条件：

1. 输入闭环：Step 1/2 的 `RuleVersionSpec`、Step 3 non-stale snapshot、Step 4 canonical `execution_config`、Step 5 range/benchmark 和 UI display labels 都被一次性冻结，后续草稿变化不会污染已创建 run。
2. 信号闭环：worker 能在历史每个 `signal_date` 用同一套 planner/catalog 重新计算候选池和 score，按 TopN 取候选，再映射到下一交易日开盘，且不使用未来数据。
3. 交易闭环：simulation 有明确 `[start_date, end_date]`、交易日历、价格、费用、滑点、手数、仓位、止盈止损、指标止损和 re-entry 语义；输出不越过用户选择的结束日。
4. 异步闭环：HTTP create 只落库和入队；NATS/worker 可以 claim、heartbeat、重投递、失败终态化和 append-only 重算；用户永远能通过 GET run 得到可解释状态。
5. 结果闭环：所有结果事实写入 `fleur_portfolio` / `fleur_calculation`，并由 PostgreSQL `current_result_attempt_id` 指向当前有效 attempt；ClickHouse 查询按 `portfolio_run_id/result_attempt_id` 命中 ORDER BY 前缀。
6. UI 闭环：Racingline Step 5 只展示真实 run 状态和 result wrapper API 数据；没有 mock success fallback，失败、空结果、数据缺失和 stale 都有明确页面状态。

Review 结论：原计划已覆盖主链路，但在补齐 G16-G22 之前只能保证 happy path，不能保证完整闭环。补齐这些缺口后，Step 5 第一版可以形成“配置 -> 异步回测 -> 结果持久化 -> UI 展示”的闭环；“将成功 backtest 发布成正式运行策略”仍按非目标另起方案。

## Step 5 UI 原型数据合同

第一版 Step 5 UI 不重新设计页面结构，以 `app/racingline_new/src/routes/strategy-page.tsx` 里的 `BacktestPanel` 原型为准。实现只把 mock 数据替换为真实数据，并补齐原型页面天然需要但当前后端还没有一等表达的数据。

| UI 区域 | 原型字段 | 权威数据源 | 必须补齐的数据设计 |
|---|---|---|---|
| 回测配置 | 周期、回测区间、业绩比较基准、重新回测按钮状态。 | `strategy_backtest_run.start_date/end_date/benchmark_security_code`，前端 period 只负责生成区间。 | GET run 返回 range、benchmark code、benchmark label；server validate benchmark 是否在允许列表内。 |
| Step 4 配置摘要 | 初始金额、买入信号 Top N、最大持仓、单票上限、买入规则、调仓规则、交易费率、滑点、卖出条件。 | canonical `execution_config` 和 `summary`。 | 新增 `StrategyBacktestConfigSummary`；`ui_display_snapshot` 只保存 label，不参与 hash 或计算。 |
| 异步状态 | idle、queued/running、succeeded、failed、stale 和 progress。 | `strategy_backtest_run.status/progress/error/current_result_attempt_id`。 | GET run 必须足够驱动页面状态，不依赖 NATS 或 worker 日志。 |
| 净值走势 | `time`、策略净值、基准净值、最新超额收益。 | `fleur_portfolio.portfolio_nav_daily` + benchmark quote/return。 | `/nav` 返回 `trade_date`、`strategy_nav`、`benchmark_nav`、`excess_return`；benchmark 缺失返回 null/status，不显示 0。 |
| 调仓日列表 | 每个交易日、持仓只数、买入/持有/卖出数量。 | nav `position_count`、trades、positions。 | `/rebalance-records` 返回 date rail；默认选最新有持仓或成交的交易日。 |
| 持仓记录表 | direction、股票名称、代码、持仓天数、涨跌幅、成本价、现价、收益贡献。 | positions、trades、closed-trades、nav、security display rows。 | 后端返回聚合行；`security_name` 缺失时前端显示 code；收益贡献由后端给出 decimal，前端只格式化。 |
| 策略业绩侧栏 | 策略净值、基准净值、收益/风险/性价比/相对市场指标。 | `calc_portfolio_performance_metric`、`calc_portfolio_trade_metric`、nav。 | performance view 由后端补 `daily_win_rate`；metric status 透传给 UI，缺失指标显示 unavailable，不显示 0。 |
| 空态和失败态 | 没有结果、接口失败、回测失败、配置过期。 | run status、query error、stale snapshot。 | 任何失败都不能 fallback 到 mock 曲线、mock 交易或 mock 绩效。 |

第一版新增的 UI 聚合响应建议固定为：

```text
StrategyBacktestNavPoint:
  trade_date
  strategy_nav
  benchmark_nav?
  excess_return?

StrategyBacktestRebalanceRecord:
  trade_date
  position_count
  buy_count
  hold_count
  sell_count
  rows[]

StrategyBacktestRebalanceRow:
  direction = buy | hold | sell
  security_code
  security_name?
  quantity
  holding_days
  change_pct?
  cost_price?
  current_price?
  contribution_pct?
  reason?
```

调仓记录聚合口径：

1. `buy` 行来自当日 filled buy trades；同日存在 position row 时用 position 的 `average_entry_price`、`close_price`、`unrealized_return` 和 `unrealized_pnl`。
2. `hold` 行来自当日 position rows，排除当日 buy securities 和当日 sell securities。
3. `sell` 行来自当日 filled sell trades，并用 closed trade row 补 `holding_days`、`realized_return`、`realized_pnl` 和 `exit_reason`。
4. `contribution_pct` 对持仓行使用 `unrealized_pnl / total_equity`；对卖出行使用 `realized_pnl / total_equity`。分母为当日 nav `total_equity`，缺失时返回 null。
5. `security_name` 通过现有 ClickHouse `mart_stock_basic_snapshot` display 查询补齐，不新增结果事实表。

## 执行流程

第一版端到端执行流程固定为：

```text
Step 3 non-stale PreviewSnapshot
  + Step 4 BacktestExecutionDraft
  + Step 5 range/benchmark
  -> POST /rearview/strategy-backtests
  -> validate rule/config/range/benchmark and compute hashes
  -> lightweight data preflight: trade calendar, benchmark availability, catalog hash
  -> PostgreSQL insert strategy_backtest_run + strategy_backtest_task_outbox
  -> rearview-server outbox dispatcher publishes typed NATS task
  -> rearview-portfolio-worker consumes kind=strategy_backtest
  -> claim strategy_backtest_run with worker attempt/lease
  -> compile transient SQL, store compiled_sql_hash and required metrics/marts
  -> materialize transient buy signals from rule_snapshot over signal dates
  -> map signal_date to execution_date and drop execution_date > end_date
  -> load market prices and active indicator-stop metrics for [start_date, end_date]
  -> build PortfolioSimulationInput from immutable execution_config + start/end window
  -> simulate_portfolio()
  -> compute_performance_metric() and benchmark normalized nav
  -> write fleur_portfolio result facts with portfolio_run_id=strategy_backtest_run_id
  -> write fleur_calculation outputs
  -> insert strategy_backtest_metric_config
  -> finalize strategy_backtest_run with current_result_attempt_id
  -> Racingline polls status and reads result wrapper APIs
```

关键约束：

1. `POST /rearview/strategy-backtests` 必须返回 `202 Accepted`，不得同步执行完整回测。
2. NATS message 只包含 task kind 和 run id，不包含 rule/config/行情/结果。
3. Worker 只从 PostgreSQL immutable snapshot 读取 rule/config/range/benchmark；不得从前端当前草稿、Step 3 preview rows 或市场默认模板重新推导。
4. 信号生成的 `top_n` 来自 `execution_config.signal_policy.buy_signal_top_n`。
5. 组合最大持仓来自 `execution_config.rebalance_policy.max_positions`。
6. 每日实际买入数量受 `buy_signal_top_n` 和空闲仓位共同约束：

```text
daily_candidate_limit = buy_signal_top_n
vacant_slots = max_positions - current_position_count_after_sells
actual_buy_count <= min(daily_candidate_limit, vacant_slots)
```

7. 如果 `max_positions = 5`，第一天 TopN 候选只有 3 只且无持仓，则最多买入 3 只；第二天 TopN 候选有 6 只但已有 3 只持仓且无卖出，剩余仓位只有 2，只能按分数优先买入 2 只。
8. 已持仓证券不重复买入；价格缺失、现金不足或低于最小手数的候选可在 TopN 候选列表内跳过，但总买入数不得超过空闲仓位。
9. ClickHouse 结果 append-only，不按 run id 做 mutation 覆盖；重算生成新 `result_attempt_id`，PostgreSQL 指向当前有效 attempt。
10. Result query 必须按 `portfolio_run_id/result_attempt_id` 过滤，命中现有 ORDER BY 前缀。
11. 回测结果窗口固定为 `[start_date, end_date]`；任何 nav、trade、position、performance observation 都不得越过 `end_date`。
12. `signal_timing = close_confirm_next_open` 表示 `signal_date` 收盘确认，下一交易日开盘成交；如果下一交易日不存在或大于 `end_date`，该信号不执行。
13. 卖出后的证券后续再次进入 TopN 且有空仓时可以重新买入；禁止的是当前持仓重复买入，不是 lifetime 去重。
14. Create API 的轻量预检可以查 calendar/benchmark/catalog metadata，但不得同步跑完整 screening、portfolio simulation 或绩效计算。
15. Worker 重投递必须通过 claim/lease 和新 `result_attempt_id` 保持幂等；已经写入但未 finalize 的旧 attempt 不作为当前结果展示。
16. 涉及证券展示、benchmark nav 或调仓聚合的 ClickHouse 查询，必须先按 run/attempt/date/security 过滤，再做必要 join；只需要单条证券展示时使用 `ANY JOIN` 或独立 display lookup。

## 实施阶段

### Phase 0: 契约冻结和漂移修正测试

目标：在动后端状态机前固定 Step 5 contract、TopN/maxPositions 语义和 deterministic fixture。

任务：

1. 定义 Rust 和 TypeScript 类型清单：
   - `StrategyBacktestCreateRequest`
   - `StrategyBacktestRunRecord`
   - `StrategyBacktestProgress`
   - `StrategyBacktestTaskMessage`
   - `StrategyBacktestMetricConfig`
   - `StrategyBacktestResultAttempt`
   - `StrategyBacktestConfigSummary`
   - `StrategyBacktestNavPoint`
   - `StrategyBacktestRebalanceRecord`
   - `StrategyBacktestRebalanceRow`
   - `StrategyBacktestPerformanceView`
2. 固定状态枚举：
   - `created`
   - `queued`
   - `compiling_signals`
   - `running_clickhouse`
   - `loading_market_data`
   - `calculating_nav`
   - `computing_performance`
   - `writing_results`
   - `succeeded`
   - `failed_validation`
   - `failed_compile`
   - `failed_market_data`
   - `failed_simulation`
   - `failed_write`
   - `cancelled`
3. 增加 `BacktestExecutionConfig::canonicalized()` 单测，证明：
   - `buy_signal_top_n = 3`
   - `max_positions = 5`
   - canonical 后两者仍分别为 3 和 5。
4. 增加 portfolio simulation fixture，覆盖：
   - `max_positions = 5`
   - 第一天 TopN 候选 3 只，买入 3 只。
   - 第二天 TopN 候选 6 只，已有 3 只持仓，无卖出，只买入分数靠前 2 只。
   - 高分候选价格缺失时，在 TopN 候选列表内检查后续候选。
   - 不重复买入已持仓证券。
   - 卖出后的证券再次进入 TopN 且有空仓时允许重新买入。
   - `signal_date = end_date` 的候选不会生成越过 `end_date` 的成交。
   - 输出 nav/trades/positions 的最大日期不超过 `end_date`。
   - 同日收盘信号不能同日开盘买入，防止 no-lookahead 语义漂移。
5. 固定 `BacktestPanel` 页面数据 contract：配置摘要、净值点、调仓记录、绩效视图都使用数值字段，格式化留在前端。
6. 固定 active indicator stop-loss metric contract：
   - MA：`price_ma_*`。
   - MA 组合：`price_avg_ma_*`。
   - EMA：`price_ema*`。
   - 所选 metric 必须能被 `PriceBar` 或动态 indicator map 表达。
7. 固定 Step 5 create request 的 hash 口径：后端重新计算 `rule_hash` 和 `execution_config_hash`，前端传入 hash 只做一致性校验。
8. 固定 provenance 字段口径：`catalog_hash`、`compiled_sql_hash`、`required_metrics`、`required_marts`、`data_preflight_snapshot`、`result_attempt_id`。
9. 固定短区间 live smoke 样本规则、日期范围、benchmark、TopN、maxPositions 和 Step 4 风控参数。

完成标准：

1. 计划和 RFC 0028 对 `TopN` / `maxPositions` 没有语义冲突。
2. 单元测试 fixture 明确证明每日可调入候选数和最大持仓数独立生效。
3. Backtest create request、run record、progress、config summary 和 result API 字段命名固定。
4. UI contract 覆盖现有 `BacktestPanel` 的全部页面字段，不再依赖 mock 字段结构。
5. 单元测试固定回测结束日裁剪、卖出后重新买入和 no-lookahead T+1 执行语义。
6. 指标止损 contract 覆盖 MA、MA 组合和 EMA 三类主图指标。

验证命令：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core strategy_backtest
cargo test -p rearview-core portfolio
git diff --check
```

### Phase 1: PostgreSQL control plane migration

目标：新增 Step 5 专用控制面表，不污染正式 rule/run/portfolio run 语义。

任务：

1. 在 `pipeline/migrate/versions/rearview/` 新增 migration。
2. 创建 `strategy_backtest_run`：
   - `strategy_backtest_run_id`
   - `rule_snapshot`
   - `rule_hash`
   - `execution_config`
   - `execution_config_hash`
   - `catalog_hash`
   - `compiled_sql_hash`
   - `required_metrics`
   - `required_marts`
   - `data_preflight_snapshot`
   - `preview_id`
   - `preview_range`
   - `start_date`
   - `end_date`
   - `benchmark_security_code`
   - `price_basis`
   - `ui_display_snapshot`
   - `client_request_id`
   - `request_hash`
   - `status`
   - `dispatch_status`
   - `nats_stream_sequence`
   - `worker_attempt_no`
   - `claimed_at`
   - `heartbeat_at`
   - `claim_expires_at`
   - `progress`
   - `summary`
   - `signal_summary`
   - `data_coverage_summary`
   - `error_type`
   - `error_message`
   - `current_result_attempt_id`
   - `created_at`
   - `updated_at`
   - `started_at`
   - `completed_at`
3. 创建 `strategy_backtest_task_outbox`，字段对齐 `portfolio_task_outbox`，但外键指向 `strategy_backtest_run`。
4. 创建 `strategy_backtest_metric_config`，字段语义对齐 `portfolio_metric_config`，但外键指向 `strategy_backtest_run`。
5. `data_preflight_snapshot` 至少保存：
   - requested range。
   - resolved trading date count。
   - first/last trading date。
   - benchmark code 和可用 return row count。
   - risk-free tenor 和可用 row count。
   - metric catalog hash。
   - required metrics/marts 初始集合。
6. `signal_summary` 和 `data_coverage_summary` 至少保存：
   - signal date count。
   - generated candidate count。
   - executable signal count。
   - dropped signal count and reason。
   - price bar security count。
   - missing price event count。
   - missing indicator event count。
   - benchmark/risk-free missing counts。
7. 增加约束：
   - `start_date <= end_date`
   - `price_basis = 'backward_adjusted'`
   - `benchmark_security_code <> ''`
   - `status` 只能取 Phase 0 枚举。
   - `dispatch_status in ('pending', 'published', 'publish_failed')`
   - `worker_attempt_no >= 0`
   - `claim_expires_at is null or claim_expires_at > claimed_at`
8. 增加索引：
   - `idx_strategy_backtest_status_created`
   - `idx_strategy_backtest_dispatch_status_created`
   - `idx_strategy_backtest_request_hash`
   - `idx_strategy_backtest_current_attempt`
   - `idx_strategy_backtest_claim_expires`
9. `ui_display_snapshot` 保存展示标签和原型上下文：
   - period value/label。
   - benchmark label。
   - buy rule label。
   - rebalance rule label。
   - fee row labels。
   - exit rule labels。
   它不得参与 `execution_config_hash`，也不得被 worker 用于计算。
10. repository 增加 create/get/list/claim/finalize/fail/progress/heartbeat/provenance update 方法。
11. claim 方法必须：
   - 只 claim `created/queued/compiling_signals/running_clickhouse/loading_market_data/calculating_nav/computing_performance/writing_results` 且未被有效 lease 持有的 run。
   - 增加 `worker_attempt_no`。
   - 设置 `claimed_at/heartbeat_at/claim_expires_at`。
   - 清空上一 attempt 的 transient error，但不删除旧 result facts。

完成标准：

1. 全新 rearview database migration 后三张表存在。
2. 重复执行 migration 不产生重复对象。
3. rollback 可删除新增表和索引。
4. repository 单测覆盖 create/get/claim/finalize/fail。
5. create/get roundtrip 能保留 `ui_display_snapshot`，但 hash 只由 rule 和 canonical execution config 决定。
6. repository 单测覆盖 claim lease、heartbeat、过期重 claim 和 terminal status 不可 claim。
7. provenance 字段能在 run get response 中返回或通过 debug 字段定位。

验证命令：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
cd ../engines
cargo test -p rearview-core strategy_backtest
```

### Phase 2: Rearview create/get API 和 typed outbox

目标：HTTP create 能落控制面、返回 `202 Accepted`，outbox 能发布 typed task。

任务：

1. 新增 API：
   - `POST /rearview/strategy-backtests`
   - `GET /rearview/strategy-backtests/{strategy_backtest_run_id}`
2. `POST` handler 执行：
   - validate request range。
   - 调用 `StrategyBacktestValidateRequest::validate()`。
   - 校验前端传入 hash 与后端 hash 一致。
   - 校验 benchmark 是否在 Step 5 允许列表内。
   - 校验 Step 4 canonical draft 未 stale；如果 request hash 与前端声明的 draft hash 不一致，返回 validation error。
   - 轻量查询交易日历，确认 `[start_date, end_date]` 内至少有两个交易日。
   - 轻量查询 benchmark returns，确认 benchmark 在回测区间有可用数据；缺失时返回 validation error 或 warning policy 明确的 degraded status。
   - 轻量查询默认 risk-free tenor，记录可用行数；缺失不阻止 create，但 performance status 必须可解释。
   - 记录 metric catalog hash 和初始 data preflight snapshot。
   - 计算 `request_hash`。
   - 生成或接收 `ui_display_snapshot`，仅用于 UI 展示。
   - 处理 `client_request_id` 幂等：相同 `client_request_id + request_hash` 返回同一 run；相同 `client_request_id` 但 request hash 不同返回 conflict。
   - 同事务写 `strategy_backtest_run` 和 `strategy_backtest_task_outbox`。
   - 返回 `202 Accepted`。
3. 扩展 NATS task message：

```json
{
  "kind": "strategy_backtest",
  "run_id": "01J..."
}
```

4. 保持兼容：旧消息含 `portfolio_run_id` 时按 `portfolio_run` 处理。
5. `rearview-server` outbox dispatcher 支持 strategy backtest outbox；可以先共享同一 stream 和 subject。
6. `GET` 返回状态、hash、range、benchmark、`config_summary`、`ui_display_snapshot`、summary、error、progress 和 `current_result_attempt_id`。
7. `config_summary` 必须覆盖 Step 4 页面配置：
   - initial cash。
   - buy signal TopN。
   - max positions。
   - single position limit。
   - buy timing。
   - rebalance policy。
   - fee profile。
   - slippage profile。
   - fixed stop loss。
   - indicator stop loss。
   - take profit。
   - time stop loss。
8. GET run 必须返回 closure diagnostics：
   - `worker_attempt_no`。
   - `data_preflight_snapshot`。
   - `signal_summary`。
   - `data_coverage_summary`。
   - `compiled_sql_hash`。
   - `current_result_attempt_id`。
9. `重新回测` 必须生成新的 `client_request_id`；否则会被幂等逻辑识别为同一次 create。

完成标准：

1. Create API 不执行回测，只创建 queued run 和 pending outbox。
2. 重复提交相同 `client_request_id` 返回同一 run。
3. Outbox publish 成功后更新 `dispatch_status = published` 和 NATS sequence。
4. 相同 `client_request_id` 搭配不同 request hash 返回 conflict，避免错误复用旧 run。
5. API 测试覆盖 validate 失败、hash mismatch、benchmark 不支持、空交易日历、idempotency、idempotency conflict 和 get not found。
6. GET run response 能直接驱动 Step 5 配置摘要、状态区和诊断区。

验证命令：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core strategy_backtest
cargo clippy -p rearview-core --all-targets --all-features -- -D warnings
```

### Phase 3: Worker transient signal materialization

目标：worker 能直接从 `rule_snapshot` 在回测区间生成 signals，不依赖正式 `source_run_id`。

任务：

1. `rearview-portfolio-worker` message loop 分发：
   - `kind = portfolio_run` 走现有流程。
   - `kind = strategy_backtest` 走新增 `process_strategy_backtest_run()`。
2. 新增 `StrategyBacktestRunRecord -> BacktestWorkerInput` 转换，读取 immutable `rule_snapshot`、`execution_config`、range 和 benchmark。
3. 修正 `BacktestExecutionConfig::canonicalized()`：
   - 不再把 `rebalance_policy.max_positions` 覆盖为 `signal_policy.buy_signal_top_n`。
   - 校验 `buy_signal_top_n > 0`。
   - 校验 `max_positions > 0`。
   - 允许 `buy_signal_top_n` 小于、大于或等于 `max_positions`。
4. signal materialization：
   - 使用 `QueryPlanner::compile()` 按 chunk 编译 transient rule。
   - chunk 只覆盖 `[start_date, end_date]` 内的 signal dates；不读取 `end_date` 之后的信号。
   - 保存每个 chunk 的 `sql_hash`，并合成 `compiled_sql_hash` 写回 run。
   - 写回 `required_metrics` 和 `required_marts`，用于 debug 和数据 provenance。
   - `top_n` 使用 `execution_config.signal_policy.buy_signal_top_n`。
   - 查询 ClickHouse screening rows。
   - 只保留 `signal_rank <= buy_signal_top_n` 的 rows。
   - 通过交易日历把 `signal_date` 映射到下一交易日 `execution_date`。
   - 丢弃 `execution_date > end_date` 或没有下一交易日的 rows，并在 `signal_summary.dropped_signal_count` 中记录 reason。
   - 转换为 `BuySignalInput[]`。
5. progress 更新：
   - `compiling_signals`
   - `running_clickhouse`
   - `loading_market_data`
   - `calculating_nav`
6. market data loading：
   - 价格查询窗口固定为 `[start_date, end_date]`，不得为了 T+1 执行把结果窗口延后。
   - security universe 来自 executable signals 和当前持仓路径，不从 Step 3 preview rows 派生。
   - active indicator stop-loss metric 必须进入 `PriceBar` 输入；MA、MA 组合、EMA 均要有字段或动态 map。
   - 所选 indicator metric 不存在于 catalog/price loader 时，run 进入 `failed_compile` 或 `failed_validation`，不得 silent skip。
   - 缺少个别证券价格按 portfolio event 记录并继续；整个区间无价格或无法形成 nav 时进入 `failed_market_data`。
7. 构造 `PortfolioSimulationInput`：
   - `initial_cash` 来自 `execution_config.account.initial_cash`。
   - `start_date` 和 `end_date` 来自 run range。
   - `max_positions` 来自 `execution_config.rebalance_policy.max_positions`。
   - `single_position_limit_pct` 来自 canonical config。
   - `fee_profile`、`slippage_profile`、`exit_rules` 均来自 immutable config。
8. 修正 portfolio simulation re-entry：
   - 删除或收缩 lifetime `bought_history` 语义。
   - 已持仓证券不得重复买入。
   - 已卖出证券后续再次入选 TopN 且有空仓时允许买入。
9. 修正 portfolio simulation end-date：
   - `PortfolioSimulationInput` 增加 `end_date`。
   - simulation 只遍历 `[start_date, end_date]` 内 trade dates。
   - pending sell 如果下一交易日超过 `end_date`，不执行，最终持仓按 `end_date` 收盘计价。
   - 输出 rows 全部满足 `trade_date/execution_date <= end_date`。
10. worker retry/lease：
   - claim 成功后设置 heartbeat。
   - 长 chunk 之间更新 heartbeat/progress。
   - 消息重投递时，只有 lease 过期或 run 处于可重算非终态才能重新 claim。
   - 每次重新执行生成新 `result_attempt_id`；旧未 finalize attempt 留在 ClickHouse 但不展示。
11. 增加 worker 单测覆盖 typed message routing、Step 5 config conversion、claim lease、end-date clipping、re-entry、indicator stop-loss metric projection。

完成标准：

1. Worker 能在无正式 `rule_version_id/source_run_id` 时生成信号并进入 portfolio simulation。
2. `TopN=3/maxPositions=5` 测试通过。
3. 信号 materialization 不写 PostgreSQL `buy_signal`，只作为本次 backtest worker 输入。
4. 任何 Step 5 回测失败都进入对应 `failed_*` 终态并 ack message；无法写终态时不 ack，依赖 JetStream 重投递。
5. 成功 run 的所有输出日期不超过 `end_date`。
6. 卖出后重新入选的证券能被再次买入。
7. active indicator stop-loss 的 MA、MA 组合和 EMA 三类指标都能进入行情输入或在 create/compile 阶段失败。
8. worker 崩溃后消息重投递不会把 run 卡在非终态；重新执行只会通过新的 result attempt 展示结果。

验证命令：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core strategy_backtest
cargo test -p rearview-core portfolio
cargo test -p rearview-portfolio-worker
cargo clippy -p rearview-portfolio-worker --all-targets --all-features -- -D warnings
```

### Phase 4: ClickHouse 结果写入和 result wrapper API

目标：Step 5 结果复用 existing portfolio/calculation data plane，并通过 strategy-backtests URL 查询。

任务：

1. 结果写入：
   - `portfolio_run_id = strategy_backtest_run_id`
   - 新 `result_attempt_id = ULID`
   - `execution_snapshot` 内记录 `source_kind = strategy_backtest`、`strategy_backtest_run_id`、`rule_hash`、`execution_config_hash`、`benchmark_security_code`、`catalog_hash`、`compiled_sql_hash`、`required_metrics`、`required_marts`。
2. 写入顺序沿用现有 portfolio writer：
   - `fleur_portfolio` result facts
   - `strategy_backtest_metric_config`
   - `fleur_calculation` outputs
   - `portfolio_run_snapshot`
   - PostgreSQL finalize
   如果任何 ClickHouse 或 PostgreSQL 写入失败，run 进入 `failed_write`，不得把 `current_result_attempt_id` 指向部分写入 attempt。
3. `PerformanceMetricConfig::default_full_period()` 改为接受 benchmark 参数。
4. performance wrapper 补齐 Step 5 原型需要的 `daily_win_rate`：
   - 口径为非空 `daily_return` 中 `daily_return > 0` 的交易日占比。
   - 由后端 Rust wrapper 基于 nav rows 计算并返回，不在第一版扩 ClickHouse calculation schema。
   - 前端只负责格式化，不自行计算权威口径。
5. `strategy_backtest_metric_config.config_hash` 与 `calc_portfolio_performance_metric.config_hash` 对齐。
6. 新增 wrapper API：
   - `/rearview/strategy-backtests/{id}/nav`
   - `/rearview/strategy-backtests/{id}/rebalance-records`
   - `/rearview/strategy-backtests/{id}/targets`
   - `/rearview/strategy-backtests/{id}/orders`
   - `/rearview/strategy-backtests/{id}/trades`
   - `/rearview/strategy-backtests/{id}/positions`
   - `/rearview/strategy-backtests/{id}/events`
   - `/rearview/strategy-backtests/{id}/performance`
   - `/rearview/strategy-backtests/{id}/closed-trades`
   - `/rearview/strategy-backtests/{id}/trade-metrics`
7. `/nav` response 面向 `BacktestNetValueChart`：
   - 返回 `trade_date`、`strategy_nav`、`benchmark_nav`、`excess_return`。
   - strategy nav 来自 `portfolio_nav_daily.nav`。
   - benchmark nav 用 `benchmark_security_code` 的 daily return 对齐 strategy nav 日期后从 1.0 复利归一化。
   - benchmark nav 的第一点与 strategy nav 第一点评估日期一致；缺少第一日 return 时从下一条有效 return 开始，缺口在 status 中返回。
   - benchmark 数据缺失时返回 null 和 status，不能补 0。
8. `/rebalance-records` response 面向原型“持仓记录”：
   - date rail 返回 `trade_date`、`position_count`、`buy_count`、`hold_count`、`sell_count`。
   - selected date 返回 `buy | hold | sell` 分组 rows。
   - rows 补齐 `security_name`、`holding_days`、`change_pct`、`cost_price`、`current_price`、`contribution_pct`、`reason`。
   - security display 使用现有 `query_security_display_rows()` 读取 `mart_stock_basic_snapshot`。
9. wrapper API 先解析 `strategy_backtest_run.current_result_attempt_id`，再调用现有 ClickHouse query 方法。
10. 查询必须按 `portfolio_run_id = strategy_backtest_run_id` 和 `result_attempt_id` 过滤，符合 `schema-pk-filter-on-orderby`。
11. 调仓聚合查询必须先将 portfolio 表过滤到当前 `portfolio_run_id/result_attempt_id/date` 后再 join 或聚合；证券展示补全使用 `query_security_display_rows()` 或 `LEFT ANY JOIN`，符合 `query-join-filter-before` 和 `query-join-use-any`。
12. 写入必须按表批量 insert，避免逐行 insert；符合 `insert-batch-size`。单行 run snapshot 允许沿用 append-only 单行 insert，但不得高频循环写。
13. 不使用 `ALTER UPDATE` 或删除旧 attempt；符合 `insert-mutation-avoid-update`。
14. `/performance` response 返回 metric value + status：
   - 缺 benchmark 或 risk-free 时对应 alpha/beta/IR/Sharpe/Treynor 等指标返回 null + reason。
   - `daily_win_rate` 由 nav rows 计算，并返回 observation count。
   - UI 不把 null 指标渲染为 0。

完成标准：

1. 成功 backtest run 能在 ClickHouse 查到 nav、targets、orders、trades、positions、events、performance。
2. wrapper API 不暴露 portfolio-run URL 给 Step 5 前端。
3. benchmark 选择影响 performance metric 输入；缺失 benchmark 时绩效状态可解释，不能显示 0。
4. ClickHouse 查询路径命中 `portfolio_run_id/result_attempt_id` 前缀。
5. `/nav` 能返回策略和 benchmark 两条曲线所需数据。
6. `/rebalance-records` 能按现有原型渲染调仓日列表和调入/持有/卖出明细。
7. performance response 覆盖原型侧栏全部指标：持仓收益、年化收益、超额收益、日胜率、最大回撤、年化波动率、下行波动率、Sharpe、Sortino、Calmar、Treynor、Alpha、Beta、Information Ratio。
8. result wrapper 的 ClickHouse 查询满足按 run/attempt 前缀过滤；涉及 join 的查询先过滤再 join，单值展示使用 `ANY JOIN` 或 display lookup。
9. 写失败或 finalize 失败不会暴露部分写入 attempt。

验证命令：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core portfolio_performance
cargo test -p rearview-core strategy_backtest
cargo clippy -p rearview-core --all-targets --all-features -- -D warnings
```

### Phase 5: Racingline Step 5 UI 接入

目标：Step 5 用户路径从 mock 成功态切换为真实异步 backtest run。

任务：

1. TypeScript 新增类型：
   - `StrategyBacktestCreateRequest`
   - `StrategyBacktestRunRecord`
   - `StrategyBacktestProgress`
   - `StrategyBacktestConfigSummary`
   - `StrategyBacktestNavPoint`
   - `StrategyBacktestRebalanceRecord`
   - `StrategyBacktestRebalanceRow`
   - `StrategyBacktestPerformanceView`
2. API client 新增：
   - `createStrategyBacktest()`
   - `getStrategyBacktest()`
   - `listStrategyBacktestNav()`
   - `listStrategyBacktestRebalanceRecords()`
   - `listStrategyBacktestPerformance()`
   - `listStrategyBacktestPositions()`
   - `listStrategyBacktestTrades()`
   - `listStrategyBacktestEvents()`
3. TanStack Query hooks：
   - create mutation。
   - status polling query。
   - succeeded 后启用 result queries。
4. `BacktestPanel` 以现有原型为准，只替换数据来源，不重做页面结构。
5. `BacktestPanel` 状态：
   - `idle`：必须持有 `BacktestExecutionDraft`；展示 Step 4 draft 摘要、建仓规则、仓位、买入规则、卖出规则、费率、调仓、range、benchmark。
   - `queued/running`：展示 stage/progress。
   - `succeeded`：展示真实 nav、rebalance records、performance、positions/trades/events。
   - `failed_*`：展示 error type/message 和重试。
   - `stale`：Step 1/2/4 改动后，已有 run 作为历史快照，当前草稿需重新回测。
6. 移除用户成功路径 mock：
   - `backtestNetValuePoints`
   - `backtestRebalanceRecords`
   - `backtestPerformanceGroups`
   - `buildBacktestTrade()`
7. UI 必须同时展示 canonical `buy_signal_top_n` 和 `max_positions`。
8. UI 必须展示 Step 4 canonical 配置：
   - 初始金额。
   - 单票上限。
   - 买入规则 `T+1日开盘价买入`。
   - 调仓规则 `仓位空余按信号调入`。
   - 佣金、印花税、过户费、滑点。
   - 固定止盈、固定止损、指标止损、时间止损。
9. `开始回测` 按钮只在以下条件满足时启用：
   - `previewSnapshot && !previewSnapshot.stale`
   - `BacktestExecutionDraft && !draft.stale`
   - market fee template 和 Step 4 validate 已成功。
   - range 和 benchmark 合法。
10. `开始回测` 使用 `buildBacktestExecutionRequestDraft()` 构造 create request，并附带：
   - `client_request_id`。
   - `rule_hash`。
   - `execution_config_hash`。
   - range。
   - benchmark。
   - `ui_display_snapshot`。
11. Step 5 不自动重新执行；用户修改 range/benchmark 后必须点击开始或重新回测。
12. `重新回测` 必须生成新的 `client_request_id`，保留旧 run 作为历史快照，当前视图切到新 run polling。
13. 修改 Step 1/2/4 后：
   - 标记当前 draft stale。
   - 禁用开始/重新回测。
   - 保留已成功 run 的结果但标记“历史快照”，不得把旧结果当作当前草稿结果。
14. result queries 只在 run `succeeded` 且 `current_result_attempt_id` 非空后启用；否则显示状态或空态。

完成标准：

1. 断开 Rearview 时 Step 5 只能显示 error，不显示 mock 成功曲线。
2. 成功 run 的净值、benchmark 净值、绩效、调仓记录、持仓和成交来自 Rearview API。
3. TopN/maxPositions 在 UI 摘要和后端结果中一致。
4. Step 1/2/4 改动后当前 Step 5 草稿 stale，不能误用旧结果。
5. 原型中的配置、净值走势、持仓记录和策略业绩区域都有真实数据或明确空态。
6. `重新回测` 不会因为复用旧 `client_request_id` 而拿到旧 run。
7. result queries 在 run 未成功前不会请求缺失的 result attempt。

验证命令：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

### Phase 6: Live smoke 和验收报告

目标：用真实 dev 依赖验证 Step 1 到 Step 5 连贯执行。

任务：

1. 启动完整 dev 环境：

```bash
make racingline-dev
```

2. 在 `/strategies` 配置代表性规则：
   - Step 1 至少 1 组选股条件。
   - Step 2 至少 2 个评分规则。
   - Step 3 成功 preview 并进入非 stale snapshot。
   - Step 4 设置 `buyTopN = 3`、`maxPositions = 5`、单票上限、费率、止损/止盈。
   - Step 5 选择短区间和 benchmark，创建 backtest run。
3. 确认 NATS outbox 发布成功，worker 消费并写结果。
4. 验证 PostgreSQL：
   - `strategy_backtest_run.status = succeeded`
   - `current_result_attempt_id` 非空。
   - `compiled_sql_hash` 非空。
   - `data_preflight_snapshot`、`signal_summary`、`data_coverage_summary` 非空。
   - `worker_attempt_no >= 1`。
5. 验证 ClickHouse：
   - `fleur_portfolio.portfolio_nav_daily` 有 run rows。
   - `fleur_calculation.calc_portfolio_performance_metric` 有 benchmark 对应 rows。
   - `max(trade_date) <= strategy_backtest_run.end_date`。
   - 查询条件使用 `portfolio_run_id` 和 `result_attempt_id`。
6. 浏览器验收：
   - Step 5 loading/running/succeeded/failed 状态。
   - 净值曲线非 mock。
   - benchmark 净值曲线来自真实 benchmark 数据。
   - 持仓记录能展示调仓日、调入/持有/卖出分组、证券名称、成本价、现价、涨跌幅和收益贡献。
   - 策略业绩侧栏覆盖原型中的收益、风险、性价比和相对市场指标。
   - TopN/maxPositions 摘要正确。
   - Step 4 的建仓规则、仓位、买入规则、卖出规则、费率和调仓配置在 Step 5 摘要中可见。
   - mobile 和 desktop 不重叠。
7. 故障和重试验收：
   - 人为让 worker 在未 finalize 前失败一次，重投递后能重新 claim 并成功或进入可解释 failed status。
   - 重算产生的新 attempt 不覆盖旧 attempt，UI 只读取 `current_result_attempt_id`。
8. 语义验收：
   - `signal_date = end_date` 的信号不产生 `end_date` 之后的成交。
   - 卖出后的证券后续再次入选 TopN 时可以重新买入。
   - MA、MA 组合、EMA 指标止损各跑一个短样本。
9. 写入 `docs/jobs/reports/` 验收报告。

完成标准：

1. Step 1 到 Step 5 在真实 Rearview/NATS/ClickHouse 环境中完成一次短区间回测。
2. 结果可通过 Step 5 UI 和 Rearview wrapper API 查询。
3. 无 mock 成功路径。
4. 验收报告包含命令、日期范围、run id、result attempt id、compiled sql hash、data coverage summary、关键截图或 Playwright 说明。

验证命令：

```bash
make racingline-dev
cd engines
cargo run -p rearview-portfolio-worker -- run --once
cd ../app/racingline_new
npm run lint
npm run typecheck
npm test
```

## 禁止模式

1. 禁止把 Step 5 回测放在 HTTP 请求内同步执行。
2. 禁止把完整 rule/config/行情/结果塞进 NATS message。
3. 禁止创建隐藏的用户可见 `rule_set`、`rule_version` 或正式 `run` 来支撑 Step 5 草稿。
4. 禁止把 `max_positions` 覆盖为 `buy_signal_top_n`。
5. 禁止把 Step 3 preview rows 当作 Step 5 历史回测输入。
6. 禁止 Step 5 UI 用 mock 数据作为接口失败后的 success fallback。
7. 禁止在 ClickHouse 上用 mutation 覆盖旧结果 attempt。
8. 禁止结果查询跳过 `portfolio_run_id/result_attempt_id` 过滤。
9. 禁止 benchmark 缺失时把绩效指标显示为 0。
10. 禁止前端自行计算 benchmark 净值、日胜率、收益贡献或绩效权威口径。
11. 禁止用 `backtestTradeCandidates` 或其他静态证券样例补调仓记录。
12. 禁止 Step 5 UI 脱离现有 `BacktestPanel` 原型重新设计首版页面结构。
13. 禁止输出 `end_date` 之后的 nav、trade、position 或 performance observation。
14. 禁止用 lifetime bought history 阻止已卖出证券后续重新买入。
15. 禁止 worker 崩溃或消息重投递后把 run 永久卡在非终态。
16. 禁止在未写完全部 result facts 和 calculation outputs 前 finalize `current_result_attempt_id`。
17. 禁止 indicator stop-loss metric 缺列时 silent skip。

## 最小质量门禁

文档-only 阶段：

```bash
make docs-check
git diff --check
```

后端阶段：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head

cd ../engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端阶段：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

端到端验收：

```bash
make racingline-dev
```

## 完成标准

1. `POST /rearview/strategy-backtests` 创建真实异步 run，并返回 `202 Accepted`。
2. `rearview-server` outbox dispatcher 能发布 typed `strategy_backtest` NATS task。
3. `rearview-portfolio-worker` 能消费 task，从 transient rule 生成 signals，执行 portfolio simulation，写入 ClickHouse，并 finalize PostgreSQL control plane。
4. `buy_signal_top_n` 和 `max_positions` 独立生效，测试覆盖 `TopN=3/maxPositions=5`。
5. 用户选择的 benchmark 进入 performance metric config。
6. `/rearview/strategy-backtests/{id}/...` wrapper APIs 能读取当前 `result_attempt_id` 的结果。
7. `/rearview/strategy-backtests/{id}/nav` 返回策略和 benchmark 两条净值曲线。
8. `/rearview/strategy-backtests/{id}/rebalance-records` 返回原型持仓记录所需的调仓日和行级字段。
9. Racingline Step 5 移除 mock 成功路径，只展示真实运行状态和结果。
10. Step 5 摘要展示 Step 4 的建仓规则、仓位、买入规则、卖出规则、费率和调仓配置。
11. 回测输出严格裁剪在 `[start_date, end_date]` 内。
12. 卖出后再次入选的证券允许重新买入。
13. MA、MA 组合、EMA 三类指标止损在 worker 行情输入和短样本中通过。
14. worker at-least-once 重投递不会导致 stuck run 或暴露部分写入 attempt。
15. run 保存 compiled SQL/data coverage provenance，能解释信号和结果来源。
16. live smoke 完成并写入 job report。

## 后续维护动作

1. 实现完成后将本计划状态改为 `Completed`，移入 `docs/plans/archive/`，并更新 `docs/plans/README.md`。
2. 同步 `docs/systems/racingline.md` 和 `docs/systems/rearview.md` 的当前事实，标记 Step 5 已实现。
3. 写入 `docs/jobs/reports/<date>-racingline-strategy-step5-backtest.md` 验收报告。
4. 如“运行策略”进入开发，基于成功 backtest run 另起 RFC/plan，定义正式策略发布、账户模板和看板回流。
