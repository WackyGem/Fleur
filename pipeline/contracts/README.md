# Data Contract Registry

`pipeline/contracts` is the machine-readable field contract source for raw ClickHouse tables, dbt staging models, and generated data dictionary docs.

Update dataset contracts first, then run:

```bash
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```
