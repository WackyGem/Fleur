# AGENTS.md — mono-fleur 项目指南

## 项目结构

```
mono-fleur/
├── pipeline/           # Python 数据工作区，由 uv 管理
│   ├── scheduler/      # Dagster 调度项目（scheduler）
│   ├── elt/            # dbt 转换项目（elt）
│   └── migrate/        # Alembic 数据库迁移
├── deploy/             # 部署配置
│   ├── docker-compose.yml
│   ├── postgres/       # PostgreSQL 配置
│   └── jiuyan_industry_ocr.dev.yaml
├── app/                # 预留应用目录
├── docs/               # 项目文档与计划
├── .env                # 环境变量（不提交）
└── .env.example        # 环境变量模板
```

## Python 与工作区

- 使用 `uv` 管理 Python 依赖和虚拟环境。
- Python 版本固定在 `3.12.13`，配置于 `pipeline/.python-version`。
- 所有 Python、dbt、Dagster 和 `dg` 命令必须在 `pipeline/` 目录下通过 `uv run` 执行。
- 同步完整工作区：

```bash
cd pipeline
uv sync --all-packages --all-groups
```

### 子项目

| 子项目 | 路径 | 包管理器 | 说明 |
|--------|------|----------|------|
| scheduler | `pipeline/scheduler/` | uv (pyproject.toml) | Dagster 调度与资产定义 |
| elt | `pipeline/elt/` | uv (pyproject.toml) | dbt 数据转换 |
| migrate | `pipeline/migrate/` | uv (pyproject.toml) | Alembic 数据库迁移 |

## Dagster（scheduler）

- 项目路径：`pipeline/scheduler/`
- 项目名称：`scheduler`
- 在 `pipeline/` 目录下使用 `uv run dg ...` 和 `uv run dagster ...`
- 优先使用 `dg` CLI 进行项目检查和脚手架操作
- Dagster 主目录：`/storage/program/mono-fleur/.dagster`

### 定义装配与资产集合

- 顶层 `src/scheduler/definitions.py` 只 re-export `scheduler.defs.definitions.defs`。
- `src/scheduler/defs/definitions.py` 通过 `SOURCE_BUNDLES` 统一装配 assets、jobs、schedules、resources。
- `SOURCE_BUNDLES` 顺序固定为 `sina`、`jiuyan`、`ths`、`baostock`、`eastmoney`。
- 每个数据源在自己的 `definitions.py` 中导出一个 `SourceBundle`，集中声明该源的 assets、jobs、schedules。
- 当前所有源资产使用 Dagster group `s3_sources`，通过 asset tags 区分 `source`、`layer`、`storage`、`state`、`modality`。

| Bundle | 主要资产 | 说明 |
|--------|----------|------|
| `sina` | `sina__trade_calendar` | A 股交易日历，作为交易日调度事实来源 |
| `jiuyan` | `action_field`、`industry_list`、`industry_images`、`industry_ocr` 及 compacted 资产 | 韭研 HTTP 数据、图片下载、OCR 与 PostgreSQL 状态 |
| `ths` | `limit_up_pool` 及 compacted 资产 | 同花顺涨停池日分区与年度压缩 |
| `baostock` | `query_stock_basic`、`query_history_k_data_plus_daily` | BaoStock TCP 数据源，基础证券信息与日 K 线 |
| `eastmoney` | 资产负债表、现金流、利润表、分红配股、股本历史 | 东方财富 F10 年分区资产，依赖 BaoStock 股票基础信息 |

### scheduler 模块边界

`pipeline/scheduler/src/scheduler/defs/` 当前按职责分层：

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

边界要求：

- 新增数据源时，在对应源目录维护 `definitions.py` 并导出 `SourceBundle`，再由 `SOURCE_BUNDLES` 显式聚合。
- 通用 job/schedule 工厂只放在 `automation/schedules.py`；A 股交易日调度只放在 `market/schedules.py`。
- `http/` 只放 HTTP 基础设施和分区物化工具，不组装具体数据源 job/schedule，也不导入 `scheduler.defs.sources`。
- BaoStock 是 TCP 数据源，不应从 `scheduler.defs.http` 复用 HTTP client 或调度入口；应使用 `automation.schedules`、`market.schedules` 和 `resources.baostock`。
- `build_trade_date_schedule()` 属于 `market/schedules.py`，因为它依赖 `sina__trade_calendar` 作为 A 股交易日事实来源。
- 数据源代码通过 `resources/` 构造通用客户端，不直接 new `HttpClientFactory`、`BaostockAioTcpClient`、`AioHttpClient`。
- 数据源代码不要直接读取 `S3Config.from_env()`，不要直接导入 `storage.parquet_readers`。
- Eastmoney 资产通过显式 `EASTMONEY_ASSETS` 管理，不使用动态 `globals()` 导出；只把 BaoStock 股票基础信息作为资产依赖，限速顺序不要编码成资产 lineage。

### 关键架构模式

- **SourceBundle 契约**：每个数据源显式提交 assets/jobs/schedules，`defs()` 只做聚合与资源注册
- **Resource 适配层**：Dagster resource 封装环境配置、客户端工厂和外部连接，业务代码依赖 resource 而不是直接读环境变量或创建底层 client
- **Repository 模式**：`PostgresIndustryImageRepository` 封装所有数据库操作
- **Object Store 模式**：`ObjectStore` 提供通用二进制对象存储，`ImageObjectStore` 只保留图片/OCR 业务映射
- **Service 层**：HTTP、BaoStock、OCR 等业务流程提取至 service 模块，资产函数保持薄封装
- **资产契约元数据**：所有源资产必须保留 owner、kind tags、source/storage/layer tags 和分区/状态元数据
- **分区与失败策略**：`partitioning/policies.py` 统一处理 backfill 限制、交易日过滤和部分失败阈值
- **类型安全**：全项目使用准确类型，最小化 `Any` 使用

### 注册资源

`defs()` 当前注册以下 Dagster resources：

- `s3_io_manager`
- `s3_settings`
- `image_object_store`
- `industry_image_repository`
- `jiuyan_ocr_settings`
- `baostock_client_factory`
- `http_client_factory`

### 环境变量

所有环境变量统一配置在根目录 `.env` 文件中：

- `RUSTFS_*`：S3 兼容对象存储（RustFS/MinIO）
- `BAOSTOCK_*`：BaoStock TCP 连接配置
- `PIPELINE_DATABASE_URL`：PostgreSQL 连接字符串（OCR 状态管理）
- `JIUYAN_*`：聚源数据 API 认证
- `JIUYAN_OCR_*`：OCR 服务配置（超时、重试、并发）

## dbt（elt）

- 项目路径：`pipeline/elt/`
- 项目名称：`elt`
- 在 `pipeline/` 目录下使用 `uv run dbt ...`
- 优先使用定向命令，除非明确要求，不要运行整个 dbt 项目
- 开发时优先使用 `dbt build --select ...` 而非 `dbt run`
- 初始 `models/example` 内容已移除，保留空目录结构

## 数据库迁移（migrate）

- 迁移路径：`pipeline/migrate/`
- 使用 Alembic 管理 PostgreSQL 表结构
- 执行迁移：

```bash
cd pipeline/migrate
uv run alembic upgrade head
```

## 质量门禁

提交代码前必须通过以下检查：

```bash
cd pipeline

# 代码检查
uv run ruff check scheduler/src scheduler/tests migrate

# 代码格式化
uv run ruff format scheduler/src scheduler/tests migrate

# 类型检查
uv run pyright scheduler/src/scheduler scheduler/tests

# 测试
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing

# Dagster definitions 检查
cd scheduler
uv run dg check defs
```

## Git 与生成文件

- 不要在项目子目录中创建嵌套 Git 仓库
- 排除模板生成的 `.git`、`.gitignore`、`.dg`、日志和示例文件
- 根目录 `.gitignore` 已排除虚拟环境、dbt 构建产物、dbt 日志/包和 Dagster 本地状态
- `.env` 文件不得提交到版本控制

## MCP 路由

| 工具 | 用途 |
|------|------|
| `context7` | 查询库、框架、SDK、API、CLI 工具和云服务的最新文档。先解析库 ID，再查询文档 |
| Web 搜索 | 仅在需要当前外部信息且 Context7 不是正确来源时使用 |

## Skills 路由

| Skill | 用途 |
|-------|------|
| `dagster-expert` | 任何 Dagster 相关任务之前使用，包括资产、物化、组件、数据管道、调度、传感器、作业、项目结构、`dg` CLI 用法或 Dagster 概念问题 |
| `dignified-python` | Python 代码质量、类型提示、现代 Python 风格、pathlib、异常处理、接口、CLI 模式或 Python 审查/重构 |
| `using-dbt-for-analytics-engineering` | 构建或修改 dbt 模型、源、测试、SQL 转换、dbt 项目调试、数据探索或影响分析 |
| `running-dbt-commands` | 格式化或执行 dbt CLI 命令、选择 dbt 可执行文件、选择资源、编译、构建、测试或显示查询输出 |
| `adding-dbt-unit-test` | 添加 dbt 单元测试或对 dbt 模型逻辑实践 TDD |
| `answering-natural-language-questions-with-dbt` | 从仓库数据、指标、KPI、语义层或临时 SQL 回答业务/分析问题。不用于 dbt 模型开发 |
| `fetching-dbt-docs` | 查找 dbt Core、dbt Cloud/平台或 dbt 语义层的 dbt 文档 |
| `configuring-dbt-mcp-server` | 设置、配置或排查 AI 工具的 dbt MCP 服务器 |
