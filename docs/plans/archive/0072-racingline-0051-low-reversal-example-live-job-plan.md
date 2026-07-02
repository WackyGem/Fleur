# Plan 0072: Racingline 0051 低位反转 example live job 实施计划

日期：2026-07-02

状态：Completed

完成日期：2026-07-02

领域：rearview, dagster, portfolio testing

关联系统：rearview, rearview-portfolio-worker, scheduler

代码根：

- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/scheduler/`
- `pipeline/migrate/`

关联文档：

- [RFC 0044: Racingline 0051 低位反转数据配置与清算回归用例](../../RFC/archive/0044-racingline-0051-low-reversal-regression-case.md)
- [2026-06-24 Racingline Strategy Portfolio Publish Dashboard Dagster](../../jobs/reports/2026-06-24-racingline-strategy-portfolio-publish-dashboard-dagster.md)
- [Plan 0052: Racingline 策略组合发布、看板真实数据与 Dagster 日运行实施计划](0052-racingline-strategy-portfolio-publish-dashboard-dagster-plan.md)
- [Rearview 系统地图](../../architecture/rearview.md)
- [Scheduler Architecture](../../architecture/scheduler-architecture.md)
- [Racingline 系统地图](../../architecture/racingline.md)
- [验收报告：2026-07-02 Racingline 0051 Low Reversal Example Live Job](../../jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md)

## 背景

RFC 0044 的核心结论是：0051 低位反转用例应固化为“数据配置入口 + Rearview 同一套后端业务管道 + Dagster 清算验收”的后端等价回归样例。

本计划不设计 Racingline 前端操作、浏览器自动化、截图或 Dashboard 跳转。实施重点是把前端创建组合背后的 Rearview validation/canonicalization/snapshot/persistence 逻辑抽成共享 service，使前端入口和 0051 data config 入口在进入 Rearview 后走同一套业务路径。Dagster 只提供显式手动入口 `example__portfolio_live_job`，负责调用 Rearview example ensure API、触发正式 daily-run range API、等待 worker 清算并记录 fact counts。

## 目标

1. 固化 0051 低位反转为稳定、可版本化的 data config。
2. Rearview 抽出或复用共享 canonical snapshot/persistence service，使正式 portfolio create/publish 和 0051 example ensure API 调用同一套后端业务逻辑。
3. 新增 Rearview example ensure API，按 `case_id + version + fixture_hash` 幂等创建或复用 example portfolio。
4. 确保 0051 portfolio 的 `planned_live_start_date` / `live_start_date` 固定为 `2024-01-02`；缺少交易日历、行情、指标或 benchmark 时 fail fast，但当天无买入信号时允许空仓建仓，后续 daily run 遇到 T 日信号后在 T+1 开盘买入。
5. 复用正式 strategy portfolio daily-run range API、outbox dispatcher、NATS worker、portfolio simulation 和 live facts 写入路径完成清算。
6. Scheduler 新增唯一外部控制入口 `example__portfolio_live_job`，不挂 schedule，不混入生产 `portfolio__daily_run_schedule`。
7. 通过服务测试、worker/simulation 测试、scheduler definitions 测试和一次 dev smoke 证明配置等价、触发隔离和清算完成。

## 非目标

1. 不实现前端自动录入、Playwright、截图、Dashboard/detail 页面验收。
2. 不让 Dagster 解析规则、计算评分、构造 portfolio snapshot、发布 NATS 或直接调用 worker。
3. 不新增 example 专用股票池、信号、持仓、清算事实表或 read model payload。
4. 不把 0051 example 放入 `strategy_portfolio__daily_run_job`、`portfolio__daily_run_schedule` 或任何生产 daily 入口。
5. 不依赖历史 backtest run UUID、result attempt、portfolio id/code 或 `2025-06-03..2026-06-01` 历史区间。
6. 不把 `run_0051_example=true/false` 这类开关塞进生产 daily job；用户是否执行只由是否 launch `example__portfolio_live_job` 决定。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| Rearview validate | `StrategyBacktestValidateRequest::validate` 会校验 rule、canonicalize execution config，并产出 `rule_hash` 与 `execution_config_hash`。 |
| Execution config | `BacktestExecutionConfig::canonicalized` 会补齐并校验 signal timing、target weighting、empty signal action、slippage 和 `single_position_limit_pct` 等配置。 |
| 现有 publish/create | `POST /rearview/strategy-portfolios` 走正式 portfolio create 路径，但当前语义绑定 source backtest/publish preview，需要先抽出共享 service，避免 example 复制写表逻辑。 |
| Daily run API | Rearview 已有 `POST /rearview/strategy-portfolios/daily-runs/range`、status、fact-counts 和 settlement-target API。 |
| Worker live run | portfolio worker 从 portfolio `rule_snapshot` 重新生成信号，调用 `simulate_portfolio`，再写入 `fleur_portfolio.live_*` 和 calculation facts。 |
| 交易语义 | simulation 按交易日先处理 pending sells，再处理 buy signals；买卖均使用执行日开盘价并应用滑点；卖出释放的现金和仓位可用于同日后续买入。 |
| Scheduler resource | `pipeline/scheduler/src/scheduler/defs/rearview/resources.py` 的 `RearviewApiResource` 目前封装正式 daily-run create/range/status/fact-counts/settlement-target API。 |
| Scheduler asset | `rearview/strategy_portfolio_daily_runs` 是日分区 asset，分区起点为 `2026-06-24`，不适合作为 `2024-01-02` example 建仓日的直接载体。 |
| Scheduler job/schedule | `strategy_portfolio__daily_run_job` 和 `portfolio__daily_run_schedule` 已注册为 portfolio live 独立入口；0051 example 必须独立注册手动 job。 |

## 硬性实施约束

1. 只有一条 Rearview 业务管道：request DTO -> validation/canonicalization -> snapshot/persistence -> daily-run API -> outbox -> worker -> live facts。
2. 0051 data config 不是 canonical snapshot 权威源；canonical `rule_snapshot`、`execution_config`、hash、date semantics 必须由 Rearview shared service 生成。
3. example ensure API 只做入口适配、幂等控制和审计 metadata，不直接手工写 `strategy_portfolio` 或拼 worker 输入。
4. `n_structure_20_is_valid = true` 固定在 Step 2，命中加 `+20`；不得移回 Step 1。
5. KDJ 两段评分互斥：`kdj_j_value < -15` 得 `+25`，`-15 <= kdj_j_value < -10` 得 `+15`。
6. 卖出风控仅启用固定止盈 `15%` 和收盘价跌破 `price_ma_10` 的指标止损；固定止损和时间止损不启用。
7. `buy_signal_top_n = 5`、`max_positions = 5`、`single_position_limit_pct = 0.2` 必须 canonical 后仍独立保留。
8. `2024-01-02` 是唯一固化建仓日 fixture；不能因数据缺失自动漂移到其他日期。
9. `2024-01-02` 当天没有可执行买入信号不是创建失败条件；portfolio 可以先写入空 `pending_buy_signal_snapshot`，worker 必须能产出现金空仓 nav，后续有买入信号时再按 T+1 规则成交。
10. `example__portfolio_live_job` 可以长期注册，但 definitions load 不得产生 portfolio、daily run 或 live facts。

## 实施阶段

### Phase 0: 入口与归属审计

目标：先确认现有 Rearview publish/create 调用链、字段归属和 daily-run API contract，避免在不确定位置写兼容 fallback。

实施项：

1. 追踪 `create_strategy_portfolio`、publish preview、PostgreSQL persistence、daily-run range create、worker settlement-target 的调用链。
2. 确认 `rule_snapshot`、`execution_config`、`initial_signal_date`、`live_start_date`、`pending_buy_signal_snapshot`、hash 和 source metadata 的唯一写入点。
3. 明确 shared service 的输入输出边界：它接收同构 Rearview request DTO 和 date/source metadata，返回 canonical snapshot 与 persistence result。
4. 确认现有 tests 覆盖 publish/create 成功路径和 archived portfolio daily-run 拒绝路径。

测试策略：

1. 本阶段以代码阅读和最小现有测试运行为主，不改变业务行为。
2. 若发现字段归属不唯一，先补小型 characterization test，再进入 Phase 1。

完成标准：

1. 明确哪些逻辑应移动到 shared service，哪些逻辑继续留在 publish/create API handler。
2. 不引入多来源 fallback，不用 example 专用写表绕过现有业务路径。

### Phase 1: Rearview shared canonical snapshot/persistence service

目标：把正式 portfolio create/publish 背后的核心业务逻辑抽成可复用 service，并保持现有前端 publish/create 行为不回归。

实施项：

1. 从现有 create/publish 路径抽出 validation、canonicalization、snapshot builder 和 portfolio persistence 边界。
2. 正式 `POST /rearview/strategy-portfolios` 改为调用 shared service，而不是在 handler 中散落构造和写入逻辑。
3. service 继续使用现有 `StrategyBacktestValidateRequest::validate` 和 `BacktestExecutionConfig::canonicalized`，不新增第二套 default expansion。
4. service 返回 portfolio id/code、rule hash、execution config hash、`initial_signal_date`、`live_start_date` 和必要审计字段。

测试策略：

1. Rust 单测：现有 publish/create allowed 场景的 persisted `rule_snapshot`、`execution_config`、hash 和 dates 不变。
2. Rust 单测：blocked/archived/expected date mismatch 等现有错误语义不变。
3. Rust 单测：shared service 生成的 canonical hash 与 validate response 一致。

完成标准：

1. 正式前端路径和后续 example ensure 路径可以调用同一个 shared service。
2. create/publish handler 只负责 HTTP request/response、source-specific preflight 和错误映射。

### Phase 2: 0051 data config 与 fixture 固化

目标：用稳定 fixture 表达“前端会提交的同一组配置”，但不保存已加工后的 canonical portfolio snapshot。

实施项：

1. 在 Rearview 侧新增 0051 data config，包含 `case_id`、`version`、rule spec、execution config、benchmark、`planned_live_start_date = 2024-01-02` 和 stable `fixture_hash`。
2. data config 转换为正式 Rearview request DTO，字段和结构与前端同配置提交保持同构。
3. 固化 Step 1 的 10 条 AND 过滤条件。
4. 固化 Step 2 的 7 条 conditional points，并确保 `n_structure_20_is_valid = true` 位于 Step 2 加 `+20`。
5. 固化 execution config：`initial_cash = 1000000`、`buy_signal_top_n = 5`、`max_positions = 5`、`single_position_limit_pct = 0.2`、默认市场费率/滑点、固定止盈 `15%`、MA10 指标止损。

测试策略：

1. Rust fixture test：fixture hash 稳定，字段顺序变化不影响 canonical hash。
2. Rust rule test：Step 1 只包含 10 条硬过滤，Step 2 包含 7 条评分项。
3. Rust scoring test：N 型反转 true 得 `+20`，false 不得分。
4. Rust scoring test：KDJ `-16` 只得 `+25`，`-12` 只得 `+15`，`-9` 不得 KDJ 分。
5. Rust config test：只启用固定止盈和 MA10 指标止损，固定止损/时间止损为 disabled 或 absent。

完成标准：

1. 0051 config 能稳定生成 Rearview request DTO。
2. fixture 不包含 portfolio id、daily run id、result attempt 或任何历史运行结果。

### Phase 3: Rearview example ensure API

目标：新增 0051 example 的后端入口适配层，复用 Phase 1 的 shared service，并实现幂等和冲突保护。

建议 API：

```text
POST /rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure
```

实施项：

1. API handler 读取 0051 data config，转换为正常 Rearview request DTO。
2. 调用 shared validation/canonicalization/snapshot/persistence service。
3. 按 `case_id + version + fixture_hash` 查找现有 example portfolio。
4. 已存在且 canonical hash 一致时复用；已存在但 hash 不一致时失败或要求新版本，不静默覆盖。
5. response 返回 `case_id`、`version`、`fixture_hash`、portfolio id/code、rule hash、execution config hash、`initial_signal_date`、`live_start_date`。

测试策略：

1. Rust API/service test：首次 ensure 创建 example portfolio。
2. Rust API/service test：重复 ensure 返回同一 portfolio 且 hash 一致。
3. Rust API/service test：相同 case/version 但 fixture hash 或 canonical hash 不一致时 fail fast。
4. Rust API/service parity test：0051 data config 和等价前端 request 经同一 service 后 canonical snapshot 一致。

完成标准：

1. example ensure API 不直接写业务表，不拼 worker payload。
2. response 足够让 Dagster 验证 case、hash 和 date semantics。

### Phase 4: Worker 清算语义与 signal summary 验收补强

目标：确保 example daily run 的股票池、TopN 信号、交易执行和 live facts 都由正式 worker 路径产出，并有足够 metadata 可验收。

实施项：

1. 确认 live daily run 从 portfolio snapshot 重新编译规则并查询 mart，不读取历史 result attempt。
2. 确认 daily-run status 或 fact-counts response 暴露足够的 signal summary：compiled SQL hash、required metrics/marts、TopN row count、dropped reason 摘要；无可执行信号时 summary 应保留 `top_n_row_count = 0` / `executable_signal_count = 0` 等诊断事实。
3. 若现有 response 不足，扩展 status/fact-counts 的 summary 字段；不要让 Dagster 直接查 ClickHouse 或解析 worker 内部状态。
4. 补强 deterministic simulation fixture，覆盖同日先卖再买、下一交易日开盘价加/减滑点、当前持仓不重复买入、无信号空仓现金 nav、live output 从 live_start_date 过滤。

测试策略：

1. Rust worker/service test：daily run 使用 portfolio `rule_snapshot` 和 `execution_config`。
2. Rust simulation test：卖出条件收盘后判定，下一交易日开盘执行。
3. Rust simulation test：同一执行日先处理 pending sells，再处理 buy signals。
4. Rust simulation test：无信号/无持仓的交易日仍产出现金 nav，不阻断 daily run。
5. Rust simulation test：`live_start_date = 2024-01-02` 之前的数据仅作为上下文，不进入 live output。

完成标准：

1. `example__portfolio_live_job` 可以仅通过 Rearview API 判断 worker 是否完成和 facts 是否写入。
2. example 的空仓、信号、持仓、订单、成交、净值路径与正式 portfolio daily run 一致。

### Phase 5: Scheduler example job 接入

目标：新增唯一外部控制入口 `example__portfolio_live_job`，显式 launch 才运行 0051 example。

实施项：

1. 在 `RearviewApiResource` 增加 example ensure 方法，调用 Phase 3 API。
2. 新增独立 unpartitioned example asset，例如 `rearview/example_0051_portfolio_live_run`；asset 内部只调用 Rearview API，不解析规则。
3. 新增 asset job `example__portfolio_live_job`，只选择该 example asset。
4. asset 执行顺序：
   - 调用 example ensure API。
   - 校验 `case_id`、`fixture_hash`、rule hash、execution config hash、`live_start_date = 2024-01-02`。
   - 调用正式 daily-run range API，默认 `start_date=end_date=2024-01-02` 且传入 ensured portfolio id；若 run config 提供 `end_date`，则从 `2024-01-02` 跑到该日期，用于验证后续 T 日信号在 T+1 买入。
   - 等待 daily run `succeeded`。
   - 查询 fact counts 和 signal summary。
   - 写入 materialization metadata。
5. `build_rearview_defs()` 注册 example asset/job，但不注册 schedule。

测试策略：

1. Python unit test：`RearviewApiResource` 对 ensure API 发出正确 path 和 payload。
2. Python asset test：mock Rearview API 时 asset 按 ensure -> daily-run range -> status -> fact-counts 顺序调用。
3. Python definitions test：存在 `example__portfolio_live_job`，不存在同名 schedule，`portfolio__daily_run_schedule` 不选择 example asset。
4. Dagster definitions check：definitions 能加载。

完成标准：

1. 用户不 launch 时不会产生任何 0051 portfolio、daily run 或 facts。
2. `uv run dg launch --job example__portfolio_live_job` 是唯一支持的外部执行入口；需要扩展清算窗口时使用该 job 的 run config，不新增第二入口。

### Phase 6: Dev smoke 与运行报告

目标：在 dev 环境完成一次真实后端链路验收，并把运行事实记录到 job report。

实施项：

1. 启动 Rearview、portfolio worker、PostgreSQL、ClickHouse 和 NATS dev 依赖。
2. 确认 `2024-01-02` 所需交易日历、行情、指标、benchmark 和 mart 数据可用。
3. 显式执行 `example__portfolio_live_job`。
4. 记录 ensure response、daily run id、status、signal summary、fact counts 和最终 materialization metadata。
5. 新增 `docs/jobs/reports/YYYY-MM-DD-racingline-0051-low-reversal-example-live-job.md`。

建议命令：

```bash
make racingline-dev
```

```bash
cd pipeline/scheduler
uv run dg launch --job example__portfolio_live_job
```

完成标准：

1. daily run 最终为 `succeeded`。
2. `live_nav_daily` 至少在目标 run 上有行。
3. fact counts 和 signal summary 可由 Rearview API 查询。
4. 运行报告说明命令、环境、日期、portfolio id、daily run id、hash、counts 和任何数据缺口。

### Phase 7: 文档与架构收敛

目标：把实施完成后的入口、边界和验收事实写回当前文档，避免后续误把 example 混入生产 daily。

实施项：

1. 更新 [Rearview 系统地图](../../architecture/rearview.md)，记录 example ensure API 和 shared service 边界。
2. 更新 [Scheduler Architecture](../../architecture/scheduler-architecture.md)，记录 `example__portfolio_live_job` 是手动 example 入口且不挂 schedule。
3. 视实施结果更新 RFC 0044 状态或补充已接受约束。
4. 完成后将本计划状态改为 `Completed` 并归档到 `docs/plans/archive/`，同步 `docs/plans/README.md`。

完成标准：

1. 当前架构文档能导航到 example job、Rearview API 和验收报告。
2. active plan 索引不保留已完成计划。

## 最小验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

Rust 实施阶段：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Scheduler 实施阶段：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

Dev smoke 阶段：

```bash
make racingline-dev
```

```bash
cd pipeline/scheduler
uv run dg launch --job example__portfolio_live_job
```

## 完成标准

1. 0051 data config 固化，且 rule/config/hash/date 的 parity test 通过。
2. 前端 publish/create 和 example ensure 复用同一个 Rearview shared canonical snapshot/persistence service。
3. example ensure API 幂等，冲突 fail fast，response 包含 Dagster 验收所需字段。
4. `example__portfolio_live_job` definitions 能加载，不挂 schedule，不被生产 daily job/schedule 选择。
5. job 显式 launch 后能 ensure portfolio、创建 `2024-01-02` daily run、等待 worker succeeded 并查询 fact counts；若当天无买入信号，验收现金空仓 nav 和 signal summary，而不是失败。
6. Dagster 不解析规则、不计算信号、不构造 snapshot、不写业务 read model。
7. dev smoke 报告已记录真实运行命令、hash、daily run id、status、signal summary 和 fact counts。

## 风险与处理

| 风险 | 处理 |
|---|---|
| 现有 publish/create 与 backtest result attempt 耦合较深 | 先抽 shared service，保留 publish-specific source 校验在 handler；example 不复制表写入。 |
| `2024-01-02` 无买入信号 | 允许创建 portfolio 和空仓 daily run，报告记录 `top_n_row_count = 0` / `executable_signal_count = 0`；后续 daily run 遇到 T 日信号后 T+1 买入，不自动改建仓日。 |
| `2024-01-02` 数据不完整 | 缺少交易日历、行情、指标或 benchmark 时 fail fast 并在报告中列明缺口；不自动改日期。 |
| signal summary 暴露不足 | 扩展 Rearview status/fact-counts response，而不是让 Dagster 查内部表。 |
| example 被误挂生产 schedule | definitions test 明确断言 `example__portfolio_live_job` 无 schedule，生产 schedule selection 不包含 example asset。 |
| fixture hash 因 JSON 顺序或默认值漂移 | 使用 canonical JSON/hash 测试固定，默认值只由 shared canonicalization service 补齐。 |
