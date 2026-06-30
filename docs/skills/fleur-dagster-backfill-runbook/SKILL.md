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

1. Source 到 ClickHouse raw 的日期型回填优先使用统一入口 `backfill__fetch_sources_to_raw_job`。
2. Snapshot reference data 和 Jiuyan OCR 这种非日期型回填使用 `backfill__fetch_snapshot_sources_to_raw_job`。
3. 只有调试单个资产、验证单个分区或统一入口暂不覆盖的临时操作，才使用精确 asset selection。
4. 判断目标是否分区。
5. 从 [references/backfill-matrix.md](references/backfill-matrix.md) 里选命令模板。
6. 需要时先用 `uv run dg list defs --target-path scheduler --json` 验证选择。
7. 先跑 `dry_run: true`，确认计划后再执行真实回填。

## 统一 Source/Raw 回填入口

日期型 scope 的首选手动入口：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_sources_to_raw_job \
  --config-json '{
    "ops": {
      "backfill__fetch_sources_to_raw_controller": {
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

日期型 `target_scope`：

- `baostock_daily_kline`
- `market_events`
- `eastmoney_f10`
- `chinabond`
- `all_raw_yearly`

非日期型 scope 使用 `backfill__fetch_snapshot_sources_to_raw_job`，Web UI 不展示 `start_date` / `end_date`：

- `snapshot_reference_data`
- `jiuyan_ocr_pipeline`

覆盖全部 source/raw 时，在 Web UI 中分三次启动：先用日期型入口选择 `all_raw_yearly` 并填写日期区间，再用非日期型入口分别选择 `snapshot_reference_data` 和 `jiuyan_ocr_pipeline`。

非日期型 dry-run 示例：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_snapshot_sources_to_raw_job \
  --config-json '{
    "ops": {
      "backfill__fetch_snapshot_sources_to_raw_controller": {
        "config": {
          "target_scope": "snapshot_reference_data",
          "execution_mode": "full",
          "dry_run": true
        }
      }
    }
  }'
```

执行规则：

- `dry_run: true` 只输出 source、compacted source 和 raw sync 的计划。
- `dry_run: false` 会按计划顺序创建 child materialization runs，并写入统一 `backfill.*` tags。
- `execution_mode: raw_only` 只重跑 raw sync，假设 source/compacted source 已经成功存在。
- `refresh_prerequisite_snapshots` 只刷新当前 scope 显式声明的 source snapshot prerequisites，不刷新无关 snapshot。
- BaoStock 当前年 partial range 会向 daily 和 compacted source 传 `cutoff_trade_date`。
- EastMoney 和 ChinaBond partial current year 会向 year source 传 `refresh_until_date`。
- Jiuyan OCR 默认 `jiuyan_ocr_limit: 100`，显式 `null` 才表示不限制。

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
uv run dg launch --target-path scheduler --job eastmoney__daily_job --partition 2024
uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/jiuyan__industry_ocr_snapshot"
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

不要用 `clickhouse__raw_sync_all_job` 作为历史分区已全部同步的证明；它只证明
asset selection 覆盖所有 raw sync assets。历史同步范围以逐分区 run log 为准。
