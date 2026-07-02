# Plan 0073: Strategy Portfolio 日度 NAV 清算入口收敛实施计划

日期：2026-07-02

状态：Completed

领域：Dagster, Scheduler, Rearview, Strategy Portfolio, Data Platform

关联系统：

- `pipeline/scheduler/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-portfolio-worker/`
- `fleur_portfolio`

关联文档：

- [RFC 0045: Strategy Portfolio 日度 NAV 清算入口收敛](../../RFC/0045-strategy-portfolio-daily-nav-liquidation.md)
- [RFC 0040: Dagster stg 到 mart 资产盘点](../../RFC/0040-dagster-stg-to-mart-asset-inventory.md)
- [Plan 0067: Daily Source to Marts Clean-Slate 编排实施计划](0067-daily-source-to-marts-clean-slate-orchestration-plan.md)
- [Plan 0062: Racingline 策略组合对账单实施计划](0062-racingline-strategy-portfolio-statement-plan.md)
- [Plan 0072: Racingline 0051 低位反转 example live job 实施计划](0072-racingline-0051-low-reversal-example-live-job-plan.md)
- [Scheduler Architecture](../../architecture/scheduler-architecture.md)
- [Data Platform Architecture](../../architecture/data-platform.md)
- [Rearview Architecture](../../architecture/rearview.md)
- [2026-07-02 Strategy Portfolio Daily NAV Liquidation Report](../../jobs/reports/2026-07-02-strategy-portfolio-daily-nav-liquidation.md)

## 背景

RFC 0045 已确认 `rearview/strategy_portfolio_daily_runs` 的现有 Dagster surface 与真实业务语义不匹配：

- 名字强调创建 daily run records，而不是组合 NAV 清算结果。
- `DailyPartitionsDefinition(start_date="2026-06-24")` 容易让 partition key 被误解为清算交易日。
- 生产 config 暴露 `trade_date/start_date/end_date/strategy_portfolio_id`，把日常生产入口和手动修复入口混在一起。
- 现有 `portfolio__daily_run_schedule` 独立于 `daily__fetch_history_sources_to_marts_schedule_job`，不能保证 portfolio live 在 source/raw/dbt/Furnace/marts 成功后运行。

本计划把 RFC 0045 拆为可实施的 scheduler 代码、测试、文档和运行验收步骤。当前版本不新增 Rearview 批处理清算 API，继续复用现有 `settlement-target`、`daily-runs`、status 和 fact-counts APIs。

## 目标

1. 将 production asset key 从 `rearview/strategy_portfolio_daily_runs` 收敛为 `rearview/daily__portfolio_nav_liquidation`。
2. 将 `daily__portfolio_nav_liquidation` 改为无分区结果资产，去掉 `DailyPartitionsDefinition`。
3. 移除 production config 中的 `trade_date/start_date/end_date/strategy_portfolio_id/chunk_size`。
4. production 默认路径只调用现有 Rearview 单日 daily-runs API：先解析全局 settlement target，再为所有 eligible active portfolios 创建或复用 latest target daily runs。
5. 将 portfolio live 清算作为 `daily__fetch_history_sources_to_marts_schedule_job` 的 terminal step，排在 source/raw、dbt 和 Furnace calculation steps 之后。
6. 移除独立 production `portfolio__daily_run_schedule` / `portfolio__daily_nav_liquidation_schedule` surface。
7. 保留 `example__portfolio_live_job` 作为 0051 手动回归入口。
8. 保留 Rearview range API resource 给 example 和后续 manual repair 使用，但不作为 production daily asset 默认路径。
9. 用 unit/integration tests 固化新命名、无分区、daily terminal step、等待 worker 终态和 fact-count metadata。
10. 更新架构事实文档、RFC 索引和运行报告，使 `daily__fetch_history_sources_to_marts_schedule_job` 成为唯一生产日常入口。

## 非目标

1. 不修改 Rearview HTTP API contract，不新增 `nav-liquidations/daily` 或类似批处理 API。
2. 不修改 portfolio simulation、撮合、费用、风控、FIFO、绩效或 live facts 写入公式。
3. 不新增 ClickHouse `fleur_portfolio.live_*` schema。
4. 不把 portfolio backtest analytics、portfolio dbt rank marts 或 historical portfolio repair 并入本轮 daily terminal step。
5. 不让 Scheduler 直接读取 ClickHouse 或 PostgreSQL 来解析 settlement target。
6. 不实现 per-portfolio settlement target 互不阻塞；当前继续采用现有全局 settlement target。
7. 不改变 `example__portfolio_live_job` 的独立手动入口语义。
8. 不在 production asset 中保留用户手填日期范围作为常规路径。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| Scheduler asset | [assets.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/assets.py) 定义 `STRATEGY_PORTFOLIO_DAILY_ASSET_KEY = AssetKey(["rearview", "strategy_portfolio_daily_runs"])`，并使用 `DailyPartitionsDefinition(start_date="2026-06-24")`。 |
| Scheduler config | `StrategyPortfolioDailyRunConfig` 暴露 `trade_date`、`start_date`、`end_date`、`strategy_portfolio_id`、`chunk_size` 和等待配置。 |
| Current production job/schedule | [definitions.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/definitions.py) 注册 `strategy_portfolio__daily_run_job` 和 `portfolio__daily_run_schedule`。 |
| Daily controller | [source_to_marts.py](../../../pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py) 的 `daily__fetch_history_sources_to_marts_schedule_job` 当前只提交 source/raw/dbt/Furnace/mart steps。 |
| Daily controller tests | [test_source_to_marts.py](../../../pipeline/scheduler/tests/unit/daily/test_source_to_marts.py) 当前明确断言 `rearview/strategy_portfolio_daily_runs` 不进入 daily plan。 |
| Definition tests | [test_definitions_and_schedules.py](../../../pipeline/scheduler/tests/integration/test_definitions_and_schedules.py) 当前断言旧 asset key、旧 portfolio job 和旧 portfolio schedule 已注册。 |
| Rearview resource | [resources.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/resources.py) 已有 `create_strategy_portfolio_daily_runs()`、`create_strategy_portfolio_daily_runs_range()`、status、fact-counts 和 settlement-target client 方法。 |
| Rearview control plane | `POST /rearview/strategy-portfolios/daily-runs` 未指定 `strategy_portfolio_id` 时由 Rearview 读取 active portfolios 并用 `(strategy_portfolio_id, trade_date)` 保持幂等。 |
| Worker semantics | `rearview-portfolio-worker` 对单个 daily run 使用 `run_start_date = initial_signal_date` 到 `trade_date` 的 full-window 输入，输出从 `live_start_date` 开始归一化的 live facts。 |

## 设计约束

1. `daily__fetch_history_sources_to_marts_schedule_job` 是唯一 production daily 入口。
2. `daily__portfolio_nav_liquidation` 是无分区 asset；schedule 触发时间和业务 target trade date 分离。
3. `daily__portfolio_nav_liquidation` 的 production config 只保留执行策略参数：`wait_for_completion`、`poll_interval_seconds`、`timeout_seconds`。
4. Production daily path 不调用 Rearview range API，不展开日期范围，不接受用户输入 `strategy_portfolio_id`。
5. Terminal step 只在 `daily__fetch_history_sources_to_marts_schedule_job` 的前置 steps 全部成功后执行。
6. `dry_run=true` 时 plan 中展示 portfolio live terminal step，但不调用 Rearview。
7. Terminal step 失败、worker failed 或 timeout 必须使 daily controller fail。
8. Scheduler 不直接读取 ClickHouse/PostgreSQL；settlement target、active portfolio 枚举、archived 过滤和 outbox 分发继续由 Rearview 负责。
9. 0051 example job 继续使用 portfolio-specific settlement target 和 range API，且不进入 production daily schedule。
10. 资产 lineage 不伪造 portfolio live 对 source/dbt/Furnace assets 的数据依赖；执行顺序由 daily controller step 顺序表达。

## 实施阶段

### Phase 0: Characterization And Guardrails

目标：先把当前注册面、测试断言和新目标边界写成可执行的 characterization，避免重命名时误删 0051 或手动能力。

实施项：

1. 审计 `pipeline/scheduler/src/scheduler/defs/rearview/assets.py` 中旧 asset、example asset 和共享 helper 的调用关系。
2. 审计 `pipeline/scheduler/src/scheduler/defs/rearview/definitions.py` 中 job/schedule 注册面，确认哪些是 production surface，哪些是手动入口。
3. 审计 `pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py` 中 `DailyPlan` / `DailyStep` / `BackfillRunSubmitter` 能否提交 unpartitioned asset step。
4. 更新或新增 characterization tests，明确当前旧 key 与旧 schedule 将被替换。
5. 在实现前记录需要保留的 helper：`_wait_for_daily_runs()`、`_query_fact_counts_for_succeeded_runs()`、example 0051 helper 和 `create_strategy_portfolio_daily_runs_range()` resource。

测试策略：

1. 只做最小测试变更，不改变行为。
2. 如果发现 helper 归属不清，先拆分 helper 命名，再进入 Phase 1。

完成标准：

1. 明确哪些测试将在 Phase 1/2 改为新断言。
2. 没有删除或弱化 0051 example job 的测试覆盖。

### Phase 1: Rename And Unpartition Rearview Asset

目标：把 `strategy_portfolio_daily_runs` 收敛为无分区结果资产 `daily__portfolio_nav_liquidation`。

实施项：

1. 在 `assets.py` 中新增或替换常量：
   - `DAILY_PORTFOLIO_NAV_LIQUIDATION_ASSET_KEY = AssetKey(["rearview", "daily__portfolio_nav_liquidation"])`。
2. 将 asset function 从 `strategy_portfolio_daily_runs()` 重命名为 `daily__portfolio_nav_liquidation()`。
3. 将 config class 从 `StrategyPortfolioDailyRunConfig` 重命名为 `DailyPortfolioNavLiquidationConfig`。
4. 从 production config 移除：
   - `trade_date`
   - `start_date`
   - `end_date`
   - `strategy_portfolio_id`
   - `chunk_size`
5. 移除 `STRATEGY_PORTFOLIO_DAILY_PARTITIONS` 和 `partitions_def=`。
6. 默认执行路径改为：
   - `get_strategy_portfolio_settlement_target()`
   - target empty -> skip metadata
   - target present -> `create_strategy_portfolio_daily_runs(trade_date=target, client_request_id=...)`
   - wait status
   - query fact-counts
   - materialize metadata
7. 删除或隔离 production asset 对 `_daily_run_range_request()`、`_date_chunks()`、`_combine_daily_run_range_responses()` 的依赖。
8. 保留 `create_strategy_portfolio_daily_runs_range()` resource 和 example 0051 清算路径。
9. 更新 metadata key：
   - `target_trade_date`
   - `active_portfolio_count`
   - `created_run_count`
   - `skipped_run_count`
   - `succeeded_run_count`
   - `failed_run_count`
   - `timeout_run_count`
   - `daily_run_ids`
   - `created_daily_run_ids`
   - `skipped_daily_run_ids`
   - `daily_run_statuses`
   - `daily_run_fact_counts`

测试策略：

1. Unit tests 覆盖 target empty skip。
2. Unit tests 覆盖 target present 时调用 single-day `create_strategy_portfolio_daily_runs()` 而不是 range API。
3. Unit tests 覆盖 status failed/timeout 仍 fail。
4. Unit tests 覆盖 fact-counts `nav_row_count <= 0` 仍 fail。
5. Resource tests 覆盖 `create_strategy_portfolio_daily_runs()` path 和 payload。
6. Example 0051 tests 继续覆盖 range API 和 example ensure API。

完成标准：

1. `rearview/daily__portfolio_nav_liquidation` 是 materializable unpartitioned asset。
2. `rearview/strategy_portfolio_daily_runs` 不再作为 production asset 注册。
3. Production asset 默认路径不再接受日期范围或指定 portfolio。

### Phase 2: Remove Independent Production Portfolio Schedule

目标：删除旧独立 production portfolio live job/schedule surface，避免 Dagster UI 出现两个 daily production 入口。

实施项：

1. 在 `pipeline/scheduler/src/scheduler/defs/rearview/definitions.py` 中移除 `STRATEGY_PORTFOLIO_DAILY_RUN_JOB` 的 production 注册。
2. 移除 `PORTFOLIO_DAILY_RUN_SCHEDULE` / `portfolio__daily_run_schedule` production schedule。
3. `REARVIEW_DEFS` 继续注册 `REARVIEW_ASSETS` 和 `EXAMPLE_PORTFOLIO_LIVE_JOB`。
4. 如果仍需要人工 launch portfolio liquidation，后续另设 `manual__portfolio_nav_liquidation_repair_job`；本阶段不新增该 job。
5. 更新 integration definitions tests：
   - 新 asset key 存在。
   - 旧 asset key 不存在。
   - production jobs 中不再包含 `strategy_portfolio__daily_run_job`。
   - production schedules 中不再包含 `portfolio__daily_run_schedule`。
   - `example__portfolio_live_job` 仍存在且不挂 schedule。

测试策略：

1. `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py` 覆盖注册面。
2. `uv run dg list defs --assets "key:rearview/daily__portfolio_nav_liquidation" --json` 用于人工核验。
3. `uv run dg check defs` 验证 definitions load。

完成标准：

1. Dagster definitions 只保留一个 production daily schedule：`daily__fetch_history_sources_to_marts_schedule`。
2. 0051 example 手动 job 不受影响。

### Phase 3: Add Portfolio Live Terminal Step To Daily Controller

目标：让 `daily__fetch_history_sources_to_marts_schedule_job` 在 source/raw/dbt/Furnace/marts 成功后提交 portfolio live terminal step。

实施项：

1. 在 `pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py` 中新增 stage 常量，例如 `STAGE_PORTFOLIO_LIVE_LIQUIDATION = "portfolio_live_liquidation"`。
2. 在 `DailyPlan` 构造阶段追加 terminal step：
   - `label = "portfolio live nav liquidation"`
   - `step_kind = "asset_materialization"` 或既有 step kind 约定中的等价值
   - `stage = STAGE_PORTFOLIO_LIVE_LIQUIDATION`
   - `asset_keys = ("rearview/daily__portfolio_nav_liquidation",)`
   - `partition = BackfillPartitionSelection()`
   - `run_config = {}`
   - tags 包含 `daily.stage=portfolio_live_liquidation`、`daily.step=...`、`daily.parent_run_id`
3. 第一版 terminal step 只在 production daily 默认范围追加：
   - `target_scope == all_source_to_marts`
   - `execution_mode == full`
   - partial/manual target scope 不自动清算 portfolio live，避免依赖未齐。
4. `dry_run=true` 时，`DailyPlan` payload 和 log lines 展示 terminal step，但 `execute_daily_fetch_history_sources_to_marts_plan()` 不提交任何 step。
5. `dry_run=false` 时，`execute_daily_fetch_history_sources_to_marts_plan()` 按 steps 顺序提交；由于 terminal step 是最后一个，前置失败时自然不会提交。
6. Terminal step 使用现有 `InProcessDagsterRunSubmitter` 通过 implicit asset job materialize unpartitioned asset。
7. Terminal step result failed 或 non-terminal 时沿用 daily controller error path，使 parent run fail。

测试策略：

1. 更新 `test_daily_plan_reuses_source_to_marts_registry_and_excludes_independent_domains()`：
   - 不再断言 portfolio live asset 被排除。
   - 仍断言 portfolio backtest analytics、`calc_portfolio_*`、`int_portfolio_*`、`mart_portfolio_*_rank` 排除。
   - 断言 `rearview/daily__portfolio_nav_liquidation` 作为 terminal step 出现。
2. 新增 test：terminal step 是 `plan.steps[-1]`。
3. 新增 test：`dry_run=true` plan 包含 terminal step，但 submitter 不提交。
4. 新增 test：source/raw failure 时 submitter 未提交 terminal step。
5. 新增 test：partial target scope 不追加 terminal step，或明确记录 omitted reason。
6. 新增 test：terminal step failure 使 daily execution fail。

完成标准：

1. `daily__fetch_history_sources_to_marts_schedule_job` dry-run plan 能看到 portfolio live terminal step。
2. 非 dry-run daily controller 只在前置步骤成功后提交 terminal step。
3. Terminal step 与 source/raw/dbt/Furnace/marts 共享 daily controller tags。

### Phase 4: Metadata And Fact Verification Tightening

目标：让 daily controller 和 `daily__portfolio_nav_liquidation` 的成功语义能证明 worker 已完成且 live facts 可读。

实施项：

1. 保留现有 `_wait_for_daily_runs()` 终态轮询。
2. 保留 failed/cancelled/`failed_*` status -> RuntimeError。
3. 保留 timeout -> TimeoutError。
4. 保留 `nav_row_count <= 0` -> RuntimeError。
5. 在 metadata 中保留 raw statuses/fact-counts JSON，便于排障。
6. 在 daily terminal step tags 或 metadata 中记录 parent daily controller run id。
7. 如果现有 fact-counts API 已返回 result attempt id，则 metadata 必须记录 `latest_result_attempt_id`。
8. 不在本阶段要求 nav min/max coverage，因为现有 fact-counts response 还不返回 coverage；若后续扩展 Rearview response，再补 coverage tests。

测试策略：

1. Scheduler unit tests 覆盖 succeeded/failure/timeout/no-nav rows。
2. Metadata unit tests 覆盖 target date、counts、daily_run_ids、statuses 和 fact_counts。
3. Daily controller test 覆盖 terminal step failed 时 parent controller fail。

完成标准：

1. Materialization metadata 足以定位 target、daily run id、status、attempt 和 fact counts。
2. Dagster success 不再只代表 daily run record 创建成功。

### Phase 5: Documentation And Architecture Closure

目标：把新入口从 RFC/plan 变成当前事实文档。

实施项：

1. 更新 [scheduler-architecture.md](../../architecture/scheduler-architecture.md)：
   - `daily__fetch_history_sources_to_marts_schedule_job` 说明包含 portfolio live terminal step。
   - 移除 portfolio live 独立 production job/schedule 描述。
   - 保留 `example__portfolio_live_job` 手动入口说明。
2. 更新 [data-platform.md](../../architecture/data-platform.md)：
   - Daily source-to-marts 终端触发 portfolio live liquidation。
   - `daily__portfolio_nav_liquidation` 通过 Rearview APIs 等待 worker 和 fact counts。
3. 更新 [rearview.md](../../architecture/rearview.md)：
   - 说明 Dagster daily controller 调用现有 daily-runs API，不新增 Rearview batch API。
4. 更新 [RFC 0040](../../RFC/0040-dagster-stg-to-mart-asset-inventory.md) 中 portfolio live 不纳入 daily network 的旧结论，改为被 RFC 0045/Plan 0073 替代。
5. 更新 [RFC 0045](../../RFC/0045-strategy-portfolio-daily-nav-liquidation.md) 的状态或实施链接。
6. 新增 job report，记录一次 dry-run plan 和至少一次 dev smoke：
   - dry-run plan 包含 terminal step。
   - 非 dry-run terminal step 成功或明确记录阻塞原因。
   - 若 dev 环境缺少 active portfolio 或 Rearview 服务，报告写明阻塞事实和已验证范围。

测试策略：

1. 文档-only 检查：`make docs-check`、`git diff --check`。
2. 最终代码检查见下方验证命令。

完成标准：

1. 文档没有继续把 `portfolio__daily_run_schedule` 作为 production schedule。
2. 运行报告能从 Dagster plan/metadata 证明 daily controller 已包含 portfolio live terminal step。

## 最小验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

Scheduler 实施阶段：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests --check
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/rearview scheduler/tests/unit/daily scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
```

若修改 Rearview Rust 代码才追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 完成标准

1. `rearview/daily__portfolio_nav_liquidation` 是 registered, executable, unpartitioned asset。
2. `rearview/strategy_portfolio_daily_runs` 不再作为 production asset key 注册。
3. Production definitions 不再注册 `portfolio__daily_run_schedule`。
4. `daily__fetch_history_sources_to_marts_schedule_job` 的 dry-run plan 在最后包含 `rearview/daily__portfolio_nav_liquidation` terminal step。
5. Terminal step 只在前置 source/raw/dbt/Furnace/marts steps 成功后执行。
6. Production path 不要求用户填写日期范围。
7. Production path 使用现有 Rearview APIs，不新增 Rearview batch API。
8. `example__portfolio_live_job` 仍可独立手动运行，不进入 production schedule。
9. Scheduler unit/integration tests 和 `dg check defs` 通过。
10. 架构事实文档和 job report 已更新，计划可归档。
