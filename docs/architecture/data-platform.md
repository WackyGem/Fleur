# Architecture: Data Platform

状态：当前事实入口（2026-06-13）

## 代码根

| 路径 | 角色 |
|---|---|
| [pipeline/scheduler/](../../pipeline/scheduler/) | Dagster 项目，负责数据采集、资产编排、S3 Parquet 写入和 ClickHouse raw sync |
| [pipeline/elt/](../../pipeline/elt/) | dbt 项目，负责 staging、intermediate、marts 建模和数据测试 |
| [pipeline/migrate/](../../pipeline/migrate/) | Alembic 迁移项目，负责 PostgreSQL 业务库 schema 变更 |

## 职责

1. 通过 Dagster asset 编排外部数据采集、Parquet 落盘、raw 同步和下游运行。
2. 通过 dbt 维护 canonical 字段、staging 清洗、intermediate wrapper 和 mart 消费层。
3. 将 pipeline 运行事实、回填记录和数据核验沉淀到 jobs 文档。
4. 保持 Dagster、dbt、contracts 和 ClickHouse 层之间的边界清晰。

## 非职责

1. 不在 Dagster asset、dbt SQL 或 Python 资源中重写 Furnace 指标递推公式。
2. 不把 raw 层字段事实绕过 contracts 手工写入 dbt generated source。
3. 不把 Rearview 规则状态、运行状态或买入信号写入 pipeline database。
4. 不承担 Racingline 前端交互和用户体验实现。

## 主要依赖

| 依赖 | 用途 |
|---|---|
| PostgreSQL `pipeline` database | OCR、调度辅助状态和 pipeline 业务状态 |
| S3-compatible object store | source parquet、图片和中间对象存储 |
| ClickHouse | raw、staging、intermediate、calculation、marts 分层存储 |
| External APIs | BaoStock、EastMoney、Sina、THS、JiuYan 等数据源 |
| Furnace CLI | 技术指标计算资产通过 Python resource 调用 Rust CLI |

## 运行入口

Python、dbt、Dagster 和 `dg` 命令必须在 `pipeline/` 目录下通过 `uv run` 执行：

```bash
cd pipeline
uv sync --all-packages --all-groups
uv run dbt parse --project-dir elt --profiles-dir elt
cd scheduler
uv run dg check defs
```

## 质量门禁

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run ruff format scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests --cov=scheduler/src/scheduler --cov=contract_tools/src/fleur_contracts --cov-report=term-missing
cd scheduler
uv run dg check defs
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [scheduler-architecture.md](scheduler-architecture.md) | 当前 scheduler 架构和主要数据流 |
| [scheduler-module-boundaries.md](scheduler-module-boundaries.md) | scheduler 模块职责、依赖方向和禁止模式 |
| [README.md](README.md) | dbt 模型设计入口 |
| [../jobs/README.md](../jobs/README.md) | Dagster 回填、运行记录和 reports 入口 |
| [../ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md](../ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md) | Dagster raw sync 与 dbt 建模边界 |
| [../ADR/0007-dbt-staging-cleaning-boundary.md](../ADR/0007-dbt-staging-cleaning-boundary.md) | dbt staging 清洗边界 |
| [../ADR/0008-raw-source-profiling-before-dbt-staging.md](../ADR/0008-raw-source-profiling-before-dbt-staging.md) | staging 前 raw profiling 约束 |
| [../ADR/0009-clickhouse-layered-databases.md](../ADR/0009-clickhouse-layered-databases.md) | ClickHouse 分层 database 决策 |

## 待决问题

1. 是否需要为 data platform 拆出更高层 architecture 文档，覆盖 scheduler、dbt、contracts、Furnace 和 Rearview 的端到端流向。
2. Dagster jobs reports 与架构事实文档之间是否需要自动索引。
