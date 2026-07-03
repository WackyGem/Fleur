# RFC 0045: Strategy Portfolio 日度 NAV 清算入口收敛

状态：Implemented（Plan 0073，2026-07-02）
日期：2026-07-02
领域：Dagster, Rearview, Strategy Portfolio, Portfolio Live Facts
关联系统：pipeline/scheduler, engines/crates/rearview-core, engines/crates/rearview-portfolio-worker, fleur_portfolio
相关文档：
- docs/RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md
- docs/RFC/archive/0036-racingline-strategy-portfolio-statement.md
- docs/RFC/archive/0044-racingline-0051-low-reversal-regression-case.md
- docs/plans/archive/0062-racingline-strategy-portfolio-statement-plan.md
- docs/plans/archive/0072-racingline-0051-low-reversal-example-live-job-plan.md
- docs/plans/archive/0073-strategy-portfolio-daily-nav-liquidation-plan.md
- docs/jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md
- docs/jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md
- docs/jobs/reports/2026-07-02-strategy-portfolio-daily-nav-liquidation.md
- docs/architecture/scheduler-architecture.md
- docs/architecture/rearview.md
- docs/architecture/data-platform.md

## 摘要

实施前 Dagster 可执行资产 `rearview/strategy_portfolio_daily_runs` 的名字和配置仍以“创建 daily run”为中心。Plan 0073 已把它收敛为一个以结果语义命名的日度组合清算入口：

```text
rearview/strategy_portfolio_daily_runs
  -> rearview/daily__portfolio_nav_liquidation
```

目标语义：

1. 生产日常入口不要求用户填写 `trade_date`、`start_date` 或 `end_date`。
2. 当前版本只由 Dagster 调度，不新增 Rearview 批处理清算 API。
3. Portfolio live 进入 `daily__fetch_history_sources_to_marts_schedule_job`，作为 source/raw/dbt/Furnace/marts 成功后的终端阶段。
4. 终端阶段调用现有 Rearview settlement target API 解析最近可清算交易日。
5. Dagster 调用现有 Rearview daily-run API，为所有 eligible active portfolios 创建或复用一个目标交易日 daily run。
6. worker 以 portfolio 持久化上下文执行全窗口计算，NAV 输出覆盖用户可见建仓日到最近可清算交易日。
7. Dagster materialization 成功必须代表 worker 已到终态，并且 ClickHouse `fleur_portfolio.live_*` facts 已可核验。

本文档记录设计方向和缺口；实现、验证和运行证据见 Plan 0073 与 2026-07-02 运行报告。

## 实施结果

Plan 0073 已完成以下落地：

1. `rearview/daily__portfolio_nav_liquidation` 已注册为 executable unpartitioned asset。
2. 旧 `rearview/strategy_portfolio_daily_runs` 不再作为 production asset key 注册。
3. 旧 `strategy_portfolio__daily_run_job` 和 `portfolio__daily_run_schedule` 不再 registered。
4. `daily__fetch_history_sources_to_marts_schedule_job` 在 `all_source_to_marts + full` plan 末尾追加 portfolio live terminal step。
5. Production asset 默认路径只调用现有 Rearview settlement-target、single-day daily-runs、status 和 fact-count APIs。
6. `example__portfolio_live_job` 仍保留为 0051 手动回归入口。

## 本次修订决策

本 RFC 采纳以下范围约束：

| 决策 | 结论 |
|---|---|
| Rearview 批处理清算 API | 不采纳当前版本新增 `nav-liquidations/daily` 之类 API；继续复用现有 `settlement-target`、`daily-runs`、status 和 fact-counts APIs。 |
| 调度归属 | 当前版本只考虑由 Dagster 调度组合 NAV 清算；portfolio live 进入 `daily__fetch_history_sources_to_marts_schedule_job`。Rearview 仍负责已有 control plane、outbox、worker 和 live facts 写入。 |
| Dagster asset 分区 | `daily__portfolio_nav_liquidation` 改为无分区结果资产。 |
| 计算语义 | 组合净值和相关指标每次都按 full-window 全量重跑，不按 Dagster partition 做增量写入。 |
| 生产 schedule | 不保留独立 `portfolio__daily_run_schedule` / `portfolio__daily_nav_liquidation_schedule`；生产触发由 `daily__fetch_history_sources_to_marts_schedule` 统一负责。 |

无分区不是单纯为了简化配置，而是为了匹配真实计算语义：一次 portfolio live 清算会从 portfolio 的持久化上下文重新生成全窗口账本，再写出最新 attempt 的 live facts。Dagster materialization 表示“当前最新全量清算结果可用”，不是“某个日期分区被增量写入完成”。

## 命名结论

采用 `daily__portfolio_nav_liquidation` 作为 Dagster 资产名。

建议落地命名：

| 对象 | 当前 | 建议 |
|---|---|---|
| Dagster asset key | `rearview/strategy_portfolio_daily_runs` | `rearview/daily__portfolio_nav_liquidation` |
| Python asset function | `strategy_portfolio_daily_runs` | `daily__portfolio_nav_liquidation` |
| Asset config | `StrategyPortfolioDailyRunConfig` | `DailyPortfolioNavLiquidationConfig` |
| Production job surface | `strategy_portfolio__daily_run_job` 独立入口 | 不保留独立 production job surface；由 `daily__fetch_history_sources_to_marts_schedule_job` 的终端步骤提交 |
| Production schedule surface | `portfolio__daily_run_schedule` | 不保留独立 production schedule；由 `daily__fetch_history_sources_to_marts_schedule` 统一触发 |

`daily__` 前缀表达它属于每日生产链路；`portfolio_nav_liquidation` 表达业务结果是组合 NAV 清算，而不是单纯创建一批 Rearview run records。

## 术语边界

本文中的“建仓日”指用户和 Racingline 页面看到的 `strategy_portfolio.live_start_date`，也就是 live NAV 归一化和展示的第一天。

当前 worker 仍需要从 `strategy_portfolio.initial_signal_date` 开始编译信号和加载价格，因为系统采用 T 日信号、T+1 开盘成交语义。这个技术窗口不应暴露为用户需要填写的日期范围：

| 字段 | 角色 |
|---|---|
| `initial_signal_date` | worker 计算窗口起点，用于保留 T -> T+1 信号和成交语义 |
| `live_start_date` | 用户可见建仓日，live NAV 输出和详情页 read model 的起点 |
| `trade_date` | 本次清算目标交易日，由系统解析最近可清算交易日得到 |

因此“从建仓日开始清算到最近交易日”的可验收结果是：最新成功 attempt 的 `live_nav_daily` 覆盖 `live_start_date..target_trade_date`；worker 内部可以继续以 `initial_signal_date..target_trade_date` 作为模拟输入窗口。

## 实施前事实基线

### Scheduler

实施前注册的可执行资产是 `rearview/strategy_portfolio_daily_runs`。`dg list defs --assets "key:rearview/strategy_portfolio_daily_runs" --json` 返回该 asset `group_name = rearview`、`is_executable = true`，tags 包含 `source=rearview`、`layer=control_plane`、`storage=postgres_clickhouse`、`state=async_worker` 和 `modality=strategy_portfolio`。

代码事实：

| 区域 | 当前事实 |
|---|---|
| Asset key | [assets.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/assets.py) 定义 `STRATEGY_PORTFOLIO_DAILY_ASSET_KEY = AssetKey(["rearview", "strategy_portfolio_daily_runs"])`。 |
| Partitions | 同文件定义 `DailyPartitionsDefinition(start_date="2026-06-24")`。 |
| Config | `StrategyPortfolioDailyRunConfig` 暴露 `trade_date`、`start_date`、`end_date`、`strategy_portfolio_id`、等待和 chunk 配置。 |
| 默认请求 | 未传日期时，asset 调用 `get_strategy_portfolio_settlement_target()`，再把 target date 作为 `start_date=end_date` 调用 range API。 |
| Range 请求 | 传 `start_date/end_date` 时，scheduler 会按 `chunk_size` 拆分自然日期区间，逐段调用 Rearview range API。 |
| 成功语义 | asset 已等待 daily run status 到终态，并查询 fact-counts；`nav_row_count <= 0` 会使 Dagster run fail。 |
| Job/Schedule | [definitions.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/definitions.py) 注册 `strategy_portfolio__daily_run_job` 和每日 20:00 的 `portfolio__daily_run_schedule`。 |
| Daily source-to-marts | [source_to_marts.py](../../../pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py) 当前的 `daily__fetch_history_sources_to_marts_schedule_job` 只展开 source/raw/dbt/Furnace/mart asset materialization steps，尚未包含 portfolio live 终端步骤。 |
| 0051 example | 同一模块另有 `rearview/example_0051_portfolio_live_run`，手动 job 默认只创建一个 latest target 的 full-window daily run，不挂 schedule。 |

### Rearview API

实施前和当前 Rearview 均暴露以下 strategy portfolio daily run API：

| API | 当前角色 |
|---|---|
| `POST /rearview/strategy-portfolios/daily-runs` | 按单个 `trade_date` 创建或复用 daily runs。 |
| `POST /rearview/strategy-portfolios/daily-runs/range` | 先查询交易日历，再对每个 resolved trade date 调用单日创建逻辑。 |
| `GET /rearview/strategy-portfolios/daily-runs/settlement-target` | 按 active portfolios 或指定 portfolio 的依赖数据最新日期解析 settlement target。 |
| `GET /rearview/strategy-portfolios/daily-runs/{id}` | 返回 daily run status。 |
| `GET /rearview/strategy-portfolios/daily-runs/{id}/fact-counts` | 查询最新 attempt 的 `live_nav_daily`、`live_trade` 和 `live_closed_trade` 行数。 |

[postgres/mod.rs](../../../engines/crates/rearview-core/src/postgres/mod.rs) 的 `create_strategy_portfolio_daily_runs_for_trade_date()` 当前行为：

1. 未指定 `strategy_portfolio_id` 时读取所有 active portfolios。
2. 指定 archived portfolio 时返回 `410 Gone`。
3. 只为 `portfolio.live_start_date <= trade_date` 的组合创建或复用 daily run。
4. 插入 `strategy_portfolio_daily_run` 时写入 `run_start_date = portfolio.initial_signal_date`、`trade_date = target`。
5. 通过唯一约束 `(strategy_portfolio_id, trade_date)` 保持幂等。
6. 新创建的 daily run 写入 outbox，供 dispatcher 发布 NATS task。

`finalize_strategy_portfolio_daily_run_to_clickhouse()` 已避免较早 trade date 的后完成 run 覆盖较晚 latest pointer：只有当当前 portfolio 没有 latest run，或已有 latest run 的 `trade_date <= 本次 trade_date` 时，才更新 `latest_daily_run_id` 和 `current_live_result_attempt_id`。

### Worker

[rearview-portfolio-worker](../../../engines/crates/rearview-portfolio-worker/src/main.rs) 的 strategy portfolio daily run 处理路径当前已经是 full-window 计算：

1. 读取 `strategy_portfolio` 的 `rule_snapshot` 和 `execution_config`。
2. 查询 `run.run_start_date..run.trade_date` 的交易日和 TopN 信号。
3. 查询同一窗口的价格和风控指标。
4. 调用 `simulate_portfolio()` 得到完整账本。
5. 用 `portfolio.live_start_date` 归一化并过滤 live output。
6. 写入 `fleur_portfolio.live_*` 和相关 calculation facts。

这说明“清算到最近交易日”不需要为历史上的每个交易日都创建 daily run。只要创建目标交易日的 daily run，worker 就会从持久化上下文重算全窗口，并产出从 `live_start_date` 开始的 live facts。

### 已有验收事实

[2026-06-29 对账单验收报告](../../jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md) 已证明：一个 latest daily run 可以产出从 `2025-01-08` 到 `2026-06-26` 的 `live_nav_daily`，并由 Dagster metadata 记录 `latest_daily_run_id`、`latest_result_attempt_id`、`nav_row_count`、`trade_row_count` 和 `closed_trade_row_count`。

[2026-07-02 0051 example 报告](../../jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md) 进一步确认：`example__portfolio_live_job` 默认解析 portfolio-specific settlement target，然后只创建一个 `trade_date = latest settlement target` 的 daily run，由 worker 一次性执行 full-window simulation。

## 设计预期

### 1. 生产入口无日期参数

`daily__portfolio_nav_liquidation` 的生产 config 不应包含：

- `trade_date`
- `start_date`
- `end_date`
- `strategy_portfolio_id`

保留的 config 应仅用于执行策略：

| 字段 | 作用 |
|---|---|
| `wait_for_completion` | 是否等待 worker 终态；生产默认 `true` |
| `poll_interval_seconds` | status polling 间隔 |
| `timeout_seconds` | 总等待超时 |

如果后续仍需要修复历史区间或指定单个 portfolio，应提供单独的 admin/backfill 入口，而不是复用生产日常 asset。

### 2. Portfolio Live 进入 Daily Source-to-Marts Job

`daily__fetch_history_sources_to_marts_schedule_job` 成为唯一生产日常入口。Portfolio live 清算作为该 controller 的终端阶段执行，顺序为：

```text
daily__fetch_history_sources_to_marts_schedule_job
  -> source/raw steps
  -> dbt staging/intermediate/marts steps
  -> Furnace calculation steps
  -> daily__portfolio_nav_liquidation terminal step
```

终端步骤只在前置 source-to-marts plan 全部成功后执行。若前置步骤失败，portfolio live 不应启动，避免用未完成的 marts/calculation 依赖清算组合 NAV。

`dry_run=true` 时，daily plan 应展示 portfolio live terminal step，但不提交 Rearview daily runs。`dry_run=false` 时，terminal step 作为同一个 daily controller run 的一部分提交并等待完成；portfolio live 失败或超时应让 daily controller run 失败。

### 3. Dagster 调度现有 Rearview APIs

“最近交易日”应解释为“最近可清算交易日”，不是当前日历上的最近交易日。它至少受以下依赖约束：

- `mart_trade_calendar`
- `mart_stock_quotes_daily`
- portfolio `required_marts`
- portfolio benchmark security returns
- risk-free rate

当前版本不新增 Rearview 批处理清算 API。Dagster 直接编排现有 Rearview APIs：

```text
daily__fetch_history_sources_to_marts_schedule_job terminal step
  -> GET /rearview/strategy-portfolios/daily-runs/settlement-target
  -> POST /rearview/strategy-portfolios/daily-runs
  -> GET /rearview/strategy-portfolios/daily-runs/{id}
  -> GET /rearview/strategy-portfolios/daily-runs/{id}/fact-counts
  -> MaterializeResult metadata
```

Scheduler 不直接读取 ClickHouse 或 PostgreSQL，也不复制 settlement target 依赖探测逻辑。active portfolio 枚举、archived 过滤、daily run 幂等创建和 outbox 分发继续由现有 Rearview daily-run API 负责。

当前版本接受现有全局 settlement target 语义：它按所有 active portfolios 的依赖数据共同可用上限解析一个目标交易日。这个语义偏保守，任一 active portfolio 的依赖滞后可能压低本次整体清算目标；但它避免为本轮更名和无分区改造引入新的 Rearview API surface。若后续需要 per-portfolio target 互不阻塞，应另起 RFC 或 plan，不放入本轮。

### 4. 每个组合每次只创建一个目标 daily run

新入口不应把 `live_start_date..target_trade_date` 展开为 N 个 daily runs。那是历史 range/backfill 验收工具的行为，不是生产日度 NAV 清算语义。

推荐行为：

```text
target_trade_date = resolve_global_settlement_target()
if target_trade_date is empty:
  materialize skipped metadata with dependency evidence
else:
  create_or_reuse daily_runs_for_all_active_portfolios(target_trade_date)
  wait for returned daily runs
  verify live fact counts and nav coverage
```

底层继续使用 `strategy_portfolio_daily_run` 表、outbox 和 worker task。要变的是 Dagster asset 的外部语义、分区模型和默认请求路径，不是立即重命名所有内部表或新增 Rearview 清算 API。

### 5. 成功语义以 live facts 覆盖为准

Materialization metadata 至少应包含：

| Metadata | 含义 |
|---|---|
| `active_portfolio_count` | Rearview daily-run API 返回的 eligible active portfolio 数 |
| `created_run_count` | 新创建 daily runs 数 |
| `skipped_run_count` | 已存在同 portfolio/date run 而跳过创建的数量 |
| `succeeded_run_count` | worker succeeded 数 |
| `failed_run_count` | worker failed/cancelled 数 |
| `timeout_run_count` | 超时未终态数 |
| `target_trade_date` | 本次全局 settlement target |
| `daily_run_results` | 每个 returned daily run 的 portfolio、daily_run_id、status、attempt、nav coverage、fact counts 和 skip reason |

对每个 succeeded daily run，fact verification 不应只检查行数大于 0，还应优先检查：

- `live_nav_daily` 最小日期等于 portfolio `live_start_date`。
- `live_nav_daily` 最大日期等于 target trade date。
- `result_attempt_id` 与 daily run `current_result_attempt_id` 一致。

如果某个 portfolio 一直 pending 或 failed，生产 asset 应 fail，而不是把部分成功伪装为整体成功。是否允许部分成功需要另起设计，明确重试和可见性策略。

## 实施前实现与设计预期的缺口

| 缺口 | 当前实现 | 设计预期 | 影响 |
|---|---|---|---|
| 资产命名 | `strategy_portfolio_daily_runs` 强调创建 run records | `daily__portfolio_nav_liquidation` 强调 NAV 清算结果 | Dagster UI 和文档难以看出该入口负责 live NAV 事实完成 |
| 日期输入 | Config 暴露 `trade_date/start_date/end_date` | 生产入口不需要用户输入日期范围 | 用户容易把 daily asset 当成手动区间回填工具 |
| 分区语义 | 使用 `DailyPartitionsDefinition(start_date="2026-06-24")` | 无分区结果资产；schedule 只决定触发时间，target date 写入 metadata | 当前 partition key 容易被误解为清算交易日，但组合 NAV 实际是全量重跑结果 |
| Range 行为 | 区间被展开为多个 trade dates 和多个 daily runs | 每个组合每次只需要 latest target 的一个 full-window run | 长区间会制造不必要 daily runs，增加 worker 和 ClickHouse 写入成本 |
| 组合范围 | Config 可填 `strategy_portfolio_id` | 生产日常入口覆盖所有 active portfolios | 生产语义和手动诊断语义混在同一个 asset |
| Settlement 粒度 | 默认调用全局 settlement target | 当前版本继续使用全局 target，不新增 per-portfolio batch API | 语义保守，可能因单个组合依赖滞后压低整体 target；这是本轮接受的范围限制 |
| Rearview API 语义 | 公开 API 名仍是 `daily-runs` 和 `range` | 当前版本继续复用现有 APIs，由 Dagster 编排清算流程 | 实现面更小，但 Scheduler 仍需要理解 daily-run status 和 fact-count evidence |
| Daily network 归属 | `strategy_portfolio__daily_run_job` / `portfolio__daily_run_schedule` 独立于 `daily__fetch_history_sources_to_marts_schedule_job` | portfolio live 是 `daily__fetch_history_sources_to_marts_schedule_job` 的终端阶段 | 当前生产 daily network 不能保证 portfolio live 在 marts/calculation 成功后再运行 |
| Metadata 粒度 | 顶层暴露 latest run 的 fact counts，详细 statuses/fact counts 存 JSON | 每个 returned daily run 都有结构化 coverage 和 fact-count evidence | 多组合场景下难以快速定位哪个组合失败或落后 |
| 文档索引 | architecture 和 RFC 0040 仍引用旧 asset/job/schedule | 文档应引用新 asset/job/schedule，并说明旧 key retired | 后续 agent 或开发者容易沿旧命名继续扩散 |
| 测试断言 | integration/unit tests 明确断言旧 asset key 存在或被 daily source-to-marts 排除 | 测试应断言新 key 存在，并作为 daily source-to-marts terminal step 进入 daily plan | 重命名和 daily network 归属变化会导致现有测试失败，需要计划内更新 |

## 推荐方案

### Scheduler-only Orchestration

当前版本不新增 Rearview 批处理清算 API。`daily__portfolio_nav_liquidation` 由 `daily__fetch_history_sources_to_marts_schedule_job` 的终端阶段直接复用现有 Rearview APIs：

1. 调用 `get_strategy_portfolio_settlement_target()` 解析全局最近可清算交易日。
2. 若 target 为空，返回 skipped materialization metadata，记录依赖缺口。
3. 若 target 非空，调用现有 `create_strategy_portfolio_daily_runs(trade_date=target)`，不走 range 展开。
4. 对返回的 `daily_run_ids` 轮询 status。
5. 对 succeeded daily runs 查询 fact-counts，并在可用时追加 nav coverage 验证。
6. 输出 `MaterializeResult` metadata。

这个方案继续让 Rearview 拥有 portfolio 枚举、archived 过滤、daily run 幂等创建、outbox 和 worker 执行。Scheduler 的职责是统一 daily 入口、前置依赖顺序、等待、失败传播和结果证据记录。

### Scheduler 改为无分区结果资产

`daily__portfolio_nav_liquidation` 改为 unpartitioned asset，由 `daily__fetch_history_sources_to_marts_schedule_job` 在 terminal step 中提交。实际 target trade date 写入 metadata。

理由：

1. 组合 NAV 清算不是按 Dagster partition 增量写入一个日期切片。
2. 每次 live 清算都会从 portfolio 持久化上下文和 `initial_signal_date..target_trade_date` 重新生成 full-window signals、prices、orders、trades、positions、NAV 和相关指标。
3. latest attempt 会替代当前组合的 live result pointer；用户和 read model 消费的是最新全量结果。
4. Dagster partition key 容易被误解为业务清算日期，但实际 target date 应由 settlement resolver 决定。

如果未来需要追溯某次日常调度，可依赖 Dagster run 时间、run id、daily controller tags、materialization metadata 和 job report，而不是用 asset partition 表达业务日期。

### 保留历史修复入口但移出生产 asset

现有 range API 对验收和历史修复仍有价值，但不应继续作为生产 daily asset 的第一层用户接口。

建议后续如果需要：

| 入口 | 角色 |
|---|---|
| `daily__fetch_history_sources_to_marts_schedule_job` | 唯一生产日常入口，最后提交 portfolio live liquidation terminal step |
| `daily__portfolio_nav_liquidation` | 无分区结果资产，无日期参数，覆盖 active portfolios latest target |
| `manual__portfolio_nav_liquidation_repair_job` | 管理员显式修复工具，允许指定 portfolio 和日期范围 |
| `example__portfolio_live_job` | 0051 regression 手动入口，继续隔离，不挂生产 schedule |

## 后续迭代阶段

### Phase 1: Rename Scheduler Surface

目标：先让 Dagster surface 与目标业务语义一致，并移除 partition 误导。

实施项：

1. 重命名 asset key、Python function 和 config class。
2. 移除生产 config 中的 `trade_date/start_date/end_date/strategy_portfolio_id`。
3. 移除 `DailyPartitionsDefinition` 和 `build_schedule_from_partitioned_job` 依赖。
4. 将旧 key 的测试断言迁移到 `rearview/daily__portfolio_nav_liquidation`。
5. 更新 `docs/architecture/scheduler-architecture.md`、`docs/architecture/data-platform.md` 和 `docs/RFC/archive/0040-dagster-stg-to-mart-asset-inventory.md` 的命名。
6. 移除或停用独立 production `portfolio__daily_run_schedule`；保留手动 repair/example 入口需另行命名。

完成标准：

1. `uv run dg list defs --assets "key:rearview/daily__portfolio_nav_liquidation" --json` 能看到新 asset。
2. `rearview/strategy_portfolio_daily_runs` 不再作为 production Dagster asset 注册。
3. 不再注册独立 production `portfolio__daily_nav_liquidation_schedule`。

### Phase 2: Daily Controller Terminal Step

目标：不新增 Rearview API，把 portfolio live 作为 `daily__fetch_history_sources_to_marts_schedule_job` 的 terminal step。

实施项：

1. 在 `DailyPlan` 中增加 portfolio live terminal step，dry-run plan 可见。
2. terminal step 排在 source/raw、dbt 和 Furnace calculation steps 之后。
3. `dry_run=true` 时只输出 terminal step plan，不调用 Rearview。
4. `dry_run=false` 且前置 steps 全部 success 后，提交 `rearview/daily__portfolio_nav_liquidation`。
5. `daily__portfolio_nav_liquidation` 默认调用 `get_strategy_portfolio_settlement_target()`，拿到全局 `settlement_target_date`。
6. target 为空时 materialize skip metadata，不创建 daily run。
7. target 非空时调用 `create_strategy_portfolio_daily_runs(trade_date=target)`，让现有 Rearview API 为所有 eligible active portfolios 创建或复用 daily runs。
8. 默认路径不再调用 range API；range 仅保留给显式 manual repair/backfill 入口。
9. `client_request_id` 使用 Dagster run id 或 daily controller parent run id，保持幂等和审计可追踪。

测试策略：

1. Daily plan unit tests 覆盖 terminal step 在 source-to-marts steps 后出现，且旧排除断言被移除。
2. Scheduler unit tests 覆盖 target unavailable skip、target available create、existing skipped 和 no daily_run_ids。
3. Resource tests 覆盖现有 settlement-target、daily-runs、status 和 fact-counts paths。
4. Integration definitions test 覆盖新 unpartitioned asset 注册，以及独立 production portfolio schedule 不再注册。

### Phase 3: Scheduler Wait And Fact Coverage Verification

目标：让 Dagster 成功严格等于 NAV 清算结果可用。

实施项：

1. Terminal step 调用现有 daily-runs API 后轮询所有 returned daily run IDs。
2. status 进入 `succeeded` 后查询 fact-counts。
3. 当前版本至少保留 `nav_row_count > 0` 作为失败门槛；如需要 nav min/max coverage，可后续扩展现有 fact-counts response 或新增轻量 verification API，另行评估。
4. Scheduler metadata 输出 per-portfolio result table/json。
5. 任一 eligible portfolio failed 或 timeout 时 fail Dagster run。

测试策略：

1. Scheduler unit tests 覆盖 success、failure、timeout、target unavailable、existing skipped。
2. Fact verification tests 覆盖 nav row count 和 attempt id；coverage start/end 只有在现有 fact-counts response 扩展后再纳入。
3. Integration definitions test 覆盖 daily controller job 注册和 portfolio terminal step 计划。

### Phase 4: Docs And Runbook Closure

目标：把新入口变成当前事实。

实施项：

1. 更新 scheduler、data-platform、rearview architecture 文档。
2. 如新增 manual repair job，同步写入 jobs/runbook 或 plans 文档。
3. 新增一次 job report，记录至少一个真实 active portfolio 从 `live_start_date` 清算到 latest target 的 Dagster metadata 和 ClickHouse facts。
4. 若旧 asset key 的 Dagster 历史仍需追溯，在报告中说明 rename 断点。

## 验收标准

1. 用户或 operator 启动生产日常 portfolio 清算时不需要填写日期范围。
2. Dagster UI 展示的资产名是 `rearview/daily__portfolio_nav_liquidation`。
3. `daily__portfolio_nav_liquidation` 是无分区 asset；它不再拥有独立 production schedule。
4. `daily__fetch_history_sources_to_marts_schedule_job` 的 plan 在 source/raw/dbt/Furnace/marts steps 后包含 portfolio live terminal step。
5. Portfolio live terminal step 只在前置 daily steps 成功后执行。
6. Dagster 使用现有 Rearview APIs 解析全局 settlement target，并创建或复用所有 eligible active portfolios 的 latest target daily run。
7. 最新成功 attempt 的 `live_nav_daily` 覆盖 `live_start_date..target_trade_date`。
8. Materialization metadata 可定位 returned daily runs 的 target、daily run、attempt、status、fact counts 和 skip/failure reason。
9. 旧 `rearview/strategy_portfolio_daily_runs` 不再作为 production Dagster surface 继续扩散。

## 非目标

1. 不重写 portfolio simulation、撮合、费用、止盈止损、FIFO 或绩效公式。
2. 不新增 ClickHouse live facts schema。
3. 不把 portfolio backtest analytics、portfolio dbt rank marts 或 historical portfolio repair 并入本轮 daily terminal step。
4. 不改变 `example__portfolio_live_job` 的隔离原则。
5. 不让 Scheduler 直接读取 ClickHouse 或 PostgreSQL 来解析 settlement target。
6. 不在生产 asset 中保留用户手填日期范围作为常规路径。
7. 当前版本不新增 Rearview 批处理清算 API。
8. 当前版本不实现 per-portfolio settlement target 互不阻塞；继续采用现有全局 settlement target。
9. 不保留独立 production portfolio live schedule；生产触发归并到 `daily__fetch_history_sources_to_marts_schedule`。

## 最小验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

实施阶段追加：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/rearview scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
```

涉及 Rearview API 或 worker 行为时追加；本 RFC 当前推荐路径不要求新增 Rearview API：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
