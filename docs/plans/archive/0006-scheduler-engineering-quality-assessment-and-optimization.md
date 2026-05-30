# Plan 0006: Scheduler 工程质量评估与优化计划

状态：草案

评估日期：2026-05-28

关联 RFC：

- 无

参考资料：

- `AGENTS.md`
- `docs/plans/0001-sina-trade-calendar-s3-ingestion.md`
- `docs/plans/0002-baostock-aio-tcp-client.md`
- `docs/plans/0003-eastmoney-f10-ingestion.md`
- `docs/plans/0004-http-client-refactor-and-rfc0003-implementation.md`
- `docs/plans/0005-jiuyan-industry-list-ocr-implementation.md`
- `pipeline/pyproject.toml`
- `pipeline/scheduler/pyproject.toml`
- `pipeline/scheduler/src/scheduler/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/pipeline_defs.py`
- `pipeline/scheduler/src/scheduler/defs/`
- `pipeline/scheduler/tests/`
- `pipeline/migrate/`
- Dagster 当前文档：`dg check defs` 和 `dg list defs` 用于校验、列出项目 definitions。

## 目标

本计划对 `pipeline/scheduler` 的工程质量做整体评估，并给出可分阶段落地的优化路线。评估范围包括 Dagster definitions 组织、Python 代码质量、依赖与工具链、测试策略、数据管道运行边界、可观测性、配置管理和本地开发体验。

核心目标：

- 明确当前 scheduler 工程质量状态和主要风险。
- 建立可重复执行的质量门禁，避免新增资产时质量回退。
- 收敛 Python、Dagster、测试、格式化和依赖管理约束。
- 降低资产函数里的副作用复杂度，提升可测试性和故障定位效率。
- 为后续持续新增数据源、OCR 流程和状态型 pipeline 提供一致工程标准。

## 非目标

本计划不直接包含：

- 修改现有业务采集逻辑。
- 新增数据源、asset、schedule、sensor 或 dbt 模型。
- 调整 S3 对象路径语义或 PostgreSQL 表结构。
- 替换 Dagster、PyArrow、aiohttp、psycopg 等核心技术选型。
- 为真实外部接口做大规模回归采集。

## 当前工程现状

`pipeline/scheduler` 已经从初始 scaffold 演进为实际可运行的 Dagster 项目。当前包含：

- 16 个 Dagster asset。
- 8 个 job。
- 5 个 schedule。
- 2 个 sensor，其中一个是 Dagster 默认 automation condition sensor。
- 1 个资源：`s3_io_manager`。
- 36 个 scheduler 源码 Python 文件。
- 8 个测试文件，当前共 59 个测试用例。
- 1 套 Alembic migration，用于 `jiuyan_industry_images` 状态表。

当前主干能力：

- HTTP 资源采集已经收敛到 `scheduler.defs.http_resources`，并有共享 `AioHttpClient`。
- S3 Parquet 输出已通过 `S3IOManager` 抽象出 `latest_snapshot` 和 `partitioned` 两种模式。
- BaoStock TCP client、EastMoney F10、Sina 交易日历、市场事件 HTTP 资源、韭研 OCR 流程均已有模块拆分和单元测试。
- OCR 图片下载和 OCR 状态推进已引入 PostgreSQL 状态表，具备可领取、可重试和失败标记能力。
- Dagster definitions 当前可以成功加载。

## 已执行基线检查

本次评估执行了以下命令：

```bash
cd pipeline/scheduler
uv run dg check defs --verbose
uv run dg list defs --json
uv run dg check toml
uv run dg check yaml
```

结果：

- `dg check defs --verbose` 通过：component YAML 和 definitions 均成功加载。
- `dg list defs --json` 成功列出当前 assets/jobs/resources/schedules/sensors。
- `dg check toml` 通过。
- `dg check yaml` 通过。

测试基线：

```bash
cd pipeline
uv run pytest scheduler/tests
uv run --with pytest pytest scheduler/tests
```

结果：

- `uv run pytest scheduler/tests` 失败，原因是当前 workspace 环境没有声明或安装 `pytest`。
- `uv run --with pytest pytest scheduler/tests` 通过：59 passed，11 个 Dagster `backfill_policy` beta warning。

静态检查基线：

```bash
cd pipeline
uv run --with ruff ruff check scheduler/src scheduler/tests migrate
uv run --with ruff ruff format --check scheduler/src scheduler/tests migrate
```

结果：

- `ruff check` 发现 1 个未使用 import：`scheduler/src/scheduler/defs/http_resources/schedules.py` 中的 `datetime.date`。
- `ruff format --check` 显示 35 个文件需要格式化，说明当前没有统一格式化门禁。

## 总体评价

当前 scheduler 的业务实现质量高于初始项目阶段：核心模块已经按数据源和基础设施拆分，关键 HTTP、S3、OCR schema、图片 URL 解析和 Sina 解码逻辑都有单元测试覆盖，Dagster definitions 可以加载，测试在临时补充 pytest 后全部通过。

主要短板集中在工程治理层，而不是单个业务函数是否可用：

- 缺少正式声明的测试、lint、format、type check 工具链。
- `pipeline/scheduler/pyproject.toml` 的 Python 版本约束与 workspace 约束、代码实际语法能力不一致。
- 部分 asset 函数仍承担编排、远端调用、状态更新、S3 写入和错误聚合等多重职责。
- 配置全部以模块级 `EnvVar` 和 `from_env()` 读取为主，缺少统一的配置校验和本地开发说明。
- OCR 和状态型 pipeline 的 PostgreSQL/S3 副作用边界已经可用，但还没有形成通用 resource 或 repository 抽象。
- 可观测性依赖手写 metadata，字段命名和失败策略尚未完全统一。
- 仓库中可见 `__pycache__`、`.pytest_cache` 等生成目录，虽未被 git 跟踪，但本地工作区卫生需要自动化处理。

## 风险分级

### P0：质量门禁缺失

问题：

- `pytest` 没有出现在 `pipeline/scheduler` 的 dev dependency 中。
- 没有 Ruff、type checker、coverage 或统一 `tool.pytest.ini_options` 配置。
- 当前测试能通过，但不能通过标准 `uv run pytest scheduler/tests` 复现。
- Ruff 尚未纳入项目，第一次启用会立刻暴露格式化漂移。

影响：

- 新增数据源时无法稳定复用同一套本地和 CI 命令。
- 代码风格和 import 卫生依赖人工 review。
- 后续引入更多状态型 pipeline 后，回归成本会上升。

### P0：Python 版本约束不一致

问题：

- `pipeline/pyproject.toml` 要求 `requires-python = ">=3.12"`。
- `pipeline/.python-version` 固定为 `3.12.13`。
- `pipeline/scheduler/pyproject.toml` 却声明 `requires-python = ">=3.10,<3.15"`。
- 代码中已经使用更现代的能力，例如 `typing.Self`，这不是 Python 3.10 的可运行基线。

影响：

- scheduler wheel 元数据会错误宣称兼容 Python 3.10。
- CI、部署镜像或 IDE 按 scheduler 子项目元数据解析时，可能选择不兼容解释器。

### P1：资产函数职责偏重

问题：

- 多个 asset 函数中使用 `asyncio.run(...)` 桥接异步实现，这是 Dagster 同步 asset 的可接受模式，但当前 asset 内部仍聚合了较多职责。
- `jiuyan_industry_ocr/assets.py` 同时处理配置读取、上游 S3 读取、图片发现、状态 upsert、下载、OCR 调用、S3 写入、失败率判断和 metadata 汇总。
- `eastmoney/assets.py`、`baostock/assets.py` 和 HTTP market event 资产也存在较重的 materialization 函数。

影响：

- 单元测试可以覆盖 helper，但 asset 级失败时定位仍需要理解较长函数。
- 后续新增状态型 pipeline 时容易复制复杂 asset 模板。

### P1：资源和副作用边界还不统一

问题：

- S3 普通 Parquet 输出通过 `S3IOManager` 管理，但 OCR 图片、OCR 单图结果、PostgreSQL 状态表由 asset 内部显式调用 helper。
- PostgreSQL 访问层是函数集合，尚未形成可注入 repository/resource。
- HTTP client 已抽象，但在具体数据源里仍需要手动管理失败策略和统计字段。

影响：

- 纯函数测试和外部副作用测试边界不完全一致。
- 状态表操作、S3 写对象和远端请求的 mock 方式分散。
- 未来加入第二个状态型 pipeline 时，容易重新实现一套 claim/mark success/mark failed 模式。

### P1：配置和运行说明不足

问题：

- `pipeline/scheduler/README.md` 仍是 Dagster 模板说明，未反映当前项目的 `uv run`、S3、PostgreSQL、OCR、BaoStock、JiuYan token 等真实配置。
- 环境变量集中在 `config.py`，但没有 `.env.example` 或配置校验命令。
- `dg check defs` 不需要真实 env，实际 materialization 需要的 env 缺少文档化边界。

影响：

- 新开发者无法快速区分“加载 definitions 所需配置”和“真实运行某个 asset 所需配置”。
- 本地调试 OCR、S3、PostgreSQL 时容易遗漏变量。

### P1：可观测性字段未完全标准化

问题：

- 多个 asset 手写 metadata，命名总体可读，但缺少统一字段集。
- 成功、跳过、失败、远端请求、重试、写入路径样例等字段在不同模块之间不完全一致。
- OCR 资产有失败率阈值，但 HTTP market event、BaoStock、EastMoney 的失败策略和 metadata 没有形成统一文档。

影响：

- Dagster UI 中跨数据源比较运行质量不够直接。
- 事故复盘需要阅读具体 asset 代码确认字段含义。

### P2：测试结构需要升级

问题：

- 当前 59 个测试主要覆盖纯解析、client、schema 和部分 asset helper。
- 缺少统一 fixtures、fake S3 filesystem、fake Postgres repository、fake Dagster context 的测试工具层。
- Alembic migration 没有自动测试。
- 当前没有 coverage 基线。

影响：

- 测试数量继续增长后会出现重复 fake 对象。
- 状态型 pipeline 的端到端行为只能靠人工或真实环境验证。

### P2：本地工作区卫生

问题：

- 本地可见 `__pycache__`、`.pytest_cache` 等生成目录。
- 根 `.gitignore` 已覆盖这些目录和文件，当前未发现被 git 跟踪，但缺少清理命令或 pre-commit 钩子。

影响：

- 不影响当前代码正确性，但会降低本地搜索和文件浏览信噪比。

## 优化原则

- 先建立质量门禁，再做大规模格式化或结构重构。
- 优先保留现有业务行为，不把工程质量优化和业务语义调整混在同一批改动。
- 对有副作用的逻辑，先抽出可注入边界，再扩大测试覆盖。
- 对机械格式化、import 清理和类型收敛，独立提交，避免掩盖行为变更。
- 继续使用 Dagster 的 definitions 加载和 `dg` CLI 做项目健康检查。
- 继续从 `pipeline/` 使用 `uv run` 运行 Python、测试、dbt、Dagster 命令；Dagster 项目内 `dg check/list` 可从 `pipeline/scheduler/` 执行。

## 阶段计划

### 阶段 1：建立可重复质量门禁

目标：

- 让任何开发者能用同一组命令复现当前健康状态。
- 在不改业务行为的前提下引入格式化、lint 和测试依赖。

建议改动：

- 在 `pipeline/scheduler/pyproject.toml` 的 dev dependency 中增加：
  - `pytest`
  - `ruff`
  - `coverage` 或 `pytest-cov`
  - 类型检查工具，建议优先评估 `pyright` 或 `basedpyright`
- 统一 Python 版本约束：
  - 推荐将 `pipeline/scheduler/pyproject.toml` 调整为 `requires-python = ">=3.12"`，与 workspace 和 `.python-version` 一致。
- 增加 `[tool.ruff]`、`[tool.ruff.lint]`、`[tool.ruff.format]`、`[tool.pytest.ini_options]`。
- 先修复 Ruff 当前唯一 lint 问题：删除 `http_resources/schedules.py` 中未使用的 `date` import。
- 单独执行一次 Ruff format，作为机械格式化提交。
- 在文档中固定本地质量命令。

建议验收命令：

```bash
cd pipeline
uv sync --all-packages --all-groups
uv run pytest scheduler/tests
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format --check scheduler/src scheduler/tests migrate

cd scheduler
uv run dg check defs --verbose
uv run dg check toml
uv run dg check yaml
```

验收标准：

- 上述命令全部通过。
- 不再需要 `uv run --with pytest ...` 或 `uv run --with ruff ...`。
- Ruff format 不再报告需要重排的文件。

### 阶段 2：整理配置和开发文档

目标：

- 让本地开发、definitions 校验、真实 asset materialization 的配置要求清晰分层。

建议改动：

- 重写 `pipeline/scheduler/README.md`，替换模板内容。
- 新增 `pipeline/scheduler/.env.example` 或 `docs/references/scheduler-env.md`。
- 按能力域分组说明环境变量：
  - RustFS/S3：`RUSTFS_ENDPOINT`、`RUSTFS_BUCKET`、`RUSTFS_ACCESS_KEY`、`RUSTFS_SECRET_KEY`
  - BaoStock：`BAOSTOCK_HOST`、`BAOSTOCK_PORT`、`BAOSTOCK_USERNAME`、`BAOSTOCK_PASSWORD`
  - JiuYan API：`JIUYAN_TOKEN`、`JIUYAN_COOKIE`
  - OCR：`JIUYAN_OCR_BASE_URL`、`JIUYAN_OCR_MODEL_NAME`、`JIUYAN_OCR_TIMEOUT_SECONDS`、`JIUYAN_OCR_MAX_RETRIES`、`JIUYAN_OCR_MAX_CONCURRENT_REQUESTS`、`JIUYAN_OCR_STALE_RUNNING_SECONDS`
  - Pipeline DB：`PIPELINE_DATABASE_URL`
- 明确哪些命令不需要真实环境变量：
  - `dg check defs`
  - `dg list defs`
  - 大多数单元测试
- 明确哪些命令需要真实服务：
  - 真实 materialization
  - S3 Parquet 写入
  - OCR asset
  - BaoStock / JiuYan / EastMoney / Sina 远端采集

验收标准：

- README 不再是 Dagster 模板。
- 新开发者可以只按文档跑通 `dg check` 和测试。
- 真实运行某个 asset 时，缺失 env 的失败信息可定位到具体变量。

### 阶段 3：收敛 asset 函数职责

目标：

- 让 asset 函数主要负责 Dagster 边界，业务流程下沉到可测试 service/helper。

建议改动：

- 为每类复杂资产建立清晰分层：
  - `assets.py`：Dagster decorators、config schema、MaterializeResult。
  - `services.py` 或现有 domain module：业务流程编排。
  - `schemas.py`：PyArrow schema、dataclass、TypedDict。
  - `repositories.py` / `postgres.py`：状态表访问。
  - `clients.py`：远端 API client。
- 优先拆分 `jiuyan_industry_ocr/assets.py`：
  - 图片发现流程。
  - 图片下载流程。
  - OCR 领取和处理流程。
  - metadata 汇总和失败率策略。
- 然后评估 `eastmoney/assets.py` 和 `baostock/assets.py`，把通用 materialization timing/metadata pattern 抽成局部 helper。

验收标准：

- 单个 asset 函数只保留配置读取、调用 service、返回 `MaterializeResult`。
- 复杂流程函数可在不构造真实 Dagster context 的情况下测试。
- 行为不变，现有 59 个测试继续通过。

### 阶段 4：统一副作用资源边界

目标：

- 让 S3、PostgreSQL、HTTP、OCR 等外部副作用的注入方式一致，降低测试成本。

建议改动：

- 为 PostgreSQL 状态表访问引入 repository class 或 Dagster resource。
- 为 OCR 图片和单图结果写入封装 `ImageObjectStore` 或等价边界。
- 保留 `S3IOManager` 管理标准 Parquet asset 输出，不强行让 object-per-image 流程走 IO manager。
- 对状态型 pipeline 抽象通用模式：
  - discover
  - upsert pending
  - claim
  - process one
  - mark success
  - mark failed
  - summarize metadata
- 给 repository 和 object store 提供 fake 实现或测试 fixture。

验收标准：

- OCR 流程测试不需要 monkeypatch 多个模块级函数。
- 新增第二个状态型 pipeline 时，可以复用 claim/mark/metadata 结构。
- PostgreSQL SQL 仍保持显式、可读、可单独测试。

### 阶段 5：增强类型检查和接口约束

目标：

- 在不制造大量噪音的前提下，引入类型检查，优先保护公共边界和状态型代码。

建议改动：

- 先对 `scheduler.defs.http_resources.client`、`scheduler.defs.io_managers.s3_io_manager`、`scheduler.defs.jiuyan_industry_ocr`、`scheduler.defs.baostock` 启用类型检查。
- 将 `Any` 使用限制在 Dagster IO manager 边界、JSON 解码边界和第三方库边界。
- 为配置 dict、metadata dict、OCR row、HTTP payload 建立 TypedDict 或 dataclass。
- 逐步减少裸 `Mapping[str, object]` 在业务层的传播。

验收标准：

- 类型检查命令纳入质量门禁。
- 新增公共函数必须有返回类型。
- 外部 JSON 进入系统后尽早转换为内部 dataclass 或明确 schema。

### 阶段 6：补齐集成测试和 migration 测试

目标：

- 覆盖当前单元测试未覆盖的外部状态边界。

建议改动：

- 为 Alembic migration 增加最小测试：
  - migration 可以 upgrade。
  - 表、索引、check constraint 存在。
  - downgrade 可执行。
- 为 OCR 状态流增加 fake Postgres 或临时数据库测试：
  - pending -> running -> success。
  - pending -> running -> failed。
  - stale running 可重新领取。
  - force OCR 可重新领取 success。
- 为 S3 写入增加 fake filesystem 或临时对象存储测试。
- 对 Dagster definitions 增加测试：
  - 关键 asset 存在。
  - job selection 符合预期。
  - schedule cron 符合预期。

验收标准：

- 测试覆盖状态流的成功、失败、重试、幂等分支。
- migration 错误能在 CI 阶段发现。
- coverage 有初始基线，并对核心模块设置最低阈值。

### 阶段 7：标准化可观测性和运行手册

目标：

- 让 Dagster UI 中的 metadata 足够统一，便于跨资产排障。

建议改动：

- 定义 asset metadata 字段约定：
  - `row_count`
  - `column_count`
  - `request_count`
  - `retry_count`
  - `success_count`
  - `failure_count`
  - `skip_count`
  - `s3_keys_sample`
  - `asset_function_seconds`
  - `remote_fetch_seconds`
  - `write_seconds`
- 每类 asset 文档化失败策略：
  - 全失败是否 raise。
  - 部分失败阈值。
  - 空结果是否允许。
  - 重跑是否覆盖。
- 编写运行手册：
  - 如何回填某年 EastMoney。
  - 如何按交易日回填 market event。
  - 如何重跑指定 OCR 图片。
  - 如何清理 stale OCR running 状态。

验收标准：

- 新增 asset 必须声明 metadata 字段和失败策略。
- 运维操作不需要直接阅读源码才能确定 run config。

## 建议优先级

第一批必须优先完成：

1. 统一 `requires-python` 到 Python 3.12。
2. 增加 `pytest`、`ruff` 和测试配置。
3. 修复 Ruff 当前 lint 问题。
4. 单独执行 Ruff format。
5. 更新 README 和环境变量说明。

第二批建议随后完成：

1. 拆分 `jiuyan_industry_ocr/assets.py` 的流程职责。
2. 为 PostgreSQL 状态访问和 object-per-image S3 写入建立可注入边界。
3. 增加 migration 和 OCR 状态流测试。

第三批作为持续治理：

1. 引入类型检查并逐步收紧。
2. 统一 metadata 和失败策略。
3. 补齐运行手册和 coverage 基线。

## 质量门禁目标态

目标态本地命令：

```bash
cd pipeline
uv sync --all-packages --all-groups
uv run ruff format --check scheduler/src scheduler/tests migrate
uv run ruff check scheduler/src scheduler/tests migrate
uv run pytest scheduler/tests

cd scheduler
uv run dg check defs --verbose
uv run dg check toml
uv run dg check yaml
uv run dg list defs --json
```

目标态 CI 应至少执行：

```bash
cd pipeline
uv sync --all-packages --all-groups
uv run ruff format --check scheduler/src scheduler/tests migrate
uv run ruff check scheduler/src scheduler/tests migrate
uv run pytest scheduler/tests
cd scheduler && uv run dg check defs --verbose
```

## 退出标准

本计划完成时应满足：

- 标准质量命令不依赖临时 `--with` 安装即可通过。
- Python 版本约束与 workspace 一致。
- README 和 env 文档反映当前真实项目。
- 复杂 asset 的业务流程已下沉到可测试服务层。
- 状态型 OCR pipeline 具备 repository/object-store 边界和状态流测试。
- Dagster definitions 校验、测试、lint、format 成为稳定门禁。
