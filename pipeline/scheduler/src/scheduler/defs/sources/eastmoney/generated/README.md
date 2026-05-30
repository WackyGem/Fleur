# EastMoney Generated Schema Constants

This package contains generated constants used by the handwritten EastMoney
fetching and table conversion code.

- `fields.py` is generated from `docs/references/openapi/eastmoney__*.yaml` by
  `pipeline/scheduler/scripts/extract_eastmoney_schema_fields.py`.
- `schemas.py` is generated from `docs/references/data_dict/*.md` by
  `pipeline/scheduler/scripts/generate_eastmoney_schemas.py`.

Keep compatibility re-export modules at `scheduler.defs.sources.eastmoney.fields`
and `scheduler.defs.sources.eastmoney.schemas` until downstream imports have
migrated.

