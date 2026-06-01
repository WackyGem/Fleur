# ClickHouse raw sync smoke test

UTC time: 2026-06-01T09:31:00Z

## Scope

- Infrastructure: `deploy/docker-compose.yml` RustFS and ClickHouse services.
- Probe table: `raw.__raw_sync_probe`.
- Smoke asset: `clickhouse/raw/baostock__query_history_k_data_plus_daily`.
- Partition: `2026`.
- Input object: `source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet`.

## Commands

```bash
set -a
. ./.env
set +a
docker compose -f deploy/docker-compose.yml up -d rustfs rustfs-init clickhouse
```

```bash
set -a
. ./.env
set +a
make dagster-home
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily" \
  --partition 2026
```

```bash
cd pipeline
set -a
. ../.env
set +a
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily \
  --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

## Results

- `s3()` Parquet probe from ClickHouse to RustFS succeeded using Docker network endpoint `http://rustfs:9000`.
- `ALTER TABLE ... REPLACE PARTITION 2026 FROM ...` probe succeeded.
- `EXCHANGE TABLES` snapshot swap probe succeeded.
- Dagster raw sync run succeeded for partition `2026`.
- ClickHouse raw partition verification:

| rows | min_year | max_year | unique_codes |
|------|----------|----------|--------------|
| 679856 | 2026 | 2026 | 7262 |

`uniq(code) = 7262`, below the `LowCardinality(String)` threshold of 10000.

## Failure Handling

The smoke run exercised the staging-then-replace path. The implementation validates staging schema,
row count, partition range and LowCardinality cardinality before running `REPLACE PARTITION`, so
validation failures do not clear the existing raw partition.

