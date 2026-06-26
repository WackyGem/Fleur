# dbt BaoStock downstream build performance

UTC time: 2026-06-26T00:05:00Z

## Scope

- Command:

```bash
cd pipeline
set -a
. /storage/program/mono-fleur/.env
set +a
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+
```

- dbt version: `1.11.11`
- Adapter: `clickhouse=1.10.0`
- Artifact source: `pipeline/elt/target/run_results.json`
- Selected graph: `stg_baostock__query_history_k_data_plus_daily+`
- Result: `PASS=72 WARN=0 ERROR=0 SKIP=0 NO-OP=0 TOTAL=72`
- Total elapsed time: `374.86s`

## Data Size

| Table | Rows | Date range |
| --- | ---: | --- |
| `fleur_raw.baostock__query_history_k_data_plus_daily_compacted` | 20,292,499 | n/a |
| `fleur_staging.stg_baostock__query_history_k_data_plus_daily` | 20,292,499 | `1990-12-19..2026-06-25` |
| `fleur_intermediate.int_stock_quotes_daily_unadj` | 18,079,273 | `1995-01-03..2026-06-25` |
| `fleur_intermediate.int_stock_adjustment_factor` | 18,079,273 | n/a |
| `fleur_intermediate.int_stock_quotes_daily_adj` | 18,079,273 | `1995-01-03..2026-06-25` |
| `fleur_intermediate.int_stock_financial_valuation` | 298,556 | n/a |
| `fleur_marts.mart_stock_quotes_daily` | 18,079,273 | `1995-01-03..2026-06-25` |

## Summary

| Category | Count | Runtime | Share |
| --- | ---: | ---: | ---: |
| Table models | 8 | 205.89s | 54.9% |
| Data tests | 60 | 168.06s | 44.8% |
| Hooks / operations | 3 | 0.17s | <0.1% |
| View models | 1 | 0.10s | <0.1% |

The run is dominated by two areas:

- Full-table table builds on 18M-row quote datasets, especially `mart_stock_quotes_daily`.
- Full-table mart consistency tests that compare wide rows across the mart and intermediate tables.

## Model Runtime

| Model | Materialized | Runtime | Path |
| --- | --- | ---: | --- |
| `mart_stock_quotes_daily` | table | 119.31s | `models/marts/mart_stock_quotes_daily.sql` |
| `int_stock_quotes_daily_adj` | table | 35.83s | `models/intermediate/int_stock_quotes_daily_adj.sql` |
| `int_stock_quotes_daily_unadj` | table | 28.76s | `models/intermediate/int_stock_quotes_daily_unadj.sql` |
| `int_stock_adjustment_factor` | table | 12.68s | `models/intermediate/int_stock_adjustment_factor.sql` |
| `int_stock_financial_valuation` | table | 6.84s | `models/intermediate/int_stock_financial_valuation.sql` |
| `int_index_quotes_daily` | table | 2.12s | `models/intermediate/int_index_quotes_daily.sql` |
| `int_benchmark_returns_daily` | table | 0.21s | `models/intermediate/int_benchmark_returns_daily.sql` |
| `mart_benchmark_returns_daily` | table | 0.15s | `models/marts/mart_benchmark_returns_daily.sql` |
| `stg_baostock__query_history_k_data_plus_daily` | view | 0.10s | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.sql` |

## Slow Tests

| Test | Runtime | Path |
| --- | ---: | --- |
| `mart_stock_quotes_daily_quote_passthrough_matches` | 63.64s | `tests/marts/mart_stock_quotes_daily_quote_passthrough_matches.sql` |
| `mart_stock_quotes_daily_adjusted_passthrough_matches` | 37.10s | `tests/marts/mart_stock_quotes_daily_adjusted_passthrough_matches.sql` |
| `mart_stock_quotes_daily_financial_valuation_asof_matches` | 30.53s | `tests/marts/mart_stock_quotes_daily_financial_valuation_asof_matches.sql` |
| `mart_stock_quotes_daily_adjusted_key_coverage` | 9.27s | `tests/marts/mart_stock_quotes_daily_adjusted_key_coverage.sql` |
| `mart_stock_quotes_daily_key_set_matches_quotes` | 8.80s | `tests/marts/mart_stock_quotes_daily_key_set_matches_quotes.sql` |
| `int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date` | 4.21s | `tests/int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date.sql` |
| `unique_combination_of_columns_stg_baostock__query_history_k_data_plus_daily_security_code__trade_date` | 2.67s | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.yml` |
| `unique_combination_of_columns_mart_stock_quotes_daily_security_code__trade_date` | 1.94s | `models/marts/mart_stock_quotes_daily.yml` |
| `unique_combination_of_columns_int_stock_adjustment_factor_security_code__trade_date` | 1.87s | `models/intermediate/int_stock_adjustment_factor.yml` |
| `unique_combination_of_columns_int_stock_quotes_daily_adj_security_code__trade_date` | 1.71s | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| `unique_combination_of_columns_int_stock_quotes_daily_unadj_security_code__trade_date` | 1.37s | `models/intermediate/int_stock_quotes_daily_unadj.yml` |

The five mart-specific tests above account for `149.34s`, or about `39.8%` of the whole build. They are valuable regression tests, but they are expensive because they scan or join large 18M-row tables.

## Bottleneck Analysis

### `mart_stock_quotes_daily`

`mart_stock_quotes_daily` is the largest single model cost at `119.31s`. It rebuilds an 18M-row `MergeTree` table and joins:

- unadjusted quote facts from `int_stock_quotes_daily_unadj`
- adjusted quotes from `int_stock_quotes_daily_adj`
- financial valuation via `ASOF LEFT JOIN`
- KDJ indicators from `int_stock_kdj_daily`

This is expected to be expensive in a full rebuild. For daily BaoStock increments, the changed surface is usually the latest trade date or the current year, not the entire 1995-2026 history.

### `int_stock_quotes_daily_adj`

`int_stock_quotes_daily_adj` costs `35.83s`. It joins the full unadjusted quote table to the full adjustment factor table and materializes all adjusted price columns. Both inputs are 18M-row tables.

### `int_stock_quotes_daily_unadj`

`int_stock_quotes_daily_unadj` costs `28.76s`. It enriches quotes with stock universe, prior trading day quote values, share history, dividend metrics, market cap, turnover, amplitude, and limit-up/down prices. This model is the first heavy full-table expansion after the staging view.

### Mart passthrough tests

The three slowest tests are full-table equality checks:

- quote passthrough: `63.64s`
- adjusted passthrough: `37.10s`
- financial valuation ASOF match: `30.53s`

These tests re-run large joins or compare many columns across full mart/intermediate tables. They are the correct shape for high-confidence regression checks, but not cheap enough for every daily incremental run if latency matters.

## Optimization Recommendations

### 1. Add current-year or partition-level rebuild path

The affected tables are partitioned by `toYear(trade_date)`. For daily BaoStock increments, rebuild only the current year partition rather than all historical partitions.

Recommended target path:

- Keep full-refresh as the historical repair path.
- Add an incremental path for:
  - `int_stock_quotes_daily_unadj`
  - `int_stock_adjustment_factor`
  - `int_stock_quotes_daily_adj`
  - `mart_stock_quotes_daily`
- Use a `trade_date` / `year` boundary from Dagster run tags or dbt vars.

For dbt-clickhouse, current adapter docs list these relevant incremental strategies:

- `insert_overwrite`: requires `partition_by`, no `unique_key`; suitable for replacing whole year partitions.
- `delete_insert`: requires lightweight deletes and `unique_key`; suitable for key-level replacement when lightweight deletes are acceptable.

Given the existing `partition_by='toYear(trade_date)'`, `insert_overwrite` by year is the cleaner first candidate.

### 2. Split heavy mart regression tests by cadence

Keep the existing full-table mart tests for release, backfill, or nightly validation. For daily runs, add a narrower test selection that only covers the rebuilt date/year window.

Candidate split:

- Daily gate:
  - `not_null`
  - `unique_combination_of_columns`
  - key coverage / passthrough tests filtered to current year or recent window
- Full gate:
  - existing full-table passthrough and ASOF match tests

This can cut most of the `149.34s` mart-test cost from daily cycles while preserving a full validation path.

### 3. Introduce date-window predicates in custom data tests

The expensive custom tests can accept a dbt var such as `validation_start_date` or `validation_year` and default to full history only when the var is absent.

Example behavior:

```text
daily run: validate trade_date >= latest_trade_date - N days
repair run: validate toYear(trade_date) = target_year
full release run: validate all history
```

This is especially useful for:

- `mart_stock_quotes_daily_quote_passthrough_matches`
- `mart_stock_quotes_daily_adjusted_passthrough_matches`
- `mart_stock_quotes_daily_financial_valuation_asof_matches`
- `mart_stock_quotes_daily_adjusted_key_coverage`
- `mart_stock_quotes_daily_key_set_matches_quotes`

### 4. Revisit `mart_stock_quotes_daily` ASOF join cost

The mart model does an `ASOF LEFT JOIN` to financial valuation. The input valuation table is much smaller than quotes, but ASOF joins over 18M rows still contribute to full rebuild cost.

Potential improvements:

- Ensure `int_stock_financial_valuation` is physically ordered by `(security_code, report_date)` if ASOF is the primary access pattern.
- Consider precomputing valuation validity intervals if ASOF join remains a recurring cost.
- Keep `mart_stock_quotes_daily` ordered by `(trade_date, security_code)` for consumption, but benchmark whether intermediate join inputs need alternative orderings.

### 5. Avoid running full downstream `model+` for routine current-day updates

The command used here is correct for a complete post-source replacement validation. It is intentionally broad. For daily automation after BaoStock compacted raw sync, use a narrower selector once incremental models and windowed tests exist.

Recommended command families:

```bash
# Full validation after historical repair or contract-impacting changes
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+

# Future daily validation after incremental/window support
uv run dbt build --project-dir elt --profiles-dir elt \
  --select mart_stock_quotes_daily \
  --vars '{"target_year": 2026, "validation_start_date": "2026-06-25"}'
```

## Full Node Runtime

| # | Type | Node | Materialized | Status | Seconds | Path |
| ---: | --- | --- | --- | --- | ---: | --- |
| 1 | operation | `elt-on-run-start-0` | view | success | 0.13 | `./dbt_project.yml` |
| 2 | operation | `elt-on-run-start-1` | view | success | 0.02 | `./dbt_project.yml` |
| 3 | operation | `elt-on-run-start-2` | view | success | 0.02 | `./dbt_project.yml` |
| 4 | model | `stg_baostock__query_history_k_data_plus_daily` | view | success | 0.10 | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.sql` |
| 5 | test | `cn_security_code_format_stg_baostock__query_history_k_data_plus_daily_security_code` | test | pass | 0.11 | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.yml` |
| 6 | test | `not_null_stg_baostock__query_history_k_data_plus_daily_is_suspend` | test | pass | 0.03 | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.yml` |
| 7 | test | `not_null_stg_baostock__query_history_k_data_plus_daily_security_code` | test | pass | 0.03 | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.yml` |
| 8 | test | `not_null_stg_baostock__query_history_k_data_plus_daily_trade_date` | test | pass | 0.03 | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.yml` |
| 9 | test | `unique_combination_of_columns_stg_baostock__query_history_k_data_plus_daily_security_code__trade_date` | test | pass | 2.67 | `models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.yml` |
| 10 | model | `int_index_quotes_daily` | table | success | 2.12 | `models/intermediate/int_index_quotes_daily.sql` |
| 11 | model | `int_stock_quotes_daily_unadj` | table | success | 28.76 | `models/intermediate/int_stock_quotes_daily_unadj.sql` |
| 12 | test | `cn_security_code_format_int_index_quotes_daily_security_code` | test | pass | 0.12 | `models/intermediate/int_index_quotes_daily.yml` |
| 13 | test | `int_index_quotes_daily_return_daily_range` | test | pass | 0.10 | `tests/intermediate/int_index_quotes_daily_return_daily_range.sql` |
| 14 | test | `not_null_int_index_quotes_daily_security_code` | test | pass | 0.04 | `models/intermediate/int_index_quotes_daily.yml` |
| 15 | test | `not_null_int_index_quotes_daily_trade_date` | test | pass | 0.04 | `models/intermediate/int_index_quotes_daily.yml` |
| 16 | test | `relationships_int_index_quotes_daily_security_code__security_code__ref_int_index_basic_snapshot_` | test | pass | 0.11 | `models/intermediate/int_index_quotes_daily.yml` |
| 17 | test | `unique_combination_of_columns_int_index_quotes_daily_security_code__trade_date` | test | pass | 0.32 | `models/intermediate/int_index_quotes_daily.yml` |
| 18 | test | `cn_security_code_format_int_stock_quotes_daily_unadj_security_code` | test | pass | 0.38 | `models/intermediate/int_stock_quotes_daily_unadj.yml` |
| 19 | test | `int_stock_quotes_daily_unadj_change_pct_range` | test | pass | 0.14 | `tests/int_stock_quotes_daily_unadj_change_pct_range.sql` |
| 20 | test | `int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date` | test | pass | 4.21 | `tests/int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date.sql` |
| 21 | test | `int_stock_quotes_daily_unadj_trade_date_after_t1_restore` | test | pass | 0.04 | `tests/int_stock_quotes_daily_unadj_trade_date_after_t1_restore.sql` |
| 22 | test | `not_null_int_stock_quotes_daily_unadj_is_suspend` | test | pass | 0.04 | `models/intermediate/int_stock_quotes_daily_unadj.yml` |
| 23 | test | `not_null_int_stock_quotes_daily_unadj_security_code` | test | pass | 0.03 | `models/intermediate/int_stock_quotes_daily_unadj.yml` |
| 24 | test | `not_null_int_stock_quotes_daily_unadj_trade_date` | test | pass | 0.03 | `models/intermediate/int_stock_quotes_daily_unadj.yml` |
| 25 | test | `relationships_int_stock_quotes_daily_unadj_security_code__security_code__ref_int_stock_basic_snapshot_` | test | pass | 0.31 | `models/intermediate/int_stock_quotes_daily_unadj.yml` |
| 26 | test | `unique_combination_of_columns_int_stock_quotes_daily_unadj_security_code__trade_date` | test | pass | 1.37 | `models/intermediate/int_stock_quotes_daily_unadj.yml` |
| 27 | model | `int_benchmark_returns_daily` | table | success | 0.21 | `models/intermediate/int_benchmark_returns_daily.sql` |
| 28 | model | `int_stock_adjustment_factor` | table | success | 12.68 | `models/intermediate/int_stock_adjustment_factor.sql` |
| 29 | model | `int_stock_financial_valuation` | table | success | 6.84 | `models/intermediate/int_stock_financial_valuation.sql` |
| 30 | test | `cn_security_code_format_int_benchmark_returns_daily_security_code` | test | pass | 0.05 | `models/intermediate/int_benchmark_returns_daily.yml` |
| 31 | test | `not_null_int_benchmark_returns_daily_security_code` | test | pass | 0.04 | `models/intermediate/int_benchmark_returns_daily.yml` |
| 32 | test | `not_null_int_benchmark_returns_daily_trade_date` | test | pass | 0.03 | `models/intermediate/int_benchmark_returns_daily.yml` |
| 33 | test | `relationships_int_benchmark_returns_daily_security_code__security_code__ref_int_benchmark_basic_snapshot_` | test | pass | 0.04 | `models/intermediate/int_benchmark_returns_daily.yml` |
| 34 | test | `unique_combination_of_columns_int_benchmark_returns_daily_security_code__trade_date` | test | pass | 0.04 | `models/intermediate/int_benchmark_returns_daily.yml` |
| 35 | test | `cn_security_code_format_int_stock_adjustment_factor_security_code` | test | pass | 0.44 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 36 | test | `not_null_int_stock_adjustment_factor_backward_adjustment_factor` | test | pass | 0.08 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 37 | test | `not_null_int_stock_adjustment_factor_backward_adjustment_ratio` | test | pass | 0.08 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 38 | test | `not_null_int_stock_adjustment_factor_forward_adjustment_factor` | test | pass | 0.04 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 39 | test | `not_null_int_stock_adjustment_factor_forward_adjustment_ratio` | test | pass | 0.08 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 40 | test | `not_null_int_stock_adjustment_factor_security_code` | test | pass | 0.03 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 41 | test | `not_null_int_stock_adjustment_factor_trade_date` | test | pass | 0.02 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 42 | test | `unique_combination_of_columns_int_stock_adjustment_factor_security_code__trade_date` | test | pass | 1.87 | `models/intermediate/int_stock_adjustment_factor.yml` |
| 43 | test | `cn_security_code_format_int_stock_financial_valuation_security_code` | test | pass | 0.05 | `models/intermediate/int_stock_financial_valuation.yml` |
| 44 | test | `not_null_int_stock_financial_valuation_report_date` | test | pass | 0.04 | `models/intermediate/int_stock_financial_valuation.yml` |
| 45 | test | `not_null_int_stock_financial_valuation_security_code` | test | pass | 0.03 | `models/intermediate/int_stock_financial_valuation.yml` |
| 46 | test | `unique_combination_of_columns_int_stock_financial_valuation_security_code__report_date` | test | pass | 0.12 | `models/intermediate/int_stock_financial_valuation.yml` |
| 47 | model | `mart_benchmark_returns_daily` | table | success | 0.15 | `models/marts/mart_benchmark_returns_daily.sql` |
| 48 | model | `int_stock_quotes_daily_adj` | table | success | 35.83 | `models/intermediate/int_stock_quotes_daily_adj.sql` |
| 49 | test | `cn_security_code_format_mart_benchmark_returns_daily_security_code` | test | pass | 0.04 | `models/marts/mart_benchmark_returns_daily.yml` |
| 50 | test | `not_null_mart_benchmark_returns_daily_security_code` | test | pass | 0.04 | `models/marts/mart_benchmark_returns_daily.yml` |
| 51 | test | `not_null_mart_benchmark_returns_daily_trade_date` | test | pass | 0.03 | `models/marts/mart_benchmark_returns_daily.yml` |
| 52 | test | `relationships_mart_benchmark_returns_daily_security_code__security_code__ref_int_benchmark_basic_snapshot_` | test | pass | 0.03 | `models/marts/mart_benchmark_returns_daily.yml` |
| 53 | test | `unique_combination_of_columns_mart_benchmark_returns_daily_security_code__trade_date` | test | pass | 0.03 | `models/marts/mart_benchmark_returns_daily.yml` |
| 54 | test | `cn_security_code_format_int_stock_quotes_daily_adj_security_code` | test | pass | 0.39 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 55 | test | `not_null_int_stock_quotes_daily_adj_backward_adjustment_factor` | test | pass | 0.07 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 56 | test | `not_null_int_stock_quotes_daily_adj_backward_adjustment_ratio` | test | pass | 0.08 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 57 | test | `not_null_int_stock_quotes_daily_adj_forward_adjustment_factor` | test | pass | 0.04 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 58 | test | `not_null_int_stock_quotes_daily_adj_forward_adjustment_ratio` | test | pass | 0.08 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 59 | test | `not_null_int_stock_quotes_daily_adj_security_code` | test | pass | 0.03 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 60 | test | `not_null_int_stock_quotes_daily_adj_trade_date` | test | pass | 0.03 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 61 | test | `unique_combination_of_columns_int_stock_quotes_daily_adj_security_code__trade_date` | test | pass | 1.71 | `models/intermediate/int_stock_quotes_daily_adj.yml` |
| 62 | model | `mart_stock_quotes_daily` | table | success | 119.31 | `models/marts/mart_stock_quotes_daily.sql` |
| 63 | test | `cn_security_code_format_mart_stock_quotes_daily_security_code` | test | pass | 0.85 | `models/marts/mart_stock_quotes_daily.yml` |
| 64 | test | `mart_stock_quotes_daily_adjusted_key_coverage` | test | pass | 9.27 | `tests/marts/mart_stock_quotes_daily_adjusted_key_coverage.sql` |
| 65 | test | `mart_stock_quotes_daily_adjusted_passthrough_matches` | test | pass | 37.10 | `tests/marts/mart_stock_quotes_daily_adjusted_passthrough_matches.sql` |
| 66 | test | `mart_stock_quotes_daily_financial_valuation_asof_matches` | test | pass | 30.53 | `tests/marts/mart_stock_quotes_daily_financial_valuation_asof_matches.sql` |
| 67 | test | `mart_stock_quotes_daily_key_set_matches_quotes` | test | pass | 8.80 | `tests/marts/mart_stock_quotes_daily_key_set_matches_quotes.sql` |
| 68 | test | `mart_stock_quotes_daily_quote_passthrough_matches` | test | pass | 63.64 | `tests/marts/mart_stock_quotes_daily_quote_passthrough_matches.sql` |
| 69 | test | `not_null_mart_stock_quotes_daily_is_suspend` | test | pass | 0.03 | `models/marts/mart_stock_quotes_daily.yml` |
| 70 | test | `not_null_mart_stock_quotes_daily_security_code` | test | pass | 0.04 | `models/marts/mart_stock_quotes_daily.yml` |
| 71 | test | `not_null_mart_stock_quotes_daily_trade_date` | test | pass | 0.04 | `models/marts/mart_stock_quotes_daily.yml` |
| 72 | test | `unique_combination_of_columns_mart_stock_quotes_daily_security_code__trade_date` | test | pass | 1.94 | `models/marts/mart_stock_quotes_daily.yml` |
