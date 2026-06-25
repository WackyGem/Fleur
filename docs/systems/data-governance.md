# System: Data Governance

状态：当前事实入口（2026-06-13）

## 代码根

| 路径 | 角色 |
|---|---|
| [pipeline/contracts/](../../pipeline/contracts/) | raw 层数据契约注册表 |
| [pipeline/contract_tools/](../../pipeline/contract_tools/) | contract 校验、生成和 schema adapter 工具 |
| [pipeline/elt/metadata/field_glossary.yml](../../pipeline/elt/metadata/field_glossary.yml) | dbt canonical 字段治理入口 |
| [docs/references/data_dict/](../references/data_dict/) | contract 生成或校验的数据字典 |
| [docs/references/raw_profile/](../references/raw_profile/) | staging 前 raw source profiling 记录 |

## 职责

1. 维护 source payload、S3 Parquet schema 和 ClickHouse raw 表字段事实。
2. 生成或校验 dbt raw `sources.yml` 和数据字典。
3. 维护 raw profiling 和 staging readiness 的可查依据。
4. 将字段命名、中文描述、raw layer 边界和 generated artifact 漂移纳入机械验证。

## 非职责

1. 不维护 dbt staging model SQL、staging column tests 或 mart 业务语义。
2. 不定义 Rearview metric 是否允许过滤、评分或输出；这些是 Rearview policy overlay 的职责。
3. 不直接执行数据采集或 ClickHouse 写入。

## 运行入口

修改字段事实后运行：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```

新增或重写 staging model 前，先维护 raw source profile：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py --source raw --table <dataset> --execute --output ../docs/references/raw_profile/<dataset>.md
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
```

## 质量门禁

```bash
cd pipeline
uv run ruff check contract_tools/src contract_tools/tests
uv run ruff format contract_tools/src contract_tools/tests
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest contract_tools/tests --cov=contract_tools/src/fleur_contracts --cov-report=term-missing
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../../pipeline/contracts/README.md](../../pipeline/contracts/README.md) | contract registry 当前边界 |
| [../../pipeline/elt/README.md](../../pipeline/elt/README.md) | dbt 字段治理和 staging readiness 入口 |
| [../ADR/0007-dbt-staging-cleaning-boundary.md](../ADR/0007-dbt-staging-cleaning-boundary.md) | staging 清洗职责边界 |
| [../ADR/0008-raw-source-profiling-before-dbt-staging.md](../ADR/0008-raw-source-profiling-before-dbt-staging.md) | raw profiling 决策 |
| [../RFC/archive/0010-data-contract-registry-and-contract-tools.md](../RFC/archive/0010-data-contract-registry-and-contract-tools.md) | contract registry 历史方案 |
| [../RFC/archive/0012-dbt-field-glossary-and-raw-source-governance.md](../RFC/archive/0012-dbt-field-glossary-and-raw-source-governance.md) | dbt field glossary 和 raw governance 历史设计 |
| [../skills/fleur-contract-data-dictionary/SKILL.md](../skills/fleur-contract-data-dictionary/SKILL.md) | contract/data dictionary 操作手册 |
| [../skills/fleur-dbt-model-readiness/SKILL.md](../skills/fleur-dbt-model-readiness/SKILL.md) | staging model 前置准备流程 |

## 待决问题

1. 是否需要为 mart 字段事实建立单独治理入口，服务 Rearview metric catalog 校验。
2. generated artifact 的 drift 检查是否需要纳入更细粒度的 CI 分组。
