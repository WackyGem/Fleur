# ADR 0007: dbt staging 清洗边界

状态：Accepted

日期：2026-06-02

## 背景

ADR 0005 已决定 Dagster 负责 ClickHouse raw 同步，dbt 负责 staging/marts 建模。RFC 0012 和 Plan 0024 进一步把 raw contract 与 dbt canonical 字段治理拆开：

```text
pipeline/contracts/datasets/*.yml
  -> source / parquet / ClickHouse raw 字段事实
  -> generated dbt raw sources.yml

pipeline/elt
  -> dbt canonical 字段名、字段描述、值格式、tests、meta
  -> staging / intermediate / mart transformations
```

实施首批 staging model 后，需要明确一个长期建模边界：staging 是否可以包含数据清洗逻辑，还是所有清洗都必须推迟到 intermediate 层。

如果 staging 完全不清洗，下游模型会重复处理字段命名、类型转换、证券代码格式、日期格式、空字符串和供应商枚举值。如果 staging 承担过多清洗，又容易把跨源实体匹配、主数据修正、指标口径和宽表聚合提前塞进第一层模型，导致语义边界模糊。

## 决策

dbt staging model 可以包含数据清洗和标准化逻辑，但只能处理单一 raw source/table 内、确定性、低业务口径风险的转换。

staging 的职责是把 raw 表变成干净、类型正确、命名统一、source-local canonical 的表。intermediate 的职责是把多个干净 staging models 组合成业务实体、业务过程或改变 grain 的中间结构。

staging 允许：

- 字段重命名：把供应商字段名显式 alias 到 dbt canonical 字段名。
- 类型转换：日期、时间戳、布尔、数值精度和字符串类型收敛。
- 格式标准化：例如 `sh.601088` 或 `601088.SH` 统一为 `601088.SH`。
- 确定性拆分：例如从标准证券代码拆出 `security_local_code` 和 `exchange_code`。
- 轻量枚举标准化：供应商状态码到项目内枚举，前提是映射明确且不依赖跨表上下文。
- 单位归一：金额、比例、百分比等，前提是 raw 字段单位确定。
- 明显占位值处理：例如空字符串、固定占位日期或供应商明确声明的缺失值转 `null`，并在 YAML metadata 中说明。
- 基础过滤：仅限排除明显无效或不可解析的 source-local 记录；不能过滤掉业务上仍需解释的记录。

staging 禁止：

- 多 source 合并、跨源 join、union 或实体匹配。
- 需要优先级判断的去重，例如“取最新供应商记录”或“优先 EastMoney 覆盖 BaoStock”。
- 证券主数据修正、退市状态修正、代码历史映射和跨市场实体归并。
- 财务指标口径重算、业务指标派生或复杂科目映射。
- 聚合、宽表组装、改变 grain 的建模。
- 依赖多表上下文的业务推断。

需要复用的 staging 标准化逻辑应优先放在 `pipeline/elt/macros/`，并由 staging YAML 的 `config.meta.normalization` 或等价 metadata 记录来源格式和转换意图。高价值字段应配套 dbt generic tests 或明确的中文豁免原因。

## 依据

dbt staging 最适合作为 source-conformed 的第一层转换：从一个 source table 读取，完成重命名、类型转换、轻量计算和基础标准化，供下游模型复用。

本项目还需要满足以下约束：

- raw contract 只覆盖 source、Parquet 和 ClickHouse raw 字段事实，不管理 dbt canonical 字段和清洗规则。
- `pipeline/elt/metadata/field_glossary.yml` 是 dbt canonical 字段事实源。
- `pipeline/elt/models/sources.yml` 是 generated raw source catalog，只表达 raw/source 语境。
- staging YAML 必须记录 `glossary_key`、`source_columns`、`data_type`、`data_tests` 和必要的 normalization metadata。
- manifest lint 和 dbt tests 是 staging 清洗边界的机械约束。

## 后果

- 下游 intermediate 和 mart 不需要重复处理基础字段清洗，能直接消费 canonical staging columns。
- staging SQL 会包含少量确定性转换，因此每个高价值转换都需要 macro、YAML metadata 和 tests 支撑。
- 对于不确定的字段语义，不允许为了让 lint 通过而伪造 canonical 含义；应使用 local/derived 例外或推迟到 intermediate。
- 如果一个清洗规则需要多个 sources、业务优先级、主数据语义或改变 grain，它必须进入 intermediate 或 mart。
- 后续新增 staging model 时，review 应优先检查清洗逻辑是否仍满足 source-local、确定性、低业务口径风险三个条件。

## 关联

- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/RFC/0012-dbt-field-glossary-and-raw-source-governance.md`
- `docs/plans/0024-dbt-field-glossary-and-raw-source-governance-implementation-plan.md`
- `pipeline/elt/metadata/field_glossary.yml`
- `pipeline/elt/scripts/validate_field_glossary.py`
