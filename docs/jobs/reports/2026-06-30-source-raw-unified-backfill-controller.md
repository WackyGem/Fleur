# Source/Raw 统一回填 Controller 验证记录

日期：2026-06-30

## 范围

- Job：`backfill__fetch_sources_to_raw_job`
- Controller op：`backfill__fetch_sources_to_raw_controller`
- `target_scope`：`baostock_daily_kline`
- 区间：`2026-01-01..2026-06-30`
- 模式：`dry_run=true`

## 命令

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_sources_to_raw_controller":{"config":{"target_scope":"baostock_daily_kline","start_date":"2026-01-01","end_date":"2026-06-30","dry_run":true}}}}'
```

## 结果

命令成功结束，controller run id：

```text
404c43d0-6e6a-412d-8f52-9fd353da3fa1
```

生成的 `backfill.id`：

```text
baostock_daily_kline-2026-01-01-2026-06-30-404c43d06e6a
```

Dry-run 计划展开为 3 个步骤：

1. `source_daily`: `source/baostock__query_history_k_data_plus_daily`，partition range `2026-01-01...2026-06-30`，op config `cutoff_trade_date=2026-06-30`。
2. `source_compacted`: `source/baostock__query_history_k_data_plus_daily_compacted`，partition `2026`，op config `cutoff_trade_date=2026-06-30`。
3. `raw`: `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`，partition `2026`。

本次没有提交真实 child materialization runs，没有触发远端 source 抓取或 ClickHouse raw sync。

## 结论

- Web UI/CLI 可启动统一 controller job。
- `target_scope`、日期区间和默认参数能生成符合 RFC 0039 的执行计划。
- 后续真实回填应先使用同一 config 再确认 `dry_run=false`，并通过 `backfill.id` 聚合 controller run 与 child runs。

## 真实提交路径验证

范围：

- `target_scope`：`chinabond`
- 区间：`2006-01-01..2006-12-31`
- 模式：`execution_mode=raw_only`，`dry_run=false`

命令：

```bash
set -a; . ./.env; set +a
make dagster-home
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_sources_to_raw_controller":{"config":{"target_scope":"chinabond","start_date":"2006-01-01","end_date":"2006-12-31","execution_mode":"raw_only","dry_run":false}}}}'
```

运行事实：

- Controller run id：`246d76d7-6c12-4e1e-ac0b-129be9fe3f6a`
- `backfill.id`：`chinabond-2006-01-01-2006-12-31-246d76d76c12`
- Child run id：`37fa09a6-3cf1-4ff1-b87b-88eb8bf9993d`
- Child asset：`clickhouse/raw/chinabond__government_bond`
- Partition：`2006`

结果：child raw sync run 被成功创建并进入 Dagster 执行，随后失败。失败原因是 source S3 parquet 不存在：

```text
source/chinabond__government_bond/year=2006/000000_0.parquet
```

ClickHouse/S3 返回 404 后，controller 按设计抛出 `RuntimeError: Backfill step failed... status=FAILURE`，没有继续提交后续步骤。

结论：

- `dry_run=false` 的 child run 创建、`backfill.*` tags 透传和失败传播路径已验证。
- 本次不是成功 raw 写入验收；失败属于 `raw_only` 模式的数据前置条件缺失，即 source partition 尚未存在。
- 后续应对同一范围先执行 `execution_mode=full`，或选择已存在 source partition 的 raw asset，再补一次成功真实 raw sync 记录。

## 成功真实回填验证

范围：

- `target_scope`：`chinabond`
- 区间：`2006-01-01..2006-12-31`
- 模式：`execution_mode=full`，`dry_run=false`

命令：

```bash
set -a; . ./.env; set +a
make dagster-home
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_sources_to_raw_controller":{"config":{"target_scope":"chinabond","start_date":"2006-01-01","end_date":"2006-12-31","execution_mode":"full","dry_run":false}}}}'
```

运行事实：

- Controller run id：`d0eb1436-e984-4b04-b423-8d7bf49b42ce`
- `backfill.id`：`chinabond-2006-01-01-2006-12-31-d0eb1436e984`
- Source child run id：`e093c9bd-dc70-4054-8afa-e0422b521acc`
- Raw child run id：`d7f393c5-d901-4e9b-8074-ec90460a4bf4`
- Source child asset：`source/chinabond__government_bond`，partition `2006`
- Raw child asset：`clickhouse/raw/chinabond__government_bond`，partition `2006`

结果：source child run、raw child run 和 controller run 均成功。

Dagster run tag 核验：

```text
d0eb1436-e984-4b04-b423-8d7bf49b42ce SUCCESS backfill.id=chinabond-2006-01-01-2006-12-31-d0eb1436e984 backfill.target_scope=chinabond
e093c9bd-dc70-4054-8afa-e0422b521acc SUCCESS backfill.id=chinabond-2006-01-01-2006-12-31-d0eb1436e984 backfill.step=source_year backfill.year=2006
d7f393c5-d901-4e9b-8074-ec90460a4bf4 SUCCESS backfill.id=chinabond-2006-01-01-2006-12-31-d0eb1436e984 backfill.step=raw backfill.year=2006
```

ClickHouse 核验：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml exec -T clickhouse \
  clickhouse-client --query \
  "SELECT count() AS rows, min(work_date), max(work_date) FROM fleur_raw.chinabond__government_bond WHERE year = 2006"
```

输出：

```text
214  2006-03-01  2006-12-31
```

结论：

- 统一 controller 的 `dry_run=false` full 模式已完成一次真实 source-to-raw 小范围成功回填。
- Child runs 保留 Dagster asset materialization、partition 和 `backfill.*` tags。
- `raw_only` 可用于 source partition 已存在后的 raw sync 恢复；source partition 不存在时会按设计失败并短路。
