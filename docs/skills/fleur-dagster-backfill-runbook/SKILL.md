---
name: fleur-dagster-backfill-runbook
description: fleur 的 Dagster 回填操作手册。用于选择 dg launch 命令、资产选择、partition 或 partition-range 参数，以及各数据源的回填模板。
---

# DG 回填手册

当 `pipeline/scheduler` 里的回填可以用 `dg launch` 表达时，使用这个 skill。

## 规则

- 所有 `dg` / `dagster` 命令必须使用根目录 `.env` 中的 `DAGSTER_HOME` 作为 Dagster home
- 执行前先在仓库根目录加载 `.env`：`set -a; . ./.env; set +a`
- 运行回填前先执行 `make dagster-home`，确保 Dagster home 和 pool 限制已初始化
- 在 `pipeline/` 下执行命令
- 使用 `uv run dg ...`
- 通过 `--target-path scheduler` 指向 scheduler 项目
- 临时回填优先用明确的 asset selection
- 只有当 job 和目标工作负载完全一致时才用 job

## 流程

1. Source 到 ClickHouse raw 的手动修复优先使用统一 raw-only 入口 `backfill__fetch_history_sources_to_raw_job`。
2. Snapshot reference data 也使用 `backfill__fetch_history_sources_to_raw_job`；`start_date` 和 `end_date` 仍必填，但该 scope 会主动忽略日期。
3. 历史 source/raw 修复后需要继续推进 dbt、Furnace 和 marts 时，使用 `backfill__fetch_history_sources_to_marts_job`。
4. 日常 source/raw/downstream 主入口使用 `daily__fetch_history_sources_to_marts_schedule_job`；它不是 history backfill，但复用 source-to-marts registry，把 `target_date` 映射为单日增量。
5. Jiuyan 异动、行业列表、图片/OCR/OCR snapshot 后续单独设计 job；不纳入本轮统一 source-to-marts 回填模板或 daily source-to-marts 入口。
6. 只有调试单个资产、验证单个分区或统一入口暂不覆盖的临时操作，才使用精确 asset selection。
7. 判断目标是否分区。
8. 从 [references/backfill-matrix.md](references/backfill-matrix.md) 里选命令模板。
9. 需要时先用 `uv run dg list defs --target-path scheduler --json` 验证选择。
10. 先跑 `dry_run: true`，确认计划后再执行真实回填或真实 daily run。

## 统一 Source/Raw 回填入口

raw-only 首选手动入口：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_history_sources_to_raw_job \
  --config-json '{
    "ops": {
      "backfill__fetch_history_sources_to_raw_controller": {
        "config": {
          "target_scope": "baostock_daily_kline",
          "start_date": "2026-01-01",
          "end_date": "2026-06-30",
          "execution_mode": "full",
          "refresh_prerequisite_snapshots": false,
          "overwrite_source_partitions": false,
          "dry_run": true
        }
      }
    }
  }'
```

raw-only `target_scope`：

- `baostock_daily_kline`
- `market_events`（只纳入 THS limit up pool；Jiuyan 异动不进入统一 raw-only 或 source-to-marts 入口）
- `eastmoney_f10`
- `chinabond`
- `snapshot_reference_data`（日期字段必填，但该 scope 忽略日期）
- `all_raw_yearly`

snapshot reference data raw-only dry-run 示例：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_history_sources_to_raw_job \
  --config-json '{
    "ops": {
      "backfill__fetch_history_sources_to_raw_controller": {
        "config": {
          "target_scope": "snapshot_reference_data",
          "start_date": "2024-01-01",
          "end_date": "2024-01-01",
          "execution_mode": "full",
          "dry_run": true
        }
      }
    }
  }'
```

覆盖本轮 source/raw 能力时，在 Web UI 中分两次启动：`all_raw_yearly` 加真实日期区间，`snapshot_reference_data` 加占位日期区间。Jiuyan 异动、行业列表和 OCR 后续独立规划，不再通过统一模板拼入本轮 source-to-marts 入口。

执行规则：

- `dry_run: true` 只输出 source、compacted source 和 raw sync 的计划。
- `dry_run: false` 会按计划顺序创建 child materialization runs，并写入统一 `backfill.*` tags。
- `execution_mode: raw_only` 只重跑 raw sync，假设 source/compacted source 已经成功存在。
- `refresh_prerequisite_snapshots` 只刷新当前 scope 显式声明的 source snapshot prerequisites，不刷新无关 snapshot。
- BaoStock 当前年 partial range 会向 daily 和 compacted source 传 `cutoff_trade_date`。
- EastMoney 和 ChinaBond partial current year 会向 year source 传 `refresh_until_date`。

## 统一 Source-to-Marts 回填入口

历史 source/raw 修复后继续重建 downstream 的手动入口：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_history_sources_to_marts_job \
  --config-json '{
    "ops": {
      "backfill__fetch_history_sources_to_marts_controller": {
        "config": {
          "target_scope": "all_source_to_marts",
          "start_date": "2024-01-01",
          "end_date": "2024-12-31",
          "execution_mode": "full",
          "refresh_prerequisite_snapshots": false,
          "overwrite_source_partitions": false,
          "dry_run": true
        }
      }
    }
  }'
```

`target_scope`：

- `baostock_daily_kline`
- `market_events`（只纳入 THS limit up pool；排除 Jiuyan 异动）
- `eastmoney_f10`
- `chinabond`
- `snapshot_reference_data`（日期字段必填，但 source/raw stage 忽略日期）
- `all_raw_yearly`
- `all_source_to_marts`

`execution_mode`：

- `full`：source/raw + dbt staging/intermediate + Furnace calculation + marts。
- `source_raw_only`：只展开 source/raw 阶段，用于和 raw-only 入口对比计划。
- `downstream_only`：假设 source/raw 已完成，只重建 downstream。

Source-to-marts 规则：

- `dry_run: true` 只输出 expanded plan，不提交 child materialization runs。
- 历史 Furnace 修复使用 `replace-cascade`；dry-run child config 使用 `dry-run`。
- Jiuyan 异动、行业列表、图片/OCR/OCR snapshot 及相关 staging/downstream 不进入 source-to-marts plan。
- Portfolio backtest analytics 和 portfolio live 不进入 source-to-marts plan。

## Daily Source-to-Marts 入口

日常 source -> raw -> stg -> int -> calculation -> mart 主入口：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job daily__fetch_history_sources_to_marts_schedule_job \
  --config-json '{
    "ops": {
      "daily__fetch_history_sources_to_marts_schedule_controller": {
        "config": {
          "target_scope": "all_source_to_marts",
          "target_date": "2026-06-30",
          "execution_mode": "full",
          "refresh_prerequisite_snapshots": false,
          "overwrite_source_partitions": false,
          "dry_run": true
        }
      }
    }
  }'
```

Daily 规则：

- `target_date` 必填；daily wrapper 将它映射为 history source-to-marts 的 `start_date=end_date=target_date`。
- `snapshot_reference_data` 仍要求传 `target_date`，但底层 snapshot source/raw plan 忽略日期窗口。
- `dry_run: true` 只输出 expanded daily plan，不提交 child materialization runs。
- 非 dry-run daily Furnace step 使用 `append-latest`；历史修复仍走 `backfill__fetch_history_sources_to_marts_job` 的 `replace-cascade`。
- `daily__fetch_history_sources_to_marts_schedule` 是唯一 daily source-to-marts ScheduleDefinition，默认 stopped；当前 schedule config 为 `dry_run: false`，启用后提交真实 daily runs。
- 当前 dry-run 验证记录：`docs/jobs/reports/2026-07-01-daily-fetch-history-sources-to-marts-schedule-job-dry-run.md`。
- Jiuyan 全系列和 portfolio backtest analytics 不进入 daily source-to-marts plan；portfolio live 仅作为 `all_source_to_marts + full` 的 terminal step 提交。

## 选择规则

- 能精确选 asset 时优先精确选：`key:source/ths__limit_up_pool`
- 需要按数据源放大范围时用 tag：`tag:source=ths`
- 只有想选整个源 bundle 时才用 `group:s3_sources`

## 分区规则

- 日分区资产用包含式范围：`--partition-range "2024-01-01...2024-01-31"`
- 年分区资产每次跑一个年分区：`--partition 2024`
- 年分区资产跨很多年时，按年份循环，不要直接拉长范围
- 遵守每个 asset 自己的回填窗口限制
- Eastmoney 的并行度依赖 `eastmoney_run_pool`，当前上限为 3 个 run

## 常用命令

```bash
cd pipeline

uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2024-01-01...2024-01-31"
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition 2024-01-02
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition-range "2026-01-01...2026-06-24" --config-json '{"ops":{"source__baostock__query_history_k_data_plus_daily":{"config":{"overwrite_existing_partitions":false,"cutoff_trade_date":"2026-06-24"}}}}'
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily_compacted" --partition 2024
uv run dg launch --target-path scheduler --assets "key:source/eastmoney__balance" --partition 2024
```

## 什么时候改用 Python CLI

如果回填需要下面这些能力，就改用 Python 包装器：

- 自动展开分区
- 多 run 重试
- 进度记录
- 可恢复执行
- 多 asset 批量提交

## ClickHouse raw sync 分区规则

ClickHouse 四层 database 迁移（`fleur_raw` / `fleur_staging` / `fleur_intermediate` / `fleur_marts`）已于 2026-06-02 完成；决策见 `docs/ADR/0009-clickhouse-layered-databases.md`，执行记录见 `docs/plans/archive/0026-clickhouse-layered-database-migration-implementation-plan.md`。下面只保留日常 raw sync 回填的可复用规则。

年分区 raw sync 按 `--partition YYYY` 单年运行，跨年时按年份循环：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/<dataset>" \
  --partition YYYY
```

snapshot raw sync 不传 partition：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/<snapshot-dataset>"
```

不要用旧 `clickhouse__raw_sync_all_job` 作为历史分区已全部同步的证明；该 job
当前不再 registered。历史同步范围以 unified controller plan 或逐分区 run log 为准。
