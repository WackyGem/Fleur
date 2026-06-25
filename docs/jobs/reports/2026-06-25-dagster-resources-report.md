# Dagster Resources Report

日期：2026-06-25

范围：

- Dagster 项目：`pipeline/scheduler`
- 当前 definitions 入口：`scheduler.defs.definitions.defs`
- 梳理对象：当前注册的 assets、asset checks、jobs、schedules、sensors 和 Dagster resources；重点展开 resource / IO manager 适配层。
- 本次只读扫描，没有执行资产物化、回填或外部 API 调用。

## Evidence

命令：

```bash
cd pipeline
set -a; . ../.env; set +a
cd scheduler
uv run dg list defs --response-schema
uv run dg list defs --json
```

辅助源码扫描：

- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/source_bundle.py`
- `pipeline/scheduler/src/scheduler/defs/resources/`
- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/assets.py`
- `pipeline/scheduler/src/scheduler/defs/furnace/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/rearview/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/rearview/assets.py`
- `pipeline/scheduler/src/scheduler/components/fleur_dbt.py`

## Current Registered Definitions

`dg list defs --json` 当前结果：

| Type | Count | Notes |
|---|---:|---|
| Assets | 101 | 96 executable assets + 5 non-executable dbt relation handles |
| Asset checks | 383 | 由 dbt component 暴露；`enable_asset_checks: true` |
| Jobs | 21 | Source、ClickHouse raw sync、dbt/transformation、Rearview daily run |
| Schedules | 10 | Source daily/yearly schedules、stock daily build、Rearview daily run |
| Sensors | 2 | 默认 automation condition sensor + Slack run failure sensor |
| Resources | 11 | Base resources + Furnace component resource + Rearview resource |

Asset group 分布：

| Group | Count | Executable | Role |
|---|---:|---:|---|
| `s3_sources` | 21 | 21 | Sina、Jiuyan、THS、BaoStock、EastMoney、ChinaBond source assets |
| `clickhouse_raw` | 17 | 17 | 从 S3 Parquet 同步到 ClickHouse raw 层 |
| `dbt_staging` | 17 | 17 | dbt staging 清洗层 |
| `dbt_intermediate` | 24 | 24 | dbt intermediate、Furnace wrapper 和组合指标层 |
| `calculation` | 6 | 6 | Furnace Rust 计算资产 |
| `dbt_marts` | 10 | 10 | dbt marts 输出 |
| `rearview` | 1 | 1 | Rearview 策略组合每日运行触发资产 |
| `default` | 5 | 0 | dbt 暴露的上游 relation handles，不可直接物化 |

`default` 组当前包含：

- `fleur_calculation/calc_portfolio_closed_trade`
- `fleur_calculation/calc_portfolio_performance_metric`
- `fleur_calculation/calc_portfolio_performance_metric_status`
- `fleur_calculation/calc_portfolio_trade_metric`
- `fleur_portfolio/portfolio_run_snapshot`

## Definition Assembly

当前装配入口在 `pipeline/scheduler/src/scheduler/defs/definitions.py`：

1. `SOURCE_BUNDLES` 显式聚合 `sina`、`jiuyan`、`ths`、`baostock`、`eastmoney`、`chinabond`。
2. `base_defs` 注册 source bundle assets/jobs/schedules、`CLICKHOUSE_RAW_ASSETS`、`CLICKHOUSE_RAW_JOBS`、`TRANSFORMATION_JOBS`、`TRANSFORMATION_SCHEDULES`、`slack_asset_failure_sensor` 和 base resources。
3. `ComponentTree.for_project(...).build_defs("dbt")` 注册 dbt component definitions。
4. `ComponentTree.for_project(...).build_defs("furnace")` 注册 Furnace component definitions 和 `furnace_cli` resource。
5. 最终返回 `Definitions.merge(base_defs, dbt_defs, furnace_defs, REARVIEW_DEFS)`，其中 `REARVIEW_DEFS` 带入 `rearview_api` resource、Rearview asset/job/schedule。

## Registered Resources

| Resource key | Type | Registered by | Runtime responsibility | Primary users |
|---|---|---|---|---|
| `s3_io_manager` | `scheduler.defs.io_managers.s3_io_manager.S3IOManager` | `base_defs` | 写入/读取 S3 Parquet，按 asset metadata 处理 `latest_snapshot` 或 `partitioned`，并校验 contract-generated Parquet schema | 多数 `s3_sources` assets 通过 `io_manager_key="s3_io_manager"` 绑定 |
| `s3_settings` | `scheduler.defs.resources.s3.S3SettingsResource` | `base_defs` | 暴露 RustFS/S3 配置和 ClickHouse 访问 S3 的 endpoint override | BaoStock daily K、Jiuyan/THS compact、EastMoney、ClickHouse raw sync 等 |
| `image_object_store` | `scheduler.defs.resources.s3.ImageObjectStoreResource` | `base_defs` | 基于 S3 配置构造 Jiuyan 图片对象存储 | `jiuyan__industry_images`、`jiuyan__industry_ocr`、`jiuyan__industry_ocr_snapshot` |
| `industry_image_repository` | `scheduler.defs.resources.database.IndustryImageRepositoryResource` | `base_defs` | 基于 `PIPELINE_DATABASE_URL` 构造 PostgreSQL 图片/OCR 状态 repository | Jiuyan 图片下载、OCR 和 OCR snapshot assets |
| `jiuyan_ocr_settings` | `scheduler.defs.resources.ocr.JiuyanOcrSettingsResource` | `base_defs` | 暴露 Jiuyan OCR 服务 base URL、模型、超时、重试、并发和 stale running 阈值 | `jiuyan__industry_ocr` |
| `baostock_client_factory` | `scheduler.defs.resources.baostock.BaostockClientFactoryResource` | `base_defs` | 构造 BaoStock async TCP client | `baostock__query_stock_basic`、`baostock__query_history_k_data_plus_daily` |
| `http_client_factory` | `scheduler.defs.resources.http.HttpClientFactoryResource` | `base_defs` | 构造统一 HTTP client factory，封装通用 retry policy | Sina、Jiuyan、THS、EastMoney、ChinaBond HTTP source assets |
| `clickhouse` | `scheduler.defs.resources.clickhouse.ClickHouseResource` | `base_defs` | 构造 `clickhouse_connect` client，用于 raw sync | 17 个 `clickhouse_raw` assets |
| `slack` | `scheduler.defs.resources.slack.SlackAlertResource` | `base_defs` | 构造 Slack WebClient，解析 channel、proxy、Dagster run URL 和 code location | `slack_asset_failure_sensor` |
| `furnace_cli` | `scheduler.defs.resources.furnace.FurnaceCliResource` | Furnace component | 调用 Rust `furnace` CLI，解析 JSON summary，设置工作目录、超时和 Rayon 线程数 | 6 个 `calculation` assets |
| `rearview_api` | `scheduler.defs.rearview.resources.RearviewApiResource` | `REARVIEW_DEFS` | 调用 Rearview control-plane HTTP API 创建策略组合 daily runs | `rearview/strategy_portfolio_daily_runs` |

## Resource Configuration Ownership

资源配置仍集中在 `scheduler.defs.config.env` 和 resource class 字段上，业务资产通过 resource 注入，不直接读取环境变量。

| Area | Environment keys |
|---|---|
| S3 / RustFS | `RUSTFS_ENDPOINT`、`RUSTFS_BUCKET`、`RUSTFS_ACCESS_KEY`、`RUSTFS_SECRET_KEY`、`CLICKHOUSE_S3_ENDPOINT` |
| BaoStock | `BAOSTOCK_HOST`、`BAOSTOCK_PORT`、`BAOSTOCK_USERNAME`、`BAOSTOCK_PASSWORD` |
| ClickHouse | `CLICKHOUSE_HOST`、`CLICKHOUSE_PORT`、`CLICKHOUSE_DATABASE`、`CLICKHOUSE_USER`、`CLICKHOUSE_PASSWORD`、`CLICKHOUSE_SECURE`、`CLICKHOUSE_CONNECT_TIMEOUT_SECONDS`、`CLICKHOUSE_QUERY_TIMEOUT_SECONDS` |
| Pipeline database | `PIPELINE_DATABASE_URL` |
| Jiuyan OCR | `JIUYAN_OCR_BASE_URL`、`JIUYAN_OCR_MODEL_NAME`、`JIUYAN_OCR_TIMEOUT_SECONDS`、`JIUYAN_OCR_MAX_RETRIES`、`JIUYAN_OCR_MAX_CONCURRENT_REQUESTS`、`JIUYAN_OCR_STALE_RUNNING_SECONDS` |
| Slack alerts | `SLACK_BOT_TOKEN`、`SLACK_CHANNEL_ID`、`SLACK_HTTP_PROXY`、`DAGSTER_WEBSERVER_BASE_URL`、`DAGSTER_CODE_LOCATION_NAME` |
| Rearview API | `REARVIEW_API_BASE_URL` or `VITE_REARVIEW_API_BASE_URL`; default fallback is `http://127.0.0.1:34057` |

`HttpClientFactoryResource` 当前字段包含超时、connector limit、per-host limit 和 request delay，但 `factory()` 目前只把 `DEFAULT_RETRY_POLICY` 传入 `HttpClientFactory`；超时和 connector 字段是否应继续保留为 Dagster config surface，需要后续单独确认。

## Resource Usage By Domain

### Source Ingestion

- `s3_io_manager` 是 source asset 的主要输出通道，负责 S3 Parquet 写入和 contract schema 校验。
- `http_client_factory` 覆盖 HTTP source：`sina__trade_calendar`、Jiuyan HTTP assets、`ths__limit_up_pool`、EastMoney F10 年分区 assets、`chinabond__government_bond`。
- `baostock_client_factory` 只服务 BaoStock TCP source，不复用 HTTP resource。
- `s3_settings` 在需要主动读 S3 或给 ClickHouse 提供 S3 input config 的资产中显式注入；单纯输出 S3 Parquet 的资产通常通过 `s3_io_manager` 完成写入。

### Jiuyan OCR State

- `image_object_store` 把 Jiuyan 图片二进制对象存储封装在 S3 配置之上。
- `industry_image_repository` 把 OCR 状态、图片记录和 snapshot 状态隔离到 PostgreSQL repository。
- `jiuyan_ocr_settings` 只承载 OCR 服务运行参数，实际 HTTP 调用仍通过 `http_client_factory` 进入 OCR service。

### ClickHouse Raw Sync

- `clickhouse` 和 `s3_settings` 是 raw sync 的核心资源组合。
- `CLICKHOUSE_RAW_ASSETS` 由 `ENABLED_CLICKHOUSE_RAW_TABLE_SPECS` 生成，当前注册 17 个 executable raw assets。
- raw sync asset 的职责是把 source S3 Parquet 同步到 `fleur_raw`，不是做 staging 清洗或 mart 建模。

### dbt Component

- `FleurDbtProjectComponent` 当前不注册额外 Dagster resource。
- dbt component 负责把 dbt model asset key 改写为 flat model key、重写 dbt-to-dbt deps，并启用 dbt asset checks。
- 当前 383 个 asset checks 来自 dbt component 暴露的 tests/checks。

### Furnace Calculation

- `furnace_cli` 由 `pipeline/scheduler/src/scheduler/defs/furnace/definitions.py` 注册。
- 当前 Furnace 注册 6 个 calculation assets：KDJ、MA、RSI、BOLL、MACD、Price Pattern。
- `FurnaceCliResource` 负责构造 CLI 命令、设置 working dir、传递环境、处理超时、解析 stdout JSON summary，并把 summary 转换为 Dagster materialization metadata。

### Rearview Control Plane

- `rearview_api` 由 `REARVIEW_DEFS` 注册，不属于 source bundle。
- `rearview/strategy_portfolio_daily_runs` 是 daily-partitioned control-plane asset，调用 Rearview API 创建 active strategy portfolio daily runs。
- `portfolio__daily_run_schedule` 当前 cron 为 `0 20 * * *`。

### Alerting

- `slack` 只被 `slack_asset_failure_sensor` 使用。
- sensor 会解析 Slack channel、proxy、Dagster run URL 和 code location；缺少 required token/channel 会在资源调用时显式失败。

## Jobs And Schedules Snapshot

当前 jobs：

- Source jobs：`sina__trade_calendar_job`、`jiuyan__action_field_daily_job`、`jiuyan__action_field_compacted_job`、`jiuyan__industry_list_snapshot_job`、`jiuyan__industry_ocr_pipeline_job`、`jiuyan__industry_ocr_snapshot_job`、`ths__limit_up_pool_daily_job`、`ths__limit_up_pool_compacted_job`、`baostock__daily_job`、`eastmoney__daily_job`、`chinabond__government_bond_job`
- ClickHouse raw jobs：`clickhouse__raw_sync_all_job`、`clickhouse__raw_sync_snapshot_job`、`clickhouse__raw_sync_baostock_job`、`clickhouse__raw_sync_eastmoney_job`、`clickhouse__raw_sync_jiuyan_market_event_job`、`clickhouse__raw_sync_ths_market_event_job`
- Transformation jobs：`dbt__staging_build_job`、`dbt__marts_build_job`、`stock__daily_build_job`
- Rearview job：`strategy_portfolio__daily_run_job`

当前 schedules：

| Schedule | Cron |
|---|---|
| `sina__trade_calendar_schedule` | `0 9 25-31 12 *` |
| `jiuyan__action_field_daily_schedule` | `45 16 * * *` |
| `jiuyan__industry_list_snapshot_schedule` | `30 17 * * *` |
| `jiuyan__industry_ocr_pipeline_schedule` | `35 17 * * *` |
| `ths__limit_up_pool_daily_schedule` | `45 16 * * *` |
| `baostock__daily_schedule` | `35 17 * * *` |
| `eastmoney__daily_schedule` | `0 16 * * *` |
| `chinabond__government_bond_schedule` | `0 16 * * *` |
| `stock__daily_build_schedule` | `30 18 * * *` |
| `portfolio__daily_run_schedule` | `0 20 * * *` |

## Drift Notes

`docs/jobs/dagster-definitions-lineage-2026-06-10.md` 是历史快照，当前实现已经增加或变化：

- `SOURCE_BUNDLES` 新增 `chinabond`。
- 当前 assets 从 71 增至 101，asset checks 从旧快照未重点展开到当前 383。
- 当前 resources 从 10 变为 11，包含 `rearview_api`；同时 base defs 中已有 `clickhouse`、`slack`，Furnace component 注册 `furnace_cli`。
- 当前 calculation assets 增至 6，包含 `calc_stock_macd_daily`。
- 当前 schedules 增至 10，包含 `chinabond__government_bond_schedule` 和 `portfolio__daily_run_schedule`。

`docs/architecture/scheduler-architecture.md` 的 `Registered Resources` 与 `Source Bundles` 段落也已落后于当前代码：它没有列出 `chinabond`、`clickhouse`、`slack`、`furnace_cli` 和 `rearview_api`。后续应单独更新该架构入口，避免当前事实入口继续引用旧资源清单。

## Verification

已完成：

- `uv run dg list defs --response-schema`
- `uv run dg list defs --json`
- 源码只读扫描 resource 注册和资产函数 resource 参数
- `make docs-check`
- `git diff --check`
- `git diff --check --no-index /dev/null docs/jobs/reports/2026-06-25-dagster-resources-report.md`
