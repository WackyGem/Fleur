# AGENTS.md — mono-fleur 项目指南

## 项目结构

```
mono-fleur/
├── pipeline/           # Python 数据工作区，由 uv 管理
│   ├── scheduler/      # Dagster 调度项目（scheduler）
│   ├── elt/            # dbt 转换项目（elt）
│   ├── contract_tools/ # 数据契约校验与生成工具
│   ├── contracts/      # 数据契约注册表（字段事实源）
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

## 文档入口

- 架构总览：`docs/architecture/scheduler-architecture.md`
- 模块边界：`docs/architecture/scheduler-module-boundaries.md`
- 长期决策：`docs/ADR/`
- 方案与历史设计：`docs/RFC/`
- 执行计划：`docs/plans/`
- 质量优化：`docs/optimize/`
- 运行报告：`docs/jobs/reports/`
- 接口、数据字典和样例：`docs/references/`
- 项目 skills：`docs/skills/`

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
| contract_tools | `pipeline/contract_tools/` | uv (pyproject.toml) | contract registry 校验与生成 |
| migrate | `pipeline/migrate/` | uv (pyproject.toml) | Alembic 数据库迁移 |

## Dagster（scheduler）

- 项目路径：`pipeline/scheduler/`
- 项目名称：`scheduler`
- 在 `pipeline/` 目录下使用 `uv run dg ...` 和 `uv run dagster ...`
- 优先使用 `dg` CLI 进行项目检查和脚手架操作
- Dagster 主目录：`/storage/program/mono-fleur/.dagster`
- 架构入口：`docs/architecture/scheduler-architecture.md`
- 模块边界和禁止模式：`docs/architecture/scheduler-module-boundaries.md`
- 回填操作：`docs/skills/dg-backfill-runbook/SKILL.md`

## dbt（elt）

- 项目路径：`pipeline/elt/`
- 项目名称：`elt`
- 在 `pipeline/` 目录下使用 `uv run dbt ...`
- 优先使用定向命令，除非明确要求，不要运行整个 dbt 项目
- 开发时优先使用 `dbt build --select ...` 而非 `dbt run`
- 初始 `models/example` 内容已移除，保留空目录结构

## 数据契约（contracts）

- 字段事实源：`pipeline/contracts/datasets/*.yml`，范围到 ClickHouse raw 层为止
- 生成/校验工具：`pipeline/contract_tools/`
- dbt `sources.yml` 和 `docs/references/data_dict/*.md` 由 contract 生成或校验
- dbt `staging.yml`、`stg_*.sql`、stg 字段描述和 tests 由 `pipeline/elt` 项目维护，不写入数据契约
- 修改字段事实后运行：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```

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
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate

# 代码格式化
uv run ruff format scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate

# 类型检查
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests

# 测试
uv run pytest scheduler/tests contract_tools/tests --cov=scheduler/src/scheduler --cov=contract_tools/src/fleur_contracts --cov-report=term-missing

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
| `dg-backfill-runbook` | mono-fleur 的 Dagster 回填操作手册，用于选择 `dg launch` 命令、资产选择、partition 参数和各数据源回填模板 |
| `dignified-python` | Python 代码质量、类型提示、现代 Python 风格、pathlib、异常处理、接口、CLI 模式或 Python 审查/重构 |
| `using-dbt-for-analytics-engineering` | 构建或修改 dbt 模型、源、测试、SQL 转换、dbt 项目调试、数据探索或影响分析 |
| `running-dbt-commands` | 格式化或执行 dbt CLI 命令、选择 dbt 可执行文件、选择资源、编译、构建、测试或显示查询输出 |
| `adding-dbt-unit-test` | 添加 dbt 单元测试或对 dbt 模型逻辑实践 TDD |
| `answering-natural-language-questions-with-dbt` | 从仓库数据、指标、KPI、语义层或临时 SQL 回答业务/分析问题。不用于 dbt 模型开发 |
| `fetching-dbt-docs` | 查找 dbt Core、dbt Cloud/平台或 dbt 语义层的 dbt 文档 |
| `configuring-dbt-mcp-server` | 设置、配置或排查 AI 工具的 dbt MCP 服务器 |
| `fleur-contract-data-dictionary` | 维护数据契约、字段 glossary、中文字段描述、dbt YAML 和 data_dict 生成/校验工作流 |
| `fleur-harness` | 维护项目 harness、agent 可读性、docs/skills 路由、架构约束、长期计划、文档治理和质量闭环 |
| `fleur-worktree` | 管理 mono-fleur 的 Git worktree、多分支、多 agent 并行任务、隔离验证、合并和清理流程 |
| `chdb-sql` | 在本地文件（parquet/csv/json）、URL、S3 路径或远程数据库（Postgres、MySQL、MongoDB、ClickHouse Cloud）上跑分析 SQL，无需启动服务器。替代 MCP 做 ClickHouse 查询 |
| `chdb-datastore` | pandas DataFrame + ClickHouse 引擎加速，处理 tabular 数据的 filter/group/aggregate/join，也支持跨数据源 DataFrame 联合查询 |
| `clickhouse-best-practices` | 审查 ClickHouse schema、查询或配置时使用，包含 31 条规则，必须在提供建议前检查 |
| `clickhouse-architecture-advisor` | 设计 ClickHouse 架构、选择摄入或建模模式、将最佳实践转化为工作负载特定系统设计时使用 |
