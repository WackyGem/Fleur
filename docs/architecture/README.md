# Architecture

本目录是 fleur 当前项目状态和架构事实入口。原 `docs/systems/` 的代码根、职责边界、运行入口和质量门禁已并入本目录；长期决策仍进入 `docs/ADR/`，历史方案讨论仍进入 `docs/RFC/`。

## 当前入口

| 文档 | 用途 |
|---|---|
| [project-status.md](project-status.md) | 当前项目状态、子域索引和跨系统边界 |
| [data-platform.md](data-platform.md) | Dagster、dbt、ClickHouse raw/marts 和 pipeline migration 边界 |
| [data-governance.md](data-governance.md) | contracts、contract tools、field glossary 和 data dictionary 边界 |
| [furnace.md](furnace.md) | Furnace Rust 技术指标计算 CLI、core/io crate 和 Dagster 调用边界 |
| [rearview.md](rearview.md) | Rearview Rust HTTP 服务、portfolio worker、PostgreSQL/ClickHouse/NATS 边界 |
| [racingline.md](racingline.md) | Racingline 前端工作台、策略创建和组合发布 UI 边界 |
| [deploy-ops.md](deploy-ops.md) | Docker Compose、本地基础设施、migration 和运行记录入口 |
| [scheduler-architecture.md](scheduler-architecture.md) | scheduler 当前架构和主要数据流 |
| [scheduler-module-boundaries.md](scheduler-module-boundaries.md) | scheduler 模块职责、依赖方向和禁止模式 |
| [dbt_layer/](dbt_layer/) | dbt staging、intermediate 和 marts 模型设计，字段边界和对应 SQL/YAML |

## dbt Layer

| 目录 | 用途 |
|---|---|
| [dbt_layer/fleur_staging/](dbt_layer/fleur_staging/) | staging 模型设计，字段清洗和 canonical 语义 |
| [dbt_layer/fleur_intermediate/](dbt_layer/fleur_intermediate/) | intermediate 模型设计，跨源组合和可复用业务过程 |
| [dbt_layer/fleur_marts/](dbt_layer/fleur_marts/) | marts 模型设计，稳定消费接口 |

模型设计文档应链接对应 SQL/YAML、字段事实来源和必要验证命令。不要把已接受的长期架构规则只写在 dbt layer 文档中；长期规则应进入 `docs/ADR/` 或本目录的架构边界文档。

## 维护规则

- 更新代码根、职责、运行入口、质量门禁或跨系统依赖时，优先同步本目录对应文档。
- 本目录只记录当前事实和可导航边界；长篇方案、执行计划和运行结果分别放在 `docs/RFC/`、`docs/plans/` 和 `docs/jobs/`。
- 历史文档不能单独作为当前事实依据；引用归档材料时，同时链接本目录文档、ADR、运行报告或当前代码。
