# Requirement Intake

状态：当前需求投递入口（2026-06-13）

本目录负责把新需求先按领域分流，再决定是否进入 `RFC/`、`plans/`、`ADR/`、`systems/`、`jobs/`、`references/` 或 `skills/`。它不保存完整方案和实施计划；完整方案仍放 `docs/RFC/`，执行计划仍放 `docs/plans/`，长期决策仍放 `docs/ADR/`。

## 分流顺序

1. 先选择最接近的领域 intake。
2. 阅读对应系统地图，确认当前代码根、边界、运行入口和质量门禁。
3. 判断文档落点：小改动直接改代码和系统地图；复杂需求写 RFC；跨阶段实施写 plan；长期约束写 ADR。
4. 实施后回到代码、测试和最小质量门禁验证事实。

## 领域入口

| 领域 | 需求入口 | 当前事实地图 | 典型需求 |
|---|---|---|---|
| 数据平台 | [data-platform.md](data-platform.md) | [../systems/data-platform.md](../systems/data-platform.md) | Dagster asset、dbt model、ClickHouse layer、数据流编排 |
| 数据治理 | [data-governance.md](data-governance.md) | [../systems/data-governance.md](../systems/data-governance.md) | contracts、字段字典、raw profiling、field glossary |
| Furnace 计算引擎 | [furnace.md](furnace.md) | [../systems/furnace.md](../systems/furnace.md) | Rust 指标公式、CLI、ClickHouse 写入、性能优化 |
| Rearview 后端 | [rearview.md](rearview.md) | [../systems/rearview.md](../systems/rearview.md) | 规则选股 API、PostgreSQL 状态、metric catalog、查询规划 |
| Racingline 前端 | [racingline.md](racingline.md) | [../systems/racingline.md](../systems/racingline.md) | React UI、路由、状态管理、图表、Playwright CDP 调试 |
| 部署与运行 | [deploy-ops.md](deploy-ops.md) | [../systems/deploy-ops.md](../systems/deploy-ops.md) | Docker Compose、环境变量、migration、runbook、运行报告 |

## 落点决策

| 情况 | 文档落点 |
|---|---|
| 小范围代码或文档修正，边界不变 | 直接修改代码和相关系统地图；必要时补 job report |
| 新功能、跨模块设计或需要评审的方案 | `docs/RFC/NNNN-short-title.md` |
| 已确定方案，需要阶段拆分、任务顺序和验收标准 | `docs/plans/NNNN-short-title.md` |
| 会长期约束架构、数据边界、运行方式或团队惯例 | `docs/ADR/NNNN-short-title.md` |
| 当前系统代码根、职责、运行入口或质量门禁变化 | `docs/systems/<domain>.md` |
| 外部接口、schema、样例、数据字典或 raw profile 变化 | `docs/references/` |
| 实际回填、重跑、性能基线、smoke run 或生产核验 | `docs/jobs/reports/` |
| 可复用 agent 操作流程变化 | `docs/skills/<skill>/SKILL.md` |

## RFC/Plan 头部建议

新 RFC 或 plan 建议在标题后保留轻量元信息，方便跨领域检索：

```text
状态：草案
领域：rearview
关联系统：rearview, racingline, data-platform
代码根：engines/crates/rearview/
需求入口：docs/intake/rearview.md
```

## 跨领域规则

- 多领域需求从用户可见目标或主要状态 owner 选择主 intake。
- 前端需求依赖后端 API 时，主入口通常是 `racingline`，并在 RFC 中列出 Rearview 后端补齐项。
- 后端服务需要新增 mart 或指标时，主入口通常是 `rearview`，并关联 `data-platform` 或 `furnace`。
- 数据契约和 raw 字段事实优先走 `data-governance`；dbt 模型语义和 mart 消费形态优先走 `data-platform`。
- 运行环境、migration、回填和 smoke run 优先走 `deploy-ops`，再链接具体系统。
