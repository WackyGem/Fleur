# Scheduler Architecture

本文是 `pipeline/scheduler` 的当前架构入口。`AGENTS.md` 只保留路径、命令和路由；scheduler 的结构、资产集合、资源和长期约束以本文和 `scheduler-module-boundaries.md` 为准。

## Definition Assembly

- 顶层 `src/scheduler/definitions.py` 只 re-export `scheduler.defs.definitions.defs`。
- `src/scheduler/defs/definitions.py` 注册 `SOURCE_BUNDLES` 里的 source assets、`CLICKHOUSE_RAW_ASSETS`、backfill jobs、daily source-to-marts job/schedule、0051 example live job、Slack failure sensor 和 resources。
- `SOURCE_BUNDLES` 顺序固定为 `sina`、`jiuyan`、`ths`、`baostock`、`eastmoney`、`chinabond`。
- 每个数据源在自己的 `definitions.py` 中导出一个 `SourceBundle`，集中声明该源的 assets、jobs、schedules；生产 definitions 只消费其中的 assets，source-specific jobs/schedules 不再注册为 production daily 入口。
- 当前所有源资产使用 Dagster group `s3_sources`，通过 asset tags 区分 `source`、`layer`、`storage`、`state`、`modality`。
- 日常 source-to-marts 入口为 `daily__fetch_history_sources_to_marts_schedule_job`；唯一日常 ScheduleDefinition 为 `daily__fetch_history_sources_to_marts_schedule`，默认 stopped，当前 schedule run config 为 `dry_run=false`。
- `daily__fetch_history_sources_to_marts_schedule_job` 在 `all_source_to_marts + full` plan 末尾追加 `rearview/daily__portfolio_nav_liquidation` terminal step；该 step 是 unpartitioned asset materialization，等待 Rearview worker 终态并记录 fact-count metadata。
- 手动 history raw-only 回填入口为 `backfill__fetch_history_sources_to_raw_job`；历史 source-to-marts 入口为 `backfill__fetch_history_sources_to_marts_job`。
- `backfill__fetch_snapshot_sources_to_raw_job` 不再作为公开 definitions 注册；snapshot reference data 由 history raw-only 入口处理。
- `dbt__staging_build_job`、`dbt__marts_build_job`、`stock__daily_build_job`、`stock__daily_build_schedule`、source-specific schedules、ClickHouse raw sync jobs 和 `baostock_raw_sync_success_triggers_stock_daily_build` 不再注册为 production orchestration surface。

## Source Bundles

| Bundle | 主要资产 | 说明 |
|--------|----------|------|
| `sina` | `sina__trade_calendar` | A 股交易日历，作为交易日调度事实来源 |
| `jiuyan` | `action_field`、`industry_list`、`industry_images`、`industry_ocr` 及 compacted 资产 | 韭研 HTTP 数据、图片下载、OCR 与 PostgreSQL 状态 |
| `ths` | `limit_up_pool` 及 compacted 资产 | 同花顺涨停池日分区与年度压缩 |
| `baostock` | `query_stock_basic`、`query_history_k_data_plus_daily` | BaoStock TCP 数据源，基础证券信息与日 K 线；K 线抓取使用单连接顺序复用 |
| `eastmoney` | 资产负债表、现金流、利润表、分红配股、股本历史 | 东方财富 F10 年分区资产，依赖 BaoStock 股票基础信息 |
| `chinabond` | `government_bond` | 中债国债收益率曲线年分区 source |

## Manual Backfill Entrypoints

| Job | 角色 |
|-----|------|
| `backfill__fetch_history_sources_to_raw_job` | 手动 source/raw 修复入口，覆盖日期型 source/raw 和 `snapshot_reference_data`；`start_date` / `end_date` 在 snapshot scope 中保持必填但被忽略 |
| `backfill__fetch_history_sources_to_marts_job` | 手动 history source-to-marts 修复入口，在 source/raw 成功后推进非 Jiuyan、非 portfolio 的 dbt staging/intermediate、Furnace calculation、wrappers 和 marts |

Jiuyan 异动、行业列表、图片/OCR/OCR snapshot 后续独立设计 job；当前不进入 `backfill__fetch_history_sources_to_marts_job`。Portfolio backtest analytics 和 portfolio live 也保持独立。

## Daily Entrypoints

| 对象 | 角色 |
|------|------|
| `daily__fetch_history_sources_to_marts_schedule_job` | 唯一日常 source -> raw -> stg -> int -> calculation -> mart -> portfolio live controller job；把 `target_date` 映射成 `start_date=end_date=target_date` 复用 history source-to-marts registry，并在 full/all scope 末尾提交 `rearview/daily__portfolio_nav_liquidation` |
| `daily__fetch_history_sources_to_marts_schedule` | 唯一日常 source-to-marts ScheduleDefinition，`45 17 * * *` Asia/Shanghai，默认 stopped；启用后提交真实 daily runs |
| `rearview/daily__portfolio_nav_liquidation` | portfolio live 日度 NAV 清算 asset；无分区、无用户日期范围 config，由 daily controller 作为 terminal step 提交，内部调用 Rearview settlement-target、single-day daily-runs、status 和 fact-count APIs |
| `example__portfolio_live_job` | 策略搜索低位反转 example 手动回归入口；沿用 `rearview/example_0051_portfolio_live_run` asset 和 legacy 0051 ensure path，当前 fixture 为 `racingline_0051_low_reversal` / `v2`；默认解析该 example portfolio 的最新 settlement target，并只创建一个 latest date 的 full-window daily run，由 worker 一次性从建仓上下文清算到该日；可用 run config 指定较短 `end_date`；不挂 schedule，不进入 production daily schedule |

Daily controller 只提交 Dagster asset materialization child runs，不直接调用 source service、dbt CLI、Furnace CLI、Rearview HTTP API 或 ClickHouse 写入逻辑。`dry_run=true` 时只输出 plan，但 plan 仍展示 portfolio live terminal step。非 dry-run 的 daily Furnace step 使用 `append-latest`；历史修复仍使用 `backfill__fetch_history_sources_to_marts_job` 的 `replace-cascade`。

## Architecture Patterns

- **SourceBundle 契约**：每个数据源显式提交 assets/jobs/schedules；`defs()` 的 production surface 只注册 source assets，source-specific jobs/schedules 保留在模块内作为历史/调试对象，不作为生产日常入口。
- **Resource 适配层**：Dagster resource 封装环境配置、客户端工厂和外部连接，业务代码依赖 resource 而不是直接读环境变量或创建底层 client。
- **Repository 模式**：`PostgresIndustryImageRepository` 封装数据库操作。
- **Object Store 模式**：`ObjectStore` 提供通用二进制对象存储，`ImageObjectStore` 只保留图片/OCR 业务映射。
- **Service 层**：HTTP、BaoStock、OCR 等业务流程提取至 service 模块，asset 函数保持薄封装。
- **资产契约元数据**：所有源资产必须保留 owner、kind tags、source/storage/layer tags 和分区/状态元数据。
- **数据契约注册表**：ClickHouse raw specs、dbt source/staging YAML 和 data_dict 字段事实来自 `pipeline/contracts`，由 `pipeline/contract_tools` 校验和生成；source 业务代码不直接解析 contract。
- **分区与失败策略**：`partitioning/policies.py` 统一处理 backfill 限制、交易日过滤和部分失败阈值。
- **类型安全**：全项目使用准确类型，最小化 `Any` 使用。

## Registered Resources

`defs()` 当前注册以下 Dagster resources：

- `s3_io_manager`
- `s3_settings`
- `image_object_store`
- `industry_image_repository`
- `jiuyan_ocr_settings`
- `baostock_client_factory`
- `http_client_factory`
- `clickhouse`
- `slack`
- `furnace_cli`
- `rearview_api`

## Environment Ownership

所有环境变量统一配置在根目录 `.env` 文件中。业务模块不要直接读取环境变量，应通过 `config/`、resource、factory 或 gateway 注入配置。

- `RUSTFS_*`：S3 兼容对象存储（RustFS/MinIO）
- `BAOSTOCK_*`：BaoStock TCP 连接配置
- `PIPELINE_DATABASE_URL`：PostgreSQL 连接字符串（OCR 状态管理）
- `JIUYAN_*`：聚源数据 API 认证
- `JIUYAN_OCR_*`：OCR 服务配置（超时、重试、并发）

## Related Documents

- 模块职责与禁止模式：`docs/architecture/scheduler-module-boundaries.md`
- Source/Raw 统一回填 RFC：`docs/RFC/archive/0039-source-raw-backfill-complexity-baseline.md`
- Source/Raw 统一回填实施计划：`docs/plans/archive/0065-source-raw-unified-backfill-controller-implementation-plan.md`
- Backfill Source-to-Marts controller 计划：`docs/plans/archive/0066-backfill-source-to-marts-controller-plan.md`
- Daily Source-to-Marts clean-slate 计划：`docs/plans/archive/0067-daily-source-to-marts-clean-slate-orchestration-plan.md`
- Daily Source-to-Marts dry-run 报告：`docs/jobs/reports/2026-07-01-daily-fetch-history-sources-to-marts-schedule-job-dry-run.md`
- Strategy Portfolio 日度 NAV 清算入口收敛 RFC：`docs/RFC/archive/0045-strategy-portfolio-daily-nav-liquidation.md`
- Strategy Portfolio 日度 NAV 清算入口收敛计划：`docs/plans/archive/0073-strategy-portfolio-daily-nav-liquidation-plan.md`
- Strategy Portfolio 日度 NAV 清算入口收敛报告：`docs/jobs/reports/2026-07-02-strategy-portfolio-daily-nav-liquidation.md`
- 0051 低位反转 example RFC：`docs/RFC/archive/0044-racingline-0051-low-reversal-regression-case.md`
- 0051 example live job 计划：`docs/plans/archive/0072-racingline-0051-low-reversal-example-live-job-plan.md`
- 0051 example live job 验收报告：`docs/jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md`
- 市场数据 raw assets 决策：`docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- S3 Parquet layout 决策：`docs/ADR/0002-s3-parquet-storage-layout.md`
- 交易日调度决策：`docs/ADR/0003-trade-calendar-driven-market-schedules.md`
- BaoStock TCP 客户端决策：`docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md`
