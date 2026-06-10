# mono-fleur Docs

本目录是项目记录系统。阅读顺序是先找当前事实，再看决策和计划，最后追溯历史材料。

## 快速入口

| 需要了解 | 入口 |
|---|---|
| 当前 scheduler 架构 | [architecture/scheduler-architecture.md](architecture/scheduler-architecture.md) |
| 当前模块边界和禁止模式 | [architecture/scheduler-module-boundaries.md](architecture/scheduler-module-boundaries.md) |
| 长期架构决策 | [ADR/README.md](ADR/README.md) |
| 活跃执行计划 | [plans/README.md](plans/README.md) |
| Dagster 回填、运行和 lineage 记录 | [jobs/README.md](jobs/README.md) |
| dbt 模型设计 | [design/README.md](design/README.md) |
| 接口、数据字典和 raw profiling | [references/README.md](references/README.md) |
| 质量审计和治理建议 | [optimize/README.md](optimize/README.md) |
| agent 操作手册 | [skills/](skills/) |

## 文档边界

| 目录 | 角色 | 当前性 |
|---|---|---|
| `architecture/` | 当前架构和边界 | 当前事实 |
| `ADR/` | 长期决策 | 当前或明确标注状态 |
| `RFC/` | 设计讨论和历史方案 | 活跃 RFC 在顶层，历史进 `archive/` |
| `plans/` | 仍需执行的计划 | 顶层只放 active plans |
| `plans/archive/` | 完成、废弃或被替代的计划 | 历史参考 |
| `jobs/` | runbook、snapshot 和运行报告入口 | 当前入口 |
| `jobs/reports/` | 实际运行事实 | 历史事实 |
| `design/` | dbt 模型设计和字段语义 | 当前事实 |
| `references/` | 外部接口、raw profile、data dict | 可查事实 |
| `optimize/` | 质量扫描和治理建议 | 建议或审计结果 |
| `skills/` | 可复用 agent 操作流程 | 当前 runbook |

## 状态约定

计划和设计类文档使用 `状态：` 标记：

- `Proposed`：已提出，尚未执行。
- `In Progress`：正在执行。
- `Blocked`：因明确外部条件暂停。
- `Completed`：已完成，等待归档或短期保留。
- `Accepted`：ADR 已接受。
- `Superseded`：被新文档替代。
- `Archived`：历史参考。

历史文档不要作为当前事实引用。引用历史方案时，同时链接当前代码、ADR、architecture 文档或运行报告作为证据。

## 最小校验

文档-only 变更至少运行：

```bash
make docs-check
git diff --check
```

涉及 Dagster、dbt、contracts 或 Rust 的事实更新时，追加对应领域校验命令。
