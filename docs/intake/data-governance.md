# Intake: Data Governance

状态：当前需求投递入口（2026-06-13）

当前事实地图：[../systems/data-governance.md](../systems/data-governance.md)

## 适用需求

- 新增或修改 `pipeline/contracts/datasets/*.yml`。
- 维护 raw 层字段事实、Parquet schema、ClickHouse raw 表定义和 data dictionary。
- 维护 dbt generated raw `sources.yml` 的 contract 生成/校验流程。
- 维护 canonical field glossary、字段中文描述、raw profile 和 staging readiness 前置材料。

## 不适用

- dbt model SQL 和 mart 业务模型设计：走 [data-platform.md](data-platform.md)。
- Rearview metric 是否可过滤、可评分、可输出：走 [rearview.md](rearview.md)，但可关联本入口做字段事实校验。
- 外部 API 调度和采集实现：走 [data-platform.md](data-platform.md)。

## 投递材料

1. 数据集名称和 source 系统。
2. 样例 payload、远端接口文档或 raw profile。
3. 字段增删改、类型、nullable、中文描述和外部语义。
4. 生成物影响：Parquet schema、ClickHouse raw、dbt source、data dictionary。
5. 是否影响 downstream staging、mart、Rearview metric catalog。

## 文档落点

| 情况 | 落点 |
|---|---|
| 新数据契约治理机制或边界调整 | `docs/RFC/` 或 `docs/ADR/` |
| 具体 contract/schema 修复计划 | `docs/plans/` |
| raw profile 或数据字典事实 | `docs/references/` |
| 当前 governance 命令或边界变化 | [../systems/data-governance.md](../systems/data-governance.md) |
| 可复用操作步骤变化 | `docs/skills/fleur-contract-data-dictionary/SKILL.md` 或相关 skill |

## 验证要求

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run python elt/scripts/validate_field_glossary.py
```

新增或重写 staging 前，按 [../skills/stg-model-readiness/SKILL.md](../skills/stg-model-readiness/SKILL.md) 维护 raw profile。
