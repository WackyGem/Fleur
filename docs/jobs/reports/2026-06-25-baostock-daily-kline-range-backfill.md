# BaoStock daily K-line range backfill

UTC time: 2026-06-25T23:10:00Z

## Scope

- Plan: `docs/plans/archive/0057-baostock-daily-kline-range-backfill-implementation-plan.md`
- RFC: `docs/RFC/0030-baostock-daily-kline-compacted-yearly-range-rebuild.md`
- Daily source asset: `source/baostock__query_history_k_data_plus_daily`
- Compacted source asset: `source/baostock__query_history_k_data_plus_daily_compacted`
- Raw asset: `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`
- ClickHouse raw table: `fleur_raw.baostock__query_history_k_data_plus_daily_compacted`
- Environment file: `/storage/program/fleur/.env`

## Date Boundary

The 2026 completeness window is based on the Sina trade calendar, not natural days.

- First valid 2026 trading day: `2026-01-05`
- Compacted cut-off trade date: `2026-06-25`
- Expected valid trading dates through cut-off: 113
- Candidate range backfill window: `2026-01-01...2026-06-25`

The final compacted and raw validation covers `2026-01-05..2026-06-25`.

## Dagster Runs

Commands were executed from `pipeline/` after loading `/storage/program/fleur/.env`.

```bash
set -a
. /storage/program/fleur/.env
set +a
cd pipeline

uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-25" \
  --config-json '{"ops":{"source__baostock__query_history_k_data_plus_daily":{"config":{"mode":"range_backfill","overwrite_existing_partitions":false,"cutoff_trade_date":"2026-06-25"}}}}'

uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026 \
  --config-json '{"ops":{"source__baostock__query_history_k_data_plus_daily_compacted":{"config":{"cutoff_trade_date":"2026-06-25"}}}}'

uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026
```

| Step | Run ID | Status |
| --- | --- | --- |
| Daily range backfill | `a1a246aa-93d8-4d28-90b3-fbc80acc88ea` | `SUCCESS` |
| 2026 compacted rebuild | `749efb38-fdf9-4ea2-ad0d-5ccaf9622151` | `SUCCESS` |
| 2026 ClickHouse raw sync | `6ad61ffc-08ff-451e-9ec1-8ee63009a7e6` | `SUCCESS` |

## Validation

| Check | Result |
| --- | --- |
| Expected trade dates | 113, `2026-01-05..2026-06-25` |
| Existing daily partitions | 113 |
| Missing daily partitions | 0 |
| Empty daily partitions | 0 |
| Daily total rows | 644,368 |
| Compacted schema | matches `baostock__query_history_k_data_plus_daily_compacted` contract |
| Compacted rows | 644,368 |
| Compacted date range | `2026-01-05..2026-06-25` |
| Compacted unique codes | 5,723 |
| Compacted duplicate `(date, code)` keys | 0 |
| Raw rows | 644,368 |
| Raw date range | `2026-01-05..2026-06-25` |
| Raw unique codes | 5,723 |
| Raw duplicate `(date, code)` keys | 0 |
| Raw `type="5"` rows after stock-basic join | 0 |

The S3 compacted row count equals the sum of all expected daily source partitions. ClickHouse raw row count equals the S3 compacted row count.

## Outcome

Plan 0057 is complete in dev:

- BaoStock daily K-line supports `daily` and `range_backfill` modes.
- The 2026 historical gap was filled through daily `trade_date=*` source partitions.
- The 2026 compacted object is rebuilt from complete daily partitions through the real first 2026 trading day to the compacted cut-off.
- ClickHouse raw 2026 partition now matches the rebuilt compacted object.
- ETF `type="5"` rows remain excluded.
