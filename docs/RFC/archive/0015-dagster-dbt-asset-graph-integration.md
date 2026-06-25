# RFC 0015: Dagster 调度 dbt asset graph 集成设计

状态：Archived（2026-06-25；归档前状态：草案 / 第一版已实施（2026-06-07））

## 摘要

本文档设计 mono-fleur 中 `pipeline/scheduler` 与 `pipeline/elt` 的集成方案：将现有 dbt
项目的 sources、models 和 tests 接入 Dagster asset graph，由 Dagster 负责调度 dbt
资产，由 dbt 继续负责 staging、intermediate、marts 三层模型的 SQL transformation 和
tests。

核心结论：

1. **使用 `dagster-dbt` 接入 dbt Core 项目。** 第一版采用官方 component 路线，并通过
   `DbtProjectComponent` 子类只定制 model asset key、group 和 tags；如后续组件接入与
   `Definitions` 聚合发生冲突，`@dbt_assets` + `DbtCliResource` 仅作为临时退路。
2. **Dagster 不直接执行 dbt compiled SQL。** Dagster materialize dbt assets 时只调用
   dbt CLI，默认命令为 `dbt build`，让 model materialization 与 data tests 在同一次运行
   中完成。
3. **dbt source 上游应连接到 ClickHouse raw assets。** dbt `source('raw', '<dataset>')`
   实际读取 `fleur_raw.<dataset>`，其 Dagster 上游应是 `clickhouse/raw/<dataset>`，不是
   更早的 S3 `source/<dataset>`。
4. **`sources.yml` 仍由 contract generator 维护。** 不手工编辑 generated
   `pipeline/elt/models/sources.yml`。为 Dagster lineage 需要新增的
   `meta.dagster.asset_key` 必须由 `pipeline/contract_tools` 生成。
5. **调度使用 asset selection。** dbt 日常运行以少量 layer/source 级 Dagster jobs 和
   schedules 表达，不在 schedule 中裸跑 `uv run dbt build`。

## 背景

当前项目的长期边界已经明确：

```text
Dagster source assets
  -> S3 Parquet objects
  -> Dagster ClickHouse raw sync assets
  -> ClickHouse fleur_raw tables
  -> dbt source('raw', ...)
  -> dbt staging models
  -> dbt intermediate models
  -> dbt marts models
```

相关决策和设计依据：

- ADR 0005：Dagster 负责 ClickHouse raw 同步，dbt 负责建模。
- ADR 0007：dbt staging 只做 source-local、确定性、低业务口径风险的清洗和标准化。
- ADR 0008：新增或重写 staging 前必须先做 raw source profiling。
- ADR 0009：ClickHouse database 固定为 `fleur_raw`、`fleur_staging`、
  `fleur_intermediate`、`fleur_marts` 四层。
- RFC 0014：ClickHouse 四层 database 改造与 raw 迁移验收设计。

本 RFC 的目标不是改变上述边界，而是让 Dagster UI、调度、重跑和告警能覆盖 dbt 模型层。

## 当前事实

### Dagster / scheduler

当前 definitions 入口：

```text
pipeline/scheduler/src/scheduler/definitions.py
pipeline/scheduler/src/scheduler/defs/definitions.py
```

当前装配方式：

- `defs()` 从 `SOURCE_BUNDLES` 聚合 source assets、jobs、schedules。
- `CLICKHOUSE_RAW_ASSETS` 和 `CLICKHOUSE_RAW_JOBS` 已独立注册。
- `CLICKHOUSE_RAW_JOBS` 已包含：
  - `clickhouse__raw_sync_all_job`
  - `clickhouse__raw_sync_snapshot_job`
  - `clickhouse__raw_sync_baostock_job`
  - `clickhouse__raw_sync_eastmoney_job`
  - `clickhouse__raw_sync_jiuyan_market_event_job`
  - `clickhouse__raw_sync_ths_market_event_job`
- 当前 resources 包含 S3、BaoStock、HTTP、ClickHouse、Slack 等资源。
- dbt assets 通过 `pipeline/scheduler/src/scheduler/defs/dbt/defs.yaml` 中的 component
  instance 注册。
- dbt jobs 和 schedule 通过 `pipeline/scheduler/src/scheduler/defs/dbt_jobs.py` 注册。

当前依赖：

- `pipeline/scheduler/pyproject.toml` 包含 `dagster==1.13.6`、`dagster-slack==0.29.6`
  和 `dagster-dbt==0.29.6`。
- 当前运行 `uv run dg list components --json` 能看到 `dagster_dbt.DbtProjectComponent`。

### dbt / elt

当前 dbt 项目路径：

```text
pipeline/elt/
├── dbt_project.yml
├── profiles.yml
├── models/
│   ├── sources.yml
│   ├── staging/
│   ├── intermediate/
│   └── marts/
├── macros/
└── tests/
```

当前 dbt manifest 事实：

- model 数量：25。
- test 数量：203。
- source 数量：16。
- `dbt_project.yml` 已配置：
  - `staging` -> `fleur_staging`
  - `intermediate` -> `fleur_intermediate`
  - `marts` -> `fleur_marts`
- `profiles.yml` 使用 ClickHouse adapter。
- workspace 根依赖已包含 `dbt-core>=1.11.11` 与 `dbt-clickhouse>=1.10.0`。

当前 generated `sources.yml` 已包含：

```yaml
sources:
  - name: raw
    schema: fleur_raw
    tables:
      - name: ths__limit_up_pool_compacted
        config:
          meta:
            upstream_raw_asset: clickhouse/raw/ths__limit_up_pool_compacted
            clickhouse_raw_table: fleur_raw.ths__limit_up_pool_compacted
```

但当前还没有 `dagster-dbt` 默认可识别的：

```yaml
meta:
  dagster:
    asset_key: ["clickhouse", "raw", "ths__limit_up_pool_compacted"]
```

因此仅靠现状还不能保证 dbt source 在 Dagster 图中连接到现有
`clickhouse/raw/<dataset>` assets。

## 目标

1. Dagster asset graph 展示完整 lineage：

   ```text
   source/<dataset>
     -> clickhouse/raw/<dataset>
     -> stg_<source>__<dataset>
     -> int_*
     -> mart_*
   ```

2. dbt models 作为 Dagster assets 被列出、调度、重跑和观察。
3. dbt model tests 作为 Dagster asset checks 展示。
4. Dagster 调度 dbt assets 时使用 `dbt build`，而不是 `dbt run`。
5. 支持 layer 级调度，例如 staging、intermediate、marts。
6. 支持 source 级调度，例如某个 ClickHouse raw asset 更新后重跑对应下游 dbt assets。
7. 保留 dbt 对模型层 SQL、tests、docs、YAML metadata 的所有权。
8. 保留 contract generator 对 raw source metadata 和 `sources.yml` 的所有权。

## 非目标

1. 本 RFC 不直接实现代码。
2. 不新增业务 dbt model。
3. 不改变 ClickHouse raw sync 资产 key：`clickhouse/raw/<dataset>`。
4. 不让 dbt 负责 raw table 创建、raw 装载或 S3 -> ClickHouse 同步。
5. 不引入 dbt Cloud；第一版只面向 colocated dbt Core 项目。
6. 不在第一版实现 dbt incremental partition vars 与 Dagster partitions 的完整映射。
7. 不为每个 dbt model 单独创建 schedule。

## 设计原则

### Dagster 编排，dbt 执行建模

Dagster 负责：

- asset graph。
- asset selection。
- schedules、sensors、automation conditions。
- dbt run/check 事件收集。
- asset materialization metadata 和失败告警。

dbt 负责：

- SQL transformation。
- `ref()` / `source()` lineage。
- materialization config。
- data tests。
- dbt docs 和模型 YAML。

### source lineage 连接到实际读取对象

dbt staging model 的 SQL 读取的是：

```sql
from {{ source('raw', '<dataset>') }}
```

这个 source 指向 ClickHouse `fleur_raw.<dataset>`，因此 Dagster lineage 必须是：

```text
clickhouse/raw/<dataset> -> stg_<source>__<dataset>
```

S3 source asset 仍然是 raw sync asset 的上游：

```text
source/<dataset> -> clickhouse/raw/<dataset>
```

### 生成物不手工维护

`pipeline/elt/models/sources.yml` 是 contract 生成物。为 Dagster source lineage 增加的
metadata 必须由 `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py` 生成，并由
contract-tools tests 保护。

### 调度围绕 asset selection

Dagster schedule 不应直接写 shell 命令：

```bash
uv run dbt build --project-dir elt --profiles-dir elt
```

而应 materialize Dagster dbt assets。dbt selection 只用于定义 dbt asset 子集或
component/job 内部行为。

## 目标架构

```text
pipeline/scheduler
  Dagster Definitions
    ├── source assets
    ├── clickhouse/raw assets
    ├── dbt assets from pipeline/elt manifest
    ├── dbt asset checks from dbt tests
    ├── raw sync jobs
    └── dbt jobs / schedules

pipeline/elt
  dbt Core project
    ├── source('raw', ...)
    ├── staging models
    ├── intermediate models
    ├── marts models
    └── tests
```

运行时：

```text
Dagster materialize dbt asset selection
  -> dagster-dbt invokes dbt build
  -> dbt builds selected models and runs selected tests
  -> dagster-dbt streams materializations and check results
  -> Slack failure sensor handles asset/check failures
```

## 推荐方案

### 依赖

在 `pipeline/scheduler/pyproject.toml` 增加：

```text
dagster-dbt==0.29.6
```

理由：

- 当前 `dagster==1.13.6`。
- 当前 `dagster-slack==0.29.6`。
- Dagster integration packages 通常以 `0.29.x` 对齐 Dagster core `1.13.x`。

同时必须确认 scheduler runtime 能 import：

- `dbt-core`
- `dbt-clickhouse`

当前它们在 workspace 根项目 `pipeline/pyproject.toml` 中；实施时需要用 `uv run` 和
部署镜像验证 scheduler 代码位置也能访问 dbt CLI 与 ClickHouse adapter。

### 接入路线

首选路线：`DbtProjectComponent`。

前置检查：

```bash
cd pipeline/scheduler
uv run dg list components --json
```

安装 `dagster-dbt` 后应能看到：

```text
dagster_dbt.DbtProjectComponent
```

脚手架建议：

```bash
cd pipeline/scheduler
uv run dg scaffold defs dagster_dbt.DbtProjectComponent dbt --project-path ../elt
```

实际参数以以下命令输出为准：

```bash
uv run dg utils inspect-component dagster_dbt.DbtProjectComponent --scaffold-params-schema
uv run dg utils inspect-component dagster_dbt.DbtProjectComponent --defs-yaml-schema
```

第一版落地配置表达：

```yaml
type: scheduler.components.fleur_dbt.FleurDbtProjectComponent

attributes:
  project:
    project_dir: "{{ context.project_root }}/../elt"
    profiles_dir: "{{ context.project_root }}/../elt"
  cli_args:
    - build
  translation_settings:
    enable_asset_checks: true
    enable_source_tests_as_checks: false
```

`enable_source_tests_as_checks` 第一版建议为 `false`，因为当前 source tests 的价值和成本
尚未单独评估；model tests 默认会作为 asset checks。

由于项目需要保留 model asset key 为 `stg_*`、`int_*`、`mart_*`，并让 dbt source 继续通过
`meta.dagster.asset_key` 连接到 `clickhouse/raw/<dataset>`，第一版用
`FleurDbtProjectComponent.get_asset_spec()` 只覆盖 dbt model 的 key、group 和 tags，不覆盖
source asset key。

过渡路线：`@dbt_assets`。

如果 `DbtProjectComponent` 与当前 `@dg.definitions` 聚合方式或组件目录结构冲突，可以先在
`pipeline/scheduler/src/scheduler/defs/dbt/` 使用 Pythonic integration：

```python
from pathlib import Path

import dagster as dg
from dagster_dbt import DbtCliResource, DbtProject, dbt_assets

DBT_PROJECT_DIR = Path(__file__).parents[5] / "elt"
dbt_project = DbtProject(project_dir=DBT_PROJECT_DIR, profiles_dir=DBT_PROJECT_DIR)
dbt_project.prepare_if_dev()


@dbt_assets(manifest=dbt_project.manifest_path)
def dbt_assets_def(context: dg.AssetExecutionContext, dbt: DbtCliResource):
    yield from dbt.cli(["build"], context=context).stream()


dbt_resource = DbtCliResource(
    project_dir=DBT_PROJECT_DIR,
    profiles_dir=DBT_PROJECT_DIR,
)
```

过渡路线必须被记录为临时方案；后续收敛回 component，避免 scheduler 中长期存在一套
自定义 dbt integration。

### dbt source asset key 映射

contract generator 应为每个 raw-enabled source table 生成：

```yaml
config:
  meta:
    upstream_raw_asset: clickhouse/raw/ths__limit_up_pool_compacted
    clickhouse_raw_table: fleur_raw.ths__limit_up_pool_compacted
    dagster:
      asset_key:
        - clickhouse
        - raw
        - ths__limit_up_pool_compacted
```

如果 `dagster-dbt` 对 `config.meta` 与 top-level `meta` 的识别存在差异，以
`pipeline/elt/target/manifest.json` 为事实源验证。验收必须检查 source node 中最终存在
可被 `dagster-dbt` 读取的 `dagster.asset_key`。

不得在 staging SQL 中加入无意义 Jinja `depends_on` 来修正普通 `source()` lineage。当前
staging model 已实际读取 `source('raw', ...)`，正确位置是 source metadata。

### dbt model asset key、group 和 tags

第一版采用保守映射：

- asset key：使用 dbt model name，例如 `stg_ths__limit_up_pool_compacted`。
- group：按 dbt layer 分组。
  - `dbt_staging`
  - `dbt_intermediate`
  - `dbt_marts`
- kind：`dbt`、`clickhouse`。
- tags：
  - `layer=staging|intermediate|marts`
  - `owner=dbt`
  - `storage=clickhouse`

不在第一版给 dbt model asset key 增加 `dbt/` 前缀。理由：

- 避免第一版引入 custom translator 复杂度。
- 先验证 lineage、checks、materialization 和 schedules。
- 如后续 UI 中 model 资产过多，再通过 component translation 或 subclass 统一 key 策略。

### dbt jobs

第一版定义少量稳定 job：

```text
dbt__staging_build_job
dbt__marts_build_job
dbt__daily_build_job
```

`dbt__staging_build_job`：

- 选择 staging layer。
- 用于 raw sync 后刷新 staging views 并运行 staging tests。
- 第一版不自动 upstream raw assets，避免每次跑 dbt 都重跑 raw sync。

`dbt__marts_build_job`：

- 选择 marts 以及必要上游 dbt assets。
- 用于刷新数据产品表。

`dbt__daily_build_job`：

- 选择日常需要刷新的 dbt assets。
- 第一版可等同于所有 dbt models。
- 后续按 tags 或 config 收敛。

### schedules 与 automation

第一版采用固定 schedule：

- raw sync jobs 仍按现有 source schedules 运行。
- `dbt__daily_build_job` 在主要 raw sync 时间窗口后运行，预留 30-60 分钟 buffer。
- marts 刷新可晚于 staging/intermediate，避免重 raw source 导致资源争用。

第二阶段再评估 asset-aware automation：

- raw asset materialization 后自动触发对应 staging downstream。
- 对 OCR、snapshot、year partition 资产分别评估触发策略。

不建议第一版直接启用复杂 automation condition，原因是当前 raw assets 有 snapshot、year
partition、OCR 异步等不同完成语义，固定 schedule 更容易观察和回滚。

### dbt tests 与 asset checks

`dagster-dbt` 会将 dbt model tests 作为 asset checks 加载。

规则：

- model-level generic tests 默认纳入。
- model-level singular tests 如果依赖多个模型，必须在 dbt test config 中声明目标 model，
  否则可能只产生 observation 而不是 check result。
- source tests 第一版不启用为 checks；后续单独评估 source test 的价值和运行成本。

### manifest state

`DbtProjectComponent` 是 state-backed component，核心状态是 dbt `manifest.json`。

本地开发：

- 允许 `prepare_if_dev` 或 component dev mode 自动 `dbt parse`。

CI/CD：

- 部署前刷新 defs state 或显式生成 manifest。
- `dg check defs` 必须能加载 dbt assets。
- 不允许生产运行时才发现 dbt parse 失败。

### 环境变量与 profiles

dbt ClickHouse profile 继续使用 `pipeline/elt/profiles.yml` 中的 env vars：

- `CLICKHOUSE_HOST`
- `CLICKHOUSE_PORT`
- `CLICKHOUSE_USER`
- `CLICKHOUSE_PASSWORD`
- `CLICKHOUSE_DBT_SCHEMA`
- `CLICKHOUSE_CONNECT_TIMEOUT_SECONDS`
- `CLICKHOUSE_QUERY_TIMEOUT_SECONDS`

scheduler runtime 必须共享这些环境变量。不要在 dbt integration 代码中硬编码 ClickHouse
连接信息。

## 实施阶段

### 阶段 1：依赖与最小 dbt assets

目标：

- 安装 `dagster-dbt`。
- Dagster 能加载 dbt manifest。
- `dg list defs --json` 能看到 dbt model assets。

主要改动：

- `pipeline/scheduler/pyproject.toml`
- `pipeline/uv.lock`
- `pipeline/scheduler/src/scheduler/defs/dbt/`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`

验收：

```bash
cd pipeline
uv sync --all-packages --all-groups
uv run dbt parse --project-dir elt --profiles-dir elt

cd scheduler
uv run dg list components --json
uv run dg check defs
uv run dg list defs --json
```

### 阶段 2：source lineage 映射

目标：

- dbt source nodes 连接到 `clickhouse/raw/<dataset>` assets。
- generated `sources.yml` 不需要手工补丁。

主要改动：

- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- `pipeline/contract_tools/tests/`
- generated `pipeline/elt/models/sources.yml`

验收：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run dbt parse --project-dir elt --profiles-dir elt

cd scheduler
uv run dg check defs
uv run dg list defs --json
```

人工验收点：

- `stg_ths__limit_up_pool_compacted` 的 upstream 包含
  `clickhouse/raw/ths__limit_up_pool_compacted`。

### 阶段 3：dbt jobs、schedules 与 checks

目标：

- 增加 dbt asset jobs。
- 增加固定时间 dbt schedule。
- dbt model tests 显示为 asset checks。

主要改动：

- `pipeline/scheduler/src/scheduler/defs/dbt/`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/tests/integration/`

验收：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select staging

cd scheduler
uv run dg check defs
uv run dg list defs --json
```

人工验收点：

- Dagster UI 中可 materialize dbt staging/marts jobs。
- dbt test failure 表现为 asset check failure，而不是 opaque op failure。

### 阶段 4：调度策略收敛

目标：

- 评估固定 schedule 是否足够。
- 记录首次 dbt asset materialization 运行报告。
- 决定是否引入 asset-aware automation。

主要改动：

- schedules 或 automation conditions。
- `docs/jobs/reports/` 运行报告。
- 如形成固定操作流程，补充 `docs/skills/` runbook。

验收：

```bash
cd pipeline/scheduler
uv run dg check defs
```

运行报告应记录：

- 运行时间。
- 运行 job。
- dbt selection。
- 成功/失败 models。
- 成功/失败 checks。
- 上游 raw assets 状态。

## 禁止模式

1. 不在 schedule 中裸跑 `uv run dbt build` 并绕过 Dagster dbt assets。
2. 不手工编辑 generated `pipeline/elt/models/sources.yml` 修 lineage。
3. 不让 dbt model 直接依赖 S3 `source/<dataset>` asset，除非模型实际读取 S3。
4. 不用 `dbt run` 作为日常调度入口。
5. 不为每个 dbt model 单独创建 schedule。
6. 不让 Dagster 直接执行 dbt model 的 compiled SQL。
7. 不在第一阶段实现复杂 incremental partition vars。
8. 不在 integration 代码中硬编码 ClickHouse credentials。

## 风险与缓解

### 风险 1：`DbtProjectComponent` 与当前 definitions 聚合冲突

当前状态：第一版已通过 `dg check defs`，未触发该冲突。

缓解：

- 先用 `dg scaffold defs` 和 `dg check defs` 验证。
- 如果冲突，短期使用 `@dbt_assets`，但保留明确退出条件。

退出条件：

- `dagster_dbt.DbtProjectComponent` 能稳定通过 `dg check defs`。
- component state refresh 能在 CI 中稳定执行。

### 风险 2：source metadata 位置不被 `dagster-dbt` 识别

缓解：

- 以 `target/manifest.json` 为事实源。
- 同时测试 `config.meta.dagster.asset_key` 和 top-level `meta.dagster.asset_key`。
- 为 generator 增加 fixture 测试。

### 风险 3：固定 schedule 与 raw 完成时间错位

缓解：

- 第一版留足 buffer。
- 运行报告记录 raw/dbt 时间线。
- 后续用 asset-aware automation 替换部分固定 schedule。

### 风险 4：dbt build 成本过高

缓解：

- jobs 按 layer/source 选择资产。
- 日常运行优先选择必要 dbt assets。
- 重型 marts 可独立 schedule。

### 风险 5：部署环境缺少 dbt adapter

缓解：

- `dg check defs` 前先运行 `dbt parse`。
- 部署镜像中验证 `dbt --version` 显示 `clickhouse` adapter。

## 验收标准

实施完成后必须满足：

1. `uv run dg list components --json` 能看到 `dagster_dbt.DbtProjectComponent`。
2. `uv run dg list defs --json` 能列出 dbt model assets。
3. `uv run dg check defs` 通过。
4. `uv run dbt parse --project-dir elt --profiles-dir elt` 通过。
5. `uv run dbt build --project-dir elt --profiles-dir elt --select staging` 通过。
6. 至少一个 staging model 的 Dagster upstream 是对应 `clickhouse/raw/<dataset>`。
7. dbt model tests 在 Dagster 中显示为 asset checks。
8. generated `sources.yml` 不需要手工补丁即可保留 Dagster asset key metadata。
9. `docs/jobs/reports/` 有首次 dbt asset materialization 运行报告。

## 最小命令集

文档-only 变更：

```bash
git diff --check
```

实施阶段：

```bash
cd pipeline
uv sync --all-packages --all-groups
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select staging
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests scheduler/tests -q

cd scheduler
uv run dg list components --json
uv run dg check defs
uv run dg list defs --json
```

## 开放问题

1. `dagster-dbt` 在当前版本中是否稳定读取 `config.meta.dagster.asset_key`，还是必须输出
   top-level `meta.dagster.asset_key`？
2. dbt asset group 第一版采用 `dbt_staging/dbt_intermediate/dbt_marts`，还是使用固定
   `dbt` group 加 `layer` tag？
3. source tests 是否有足够价值纳入 Dagster asset checks？
4. `dbt__daily_build_job` 第一版应覆盖全部 models，还是只覆盖 marts 及其上游？
5. OCR snapshot 类 source 是否需要独立 dbt downstream job？
