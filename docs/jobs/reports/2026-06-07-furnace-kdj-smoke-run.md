# Furnace KDJ Smoke Run

日期：2026-06-07

范围：

- 输入表：`fleur_intermediate.int_stock_quotes_daily_adj`
- 输出表：`fleur_calculation.calc_stock_kdj_daily`
- 证券：`000069.SZ`
- 请求区间：`2026-05-06` 至 `2026-06-01`
- 参数：`KDJ(9,3,3)`，前复权价格口径

## 环境

本机未安装 `clickhouse-client`，本次通过 Docker 容器内客户端执行：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client'
CLICKHOUSE_HOST=127.0.0.1
CLICKHOUSE_NATIVE_PORT=9000
```

输入表检查：

```text
EXISTS TABLE fleur_intermediate.int_stock_quotes_daily_adj -> 1
min(trade_date) = 1995-01-03
max(trade_date) = 2026-06-01
rows = 17990764
securities = 5532
```

样本证券选择：

```text
000069.SZ    2026-05-06    2026-06-01    19
```

## Dry Run

命令：

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
./target/debug/furnace kdj \
  --from 2026-05-06 \
  --to 2026-06-01 \
  --symbols 000069.SZ \
  --mode dry-run \
  --run-id smoke-dry-run \
  --output-format json
```

结果摘要：

```text
input_from = 2026-03-25
input_to = 2026-06-01
input_rows = 45
output_rows = 19
null_indicator_rows = 0
affected_years = [2026]
writes_applied = false
```

## Append Latest

命令同上，`--mode append-latest --run-id smoke-append-latest`。

结果摘要：

```text
input_rows = 45
output_rows = 19
null_indicator_rows = 0
affected_years = [2026]
state_source = initial_50
writes_applied = true
```

写入后校验：

```text
rows = 19
securities = 1
min_trade_date = 2026-05-06
max_trade_date = 2026-06-01
duplicate key rows = 0
canonical params = 9 / 3 / 3, rows = 19
active parts: partition 2026, parts = 1, rows = 19
```

## Replace Cascade

命令同上，`--mode replace-cascade --run-id smoke-replace-cascade`。

结果摘要：

```text
effective_output_from = 2026-05-06
effective_output_to = 2026-06-01
input_from = 2026-03-25
input_to = 2026-06-01
input_rows = 45
output_rows = 19
affected_years = [2026]
retained_rows = 0
staging_table = fleur_calculation.calc_stock_kdj_daily__staging__smoke_replace_cascade
staging_validation.status = passed
staging_validation.duplicate_keys = 0
partition_replace.status = replaced
writes_applied = true
```

替换后校验：

```text
rows = 19
securities = 1
min_trade_date = 2026-05-06
max_trade_date = 2026-06-01
duplicate key rows = 0
active parts: partition 2026, parts = 1, rows = 19
```

## Month Backfill Cascade

为验证历史请求区间会级联到受影响证券的最新输入交易日，补充执行完整 2026 年 5 月请求区间：

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
./target/debug/furnace kdj \
  --from 2026-05-01 \
  --to 2026-05-31 \
  --symbols 000069.SZ \
  --mode replace-cascade \
  --run-id smoke-month-backfill-cascade \
  --output-format json
```

结果摘要：

```text
request_from = 2026-05-01
request_to = 2026-05-31
effective_output_from = 2026-05-01
effective_output_to = 2026-06-01
input_from = 2026-03-24
input_to = 2026-06-01
input_rows = 46
output_rows = 19
affected_years = [2026]
staging_validation.status = passed
partition_replace.status = replaced
writes_applied = true
```

级联验收：

```text
000069.SZ min_written_date = 2026-05-06
000069.SZ max_written_date = 2026-06-01
rows = 19
```

该运行证明请求区间 `2026-05-01` 至 `2026-05-31` 会因 K/D 递推状态扩展到受影响证券当前最新输入交易日 `2026-06-01`。

## dbt Wrapper 验收

命令：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_kdj_daily --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

结果：通过。

## Dagster Asset Dry Run 验收

命令：

```bash
cd pipeline
DAGSTER_HOME=/storage/program/mono-fleur/.dagster \
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
uv run dg launch \
  --target-path scheduler \
  --job furnace__kdj_dry_run_job \
  --config-json '{"ops":{"fleur_calculation__calc_stock_kdj_daily":{"config":{"request_from":"2026-05-06","request_to":"2026-06-01","mode":"dry-run","symbols":["000069.SZ"],"rsv_window":9,"k_smoothing":3,"d_smoothing":3,"insert_batch_size":10000}}}}'
```

结果：

```text
run_id = 05a053c5-bc99-49b0-ac90-ae6824c87b04
job = furnace__kdj_dry_run_job
step = fleur_calculation__calc_stock_kdj_daily
event = ASSET_MATERIALIZATION
asset = fleur_calculation/calc_stock_kdj_daily
status = RUN_SUCCESS
```

## 结论

- Furnace CLI 能从 `int_stock_quotes_daily_adj` 读取前复权日线输入并计算 KDJ。
- `dry-run` 不写 ClickHouse，仅返回 JSON summary。
- `append-latest` 能创建并写入 `fleur_calculation.calc_stock_kdj_daily`。
- `replace-cascade` 能通过 staging table 校验后执行年度分区替换。
- 历史月度请求区间能将 `effective_output_to` 级联到受影响证券最新输入交易日。
- dbt `int_stock_kdj_daily` wrapper 能消费 Furnace 输出表并通过定向 build。
- Dagster `furnace__kdj_dry_run_job` 能调用 Rust CLI，并记录 Furnace 计算资产物化事件。
