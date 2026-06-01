# Data Contract Registry

`pipeline/contracts` is the machine-readable field contract source for source payloads, S3 Parquet schemas, raw ClickHouse tables, dbt `sources.yml`, and generated data dictionary docs.

The contract scope stops at the ClickHouse raw layer. dbt `staging.yml`, `stg_*.sql`, staging column descriptions, and staging tests are owned by the `pipeline/elt` dbt project.

Update dataset contracts first, then run:

```bash
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```
