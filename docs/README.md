# mono-fleur Docs

本目录是项目记录系统。新需求先从当前架构事实确认影响范围，再进入决策、计划、运行记录或历史材料。

## 快速入口

### 按当前架构查事实

| 需要了解 | 入口 |
|---|---|
| 多工程项目状态 | [architecture/project-status.md](architecture/project-status.md) |
| 数据平台：Dagster、dbt、ClickHouse raw/marts | [architecture/data-platform.md](architecture/data-platform.md) |
| 数据治理：contracts、field glossary、data dictionary | [architecture/data-governance.md](architecture/data-governance.md) |
| Furnace Rust 技术指标计算引擎 | [architecture/furnace.md](architecture/furnace.md) |
| Rearview Rust 规则选股后端服务 | [architecture/rearview.md](architecture/rearview.md) |
| Racingline Rearview 前端工作台 | [architecture/racingline.md](architecture/racingline.md) |
| 部署、迁移和运行记录 | [architecture/deploy-ops.md](architecture/deploy-ops.md) |

### 按文档类型

| 需要了解 | 入口 |
|---|---|
| 当前架构边界和 dbt 模型设计 | [architecture/README.md](architecture/README.md) |
| 长期架构决策 | [ADR/README.md](ADR/README.md) |
| 方案讨论和历史设计 | [RFC/](RFC/) |
| 活跃执行计划 | [plans/README.md](plans/README.md) |
| 技术债、实现漂移和质量审计 | [issues/README.md](issues/README.md) |
| Dagster 回填、运行和 lineage 记录 | [jobs/README.md](jobs/README.md) |
| 集成发布快照和版本 manifest | [releases/README.md](releases/README.md) |
| 接口、数据字典和 raw profiling | [references/README.md](references/README.md) |
| 设计问答和讨论记录 | [Q&A/](Q&A/)；Racingline 当前用户画像见 [Q&A/user-logic.md](Q&A/user-logic.md)；两入口导航见 [Q&A/0003-racingline-strategy-lab-two-entry-navigation.md](Q&A/0003-racingline-strategy-lab-two-entry-navigation.md)；原型看板到策略创建闭环见 [Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md](Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)；前端原型流程 RFC 见 [RFC/archive/0023-racingline-frontend-prototype-led-development.md](RFC/archive/0023-racingline-frontend-prototype-led-development.md)；策略选股 Step 1 RFC 见 [RFC/archive/0024-racingline-strategy-selection-step1.md](RFC/archive/0024-racingline-strategy-selection-step1.md)；策略权重配置 Step 2 RFC 见 [RFC/archive/0025-racingline-strategy-weight-configuration-step2.md](RFC/archive/0025-racingline-strategy-weight-configuration-step2.md)；股池预览 Step 3 RFC 见 [RFC/archive/0026-racingline-strategy-pool-preview-step3.md](RFC/archive/0026-racingline-strategy-pool-preview-step3.md)；模拟建仓 Step 4 RFC 见 [RFC/archive/0027-racingline-strategy-simulation-position-step4.md](RFC/archive/0027-racingline-strategy-simulation-position-step4.md)；策略回测 Step 5 RFC 见 [RFC/archive/0028-racingline-strategy-backtest-step5.md](RFC/archive/0028-racingline-strategy-backtest-step5.md)；UI 栈变体评估 ADR 见 [ADR/0013-racingline-ui-stack-variant-evaluation.md](ADR/0013-racingline-ui-stack-variant-evaluation.md) |
| agent 操作手册 | [skills/](skills/) |

## 文档边界

| 目录 | 角色 | 当前性 |
|---|---|---|
| `architecture/` | 当前项目状态、架构边界、代码根、运行入口、质量门禁和 dbt layer 模型语义 | 当前事实入口 |
| `ADR/` | 长期决策 | 当前或明确标注状态 |
| `RFC/` | 设计讨论和历史方案 | 当前已统一归档到 `archive/`；活跃 RFC 需重新放顶层并标注状态 |
| `plans/` | 仍需执行的计划 | 顶层只放 active plans |
| `plans/archive/` | 完成、废弃或被替代的计划 | 历史参考 |
| `issues/` | 技术债、实现漂移、质量扫描和治理建议入口 | 历史参考 |
| `jobs/` | runbook、snapshot 和运行报告入口 | 当前入口 |
| `jobs/reports/` | 实际运行事实 | 历史事实 |
| `releases/` | 集成 release note、版本 manifest schema 和 tag 前检查 | 当前入口 |
| `references/` | 外部接口、raw profile、data dict | 可查事实 |
| `Q&A/` | 设计问答、临时讨论和待升级为 RFC/ADR 的架构判断 | Proposed 或 Temporary |
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

历史文档不要作为当前事实引用。引用历史方案时，同时链接当前架构事实文档、代码、ADR 或运行报告作为证据。

## 最小校验

文档-only 变更至少运行：

```bash
make docs-check
git diff --check
```

涉及 Dagster、dbt、contracts 或 Rust 的事实更新时，追加对应领域校验命令。

版本治理或发布快照变更追加：

```bash
make versions-check
```
