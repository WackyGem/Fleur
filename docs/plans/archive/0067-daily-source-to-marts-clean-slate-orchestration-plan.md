# Plan 0067: Daily Source to Marts Clean-Slate 编排实施计划

日期：2026-07-01

状态：Completed

领域：Dagster, dbt, Furnace, ClickHouse, data-platform

关联系统：pipeline/scheduler, pipeline/elt, fleur_staging, fleur_intermediate, fleur_calculation, fleur_marts

代码根：

- `pipeline/scheduler/src/scheduler/defs/`
- `pipeline/scheduler/tests/`
- `pipeline/elt/models/`

关联文档：

- [RFC 0040: Dagster stg 到 mart 资产盘点](../../RFC/0040-dagster-stg-to-mart-asset-inventory.md)
- [数据平台地图](../../architecture/data-platform.md)
- [Scheduler 架构](../../architecture/scheduler-architecture.md)
- [Scheduler 模块边界](../../architecture/scheduler-module-boundaries.md)
- [Plan 0065: Source/Raw 统一回填 Controller 实施计划](0065-source-raw-unified-backfill-controller-implementation-plan.md)
- [Plan 0066: Backfill Source to Marts Controller 实施计划](0066-backfill-source-to-marts-controller-plan.md)
- [Daily dry-run 验证报告](../../jobs/reports/2026-07-01-daily-fetch-history-sources-to-marts-schedule-job-dry-run.md)

## 背景

RFC 0040 已确认当前日常编排面存在多套互相重叠的入口：

- source-specific daily jobs/schedules。
- ClickHouse raw sync jobs。
- dbt layer jobs。
- 不完整的 `stock__daily_build_job`。
- portfolio live daily run job。

继续在这些旧入口旁边新增 downstream job，会让 Dagster UI 和运行记录中出现多个“看起来都像生产日常入口”的定义。后续排障时无法快速判断真实 daily production path。

本计划以 clean-slate 方式新增 `daily__fetch_history_sources_to_marts_schedule_job`，作为唯一日常 source -> raw -> stg -> int -> calculation -> mart 主入口。旧 daily/transformation/source-specific production jobs 在迁移完成后删除或降级为 manual/debug；backfill 和 portfolio live 保持独立。

## 关键设计选择

`daily__fetch_history_sources_to_marts_schedule_job` 第一版采用 controller job，而不是单一 asset selection job。

原因是当前 daily network 横跨多种执行语义：

| 范围 | 当前语义 |
| --- | --- |
| daily source assets | daily partition，例如 BaoStock 日 K、THS limit up pool；Jiuyan action field 不纳入本轮 daily wrapper |
| yearly source/raw assets | year partition，例如 EastMoney、ChinaBond、compacted assets、ClickHouse raw year sync |
| snapshot assets | 无日期分区，例如 stock basic、trade calendar；Jiuyan industry list / OCR snapshot 不纳入本次迭代 |
| dbt models | 多数无 Dagster partition，部分 table/view materialization |
| Furnace calculation | 需要 per-run config：由 `target_date` 派生 `request_from=request_to=target_date`；每日增量使用 `mode=append-latest` |
| portfolio backtest analytics assets | 外部 worker 输出和 dbt portfolio marts；不纳入 source-to-marts |

controller job 负责把一个 `target_date` 展开为阶段化 materialization plan，并按阶段提交真实 asset runs。执行逻辑对齐 `backfill__fetch_history_sources_to_marts_job`：复用同一套 source-to-marts scope、stage、asset key、run config 和排除规则，但每日增量将日期范围固定为 `target_date..target_date`，并把 Furnace 历史修复语义从 `replace-cascade` 改为 `append-latest`。

## 目标

1. 新增 `daily__fetch_history_sources_to_marts_schedule_job`，作为唯一日常 source -> raw -> stg -> int -> calculation -> mart 主入口。
2. 新增 daily wrapper config，支持必填 `target_date`、`target_scope`、`execution_mode`、`dry_run`、`refresh_prerequisite_snapshots` 和 `overwrite_source_partitions`；不使用 `start_date/end_date` 或 `run_date/trade_date` 双日期口径。
3. 复用 `backfill__fetch_history_sources_to_marts_job` 的 source-to-marts registry，并在每日入口中把 `target_date` 映射为单日增量范围；不要再维护第二套 daily asset registry。
4. 将 0066 `all_source_to_marts` 当前覆盖的 14 个非 Jiuyan `dbt_staging`、14 个非 Jiuyan/非 portfolio `dbt_intermediate`、9 个非 portfolio `dbt_marts`、6 个 Furnace calculation assets 和 6 个 calculation wrapper assets 纳入第一版 daily network。
5. 第一版排除 portfolio backtest analytics：`fleur_portfolio/portfolio_run_snapshot`、4 个 `fleur_calculation/calc_portfolio_*` source tables、4 个 `int_portfolio_*` wrappers 和 2 个 `mart_portfolio_*_rank` marts 不纳入 daily source-to-marts schedule job。
6. 第一版排除 Jiuyan 全系列：`source/jiuyan__action_field`、`source/jiuyan__action_field_compacted`、`source/jiuyan__industry_list`、`source/jiuyan__industry_images`、`source/jiuyan__industry_ocr`、`source/jiuyan__industry_ocr_snapshot`、相关 ClickHouse raw 和 `stg_jiuyan__*` downstream 后续独立规划。
7. 第一版排除 portfolio live：`rearview/strategy_portfolio_daily_runs`、`portfolio__daily_run_schedule` 和 `fleur_portfolio.live_*` 不纳入 daily source-to-marts schedule job。
8. 清理旧 daily/transformation/source-specific production jobs 和 schedules，避免旧入口污染新设计。
9. 保留手动回填入口和跨 job 告警入口。
10. 增加 tests 防止 daily wrapper 与 0066 source-to-marts registry 及真实 Dagster definitions 漂移。
11. 更新架构事实文档和 runbook，使 `daily__fetch_history_sources_to_marts_schedule_job` 成为唯一日常编排入口。

## 非目标

1. 不修改 source 抓取业务逻辑。
2. 不修改 S3 layout、Parquet schema、ClickHouse raw schema 或 contracts。
3. 不修改 dbt SQL 模型语义、字段契约或 materialization 策略。
4. 不修改 Furnace 指标公式或 Rust CLI 计算语义。
5. 不把 Jiuyan 全系列、portfolio backtest analytics 或 portfolio live 清算并入第一版 daily source-to-marts schedule job。
6. 不改变 `backfill__fetch_history_sources_to_raw_job` 的 raw-only 语义；snapshot reference data 合并回该入口，`backfill__fetch_snapshot_sources_to_raw_job` 移除，历史 source-to-marts 扩展由 Plan 0066 的上层 job 处理。
7. 不在第一版实现复杂 incremental state machine、`_sync_at` 水位线或 changed partition 推断。
8. 不删除 asset 本身；只清理旧 production 编排入口。

## 当前事实基线

### 当前注册 jobs

`uv run dg list defs --target-path scheduler --json` 当前返回以下 jobs：

| Job | 当前处理方向 |
| --- | --- |
| `backfill__fetch_history_sources_to_raw_job` | 保留 raw-only 语义，统一手动 history source/raw 回填入口；history source-to-marts 扩展由 Plan 0066 的 `backfill__fetch_history_sources_to_marts_job` 承担 |
| `backfill__fetch_history_sources_to_marts_job` | 保留手动 history source-to-marts 入口；daily wrapper 复用其 registry 和阶段逻辑，但不复用历史 `replace-cascade` 语义 |
| `baostock__daily_job` | 并入 daily wrapper 或降级 manual/debug |
| `baostock__query_history_k_data_plus_daily_compacted_job` | 并入 daily wrapper 或降级 manual/debug |
| `chinabond__government_bond_job` | 并入 daily wrapper 或降级 manual/debug |
| `clickhouse__raw_sync_all_job` | 删除 production 入口；必要时保留 manual/raw debug |
| `clickhouse__raw_sync_baostock_job` | 并入 daily wrapper；不再单独触发 downstream |
| `clickhouse__raw_sync_eastmoney_job` | 并入 daily wrapper；不再单独暴露 production schedule |
| `clickhouse__raw_sync_jiuyan_market_event_job` | 不纳入本轮 daily wrapper；后续随 Jiuyan 独立 job 处理或降级 manual/debug |
| `clickhouse__raw_sync_snapshot_job` | 并入 daily wrapper 或 manual snapshot refresh |
| `clickhouse__raw_sync_ths_market_event_job` | 并入 daily wrapper；不再单独暴露 production schedule |
| `dbt__marts_build_job` | 删除或改名为 manual full validation |
| `dbt__staging_build_job` | 删除或改名为 manual staging validation |
| `eastmoney__daily_job` | 并入 daily wrapper 或降级 manual/debug |
| `jiuyan__action_field_compacted_job` | 不纳入本轮 daily wrapper；后续 Jiuyan 独立 job 处理或降级 manual/debug |
| `jiuyan__action_field_daily_job` | 不纳入本轮 daily wrapper；后续 Jiuyan 独立 job 处理或降级 manual/debug |
| `jiuyan__industry_list_snapshot_job` | 不纳入本轮 daily wrapper；后续 Jiuyan 独立 job 处理或降级 manual/debug |
| `jiuyan__industry_ocr_pipeline_job` | 不纳入本次 daily source-to-marts；后续独立 job 或 manual OCR debug |
| `jiuyan__industry_ocr_snapshot_job` | 不纳入本次 daily source-to-marts；后续独立 job 或 manual debug |
| `sina__trade_calendar_job` | 作为 prerequisite refresh 能力保留在 daily wrapper 内部 |
| `stock__daily_build_job` | 删除，用 `daily__fetch_history_sources_to_marts_schedule_job` 替代 |
| `strategy_portfolio__daily_run_job` | 保留，portfolio live 独立入口 |
| `ths__limit_up_pool_compacted_job` | 并入 daily wrapper 或降级 manual/debug |
| `ths__limit_up_pool_daily_job` | 并入 daily wrapper 或降级 manual/debug |

### 当前注册 schedules

| Schedule | 当前处理方向 |
| --- | --- |
| `baostock__daily_schedule` | 删除，用 `daily__fetch_history_sources_to_marts_schedule` 统一触发 `daily__fetch_history_sources_to_marts_schedule_job` |
| `chinabond__government_bond_schedule` | 删除，用 `daily__fetch_history_sources_to_marts_schedule` 统一触发 `daily__fetch_history_sources_to_marts_schedule_job` |
| `eastmoney__daily_schedule` | 删除，用 `daily__fetch_history_sources_to_marts_schedule` 统一触发 `daily__fetch_history_sources_to_marts_schedule_job` |
| `jiuyan__action_field_daily_schedule` | 不纳入本轮 daily wrapper；后续 Jiuyan 独立调度设计 |
| `jiuyan__industry_list_snapshot_schedule` | 不纳入本轮 daily wrapper；后续 Jiuyan 独立调度设计 |
| `jiuyan__industry_ocr_pipeline_schedule` | 不纳入本次 daily source-to-marts；后续独立 OCR 策略处理 |
| `portfolio__daily_run_schedule` | 保留，portfolio live 独立 |
| `sina__trade_calendar_schedule` | 保留年度/低频刷新或纳入 daily prerequisite 策略；不作为 daily marts 入口 |
| `stock__daily_build_schedule` | 删除，用 `daily__fetch_history_sources_to_marts_schedule` 统一触发 `daily__fetch_history_sources_to_marts_schedule_job` |
| `ths__limit_up_pool_daily_schedule` | 删除，用 `daily__fetch_history_sources_to_marts_schedule` 统一触发 `daily__fetch_history_sources_to_marts_schedule_job` |

### 当前注册 sensors

| Sensor | 当前处理方向 |
| --- | --- |
| `baostock_raw_sync_success_triggers_stock_daily_build` | 删除；不能继续触发不完整 stock 子集 |
| `default_automation_condition_sensor` | 评估是否由 dbt/Dagster automation condition 自动生成；不得承担 production daily path |
| `slack_asset_failure_sensor` | 保留，跨 job 告警能力 |

### 当前 asset 覆盖基线

第一版 daily wrapper 以 0066 `all_source_to_marts` 为覆盖基线：

- 16 个非 Jiuyan source / compacted source assets，作为 daily root 或 prerequisite。
- 14 个非 Jiuyan `clickhouse_raw` assets。
- 14 个非 Jiuyan `dbt_staging` assets。
- 14 个非 Jiuyan、非 portfolio `dbt_intermediate` assets。
- 9 个非 portfolio `dbt_marts` assets。
- 6 个 `calculation` assets。
- 6 个 calculation wrapper assets。

第一版明确排除：

- `fleur_portfolio/portfolio_run_snapshot`。
- 4 个 `fleur_calculation/calc_portfolio_*` portfolio worker source tables。
- 4 个 `int_portfolio_*` wrappers。
- `mart_portfolio_performance_metric_rank` 和 `mart_portfolio_trade_metric_rank`。
- Jiuyan action field、industry list、images/OCR/snapshot source、raw 和 staging/downstream assets。
- `rearview/strategy_portfolio_daily_runs`。
- `pipeline/elt/models/sources_fleur_portfolio.yml` 中的 `live_*` tables。

## 设计约束

1. `daily__fetch_history_sources_to_marts_schedule_job` 是唯一 production daily source-to-marts 入口。
2. 旧 daily/transformation/source-specific schedules 完成迁移后不得继续注册。
3. 资产 lineage 必须仍由真实 asset materialization 表达；controller 不直接调用业务函数写表。
4. controller 只能提交或组织 asset materialization runs，不能绕过 Dagster asset 图直接写 S3、ClickHouse 或 dbt。
5. source/raw 阶段失败时，不触发依赖它的 dbt/calculation/mart 阶段。
6. Furnace 阶段使用统一 daily config，默认 `mode=append-latest`，并由 `target_date` 派生单日请求窗口；历史修正仍走手动 `backfill__fetch_history_sources_to_marts_job` 的 `replace-cascade`。
7. daily schedule job 必须复用 backfill source-to-marts registry；测试要证明它没有维护第二套 asset scope。
8. backfill jobs、portfolio backtest analytics 和 portfolio live job 不在第一版清理范围内。
9. manual/debug jobs 如果保留，命名必须带 `manual__` 或等价前缀，避免 Dagster UI 误解为 production daily path。
10. 第一版不依赖并发优化；先保证唯一入口、覆盖完整和可恢复。

## 实施阶段

### Phase 0: 编排面冻结和目标定义

修改范围：

- `docs/plans/archive/0067-daily-source-to-marts-clean-slate-orchestration-plan.md`
- `docs/RFC/0040-dagster-stg-to-mart-asset-inventory.md`
- `docs/architecture/scheduler-architecture.md`

任务：

1. 以本计划和 RFC 0040 为准，冻结现有 daily/transformation/source-specific production jobs 的新增。
2. 在架构文档中声明目标状态：`daily__fetch_history_sources_to_marts_schedule_job` 是唯一日常 source-to-marts 入口。
3. 明确保留范围：backfill、portfolio live、slack failure sensor。
4. 明确删除/降级范围：`dbt__*`、`stock__daily_*`、source-specific daily schedules、raw sync production jobs。

完成标准：

- 文档中没有继续建议在 `stock__daily_build_job` 旁边新增 daily jobs。
- `docs/architecture/scheduler-architecture.md` 指向本计划或 RFC 0040。

验证：

```bash
make docs-check
git diff --check
```

### Phase 1: Daily incremental wrapper 和 plan expansion

修改范围：

- `pipeline/scheduler/src/scheduler/defs/daily/`
- `pipeline/scheduler/tests/unit/daily/`

任务：

1. 新增 `scheduler.defs.daily` 包。
2. 新增 daily incremental wrapper，复用 `scheduler.defs.automation.source_to_marts_backfill` 的 scope registry、source/raw steps、downstream steps、Jiuyan 排除和 portfolio 排除规则。
3. daily wrapper 把 `target_date` 映射为单日 source-to-marts request：
   - `start_date = target_date`
   - `end_date = target_date`
   - `target_scope` 默认 `all_source_to_marts`
   - schedule 默认 `execution_mode=full`
   - `target_date` 对所有手动 scope 保持必填；当手动选择 `snapshot_reference_data` 时，wrapper 仍接收 `target_date`，但底层 0066 source-to-marts plan 会主动忽略日期。
4. 定义 `DailyFetchHistorySourcesToMartsConfig`：
   - `target_scope: str = "all_source_to_marts"`
   - `target_date: str`
   - `execution_mode: Literal["full", "source_raw_only", "downstream_only"] = "full"`
   - `dry_run: bool = True`
   - `refresh_snapshot_prerequisites: bool = False`
   - `overwrite_source_partitions: bool = False`
5. 定义 `DailyPlan` / `DailyStep` 数据结构，输出 stage、asset keys、partition selection、run config、tags；可以包装 0066 的 `SourceToMartsPlan` / `BackfillStep`，但对外 payload 使用 `daily.*` 字段。
6. Furnace step 的非 dry-run config 必须从 history `replace-cascade` 改为 daily `append-latest`；`dry_run=true` 时仍使用 Furnace `dry-run`。
7. 测试 wrapper 覆盖当前 0066 source-to-marts registry 暴露的非 Jiuyan、非 portfolio scope，并明确排除 Jiuyan 全系列、portfolio backtest analytics 和 `rearview/strategy_portfolio_daily_runs`。

完成标准：

- dry-run 能打印完整 `DailyPlan`，包含 source/raw/dbt/calculation/mart 阶段。
- registry parity test 能发现 0066 source-to-marts registry 变更后 daily wrapper 未同步。
- exclusion test 明确 Jiuyan 全系列、`portfolio_run_snapshot`、`calc_portfolio_*`、`int_portfolio_*` 和 `mart_portfolio_*_rank` 不进入 daily source-to-marts。

验证：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/daily
```

### Phase 2: 新增 `daily__fetch_history_sources_to_marts_schedule_job`

修改范围：

- `pipeline/scheduler/src/scheduler/defs/daily/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/daily/assets.py` 或 `controller.py`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/tests/`

任务：

1. 新增 controller op：`daily__fetch_history_sources_to_marts_schedule_controller`。
2. 新增 job：`daily__fetch_history_sources_to_marts_schedule_job`。
3. controller dry-run 时只输出 plan，不提交 child runs。
4. controller 非 dry-run 时按阶段提交真实 materialization runs。
5. 每个 child run 写入统一 tags：
   - `daily.kind=fetch_history_sources_to_marts_schedule`
   - `daily.id`
   - `daily.target_scope`
   - `daily.target_date`
   - `daily.execution_mode`
   - `daily.stage`
   - `daily.parent_run_id`
6. raw/source/dbt/calculation 阶段失败时停止后续阶段。
7. Furnace 阶段使用 0066 source-to-marts calculation asset list 和 op name 映射，但每日非 dry-run 模式固定为 `append-latest`。
8. 不在 controller 内直接运行 dbt CLI 或 Furnace CLI；必须通过 Dagster asset materialization。

完成标准：

- `uv run dg list defs --target-path scheduler --json` 能看到 `daily__fetch_history_sources_to_marts_schedule_job`。
- dry-run job 可在本地加载并输出 plan。
- controller unit tests 覆盖 successful plan、stage failure stop、dry-run no submit、tags。

验证：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/daily
cd scheduler
uv run dg check defs
```

### Phase 3: 新增唯一 daily ScheduleDefinition

修改范围：

- `pipeline/scheduler/src/scheduler/defs/daily/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/tests/`

任务：

1. 新增唯一日常自动触发入口：`ScheduleDefinition` 名为 `daily__fetch_history_sources_to_marts_schedule`，触发 job 为 `daily__fetch_history_sources_to_marts_schedule_job`；不再新增或保留 `daily__source_to_marts_schedule`。
2. 首期建议 cron 晚于当前 source schedules 的最后时间窗口，例如 `30 18 * * *` 或重新评估 Asia/Shanghai 市场数据落库时间。
3. schedule 默认状态先保持 stopped，直到 Phase 5 完成真实 dry-run 和小范围验证。
4. schedule run config 统一传：
   - `target_date = scheduled_execution_time` 的 Asia/Shanghai 日期。
   - 后续如需跳过非交易日，由交易日 resolver 在生成 schedule run config 前显式决定是否提交 run 或把 `target_date` 收敛到有效交易日。
   - `dry_run = false` 仅在启用生产前切换。
   - `target_scope = all_source_to_marts`。
   - `execution_mode = full`。
5. 不再保留 `baostock_raw_sync_success_triggers_stock_daily_build` 作为生产触发器。

完成标准：

- Dagster definitions 中只有一个 source-to-marts daily ScheduleDefinition，且它只触发 `daily__fetch_history_sources_to_marts_schedule_job`。
- schedule/job tags 可以按 `daily.kind=fetch_history_sources_to_marts_schedule` 检索。
- 默认未启用生产触发，避免与旧 schedules 并行。

验证：

```bash
cd pipeline
uv run dg list defs --target-path scheduler --json | jq -r '.jobs[].name'
uv run dg list defs --target-path scheduler --json | jq '.schedules[].name'
cd scheduler
uv run dg check defs
```

### Phase 4: 清理旧 production 编排入口

修改范围：

- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/dbt_jobs.py`
- source bundle `definitions.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py`
- `pipeline/scheduler/tests/`

任务：

1. 移除 `TRANSFORMATION_JOBS` 中的 `dbt__staging_build_job`、`dbt__marts_build_job`、`stock__daily_build_job`。
2. 移除 `stock__daily_build_schedule`。
3. 移除 `baostock_raw_sync_success_triggers_stock_daily_build`。
4. 调整 `SOURCE_BUNDLES` 的 jobs/schedules 注册策略：
   - assets 继续注册。
   - production source-specific schedules 不再注册。
   - 如需保留调试 job，统一改名为 `manual__...` 并不挂 production schedule。
5. 调整 `CLICKHOUSE_RAW_JOBS` 暴露策略：
   - raw assets 继续注册。
   - raw sync production jobs 不再作为 daily path 暴露。
   - 如需保留手动 raw debug job，统一改名或集中到 manual module。
6. 保留：
   - `backfill__fetch_history_sources_to_raw_job`
   - `strategy_portfolio__daily_run_job`
   - `portfolio__daily_run_schedule`
   - `slack_asset_failure_sensor`

完成标准：

- `dg list defs --json` 不再包含旧 production daily/transformation jobs。
- 旧 source-specific production schedules 不再出现。
- `portfolio__daily_run_schedule` 和 backfill jobs 仍存在。
- `daily__fetch_history_sources_to_marts_schedule_job` 是唯一日常 source-to-marts 入口。

验证：

```bash
cd pipeline
uv run dg list defs --target-path scheduler --json | jq -r '.jobs[].name'
uv run dg list defs --target-path scheduler --json | jq -r '.schedules[].name'
cd scheduler
uv run dg check defs
```

### Phase 5: Daily network 验证和运行报告

修改范围：

- `docs/jobs/reports/`
- `docs/jobs/README.md` 或相关 runbook
- `docs/architecture/scheduler-architecture.md`

任务：

1. 运行 `daily__fetch_history_sources_to_marts_schedule_job` dry-run，记录 expanded plan。
2. 选择一个最近交易日做小范围真实运行，或先用全部阶段 dry-run + source/raw/dbt 子阶段手动验证。
3. 核对 materialization coverage：
   - raw sync 成功。
   - dbt staging/intermediate/marts 被覆盖。
   - 6 个 Furnace calculation 资产使用同一 `target_date` config。
   - Jiuyan 全系列和 portfolio backtest analytics 不在 expanded plan 中。
4. 记录失败恢复方式：按 `daily.id` 查找 child runs，重跑失败 stage 或重新提交 parent job。
5. 输出运行报告，例如 `docs/jobs/reports/2026-07-xx-daily-fetch-history-sources-to-marts-schedule-job.md`。
6. 更新 scheduler 架构文档，把旧编排入口替换为 daily network 入口。

完成标准：

- 有一份 dry-run 或真实运行报告记录命令、日期、资产范围和结果。
- 报告中证明旧 production schedules 不再与 daily job 并行。
- 架构事实文档指向新 daily job 和保留的 backfill/live 入口。

验证：

```bash
make docs-check
git diff --check
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
cd scheduler
uv run dg check defs
```

## 禁止模式

1. 不允许在旧 `stock__daily_build_job` 旁边新增另一个 stock daily job。
2. 不允许保留多个 production daily schedules，让它们同时刷新同一批 source/raw/downstream。
3. 不允许 controller 直接调用 source service、dbt CLI、Furnace CLI 或 ClickHouse 写入逻辑绕过 asset materialization。
4. 不允许用 `a || b` 式 fallback 猜测 asset key、op name 或 run config；必须来自 registry 或真实 definitions。
5. 不允许把 Jiuyan 全系列、portfolio backtest analytics 或 portfolio live 表误纳入第一版 daily source-to-marts graph。
6. 不允许删除 backfill 和 portfolio live 入口来制造“唯一入口”的假象。
7. 不允许只改 schedule 不改 job surface；Dagster UI 中的旧 production job 也必须清理或降级命名。

## 测试策略

| 范围 | 测试 |
| --- | --- |
| Registry parity | 每日 wrapper 复用 0066 source-to-marts registry，当前非 Jiuyan、非 portfolio `dbt_staging` / `dbt_intermediate` / `dbt_marts` / `calculation` assets 与 backfill source-to-marts coverage 保持一致 |
| Exclusion | Jiuyan 全系列、`portfolio_run_snapshot`、`calc_portfolio_*`、`int_portfolio_*`、`mart_portfolio_*_rank`、`rearview/strategy_portfolio_daily_runs` 和 `fleur_portfolio.live_*` 不进入第一版 daily network |
| Plan expansion | `target_date` 能展开 source/raw/dbt/calc/mart 阶段和 partition selections |
| Furnace config | 6 个 Furnace assets 使用同一 `request_from=request_to=target_date` 和 `append-latest` |
| Failure handling | 上游阶段失败时不提交下游 stage |
| Definition surface | 旧 production daily jobs/schedules 不再注册；backfill/live/slack 保留 |

## 验收标准

1. `daily__fetch_history_sources_to_marts_schedule_job` 已注册，并可 dry-run 输出完整 daily plan。
2. `daily__fetch_history_sources_to_marts_schedule` 是唯一 source-to-marts production daily ScheduleDefinition，触发 `daily__fetch_history_sources_to_marts_schedule_job`，启用前默认 stopped。
3. `dbt__staging_build_job`、`dbt__marts_build_job`、`stock__daily_build_job` 和 `stock__daily_build_schedule` 不再作为 production 入口注册。
4. source-specific daily schedules 不再作为 production 入口注册。
5. `baostock_raw_sync_success_triggers_stock_daily_build` 已删除或不再触发 production downstream。
6. backfill jobs、Jiuyan 全系列、portfolio backtest analytics、portfolio live job/schedule 和 slack failure sensor 保留或独立规划，不并入 source-to-marts。
7. registry parity tests 和 definition surface tests 通过。
8. `uv run dg check defs` 通过。
9. 运行报告记录 dry-run 或小范围真实运行结果。
10. `docs/architecture/scheduler-architecture.md` 和相关 runbook 已更新。

## 最小验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

代码实施阶段：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
cd scheduler
uv run dg check defs
```

定义面核验：

```bash
cd pipeline
uv run dg list defs --target-path scheduler --json | jq -r '.jobs[].name'
uv run dg list defs --target-path scheduler --json | jq -r '.schedules[].name'
uv run dg list defs --target-path scheduler --json | jq -r '.sensors[].name'
```

## 完成后的维护动作

1. 将本计划状态更新为 `Completed` 并移入 `docs/plans/archive/`。
2. 更新 `docs/plans/README.md` 的 Recently Completed。
3. 新增或更新运行报告，记录 first dry-run / first production run。
4. 若 daily wrapper 成为长期维护边界，补充 scheduler 架构事实文档和必要的 agent skill/runbook。
