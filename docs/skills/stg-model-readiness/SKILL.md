---
name: stg-model-readiness
description: mono-fleur 的 dbt staging 前置准备工作流。用于新增或重写 staging model、讨论 staging 清洗规则、为 raw table 设计 canonical 字段、或需要先完成 raw source profiling 再写 stg SQL/YAML/tests 的任务。
---

# Staging Model Readiness

在写或重写 `pipeline/elt/models/staging/**` 前使用本 skill。目标是先理解 raw source 的真实数据质量和数据特征，再设计 staging 清洗、字段治理和 tests。

## 必读入口

1. `AGENTS.md` 的 dbt 入口。
2. `docs/ADR/0007-dbt-staging-cleaning-boundary.md`。
3. `docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`。
4. `docs/RFC/0013-raw-source-profiling-before-dbt-staging.md`。
5. `docs/plans/0025-raw-source-profiling-before-dbt-staging-implementation-plan.md`。

## Workflow

1. 确认目标 staging model 和直接 raw inputs。
   - 读取 `pipeline/elt/models/sources.yml`。
   - 读取对应 `pipeline/contracts/datasets/<dataset>.yml`。
   - 如存在，读取 `docs/references/data_dict/<dataset>.md`。
2. 按 dbt discovering-data 方法完成 raw profiling。
   - inventory。
   - sample raw data。
   - row count、日期/分区范围。
   - grain 和候选自然键。
   - null、空字符串、占位值。
   - 枚举值和 top values。
   - 证券代码/日期/高价值字符串格式。
   - 数值范围、负数、零值、极端值和单位判断。
3. 写入 `docs/references/raw_profile/<dataset>.md`。
4. 从 report 中提取 staging 设计：
   - rename。
   - cast。
   - normalize。
   - null handling。
   - data tests。
   - YAML `config.meta.source_columns` 和 normalization metadata。
   - deferred to intermediate/mart。
5. 只有完成 raw profile 后，再写 staging SQL/YAML。
6. 运行验证。

## Commands

列出 raw sources：

```bash
cd pipeline
uv run dbt ls --project-dir elt --profiles-dir elt --select "source:raw.*" --output json
```

生成 profile report 草稿：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py \
  --source raw \
  --table <dataset> \
  --output ../docs/references/raw_profile/<dataset>.md
```

执行基础 profiling 并写入报告：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py \
  --source raw \
  --table <dataset> \
  --execute \
  --output ../docs/references/raw_profile/<dataset>.md
```

staging 修改后的验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select staging --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

## Rules

- 不把 profiling report 写回 `pipeline/contracts`。
- 不把 `profile_raw_source.py` 或 readiness lint 放进 `pipeline/contracts`。
- 不伪造未执行的数据观察；数据环境不可用时保持 `Draft` 并记录原因。
- 不用全字段 `not_null` / `accepted_values` 代替 profiling。
- 不把跨源去重、主数据修正、业务优先级判断写入 staging 清洗建议。
- 对复杂 normalization macro 或 transformation，后续使用 dbt unit test 做 TDD。
