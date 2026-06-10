---
name: fleur-harness
description: mono-fleur 的项目 harness 工程技能。用于把复杂开发、重构、质量治理、文档维护和 agent 可读性工作落到仓库内的 docs、测试、脚本和质量门禁中；尤其适用于需要整理 AGENTS.md、ADR/RFC/plan/optimize/job report、架构边界或长期维护规则的任务。
---

# Fleur Harness

当任务不只是改一段代码，而是要提升 mono-fleur 的可维护性、agent 可读性、架构约束或长期执行可靠性时，使用这个 skill。

核心原则：`AGENTS.md` 是地图，`docs/` 是记录系统，测试和脚本是可执行约束。不要把长期规则只留在一次性提示或聊天上下文里。

## 适用场景

- 复杂重构、跨模块改动、架构边界调整。
- 新增数据源、Dagster asset、dbt 模型、迁移或长期运行任务。
- 维护 `AGENTS.md`、ADR、RFC、plan、optimize 文档、job report 或 repo skill。
- 把重复出现的问题沉淀为测试、脚本、文档规则或专用 skill。
- 检查文档是否过期、重复、互相冲突或没有对应机械验证。

## 工作循环

1. 先读地图：从 `AGENTS.md` 和相关 `docs/` 入口确认项目结构、命令、边界和质量门禁。
2. 再读事实：用 `rg`/`find` 查真实代码、测试、现有文档和最近报告，不用旧文档替代当前代码。
3. 明确意图：小变更可直接执行；跨阶段或高风险工作应维护 `docs/plans/` 或 `docs/optimize/` 中的执行计划。
4. 编码约束：反复出现的 review 意见或架构偏好，优先沉淀为测试、脚本、skill 或 AGENTS/docs 路由。
5. 验证闭环：按改动范围运行最小可证明的检查，并在文档中记录无法验证的原因。
6. 清理熵增：发现过期文档、废弃计划、重复规则或源码噪声时，随任务一并收敛，避免继续复制坏模式。

## 文档路由

- `docs/README.md`：docs 总入口，说明当前事实、决策、计划、运行记录、设计和 references 的阅读顺序。
- `AGENTS.md`：repo 入口地图，只放稳定目录、命令、边界和 skill 路由；不要扩展成百科。
- `docs/architecture/`：当前架构边界和禁止模式；改目录职责或依赖方向时同步维护。
- `docs/ADR/`：已经接受的长期决策；当行为应长期稳定且影响后续设计时新增或更新。
- `docs/RFC/`：方案讨论和历史设计；实现完成或废弃后移动到 `archive/` 或在文档中标明状态。
- `docs/plans/`：复杂执行计划、阶段拆分、验收标准；活跃计划保留在顶层，完成后归档；新增、归档或改名 active plan 后同步维护 `docs/plans/README.md`。
- `docs/optimize/`：质量扫描、可维护性审计、下一阶段治理建议。
- `docs/jobs/reports/`：实际运行、回填、重跑和数据核验记录；必须包含命令、时间、范围和结果。
- `docs/design/`：dbt layer 和模型设计文档，记录模型语义、字段边界和对应 SQL/YAML。
- `docs/references/`：远端接口、OpenAPI、数据字典、样例图片、服务器协议等可查事实。
- `docs/skills/`：可复用 agent 操作手册；只保留 `SKILL.md` 和必要 references/scripts/assets。

## 文档地图阅读顺序

处理 harness、架构、长期维护或文档治理任务时，按下面顺序读文档；只在任务相关时深入，不要无差别通读整个 `docs/`。

1. `AGENTS.md`：确认项目入口、命令约束、质量门禁、MCP 路由和 skill 路由。
2. `docs/README.md`：确认 docs 当前入口、目录职责、状态模型和归档规则。
3. `docs/architecture/scheduler-architecture.md`：确认当前 scheduler 总体架构和主要数据流。
4. `docs/architecture/scheduler-module-boundaries.md`：确认模块职责、依赖方向和禁止模式。
5. `docs/ADR/README.md`，再读相关 `docs/ADR/*.md`：确认已经接受且仍应遵守的长期决策。
6. 活跃设计与执行文档：先读相关 `docs/RFC/*.md`，再读相关 `docs/plans/*.md` 和 `docs/optimize/*.md`，确认目标、非目标、阶段和验收标准。
7. 运行事实：涉及回填、重跑、数据核验或生产结果时，读 `docs/jobs/README.md`、`docs/jobs/dagster-backfill-2026.md` 和相关 `docs/jobs/reports/*.md`。
8. 接口与数据事实：涉及远端 API、schema、样例或数据契约时，读相关 `docs/references/README.md`、`docs/references/openapi/`、`docs/references/remote_endpoint/`、`docs/references/data_dict/`、`docs/references/remote_server/`。
9. 复用手册：涉及具体 agent 操作时，读相关 `docs/skills/*/SKILL.md`，并遵守其中更具体的 runbook。
10. 历史材料：`docs/RFC/archive/` 和 `docs/plans/archive/` 只用于追溯背景；不得把归档内容当作当前事实，除非当前代码或活跃文档再次确认。

读完文档后必须回到代码和测试验证事实：用 `rg`/`find` 定位当前实现，用最小质量门禁确认文档没有脱离代码。

## mono-fleur 约束

- Python、dbt、Dagster、`dg` 命令在 `pipeline/` 下用 `uv run` 执行。
- Dagster 任务先使用 `dagster-expert`；dbt 任务使用对应 dbt skills。
- scheduler 边界以 `pipeline/scheduler/src/scheduler/defs/` 的职责分层为准。
- 新数据源应通过自己的 `definitions.py` 暴露 `SourceBundle`，再由顶层 `SOURCE_BUNDLES` 显式聚合。
- 数据源业务代码通过 resources、factory、gateway 注入外部能力，不直接读取环境变量或构造底层 client。
- 资产 lineage 只表达真实数据依赖；限流、批量、重试和回填策略用执行策略、schedule、pool、runner 或 runbook 表达。
- 所有长期资产契约要保留 owner、kind、source/layer/storage/state/modality tags 和可核验 metadata。

## 计划写法

复杂计划应包含：

- 目标和非目标。
- 关联设计文档、ADR、RFC 或 optimize 文档。
- 当前事实基线，不复述未经验证的历史假设。
- 阶段拆分和每阶段完成标准。
- 禁止模式和允许保留的例外。
- 最小验证命令。
- 完成后的归档或后续维护动作。

如果计划会修改生产代码，每个阶段都要同步说明测试策略。不要把“最后补测试”作为默认路径。

## 文档维护规则

- 文档应链接到真实代码路径、命令或报告；避免只有抽象口号。
- 更新规则时，检查是否也需要更新 `AGENTS.md`、架构文档、相关 skill 或测试。
- 同一规则只保留一个权威位置，其他位置使用简短指针。
- 如果代码已经否定旧文档，优先修文档；如果文档表达的是仍应遵守的约束，优先补测试。
- 历史文档不要伪装成当前状态：归档、标注日期或写明“当前实现已变化”。

## 机械验证

按改动范围选择最小检查：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

文档-only 变更至少运行：

```bash
make docs-check
git diff --check
```

涉及回填或 Dagster 运行时命令时，优先使用 `docs/skills/dg-backfill-runbook/SKILL.md`。

## 升级规则

当发现下面任一情况，不要只在回复里解释，应考虑把规则写回仓库：

- 同类问题第二次出现。
- 需要 agent 每次都记住的边界或偏好。
- review 意见能被静态测试、脚本或文档结构检查表达。
- 远端接口、数据契约或运行手册有新事实。
- 计划执行结果产生了可复用命令、失败样本或核验方法。

优先级：测试/脚本 > skill > 架构文档/ADR > AGENTS 指针。`AGENTS.md` 只放入口和稳定路由。
