# Furnace MA Full-Market Parallel Validation

日期：2026-06-08

## 结论

Furnace Moving Average 基本功能已完成全市场、全历史数据量并行验收。验证使用隔离输出表：

```text
fleur_calculation.calc_stock_ma_daily_validation
```

未写入生产 canonical 表 `fleur_calculation.calc_stock_ma_daily`，避免在开发验收中污染生产消费表。

## 输入数据

```sql
SELECT
    min(trade_date),
    max(trade_date),
    count() AS input_rows,
    uniqExact(security_code) AS symbols
FROM fleur_intermediate.int_stock_quotes_daily_adj
```

结果：

| min_trade_date | max_trade_date | input_rows | symbols |
|---|---:|---:|---:|
| 1995-01-03 | 2026-06-01 | 17,990,764 | 5,532 |

当前输入表 `close_price_forward_adj IS NULL` 行数为 0。

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
target/release/furnace ma \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode dry-run \
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
target/release/furnace ma \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --output-table fleur_calculation.calc_stock_ma_daily_validation \
  --run-id furnace_ma_full_market_validation_20260608 \
  --insert-batch-size 100000 \
  --output-format json
```

## Dry-Run 摘要

```json
{
  "indicator": "ma",
  "mode": "dry-run",
  "symbols_count": 5532,
  "input_rows": 17990764,
  "output_rows": 17990764,
  "valid_close_rows": 17990764,
  "null_indicator_rows": 11064,
  "ema_state_source": "full-history",
  "writes_applied": false,
  "performance_metrics": {
    "total_ms": 5461,
    "read_input_ms": 1829,
    "group_ms": 2052,
    "compute_ms": 607,
    "parallelism": "rayon",
    "worker_threads": 8,
    "input_rows_per_sec": 9832714.76551657,
    "output_rows_per_sec": 29628730.40568641
  }
}
```

## Replace-Cascade 摘要

```json
{
  "indicator": "ma",
  "mode": "replace-cascade",
  "symbols_count": 5532,
  "input_rows": 17990764,
  "output_rows": 17990764,
  "valid_close_rows": 17990764,
  "null_indicator_rows": 11064,
  "affected_years": [1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026],
  "retained_rows": 0,
  "staging_table": "fleur_calculation.calc_stock_ma_daily_validation__staging__furnace_ma_full_market_validation_20260608",
  "staging_validation": {"status": "passed", "duplicate_keys": 0},
  "partition_replace": {"status": "replaced"},
  "ema_state_source": "full-history",
  "writes_applied": true,
  "performance_metrics": {
    "total_ms": 91462,
    "read_input_ms": 1748,
    "group_ms": 2045,
    "compute_ms": 5141,
    "write_ms": 77935,
    "staging_ms": 2447,
    "partition_replace_ms": 683,
    "parallelism": "rayon",
    "worker_threads": 8,
    "input_rows_per_sec": 10287847.678230083,
    "output_rows_per_sec": 3499270.492117547
  }
}
```

## 结果校验

验证表行数和唯一键：

| rows | symbols | duplicate_keys |
|---:|---:|---:|
| 17,990,764 | 5,532 | 0 |

字段口径检查：

```sql
SELECT groupArray(name)
FROM system.columns
WHERE database = 'fleur_calculation'
  AND table = 'calc_stock_ma_daily_validation'
  AND (name LIKE '%47%' OR name IN ('ma_57', 'avg_ma_14_28_57_114', 'ema2_10'))
```

结果：

```text
['ma_57','avg_ma_14_28_57_114','ema2_10']
```

说明：不存在 `ma_47` 或 `avg_ma_14_28_47_114` 字段。

EMA 启动 spot check：`000001.SZ` 的 `ema2_10` 在第 19 个有效 close 对应日期 `1995-01-27` 首次非空，符合 `EMA(EMA(close, 10), 10)` 的 SMA 启动规则。

## 性能调优记录

第一轮实现的 `MaOutput` 使用 `BTreeMap<usize, Option<f64>>` 保存每行 MA 值；在小范围 dry-run 中，由于仍需全历史推导 EMA，读取 17,518,124 行时 debug 计算耗时约 18s。优化为固定字段结构后，release dry-run 同数据量计算阶段约 0.58s。

当前 full-history dry-run 的主要瓶颈已经不是指标计算：

| 阶段 | full-history dry-run ms |
|---|---:|
| read_input | 1,829 |
| group | 2,052 |
| compute | 607 |
| total | 5,461 |

全量写入主要瓶颈是 RowBinary 插入：

| 阶段 | replace-cascade ms |
|---|---:|
| write | 77,935 |
| compute | 5,141 |
| staging | 2,447 |
| partition_replace | 683 |
| total | 91,462 |

后续优化方向：

1. 将 RowBinary 读取后的分组阶段改为流式分段计算，减少全量 `Vec<MaInput>` 和二次遍历。
2. 写入阶段可评估更大 batch、Native protocol 长连接或 ClickHouse local ingestion，当前 docker 包装 `clickhouse-client` 每批仍有额外开销。
3. 隔离表 active parts 每年为 2-11 个，生产首次全量写入后可视需要执行 `OPTIMIZE TABLE ... FINAL`，不建议作为默认流程。

## 增量路径优化

在隔离验证表已有完整历史结果后，追加/日常窗口可以读取上一条完整 EMA 状态，并仅回看 MA250 所需的最近 250 个交易日。对于没有上一状态但在 lookback 内新上市的证券，走 mixed 策略；如果缺状态证券早于 lookback 已存在，则回退 full-history，避免 EMA 截断误差。

验证命令：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
target/release/furnace ma \
  --from 2026-05-25 \
  --to 2026-06-01 \
  --mode dry-run \
  --output-table fleur_calculation.calc_stock_ma_daily_validation \
  --output-format json
```

结果摘要：

```json
{
  "input_from": "2025-05-14",
  "input_rows": 1318609,
  "output_rows": 31246,
  "ema_state_source": "mixed:previous-state:5523,full-history:9",
  "performance_metrics": {
    "total_ms": 2928,
    "read_input_ms": 260,
    "read_state_ms": 1736,
    "group_ms": 155,
    "compute_ms": 59,
    "parallelism": "rayon",
    "worker_threads": 8
  }
}
```

同一输出窗口在优化前因状态覆盖判断过严回退 full-history，读取 17,990,764 行、端到端约 6,932ms；优化后读取 1,318,609 行、端到端约 2,928ms。当前增量主瓶颈是上一状态查询，后续可考虑按请求证券集预聚合或改为专用状态表。
