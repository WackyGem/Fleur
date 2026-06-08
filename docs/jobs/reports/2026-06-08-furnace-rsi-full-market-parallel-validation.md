# Furnace RSI Full-Market Parallel Validation

日期：2026-06-08

## 结论

Furnace RSI 已完成全市场、全历史数据量并行验收，并已写入 canonical 表：

```text
fleur_calculation.calc_stock_rsi_daily
```

同时使用隔离表完成了同等数据量的 replace-cascade 写入验证：

```text
fleur_calculation.calc_stock_rsi_daily_validation
```

RSI canonical 窗口为 `6, 12, 14, 24, 25, 50`，输入价格口径为 `close_price_forward_adj`。全量计算阶段的主要瓶颈不是 RSI 计算，而是 ClickHouse RowBinary 写入；日常增量路径已启用 previous-state + mixed input 优化。

## 输入数据

```sql
SELECT
    min(trade_date),
    max(trade_date),
    count() AS input_rows,
    countIf(close_price_forward_adj IS NOT NULL) AS valid_close_rows,
    uniqExact(security_code) AS symbols
FROM fleur_intermediate.int_stock_quotes_daily_adj
```

结果：

| min_trade_date | max_trade_date | input_rows | valid_close_rows | symbols |
|---|---:|---:|---:|---:|
| 1995-01-03 | 2026-06-01 | 17,990,764 | 17,990,764 | 5,532 |

## 命令

Release build：

```bash
cd engines
cargo build --release -p furnace
```

全市场、全历史 dry-run：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
target/release/furnace rsi \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode dry-run \
  --insert-batch-size 100000 \
  --output-format json
```

全市场、全历史隔离 replace-cascade：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
target/release/furnace rsi \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --output-table fleur_calculation.calc_stock_rsi_daily_validation \
  --run-id furnace_rsi_full_market_validation_20260608 \
  --insert-batch-size 100000 \
  --output-format json
```

全市场、全历史 canonical replace-cascade：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
target/release/furnace rsi \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id furnace_rsi_full_market_production_20260608 \
  --insert-batch-size 100000 \
  --output-format json
```

## Dry-Run 摘要

```json
{
  "indicator": "rsi",
  "mode": "dry-run",
  "symbols_count": 5532,
  "input_rows": 17990764,
  "output_rows": 17990764,
  "valid_close_rows": 17990764,
  "null_indicator_rows": 341406,
  "rsi_windows": [6, 12, 14, 24, 25, 50],
  "rsi_state_source": "full-history",
  "gap_symbols_count": 0,
  "gap_fill_from": null,
  "writes_applied": false,
  "performance_metrics": {
    "total_ms": 5749,
    "read_input_ms": 1932,
    "read_state_ms": 0,
    "group_ms": 2107,
    "compute_ms": 620,
    "write_ms": 0,
    "staging_ms": 0,
    "partition_replace_ms": 0,
    "parallelism": "rayon",
    "worker_threads": 8
  }
}
```

说明：全历史请求覆盖输入表最早日期时，RSI 不读取 previous state，避免在已有 canonical 表后做无意义的状态扫描。

## Replace-Cascade 摘要

隔离表写入：

```json
{
  "indicator": "rsi",
  "mode": "replace-cascade",
  "symbols_count": 5532,
  "input_rows": 17990764,
  "output_rows": 17990764,
  "valid_close_rows": 17990764,
  "null_indicator_rows": 341406,
  "affected_years": [1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026],
  "retained_rows": 0,
  "staging_table": "fleur_calculation.calc_stock_rsi_daily_validation__staging__furnace_rsi_full_market_validation_20260608",
  "staging_validation": {"status": "passed", "duplicate_keys": 0},
  "partition_replace": {"status": "replaced"},
  "rsi_state_source": "full-history",
  "writes_applied": true,
  "performance_metrics": {
    "total_ms": 89791,
    "read_input_ms": 1892,
    "read_state_ms": 0,
    "group_ms": 2062,
    "compute_ms": 5177,
    "write_ms": 75796,
    "staging_ms": 2576,
    "partition_replace_ms": 674,
    "parallelism": "rayon",
    "worker_threads": 8
  }
}
```

Canonical 表写入：

```json
{
  "indicator": "rsi",
  "mode": "replace-cascade",
  "symbols_count": 5532,
  "input_rows": 17990764,
  "output_rows": 17990764,
  "valid_close_rows": 17990764,
  "null_indicator_rows": 341406,
  "staging_validation": {"status": "passed", "duplicate_keys": 0},
  "partition_replace": {"status": "replaced"},
  "rsi_state_source": "full-history",
  "writes_applied": true,
  "performance_metrics": {
    "total_ms": 90269,
    "read_input_ms": 1737,
    "read_state_ms": 0,
    "group_ms": 2092,
    "compute_ms": 5044,
    "write_ms": 76532,
    "staging_ms": 2574,
    "partition_replace_ms": 650,
    "parallelism": "rayon",
    "worker_threads": 8
  }
}
```

## 结果校验

隔离表行数和唯一键：

| rows | symbols | duplicate_keys |
|---:|---:|---:|
| 17,990,764 | 5,532 | 0 |

值域和状态健康：

| check | result |
|---|---:|
| out_of_range_rows | 0 |
| negative_state_rows | 0 |

启动口径 spot check：

| security_code | check | result |
|---|---|---|
| 000001.SZ | `rsi_50` 首次非空日期 | 1995-03-21 |
| 000001.SZ | `rsi_50` 首次非空值 | 49.27302100161552 |

独立 Python 脚本复算 `000001.SZ`，与 ClickHouse 结果逐字段一致：

| trade_date | rsi_6 | rsi_14 | rsi_50 |
|---|---:|---:|---:|
| 1995-01-12 | 38.51174934725851 | NULL | NULL |
| 1995-01-24 | 50.427717881586325 | 43.44155844155846 | NULL |
| 1995-03-21 | 40.03006252728159 | 46.43709661496186 | 49.27302100161552 |

当前全量输入表 `close_price_forward_adj IS NULL` 行数为 0，因此真实数据验收没有缺价样本。缺价不推进状态、下一有效 close 继续使用上一有效 close 的口径由 `furnace-core::indicators::rsi` 单元测试覆盖。

## 增量路径优化

在 canonical RSI 表已有完整历史结果后，2026-05-25 至 2026-06-01 的 dry-run 使用 previous-state + mixed input 查询。该路径只读取 previous state anchor 和目标区间附近输入，不再全历史回读。

验证命令：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
target/release/furnace rsi \
  --from 2026-05-25 \
  --to 2026-06-01 \
  --mode dry-run \
  --insert-batch-size 100000 \
  --output-format json
```

优化后结果：

```json
{
  "input_from": "1999-07-12",
  "input_rows": 37202,
  "output_rows": 31246,
  "valid_close_rows": 37202,
  "null_indicator_rows": 15,
  "rsi_state_source": "mixed:previous-state:5510,full-history:0",
  "gap_symbols_count": 0,
  "gap_fill_from": null,
  "performance_metrics": {
    "total_ms": 4427,
    "read_input_ms": 1126,
    "read_state_ms": 2813,
    "group_ms": 12,
    "compute_ms": 2,
    "parallelism": "rayon",
    "worker_threads": 8
  }
}
```

优化过程记录：

| 场景 | input_rows | read_state_ms | read_input_ms | group_ms | compute_ms | total_ms |
|---|---:|---:|---:|---:|---:|---:|
| 增量窗口优化前，回退全历史读取 | 17,990,764 | 11,279 | 1,815 | 2,066 | 506 | 16,600 |
| 混合输入 + 状态聚合优化后 | 37,202 | 2,813 | 1,126 | 12 | 2 | 4,427 |

当前增量瓶颈是上一状态查询和 ClickHouse 输入读取；RSI 计算本身不是瓶颈。

## 质量门禁

Rust：

```bash
cd engines
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --release -p furnace
```

结果：全部通过。`furnace-io` 当前 25 个单元测试通过，包含 RSI append-latest 缺口保护 fixture。

dbt：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_rsi_daily
uv run python elt/scripts/validate_field_glossary.py
```

结果：`int_stock_rsi_daily` build 通过，`PASS=8 WARN=0 ERROR=0 SKIP=0 NO-OP=0 TOTAL=8`；field glossary lint 通过。

Dagster / Python：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/resources/test_furnace.py scheduler/tests/unit/furnace/test_furnace_definitions.py
cd scheduler
uv run dg check defs
```

结果：ruff、pyright、定向 pytest 24 项、`dg check defs` 全部通过。

## 后续优化

1. 全量写入主要耗时在 RowBinary INSERT，后续可评估更大 batch、Native protocol 长连接或 ClickHouse local ingestion。
2. 日常增量主要耗时在 previous state 查询，可评估专用状态表或按请求证券集的状态缓存。
3. RowBinary 读取后的分组仍可继续流式化，降低全量运行时内存峰值和二次遍历成本。
