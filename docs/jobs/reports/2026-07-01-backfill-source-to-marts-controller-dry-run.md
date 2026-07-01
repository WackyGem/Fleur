# Backfill Source-to-Marts Controller Dry-Run 验证记录

日期：2026-07-01

## 范围

- Job：`backfill__fetch_history_sources_to_marts_job`
- Controller op：`backfill__fetch_history_sources_to_marts_controller`
- `target_scope`：`all_source_to_marts`
- 区间：`2024-01-01..2024-12-31`
- 模式：`execution_mode=full`，`dry_run=true`

## 命令

```bash
cd pipeline
set -a
. ../.env
set +a
uv run dg launch --target-path scheduler \
  --job backfill__fetch_history_sources_to_marts_job \
  --config-json '{"ops":{"backfill__fetch_history_sources_to_marts_controller":{"config":{"target_scope":"all_source_to_marts","start_date":"2024-01-01","end_date":"2024-12-31","execution_mode":"full","refresh_prerequisite_snapshots":false,"overwrite_source_partitions":false,"dry_run":true}}}}'
```

## 结果

命令成功结束，controller run id：

```text
46648adc-c7ca-4b8d-8f18-91c6f44e4e71
```

生成的 `backfill.id`：

```text
all_source_to_marts-2024-01-01-2024-12-31-46648adcc7ca
```

Planned child step tags 均包含：

```text
backfill.parent_run_id=46648adc-c7ca-4b8d-8f18-91c6f44e4e71
backfill.kind=fetch_history_sources_to_marts
```

Dry-run 展开为 17 个步骤：

1. `source_raw/source_snapshot`：`source/sina__trade_calendar`、`source/baostock__query_stock_basic`。
2. `source_raw/raw`：`clickhouse/raw/sina__trade_calendar`、`clickhouse/raw/baostock__query_stock_basic`。
3. `source_raw/source_daily`：`source/baostock__query_history_k_data_plus_daily`，partition range `2024-01-01...2024-12-31`。
4. `source_raw/source_compacted`：`source/baostock__query_history_k_data_plus_daily_compacted`，partition `2024`。
5. `source_raw/raw`：`clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`，partition `2024`。
6. `source_raw/source_daily`：`source/ths__limit_up_pool`，partition range `2024-01-01...2024-12-31`。
7. `source_raw/source_compacted`：`source/ths__limit_up_pool_compacted`，partition `2024`。
8. `source_raw/raw`：`clickhouse/raw/ths__limit_up_pool_compacted`，partition `2024`。
9. `source_raw/source_year`：9 个 EastMoney F10 source assets，partition `2024`。
10. `source_raw/raw`：9 个 EastMoney F10 raw assets，partition `2024`。
11. `source_raw/source_year`：`source/chinabond__government_bond`，partition `2024`。
12. `source_raw/raw`：`clickhouse/raw/chinabond__government_bond`，partition `2024`。
13. `dbt_staging/dbt_staging`：14 个非 Jiuyan staging assets。
14. `dbt_intermediate/dbt_intermediate`：14 个非 portfolio intermediate assets。
15. `furnace_calculation/furnace_calculation`：6 个股票技术指标 calculation assets。
16. `dbt_calculation_wrappers/dbt_calculation_wrappers`：6 个股票技术指标 dbt wrappers。
17. `dbt_marts/dbt_marts`：9 个非 portfolio marts。

Furnace dry-run child config 使用：

```text
request_from=2024-01-01
request_to=2024-12-31
mode=dry-run
symbols=[]
```

本次没有提交真实 child materialization runs，没有触发远端 source 抓取、ClickHouse raw sync、dbt build 或 Furnace 写入。

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

- `backfill__fetch_history_sources_to_marts_job` 可通过 CLI/Web UI 启动并输出完整 source/raw/downstream plan。
- `all_source_to_marts` 覆盖本轮允许的 source/raw、非 Jiuyan/非 portfolio dbt downstream 和 6 个 Furnace stock calculation assets。
- Jiuyan 和 portfolio 相关链路按 Plan 0066 要求排除。
- 真实执行前应先复用同一 config 确认 dry-run plan，再把 `dry_run` 改为 `false`。
