# ADR 0008: dbt staging 前置 raw source profiling

状态：Accepted

日期：2026-06-02

## 背景

ADR 0007 已决定 dbt staging 可以做 source-local、确定性、低业务口径风险的数据清洗和标准化。这个决策依赖一个前提：staging 作者必须先理解 raw source 的真实数据质量和数据特征。

`pipeline/contracts/datasets/*.yml` 和 generated `pipeline/elt/models/sources.yml` 能描述 raw 字段事实、类型、字段说明和 lineage，但不能完整回答以下问题：

- raw 表的实际 grain 是什么。
- 自然键是否唯一，重复来自哪里。
- 日期、报告期、分区和数据范围是否符合预期。
- 空字符串、占位值、异常枚举和格式漂移是否存在。
- 证券代码、交易所、状态码、金额、比例等高价值字段在真实数据中如何分布。
- 哪些问题适合在 staging 轻清洗，哪些必须留到 intermediate 或 mart。

如果不先做 raw profiling，staging model 容易只根据字段名和 contract 推测逻辑，导致清洗规则不完整、测试缺失或把跨源业务判断提前放入 staging。

## 决策

新增或重写 dbt staging model 前，必须先完成对应 raw source/table 的数据质量与数据特征分析，并把结论记录为可复用文档。

profiling 是 staging 设计的前置输入，不是事后补充材料。staging SQL、YAML metadata、field glossary 扩展、normalization macro 和 data tests 应基于 profiling 结论制定。

第一版采用项目内轻量工作流，不引入独立数据质量平台：

```text
generated raw source catalog
  -> raw source profiling queries
  -> docs/references/raw_profile/<dataset>.md
  -> staging SQL / YAML / tests / glossary updates
  -> manifest lint and dbt build
```

每份 raw profile report 至少记录：

- source/table 名称、关联 contract dataset 和 profiling 日期。
- 行数、分区范围、日期范围和数据规模分布。
- 表 grain 和候选自然键。
- 关键字段的 null rate、空字符串、占位值和异常值。
- 枚举字段的 distinct values 和样本。
- 高价值字符串字段的格式样本，例如证券代码格式。
- 数值字段的 min/max、负数、零值、极端值和单位判断。
- 重复记录、缺失记录或明显不一致记录。
- 推荐进入 staging 的清洗和标准化规则。
- 必须推迟到 intermediate/mart 的问题。
- 待确认事项和不能静默处理的问题。

dbt 作为执行和校验底座使用：

- 使用 `dbt show --inline` 或等价 dbt/ClickHouse 查询进行抽样、聚合和质量检查。
- 使用 `sources.yml` 和 dbt docs 理解 raw source metadata。
- 对已经确认的 source 断言，可以在 source YAML 上增加 dbt data tests。
- 对 staging 输出，继续使用 manifest lint、dbt generic tests 和 `dbt build --select staging` 校验。

后续可以新增项目脚本和 skill 固化流程：

- `pipeline/elt/scripts/profile_raw_source.py`：生成标准 profiling 查询或报告草稿。
- `docs/references/raw_profile/<dataset>.md`：保存 profiling 结果。
- `docs/skills/fleur-dbt-model-readiness/SKILL.md`：定义 agent 在写 staging 前必须执行的工作流。
- `pipeline/elt/scripts/validate_staging_readiness.py`：检查 staging model 是否有对应 raw profile report。

## 非目标

- 不把 profiling 结论写回 `pipeline/contracts/datasets/*.yml`。
- 不把 raw contract 扩展成 dbt staging 清洗规则事实源。
- 不要求一开始引入 Great Expectations、Soda、OpenMetadata 或类似平台。
- 不要求对所有 raw tables 一次性完成 profiling；只强制覆盖当前要新增或重写 staging model 的 raw inputs。
- 不用 profiling report 替代 dbt tests。报告用于设计，tests 用于持续校验。

## 依据

dbt 提供 source metadata、source tests、source freshness、`dbt show`、dbt docs、data tests、unit tests 和 model contracts 等能力，但 dbt Core 不会自动完成面向项目语义的 raw profiling，也不会自动判断清洗逻辑应该位于 staging 还是 intermediate。

mono-fleur 已经有 raw contract、generated source catalog、dbt field glossary 和 manifest lint。当前更适合先用项目内脚本、报告模板和 skill 固化流程，等 raw/staging 规模扩大后再评估是否引入外部数据质量平台。

## 后果

- staging 设计会多一个明确前置步骤，但能减少凭字段名猜测逻辑造成的返工。
- profiling report 会成为后续 review、debug、字段治理和 tests 设计的事实依据。
- 新增 staging model 时，review 应检查对应 raw profile 是否存在且覆盖实际使用的 raw columns。
- 如果 profiling 发现问题需要跨源判断、业务优先级或改变 grain，应按 ADR 0007 推迟到 intermediate/mart。
- 后续可把 readiness 检查加入 dbt 开发门禁，避免 staging model 绕过前置分析。

## 关联

- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/RFC/0013-raw-source-profiling-before-dbt-staging.md`
- `docs/RFC/0012-dbt-field-glossary-and-raw-source-governance.md`
- `docs/plans/archive/0024-dbt-field-glossary-and-raw-source-governance-implementation-plan.md`
- `pipeline/elt/models/sources.yml`
- `pipeline/elt/metadata/field_glossary.yml`
- `pipeline/elt/scripts/validate_field_glossary.py`
