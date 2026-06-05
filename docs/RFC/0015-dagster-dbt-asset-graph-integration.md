# RFC 0015: Dagster 与 dbt asset graph 集成设计

状态：草案（2026-06-04）

## 摘要

本文档提出 mono-fleur 中 Dagster scheduler 项目与 dbt elt 项目的集成方案：把 dbt
models 表达为 Dagster assets，让 Dagster 能看到从采集、S3 Parquet、ClickHouse raw sync
到 dbt staging/intermediate/marts 的完整 lineage，而不是只在 Dagster 中执行一条
`dbt build` shell 命令。

核心方案：

1. **dbt 项目通过 `dagster-dbt` 接入 Dagster asset graph**：优先使用
   `dagster_dbt.DbtProjectComponent`，把 `pipeline/elt` 的 dbt manifest 编译成 Dagster
   dbt assets。
2. **dbt source lineage 接到 ClickHouse raw assets**：dbt `source('raw', '<dataset>')`
   实际读取 `fleur_raw.<dataset>`，因此它在 Dagster 图中的上游应是
   `clickhouse/raw/<dataset>`，不是更早的 S3 `source/<dataset>`。
3. **generated `sources.yml` 继续由 contract 工具维护**：`pipeline/elt/models/sources.yml`
   是生成物，不手工编辑；应扩展 contract generator，让 dbt source table metadata 能被
   `dagster-dbt` 识别为对应的 Dagster upstream asset key。
4. **调度使用 Dagster asset selection 和 `dbt build`**：dbt materialization 运行
   `dbt build`，让 dbt tests 映射为 Dagster asset checks；调度按 staging/marts 或 source
   影响范围选择资产。
5. **manifest 状态纳入 CI/CD 与本地验证**：本地开发允许自动 `dbt parse`；部署前必须刷新
   component state 或显式生成 manifest，避免运行时才发现 dbt parse 失败。

## 背景

当前项目已经形成如下边界：

```text
Dagster source assets
  -> S3 Parquet source objects
  -> Dagster ClickHouse raw sync assets
  -> ClickHouse fleur_raw tables
  -> dbt source('raw', ...)
  -> dbt staging models
  -> dbt intermediate / marts models
```

相关长期决策和设计：

- ADR 0005：Dagster 负责 ClickHouse raw 同步，dbt 负责建模。
- ADR 0007：dbt staging 只做 source-local、确定性、低业务口径风险的清洗和标准化。
- ADR 0008：新增或重写 staging 前必须先做 raw source profiling。
- ADR 0009：ClickHouse database 固定为 `fleur_raw`、`fleur_staging`、
  `fleur_intermediate`、`fleur_marts` 四层。
- RFC 0014：ClickHouse 四层 database 改造与 raw 迁移验收设计。

## 当前事实

### Dagster / scheduler

当前 Dagster definitions 入口为：

```text
pipeline/scheduler/src/scheduler/definitions.py
pipeline/scheduler/src/scheduler/defs/definitions.py
```

当前已注册：

- S3 source assets：asset key 形如 `source/<dataset>`。
- ClickHouse raw sync assets：asset key 形如 `clickhouse/raw/<dataset>`。
- raw sync jobs：`clickhouse__raw_sync_*_job`。
- source schedules：`jiuyan__industry_ocr_pipeline_schedule`、`ths__limit_up_pool_daily_schedule`
  等。
- default automation condition sensor 和 Slack failure sensor。

当前尚未注册：

- `dagster-dbt` 依赖。
- dbt assets。
- dbt asset checks。
- dbt-specific jobs 或 schedules。

### dbt / elt

当前 dbt 项目路径为：

```text
pipeline/elt/
├── dbt_project.yml
├── profiles.yml
├── models/
│   ├── sources.yml
│   └── staging/
├── macros/
└── scripts/
```

当前事实：

- `pipeline/elt/dbt_project.yml` 已按目录配置：
  - `staging` materialized 为 view，schema 为 `fleur_staging`。
  - `intermediate` materialized 为 view，schema 为 `fleur_intermediate`。
  - `marts` materialized 为 table，schema 为 `fleur_marts`。
- `on-run-start` 已创建 `fleur_staging`、`fleur_intermediate`、`fleur_marts`。
- staging SQL 通过 `source('raw', '<dataset>')` 读取 ClickHouse raw 表。
- `pipeline/elt/models/sources.yml` 由 contract 工具生成，当前已有
  `meta.upstream_raw_asset: clickhouse/raw/<dataset>`，但这还不是 `dagster-dbt`
  默认识别的 upstream asset key 配置。

### Python workspace

当前 `pipeline/pyproject.toml` 已包含：

- `dbt-core`
- `dbt-clickhouse`

当前 `pipeline/scheduler/pyproject.toml` 已包含：

- `dagster==1.13.6`
- `dagster-slack==0.29.6`

但还没有：

- `dagster-dbt`

## 目标

1. 让 Dagster UI 展示完整 lineage：

   ```text
   source/<dataset>
     -> clickhouse/raw/<dataset>
     -> stg_<source>__<dataset>
     -> int_*
     -> mart_*
   ```

2. 让 dbt models 成为 Dagster assets，可按资产选择运行、重跑和观察。
3. 让 dbt tests 成为 Dagster asset checks。
4. 保留 dbt 对模型层的所有权：Dagster 不直接执行模型 SQL，不手写 ClickHouse DDL/DML 建
   staging/intermediate/marts。
5. 保留 contract 工具对 raw source metadata 的所有权：`sources.yml` 仍由 generator 维护。
6. 支持 source-specific 和 layer-specific 调度，例如只跑某个 raw table 下游 staging，
   或每天统一跑 marts。
7. 为后续实施提供明确的阶段、验收命令和禁止模式。

## 非目标

1. 本 RFC 不直接实现代码。
2. 不新增业务 dbt model。
3. 不改变现有 raw sync asset key：`clickhouse/raw/<dataset>` 继续作为 raw 层 Dagster
   asset key。
4. 不让 dbt 负责 raw 装载或 raw table 创建。
5. 不把 `dbt run` 作为调度入口；开发和调度默认使用 `dbt build`。
6. 不引入 dbt Cloud；当前方案面向 colocated dbt Core 项目。
7. 不在本 RFC 内设计 Dagster partitions 到 dbt incremental vars 的完整实现；后续如新增
   incremental marts，再单独设计。

## 设计原则

### Dagster 编排资产，dbt 维护模型

Dagster 应负责：

- 表达 asset graph。
- 触发 dbt materialization。
- 收集 dbt run metadata。
- 展示 dbt tests 对应的 asset checks。
- 调度和重跑指定 asset selection。

dbt 应负责：

- SQL transformation。
- `ref()` / `source()` lineage。
- model materialization。
- dbt tests。
- dbt docs 和模型 YAML metadata。

### dbt source 的上游是 ClickHouse raw asset

dbt staging model 读取的是 ClickHouse raw table：

```sql
from {{ source('raw', '<dataset>') }}
```

因此 Dagster lineage 应为：

```text
clickhouse/raw/<dataset> -> stg_<source>__<dataset>
```

而不是：

```text
source/<dataset> -> stg_<source>__<dataset>
```

S3 source asset 仍然是 raw sync asset 的上游：

```text
source/<dataset> -> clickhouse/raw/<dataset>
```

### 生成物不手工维护

`pipeline/elt/models/sources.yml` 是 contract 生成物。若需要新增
`meta.dagster.asset_key`，应修改 contract adapter 的生成逻辑，而不是手工编辑
`sources.yml`。

### 调度用资产选择，不用裸命令

不要在 Dagster schedule 中直接执行：

```text
cd pipeline && uv run dbt build --project-dir elt --profiles-dir elt
```

应让 dbt models 先成为 Dagster assets，然后通过 Dagster asset selection 运行：

- layer selection：staging、intermediate、marts。
- source selection：某个 source 的 raw 下游。
- dependency selection：某个 raw asset 的 downstream。

## 推荐方案

### 依赖与运行环境

在 scheduler 项目中加入与当前 Dagster 版本匹配的 integration package：

```text
dagster-dbt==0.29.6
```

理由：

- 当前 scheduler 使用 `dagster==1.13.6`。
- 当前 integration package `dagster-slack` 为 `0.29.6`。
- Dagster integration packages 通常按 `0.x` 版本与 Dagster core `1.x` 版本对应。

dbt CLI 运行依赖当前已在 workspace 层存在：

- `dbt-core`
- `dbt-clickhouse`

实施时仍需确认 deployed scheduler runtime 能访问这些依赖，而不只是本地
`pipeline/` workspace 能访问。

### dbt assets 载入方式

优先使用 `dagster_dbt.DbtProjectComponent`。

建议实施流程：

```bash
cd pipeline/scheduler
uv run dg list components --json
uv run dg scaffold defs dagster_dbt.DbtProjectComponent dbt
```

实际 scaffold 参数以 `dg utils inspect-component` 输出为准。组件应指向 colocated dbt
project：

```text
../elt
```

component 应配置：

- `project`: `pipeline/elt`
- `profiles_dir`: `pipeline/elt`
- `cli_args`: `build`
- `translator_settings.enable_source_tests_as_checks`: 按需要开启
- `include_metadata`: 后续可开启 `row_count` 和 `column_metadata`

若 component 与当前 Python-only `Definitions` 聚合方式冲突，允许以小步方式先用
`@dbt_assets` Pythonic integration 接入，但这只能作为过渡。长期目标仍是 component 化，
因为当前 scheduler 已配置 `registry_modules = ["scheduler.components.*"]`，具备组件注册入口。

### dbt source 到 Dagster raw asset 的映射

生成后的 dbt source table metadata 应让 `dagster-dbt` 在 manifest 中得到如下语义：

```yaml
sources:
  - name: raw
    tables:
      - name: ths__limit_up_pool_compacted
        meta:
          dagster:
            asset_key: ["clickhouse", "raw", "ths__limit_up_pool_compacted"]
```

如果继续保留当前 `config.meta.upstream_raw_asset`，也可以保留为 contract/data dictionary
元数据，但不能只依赖它表达 Dagster lineage。实施时应以 dbt parse 后的
`target/manifest.json` 为准，确认 source node 中存在 `dagster.asset_key` 可被
`dagster-dbt` 读取。

建议 contract generator 同时输出：

```yaml
config:
  meta:
    upstream_raw_asset: clickhouse/raw/ths__limit_up_pool_compacted
    dagster:
      asset_key: ["clickhouse", "raw", "ths__limit_up_pool_compacted"]
```

如 dbt/dagster-dbt 对 source-level `config.meta` 与 top-level `meta` 的解析存在差异，
以 manifest 验证结果决定最终 YAML 形态。

### dbt asset key 与 group

初始阶段建议保守处理：

- dbt model asset key 默认使用 dbt model name，例如 `stg_ths__limit_up_pool_compacted`。
- group 按 dbt layer 或固定 `dbt` group 组织。
- 不在第一阶段强行给 dbt model asset key 增加 `dbt/staging/...` 前缀。

理由：

- 避免过早引入 custom translator。
- 先验证 lineage、运行、asset checks 和调度闭环。
- 后续如果 Dagster UI 中资产数量变多，再通过 translation 或 custom component 统一
  key/group/tag。

建议 tags/kinds：

- staging model：
  - kind: `dbt`, `clickhouse`
  - tags: `layer=staging`
- intermediate model：
  - kind: `dbt`, `clickhouse`
  - tags: `layer=intermediate`
- marts model：
  - kind: `dbt`, `clickhouse`
  - tags: `layer=marts`

### Jobs 与 schedules

建议先定义少量稳定 job：

```text
dbt__staging_job
dbt__marts_job
dbt__daily_build_job
```

`dbt__staging_job`：

- selection：dbt staging group/path/tag。
- 用途：raw sync 后刷新 staging views 并运行 staging tests。

`dbt__marts_job`：

- selection：marts 及其必要上游。
- 用途：生产数据产品刷新。

`dbt__daily_build_job`：

- selection：日常需要刷新的 dbt assets。
- 用途：统一日调度，避免为每个 staging model 建 schedule。

调度策略：

1. 初期使用固定时间 schedule，例如 raw sync 后 30-60 分钟运行 `dbt__daily_build_job`。
2. 当 lineage 验证稳定后，可给 dbt assets 增加 dependency-aware automation condition，
   让 raw asset 更新后自动触发对应下游。
3. 对 OCR 等长尾异步来源，不应让 dbt schedule 假设 OCR 一定在固定时间完成；需要么使用
   downstream asset selection 手动/自动触发，要么用 asset-aware automation。

### dbt tests 与 asset checks

dbt model tests 应作为 Dagster asset checks 展示。

建议：

- model-level tests 默认纳入。
- source tests 是否纳入由第一阶段验证决定；如果 source tests 对 generated raw sources 有价值，
  开启 `enable_source_tests_as_checks`。
- 对依赖多个模型的 singular tests，必须在 dbt test config 中显式声明对应目标 model，
  避免只产生 observation 而不是 asset check。

### manifest state 管理

`DbtProjectComponent` 是 state-backed component，状态核心是 dbt `manifest.json`。

建议：

- 本地开发：允许 `prepare_if_dev` 自动 parse。
- CI/CD：部署前运行 component state refresh。
- 验收：`dg check defs` 必须能加载 dbt assets，不允许运行时才 parse 失败。

## 实施阶段

### 阶段 1：依赖与最小 dbt asset 接入

目标：

- scheduler runtime 安装 `dagster-dbt`。
- `pipeline/elt` 被 Dagster 识别为 dbt project。
- `dg list defs --json` 能看到 dbt model assets。

主要改动：

- `pipeline/scheduler/pyproject.toml`
- `pipeline/uv.lock`
- 新增 dbt component defs 或 Pythonic `@dbt_assets` 过渡模块
- `pipeline/scheduler/src/scheduler/defs/definitions.py`

验收：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

### 阶段 2：raw source lineage 映射

目标：

- dbt `source('raw', '<dataset>')` 在 Dagster 图中依赖 `clickhouse/raw/<dataset>`。
- 不手工编辑 generated `sources.yml`。

主要改动：

- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- contract generator tests
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

- 任一 staging model，例如 `stg_ths__limit_up_pool_compacted`，在 Dagster asset graph 中的
  upstream 包含 `clickhouse/raw/ths__limit_up_pool_compacted`。

### 阶段 3：dbt jobs、schedules 与 checks

目标：

- 增加少量 dbt asset jobs。
- 增加日常 dbt schedule。
- dbt tests 显示为 asset checks。

主要改动：

- `pipeline/scheduler/src/scheduler/defs/dbt/`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- scheduler unit/integration tests

验收：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select staging
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

人工验收点：

- Dagster UI 中可按 dbt layer/job 运行。
- dbt test 失败时表现为 asset check failure，而不是只有 opaque op failure。

### 阶段 4：调度策略收敛

目标：

- 根据实际运行情况决定固定 schedule 与 automation condition 的边界。
- 明确 OCR、snapshot、year partition raw assets 对 dbt 层的触发策略。

主要改动：

- schedules 或 automation condition 配置。
- `docs/jobs/reports/` 中记录首次集成运行结果。
- 如形成固定操作流程，补充 `docs/skills/` runbook。

验收：

```bash
cd pipeline/scheduler
uv run dg check defs
```

运行报告应记录：

- 运行时间。
- 运行 job。
- 涉及 dbt selection。
- 成功/失败模型与 checks。
- 上游 raw assets 是否已 materialized。

## 禁止模式

1. 不在 schedule 中裸跑 `uv run dbt build` 并绕过 Dagster dbt assets。
2. 不手工编辑 generated `pipeline/elt/models/sources.yml` 来修 lineage。
3. 不让 dbt model 直接依赖 S3 `source/<dataset>` asset，除非模型实际读取 S3。
4. 不用 `dbt run` 作为日常调度入口。
5. 不为每个 staging model 单独创建 schedule。
6. 不让 Dagster 直接执行 dbt model 的 compiled SQL。
7. 不在第一阶段引入复杂 custom translator，除非 default/component translation 无法表达
   `clickhouse/raw/<dataset>` lineage。

## 风险与缓解

### 风险 1：component 接入与当前 Python Definitions 聚合冲突

缓解：

- 先用 `dg scaffold defs` 和 `dg check defs` 验证。
- 如果冲突，短期使用 Pythonic `@dbt_assets` 接入，长期再收敛到 component。

### 风险 2：`config.meta` 未被 `dagster-dbt` 按预期读取

缓解：

- 以 `dbt parse` 生成的 `target/manifest.json` 为事实源。
- 若 manifest 中 source node 没有可识别的 `dagster.asset_key`，调整 generator 输出到
  top-level `meta`。
- 为 generator 增加测试，避免 sources.yml 再次漂移。

### 风险 3：固定 schedule 与 raw 完成时间错位

缓解：

- 初期 schedule 留足 raw sync 时间窗口。
- 对长耗时或异步 source 使用 asset-aware automation 或手动 downstream materialization。
- 在运行报告中记录 raw/dbt 运行先后和失败样本。

### 风险 4：dbt asset 数量增加后 Dagster UI 可读性下降

缓解：

- 初期按 layer group。
- 后续通过 component translation 或 custom component 统一 key、group、tags。

## 验收标准

实施完成后必须满足：

1. `dg list defs --json` 能列出 dbt model assets。
2. `dg check defs` 通过。
3. `dbt parse` 通过。
4. `dbt build --select staging` 通过。
5. 任一 staging model 的 Dagster upstream 为对应 `clickhouse/raw/<dataset>`。
6. dbt model tests 在 Dagster 中显示为 asset checks。
7. generated `sources.yml` 不需要手工补丁即可保留 `dagster.asset_key` metadata。
8. 至少有一个 `docs/jobs/reports/` 运行报告记录首次 dbt asset materialization。

## 最小验证命令

文档落地后，实施阶段至少运行：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select staging
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q

cd scheduler
uv run dg check defs
uv run dg list defs --json
```

文档-only 变更至少运行：

```bash
git diff --check
```

## 开放问题

1. dbt source metadata 最终应使用 top-level `meta`，还是当前 generated YAML 中的
   `config.meta` 即可被 `dagster-dbt` 稳定识别？
2. dbt model asset key 是否需要统一加 `dbt/` 前缀，还是先保留 model name？
3. source tests 是否全部纳入 Dagster asset checks，还是只纳入高价值 raw source tests？
4. daily dbt schedule 是固定时间触发，还是第一版就启用 dependency-aware automation？
5. OCR snapshot 类资产是否需要独立 dbt downstream job，避免和普通日行情 refresh 混在一起？
