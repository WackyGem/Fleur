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
├── engines/            # Rust 后端和计算引擎工作区，由 Cargo 管理
├── deploy/             # 部署配置
│   ├── docker-compose.yml
│   ├── postgres/       # PostgreSQL 配置
│   └── jiuyan_industry_ocr.dev.yaml
├── app/                # 前端应用工作区（racingline 规划中）
├── docs/               # 项目文档与计划
├── .env                # 环境变量（不提交）
└── .env.example        # 环境变量模板
```

## 文档入口

- 文档总入口：`docs/README.md`
- 多工程系统地图：`docs/systems/README.md`
- 数据平台地图：`docs/systems/data-platform.md`
- 数据治理地图：`docs/systems/data-governance.md`
- Furnace 计算引擎地图：`docs/systems/furnace.md`
- Rearview 后端服务地图：`docs/systems/rearview.md`
- Racingline 前端工作台地图：`docs/systems/racingline.md`
- 部署与运行地图：`docs/systems/deploy-ops.md`
- 架构边界：`docs/architecture/`
- 模块边界：`docs/architecture/scheduler-module-boundaries.md`
- 长期决策：`docs/ADR/`
- 方案与历史设计：`docs/RFC/`
- 执行计划：`docs/plans/README.md`
- 质量优化：`docs/optimize/`
- 运行报告：`docs/jobs/reports/`
- dbt 模型设计：`docs/design/`
- 接口、数据字典和样例：`docs/references/`
- 项目 skills：`docs/skills/`
- Rust engines 文档地图：`engines/README.md`

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

## Rust 与 engines 工作区

- Rust workspace 路径：`engines/`
- 使用 Cargo 管理 Rust crate，不放入 `pipeline/` 的 uv 工作区。
- 所有 Rust / Cargo 命令在 `engines/` 目录下执行。
- engines 文档地图：`engines/README.md`
- Furnace 设计入口：`docs/RFC/0016-rust-furnace-compute-engine.md`
- Furnace KDJ 历史实施与性能计划：`docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`、`docs/plans/archive/0028-furnace-kdj-parallel-performance-implementation-plan.md`
- Furnace 运行报告：`docs/jobs/reports/2026-06-07-furnace-kdj-smoke-run.md`、`docs/jobs/reports/2026-06-07-furnace-kdj-performance-baseline.md`、`docs/jobs/reports/2026-06-07-furnace-kdj-parallel-optimization.md`
- 当前 crates：

| Crate | 路径 | 类型 | 说明 |
|-------|------|------|------|
| furnace | `engines/crates/furnace/` | binary | `furnace kdj` CLI 入口、参数解析、请求校验和 JSON summary 输出 |
| furnace-core | `engines/crates/furnace-core/` | library | KDJ 参数、输入/输出模型、单证券 RSV/KDJ 纯计算；不依赖 ClickHouse、Dagster、dbt、Rayon 或环境变量 |
| furnace-io | `engines/crates/furnace-io/` | library | ClickHouse DDL/SQL、`clickhouse-client` 执行、RowBinary 读写、按证券并行调度、staging/partition replace 和运行摘要 |

- Rust API 文档：

```bash
make rust-doc
make rust-doc-serve
```

### Furnace 边界

- 指标公式只放在 `furnace-core`，不要在 Python asset、dbt SQL 或 ClickHouse SQL 中重写 RSV/KDJ 递推公式。
- ClickHouse、RowBinary、Rayon 并行、staging 和分区替换逻辑放在 `furnace-io`。
- Dagster 通过 `pipeline/scheduler/src/scheduler/defs/resources/furnace.py` 调用 Rust CLI，传入运行参数并读取 JSON summary。
- 当前 Furnace 输出表：`fleur_calculation.calc_stock_kdj_daily`；dbt wrapper：`fleur_intermediate.int_stock_kdj_daily`。
- 生产 KDJ 写入只允许 canonical 参数 `KDJ(9,3,3)`；历史修正使用 `replace-cascade` 并级联到受影响证券的最新输入交易日。

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
- dbt canonical 字段治理入口：`pipeline/elt/metadata/field_glossary.yml`
- dbt staging 清洗边界：`docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- dbt staging 前置 raw profiling：`docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`、`docs/RFC/archive/0013-raw-source-profiling-before-dbt-staging.md`、`docs/plans/archive/0025-raw-source-profiling-before-dbt-staging-implementation-plan.md`
- 新增或重写 staging model 前先使用 `docs/skills/stg-model-readiness/SKILL.md`，并维护 `docs/references/raw_profile/<dataset>.md`
- 修改 staging model 后运行：`uv run dbt parse --project-dir elt --profiles-dir elt`、`uv run python elt/scripts/validate_staging_readiness.py` 和 `uv run python elt/scripts/validate_field_glossary.py`

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

## 前端与浏览器调试

- `racingline` 前端规划路径：`app/racingline/`
- 前端系统地图：`docs/systems/racingline.md`
- Playwright CLI 使用全局安装的 `@playwright/cli`，命令为 `playwright-cli`
- 当前浏览器调试环境通过 Docker `vnc-mini-desktop` 暴露 Chromium CDP 端口，默认 `PLAYWRIGHT_CDP_ENDPOINT=http://127.0.0.1:9222`
- CDP 连通性检查：

```bash
node scripts/check_playwright_cdp.mjs
```

## 质量门禁

提交代码前必须通过以下检查：

文档-only 变更至少运行：

```bash
make docs-check
git diff --check
```

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

涉及 Rust engines 时额外运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
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
| `dagster-http-source-asset` | 用户提供远端 HTTP/HTTPS 链接、API endpoint 或样例，希望新增 Dagster source asset，落 S3 Parquet 并按 contract 同步 ClickHouse raw 层时使用 |
| `dg-backfill-runbook` | mono-fleur 的 Dagster 回填操作手册，用于选择 `dg launch` 命令、资产选择、partition 参数和各数据源回填模板 |
| `dignified-python` | Python 代码质量、类型提示、现代 Python 风格、pathlib、异常处理、接口、CLI 模式或 Python 审查/重构 |
| `rust-best-practices` | 编写、审查或重构 Rust 代码时使用，覆盖所有权/借用、错误处理、性能、Clippy、文档和基础测试规范 |
| `rust-patterns` | 设计 Rust crate 结构、模块边界、trait/generic、领域类型、错误模型或并发模式时使用 |
| `rust-async-patterns` | 构建或调试 Tokio/async Rust 应用、异步 I/O、任务并发、channel、取消和 async 性能问题时使用 |
| `rust-testing` | 为 Rust 代码添加单元测试、集成测试、异步测试、property-based tests、mock、benchmark 或 TDD 工作流时使用 |
| `using-dbt-for-analytics-engineering` | 构建或修改 dbt 模型、源、测试、SQL 转换、dbt 项目调试、数据探索或影响分析 |
| `running-dbt-commands` | 格式化或执行 dbt CLI 命令、选择 dbt 可执行文件、选择资源、编译、构建、测试或显示查询输出 |
| `stg-model-readiness` | 新增或重写 dbt staging model 前使用，完成 raw source profiling、报告、staging 清洗建议和 readiness 校验 |
| `adding-dbt-unit-test` | 添加 dbt 单元测试或对 dbt 模型逻辑实践 TDD |
| `answering-natural-language-questions-with-dbt` | 从仓库数据、指标、KPI、语义层或临时 SQL 回答业务/分析问题。不用于 dbt 模型开发 |
| `fetching-dbt-docs` | 查找 dbt Core、dbt Cloud/平台或 dbt 语义层的 dbt 文档 |
| `configuring-dbt-mcp-server` | 设置、配置或排查 AI 工具的 dbt MCP 服务器 |
| `fleur-contract-data-dictionary` | 维护数据契约、字段 glossary、中文字段描述、dbt YAML 和 data_dict 生成/校验工作流 |
| `fleur-harness` | 维护项目 harness、agent 可读性、docs/skills 路由、架构约束、长期计划、文档治理和质量闭环 |
| `fleur-worktree` | 管理 mono-fleur 的 Git worktree、多分支、多 agent 并行任务、隔离验证、合并和清理流程 |
| `playwright-cdp-frontend-debug` | 使用全局 `playwright-cli` 通过 `vnc-mini-desktop` 暴露的 CDP 端点调试 `app/` 前端，检查截图、DOM、console、network 和响应式布局 |
| `chdb-sql` | 在本地文件（parquet/csv/json）、URL、S3 路径或远程数据库（Postgres、MySQL、MongoDB、ClickHouse Cloud）上跑分析 SQL，无需启动服务器。替代 MCP 做 ClickHouse 查询 |
| `chdb-datastore` | pandas DataFrame + ClickHouse 引擎加速，处理 tabular 数据的 filter/group/aggregate/join，也支持跨数据源 DataFrame 联合查询 |
| `clickhouse-best-practices` | 审查 ClickHouse schema、查询或配置时使用，包含 31 条规则，必须在提供建议前检查 |
| `clickhouse-architecture-advisor` | 设计 ClickHouse 架构、选择摄入或建模模式、将最佳实践转化为工作负载特定系统设计时使用 |
