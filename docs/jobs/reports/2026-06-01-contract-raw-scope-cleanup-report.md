# Contract raw scope cleanup report

日期：2026-06-01

关联计划：`docs/plans/0021-contract-scope-raw-layer-cleanup-plan.md`

## 范围

本次实施把 `pipeline/contracts` 和 `pipeline/contract_tools` 的字段事实范围收缩到 ClickHouse raw 层：

- 删除 dataset contract 中的 `dbt_staging` 块。
- 删除 contract schema、registry validation、generator 和 data_dict renderer 中的 stg 字段事实处理。
- `fleur-contracts generate` 不再写入 `pipeline/elt/models/staging/staging.yml`。
- data_dict 字段链路不再展示 `stg 字段` 列。
- 更新 `AGENTS.md`、`docs/skills/fleur-contract-data-dictionary/SKILL.md`、`pipeline/contracts/README.md` 的当前边界说明。

按计划要求，已实施完成的历史 plans 未修改：

- `docs/plans/0018-data-contract-registry-and-staging-layer-plan.md`
- `docs/plans/0019-contract-zh-description-quality-remediation-plan.md`
- `docs/plans/0020-field-type-normalization-debt-remediation-plan.md`

## 改动结果

- `pipeline/contracts/datasets/*.yml` 不再包含顶层 `dbt_staging`。
- `pipeline/contract_tools/src/fleur_contracts/schema.py` 不再定义 `DbtStagingField` / `DbtStagingSpec`。
- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py` 只保留 `render_sources_yaml()`。
- `pipeline/contract_tools/src/fleur_contracts/generate.py` 只生成：
  - `pipeline/elt/models/sources.yml`
  - `docs/references/data_dict/*.md`
- `pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py` 对 raw dataset 输出：

```markdown
| # | 外源字段 | 外源类型 | Parquet 类型 | ClickHouse raw 字段 | ClickHouse 类型 | 中文描述 |
```

- source-only dataset 的 data_dict 继续不展示 ClickHouse raw columns。
- `pipeline/elt/models/staging/staging.yml` 保留为 dbt-owned 文件。当前工作区内该文件已有 dbt-owned 字段命名变更；它不再由 contract generator 管理。

## 验证

已通过：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run ruff check contract_tools/src contract_tools/tests
uv run ruff format --check contract_tools/src contract_tools/tests
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
set -a && . ../.env && set +a && uv run dbt compile --project-dir elt --profiles-dir elt --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
git diff --check
```

结果：

- `fleur-contracts validate`：Validated 18 dataset contracts.
- `fleur-contracts generate --check`：Generated outputs are current for 0 changed files.
- `pytest contract_tools/tests -q`：9 passed.
- `ruff check`：All checks passed.
- `ruff format --check`：15 files already formatted.
- `pyright`：0 errors, 0 warnings, 0 informations.
- `dbt compile`：通过。需要 repo `.env` 中的 ClickHouse profile 环境变量，并显式使用 `--profiles-dir elt`。
- `git diff --check`：通过。

额外核验：

```bash
rg -n "^dbt_staging:" pipeline/contracts/datasets
rg -n "stg 字段|dbt_staging|render_staging_yaml|DbtStaging" pipeline/contract_tools docs/references/data_dict pipeline/contracts/datasets
git diff --name-only -- docs/plans/0018-data-contract-registry-and-staging-layer-plan.md docs/plans/0019-contract-zh-description-quality-remediation-plan.md docs/plans/0020-field-type-normalization-debt-remediation-plan.md
```

结果均无输出。

## dbt build 结果

尝试运行：

```bash
cd pipeline
set -a && . ../.env && set +a
uv run dbt build --project-dir elt --profiles-dir elt --select path:models/staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

结果失败在本地 ClickHouse 缺少 raw 表，不是 dbt parse、compile 或 contract generator 问题。失败表：

- `raw.baostock__query_stock_basic`
- `raw.jiuyan__industry_list`
- `raw.jiuyan__industry_ocr_snapshot`
- `raw.sina__trade_calendar`

这些表需要通过 Dagster raw sync 或相应回填先物化，再运行 staging build。

## 残留风险

- dbt staging 字段描述和 naming 一致性不再由 contract_tools 校验；后续如需治理，应在 dbt 项目范围内新增独立检查。
- `pipeline/contracts/glossary/fields.yml` 保留为 raw/data_dict 参考词表，不再强制约束 stg 字段。
- 当前工作区仍包含 Plan 0020 相关既有变更，本报告只记录 Plan 0021 的 scope cleanup。
