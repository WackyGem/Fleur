# Scheduler Architecture

本文是 `pipeline/scheduler` 的当前架构入口。`AGENTS.md` 只保留路径、命令和路由；scheduler 的结构、资产集合、资源和长期约束以本文和 `scheduler-module-boundaries.md` 为准。

## Definition Assembly

- 顶层 `src/scheduler/definitions.py` 只 re-export `scheduler.defs.definitions.defs`。
- `src/scheduler/defs/definitions.py` 通过 `SOURCE_BUNDLES` 统一装配 assets、jobs、schedules、resources。
- `SOURCE_BUNDLES` 顺序固定为 `sina`、`jiuyan`、`ths`、`baostock`、`eastmoney`。
- 每个数据源在自己的 `definitions.py` 中导出一个 `SourceBundle`，集中声明该源的 assets、jobs、schedules。
- 当前所有源资产使用 Dagster group `s3_sources`，通过 asset tags 区分 `source`、`layer`、`storage`、`state`、`modality`。

## Source Bundles

| Bundle | 主要资产 | 说明 |
|--------|----------|------|
| `sina` | `sina__trade_calendar` | A 股交易日历，作为交易日调度事实来源 |
| `jiuyan` | `action_field`、`industry_list`、`industry_images`、`industry_ocr` 及 compacted 资产 | 韭研 HTTP 数据、图片下载、OCR 与 PostgreSQL 状态 |
| `ths` | `limit_up_pool` 及 compacted 资产 | 同花顺涨停池日分区与年度压缩 |
| `baostock` | `query_stock_basic`、`query_history_k_data_plus_daily` | BaoStock TCP 数据源，基础证券信息与日 K 线 |
| `eastmoney` | 资产负债表、现金流、利润表、分红配股、股本历史 | 东方财富 F10 年分区资产，依赖 BaoStock 股票基础信息 |

## Architecture Patterns

- **SourceBundle 契约**：每个数据源显式提交 assets/jobs/schedules，`defs()` 只做聚合与资源注册。
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

## Environment Ownership

所有环境变量统一配置在根目录 `.env` 文件中。业务模块不要直接读取环境变量，应通过 `config/`、resource、factory 或 gateway 注入配置。

- `RUSTFS_*`：S3 兼容对象存储（RustFS/MinIO）
- `BAOSTOCK_*`：BaoStock TCP 连接配置
- `PIPELINE_DATABASE_URL`：PostgreSQL 连接字符串（OCR 状态管理）
- `JIUYAN_*`：聚源数据 API 认证
- `JIUYAN_OCR_*`：OCR 服务配置（超时、重试、并发）

## Related Documents

- 模块职责与禁止模式：`docs/architecture/scheduler-module-boundaries.md`
- 市场数据 raw assets 决策：`docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- S3 Parquet layout 决策：`docs/ADR/0002-s3-parquet-storage-layout.md`
- 交易日调度决策：`docs/ADR/0003-trade-calendar-driven-market-schedules.md`
- BaoStock TCP 客户端决策：`docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md`
