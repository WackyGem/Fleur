# Scheduler Module Boundaries

`pipeline/scheduler/src/scheduler/defs` uses one canonical import path per business symbol. Package `__init__.py` files are not compatibility re-export surfaces.

## Boundaries

| 目录 | 用途 |
|------|------|
| `automation/` | 跨数据源 Dagster job/schedule 工厂，如 `AssetJobSpec`、`ScheduleSpec`、`build_asset_job()`、`build_schedule()`、`build_year_refresh_schedule()` |
| `common/` | 无业务含义的纯 helper，如异步边界、并发 runner、时间、字符串、数字、schema、metadata、retry、通用类型 |
| `config/` | 环境变量 getter 与配置数据类；业务模块不要直接调用 `dg.EnvVar` |
| `resources/` | Dagster `ConfigurableResource` 适配层，负责 HTTP、BaoStock、S3、OCR、数据库等资源构造 |
| `storage/` | S3、object key、bytes、Parquet、dataset 写入/读取、通用 `ObjectStore`；不得依赖具体数据源定义 |
| `market/` | 跨数据源市场概念，如 asset key、证券范围、交易日历、A 股交易日 schedule 工厂 `build_trade_date_schedule()` |
| `http/` | HTTP client/factory、protocol、flatten、pagination、schema、HTTP 分区物化工具；不得依赖具体数据源定义 |
| `partitioning/` | 分区选择、backfill 限制、交易日过滤、部分失败阈值等通用策略 |
| `ocr/` | 通用 OCR client、schema 与 service；韭研业务编排保留在 `sources/jiuyan/` |
| `source_bundle.py` | `SourceBundle` 契约与 bundle flatten helper，是 definitions 聚合入口 |
| `sources/` | HTTP 数据源业务逻辑，按 `sina/`、`jiuyan/`、`ths/`、`eastmoney/` 分包；每个源自带 assets/services/definitions |
| `baostock/` | BaoStock TCP 客户端、协议、schema、service、资产与 schedule |
| `repositories/` | 数据库 repository，仅保留类 API，不导入 Dagster |
| `io_managers/` | Dagster IOManager 实现 |

## Required Patterns

- 新增数据源时，在对应源目录维护 `definitions.py` 并导出 `SourceBundle`，再由 `SOURCE_BUNDLES` 显式聚合。
- 通用 job/schedule 工厂只放在 `automation/schedules.py`；A 股交易日调度只放在 `market/schedules.py`。
- BaoStock 是 TCP 数据源，不从 `scheduler.defs.http` 复用 HTTP client 或调度入口；使用 `automation.schedules`、`market.schedules` 和 `resources.baostock`。
- `build_trade_date_schedule()` 属于 `market/schedules.py`，因为它依赖 `sina__trade_calendar` 作为 A 股交易日事实来源。
- 数据源代码通过 `resources/` 构造通用客户端，不直接 new `HttpClientFactory`、`BaostockAioTcpClient`、`AioHttpClient`。
- EastMoney 资产通过显式 `EASTMONEY_ASSETS` 管理，不使用动态 `globals()` 导出；只把 BaoStock 股票基础信息作为资产依赖，限速顺序不要编码成 asset lineage。

## Prohibited Patterns

- Do not recreate `scheduler.defs.http.schedules` or other source definition aggregation under `defs/http`.
- Do not make `http/` aggregate source jobs/schedules or import `scheduler.defs.sources`.
- Do not add generated EastMoney constants packages; EastMoney schema and source field names come from `scheduler.defs.contract_schemas`.
- Do not inject asset symbols into module globals with `globals()`.
- Do not read S3 environment settings from business assets or source services; pass `S3SettingsResource`, `S3Config`, or an explicit reader/service.
- Do not directly import `storage.parquet_readers` from data-source business code.
- Do not make `storage/` import source-specific modules. Put source-specific key mapping adapters under the owning source package.
