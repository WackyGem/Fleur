# Furnace MA Full-Market Parallel Validation

日期：2026-06-08

## 结论

Furnace Moving Average 已完成全市场、全历史数据量并行验收，并已重建 canonical 输出表：

```text
fleur_calculation.calc_stock_ma_daily
```

重建前的旧裸字段表已保留为：

```text
fleur_calculation.calc_stock_ma_daily_legacy_20260608_pre_price_volume
```

## 输入数据

```sql
SELECT
    min(adj.trade_date),
    max(adj.trade_date),
    count() AS input_rows,
    countIf(adj.close_price_forward_adj IS NOT NULL) AS valid_close_rows,
    countIf(unadj.volume IS NOT NULL) AS valid_volume_rows,
    uniqExact(adj.security_code) AS symbols
FROM fleur_intermediate.int_stock_quotes_daily_adj AS adj
LEFT JOIN fleur_intermediate.int_stock_quotes_daily_unadj AS unadj
  ON adj.security_code = unadj.security_code
 AND adj.trade_date = unadj.trade_date
```

结果：

| min_trade_date | max_trade_date | input_rows | valid_close_rows | valid_volume_rows | symbols |
|---|---:|---:|---:|---:|---:|
| 1995-01-03 | 2026-06-01 | 17,990,764 | 17,990,764 | 17,990,764 | 5,532 |

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
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
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

全市场、全历史 canonical replace-cascade：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
target/release/furnace ma \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id furnace_ma_canonical_full_market_volume_cast_20260608 \
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
  "valid_volume_rows": 17990764,
  "null_indicator_rows": 11064,
  "price_ma_windows": [3, 5, 6, 10, 12, 14, 20, 24, 28, 57, 60, 114, 250],
  "volume_ma_windows": [5, 10, 20, 60],
  "ema_state_source": "full-history",
  "writes_applied": false,
  "performance_metrics": {
    "total_ms": 10171,
    "read_input_ms": 5872,
    "read_state_ms": 163,
    "group_ms": 2274,
    "compute_ms": 743,
    "parallelism": "rayon",
    "worker_threads": 8,
    "input_rows_per_sec": 3063585.7642098684,
    "output_rows_per_sec": 24187377.2551975
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
  "valid_volume_rows": 17990764,
  "null_indicator_rows": 11064,
  "affected_years": [1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026],
  "retained_rows": 0,
  "staging_table": "fleur_calculation.calc_stock_ma_daily__staging__furnace_ma_canonical_full_market_volume_cast_20260608",
  "staging_validation": {"status": "passed", "duplicate_keys": 0},
  "partition_replace": {"status": "replaced"},
  "ema_state_source": "full-history",
  "writes_applied": true,
  "performance_metrics": {
    "total_ms": 110623,
    "read_input_ms": 5676,
    "read_state_ms": 141,
    "group_ms": 2316,
    "compute_ms": 6040,
    "write_ms": 90770,
    "staging_ms": 2804,
    "partition_replace_ms": 1276,
    "parallelism": "rayon",
    "worker_threads": 8,
    "input_rows_per_sec": 3169482.498651148,
    "output_rows_per_sec": 2978311.120692717
  }
}
```

## 结果校验

验证表行数和唯一键：

| table | rows | symbols | duplicate_keys | min_date | max_date |
|---|---:|---:|---:|---|---|
| `fleur_calculation.calc_stock_ma_daily` | 17,990,764 | 5,532 | 0 | 1995-01-03 | 2026-06-01 |

字段口径检查：

```sql
SELECT groupArray(name)
FROM system.columns
WHERE database = 'fleur_calculation'
  AND table = 'calc_stock_ma_daily'
  AND (
      name IN (
          'price_ma_3',
          'price_ma_5',
          'price_ma_57',
          'price_avg_ma_3_6_12_24',
          'price_avg_ma_14_28_57_114',
          'price_ema2_10',
          'volume_ma_5',
          'volume_ma_10',
          'volume_ma_20',
          'volume_ma_60'
      )
      OR name IN ('ma_57', 'avg_ma_14_28_57_114', 'ema2_10')
      OR name LIKE '%47%'
      OR name IN ('price_ma5', 'volume_ma5', 'price_ema_2_10')
  )
```

结果：

```text
[
  'price_ma_3',
  'price_ma_5',
  'price_ma_57',
  'price_avg_ma_3_6_12_24',
  'price_avg_ma_14_28_57_114',
  'price_ema2_10',
  'volume_ma_5',
  'volume_ma_10',
  'volume_ma_20',
  'volume_ma_60'
]
```

说明：存在 Plan 0029 + ADR 0010 要求的 `price_*` 和 `volume_*` canonical 字段；不存在精确裸字段名 `ma_57`、`avg_ma_14_28_57_114`、`ema2_10`，不存在 `ma_47` 或 `avg_ma_14_28_47_114` 字段，也不存在 `price_ma5`、`volume_ma5`、`price_ema_2_10` 这类错误写法。

EMA 启动 spot check：`000001.SZ` 的 `price_ema2_10` 在第 19 个有效 close 对应日期 `1995-01-27` 首次非空，符合 `EMA(EMA(close, 10), 10)` 的 SMA 启动规则。该字段名中 `2` 表示二重 EMA，`10` 表示 EMA 窗口；MA 字段如 `price_ma_5` / `volume_ma_5` 中的 `5` 表示有效输入窗口。

成交量 spot check：`volume_ma_5`、`volume_ma_10`、`volume_ma_20`、`volume_ma_60` 来源为 `fleur_intermediate.int_stock_quotes_daily_unadj.volume`，输出 grain 以 `int_stock_quotes_daily_adj` 为准。`000004.SZ` 在 `2026-06-01` 的最近 5 个有效 volume 为 `[0,0,0,0,0]`，`volume_ma_5 = 0`，证明 `volume = 0` 作为有效值进入窗口。`000001.SZ` 和 `600000.SH` 在同日的 `volume_ma_5` 分别为 `98,634,972.8` 和 `106,610,872.4`，与独立 SQL 窗口结果一致。

RowBinary 类型修正：`int_stock_quotes_daily_unadj.volume` 是 `Nullable(Int64)`，Furnace MA 输入 RowBinary 读取为 `Nullable(Float64)`。最终实现已在输入 SQL 中使用 `CAST(unadj.volume, 'Nullable(Float64)')`，避免把 Int64 字节误读为 Float64 导致非零均量异常。

## 性能调优记录

第一轮实现的 `MaOutput` 使用 `BTreeMap<usize, Option<f64>>` 保存每行 MA 值；在小范围 dry-run 中，由于仍需全历史推导 EMA，读取 17,518,124 行时 debug 计算耗时约 18s。优化为固定字段结构后，release dry-run 同数据量计算阶段约 0.58s。

当前 full-history dry-run 的主要瓶颈已经不是指标计算：

| 阶段 | final full-history dry-run ms |
|---|---:|
| read_input | 5,872 |
| read_state | 163 |
| group | 2,274 |
| compute | 743 |
| total | 10,171 |

全量写入主要瓶颈是 RowBinary 插入：

| 阶段 | final replace-cascade ms |
|---|---:|
| write | 90,770 |
| read_input | 5,676 |
| group | 2,316 |
| compute | 6,040 |
| staging | 2,804 |
| partition_replace | 1,276 |
| total | 110,623 |

后续优化方向：

1. 将 RowBinary 读取后的分组阶段改为流式分段计算，减少全量 `Vec<MaInput>` 和二次遍历。
2. 写入阶段可评估更大 batch、Native protocol 长连接或 ClickHouse local ingestion，当前 docker 包装 `clickhouse-client` 每批仍有额外开销。
3. `calc_stock_ma_daily` 当前 active parts 为 201 个、17,990,764 行。生产首次全量写入后可视需要执行 `OPTIMIZE TABLE ... FINAL`，不建议作为默认流程。

## 增量路径优化

在 canonical 表已有完整历史结果后，追加/日常窗口可以读取上一条完整 EMA 状态，并回看每证券最近 250 个有效 close 和最近 60 个有效 volume 所需的最早日期。对于没有上一状态但在 lookback 内新上市的证券，走 `mixed` 策略；如果缺状态证券早于 lookback 已存在，则回退 `full-history`，避免 EMA 截断误差。

验证命令：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
RAYON_NUM_THREADS=8 \
target/release/furnace ma \
  --from 2026-05-25 \
  --to 2026-06-01 \
  --mode dry-run \
  --output-format json
```

结果摘要：

```json
{
  "input_from": "1998-07-03",
  "input_rows": 17576194,
  "output_rows": 31246,
  "valid_close_rows": 31246,
  "valid_volume_rows": 31246,
  "null_indicator_rows": 4,
  "ema_state_source": "mixed",
  "performance_metrics": {
    "total_ms": 29474,
    "read_input_ms": 9110,
    "read_state_ms": 5774,
    "group_ms": 2240,
    "compute_ms": 1430,
    "parallelism": "rayon",
    "worker_threads": 8
  }
}
```

当前 all-market 增量 dry-run 仍读取 17,576,194 行，因为第一版按全局最早 `input_from` 一次性读取所有受影响证券，稀疏证券会把全局 lookback 拉早。后续优化方向是把输入读取改为按证券分段 lookback，或引入专用状态/窗口快照表，避免为了少数稀疏证券回读大部分历史。

2026-06-08 追加修正：早期增量优化曾使用“全市场最近 250 个交易日”的自然日近似 lookback，可能对停牌或缺 volume 的稀疏证券读取不足。当前实现已改为按证券、按有效 close / 有效 volume 分别使用 `row_number() OVER (PARTITION BY security_code ORDER BY trade_date DESC)` 选择 250 / 60 个有效输入，并用单元测试 `run_ma_with_previous_state_uses_per_security_valid_price_and_volume_lookback` 固化。

## Dagster 验收

Dagster definitions 检查：

```bash
cd pipeline/scheduler
uv run dg check defs
```

结果：`All component YAML validated successfully.` 和 `All definitions loaded successfully.`

Dagster MA dry-run job：

```bash
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_USER=mono_fleur \
CLICKHOUSE_PASSWORD=change-me-clickhouse-password \
CLICKHOUSE_QUERY_TIMEOUT_SECONDS=900 \
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
RAYON_NUM_THREADS=8 \
uv run dg launch \
  --job furnace__ma_dry_run_job \
  --config-json '{"ops":{"fleur_calculation__calc_stock_ma_daily":{"config":{"request_from":"2026-06-01","request_to":"2026-06-01","mode":"dry-run","symbols":[],"input_table":"fleur_intermediate.int_stock_quotes_daily_adj","volume_input_table":"fleur_intermediate.int_stock_quotes_daily_unadj","output_table":"fleur_calculation.calc_stock_ma_daily","price_column":"close_price_forward_adj","volume_column":"volume","insert_batch_size":10000}}}}'
```

结果：run id `c9f6fb35-9457-459c-9d02-4b6708e4119b` 成功，`fleur_calculation__calc_stock_ma_daily` step materialized `fleur_calculation/calc_stock_ma_daily` in dry-run mode，step 耗时 `25.81s`。
