# EastMoney Generated Field Constants

This package contains generated constants used by the handwritten EastMoney
fetching and table conversion code.

- `fields.py` is generated from `docs/references/openapi/eastmoney__*.yaml` by
  `pipeline/scheduler/scripts/extract_eastmoney_schema_fields.py`.

EastMoney Parquet schemas are generated with all other dataset schemas in
`scheduler.defs.contract_schemas`.

Use `scheduler.defs.sources.eastmoney.generated.fields` as the canonical import
path for OpenAPI field-order constants.
