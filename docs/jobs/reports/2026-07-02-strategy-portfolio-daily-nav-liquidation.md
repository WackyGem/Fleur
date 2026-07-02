# 2026-07-02 Strategy Portfolio Daily NAV Liquidation

日期：2026-07-02

范围：`rearview/daily__portfolio_nav_liquidation`、`daily__fetch_history_sources_to_marts_schedule_job` terminal step、Rearview single-day daily-runs API、Scheduler definitions 和相关测试。

## 结论

Plan 0073 已完成。Production portfolio live 清算入口从 `rearview/strategy_portfolio_daily_runs` 收敛为无分区资产 `rearview/daily__portfolio_nav_liquidation`，并进入 `daily__fetch_history_sources_to_marts_schedule_job` 的 `all_source_to_marts + full` terminal step。

当前 production path：

- 不再暴露 `trade_date/start_date/end_date/strategy_portfolio_id/chunk_size` config。
- 不再注册 `strategy_portfolio__daily_run_job` 或 `portfolio__daily_run_schedule`。
- 默认调用现有 Rearview `settlement-target`、single-day `daily-runs`、status 和 fact-count APIs。
- `example__portfolio_live_job` 仍保留为 0051 手动回归入口。

## Dry-Run Plan

命令：

```bash
cd pipeline
uv run python - <<'PY'
from datetime import date
from scheduler.defs.automation.source_raw_backfill import EXECUTION_MODE_FULL
from scheduler.defs.automation.source_to_marts_backfill import ALL_SOURCE_TO_MARTS_SCOPE
from scheduler.defs.daily.source_to_marts import DailyFetchHistorySourcesToMartsRequest, build_daily_fetch_history_sources_to_marts_plan

plan = build_daily_fetch_history_sources_to_marts_plan(
    DailyFetchHistorySourcesToMartsRequest(
        target_scope=ALL_SOURCE_TO_MARTS_SCOPE,
        target_date="2026-06-30",
        execution_mode=EXECUTION_MODE_FULL,
        refresh_prerequisite_snapshots=False,
        overwrite_source_partitions=False,
        dry_run=True,
    ),
    today=date(2026, 6, 30),
    controller_run_id="report-dry-run-0073",
)
last = plan.steps[-1]
print(len(plan.steps), last.label, last.stage, last.asset_keys, last.partition.label(), last.tags)
PY
```

结果：

| 字段 | 值 |
|---|---|
| step_count | `18` |
| last_label | `portfolio live nav liquidation` |
| last_stage | `portfolio_live_liquidation` |
| last_assets | `rearview/daily__portfolio_nav_liquidation` |
| last_partition | `unpartitioned` |
| tags | 包含 `daily.parent_run_id=report-dry-run-0073`、`daily.stage=portfolio_live_liquidation`、`daily.step=asset_materialization` |

## Direct Asset Smoke

命令：

```bash
cd pipeline/scheduler
PYTHONWARNINGS=ignore DAGSTER_HOME=/storage/program/fleur/.dagster \
  uv run dg launch --assets "key:rearview/daily__portfolio_nav_liquidation"
```

结果：

| 字段 | 值 |
|---|---|
| Dagster run id | `33774c70-10de-45a1-b8ed-1af7d8224a77` |
| Dagster status | `RUN_SUCCESS` |
| step | `rearview__daily__portfolio_nav_liquidation` |
| step duration | `381ms` |
| materialized asset | `rearview/daily__portfolio_nav_liquidation` |

Materialization metadata：

| Metadata | 值 |
|---|---|
| target_trade_date | `2026-07-02` |
| settlement_target_date | `2026-07-02` |
| active_portfolio_count | `1` |
| created_run_count | `0` |
| skipped_run_count | `1` |
| succeeded_run_count | `1` |
| failed_run_count | `0` |
| timeout_run_count | `0` |
| latest_daily_run_id | `01KWJ4553QHK2F52S3VZZ3N410` |
| latest_result_attempt_id | `01KWJ45HWZA5Q5MC7WEENJRMFV` |
| nav_row_count | `603` |
| trade_row_count | `1271` |
| closed_trade_row_count | `635` |

解释：本次 smoke 复用了已存在的 target daily run，因此 `created_run_count=0`、`skipped_run_count=1`。Dagster materialization 成功代表 worker status 已到 `succeeded`，且 fact-count API 返回了 live NAV/trade/closed trade 行数。

## Definitions Evidence

命令：

```bash
cd pipeline/scheduler
uv run dg list defs --assets "key:rearview/daily__portfolio_nav_liquidation" --json
uv run dg list defs --json | rg '"asset_key": "rearview/strategy_portfolio_daily_runs"|"asset_key": "rearview/daily__portfolio_nav_liquidation"|"name": "portfolio__daily_run_schedule"|"name": "strategy_portfolio__daily_run_job"|"name": "example__portfolio_live_job"|"name": "daily__fetch_history_sources_to_marts_schedule"'
```

结果：

- `rearview/daily__portfolio_nav_liquidation` registered, group `rearview`, executable `true`。
- `rearview/strategy_portfolio_daily_runs` 未出现。
- `daily__fetch_history_sources_to_marts_schedule` 仍是唯一 production daily schedule。
- `example__portfolio_live_job` 仍存在。
- `portfolio__daily_run_schedule` 和 `strategy_portfolio__daily_run_job` 未出现。

注意：`dg list defs` / tests / `dg check defs` 不要并行运行；Dagster dbt component 的 `.local_defs_state` 会出现临时竞态。串行执行通过。

## 验证命令

通过：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests --check
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/rearview scheduler/tests/unit/daily scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
```

通过：

```bash
make docs-check
git diff --check
```
