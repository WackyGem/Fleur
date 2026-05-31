# Scheduler Module Boundaries

`pipeline/scheduler/src/scheduler/defs` uses one canonical import path per
business symbol. Package `__init__.py` files are not compatibility re-export
surfaces.

## Boundaries

- `automation/` contains cross-source Dagster job and schedule factories.
- `common/` contains pure helpers with no data-source business meaning.
- `config/` owns environment access and configuration models.
- `resources/` adapts configuration into Dagster resources.
- `storage/` owns generic object storage, S3 path handling, Parquet read/write,
  and dataset metadata. It must not import `scheduler.defs.sources.*`.
- `http/` owns generic HTTP clients, pagination, schemas, flattening, and
  partition materialization helpers. It must not aggregate or import source
  definitions.
- `market/` owns cross-source market concepts such as asset keys, securities,
  trade calendars, and A-share trading-day schedules.
- `sources/` owns data-source business logic and source-specific adapters.
- `repositories/` owns database repository implementations and must not depend
  on Dagster execution context.
- `io_managers/` adapts Dagster IO manager context into storage services.

## Prohibited Patterns

- Do not recreate `scheduler.defs.http.schedules` or other source definition
  aggregation under `defs/http`.
- Do not add compatibility re-export modules for generated EastMoney constants;
  use `scheduler.defs.sources.eastmoney.generated.fields` and
  `scheduler.defs.sources.eastmoney.generated.schemas`.
- Do not inject asset symbols into module globals with `globals()`.
- Do not read S3 environment settings from business assets or source services;
  pass `S3SettingsResource`, `S3Config`, or an explicit reader/service.
- Do not make `storage/` import source-specific modules. Put source-specific
  key mapping adapters under the owning source package.
