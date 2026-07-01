# Daily Source-to-Marts Schedule Job Dry-Run 验证记录

日期：2026-07-01

## 范围

- Job：`daily__fetch_history_sources_to_marts_schedule_job`
- Controller op：`daily__fetch_history_sources_to_marts_schedule_controller`
- `target_scope`：`all_source_to_marts`
- `target_date`：`2026-06-30`
- 模式：`execution_mode=full`，`dry_run=true`

## 命令

```bash
cd pipeline
set -a
. ../.env
set +a
uv run dg launch --target-path scheduler \
  --job daily__fetch_history_sources_to_marts_schedule_job \
  --config-json '{"ops":{"daily__fetch_history_sources_to_marts_schedule_controller":{"config":{"target_scope":"all_source_to_marts","target_date":"2026-06-30","execution_mode":"full","refresh_prerequisite_snapshots":false,"overwrite_source_partitions":false,"dry_run":true}}}}'
```

## 结果

命令成功结束，controller run id：

```text
3b2bb23c-efdf-4cf5-8f90-375224c95f8b
```

生成的 `daily.id`：

```text
all_source_to_marts-2026-06-30-3b2bb23cefdf
```

Planned child step tags 均包含：

```text
daily.kind=fetch_history_sources_to_marts_schedule
daily.id=all_source_to_marts-2026-06-30-3b2bb23cefdf
daily.target_scope=all_source_to_marts
daily.target_date=2026-06-30
daily.execution_mode=full
daily.parent_run_id=3b2bb23c-efdf-4cf5-8f90-375224c95f8b
```

Dry-run 展开为 17 个步骤：

1. `source_raw/source_snapshot`：`source/sina__trade_calendar`、`source/baostock__query_stock_basic`。
2. `source_raw/raw`：`clickhouse/raw/sina__trade_calendar`、`clickhouse/raw/baostock__query_stock_basic`。
3. `source_raw/source_daily`：`source/baostock__query_history_k_data_plus_daily`，partition range `2026-06-30...2026-06-30`。
4. `source_raw/source_compacted`：`source/baostock__query_history_k_data_plus_daily_compacted`，partition `2026`。
5. `source_raw/raw`：`clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`，partition `2026`。
6. `source_raw/source_daily`：`source/ths__limit_up_pool`，partition range `2026-06-30...2026-06-30`。
7. `source_raw/source_compacted`：`source/ths__limit_up_pool_compacted`，partition `2026`。
8. `source_raw/raw`：`clickhouse/raw/ths__limit_up_pool_compacted`，partition `2026`。
9. `source_raw/source_year`：9 个 EastMoney F10 source assets，partition `2026`，`refresh_until_date=2026-06-30`。
10. `source_raw/raw`：9 个 EastMoney F10 raw assets，partition `2026`。
11. `source_raw/source_year`：`source/chinabond__government_bond`，partition `2026`，`refresh_until_date=2026-06-30`。
12. `source_raw/raw`：`clickhouse/raw/chinabond__government_bond`，partition `2026`。
13. `dbt_staging/dbt_staging`：14 个非 Jiuyan staging assets。
14. `dbt_intermediate/dbt_intermediate`：14 个非 portfolio intermediate assets。
15. `furnace_calculation/furnace_calculation`：6 个股票技术指标 calculation assets。
16. `dbt_calculation_wrappers/dbt_calculation_wrappers`：6 个股票技术指标 dbt wrappers。
17. `dbt_marts/dbt_marts`：9 个非 portfolio marts。

Furnace dry-run child config 使用：

```text
request_from=2026-06-30
request_to=2026-06-30
mode=dry-run
symbols=[]
```

本次没有提交真实 child materialization runs，没有触发远端 source 抓取、ClickHouse raw sync、dbt build 或 Furnace 写入。

## Definitions Surface

`uv run dg list defs --target-path scheduler --json` 当前注册 jobs：

```text
backfill__fetch_history_sources_to_marts_job
backfill__fetch_history_sources_to_raw_job
daily__fetch_history_sources_to_marts_schedule_job
strategy_portfolio__daily_run_job
```

当前注册 schedules：

```text
daily__fetch_history_sources_to_marts_schedule
portfolio__daily_run_schedule
```

当前注册 sensors：

```text
default_automation_condition_sensor
slack_asset_failure_sensor
```

其中 `default_automation_condition_sensor` 由 Dagster automation condition 生成；本次清理后用户定义 production trigger 只保留 `slack_asset_failure_sensor`，旧的 `baostock_raw_sync_success_triggers_stock_daily_build` 不再注册。

## 排除项核验

展开计划中没有出现以下资产类别：

- Jiuyan 异动、行业列表、图片/OCR/OCR snapshot source 或 raw assets。
- `stg_jiuyan__*` downstream assets。
- `fleur_portfolio/portfolio_run_snapshot`。
- `fleur_calculation/calc_portfolio_*`。
- `int_portfolio_*`。
- `mart_portfolio_*_rank`。
- `rearview/strategy_portfolio_daily_runs`。

## 结论

- `daily__fetch_history_sources_to_marts_schedule_job` 可通过 CLI/Web UI 启动并输出完整 daily source/raw/downstream plan。
- `daily__fetch_history_sources_to_marts_schedule` 是唯一注册的 source-to-marts daily ScheduleDefinition，默认 stopped，当前 schedule config 保持 `dry_run=true`。
- 旧 production daily/transformation/source-specific jobs、source-specific schedules、ClickHouse raw sync jobs 和 `baostock_raw_sync_success_triggers_stock_daily_build` 不再注册。
- 真实生产启用前，需要把 schedule run config 的 `dry_run` 改为 `false`，并至少完成一次小范围 non-dry-run 验证。
