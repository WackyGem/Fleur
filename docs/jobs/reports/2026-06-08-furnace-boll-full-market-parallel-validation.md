# Furnace BOLL Full Market Parallel Validation

日期：2026-06-08

状态：Passed

worktree commit：`9865277`

## 范围

本次验收覆盖 Furnace Bollinger Bands 日线指标：

- `BOLL(10, 1.5)`：`boll_mid_10_1p5`、`boll_up_10_1p5`、`boll_dn_10_1p5`
- `BOLL(20, 2)`：`boll_mid_20_2`、`boll_up_20_2`、`boll_dn_20_2`
- `BOLL(50, 2.5)`：`boll_mid_50_2p5`、`boll_up_50_2p5`、`boll_dn_50_2p5`

公式口径：

- 输入价格：`fleur_intermediate.int_stock_quotes_daily_adj.close_price_forward_adj`
- 标准差：总体标准差，`stddev_ddof = 0`
- 窗口：按有效 close 计数
- NULL close：输出行保留，但 BOLL 字段为 NULL，且不推进 rolling window

## 输入基线

```sql
SELECT
    min(trade_date),
    max(trade_date),
    count(),
    countIf(close_price_forward_adj IS NOT NULL),
    uniqExact(security_code)
FROM fleur_intermediate.int_stock_quotes_daily_adj
```

结果：

| min_date | max_date | input_rows | valid_close_rows | symbols |
|----------|----------|------------|------------------|---------|
| 1995-01-03 | 2026-06-01 | 17,990,764 | 17,990,764 | 5,532 |

## Dry Run

命令：

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
./target/release/furnace boll \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode dry-run \
  --insert-batch-size 10000 \
  --output-format json
```

关键 summary：

| 字段 | 值 |
|------|----|
| `input_rows` | 17,990,764 |
| `output_rows` | 17,990,764 |
| `input_valid_close_rows` | 17,990,764 |
| `output_valid_close_rows` | 17,990,764 |
| `symbols_count` | 5,532 |
| `null_indicator_rows` | 49,779 |
| `parallelism` | `rayon` |
| `worker_threads` | 8 |
| `total_ms` | 5,289 |
| `read_input_ms` | 1,789 |
| `group_ms` | 2,090 |
| `compute_ms` | 443 |
| `input_rows_per_sec` | 10,052,760.98 |
| `output_rows_per_sec` | 40,568,790.79 |

## 算法与性能优化

初始全量 dry-run 暴露了标准差快速公式的数值稳定问题：

```text
Bollinger Bands input values must be finite
```

下钻后确认 `close_price_forward_adj` 不存在 NaN/Inf：

```sql
SELECT countIf(NOT isFinite(close_price_forward_adj))
FROM fleur_intermediate.int_stock_quotes_daily_adj
WHERE NOT isFinite(close_price_forward_adj)
```

结果为 `0`。问题来自 `sum_sq / N - mean^2` 在大数值窗口上发生浮点抵消，方差可能变成负数或被抵消为 0。

修复：

- 新增 `RollingMeanStdDev`，每个窗口只维护一次 `VecDeque`、`sum`、`sum_sq`。
- BOLL 核心计算从独立 `RollingSma + RollingStdDev` 切换为联合 rolling mean/stddev，减少重复窗口维护。
- 当快速方差接近 0 且窗口内值并不全等，或快速方差明显为负时，使用同一窗口做稳定两遍 fallback。
- 新增 `rolling_mean_stddev_falls_back_when_fast_variance_cancels` 单元测试。

性能瓶颈结论：

1. dry-run 主要瓶颈是 RowBinary 分组和输入读取，计算本身约 443ms。
2. 初版 replace-cascade 主要瓶颈是 RowBinary 写入，写入阶段占总耗时约 97%。
3. 分区替换耗时很低，年度 `REPLACE PARTITION` 不是本次瓶颈。
4. 单纯把 `insert_batch_size` 从 10,000 放大到 250,000 实测更慢，说明瓶颈不是 batch 数量本身，而是多次启动 `docker exec clickhouse-client` 的子进程/连接成本和超大 chunk 传输成本。
5. 最终采用单进程流式 RowBinary insert：保持 10,000 行小批次编码，但多个 chunk 连续写入同一个 `clickhouse-client` stdin。

## Replace Cascade

命令：

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
./target/release/furnace boll \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id furnace_boll_full_market_20260608 \
  --insert-batch-size 10000 \
  --output-format json
```

关键 summary：

| 字段 | 值 |
|------|----|
| `input_rows` | 17,990,764 |
| `output_rows` | 17,990,764 |
| `symbols_count` | 5,532 |
| `null_indicator_rows` | 49,779 |
| `affected_years` | 1995..2026 |
| `staging_validation.status` | `passed` |
| `staging_validation.duplicate_keys` | 0 |
| `partition_replace.status` | `replaced` |
| `writes_applied` | true |
| `total_ms` | 419,406 |
| `read_input_ms` | 1,578 |
| `group_ms` | 2,061 |
| `compute_ms` | 3,106 |
| `write_ms` | 408,145 |
| `staging_ms` | 2,490 |
| `partition_replace_ms` | 554 |
| `parallelism` | `rayon` |
| `worker_threads` | 8 |

## Replace Cascade 写入优化复测

优化后命令保持 `--insert-batch-size 10000`，但每次目标表 INSERT 只启动一个 `clickhouse-client`，多个 RowBinary chunk 连续写入同一 stdin。

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
./target/release/furnace boll \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id furnace_boll_full_market_streaming_20260608 \
  --insert-batch-size 10000 \
  --output-format json
```

关键 summary：

| 字段 | 优化前 | 优化后 |
|------|--------|--------|
| `total_ms` | 419,406 | 24,558 |
| `write_ms` | 408,145 | 13,531 |
| `read_input_ms` | 1,578 | 1,694 |
| `group_ms` | 2,061 | 2,088 |
| `compute_ms` | 3,106 | 3,129 |
| `staging_ms` | 2,490 | 2,189 |
| `partition_replace_ms` | 554 | 507 |

优化后写入阶段从约 408s 降至约 13.5s，端到端 replace-cascade 从约 419s 降至约 24.6s。

## 表验收

```sql
SELECT
    count() AS rows,
    uniqExact(security_code, trade_date) AS unique_keys,
    count() - uniqExact(security_code, trade_date) AS duplicate_keys,
    min(trade_date),
    max(trade_date),
    uniqExact(security_code)
FROM fleur_calculation.calc_stock_boll_daily
```

| rows | unique_keys | duplicate_keys | min_date | max_date | symbols |
|------|-------------|----------------|----------|----------|---------|
| 17,990,764 | 17,990,764 | 0 | 1995-01-03 | 2026-06-01 | 5,532 |

UP/MID/DN 顺序检查：

```sql
SELECT
    countIf(boll_up_10_1p5 < boll_mid_10_1p5 OR boll_mid_10_1p5 < boll_dn_10_1p5),
    countIf(boll_up_20_2 < boll_mid_20_2 OR boll_mid_20_2 < boll_dn_20_2),
    countIf(boll_up_50_2p5 < boll_mid_50_2p5 OR boll_mid_50_2p5 < boll_dn_50_2p5)
FROM fleur_calculation.calc_stock_boll_daily
```

结果：

```text
0    0    0
```

dbt wrapper 行数：

```sql
SELECT
    count(),
    uniqExact(security_code),
    min(trade_date),
    max(trade_date),
    countIf(boll_mid_50_2p5 IS NULL)
FROM fleur_intermediate.int_stock_boll_daily
```

结果：

| rows | symbols | min_date | max_date | null_boll_mid_50_2p5 |
|------|---------|----------|----------|----------------------|
| 17,990,764 | 5,532 | 1995-01-03 | 2026-06-01 | 270,566 |

## Spot Check

样本：`000001.SZ` / `1995-03-20`

独立 SQL 使用 ClickHouse `stddevPop`，即 `ddof=0`。

| 配置 | 独立 MID | 独立 UP | 独立 DN | Furnace MID | Furnace UP | Furnace DN |
|------|----------|---------|---------|-------------|------------|------------|
| `BOLL(10, 1.5)` | 0.500561618482453 | 0.5172126884870488 | 0.4839105484778571 | 0.500561618482453 | 0.517212688487045 | 0.4839105484778608 |
| `BOLL(20, 2)` | 0.4994767528516923 | 0.5216096795100457 | 0.4773438261933389 | 0.49947675285169213 | 0.521609679510061 | 0.4773438261933232 |
| `BOLL(50, 2.5)` | 0.4916888025576787 | 0.5236106745628358 | 0.45976693055252166 | 0.4916888025576786 | 0.5236106745628424 | 0.4597669305525148 |

结论：三组配置均与独立总体标准差公式一致，差异仅为 Float64 舍入误差。

## dbt 和 Dagster 验收

dbt：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_boll_daily
uv run python elt/scripts/validate_field_glossary.py
```

结果：

- `dbt parse` passed
- `dbt build --select int_stock_boll_daily` passed：1 view model + 4 data tests
- `validate_field_glossary.py` passed

Dagster / Python：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

结果：

- `ruff check` passed
- `ruff format` left 162 files unchanged
- `pyright` passed：0 errors
- `pytest scheduler/tests` passed：349 passed，coverage 76.06%
- `dg check defs` passed

Rust：

```bash
cd engines
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --release -p furnace
```

结果：

- `cargo test --workspace` passed
- `cargo clippy ... -D warnings` passed
- release binary built successfully

## 结论

Furnace Bollinger Bands 日线指标实现通过全市场、全历史 dry-run 和 replace-cascade 写入验收。

主要性能结论：

- 算法计算不是瓶颈；优化后全量计算约 3.1s，dry-run 计算约 0.44s。
- 当前最大瓶颈是通过 `docker exec clickhouse-client` 进行 RowBinary 写入，约 408s。
- 后续若要继续优化，应优先考虑本机 `clickhouse-client`、更大 batch、直接 native client 库或 HTTP/native streaming 写入优化，而不是继续优化核心公式。
