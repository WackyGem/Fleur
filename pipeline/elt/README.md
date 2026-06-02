# mono-fleur dbt project

`pipeline/elt` owns canonical field names, staging transformations, reusable dbt field docs, generic data tests, and dbt manifest linting.

The canonical field glossary is `metadata/field_glossary.yml`. It is separate from `pipeline/contracts`, which stops at source, Parquet, and ClickHouse raw field facts.

Generated raw source metadata lives in `models/sources.yml` and should be regenerated from contracts, not edited by hand.

Core local checks:

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

Before adding or rewriting a staging model, create or update the corresponding raw source profile in `docs/references/raw_profile/`:

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py --source raw --table <dataset> --execute --output ../docs/references/raw_profile/<dataset>.md
```

`security_code` uses `<6位证券代码>.<交易所代码>` as the canonical format, for example `601088.SH`.
