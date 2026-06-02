# Data Contract Registry

`pipeline/contracts` is the machine-readable field contract source for source payloads, S3 Parquet schemas, raw ClickHouse tables, generated dbt raw `sources.yml`, and generated data dictionary docs.

The contract scope stops at the ClickHouse raw layer. dbt `staging.yml`, `stg_*.sql`, staging column descriptions, and staging tests are owned by the `pipeline/elt` dbt project.

`pipeline/contracts/glossary/tables.yml` only describes raw tables. Canonical dbt field names and field-value standards live in `pipeline/elt/metadata/field_glossary.yml`.

Update dataset contracts first, then run:

```bash
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```
