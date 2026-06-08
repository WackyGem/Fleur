# Furnace KDJ Parallel Optimization Report

Date: 2026-06-07

Updated: 2026-06-08

Scope:

- Implement `docs/plans/0028-furnace-kdj-parallel-performance-implementation-plan.md` first pass.
- Optimize all-market daily KDJ calculation for Furnace.
- Validate compute parallelism, ClickHouse read path, full-range storage writes, and target part health.

Build:

- Furnace binary: `engines/target/release/furnace`
- Build mode: release
- Git HEAD at benchmark time: `7f606ba`
- Worktree state: dirty with the implementation and report changes in this optimization run.

## Changes Validated

1. `furnace-io` emits structured `performance_metrics` in `KdjRunSummary`.
2. Dagster Furnace asset metadata now passes through `performance_metrics`.
3. KDJ calculation is parallelized across `security_code` using Rayon.
4. Single-security or small runs keep a serial fast path.
5. All-market runs no longer build a very large `security_code IN (...)` filter after resolving all symbols; they use `1 = 1`.
6. Explicit symbol runs still use symbol filtering.
7. CLI JSON summary keeps `symbols_count` but no longer emits the full all-market symbol list.
8. Input scans now use `FORMAT RowBinary` and a dedicated Rust RowBinary decoder for KDJ input grouping.
9. Full-year all-market `replace-cascade` skips unnecessary old-row retention queries for fully covered year partitions.
10. Result-row TSV generation writes directly into batch buffers instead of allocating per-row joined strings.
11. Result writes now use `FORMAT RowBinary` instead of TSV.
12. Staging duplicate validation now checks all affected years with one grouped query instead of one query per year.
13. Staging setup and yearly `REPLACE PARTITION` statements now use multi-query execution to reduce `clickhouse-client` subprocess round trips.

## Workload

Command template:

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
RAYON_NUM_THREADS=<threads> \
./target/release/furnace kdj \
  --from 2026-05-06 \
  --to 2026-06-01 \
  --mode dry-run \
  --output-format json
```

All-market dry-run after optimization:

| Threads | Total ms | Read state ms | Read input ms | Group ms | Compute ms | Input rows | Output rows | Parallelism |
|---------|----------|---------------|---------------|----------|------------|------------|-------------|-------------|
| 1 | 1123 | 141 | 383 | 63 | 57 | 234005 | 98892 | serial |
| 4 | 1071 | 137 | 351 | 65 | 39 | 234005 | 98892 | rayon |
| 8 | 1042 | 150 | 328 | 65 | 31 | 234005 | 98892 | rayon |
| 20 | 1171 | 155 | 390 | 63 | 47 | 234005 | 98892 | rayon |

Earlier same-range run before removing the all-market `IN (...)` filter:

| Threads | Total ms | Read state ms | Read input ms | Group ms | Compute ms | Input rows | Output rows | Parallelism |
|---------|----------|---------------|---------------|----------|------------|------------|-------------|-------------|
| 1 | 1540 | 239 | 449 | 62 | 63 | 233946 | 98892 | serial |
| 4 | 1475 | 230 | 441 | 63 | 39 | 233946 | 98892 | rayon |
| 8 | 1413 | 216 | 451 | 64 | 31 | 233946 | 98892 | rayon |
| 20 | 1421 | 238 | 425 | 65 | 45 | 233946 | 98892 | rayon |

Single-symbol dry-run:

```text
symbols_count=1
input_rows=45
output_rows=19
writes_applied=false
parallelism=serial
```

Full-range all-market workload:

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
RAYON_NUM_THREADS=8 \
./target/release/furnace kdj \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id full_kdj_1995_2026_rowbinary_single_batch_multiquery_replace \
  --insert-batch-size 20000000 \
  --output-format json
```

Full-range dry-run, all 5,532 securities and 17,990,764 input rows:

| Variant | Threads | Total ms | Read state ms | Read input ms | Group ms | Compute ms | Input rows | Output rows |
|---------|---------|----------|---------------|---------------|----------|------------|------------|-------------|
| TSV input | 8 | 12198 | 148 | 4290 | 6123 | 438 | 17990764 | 17990764 |
| RowBinary input | 4 | 7075 | 159 | 2398 | 2469 | 828 | 17990764 | 17990764 |
| RowBinary input | 8 | 6505 | 159 | 2225 | 2443 | 460 | 17990764 | 17990764 |
| RowBinary input | 12 | 6627 | 150 | 2325 | 2499 | 385 | 17990764 | 17990764 |
| RowBinary input | 16 | 6801 | 172 | 2507 | 2497 | 361 | 17990764 | 17990764 |

Full-range `replace-cascade`, all 5,532 securities and 17,990,764 output rows:

| Variant | Insert batch size | Total ms | Read input ms | Group ms | Compute ms | Write ms | Staging ms | Replace ms |
|---------|-------------------|----------|---------------|----------|------------|----------|------------|------------|
| RowBinary + TSV row join | 100000 | 125055 | 2196 | 2497 | 2230 | 92191 | 18841 | 5295 |
| RowBinary + direct TSV buffer | 100000 | 115499 | 2454 | 2520 | 2181 | 82428 | 18804 | 5267 |
| RowBinary + direct TSV buffer + covered-year retain skip | 100000 | 105596 | 2451 | 2455 | 2208 | 82026 | 9353 | 5294 |
| RowBinary input + RowBinary output | 100000 | 72943 | 2435 | 2544 | 2180 | 49297 | 9410 | 5247 |
| RowBinary input + RowBinary output | 250000 | 51171 | 2359 | 2472 | 2233 | 27862 | 9301 | 5120 |
| RowBinary input + RowBinary output | 500000 | 43123 | 2123 | 2502 | 2235 | 20140 | 9165 | 5151 |
| RowBinary input + RowBinary output | 1000000 | 38984 | 2207 | 2535 | 2209 | 15729 | 9367 | 5079 |
| RowBinary input + RowBinary output | 2000000 | 36131 | 2334 | 2478 | 2239 | 12526 | 9483 | 5242 |
| RowBinary input + RowBinary output | 4000000 | 34652 | 2256 | 2514 | 2221 | 11000 | 9585 | 5288 |
| RowBinary output + 4000000 + single validation query | 4000000 | 27964 | 2352 | 2557 | 2141 | 11096 | 2813 | 5142 |
| RowBinary output + 4000000 + single validation + multi-query replace | 4000000 | 22669 | 2164 | 2601 | 2187 | 11054 | 2480 | 371 |
| RowBinary output + 8000000 + single validation + multi-query replace | 8000000 | 21900 | 2420 | 2519 | 2177 | 10023 | 2428 | 464 |
| RowBinary output + single batch + single validation + multi-query replace | 20000000 | 21008 | 2243 | 2476 | 2210 | 9514 | 2275 | 448 |

## Findings

1. Best observed setting for this all-market one-month dry-run is 8 Rayon threads.
2. CPU compute is not the dominant bottleneck after Rayon: `compute_ms=31` at 8 threads versus `total_ms=1042`.
3. Removing the all-market `IN (...)` filter reduced total runtime by roughly 26% for the best observed run, from 1413 ms to 1042 ms.
4. RowBinary input reduced full-range dry-run runtime from 12.2s to 6.5s, mainly by reducing `read_input_ms` and `group_ms`.
5. RowBinary output reduced full-range 100,000-row batch write runtime from 105.6s to 72.9s.
6. Larger RowBinary batches show the remaining cost is strongly tied to insert batch/subprocess count. The best measured full-range run is 21.0s with a single full-range RowBinary batch, single-query staging validation, and multi-query partition replacement.
7. Full-range compute is not the bottleneck: `compute_ms=2210` during the final write run, while `write_ms=9514`, `staging_ms=2275`, and `partition_replace_ms=448`.
8. Per `insert-batch-size`, the default and routine incremental guidance should remain in the 10,000-100,000 row range. The 4,000,000-row batch is the safer historical full-backfill setting. The single-batch run is the fastest measured local run and requires explicit memory headroom plus part-health validation.

## ClickHouse Evidence

Input query plan:

```sql
EXPLAIN indexes = 1
SELECT
    security_code,
    trade_date,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj
FROM fleur_intermediate.int_stock_quotes_daily_adj
WHERE trade_date >= toDate('2026-03-25')
  AND trade_date <= toDate('2026-06-01')
ORDER BY
    security_code,
    trade_date;
```

Summary:

```text
Min-Max index on trade_date: Parts 3/88, Granules 61/2228
Partition key toYear(trade_date): Parts 3/3, Granules 61/61
PrimaryKey condition on trade_date: Parts 3/3, Granules 61/61
Ranges: 3
```

Target part health:

```text
1995 parts=2 rows=75156
1996 parts=2 rows=95794
1997 parts=5 rows=155054
1998 parts=5 rows=191753
1999 parts=6 rows=209625
2000 parts=6 rows=234492
2001 parts=6 rows=266683
2002 parts=6 rows=277146
2003 parts=6 rows=297217
2004 parts=6 rows=320846
2005 parts=6 rows=329542
2006 parts=6 rows=329357
2007 parts=2 rows=354952
2008 parts=2 rows=388176
2009 parts=4 rows=395732
2010 parts=5 rows=453319
2011 parts=5 rows=537338
2012 parts=5 rows=587438
2013 parts=4 rows=588294
2014 parts=4 rows=620803
2015 parts=3 rows=665917
2016 parts=3 rows=704878
2017 parts=3 rows=799801
2018 parts=7 rows=857532
2019 parts=8 rows=890739
2020 parts=8 rows=952245
2021 parts=3 rows=1062633
2022 parts=7 rows=1149504
2023 parts=7 rows=1212196
2024 parts=6 rows=1236574
2025 parts=6 rows=1251474
2026 parts=7 rows=498554
total_rows=17990764
unique_keys=17990764
date_range=1995-01-03..2026-06-01
symbols=5532
final_run_parts_max_per_partition=8
final_run_size=577.21 MiB
final_run_memory_available=109 GiB
```

Current table layouts:

```text
fleur_intermediate.int_stock_quotes_daily_adj
PARTITION BY toYear(trade_date)
ORDER BY (security_code, trade_date)

fleur_calculation.calc_stock_kdj_daily
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

## Decision

The implemented first-pass optimal scheme is:

```text
all-market request
  -> resolve symbols for metadata and output scope
  -> read RowBinary input with date range + 1 = 1, ordered by security_code, trade_date
  -> decode and group by security_code
  -> Rayon per-security KDJ calculation
  -> deterministic output sort
  -> batched RowBinary ClickHouse write and single-coordinator partition replace
  -> single-query staging duplicate validation
  -> multi-query yearly partition replacement
```

Dagster-triggered Furnace runs default to `RAYON_NUM_THREADS=8` through `FurnaceCliResource`, while preserving any externally supplied `RAYON_NUM_THREADS` value.

The storage scheme remains:

```text
fleur_calculation.calc_stock_kdj_daily
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

Rationale:

- Per `insert-batch-size`, parallel workers must not write per security. The current single coordinator preserves batched inserts. Routine jobs keep the normal 10,000-100,000 row guidance; measured full historical backfills can use larger batches when memory and part health are checked.
- Per `insert-format-native`, RowBinary is an efficient row-based alternative to TSV and avoids text parsing overhead.
- Per `schema-partition-low-cardinality` and `decision-partitioning-timeseries`, year partitions remain bounded and are appropriate for current staging/replace operations.
- Per `schema-pk-filter-on-orderby` and `schema-pk-prioritize-filters`, the measured input query uses date pruning well enough for the current scale; a helper/projection table is not yet justified by this benchmark.
- Full-range target part counts remain low after RowBinary writes and yearly `REPLACE PARTITION`; no high-cardinality partitioning change is justified.

## Follow-Up Candidates

1. Consider a long-lived streaming `clickhouse-client` insert path or Native columnar output only if future larger datasets make the remaining `write_ms=9514` material or if single-batch memory pressure becomes unacceptable.
2. Consider computing and writing bounded chunks to avoid holding all result rows for full historical writes; this is primarily a memory-risk improvement, not a proven speed improvement for the current dataset.
3. Keep `RAYON_NUM_THREADS=8` as the default unless future hardware-specific benchmarks show a better setting.
4. Revisit a helper/projection table only if input read again becomes the bottleneck after write-path improvements.

## Validation

Commands run:

```bash
cd engines
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D clippy::perf
cargo build --release -p furnace
```

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests/unit/furnace scheduler/tests/unit/resources/test_furnace.py
uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/unit/resources/test_furnace.py
```

All listed validation commands passed.
