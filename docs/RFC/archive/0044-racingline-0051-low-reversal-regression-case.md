# RFC 0044: Racingline 0051 低位反转数据配置与清算回归用例

状态：Implemented
日期：2026-07-02
实现日期：2026-07-02
领域：rearview, dagster, portfolio testing
关联系统：engines/crates/rearview-core, engines/crates/rearview-server, engines/crates/rearview-portfolio-worker, pipeline/scheduler
相关文档：
- docs/jobs/reports/2026-06-24-racingline-strategy-portfolio-publish-dashboard-dagster.md
- docs/plans/archive/0051-racingline-strategy-backtest-step5-implementation-plan.md
- docs/RFC/archive/0028-racingline-strategy-backtest-step5.md
- docs/RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md
- docs/architecture/rearview.md
- docs/architecture/scheduler-architecture.md
- docs/architecture/racingline.md
- docs/plans/archive/0072-racingline-0051-low-reversal-example-live-job-plan.md
- docs/jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md

## 摘要

0051 低位反转用例应固化为“数据配置入口 + Rearview 同一套后端业务管道 + Dagster 清算验收”的回归样例。

第一性原理是：前端不是策略、组合和清算的业务实现，前端只是配置录入客户端。真正必须复用的是前端路径背后的 Rearview service/canonicalization/persistence/worker 逻辑。只要纯数据配置入口复用这条后端路径，就可以把差异限制在“入口来源不同”；配置进入 Rearview 之后，规则规范化、控制面快照、股票池、信号、持仓、订单、成交、净值和 live facts 的运行与加工过程应与前端提交同配置完全一致。

本 RFC 不设计浏览器操作、Playwright、截图或 Dashboard 跳转验收。`example__portfolio_live_job` 只负责调用 Rearview example ensure API、触发 daily run、等待 worker 清算并查询 fact counts。

## 核心原则

系统只应有一条业务管道：

```text
External input
  -> Rearview request DTO
  -> shared validation/canonicalization service
  -> shared portfolio persistence/control-plane service
  -> official portfolio daily-run API
  -> rearview-portfolio-worker
  -> rearview-core portfolio simulation
  -> live_* facts and calculation facts
```

前端创建组合和 0051 example job 的区别只在入口：

| 入口 | 作用 |
|---|---|
| Racingline | 用户交互后提交配置 |
| 0051 example data config | 稳定 fixture 生成同构 Rearview request |

两者进入 Rearview 后必须走同一套 service/canonicalization/persistence 逻辑。不能为 example job 新增第二套规则解释、第二套配置规范化、第二套 portfolio snapshot 构造、第二套 daily-run worker 输入或第二套 live fact 写入。

## 设计结论

采用一个单一方案：

1. 固化 0051 为可版本化 data config。
2. Rearview 新增或抽出一个共享后端 service boundary，把“前端提交配置”和“example data config”都转换为同一个 canonical portfolio create/publish 输入。
3. Rearview example ensure API 只作为入口适配层，复用上述共享 service，不直接写表。
4. Dagster 注册手动 `example__portfolio_live_job`，调用 Rearview ensure API 和正式 daily-run API。
5. Worker 从持久化后的 portfolio snapshot 重新生成股票池和信号，再调用同一个 portfolio simulation 与 live facts 写入路径。

这个方案避免绕远路：不需要设计前端自动化，也不需要为了 example 复制一套后端计算。要验证的是“同一配置进入 Rearview 后得到同一 canonical snapshot，并沿同一清算路径运行”。

## 当前事实

运行报告 [2026-06-24 Racingline Strategy Portfolio Publish Dashboard Dagster](../../jobs/reports/2026-06-24-racingline-strategy-portfolio-publish-dashboard-dagster.md) 已确认：

- Step 5 支持基于成功 backtest result 建立正式 strategy portfolio。
- Dagster 已注册日分区 asset `rearview/strategy_portfolio_daily_runs`、job `strategy_portfolio__daily_run_job` 和 schedule `portfolio__daily_run_schedule`。
- 0051 低位反转用例已使用真实 Rearview API、PostgreSQL、ClickHouse 和 worker 计算数据完成回测并创建策略组合。
- Racingline Dashboard/detail 已消费 Rearview API/read model，但这只作为既有系统事实，不作为本 RFC 的测试入口。

当前代码事实：

- `StrategyBacktestValidateRequest::validate` 会校验 rule、canonicalize execution config，并返回稳定 `rule_hash` 和 `execution_config_hash`。
- `BacktestExecutionConfig::canonicalized` 会补齐 `signal_timing = close_confirm_next_open`、`target_weighting = equal_weight_capped`、`empty_signal_action = hold` 和默认 slippage mode，并执行校验。
- JSON hash 使用 canonical key ordering，保证同一语义输入得到稳定 hash。
- `RearviewApiResource` 目前只封装 strategy portfolio daily-run create/range/status/fact-counts/settlement-target API，没有 example portfolio ensure/bootstrap 方法。
- `rearview/strategy_portfolio_daily_runs` asset 的分区起点是 `2026-06-24`，不适合作为 `2024-01-02` example 建仓日的分区资产直接复用。
- Portfolio worker 的 live daily run 会从 portfolio 的 `rule_snapshot` 重新编译规则，在 `run_start_date..trade_date` 查询 TopN 信号，加载行情，再调用 portfolio simulation。
- Portfolio simulation 当前按交易日循环：先执行已排队到当天的卖出，再按当天 `execution_date` 的买入信号补空仓；卖出和买入都使用执行日开盘价并应用滑点。卖出释放的现金和仓位可以用于同一天后续买入。
- live output 会从 `portfolio.live_start_date` 开始归一化并过滤，清算后写入 `fleur_portfolio.live_*` 和相关 calculation facts。

0051 的历史运行值，例如 backtest run UUID、result attempt、portfolio id/code 和 `2025-06-03..2026-06-01` 回测区间，只是历史证据，不应成为长期断言。

## 0051 Data Config

0051 data config 是入口 fixture，不是业务事实源。它的职责是稳定表达“用户会从前端提交的同一组配置”。

配置至少包含：

| 配置块 | 内容 |
|---|---|
| `case_id` | 稳定标识，例如 `racingline_0051_low_reversal` |
| `version` | fixture schema/config 版本 |
| `rule_spec` | Step 1 过滤条件、Step 2 评分条件、条件组和 canonical 字段 |
| `execution_config` | 初始资金、TopN、最大持仓、单票上限、费率、滑点和风控 |
| `benchmark_security_code` | `000300.SH` |
| `planned_live_start_date` | `2024-01-02` |
| `fixture_hash` | 对 config 做稳定 hash，用于幂等和审计 |

关键约束：

1. data config 不应保存“已经规范化后的最终 portfolio snapshot”作为权威输入。
2. data config 应被转换为与前端同配置一致的 Rearview request DTO。
3. canonical `rule_snapshot`、`execution_config`、rule hash 和 execution config hash 必须由 Rearview 共享 service 产出。
4. Dagster 不解析规则、不计算评分、不决定日期语义、不构造 portfolio snapshot。

### Step 1 过滤条件

Step 1 是一个 AND 条件组：

```text
kdj_j_value < 13
pct_amplitude < 4
pct_change > -2
pct_change < 2
volume < prev_volume * 0.8
price_ema2_10 > price_avg_ma_14_28_57_114
close_down_streak_days < 4
close_price_forward_adj > price_avg_ma_3_6_12_24
price_ma_60 > price_ma_114
price_ma_114 > price_ma_250
```

### Step 2 评分条件

Step 2 使用 conditional points。`n_structure_20_is_valid = true` 是 N 型反转结构评分项，命中加 `+20`；它不属于 Step 1 硬过滤。

当前字段来自 `mart_stock_price_pattern_daily`，语义是 20 根有效 high/low 窗口内存在 L1 -> H1 -> L2，且当前有效 K 线已从 L2 重新上攻；仅 `rebound` 或 `breakout` 阶段为 true。

```text
kdj_j_value < -15                                      +25
-15 <= kdj_j_value < -10                               +15
volume < volume_ma_5 * 0.6                             +20
n_structure_20_is_valid = true                         +20
price_ma_20 < close_price_forward_adj < price_ma_60    +15
close_price_forward_adj < boll_lower_20_2              +15
rsi_6 < 25                                             +5
```

KDJ 两条评分规则必须互斥：`kdj_j_value < -15` 命中 `+25`，`-15 <= kdj_j_value < -10` 命中 `+15`。不得把 `J < -15` 同时计入 `J < -10` 的 `+15`。

### 执行配置

| 配置 | 值 |
|---|---|
| `initial_cash` | `1000000` |
| `buy_signal_top_n` | `5` |
| `max_positions` | `5` |
| `single_position_limit_pct` | `0.2` |
| 费率、滑点 | Rearview default market fee template |
| 卖出风控 | 仅启用固定止盈和指标止损两条 |
| 固定止盈 | 收益达到 `15%` 时止盈卖出 |
| 指标止损 | 收盘价跌破 `price_ma_10` (MA10) 时止损 |
| `benchmark_security_code` | `000300.SH` |
| `planned_live_start_date` | `2024-01-02` |

`buy_signal_top_n` 和 `max_positions` 必须独立生效：前者限制每日按分数优先取出的可调入候选数量，后者限制组合最大持仓数和空闲仓位。

0051 example 不启用固定止损和时间止损。若 Rearview default market fee template 或 Step 4 UI 默认风控未来变化，本用例仍以本表中的两条卖出风控为准。

## 后端等价入口

Rearview 需要提供一个 example ensure 能力，但它只做入口适配和幂等控制。

建议语义：

```text
POST /rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure
```

该 API 的内部路径必须是：

```text
0051 data config
  -> convert to normal Rearview request DTO
  -> shared validate/canonicalize service
  -> shared portfolio snapshot builder
  -> shared portfolio persistence/control-plane service
  -> return portfolio id + hashes + dates
```

幂等规则：

1. 按 `case_id + version + fixture_hash` 查找现有 example portfolio。
2. 已存在且 canonical hash 一致时复用。
3. 已存在但 hash 不一致时失败或创建新版本，不静默覆盖。
4. 返回 portfolio id、portfolio code、fixture hash、rule hash、execution config hash、`live_start_date` 和 `initial_signal_date`。

禁止模式：

1. 不直接写 PostgreSQL 表来模拟 portfolio。
2. 不在 example API 中手工拼 `rule_snapshot` 或 `execution_config`。
3. 不实现第二套 canonicalization/default template/date resolve。
4. 不让 Dagster 传入已经加工好的 worker 输入。
5. 不创建 example 专用股票池、信号、持仓或 live facts 表。

如果现有 publish/create 逻辑强依赖 backtest result attempt，而 example 不需要真实走完整 backtest，则正确方向是抽出共享的 canonical snapshot/persistence service，让 publish/create 和 example ensure 都调用它；不是让 example 复制 publish 的数据库写入逻辑。

## 触发模型

共享 canonical snapshot/persistence service 本身不应该是一个可被 Dagster 或用户直接触发的独立任务。它是 Rearview 内部业务 service，由两个入口同步调用：

| 触发来源 | 入口 | 后端动作 |
|---|---|---|
| 前端用户路径 | Racingline 调用正式 portfolio create/publish API | API handler 调用共享 validation/canonicalization/snapshot/persistence service |
| 0051 example 路径 | Dagster `example__portfolio_live_job` 调用 Rearview example ensure API | ensure API handler 调用同一个共享 service |

也就是说，“先抽出/复用共享 service”不是新增一个调度步骤，而是重构 Rearview 代码边界：把原来藏在前端 publish/create API 背后的核心业务逻辑抽成 service，让正式 API 和 example ensure API 都调用它。

完整触发链路应是：

```text
manual/CI dg launch
  -> example__portfolio_live_job
  -> POST Rearview example ensure API
  -> shared validation/canonicalization/snapshot/persistence service
  -> return ensured strategy_portfolio_id + hashes + dates
  -> POST official Rearview daily-run range API
  -> PostgreSQL daily_run + outbox
  -> Rearview outbox dispatcher publishes NATS task
  -> rearview-portfolio-worker consumes task
  -> worker recomputes pool/signals, simulates portfolio, writes live facts
  -> Dagster polls daily-run status and fact counts
```

本地或 CI 的显式触发点是 Dagster job。实现完成后可用：

```bash
cd pipeline/scheduler
uv run dg launch --job example__portfolio_live_job
```

该 job 不挂 schedule。需要进入 CI 时，也应由 CI 显式运行同一个 job，而不是让生产 `portfolio__daily_run_schedule` 自动触发 0051 example。

清算触发仍复用正式 daily-run 机制：Dagster 不直接投递 NATS，也不直接调用 worker。Dagster 只调用 Rearview daily-run API；由 Rearview 创建 daily run/outbox，再由现有 outbox dispatcher 和 worker 完成异步清算。

## 单一控制入口

0051 example 的执行选择应收敛到一个入口：

```text
example__portfolio_live_job
```

控制原则：

1. Definitions 可以长期注册该 job，但注册 definitions 不应产生任何 portfolio、daily run 或 live facts。
2. 只有显式 launch `example__portfolio_live_job` 时才执行 0051 用例。
3. `portfolio__daily_run_schedule`、`strategy_portfolio__daily_run_job` 和其他生产日常入口不得隐式包含 0051 example。
4. CI 若需要可选执行，应在 CI 层用一个明确 gate 控制是否 launch 该 job，例如只在手动 workflow、nightly job 或显式变量打开时运行。
5. 不建议在生产 daily job 内增加 `run_0051_example=true/false` 这类业务开关；这会让 example 用例混入生产调度语义。
6. 若未来需要 `validate_only` 或 `ensure_only`，也应作为 `example__portfolio_live_job` 的显式 run config 模式，而不是新增第二个入口。

因此，用户可以灵活选择是否执行：不 launch 就不执行；launch `example__portfolio_live_job` 就执行完整 ensure + daily run + 清算验收。这个 job 是唯一被支持的外部控制面。

## 等价性定义

本 RFC 要保证的是后端业务等价，不是 UI 行为等价。

| 阶段 | 等价条件 |
|---|---|
| 输入映射 | 0051 data config 能生成与前端同配置一致的 Rearview request DTO |
| 规范化 | 两个入口调用同一套 validation/canonicalization/default expansion |
| 控制面 | 持久化后的 `rule_snapshot`、`execution_config`、benchmark、dates 和 hashes 一致 |
| daily run | 两个入口创建的 portfolio 都通过正式 daily-run API 运行 |
| 信号 | worker 从 portfolio snapshot 重新编译规则并查询 mart |
| 模拟 | 使用同一个 `simulate_portfolio` 实现 |
| 清算 | 使用同一个 live facts 写入路径 |

允许差异：

1. `case_id`、`fixture_hash`、source/created-by metadata。
2. portfolio id/code、daily run id、result attempt id。
3. Dagster materialization metadata。
4. 创建时间、审计字段和幂等 request id。

不允许差异：

1. canonical `rule_snapshot`。
2. canonical `execution_config`。
3. rule hash 和 execution config hash。
4. `live_start_date` / `initial_signal_date` 语义。
5. worker 输入来源。
6. 股票池、TopN 信号、订单、成交、持仓、净值和 live facts 的计算路径。

## 清算作业流程

### 1. Ensure Portfolio

`example__portfolio_live_job` 调用 Rearview example ensure API，拿到真实 strategy portfolio id 和 canonical hashes。Dagster 只核对返回值，不解释配置。

### 2. Create Daily Run

Dagster 对该 portfolio 调用正式 strategy portfolio daily-run range API：

```text
start_date = 2024-01-02
end_date = 2024-01-02
strategy_portfolio_id = ensured portfolio id
```

`2024-01-02` 是本 RFC 固定的 live 建仓日 fixture。若缺少交易日历、行情、指标或 benchmark，应 fail fast，不自动漂移到其他日期。若该日没有可执行买入信号，仍允许创建 portfolio 并完成空仓 daily run；组合保持现金仓位，后续 daily run 遇到 T 日买入信号后在 T+1 开盘买入。

### 3. Worker Recomputes Pool And Signals

Worker 从 portfolio 的 `rule_snapshot` 重新编译 0051 规则，在 `run_start_date..trade_date` 查询 mart 指标：

```text
trade_date 指标行
  -> Step 1 AND 过滤
  -> Step 2 conditional points 评分
  -> score / rank / security_code 排序
  -> 每日候选股票池
  -> buy_signal_top_n = 5
  -> signal_date / execution_date / rank / score
```

信号约束：

1. `execution_date` 必须晚于 `signal_date`。
2. 没有下一交易日的信号必须丢弃并计入 dropped reason。
3. `execution_date > trade_date` 的信号不能进入当前 daily run。
4. 同一执行日的信号按 `rank` 排序；同分排序应保持 deterministic。
5. 当前已持仓的证券不能重复买入。
6. 没有买入信号不是 daily run 失败条件；worker 应产出现金空仓 nav，并在 signal summary 中记录 TopN/可执行信号计数为 0。

### 4. Portfolio Simulation

Portfolio simulation 使用同一个 `simulate_portfolio` 实现。需要固化的语义：

1. 交易日循环中先处理当天 pending sells，再处理当天 buy signals。
2. 卖出条件在收盘后评估，满足风控后排队到下一交易日执行。
3. 卖出使用执行日 `open_price_backward_adj * (1 - sell_bps / 10000)`。
4. 买入使用执行日 `open_price_backward_adj * (1 + buy_bps / 10000)`。
5. 卖出释放的现金和仓位可用于同一天后续买入。
6. 买入数量受单票目标权重、现金、lot size、min lots 和空闲仓位约束。
7. 每日收盘后用 close price 估值，产出 position day 和 nav row。
8. 无持仓、无信号的交易日仍产出现金 nav row。
9. live output 从 `live_start_date = 2024-01-02` 开始归一化，之前仅作为信号和持仓计算上下文。

因此，买入信号和卖出信号同日存在不冲突；系统语义是“当日先卖出、再买入补位”。

### 5. Settlement

Worker 在 simulation 输出后：

1. 生成新的 `result_attempt_id`。
2. 写入 `fleur_portfolio.live_nav_daily`、`live_target`、`live_order`、`live_trade`、`live_position_day`、`live_event` 等 portfolio live facts。
3. 写入 `live_closed_trade`、performance metric、trade metric 等 calculation facts。
4. 更新 strategy portfolio 的 `current_live_result_attempt_id`。
5. 将 daily run 标记为 `succeeded`。

`example__portfolio_live_job` 必须等待这一阶段完成，并查询 fact counts。只发起 request、只创建 daily run 记录或只拿到 queued/running 状态都不算成功。

## Dagster Example Job

建议新增手动 example job：

```text
example__portfolio_live_job
```

建议实现形态：独立 unpartitioned example asset + asset job，避免污染正式 `rearview/strategy_portfolio_daily_runs` 的 `2026-06-24` 起始分区语义。

第一版职责：

1. 调用 Rearview example ensure API。
2. 验证返回的 `case_id`、`fixture_hash`、rule hash、execution config hash 和 `live_start_date = 2024-01-02`。
3. 对该 portfolio 调用 Rearview daily-run range API，默认 `start_date=end_date=2024-01-02`；需要验证后续信号买入时，通过同一个 job 的 run config 设置更晚的 `end_date`。
4. 等待 worker 完成。
5. 查询 daily-run status、signal summary 和 fact counts。
6. 在 materialization metadata 中记录 portfolio id、fixture hash、live start date、daily run id、status、signal summary 摘要和 live fact counts。

若 `2024-01-02` 单日没有买入信号，job 仍应成功，metadata 中记录空 signal summary 和现金 nav counts。若需要证明“后续有 T 日信号后 T+1 买入”，仍通过同一个 `example__portfolio_live_job` 的显式 run config 扩展清算区间，不新增第二个入口。

非目标：

1. 不替代 `strategy_portfolio__daily_run_job` 或 `portfolio__daily_run_schedule`。
2. 不对所有 active portfolio 执行 daily run。
3. 不依赖 2026-06-24 历史 portfolio id、result attempt 或 portfolio code。
4. 不挂 schedule；需要时通过 `uv run dg launch` 或 CI 显式触发。
5. 不直接构建 Dashboard、detail、statement 或 rebalance-records read model payload。
6. 不设计 Racingline 前端录入、浏览器自动化、截图或页面跳转验收。

## 验收不变量

### 配置与等价性

1. 0051 data config 有稳定 `case_id`、`version` 和 `fixture_hash`。
2. Step 1 的 10 条过滤条件保持 canonical 字段和 AND 语义。
3. Step 2 包含 7 条评分条件，`n_structure_20_is_valid = true` 位于 Step 2，命中加 `+20`。
4. KDJ 两段评分互斥。
5. `buy_signal_top_n = 5` 和 `max_positions = 5` canonical 后仍独立。
6. `planned_live_start_date` 固定为 `2024-01-02`。
7. example ensure 产出的 canonical snapshot 与前端同配置 request 产出的 canonical snapshot 一致。

### 运行和加工

1. 股票池由 0051 rule/config 在 mart 指标上计算，不读取历史 2026 result attempt。
2. TopN 信号由每个信号日的评分排序得到，不由 Dagster 或前端构造。
3. 买入信号执行日是下一交易日开盘。
4. 风控卖出在收盘后判定，下一交易日开盘执行。
5. 同一执行日先处理卖出，再处理买入。
6. 当前持仓证券不能重复买入。
7. 无买入信号时，portfolio 和 daily run 不失败，live facts 至少包含现金空仓 nav。
8. `live_start_date` 之前的数据只作为上下文，live nav 和 read model 从 `2024-01-02` 开始。
9. 清算后的 `live_*` facts 是 read model 的权威事实源。

### Dagster 验收

1. `example__portfolio_live_job` definitions 必须能加载。
2. job 不挂生产 schedule。
3. job 只针对 0051 example portfolio，不影响其他 active portfolios。
4. job 必须等待 Rearview daily run succeeded。
5. job 必须查询目标 run 的 live fact counts。
6. job 不直接写业务 read model payload。

## 建议测试资产

### Parity Test

新增不依赖浏览器的 API/service parity test：

1. 用 0051 data config 生成 Rearview request DTO。
2. 用同一组配置构造“前端会提交的”等价 request DTO。
3. 调用同一套 validation/canonicalization service。
4. 断言 canonical `rule_snapshot`、`execution_config`、rule hash、execution config hash、benchmark 和 date semantics 一致。
5. 调用 example ensure API 后读取 portfolio snapshot，再次断言上述字段一致。

### Contract Fixtures

建议在 Rearview core 或共享 fixture 模块固定：

1. 0051 rule fixture：包含 Step 1 的 10 条过滤条件和 Step 2 的 7 条评分条件。
2. Metric policy fixture：确认所有字段均来自当前 catalog/policy。
3. N 型反转 fixture：`n_structure_20_is_valid = true` 命中 `+20`，false 时不加分。
4. KDJ scoring fixture：`-16` 只得 `+25`，`-12` 只得 `+15`，`-9` 不命中两条 KDJ 加分。
5. Execution config fixture：`buy_signal_top_n = 5`、`max_positions = 5` canonical 后仍独立，`single_position_limit_pct = 0.2` 被保留。
6. Risk exit fixture：只启用固定止盈 `15%` 和收盘价跌破 `price_ma_10` 的指标止损；固定止损和时间止损不启用。
7. Date fixture：`live_start_date = 2024-01-02` 时，`initial_signal_date` 由交易日历解析，不由调用方硬编码。

### Signal/Simulation Fixtures

建议补一个小型 deterministic 行情窗口，覆盖关键执行语义：

1. 同一信号日有超过 5 只候选，最终只生成 `rank <= 5` 的买入信号。
2. 某天已有持仓且无卖出，只能买入剩余空仓数量。
3. 某天先触发 pending sell，再出现买入信号，卖出释放的仓位允许当天买入。
4. 买入和卖出均使用执行日开盘价加/减滑点。
5. 无信号空仓日仍产出现金 nav。
6. `2024-01-02` 作为 live_start_date，之前的信号日只用于建立当天可执行信号，不出现在 live read model 输出中。

### Service Integration

建议建立可控集成测试路径：

1. 使用 seeded PostgreSQL control plane。
2. 使用 seeded ClickHouse market/indicator/benchmark 数据，窗口覆盖少量交易日即可。
3. ensure 0051 example portfolio。
4. 触发 daily run 到 `2024-01-02`。
5. 等待 worker 完成。
6. 读取 daily run status、signal summary 和 fact counts。

必须断言：

| 断言 | 说明 |
|---|---|
| parity hash | example ensure snapshot 与共享 canonicalization service 的 rule/config hash 一致 |
| bootstrap snapshot | portfolio 持有 0051 rule/config 和 `live_start_date = 2024-01-02` |
| daily run status | 最终为 `succeeded` |
| signal summary | 有 compiled SQL hash、required metrics/marts、TopN row count 和 dropped reason |
| live facts | `live_nav_daily` 在目标日期有行，fact counts 可查询 |
| Dagster definitions | definitions 中存在 `example__portfolio_live_job`，且不挂 schedule |

## 不应固化的值

| 值 | 原因 |
|---|---|
| backtest run UUID | 每次运行都会变化 |
| result attempt ULID | append-only attempt 标识，每次运行都会变化 |
| strategy portfolio id/code | ensure 或发布时生成，不稳定 |
| `2025-06-03..2026-06-01` | 历史动态区间，会随数据刷新和 fixture 变化 |
| `2026-06-01` latest available trade date | 真实环境会前进 |
| 具体收益率、净值和持仓列表 | 依赖行情、指标、费用、worker 细节和数据版本；只有 deterministic fixture 才能精确断言 |

例外：`2024-01-02` 是本 RFC 为 `example__portfolio_live_job` 固定的建仓日 fixture，可以成为 example job 的稳定断言。

## 实现缺口

1. Rearview 需要 example ensure API。
2. Rearview 需要抽出或暴露共享 validation/canonicalization/snapshot/persistence service，让前端 publish/create 和 example ensure 复用。
3. Scheduler `RearviewApiResource` 需要新增 example ensure 方法。
4. `strategy_portfolio_daily_runs` 分区起点是 `2026-06-24`，example job 应使用独立 unpartitioned asset/job。
5. daily run metadata/fact-counts 需要暴露足够的 signal materialization 摘要，方便证明股票池和 TopN 信号确实由规则计算而来。

## 待决问题

1. 0051 fixture 应落在 `rearview-core` 的 rule/config 层，还是单独建 `rearview-test-fixtures` 模块供 server、worker 和 scheduler 复用？
2. example ensure API 是否需要强制走 backtest -> publish，还是直接复用抽出的 snapshot/persistence service 创建 portfolio？本 RFC 倾向后者，以避免为了回归用例重跑不必要的 backtest。
3. service integration 是维护专用 seeded ClickHouse 数据集，还是复用开发环境数据并降低断言强度？
4. daily run fact-counts 是否需要追加 signal summary 摘要，还是由 status/progress API 暴露？
5. `example__portfolio_live_job` 第一版只跑 `2024-01-02` 单日，还是同时跑 `2024-01-02..2024-01-05` 短窗口以覆盖建仓后持仓、卖出和再买入？

## 最小验收命令

文档讨论阶段不新增代码。若后续实施该测试固化，建议至少保留以下门禁：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core strategy_backtest
cargo test -p rearview-core portfolio
cargo test -p rearview-server strategy_portfolio
```

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

文档-only 变更继续运行：

```bash
make docs-check
git diff --check
```

## 建议结论

0051 低位反转应被固化为一个后端等价性回归用例，而不是前端操作用例。核心设计是“两种入口，一条 Rearview 业务管道”：data config 和前端同配置 request 进入 Rearview 后复用同一套 service/canonicalization/persistence/worker 逻辑，从而把差异限制在入口来源和审计元数据。

实施优先级建议为：先抽出共享 canonical snapshot/persistence service 和 0051 data config；再补 parity test；随后实现 Rearview example ensure API 与 Dagster `example__portfolio_live_job`；最后补 signal/simulation fixtures 和 service integration。历史运行报告继续作为证据链入口，未来自动化不引用历史 UUID、历史交易日或历史组合 code。
