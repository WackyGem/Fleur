# Plan 0066: Backfill Sources to Marts Controller 实施计划

日期：2026-07-01

状态：Completed

领域：Dagster, dbt, Furnace, ClickHouse, data-platform

关联系统：pipeline/scheduler, pipeline/elt, fleur_raw, fleur_staging, fleur_intermediate, fleur_calculation, fleur_marts

代码根：

- `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`
- `pipeline/scheduler/src/scheduler/defs/automation/source_raw_backfill.py`（复用既有 raw-only plan；本计划不新增 raw-only scope）
- `pipeline/scheduler/src/scheduler/defs/daily/`
- `pipeline/scheduler/src/scheduler/defs/dbt_jobs.py`
- `pipeline/scheduler/src/scheduler/defs/furnace/`
- `pipeline/scheduler/tests/`
- `pipeline/elt/models/`

关联文档：

- [RFC 0040: Dagster stg 到 mart 资产盘点](../../RFC/0040-dagster-stg-to-mart-asset-inventory.md)
- [Plan 0065: Source/Raw 统一回填 Controller 实施计划](0065-source-raw-unified-backfill-controller-implementation-plan.md)
- [Plan 0067: Daily Source to Marts Clean-Slate 编排实施计划](0067-daily-source-to-marts-clean-slate-orchestration-plan.md)
- [Source/Raw 回填运行手册](../../skills/fleur-dagster-backfill-runbook/SKILL.md)
- [数据平台地图](../../architecture/data-platform.md)
- [Scheduler 架构](../../architecture/scheduler-architecture.md)

## 背景

当前统一回填入口 `backfill__fetch_history_sources_to_raw_job` 已经覆盖 source -> compacted source -> ClickHouse raw，并已作为 history source/raw raw-only 修复入口。它适合作为历史 source/raw 修复入口，但不会继续触发 dbt staging/intermediate/marts 或 Furnace calculation。

本计划不再把 `backfill__fetch_history_sources_to_raw_job` 的 scope、config 或 source/raw plan expansion 作为待实现内容。`backfill__fetch_snapshot_sources_to_raw_job` 不再作为独立入口保留；snapshot reference data 的 raw-only 支持以既有 `backfill__fetch_history_sources_to_raw_job` 能力为前置，`start_date` 和 `end_date` 继续作为必填 config 暴露，snapshot scope 主动忽略日期参数。

Jiuyan 异动、行业列表、OCR 及相关 source/raw/staging/downstream 后续单独设计独立回填任务，不纳入本次迭代。

随着 `daily__fetch_history_sources_to_marts_schedule_job` 设计成为唯一日常 source -> raw -> stg -> int -> calculation -> mart 主入口，历史修复也需要对应的上层入口。否则一次历史 source/raw 回填后，还需要人工再判断并执行 downstream dbt/Furnace/mart 重建，容易遗漏。

本计划新增 `backfill__fetch_history_sources_to_marts_job`。它不原地改写 `backfill__fetch_history_sources_to_raw_job` 的语义，而是在其上方建立更宽的 source-to-marts controller：

```text
backfill__fetch_history_sources_to_marts_job
  -> source/raw backfill stages
  -> dbt staging/intermediate stages
  -> Furnace calculation stages
  -> dbt wrappers/marts stages
```

## 2026-07-01 实施记录

本轮已按 plural `sources` 命名落地 source-to-marts 入口：

- Job：`backfill__fetch_history_sources_to_marts_job`
- Controller op：`backfill__fetch_history_sources_to_marts_controller`
- 代码入口：`pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`

同步完成的范围调整：

- `backfill__fetch_history_sources_to_raw_job` 保留 raw-only 语义，并作为 source-to-marts 的 source/raw 前置能力复用；0066 不再把扩展 raw-only job 作为实施任务。
- `snapshot_reference_data` 已纳入 `backfill__fetch_history_sources_to_raw_job`，`start_date` / `end_date` 在 config schema 中继续必填，但该 scope 主动忽略日期。
- `backfill__fetch_snapshot_sources_to_raw_job` 和 `backfill__fetch_snapshot_sources_to_raw_controller` 不再作为公开 definitions 注册。
- `backfill__fetch_history_sources_to_marts_job` 的 source/raw stage 对底层 mixed scope 显式过滤 Jiuyan asset keys；`market_events` 在 source-to-marts 中只保留 THS limit up pool 相关链路。
- Jiuyan 异动、行业列表、图片/OCR/OCR snapshot 及其 downstream 不纳入本次 source-to-marts 迭代，后续独立 job/plan 处理。
- Portfolio backtest analytics 和 portfolio live 不纳入本 job。

已记录 dry-run 验收报告：[2026-07-01-backfill-source-to-marts-controller-dry-run.md](../../jobs/reports/2026-07-01-backfill-source-to-marts-controller-dry-run.md)。

## 核心决策

不把 `backfill__fetch_history_sources_to_raw_job` 原地扩展成 source-to-marts，也不删除 raw-only 入口。

原因：

1. 现有 runbook 和操作习惯已经把它定义为 source/raw 修复入口。
2. raw-only 恢复是必要能力；历史修复时经常只需要重跑 raw sync，不应被迫触发下游全量重建。
3. source/raw 和 downstream 的失败恢复粒度不同，放在一个 controller 的不同模式下更清楚。
4. `backfill__fetch_history_sources_to_marts_job` 可以复用 source/raw plan，但必须显式表达 downstream 策略和成本。

最终入口分层：

| Job | 角色 |
| --- | --- |
| `backfill__fetch_history_sources_to_raw_job` | 已有 raw-only 前置入口；本计划复用它，不新增或扩展 raw-only scope |
| `backfill__fetch_history_sources_to_marts_job` | 新增，历史 source/raw 修复后继续推进到 dbt/Furnace/marts 的上层入口 |

`backfill__fetch_snapshot_sources_to_raw_job` 在本计划实施时移除。Jiuyan 异动、行业列表和 OCR 不迁入本次 source-to-marts 回填范围，后续独立规划。

## 目标

1. 新增 `backfill__fetch_history_sources_to_marts_job`。
2. 新增 `backfill__fetch_history_sources_to_marts_controller`，支持 dry-run 输出完整 source-to-marts 回填计划。
3. 复用 Plan 0065 的 source/raw registry 和 child run 提交机制，避免复制 source/raw 映射。
4. 移除 `backfill__fetch_snapshot_sources_to_raw_job` 独立入口。
5. 不再新增或扩展 `backfill__fetch_history_sources_to_raw_job`；它是已具备的 raw-only 前置能力。
6. 保持 `start_date` 和 `end_date` 为必填 config；snapshot reference data scope 忽略这两个参数。
7. 在 source/raw 成功后追加 downstream 阶段：
   - dbt staging。
   - intermediate。
   - Furnace calculation。
   - calculation wrappers。
   - marts。
8. 支持 `execution_mode`：
   - `full`：source/raw + downstream。
   - `source_raw_only`：等价于底层 source/raw 计划，用于新入口 dry-run 对比。
   - `downstream_only`：假设 source/raw 已完成，只重建 downstream。
9. 支持按 `target_scope` 控制 downstream selection，避免所有历史修复都重建全项目。
10. 对历史技术指标修复使用 `replace-cascade`，不使用 daily `append-latest`。
11. 不纳入 Jiuyan 异动、行业列表、OCR 及相关下游。
12. 不纳入 portfolio backtest analytics。
13. 不纳入 portfolio live。
14. 更新 backfill runbook，明确统一 history source/raw 入口和 source-to-marts 入口。

## 非目标

1. 不删除 `backfill__fetch_history_sources_to_raw_job`。
2. 不保留 `backfill__fetch_snapshot_sources_to_raw_job`。
3. 不改变 source/raw 资产的业务抓取、S3 layout、ClickHouse raw schema 或 contracts。
4. 不把 dbt models 改成分区增量。
5. 不新增 `_sync_at` 水位线或 changed partition 状态机。
6. 不让 controller 直接调用 dbt CLI、Furnace CLI、source service 或 ClickHouse 写入逻辑绕过 asset materialization。
7. 不物化 `fleur_portfolio/portfolio_run_snapshot` 和 `fleur_calculation/calc_portfolio_*` 外部 source tables。
8. 不重建 `int_portfolio_*` wrappers 或 `mart_portfolio_*_rank` marts。
9. 不把 `rearview/strategy_portfolio_daily_runs` 或 `fleur_portfolio.live_*` 纳入第一版 backfill source-to-marts。
10. 不把 Jiuyan action field、industry list、images/OCR/snapshot pipeline 或相关 staging/downstream 纳入本次迭代；后续另开独立 job 计划。

## 当前事实基线

### Source/Raw 回填入口

Plan 0065 和后续调整已提供 raw-only 前置能力：

| Job | 范围 |
| --- | --- |
| `backfill__fetch_history_sources_to_raw_job` | history source/raw raw-only 入口；支持日期型 source/raw 和 snapshot reference data |
| `backfill__fetch_snapshot_sources_to_raw_job` | 旧 snapshot 专用公开入口；本计划只清理该公开入口，不把它作为 source-to-marts 前置能力 |

本计划不再实现 `backfill__fetch_history_sources_to_raw_job` 的能力扩展；它已经是 `backfill__fetch_history_sources_to_marts_job` 的 raw-only 前置入口。Jiuyan 异动、行业列表和 OCR 后续独立 job 设计。

当前 `backfill__fetch_history_sources_to_raw_job` 支持：

- `dry_run`。
- `execution_mode=full/raw_only`。
- `refresh_prerequisite_snapshots`。
- `overwrite_source_partitions`。
- child materialization runs。
- 统一 `backfill.*` tags。

### Downstream 资产范围

`backfill__fetch_history_sources_to_marts_job` 第一版 downstream 覆盖：

| 范围 | 处理 |
| --- | --- |
| 非 Jiuyan `dbt_staging` | materialize |
| 非 Jiuyan、非 portfolio `dbt_intermediate` | materialize |
| 非 Jiuyan、非 portfolio `dbt_marts` | materialize |
| 6 个 Furnace `calculation` | materialize，历史修复使用 `replace-cascade` |
| Jiuyan action/industry/OCR | 排除；另开独立 Jiuyan 回填 job/plan |
| portfolio backtest analytics | 排除；另开 portfolio analytics job/backfill plan |
| portfolio live | 排除 |

## Scope 到 downstream 策略

第一版不要用一个全量 downstream selector 处理所有回填。应按 `target_scope` 显式映射：

| `target_scope` | source/raw 范围 | downstream 策略 |
| --- | --- | --- |
| `baostock_daily_kline` | BaoStock daily kline source、compacted、raw | 股票行情核心链路、6 个 Furnace calc、股票行情 mart、技术指标 marts、指数/基准链路 |
| `market_events` | THS limit up pool 及 raw；不包含 Jiuyan action field | 对应 staging 和当前 downstream；若没有 mart 消费，仍只跑 staging/int 中存在的链路 |
| `eastmoney_f10` | EastMoney F10 source/raw | shares、exrights、financial valuation、stock quotes mart |
| `chinabond` | ChinaBond source/raw | government bond yields、risk free rate mart |
| `all_raw_yearly` | 本计划允许的日期型 source/raw；不直接透传包含 Jiuyan 的底层宽 scope | 全部非 Jiuyan、非 portfolio dbt staging/intermediate/marts + Furnace calculation |
| `snapshot_reference_data` | trade calendar、stock basic 等非 Jiuyan snapshot raw | 纳入本次迭代；`start_date/end_date` 必填但被该 scope 忽略 |
| `all_source_to_marts` | 本计划允许的日期型 + 非 Jiuyan snapshot reference data | 全部本次覆盖的 source/raw + 非 Jiuyan、非 portfolio stg/int/calc/mart，作为完整历史修复入口 |

实现时不能直接把底层 `market_events` 或 `all_raw_yearly` raw scope 透传到 source-to-marts controller，如果这些底层 scope 仍包含 Jiuyan action field、industry list 或 OCR steps。第一版必须在 source-to-marts registry 中显式拆出允许的非 Jiuyan source/raw steps。

## Downstream 语义

### dbt

当前 dbt staging/intermediate/marts 大多不是 Dagster 分区资产。历史 source/raw 回填后，第一版 downstream dbt 应采用固定 selector 全量重建相关 table/view，而不是尝试根据 source 回填日期局部更新。

允许后续优化：

- 对适合分区增量的模型单独评估 `insert_overwrite`。
- 对重型 tests 做窗口化。
- 对 `mart_stock_quotes_daily` 等重型模型建立 query log 基准。

### Furnace

历史回填不能使用 daily `append-latest` 语义。第一版规则：

| 场景 | Furnace mode |
| --- | --- |
| latest trade date 日常刷新 | `append-latest`，由 `daily__fetch_history_sources_to_marts_schedule_job` 管理 |
| 历史区间 source/raw 回填 | `replace-cascade` |
| dry-run 验证 | `dry-run` |

`backfill__fetch_history_sources_to_marts_job` 需要把 `start_date/end_date` 映射为 Furnace `request_from/request_to`。对影响状态递推的指标，Rust/Furnace 负责按 `replace-cascade` 级联到受影响证券的最新输入交易日。

### Portfolio backtest analytics

Portfolio backtest analytics 不纳入 `backfill__fetch_history_sources_to_marts_job` 第一版。它的上游是 Rust portfolio worker 写入的 `fleur_portfolio/portfolio_run_snapshot` 和 `fleur_calculation/calc_portfolio_*` 外部 source tables，不是 source -> raw 链路产物。

如果需要刷新 `int_portfolio_*` wrappers 或 `mart_portfolio_*_rank` marts，应另开 portfolio analytics job/backfill plan；如果需要重新产生 `calc_portfolio_*`，应另开 Rearview/worker backfill 计划。

## 实施阶段

### Phase 0: 文档和入口命名定稿

修改范围：

- `docs/plans/archive/0066-backfill-source-to-marts-controller-plan.md`
- `docs/plans/archive/0067-daily-source-to-marts-clean-slate-orchestration-plan.md`
- `docs/skills/fleur-dagster-backfill-runbook/SKILL.md`

任务：

1. 确认 `backfill__fetch_history_sources_to_raw_job` 保留 raw-only 语义。
2. 确认新增 job 名为 `backfill__fetch_history_sources_to_marts_job`。
3. 确认 `backfill__fetch_snapshot_sources_to_raw_job` 在本计划中移除。
4. 确认 `backfill__fetch_history_sources_to_raw_job` 已作为 raw-only 前置能力，不再列入 0066 待实现项。
5. 确认 `start_date/end_date` 对所有 scope 保持必填，snapshot reference data scope 忽略它们。
6. 确认 Jiuyan 异动、行业列表和 OCR 后续独立规划。
7. 在 runbook 中预留 source-to-marts 入口说明。
8. 明确 `backfill__fetch_history_sources_to_marts_job` 不属于 daily production schedule。

完成标准：

- 文档中没有要求把 raw-only job 原地扩展为 source-to-marts。
- 文档中没有继续把 `backfill__fetch_history_sources_to_raw_job` 的能力扩展列为 0066 实施任务。
- 文档中没有继续要求保留 `backfill__fetch_snapshot_sources_to_raw_job`。
- runbook 更新任务能区分 raw-only 和 source-to-marts。

验证：

```bash
make docs-check
git diff --check
```

### Phase 1: Source-to-marts registry

修改范围：

- `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`
- `pipeline/scheduler/tests/unit/automation/`

任务：

1. 新增 `source_to_marts_backfill.py`，不要把 downstream 逻辑塞进 source/raw controller。
2. 定义 `SourceToMartsBackfillConfig`：
   - `target_scope`
   - `start_date`，必填；snapshot reference data scope 忽略。
   - `end_date`，必填；snapshot reference data scope 忽略。
   - `execution_mode`
   - `refresh_prerequisite_snapshots`
   - `overwrite_source_partitions`
   - `dry_run`
3. 定义 `SourceToMartsPlan`，由 source/raw plan + downstream plan 组成。
4. 建立 `target_scope` 到允许 source/raw steps 和 downstream selector 的 registry。
5. 当底层 source/raw scope 包含 Jiuyan steps 时，source-to-marts registry 必须显式过滤或拆分，不能直接透传。
6. 测试每个 scope 的 source/raw step selection 和 downstream selection 是否符合上表。

完成标准：

- dry-run 能输出 source/raw stages 和 downstream stages。
- registry drift test 覆盖非 Jiuyan、非 portfolio dbt、Furnace 和 mart asset 变更。
- Jiuyan action field、industry list、OCR 相关 source/raw/staging/downstream 不进入任何 source-to-marts expanded plan。

验证：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/automation
```

### Phase 2: 新增 controller job

修改范围：

- `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`
- `pipeline/scheduler/src/scheduler/defs/automation/source_raw_backfill.py`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/tests/`

任务：

1. 新增 op：`backfill__fetch_history_sources_to_marts_controller`。
2. 新增 job：`backfill__fetch_history_sources_to_marts_job`。
3. 移除 `backfill__fetch_snapshot_sources_to_raw_job` 和 `backfill__fetch_snapshot_sources_to_raw_controller` 的注册。
4. dry-run 只输出 plan，不提交 child runs。
5. 非 dry-run 按阶段提交 child runs。
6. 写入统一 tags：
   - `backfill.kind=fetch_history_sources_to_marts`
   - `backfill.id`
   - `backfill.target_scope`
   - `backfill.start_date`
   - `backfill.end_date`
   - `backfill.stage`
   - `backfill.parent_run_id`
7. source/raw 阶段失败时不提交 downstream。
8. downstream 阶段失败时保留可恢复 stage 信息。
9. dry-run 和真实提交都必须证明 Jiuyan steps 未被带入。

完成标准：

- `dg list defs` 能看到 `backfill__fetch_history_sources_to_marts_job`。
- `backfill__fetch_history_sources_to_raw_job` 仍存在。
- `backfill__fetch_snapshot_sources_to_raw_job` 不再注册。
- `backfill__fetch_history_sources_to_marts_job` 的 expanded plan 不包含 Jiuyan assets。

验证：

```bash
cd pipeline
uv run dg list defs --target-path scheduler --json | jq -r '.jobs[].name'
cd scheduler
uv run dg check defs
```

### Phase 3: Snapshot reference data downstream

修改范围：

- `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`
- `docs/skills/fleur-dagster-backfill-runbook/SKILL.md`
- `pipeline/scheduler/tests/`

任务：

1. 在 `backfill__fetch_history_sources_to_marts_job` 中支持 `snapshot_reference_data` downstream。
2. `SourceToMartsBackfillConfig` 继续要求 `start_date` 和 `end_date` 必填。
3. `snapshot_reference_data` 的 source/raw stage 复用既有 raw-only 能力，并主动忽略 `start_date/end_date`，不产生日期分区选择。
4. `snapshot_reference_data` 只覆盖非 Jiuyan snapshot reference data。
5. 不支持 Jiuyan action field、industry list、`jiuyan_ocr_pipeline` 或相关 downstream；后续另开独立 job 计划。

完成标准：

- snapshot reference data dry-run 能输出清晰计划。
- Web UI / typed config 中 `start_date/end_date` 仍为必填。
- Jiuyan assets 不出现在本次 source/raw 或 source-to-marts scope 中。

验证：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/automation
cd scheduler
uv run dg check defs
```

### Phase 4: 运行手册和验收报告

修改范围：

- `docs/skills/fleur-dagster-backfill-runbook/SKILL.md`
- `docs/jobs/reports/`
- `docs/jobs/README.md`
- `docs/architecture/scheduler-architecture.md`

任务：

1. 更新 backfill runbook：
   - raw-only 修复使用 `backfill__fetch_history_sources_to_raw_job`。
   - snapshot reference data raw-only 修复同样使用 `backfill__fetch_history_sources_to_raw_job`，并说明日期参数会被忽略。
   - source-to-marts 历史修复使用 `backfill__fetch_history_sources_to_marts_job`。
   - downstream-only 重建使用 `execution_mode=downstream_only`。
   - Jiuyan 异动、行业列表和 OCR 后续独立 job，本次不提供统一入口模板。
2. 记录至少一次 dry-run report。
3. 若执行真实小范围 backfill，记录 child runs、范围、结果和失败恢复方式。
4. 更新 scheduler 架构文档，说明 backfill source-to-marts 与 daily source-to-marts 的区别。

完成标准：

- runbook 给出 `backfill__fetch_history_sources_to_marts_job` dry-run 示例。
- 有运行报告证明 plan expansion 正确；当前记录见 [2026-07-01 backfill source-to-marts dry-run](../../jobs/reports/2026-07-01-backfill-source-to-marts-controller-dry-run.md)。

验证：

```bash
make docs-check
git diff --check
```

## 禁止模式

1. 不允许把 `backfill__fetch_history_sources_to_raw_job` 原地改成 source-to-marts，导致 raw-only 入口消失。
2. 不允许继续保留 `backfill__fetch_snapshot_sources_to_raw_job` 作为公开入口。
3. 不允许把 `start_date/end_date` 改成可选字段；snapshot reference data 必须显式忽略日期。
4. 不允许把 Jiuyan action field、industry list、OCR 或相关 downstream 混入本次 source-to-marts 的 source/raw stage 或 downstream scope。
5. 不允许 source-to-marts controller 直接调用 dbt CLI 或 Furnace CLI 绕过 Dagster assets。
6. 不允许所有 scope 默认重建全部 marts；必须按 scope 显式映射。
7. 不允许历史 Furnace 修复使用 `append-latest`。
8. 不允许在第一版物化 portfolio live。
9. 不允许把 portfolio worker backfill 混入本 job。
10. 不允许用 `_sync_at` 当作 downstream affected range 的事实来源。
11. 不允许直接透传底层包含 Jiuyan 的 `market_events` 或 `all_raw_yearly` raw scope。

## 测试策略

| 范围 | 测试 |
| --- | --- |
| Config validation | 所有 scope 都要求 `start_date/end_date`；snapshot reference data 忽略这两个参数 |
| Plan expansion | 每个 target scope 展开 source/raw + downstream stages |
| Raw-only preservation | `backfill__fetch_history_sources_to_raw_job` 仍注册且保留 raw-only 语义 |
| Snapshot job removal | `backfill__fetch_snapshot_sources_to_raw_job` 不再注册 |
| Jiuyan exclusion | Jiuyan action field、industry list、`jiuyan_ocr_pipeline` 和相关 assets 不在本次 scope 枚举、plan expansion 或 runbook 模板中 |
| Downstream scope | BaoStock/EastMoney/ChinaBond/market_events 各自只选择相关非 Jiuyan、非 portfolio downstream |
| Furnace mode | 历史范围使用 `replace-cascade`，dry-run 使用 `dry-run` |
| Failure handling | source/raw 失败不提交 downstream |
| Tags | parent/child runs 写入统一 `backfill.kind=fetch_history_sources_to_marts` tags |
| Definition surface | `backfill__fetch_history_sources_to_marts_job` 和 `backfill__fetch_history_sources_to_raw_job` 同时存在，`backfill__fetch_snapshot_sources_to_raw_job` 不再注册 |
| Portfolio exclusion | `portfolio_run_snapshot`、`calc_portfolio_*`、`int_portfolio_*` 和 `mart_portfolio_*_rank` 不进入 source-to-marts plan |

## 验收标准

1. `backfill__fetch_history_sources_to_marts_job` 已注册。
2. `backfill__fetch_history_sources_to_raw_job` 仍保留 raw-only 语义。
3. `backfill__fetch_snapshot_sources_to_raw_job` 已移除。
4. `start_date/end_date` 保持必填；`snapshot_reference_data` 忽略它们。
5. Jiuyan action field、industry list、`jiuyan_ocr_pipeline` 和相关 downstream 不在本次迭代 scope 中。
6. dry-run 能输出完整 source/raw/downstream 计划。
7. `execution_mode=downstream_only` 能在不提交 source/raw runs 的情况下重建 downstream。
8. BaoStock 历史范围能映射到 quote int、Furnace `replace-cascade` 和 stock marts。
9. EastMoney 历史范围能映射到 shares/exrights/valuation/quotes mart downstream。
10. ChinaBond 历史范围能映射到 risk free downstream。
11. portfolio backtest analytics 和 portfolio live 不在 plan 中。
12. tests、`dg check defs` 和 docs-check 通过。
13. backfill runbook 已更新。

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
```

## 完成后的维护动作

1. 将本计划状态更新为 `Completed` 并移入 `docs/plans/archive/`。
2. 更新 `docs/plans/README.md` 的 Recently Completed。
3. 更新 backfill runbook 中的命令模板。
4. 输出 dry-run 或真实小范围 backfill 报告。
5. 如后续需要 portfolio worker backfill，新增独立 Rearview/portfolio worker backfill plan。
