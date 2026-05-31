# EastMoney Generated Schema Constants

This package contains generated constants used by the handwritten EastMoney
fetching and table conversion code.

- `fields.py` is generated from `docs/references/openapi/eastmoney__*.yaml` by
  `pipeline/scheduler/scripts/extract_eastmoney_schema_fields.py`.
- `schemas.py` is generated from `docs/references/data_dict/*.md` by
  `pipeline/scheduler/scripts/generate_eastmoney_schemas.py`.

Use `scheduler.defs.sources.eastmoney.generated.fields` and
`scheduler.defs.sources.eastmoney.generated.schemas` as the canonical import
paths for generated constants.
