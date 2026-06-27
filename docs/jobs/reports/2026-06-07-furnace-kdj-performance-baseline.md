# Furnace KDJ Performance Baseline

Date: 2026-06-07

Updated: 2026-06-08

Scope:

- Establish the first measured baseline required by `docs/plans/archive/0028-furnace-kdj-parallel-performance-implementation-plan.md`.
- Record the baseline bottleneck before the later RowBinary output, large historical batch, single-query validation, and multi-query partition replacement optimizations.
- Keep the detailed optimization comparison in `docs/jobs/reports/2026-06-07-furnace-kdj-parallel-optimization.md`.

## Baseline Environment

Command pattern:

```bash
cd engines
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
RAYON_NUM_THREADS=<threads> \
./target/release/furnace kdj \
  --from <from> \
  --to <to> \
  --mode dry-run \
  --output-format json
```

Input table:

```text
fleur_intermediate.int_stock_quotes_daily_adj
PARTITION BY toYear(trade_date)
ORDER BY (security_code, trade_date)
```

Output table:

```text
fleur_calculation.calc_stock_kdj_daily
PARTITION BY toYear(trade_date)
ORDER BY (trade_date, security_code)
```

## One-Month All-Market Baseline

Range: `2026-05-06..2026-06-01`

| Variant | Threads | Total ms | Read state ms | Read input ms | Group ms | Compute ms | Input rows | Output rows | Parallelism |
|---------|---------|----------|---------------|---------------|----------|------------|------------|-------------|-------------|
| all-market `IN (...)` filter | 1 | 1540 | 239 | 449 | 62 | 63 | 233946 | 98892 | serial |
| all-market `IN (...)` filter | 4 | 1475 | 230 | 441 | 63 | 39 | 233946 | 98892 | rayon |
| all-market `IN (...)` filter | 8 | 1413 | 216 | 451 | 64 | 31 | 233946 | 98892 | rayon |
| all-market `IN (...)` filter | 20 | 1421 | 238 | 425 | 65 | 45 | 233946 | 98892 | rayon |
| all-market `1 = 1` filter | 1 | 1123 | 141 | 383 | 63 | 57 | 234005 | 98892 | serial |
| all-market `1 = 1` filter | 4 | 1071 | 137 | 351 | 65 | 39 | 234005 | 98892 | rayon |
| all-market `1 = 1` filter | 8 | 1042 | 150 | 328 | 65 | 31 | 234005 | 98892 | rayon |
| all-market `1 = 1` filter | 20 | 1171 | 155 | 390 | 63 | 47 | 234005 | 98892 | rayon |

Baseline conclusion:

- 8 Rayon threads was the best observed one-month all-market setting.
- Removing the generated all-market `IN (...)` filter reduced the best observed run from 1413ms to 1042ms.
- CPU compute was already not the dominant bottleneck: the best one-month run had `compute_ms=31` and `total_ms=1042`.

## Full-Range Dry-Run Baseline

Range: `1995-01-03..2026-06-01`

| Variant | Threads | Total ms | Read state ms | Read input ms | Group ms | Compute ms | Input rows | Output rows |
|---------|---------|----------|---------------|---------------|----------|------------|------------|-------------|
| TSV input | 8 | 12198 | 148 | 4290 | 6123 | 438 | 17990764 | 17990764 |
| RowBinary input | 8 | 6505 | 159 | 2225 | 2443 | 460 | 17990764 | 17990764 |

Baseline conclusion:

- TSV input parsing made full-range dry-run spend most time in input read and grouping.
- RowBinary input reduced full-range dry-run from 12.2s to 6.5s.
- After RowBinary input, compute remained a small part of total runtime.

## ClickHouse Evidence

Input query plan for the one-month all-market run:

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

## Baseline Bottleneck

The first measured bottleneck was not KDJ formula compute. It was ClickHouse I/O and input parsing/grouping:

- One-month all-market best run: `read_input_ms=328`, `group_ms=65`, `compute_ms=31`.
- Full-range TSV dry-run: `read_input_ms=4290`, `group_ms=6123`, `compute_ms=438`.

This justified:

1. Keeping KDJ parallelism at the security level.
2. Keeping `RAYON_NUM_THREADS=8` as the default Dagster setting.
3. Moving input scans to RowBinary.
4. Focusing subsequent optimization on write, staging, and partition replacement rather than RSV/KDJ micro-optimizations.

## Follow-Up Report

The later optimized full-range `replace-cascade` result is documented in:

- `docs/jobs/reports/2026-06-07-furnace-kdj-parallel-optimization.md`

