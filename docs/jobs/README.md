# Jobs

本目录记录 Dagster 运行手册、definitions snapshot 和实际运行报告。

## Runbooks

| 文档 | 用途 |
|---|---|
| [dagster-backfill-2026.md](dagster-backfill-2026.md) | Dagster 回填命令、范围和运行约束 |

## Snapshots

| 文档 | 用途 |
|---|---|
| [dagster-definitions-lineage-2026-06-10.md](dagster-definitions-lineage-2026-06-10.md) | 2026-06-10 的 Dagster assets、jobs、schedules、resources、sensors 和 lineage 快照 |

Snapshot 文档必须写明生成日期和生成命令。

## Reports

实际运行、回填、重跑和数据核验记录放在 [reports/](reports/)。

最近的运行验证：

| 报告 | 范围 |
|---|---|
| [2026-07-01-furnace-clickhouse-rust-client-migration.md](reports/2026-07-01-furnace-clickhouse-rust-client-migration.md) | Furnace 全部股票技术指标迁移到官方 `clickhouse` Rust HTTP client，并移除外部 `clickhouse-client` / Docker exec 运行时依赖 |
| [2026-07-01-daily-fetch-history-sources-to-marts-schedule-job-dry-run.md](reports/2026-07-01-daily-fetch-history-sources-to-marts-schedule-job-dry-run.md) | `daily__fetch_history_sources_to_marts_schedule_job` 的 `all_source_to_marts` 单日 dry-run plan expansion |
| [2026-07-01-backfill-source-to-marts-controller-dry-run.md](reports/2026-07-01-backfill-source-to-marts-controller-dry-run.md) | `backfill__fetch_history_sources_to_marts_job` 的 `all_source_to_marts` dry-run plan expansion |

运行报告至少包含：

- 日期或时间。
- 范围：资产、模型、表、分区、证券或请求区间。
- 命令或等价执行入口。
- 结果：状态、摘要、失败原因或验证结论。

## 校验

```bash
make docs-check
git diff --check
```
