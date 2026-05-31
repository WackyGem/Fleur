# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在本仓库中工作时提供指导。

## 项目概览

数据管道 monorepo，使用 Dagster 编排、dbt 转换。由 `uv` 工作区管理，Python 3.12。

## 常用命令

**基础设施：**
```bash
make dev-up          # 启动 Docker 服务（Postgres、ClickHouse、RustFS、NATS）
make dev-down        # 停止服务
make dev-logs        # 查看服务日志
```

**Dagster：**
```bash
cd pipeline
uv run dg dev                              # 启动 Dagster Web UI
uv run dg launch --target-path scheduler --assets <asset_name>  # 物化资产
cd scheduler
uv run dg check defs                       # 验证定义
```

**dbt：**
```bash
cd pipeline
uv run dbt build --select <model>    # 构建指定模型
uv run dbt test                      # 运行测试
```

**依赖管理：**
```bash
cd pipeline
uv sync --all-packages --all-groups  # 同步所有工作区依赖
```

**测试：**
```bash
cd pipeline
uv run pytest scheduler/tests/           # 运行所有测试
uv run pytest scheduler/tests/unit/<path>/test_<name>.py  # 运行指定测试
```

**质量门禁：**
```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate  # 代码检查
uv run ruff format scheduler/src scheduler/tests migrate  # 代码格式化
uv run pyright scheduler/src/scheduler scheduler/tests    # 类型检查
```

## 架构

**Dagster 项目** (`pipeline/scheduler/`)：
- 顶层 `src/scheduler/definitions.py` 直接 re-export `scheduler.defs.definitions.defs`
- `src/scheduler/defs/definitions.py` 通过 `SOURCE_BUNDLES` 集中组装 assets、jobs、schedules、resources
- 每个数据源在自己的 `definitions.py` 中导出 `SourceBundle`；当前顺序为 `sina`、`jiuyan`、`ths`、`baostock`、`eastmoney`
- 资产/基础设施按职责分层：
  - `automation/`：跨数据源 Dagster job/schedule 工厂
  - `common/`：无业务含义的纯 helper，包括异步边界、并发、时间、schema、metadata、retry、通用类型
  - `config/`：环境变量 getter 与配置数据类
  - `resources/`：Dagster `ConfigurableResource` 适配层，封装 HTTP、BaoStock、S3、OCR、数据库等资源构造
  - `storage/`：S3、object key、bytes、Parquet、dataset 读写、通用对象存储
  - `market/`：asset key、证券范围、交易日历、A 股交易日 schedule 工厂
  - `http/`：HTTP client/factory、protocol、flatten、pagination、schema、HTTP 分区物化工具
  - `partitioning/`：分区选择、backfill 限制、交易日过滤、部分失败阈值策略
  - `ocr/`：通用 OCR client、schema、service
  - `source_bundle.py`：`SourceBundle` 契约与 bundle flatten helper
  - `sources/`：HTTP 数据源业务逻辑（`sina/`、`jiuyan/`、`ths/`、`eastmoney/`）
  - `baostock/`：BaoStock TCP 客户端、协议、schema、service、资产与 schedule
  - `repositories/`：数据库 repository，不导入 Dagster
  - `io_managers/`：Dagster IOManager
- 使用并发池：`baostock_run_pool` (1)、`eastmoney_run_pool` (3)
- 通过环境变量配置（见 `.env`）

**当前 SourceBundle：**
- `sina`：`sina__trade_calendar`
- `jiuyan`：`jiuyan__action_field`、`jiuyan__action_field_compacted`、`jiuyan__industry_list`、`jiuyan__industry_images`、`jiuyan__industry_ocr`
- `ths`：`ths__limit_up_pool`、`ths__limit_up_pool_compacted`
- `baostock`：`baostock__query_stock_basic`、`baostock__query_history_k_data_plus_daily`
- `eastmoney`：balance、cashflow、income、dividend、equity history 等 F10 年分区资产

**调度边界：**
- `automation/schedules.py` 放通用 Dagster 工厂：`AssetJobSpec`、`ScheduleSpec`、`build_asset_job()`、`build_schedule()`、`build_year_refresh_schedule()`
- `market/schedules.py` 放依赖 A 股交易日历的 `build_trade_date_schedule()`
- `http/` 不组装具体数据源 job/schedule，不定义或 re-export 调度工厂，不导入 `scheduler.defs.sources`
- BaoStock 是 TCP 数据源，不应从 `scheduler.defs.http` 复用 HTTP client 或调度入口
- 新增数据源时，在对应源目录维护 `definitions.py` 并导出 `SourceBundle`，再由 `SOURCE_BUNDLES` 显式聚合

**代码边界：**
- 数据源代码通过 `resources/` 构造通用客户端，不直接 new `HttpClientFactory`、`BaostockAioTcpClient`、`AioHttpClient`
- 数据源代码不要直接读取 `S3Config.from_env()`，不要直接导入 `storage.parquet_readers`
- `storage/` 和 `http/` 不导入 source definitions；`repositories/` 不导入 Dagster
- Eastmoney 资产使用显式 `EASTMONEY_ASSETS`，不使用动态 `globals()` 导出；只依赖 BaoStock 股票基础信息，限速顺序不要编码成资产 lineage

**关键设计模式：**
- **SourceBundle 契约**：每个数据源显式提交 assets/jobs/schedules，`defs()` 只做聚合和资源注册
- **Resource 适配层**：Dagster resource 封装环境配置、客户端工厂和外部连接
- **Repository 模式**：`PostgresIndustryImageRepository` 封装数据库操作
- **Object Store 模式**：`ObjectStore` 提供通用二进制对象存储，`ImageObjectStore` 只保留图片/OCR 业务映射
- **Service 层**：HTTP、BaoStock、OCR 等业务流程提取至 service 模块，资产函数保持薄封装
- **资产契约元数据**：所有源资产保留 owner、kind tags、source/storage/layer tags 和分区/状态元数据
- **分区与失败策略**：`partitioning/policies.py` 统一处理 backfill 限制、交易日过滤和部分失败阈值
- **类型安全**：全项目使用准确类型，最小化 `Any` 使用

**dbt 项目** (`pipeline/elt/`)：
- 标准 dbt 项目结构
- 模型在 `models/`，宏在 `macros/`
- 目标数据库通过 `PIPELINE_DATABASE_URL` 配置

**存储层：**
- PostgreSQL：元数据和事务数据
- ClickHouse：分析查询
- RustFS：S3 兼容对象存储
- NATS：事件消息

## 关键目录

- `pipeline/scheduler/src/scheduler/defs/` - Dagster 资产、调度、资源
- `pipeline/scheduler/src/scheduler/defs/source_bundle.py` - SourceBundle 聚合契约
- `pipeline/scheduler/src/scheduler/defs/resources/` - Dagster resource 适配层
- `pipeline/scheduler/src/scheduler/defs/partitioning/` - 分区、backfill、部分失败策略
- `pipeline/scheduler/src/scheduler/defs/ocr/service.py` - OCR 批处理服务
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_services.py` - 韭研 OCR 业务流程
- `pipeline/elt/models/` - dbt 转换模型
- `pipeline/migrate/` - Alembic 数据库迁移
- `deploy/` - Docker Compose 基础设施
- `docs/` - RFC、计划、ADR 文档

## 环境配置

复制 `.env.example` 到 `.env` 并更新凭证。关键服务：
- RustFS：对象存储（S3 API）
- ClickHouse：分析数据库
- PostgreSQL：元数据库
- NATS：消息代理

**重要：** 所有环境变量统一配置在根目录 `.env` 文件中，不要在子目录创建额外的 `.env` 文件。

## 开发工作流

1. 启动服务：`make dev-up`
2. 同步依赖：`cd pipeline && uv sync --all-packages --all-groups`
3. 运行 Dagster：`cd pipeline && uv run dg dev`
4. 通过 Web UI 或 CLI 物化资产
5. 运行 dbt 模型：`cd pipeline && uv run dbt build --select <model>`
6. 提交前通过质量门禁（ruff、pyright、pytest）

## 测试规范

测试位于 `pipeline/scheduler/tests/`：
- `tests/fakes/`：共享测试替身（HTTP、Dagster、storage、database、BaoStock）
- `tests/helpers/`：共享断言和路径辅助
- `tests/unit/`：按生产模块分层的单元测试
- `tests/integration/`：definitions、asset/job/schedule contract 测试

测试不应 patch 已删除旧模块路径，也不应让 fake 继承真实 client 来绕过类型检查。

## 重要

- 推理强度设置为 xhigh。请仔细思考任务，验证关键假设，考虑可行的替代方案，并优先考虑正确性、一致性和清晰度。

详见 `AGENTS.md` 获取详细的工具和 MCP 路由说明。
