# Intake: Data Platform

状态：当前需求投递入口（2026-06-13）

当前事实地图：[../systems/data-platform.md](../systems/data-platform.md)

## 适用需求

- Dagster asset、job、schedule、sensor、resource 或 `dg` definitions 变更。
- dbt staging、intermediate、marts 模型和数据测试变更。
- ClickHouse raw sync、layered database、mart 消费形态和数据流编排。
- Pipeline 与 Furnace、Rearview、contracts 的数据边界调整。

## 不适用

- raw 字段事实、contract registry 和 generated data dictionary 维护：走 [data-governance.md](data-governance.md)。
- Rust 指标公式和高性能写入路径：走 [furnace.md](furnace.md)。
- Rearview 规则选股服务状态和 API：走 [rearview.md](rearview.md)。

## 投递材料

1. 目标数据流或目标模型。
2. 受影响的 Dagster asset、dbt model、ClickHouse database/table。
3. 输入数据来源、目标分区或日期范围。
4. 字段语义、清洗边界和是否需要 raw profiling。
5. 验收命令和期望数据核验口径。

## 文档落点

| 情况 | 落点 |
|---|---|
| 新数据源或跨层数据流设计 | `docs/RFC/` |
| 已定方案的 Dagster/dbt 分阶段实施 | `docs/plans/` |
| staging 清洗、raw profiling 或 layer 边界长期变化 | `docs/ADR/` |
| 当前数据平台运行入口或质量门禁变化 | [../systems/data-platform.md](../systems/data-platform.md) |
| 实际回填、重跑或核验 | `docs/jobs/reports/` |

## 验证要求

按变更范围选择最小命令，通常从以下命令开始：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
cd scheduler
uv run dg check defs
```

Dagster 相关任务先使用 `dagster-expert`；dbt 相关任务使用 dbt skills。
