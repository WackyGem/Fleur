# Raw date placeholder nullability remediation report

Date: 2026-06-02

Status: Completed

Plan: `docs/optimize/raw-date-placeholder-nullability-remediation-2026-06-02.md`

Run ledger: `docs/jobs/reports/2026-06-02-raw-date-placeholder-nullability-remediation-runs.tsv`

## Summary

The raw ClickHouse nullable date/time remediation is complete for the affected datasets in the
plan. The contract/schema generation now emits effective `Nullable(...)` ClickHouse types for
nullable raw fields, the raw sync path preserves nullable Parquet values, affected raw assets were
rematerialized, and raw profile reports were regenerated with status `Accepted`.

The final ClickHouse validation found zero remaining `1970-01-01` / `1970-01-01 00:00:00`
placeholder values in the affected fields. Missing source values are now represented as `NULL`.

## Backfill scope

The final rematerialization ledger contains 225 successful runs and no failures.

| Dataset | Runs | Scope |
|---|---:|---|
| `baostock__query_stock_basic` | 1 | snapshot |
| `eastmoney__balance` | 37 | yearly partitions `1990`-`2026` |
| `eastmoney__cashflow_sq` | 37 | yearly partitions `1990`-`2026` |
| `eastmoney__cashflow_ytd` | 37 | yearly partitions `1990`-`2026` |
| `eastmoney__income_sq` | 37 | yearly partitions `1990`-`2026` |
| `eastmoney__income_ytd` | 37 | yearly partitions `1990`-`2026` |
| `eastmoney__dividend_main` | 37 | yearly partitions `1990`-`2026` |
| `jiuyan__action_field_compacted` | 1 | available yearly partition |
| `jiuyan__industry_list` | 1 | snapshot |

## ClickHouse validation counts

Direct aggregate validation against `fleur_raw`:

| Table | Field | Rows | NULL count | Placeholder count |
|---|---|---:|---:|---:|
| `baostock__query_stock_basic` | `outDate` | 8,769 | 7,644 | 0 |
| `eastmoney__balance` | `UPDATE_DATE` | 284,265 | 976 | 0 |
| `eastmoney__cashflow_sq` | `UPDATE_DATE` | 274,016 | 449 | 0 |
| `eastmoney__cashflow_ytd` | `UPDATE_DATE` | 283,613 | 686 | 0 |
| `eastmoney__income_sq` | `UPDATE_DATE` | 279,918 | 193 | 0 |
| `eastmoney__income_ytd` | `UPDATE_DATE` | 298,396 | 523 | 0 |
| `eastmoney__dividend_main` | `EQUITY_RECORD_DATE` | 151,606 | 95,808 | 0 |
| `eastmoney__dividend_main` | `EX_DIVIDEND_DATE` | 151,606 | 96,900 | 0 |
| `eastmoney__dividend_main` | `PAY_CASH_DATE` | 151,606 | 99,965 | 0 |
| `eastmoney__dividend_main` | `GMDECISION_NOTICE_DATE` | 151,606 | 70,793 | 0 |
| `eastmoney__dividend_main` | `DAT_YAGGR` | 151,606 | 100,343 | 0 |
| `eastmoney__dividend_main` | `LAST_TRADE_DATE` | 151,606 | 151,606 | 0 |
| `jiuyan__action_field_compacted` | `delete_time` | 5,853 | 5,853 | 0 |
| `jiuyan__action_field_compacted` | `update_time` | 5,853 | 5,853 | 0 |
| `jiuyan__industry_list` | `delete_time` | 956 | 956 | 0 |

The ClickHouse aggregate queries were bounded with `max_execution_time`,
`timeout_before_checking_execution_speed`, `max_rows_to_read`, and `max_result_rows`.

## Raw profile regeneration

The following reports were regenerated with `--execute --status Accepted` and now report no
active date placeholder quality issue rows:

- `docs/references/raw_profile/baostock__query_stock_basic.md`
- `docs/references/raw_profile/eastmoney__balance.md`
- `docs/references/raw_profile/eastmoney__cashflow_sq.md`
- `docs/references/raw_profile/eastmoney__cashflow_ytd.md`
- `docs/references/raw_profile/eastmoney__income_sq.md`
- `docs/references/raw_profile/eastmoney__income_ytd.md`
- `docs/references/raw_profile/eastmoney__dividend_main.md`
- `docs/references/raw_profile/jiuyan__action_field_compacted.md`
- `docs/references/raw_profile/jiuyan__industry_list.md`

During final profiling, `profile_raw_source.py` was also fixed so string date-like columns such as
`REPORT_DATE_NAME` are compared to the string placeholder value instead of `toDate('1970-01-01')`.
Captured `dbt show` output is normalized line-by-line to avoid generated trailing whitespace.

## Validation commands

Commands run from `pipeline/` with `../.env` loaded:

```bash
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
uv run pytest contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
uv run pytest scheduler/tests/unit/clickhouse -q
uv run pytest contract_tools/tests/test_profile_raw_source.py -q
```

Observed results:

- Contract validation: 18 dataset contracts validated.
- Generated output check: current.
- Parquet validation: 15 objects checked, 3 missing source objects skipped
  (`source/jiuyan__action_field/year=2026/000000_0.parquet`,
  `source/jiuyan__industry_ocr/year=2026/000000_0.parquet`,
  `source/ths__limit_up_pool/year=2026/000000_0.parquet`).
- ClickHouse schema validation: 15 tables checked, 0 missing.
- `contract_tools/tests`: 32 passed before the final focused profiler regression patch.
- `scheduler/tests/unit/test_contract_schemas.py`: 3 passed.
- `scheduler/tests/unit/clickhouse`: 27 passed, 12 Dagster beta warnings.
- `contract_tools/tests/test_profile_raw_source.py`: 6 passed after the final profiler patch.

## ClickHouse rules applied

- Per `agent-connect-mcp`, ClickHouse credentials were loaded from the existing environment rather
  than prompting or hardcoding connection details.
- Per `agent-discovery-schema`, schema validation queried ClickHouse `system.columns` through the
  project validator before declaring the raw table schemas aligned.
- Per `agent-query-safety`, direct verification used aggregate-only queries with explicit query
  settings and bounded result sets.
- Per `schema-types-avoid-nullable`, `Nullable(...)` is used only where missing date/time values
  are semantically meaningful for raw source fidelity.

## Remaining notes

No raw-layer date placeholder remediation tasks remain for the affected fields in this plan.
Future staging work should use the regenerated `Accepted` raw profiles as the staging-readiness
inputs.
