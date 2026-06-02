# Plan 0021: Contract scope raw-layer cleanup

日期：2026-06-01

状态：Implemented（2026-06-01）

## 1. 背景

Plan 0018 和 RFC 0010 把 `pipeline/contracts` 设计成覆盖 `source -> parquet -> clickhouse_raw -> dbt_staging` 的字段事实源。当前实现也沿用了这个方向：

- `pipeline/contracts/datasets/*.yml` 包含 `dbt_staging` 块。
- `pipeline/contract_tools/src/fleur_contracts/schema.py` 定义并校验 `DbtStagingSpec` / `DbtStagingField`。
- `pipeline/contract_tools/src/fleur_contracts/generate.py` 生成 `pipeline/elt/models/staging/staging.yml`。
- `pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py` 在数据字典中展示 `stg 字段`。
- `pipeline/contract_tools/tests/test_contract_registry.py` 把 active staging 字段、glossary 和 raw 字段引用作为 contract 测试内容。
- `AGENTS.md`、`docs/skills/fleur-contract-data-dictionary/SKILL.md`、RFC/plan 文档仍写着 `staging.yml` 由 contract 生成或校验。

这个范围过大。ADR 0005 已明确：Dagster 负责 ClickHouse raw 同步，dbt 负责 staging/marts 建模。`dbt_staging` 字段事实继续留在 contract registry，会让 dbt staging 的命名、测试、字段文档和轻清洗被 raw contract 绑定，削弱 dbt 项目的建模自治。

本计划一刀切收缩 contract scope：`pipeline/contracts` 和 `contract_tools` 只管理 raw ingestion contract，不再管理 stg 层字段事实。

## 2. 决策

从本计划完成后开始：

```text
source payload
  -> S3 Parquet source asset
  -> ClickHouse raw table
  -> dbt source()
```

属于 `pipeline/contracts` / `contract_tools` 的治理范围。

```text
dbt staging model
  -> dbt intermediate/mart model
```

属于 `pipeline/elt` 的 dbt 项目范围。

具体边界：

| 层 | 所有者 | 说明 |
|----|--------|------|
| `source.fields` | contract | 外源字段、外源类型、供应商语境中文描述 |
| `parquet.fields` | contract | S3 Parquet 发布 schema |
| `clickhouse_raw.fields` | contract | ClickHouse raw schema、分区、排序键、raw asset key |
| dbt `sources.yml` | contract 生成 | 只声明 ClickHouse raw source table 和 contract metadata |
| dbt `staging.yml` | dbt 项目手写维护 | 字段描述、tests、config、meta 由 dbt owning |
| `stg_*.sql` | dbt 项目手写维护 | canonical 命名、轻清洗、类型收敛、派生字段 |
| `docs/references/data_dict/*.md` | contract 生成 | 只展示外源、Parquet、ClickHouse raw 字段链路 |
| `glossary/fields.yml` | contract 参考资料 | 可保留为 raw 字段语义参考，不再强制约束 stg 字段 |

## 3. 目标

1. 从所有 dataset contract 中删除 `dbt_staging` 块。
2. 从 contract schema、registry validation、generator 和 data dictionary renderer 中删除 stg 字段事实处理。
3. `fleur-contracts generate` 停止写入 `pipeline/elt/models/staging/staging.yml`。
4. 当前 `pipeline/elt/models/staging/staging.yml` 变为 dbt-owned 手写文件，不再被 contract generate/check 覆盖。
5. 更新文档和 skill 路由，避免后续 agent 继续把 stg 字段写回 contract。
6. 保持 raw ingestion contract 校验能力：contract validate、generated `sources.yml`、generated data_dict、Parquet/ClickHouse schema validation 不回退。

## 4. 非目标

1. 不删除 dbt staging models。
2. 不重写 `stg_*.sql` 的字段命名、类型转换或业务逻辑。
3. 不要求本计划同时补齐 dbt staging 字段描述质量治理；该工作如有必要应另开 dbt 文档计划。
4. 不把 dbt staging SQL 模板化。
5. 不删除 `pipeline/contracts/glossary/fields.yml`，除非后续确认 raw/data_dict 不再需要这些中文语义参考。
6. 不改变 Dagster raw sync 的运行协议、ClickHouse 表结构或历史数据。

## 5. 改动范围

### 5.1 Contract 数据

目标文件：

- `pipeline/contracts/datasets/*.yml`

操作：

- 删除每个 dataset 的顶层 `dbt_staging` 块。
- 保留 `source`、`parquet`、`clickhouse_raw`、`raw_asset_key`、`validation_notes`、`dataset_note_zh`。
- source-only dataset 也不再写 `dbt_staging: null` 或 `dbt_staging.status: not_started`。

完成标准：

- `rg -n "^dbt_staging:" pipeline/contracts/datasets` 无输出。
- `uv run fleur-contracts validate` 可加载所有 dataset。

### 5.2 Contract schema 和校验

目标文件：

- `pipeline/contract_tools/src/fleur_contracts/schema.py`
- `pipeline/contract_tools/src/fleur_contracts/validate.py`
- `pipeline/contract_tools/src/fleur_contracts/description_quality.py`

操作：

- 删除 `DbtStagingField`、`DbtStagingSpec`。
- 删除 `DatasetContract.dbt_staging` 字段。
- 删除 source-only dataset 禁止 active dbt staging 的校验。
- 删除 active stg field `from`、canonical naming、`glossary_key`、`canonical_exempt` 校验。
- 删除 registry 层对 stg glossary key 的存在性和 canonical name 校验。
- 确认描述质量校验仍覆盖：
  - `source.fields[].external_description_zh`
  - `glossary/fields.yml`
  - 必要时 `clickhouse_raw.fields[].reason`

完成标准：

- schema 中不再出现 `DbtStaging` 或 `dbt_staging`。
- contract registry tests 不再断言 active staging field。

### 5.3 生成器

目标文件：

- `pipeline/contract_tools/src/fleur_contracts/generate.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py`
- `pipeline/contract_tools/tests/test_contract_registry.py`

操作：

- 删除 `render_staging_yaml()`。
- `generate_outputs()` 只生成：
  - `pipeline/elt/models/sources.yml`
  - `docs/references/data_dict/*.md`
- `render_sources_yaml()` 保留，继续只输出有 `clickhouse_raw` 的 raw datasets。
- `render_data_dict_markdown()` 删除 stg lookup、stg glossary 描述优先逻辑和 `stg 字段` 列。
- raw dataset 的 data_dict 字段表调整为：

```markdown
| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
```

- source-only dataset 的 data_dict 表保持：

```markdown
| # | 外源字段 | 外源类型 | Parquet 类型 | 中文描述 |
```

完成标准：

- `uv run fleur-contracts generate --check` 不再检查或写入 `pipeline/elt/models/staging/staging.yml`。
- `rg -n "stg 字段|render_staging_yaml|dbt_staging" pipeline/contract_tools` 无实现引用。

### 5.4 dbt staging 所有权切换

目标文件：

- `pipeline/elt/models/staging/staging.yml`
- `pipeline/elt/models/staging/stg_*.sql`

操作：

- 不删除 `staging.yml`。
- 把 `staging.yml` 明确视作 dbt-owned 文件；若现有内容来自 contract 生成，作为当前初始手写基线保留。
- 后续字段描述、tests、model config、meta 调整通过 dbt 项目 review 和 dbt tests 管理。
- 可保留每个 model 的 `meta.contract_dataset` / `meta.contract_version`，但这些 metadata 只是 lineage/reference，不代表 contract_tools 校验 stg 字段。

完成标准：

- `git diff` 中 `staging.yml` 不因 `fleur-contracts generate` 被重写。
- dbt compile/build 仍能解析 staging models。

### 5.5 文档和 agent 路由

目标文件：

- `AGENTS.md`
- `docs/skills/fleur-contract-data-dictionary/SKILL.md`
- `pipeline/contracts/README.md`

操作：

- `AGENTS.md` 改为：`sources.yml` 和 `docs/references/data_dict/*.md` 由 contract 生成或校验；`staging.yml` 由 dbt 项目维护。
- skill 改为 raw-layer contract workflow：
  - 字段链路为 `source.fields -> parquet.fields -> clickhouse_raw.fields`。
  - 删除 `dbt_staging.fields` 修改步骤。
  - 删除 active stg 字段 glossary/canonical 校验要求。
  - raw 字段变更后只重新生成 `sources.yml` 和 data_dict。
- `pipeline/contracts/README.md` 改为 raw ClickHouse tables 和 generated data dictionary docs 的事实源。
- 不修改已经实施完成的历史 plans：
  - `docs/plans/0018-data-contract-registry-and-staging-layer-plan.md`
  - `docs/plans/0019-contract-zh-description-quality-remediation-plan.md`
  - `docs/plans/0020-field-type-normalization-debt-remediation-plan.md`
- 这些历史 plans 只作为当时实施背景读取；当前和未来执行以 `AGENTS.md`、repo skill、`pipeline/contracts/README.md` 和本 Plan 0021 为准。

完成标准：

- `rg -n "staging.yml.*contract|dbt_staging|stg 字段|raw/stg" AGENTS.md docs/skills/fleur-contract-data-dictionary/SKILL.md pipeline/contracts/README.md` 没有当前规则冲突。
- `docs/plans/0018-data-contract-registry-and-staging-layer-plan.md`、`docs/plans/0019-contract-zh-description-quality-remediation-plan.md`、`docs/plans/0020-field-type-normalization-debt-remediation-plan.md` 不出现在本计划实施 diff 中。

## 6. 实施阶段

### Phase 1: 固化边界文档

改动：

- 新增本计划。
- 更新 `AGENTS.md` 和 `docs/skills/fleur-contract-data-dictionary/SKILL.md` 的 contract/dbt 边界。
- 更新 `pipeline/contracts/README.md` 的 contract scope。
- 不修改已实施完成的 Plan 0018、Plan 0019、Plan 0020；这些文档保留历史执行记录属性。

验证：

```bash
git diff --check
```

完成标准：

- 后续 agent 能从 repo 文档读到同一条规则：contract 只到 ClickHouse raw，stg 由 dbt 管。

### Phase 2: 删除 contract 中的 stg payload

改动：

- 从 18 个 dataset YAML 删除 `dbt_staging` 块。
- source-only dataset 不再显式写 stg not-started 状态。

验证：

```bash
cd pipeline
uv run fleur-contracts validate
```

完成标准：

- 所有 dataset contract 加载通过。
- `rg -n "^dbt_staging:" pipeline/contracts/datasets` 无输出。

### Phase 3: 收缩 schema 和 tests

改动：

- 删除 schema 中的 stg 模型。
- 删除 stg glossary/canonical validation。
- 重写 contract registry tests：
  - 保留 raw/source-only dataset 数量测试。
  - 保留 generated outputs current 测试。
  - 保留 source-only dataset 不进入 dbt sources 和 ClickHouse specs 的测试。
  - 删除 active staging fields 测试。
  - 删除 source-only active dbt staging reject 测试。

验证：

```bash
cd pipeline
uv run pytest contract_tools/tests -q
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
```

完成标准：

- contract_tools 测试不再依赖 stg 字段事实。

### Phase 4: 停止生成 staging YAML

改动：

- 删除 `render_staging_yaml()`。
- `generate_outputs()` 移除 `pipeline/elt/models/staging/staging.yml`。
- `render_data_dict_markdown()` 删除 stg 列和 stg glossary 描述优先级。
- 重新生成 data_dict 和 `sources.yml`。

验证：

```bash
cd pipeline
uv run fleur-contracts generate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
```

完成标准：

- `pipeline/elt/models/staging/staging.yml` 不再出现在 `generate_outputs()` 的 rendered path 集合。
- data_dict 中没有 `stg 字段` 列。

### Phase 5: dbt 独立验证

改动：

- 不主动重写 `stg_*.sql`。
- 如 `staging.yml` 因 ownership 切换需要小的注释或 meta 调整，限制在 dbt YAML 内完成。

验证：

```bash
cd pipeline
uv run dbt compile --project-dir elt
uv run dbt build --project-dir elt --select staging
```

如果本地 ClickHouse 不可用，至少运行：

```bash
cd pipeline
uv run dbt compile --project-dir elt
```

完成标准：

- dbt 项目能独立解析 staging models。
- contract_tools 检查不再作为 staging YAML 的一致性来源。

### Phase 6: 全量最小门禁和报告

验证：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run ruff check contract_tools/src contract_tools/tests
uv run ruff format --check contract_tools/src contract_tools/tests
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
uv run dbt compile --project-dir elt
git diff --check
```

如代码改动触及 scheduler raw sync adapter，再追加：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests -q
cd scheduler
uv run dg check defs
```

完成标准：

- 所有适用检查通过。
- 若 ClickHouse/Parquet 实存 schema validation 因历史对象失败，记录为数据重建/回填问题，不阻塞本计划的代码边界清理。
- 新增 `docs/jobs/reports/<date>-contract-raw-scope-cleanup-report.md` 记录命令、结果、残留风险。

## 7. 禁止模式

- 禁止在新的 dataset contract 中重新加入 `dbt_staging`。
- 禁止让 `fleur-contracts generate` 写入 `pipeline/elt/models/staging/staging.yml`。
- 禁止在 contract_tools 中校验 dbt staging 字段名、字段测试、字段描述或 `stg_*.sql` 输出列。
- 禁止把 dbt-owned staging YAML 的漂移当作 contract generate failure。
- 禁止为了保留旧 data_dict 形态而从 stg SQL 反向解析字段。

## 8. 允许保留的例外

- `pipeline/elt/models/staging/staging.yml` 可以保留 `meta.contract_dataset` 和 `meta.contract_version`，用于说明上游 raw contract 来源。
- `pipeline/contracts/glossary/fields.yml` 可以继续作为中文描述和 canonical 命名参考，但不再强制 stg 字段匹配。
- 已实施完成的旧 plans 中的 stg contract 设计可以作为历史背景保留，不要求回改；当前规则以 Plan 0021 和入口文档为准。

## 9. 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| stg 字段描述质量失去 contract 校验 | dbt docs 质量可能回退 | 后续新增 dbt-owned 文档质量计划或 dbt YAML lint |
| data_dict 不再展示 stg 字段 | 阅读 raw 到 stg 映射时少一列 | 使用 dbt docs / lineage / `stg_*.sql` 查看建模映射 |
| glossary 不再强制约束 stg canonical | 字段命名一致性依赖 dbt review | 在 dbt 层另建 naming/test 规则，而不是放回 contract_tools |
| 旧文档误导 agent | 后续又把 stg 写回 contract | Phase 1 必须先更新 AGENTS、skill 和 contracts README；已完成 plans 保留历史属性，不作为当前规则入口 |

## 10. 验收清单

- [x] `pipeline/contracts/datasets/*.yml` 无 `dbt_staging`。
- [x] `pipeline/contract_tools/src/fleur_contracts/schema.py` 无 `DbtStaging` / `dbt_staging`。
- [x] `pipeline/contract_tools/src/fleur_contracts/generate.py` 不生成 `pipeline/elt/models/staging/staging.yml`。
- [x] `pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py` 不输出 `stg 字段` 列。
- [x] `pipeline/elt/models/staging/staging.yml` 保留为 dbt-owned 文件。
- [x] `fleur-contracts validate` 通过。
- [x] `fleur-contracts generate --check` 通过。
- [x] `pytest contract_tools/tests -q` 通过。
- [x] `pyright contract_tools/src/fleur_contracts contract_tools/tests` 通过。
- [x] `dbt compile --project-dir elt --profiles-dir elt` 通过。
- [x] 文档和 skill 不再声明 stg 字段由 contract 管理。

## 11. 后续维护

本计划完成后，如需要加强 dbt staging 治理，应新建独立 dbt 计划，候选方向包括：

- dbt YAML 字段描述质量检查。
- staging model 命名规范检查。
- `stg_*.sql` 输出列和 `staging.yml` columns 的一致性检查。
- dbt tests 覆盖策略。

这些治理应放在 `pipeline/elt` / dbt tooling 范围内，不回流到 `pipeline/contracts`。
