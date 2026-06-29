# dbt BaoStock downstream performance optimization

UTC time: 2026-06-29T02:52:25Z

## Scope

This run validates Plan 0064 after deleting the three low-value mart field-matching tests, shrinking the stock daily Dagster path, and applying the low-risk `mart_stock_quotes_daily` KDJ join optimization.

- Command:

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj int_stock_kdj_daily mart_stock_quotes_daily
```

- dbt version: `1.11.11`
- Adapter: `clickhouse=1.10.0`
- Artifact source: `pipeline/elt/target/run_results.json`
- Invocation id: `8564e3f6-e478-4954-b1ea-7a64081ee598`
- Result: `PASS=50 WARN=0 ERROR=0 SKIP=0 NO-OP=0 TOTAL=50`
- Total elapsed time: `203.03s`

## Deleted Tests

The following tests are absent from the parsed dbt manifest and did not run:

| Test | 2026-06-26 baseline runtime |
| --- | ---: |
| `mart_stock_quotes_daily_quote_passthrough_matches` | `63.64s` |
| `mart_stock_quotes_daily_adjusted_passthrough_matches` | `37.10s` |
| `mart_stock_quotes_daily_financial_valuation_asof_matches` | `30.53s` |

Removed baseline cost: `131.27s`.

## Build Runtime

| Model | Runtime | Status |
| --- | ---: | --- |
| `int_stock_kdj_daily` | `0.10s` | success |
| `int_stock_quotes_daily_unadj` | `29.22s` | success |
| `int_stock_adjustment_factor` | `13.26s` | success |
| `int_stock_quotes_daily_adj` | `35.84s` | success |
| `mart_stock_quotes_daily` | `86.61s` | success |

## Retained Slow Tests

| Test | Runtime | Status |
| --- | ---: | --- |
| `mart_stock_quotes_daily_adjusted_key_coverage` | `8.96s` | pass |
| `mart_stock_quotes_daily_key_set_matches_quotes` | `9.32s` | pass |
| `int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date` | `4.20s` | pass |
| `unique_combination_of_columns_int_stock_kdj_daily_security_code__trade_date` | `2.30s` | pass |
| `unique_combination_of_columns_int_stock_quotes_daily_unadj_security_code__trade_date` | `1.46s` | pass |
| `unique_combination_of_columns_int_stock_adjustment_factor_security_code__trade_date` | `1.88s` | pass |
| `unique_combination_of_columns_int_stock_quotes_daily_adj_security_code__trade_date` | `1.70s` | pass |
| `unique_combination_of_columns_mart_stock_quotes_daily_security_code__trade_date` | `1.73s` | pass |

## Comparison

| Run | Selected graph | Result | Runtime |
| --- | --- | --- | ---: |
| 2026-06-26 baseline | `stg_baostock__query_history_k_data_plus_daily+` | `PASS=72` | `374.86s` |
| 2026-06-29 P0 selector/test pruning | fixed quote int + mart selector | `PASS=39` | `222.20s` |
| 2026-06-29 SQL optimized daily chain | quote int + KDJ wrapper + mart selector | `PASS=50` | `203.03s` |

The optimized selector removes the three wide mart field-matching tests and excludes unrelated BaoStock downstream objects from the daily path. It still performs full table rebuilds for the selected int/mart tables and retains key/basic quality gates.

## Mart SQL Baseline

`FORMAT Null` baselines were captured with ClickHouse `system.query_log` using `log_comment=plan0062_mart_*`.

| Slice | Join shape | Wall time | query_duration_ms | read_rows | read_bytes | memory_usage | query_id |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| quotes + adjusted | `LEFT ANY JOIN` | `5.74s` | `5752` | `36,168,986` | `1,464,843,933` | `3,435,625,170` | `b63865f3-c22d-46ae-905a-ad73eb2937f5` |
| quotes + financial valuation | `ASOF LEFT JOIN` | `1.22s` | `1224` | `18,383,049` | `426,642,214` | `58,627,292` | `51ac0f9e-c491-40ab-9ac9-3cef718a95a9` |
| quotes + KDJ | `LEFT JOIN` | `26.08s` | `26222` | `36,080,463` | `1,315,740,317` | `5,185,240,984` | `3e8ed0df-0cc9-40cf-a5bd-6eaca268359d` |
| full select | original KDJ `LEFT JOIN` | `74.51s` | `75424` | `54,463,512` | `7,477,841,931` | `7,385,498,476` | `dc06ea2d-54b0-444d-994d-72f56b64f3c2` |

KDJ wrapper uniqueness is covered by `unique_combination_of_columns_int_stock_kdj_daily_security_code__trade_date`, which passed in the final dbt build. Based on that key guarantee, `mart_stock_quotes_daily` changed its KDJ join from `LEFT JOIN` to `LEFT ANY JOIN`.

| Slice | Join shape | Wall time | query_duration_ms | read_rows | read_bytes | memory_usage | query_id |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| quotes + KDJ | `LEFT ANY JOIN` | n/a | `4455` | `36,080,463` | `1,315,740,317` | `3,427,523,568` | `12e1686e-1728-4d81-8aee-f6c13e36c67a` |
| full select | KDJ `LEFT ANY JOIN` | `42.28s` | `42827` | `54,463,512` | `7,477,841,931` | `8,722,109,240` | `b5106f36-4581-4968-82bc-05cc641e78c9` |

The KDJ slice improved from `26222ms` to `4455ms`, and the full select improved from `75424ms` to `42827ms`. This exceeded the Plan 0064 `10%` threshold, so the SQL change was kept.

## Remaining Bottlenecks

`mart_stock_quotes_daily` remains the largest single model cost at `86.61s`, down from `109.57s` in the P0-only run and `119.31s` in the original baseline. If the optimized daily path remains above the target SLA, the next step is:

- profile table materialization/write cost, because the optimized full select is `42.83s` while dbt table materialization is `86.61s`;
- evaluate physical table settings or write path only with before/after query log evidence;
- leave int-layer incremental design to a separate RFC.

## Dagster Job Changes

Plan 0064 Phase 2/3 implementation changes are covered by scheduler tests:

- `stock__daily_build_job` selection is now key-based for the quote int chain, calculation assets, `int_stock_kdj_daily`, and `mart_stock_quotes_daily`.
- `dbt__marts_build_job` remains available for manual full validation.
- `baostock_raw_sync_success_triggers_stock_daily_build` is registered with `DefaultSensorStatus.RUNNING`, listens for successful `clickhouse__raw_sync_baostock_job` runs, and launches `stock__daily_build_job`.
- The trigger passes run metadata tags and does not use `_sync_at`, raw sync state tables, or dbt vars watermarks.

## Validation

Completed:

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj int_stock_kdj_daily mart_stock_quotes_daily
uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py::test_stock_daily_job_selects_dbt_calculation_and_mart_assets \
  scheduler/tests/unit/furnace/test_furnace_definitions.py::test_baostock_raw_sync_success_sensor_launches_stock_daily_job \
  scheduler/tests/integration/test_definitions_and_schedules.py::test_registered_definitions_match_source_bundles \
  scheduler/tests/integration/test_definitions_and_schedules.py::test_stock_daily_job_splits_dbt_around_furnace_assets -q
```

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

All validation commands above passed. `pyright` emitted only its upstream version availability notice.
