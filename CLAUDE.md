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
uv run dg check defs --target-path scheduler  # 验证定义
uv run dg launch --target-path scheduler --assets <asset_name>  # 物化资产
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
uv run pytest scheduler/tests/test_<name>.py  # 运行指定测试
```

**质量门禁：**
```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate  # 代码检查
uv run ruff format scheduler/src scheduler/tests migrate  # 代码格式化
uv run pyright scheduler/src scheduler/tests              # 类型检查
```

## 架构

**Dagster 项目** (`pipeline/scheduler/`)：
- 定义从 `src/scheduler/defs/` 目录加载
- 管道模块：`baostock/`、`eastmoney/`、`http_resources/`、`jiuyan_industry_ocr/`
- 使用并发池：`baostock_run_pool` (1)、`eastmoney_run_pool` (3)
- 通过环境变量配置（见 `.env`）

**关键设计模式：**
- **Repository 模式**：`PostgresIndustryImageRepository` 封装数据库操作
- **Object Store 模式**：`ImageObjectStore` 提供 S3 文件系统抽象
- **Service 层**：业务逻辑提取至 `services.py`，便于测试和维护
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
- `pipeline/scheduler/services.py` - OCR 业务逻辑服务层
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

测试位于 `pipeline/scheduler/tests/`，覆盖：
- 资产函数逻辑
- 服务层业务逻辑
- 数据库迁移
- OCR 状态流转换

使用 `pytest` 运行测试，支持覆盖率报告。

## 重要

- 推理强度设置为 xhigh。请仔细思考任务，验证关键假设，考虑可行的替代方案，并优先考虑正确性、一致性和清晰度。

详见 `AGENTS.md` 获取详细的工具和 MCP 路由说明。
