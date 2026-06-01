# Field Type Normalization Migration Report

UTC time: 2026-06-01

## Scope

This report records the storage-state follow-up for `docs/plans/0020-field-type-normalization-debt-remediation-plan.md`.
The contract, scheduler schema, dbt staging and generated documentation changes are handled in the
Plan 0020 worktree. This report distinguishes those code-level fixes from already-published S3
Parquet objects and ClickHouse raw tables.

Per ClickHouse `schema-types-native-types`, `schema-types-minimize-bitwidth`,
`schema-types-lowcardinality` and `schema-types-avoid-nullable`, the contract now uses native
`Bool`/`Date` types where the source semantics justify them, keeps source payload types as observed,
and only retains `LowCardinality(String)` where statistics or stable enum semantics support it.

## Verification Commands

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
git diff --check
```

Additional storage-state probes were run against the currently available RustFS and ClickHouse
environment:

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

Both storage-state commands correctly detect historical objects/tables that have not yet been
rewritten to the new contract.

## Current Storage State

### S3 Parquet

| Dataset | Object | Status | Required action |
|---|---|---|---|
| `baostock__query_history_k_data_plus_daily` | `source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet` | Mismatch: `isST` is still `int8`; contract and scheduler now require `bool` | Re-materialize affected BaoStock yearly partitions after the schema change. Historical 1995-2026 objects should be considered stale until rewritten. |
| `eastmoney__dividend_allotment` | `source/eastmoney__dividend_allotment/year=2026/000000_0.parquet` | Mismatch: `EX_DIVIDEND_DATEE` is still `string`; contract and generated scheduler schema now require `date32[day]` | Re-materialize affected EastMoney dividend allotment yearly partitions after the schema change. Historical 1990-2026 objects should be considered stale until rewritten. |
| `jiuyan__action_field` | `source/jiuyan__action_field/year=2026/000000_0.parquet` | Missing in current probe | Source-only contract exists; materialize when that source partition is needed. No ClickHouse raw sync is expected. |
| `jiuyan__industry_ocr` | `source/jiuyan__industry_ocr/year=2026/000000_0.parquet` | Missing in current probe | Source-only contract exists; materialize when OCR source partitions are needed. No ClickHouse raw sync is expected. |
| `ths__limit_up_pool` | `source/ths__limit_up_pool/year=2026/000000_0.parquet` | Missing in current probe | Source-only contract exists; materialize when that source partition is needed. No ClickHouse raw sync is expected. |

All other probed S3 objects matched their contract schema at the sampled object path.

### ClickHouse Raw

| Dataset | Table | Status | Required action |
|---|---|---|---|
| `baostock__query_history_k_data_plus_daily` | `raw.baostock__query_history_k_data_plus_daily` | Mismatch: `isST` is still `Int8`; contract now requires `Bool` | Rebuild the raw table or run a controlled migration after the BaoStock source Parquet is rewritten. Do not treat the existing raw table as contract-compliant. |
| `baostock__query_stock_basic` and all EastMoney/Jiuyan/Sina/THS raw tables except the BaoStock history table | `raw.<dataset>` | Missing in current ClickHouse probe | No online ALTER was required during this pass because the tables are absent. They should be created from the updated contract on the next raw sync. |

## Raw Type Changes Requiring Migration Awareness

| Dataset | Field | Old raw type | New raw type | Storage status |
|---|---|---|---|---|
| `baostock__query_history_k_data_plus_daily` | `isST` | `Int8` | `Bool` | Existing Parquet and ClickHouse table still use the old type; requires re-materialization and raw rebuild. |
| `eastmoney__dividend_allotment` | `EX_DIVIDEND_DATEE` | `LowCardinality(String)` / Parquet `string` | `Date` / Parquet `date32[day]` | Current sampled Parquet still uses `string`; raw table is absent and should be created from the updated contract after source rewrite. |
| `eastmoney__dividend_main` | `INFO_CODE` | `LowCardinality(String)` | `String` | Raw table is absent; next raw sync should create the updated type. |
| `jiuyan__action_field_compacted` | `expound` | `LowCardinality(String)` | `String` | Raw table is absent; next raw sync should create the updated type. |
| `jiuyan__industry_list` | `industry_id` | `LowCardinality(String)` | `String` | Raw table is absent; next raw sync should create the updated type. |
| `jiuyan__industry_ocr_snapshot` | `relation` | `LowCardinality(String)` | `String` | Raw table is absent; next raw sync should create the updated type. |
| `ths__limit_up_pool_compacted` | `reason_type` | `LowCardinality(String)` | `String` | Raw table is absent; next raw sync should create the updated type. |

## Next Run Window

Use targeted Dagster runs rather than a whole-project rebuild:

```bash
cd pipeline
for year in $(seq 1995 2026); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/baostock__query_history_k_data_plus_daily" \
    --partition "$year"
done
```

```bash
cd pipeline
for year in $(seq 1990 2026); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/eastmoney__dividend_allotment" \
    --partition "$year"
done
```

After source objects are rewritten, run the corresponding ClickHouse raw sync jobs for changed
datasets and then rerun:

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```
