# Design Docs

本目录记录模型和数据产品设计，当前主要维护 dbt layer 文档。

## dbt Layer

| 目录 | 用途 |
|---|---|
| [dbt_layer/fleur_staging/](dbt_layer/fleur_staging/) | staging 模型设计，字段清洗和 canonical 语义 |
| [dbt_layer/fleur_intermediate/](dbt_layer/fleur_intermediate/) | intermediate 模型设计，跨源组合和可复用业务过程 |
| [dbt_layer/fleur_marts/](dbt_layer/fleur_marts/) | marts 模型设计，稳定消费接口 |

模型设计文档应链接对应 SQL/YAML、字段事实来源和必要验证命令。不要把已接受的长期架构规则只写在 design 文档中；长期规则应进入 `docs/ADR/` 或 `docs/architecture/`。
