# Furnace Price Pattern Full-Market Validation

日期：2026-06-09

状态：Passed

## 范围

本次验收覆盖 Furnace 日频价格行为与前低-次低结构指标：

- 连阳/连阴：使用 `fleur_intermediate.int_stock_quotes_daily_unadj.close_price` 和 `prev_close_price`。
- 前低-次低结构：使用 `fleur_intermediate.int_stock_quotes_daily_adj.high_price_forward_adj` 和 `low_price_forward_adj`。
- 输出表：`fleur_calculation.calc_stock_price_pattern_daily`。
- dbt wrapper：`fleur_intermediate.int_stock_price_pattern_daily`。

## 输入基线

结构输入：

| min_date | max_date | rows | valid_high_low_rows | symbols |
|----------|----------|------|---------------------|---------|
| 1995-01-03 | 2026-06-01 | 17,990,764 | 17,990,764 | 5,532 |

连阳/连阴输入：

| min_date | max_date | rows | valid_close_prev_close_rows | symbols |
|----------|----------|------|-----------------------------|---------|
| 1995-01-03 | 2026-06-01 | 17,990,764 | 17,990,764 | 5,532 |

真实数据中有 3 行前复权 high/low 同时为 0：

| security_code | trade_date | high_price_forward_adj | low_price_forward_adj |
|---------------|------------|------------------------|-----------------------|
| 000022.SZ | 2018-12-26 | 0 | 0 |
| 000043.SZ | 2019-12-16 | 0 | 0 |
| 300114.SZ | 2025-02-17 | 0 | 0 |

处理决策：非有限价格仍为硬错误；有限但不可用于结构窗口的 high/low（`low <= 0` 或 `high < low`）不推进结构窗口，避免少量行情异常中断全市场计算。连阳/连阴仍按 close/preclose 正常输出。

## Dry Run

命令：

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
./target/release/furnace price-pattern \
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
| `input_valid_streak_rows` | 17,990,764 |
| `input_valid_structure_bar_rows` | 17,990,761 |
| `valid_streak_rows` | 17,990,764 |
| `valid_structure_bar_rows` | 17,990,761 |
| `null_streak_rows` | 0 |
| `null_second_low_rows` | 1,871,952 |
| `parallelism` | `rayon` |
| `worker_threads` | 8 |
| `total_ms` | 11,818 |
| `read_input_ms` | 6,585 |
| `group_ms` | 2,823 |
| `compute_ms` | 1,275 |

dry-run 瓶颈是 ClickHouse RowBinary 输入读取和分组，计算本身约 1.3s。

## Replace Cascade

初始全市场 canonical replace-cascade 写入成功，但暴露 RowBinary 写入瓶颈：

| 字段 | 初始值 |
|------|--------|
| `total_ms` | 462,127 |
| `write_ms` | 444,633 |
| `read_input_ms` | 6,284 |
| `group_ms` | 2,756 |
| `compute_ms` | 4,080 |
| `staging_ms` | 2,329 |
| `partition_replace_ms` | 466 |

优化：将 shared RowBinary writer 改为单 `clickhouse-client` 进程流式写入，同一 INSERT stdin 内连续写入多个 batch，避免每 10,000 行启动一次 `docker exec clickhouse-client`。

优化后命令：

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
./target/release/furnace price-pattern \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id furnace_price_pattern_full_market_streaming_20260609 \
  --insert-batch-size 10000 \
  --output-format json
```

优化后 summary：

| 字段 | 值 |
|------|----|
| `input_rows` | 17,990,764 |
| `output_rows` | 17,990,764 |
| `valid_streak_rows` | 17,990,764 |
| `valid_structure_bar_rows` | 17,990,761 |
| `affected_years` | 1995..2026 |
| `staging_validation.status` | `passed` |
| `staging_validation.duplicate_keys` | 0 |
| `partition_replace.status` | `replaced` |
| `writes_applied` | true |
| `total_ms` | 29,368 |
| `write_ms` | 11,296 |
| `read_input_ms` | 7,007 |
| `group_ms` | 2,738 |
| `compute_ms` | 4,000 |
| `staging_ms` | 2,216 |
| `partition_replace_ms` | 504 |

端到端 replace-cascade 从 462.1s 降至 29.4s，写入阶段从 444.6s 降至 11.3s。

## 表验收

```sql
SELECT
    count() AS rows,
    uniqExact(security_code, trade_date) AS unique_keys,
    count() - uniqExact(security_code, trade_date) AS duplicate_keys,
    min(trade_date),
    max(trade_date),
    uniqExact(security_code) AS symbols
FROM fleur_calculation.calc_stock_price_pattern_daily
```

| rows | unique_keys | duplicate_keys | min_date | max_date | symbols |
|------|-------------|----------------|----------|----------|---------|
| 17,990,764 | 17,990,764 | 0 | 1995-01-03 | 2026-06-01 | 5,532 |

值域检查：

| check | result |
|-------|--------|
| bad_direction_rows | 0 |
| bad_valid_bar_rows | 0 |
| bad_true_valid_rows | 0 |
| bad_false_valid_rows | 0 |
| null_ratio_rows | 1,871,952 |

dbt wrapper：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_price_pattern_daily
```

结果：`PASS=9 WARN=0 ERROR=0 SKIP=0`。
