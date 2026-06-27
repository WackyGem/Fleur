# Furnace MACD Performance Baseline

日期：2026-06-10

范围：

- 完成 `docs/plans/archive/0034-furnace-macd-technical-indicator-implementation-plan.md` 要求的全市场、全历史 dry-run 和 production-like 写入验收。
- 输入表：`fleur_intermediate.int_stock_quotes_daily_adj`
- 输出表：`fleur_calculation.calc_stock_macd_daily`
- 参数：canonical `MACD(12,26,9)`
- histogram 口径：`DIF - DEA`

## 环境

命令环境：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client'
CLICKHOUSE_HOST=127.0.0.1
CLICKHOUSE_NATIVE_PORT=9000
CLICKHOUSE_USER=mono_fleur
CLICKHOUSE_PASSWORD=change-me-clickhouse-password
```

release binary：

```text
./engines/target/release/furnace
```

输入数据：

```text
rows = 17990764
min_trade_date = 1995-01-03
max_trade_date = 2026-06-01
securities = 5532
```

## Full-History Dry Run

命令：

```bash
./engines/target/release/furnace macd \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode dry-run \
  --run-id macd_full_history_dry_run_20260610_1842 \
  --output-format json
```

结果摘要：

```text
input_rows = 17990764
output_rows = 17990764
valid_close_rows = 17990764
null_indicator_rows = 138192
affected_years = 1995..2026
macd_state_source = full-history
incomplete_state_symbols_count = 0
gap_symbols_count = 0
writes_applied = false
```

性能：

```text
total_ms = 5125
read_input_ms = 1463
read_state_ms = 317
group_ms = 0
compute_ms = 2711
input_rows_per_sec = 12294959.906779105
output_rows_per_sec = 6635111.874460219
parallelism = serial-streaming
worker_threads = 20
```

说明：dry-run 使用 RowBinary streaming 计数路径，不分配完整输出行向量，因此 `group_ms=0`。该优化把 dry-run 的原 grouping 成本移到 streaming compute 计时中；全量 dry-run 主要瓶颈仍是 RowBinary 传输、解析和单证券递推扫描，而不是 ClickHouse partition replace 或 dbt wrapper。

## Full-History Replace Cascade

命令：

```bash
./engines/target/release/furnace macd \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id macd_full_history_replace_20260610_1839 \
  --output-format json
```

结果摘要：

```text
input_rows = 17990764
output_rows = 17990764
valid_close_rows = 17990764
null_indicator_rows = 138192
affected_years = 1995..2026
retained_rows = 0
staging_table = fleur_calculation.calc_stock_macd_daily__staging__macd_full_history_replace_20260610_1839
staging_validation.status = passed
staging_validation.duplicate_keys = 0
partition_replace.status = replaced
macd_state_source = full-history
incomplete_state_symbols_count = 0
gap_symbols_count = 0
writes_applied = true
```

性能：

```text
total_ms = 20101
read_input_ms = 1758
read_state_ms = 319
group_ms = 2182
compute_ms = 2208
write_ms = 9314
staging_ms = 2287
partition_replace_ms = 457
input_rows_per_sec = 10232013.241339508
output_rows_per_sec = 8147617.446831923
parallelism = rayon
worker_threads = 20
```

## 写入质量验收

全表行数和范围：

```text
rows = 17990764
min_trade_date = 1995-01-03
max_trade_date = 2026-06-01
securities = 5532
null macd_dif rows = 138192
null macd_dea rows = 182354
null macd_histogram rows = 182354
```

唯一键：

```text
duplicate (security_code, trade_date) groups = 0
```

分区：

```text
min_year = 1995
max_year = 2026
distinct_years = 32
```

histogram 口径：

```text
mismatched_histogram_rows where abs(macd_histogram - (macd_dif - macd_dea)) > 1e-12 = 0
```

`600674.SH` 启动点抽样：

```text
first_dif = 1995-02-14, valid_close_index = 26
first_dea = 1995-02-24, valid_close_index = 34
first_histogram = 1995-02-24, valid_close_index = 34
```

## 验证命令

已通过：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_macd_daily

cd pipeline/scheduler
uv run dg check defs
```

## 结论

- 全市场全历史 MACD 写入行数与输入行情行数一致。
- 第一条 DIF/DEA/histogram 的有效 close 序号符合 SMA 启动语义。
- 生产写入路径使用 staging + 年度 `REPLACE PARTITION`，未使用 update/delete mutation。
- 性能瓶颈主要在大批量 RowBinary 写入和 staging 流程；纯指标计算约 2.2 秒，不是当前主瓶颈。
