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
4. 在 report 前半部分提炼“数据分析发现”，不要只留下 SQL 输出。
   - 数据量、覆盖范围、分区范围。
   - grain、候选键、重复情况；如存在新旧候选键，必须对比说明。
   - 关键字段 null / 空字符串 / 占位值，并解释哪些是预期缺失。
   - 证券代码、日期、报告期、状态字段等高价值字段的格式和异常样本。
   - 枚举分布、长尾文本、低频异常值；必要时补充字段间相关性。
   - 时间字段范围和业务合理性检查，例如日期先后关系。
   - 数值字段负数、零值、极端值和单位判断。
   - 只把已执行查询支持的事实写成结论；推断必须标明为判断。
5. 从 report 中提取 staging 设计：
   - rename。
   - cast。
   - normalize。
   - null handling。
   - data tests。
   - YAML `config.meta.source_columns` 和 normalization metadata。
   - deferred to intermediate/mart。
6. 只有完成 raw profile 后，再写 staging SQL/YAML。
7. 运行验证。

## Report Shape

raw profile report 应采用“结论优先，证据后置”的结构。SQL 附录用于追溯，
但 reviewer 应能先通过“数据分析发现”和“Staging 设计决策”判断模型怎么写。

推荐章节：

1. `范围与执行信息`：命令、日期、状态、contract、source、ClickHouse raw 表。
2. `数据分析发现`：用分组 bullet 写高信号事实。
   - 数据量与覆盖：总行数、distinct 主体数、分区 / 日期范围。
   - 粒度与候选键：候选键去重结果、旧键对比、重复样本归因。
   - 缺失与占位：关键字段 NULL、空字符串、`1970-01-01`、供应商占位。
   - 格式与参照完整性：证券代码格式、报告期标签、是否能命中直接依赖 raw input。
   - 分布与相关性：枚举 top values、少量值、长尾文本、字段间强相关。
   - 时间合理性：min/max、日期顺序异常、批次时间范围。
   - 数值合理性：负数、零值、极端值、单位假设。
3. `字段画像`：保留逐字段表，用于快速查 NULL、占位、distinct 和备注。
4. `数据质量问题`：只列实际发现的问题；通用风险不要写成问题。
5. `Staging 设计决策`：明确每个清洗动作是否进入 staging、测试和 metadata。
6. `延后到 Intermediate/Mart`：跨源归并、业务优先级、主数据修正、复杂文本归一化。
7. `待确认问题`。
8. `关键 SQL 证据摘要`：只放支撑结论的高信号结果。
9. `验收清单`：保持 `## 9. 验收清单` 标题，兼容 readiness lint。
10. `Profiling SQL 与结果`：完整查询和输出。

写法规则：

- 先写“事实”，再写“解释”，最后写“staging 决策”。
- 数字结论必须带计数；占比只作为辅助，不能替代计数。
- 对缺失值说明语义：预期缺失、source 未提供、占位待修复、异常缺失。
- 对长尾文本只做观察；同义归一化默认延后到 intermediate，除非是 trim/nullif 这类 source-local 规则。
- 参照完整性只能检查直接 raw input 或稳定主数据；不要在 staging 报告里做跨源主数据裁决。
- 发现“旧候选键重复、新候选键唯一”时，要记录两组键的对比和重复原因。
- 没执行过的检查保持 `待补充` 或 `未执行`，不要伪造分析结论。

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
