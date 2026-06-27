# Plan 0033: docs governance implementation plan

日期：2026-06-10

状态：Archived

归档日期：2026-06-10

归档原因：Completed

完成日期：2026-06-10

实际验证：

- `make docs-check`
- `git diff --check`

关联文档：

- `AGENTS.md`
- `docs/skills/fleur-harness/SKILL.md`
- `docs/architecture/scheduler-architecture.md`
- `docs/architecture/scheduler-module-boundaries.md`
- `docs/ADR/README.md`
- `docs/jobs/dagster-definitions-lineage-2026-06-10.md`

相关背景：

- 当前仓库已经把 `docs/` 作为记录系统，但文档规模增长后，入口、状态、归档和机械验证没有同等增长。
- `AGENTS.md` 提供了目录地图，但 `docs/` 下还没有面向人和 agent 的总索引。
- `docs/plans/` 顶层包含多个已完成计划，`docs/plans/archive/` 又保留历史计划；顶层“活跃计划”和“历史记录”的边界不够硬。
- `docs/RFC/` 与 `docs/RFC/archive/` 并存，部分实现完成后的 RFC、plan、job report、optimize 文档之间需要更明确的引用方向。
- `docs/jobs/` 同时包含回填 runbook、运行报告目录和 definitions snapshot，需要把“操作手册”和“运行事实”区分清楚。
- `docs/architecture/` 已存在 dbt layer 设计文档，但不在 `AGENTS.md` 当前文档入口列表中，发现成本偏高。
- 计划编号已经出现过重号，例如 `docs/plans/0030-*` 有两个文件，说明文档命名缺少轻量校验。

## 1. 目标

把 `docs/` 从“能记录很多内容”治理成“能快速找到当前事实、能判断历史材料状态、能用命令验证关键约束”的记录系统。

完成后应满足：

1. 每个文档目录都有清晰职责、入口和归档规则。
2. 当前事实、长期约束、执行计划、运行结果、历史方案彼此有明确边界。
3. `docs/plans/` 顶层只保留 `Proposed`、`In Progress`、`Blocked` 等仍需行动的计划；完成或废弃计划归档。
4. 所有 active plan 有统一状态字段、关联文档、目标、非目标、阶段、完成标准和验证命令。
5. `docs/README.md` 成为 `docs/` 的人类/agent 总入口，`AGENTS.md` 只保留稳定目录地图和命令入口。
6. `docs/jobs/` 区分 runbook、snapshot 和实际运行报告；运行报告继续落 `docs/jobs/reports/`。
7. 旧文档不会伪装成当前事实：归档文档必须位于 archive 目录，或在文首标明当前状态。
8. 提供最小机械校验，至少覆盖编号重复、顶层 plan 状态、文档路由、尾随空白和坏链接的可检测部分。

## 2. 非目标

本计划不做以下事情：

1. 不重写所有历史文档内容。
2. 不删除有效历史材料；历史材料应归档和标注，而不是直接丢弃。
3. 不把 `AGENTS.md` 扩展成百科；它仍只做项目入口和稳定路由。
4. 不引入外部文档站点、知识库或数据目录平台。
5. 不要求一次性给所有历史文档补齐严格 front matter。
6. 不把运行报告改造成计划文档；运行报告仍以命令、时间、范围、结果为核心。
7. 不为了统一格式破坏已有高价值上下文；治理优先解决发现成本和当前事实冲突。

## 3. 治理原则

### 3.1 单一事实入口

同一类事实只保留一个权威入口：

| 事实类型 | 权威位置 | 说明 |
|---|---|---|
| 当前架构边界 | `docs/architecture/` | 当前实现和禁止模式 |
| 长期决策 | `docs/ADR/` | Accepted / Proposed / Superseded |
| 仍需执行的计划 | `docs/plans/` 顶层 | 只放活跃或待执行计划 |
| 历史计划 | `docs/plans/archive/` | 完成、废弃或被替代的计划 |
| 当前运行手册 | `docs/skills/` 或 `docs/jobs/*.md` | 可复用操作步骤 |
| 实际运行事实 | `docs/jobs/reports/` | 命令、时间、范围、结果 |
| 接口和数据事实 | `docs/references/` | OpenAPI、remote endpoint、raw profile、data dict |
| dbt 模型设计 | `docs/architecture/dbt_layer/` | 每个模型的设计和字段语义 |
| 质量审计和治理建议 | `docs/issues/archive/optimize/` | 质量扫描、后续优化方向 |

其他文档引用事实时，应链接到权威位置，不复制长段规则。

### 3.2 当前事实优先

文档不能只凭历史计划判断当前状态。涉及代码、资产、模型、contract、运行命令时，必须链接或记录当前验证证据，例如：

- `uv run dg list defs --target-path scheduler --json`
- `uv run dg check defs --target-path scheduler`
- `uv run dbt parse --project-dir elt --profiles-dir elt`
- `uv run fleur-contracts validate`
- `cargo test --workspace`
- 具体 job report 或 generated artifact

### 3.3 历史材料显式降级

历史 RFC、完成计划、旧优化建议和一次性运行记录不能留在顶层伪装成当前任务。归档后文首保留：

```text
状态：Archived
归档日期：YYYY-MM-DD
当前入口：<link>
归档原因：Completed | Superseded | Obsolete | Historical reference
```

### 3.4 可机械验证优先

能用脚本验证的规则不要只写在文档里。第一版 docs governance validator 只做低风险检查：

1. `docs/plans/` 顶层编号不重复。
2. 顶层 plan 必须包含 `状态：`。
3. `状态：Completed` 或 `状态：Superseded` 的顶层 plan 必须提示应归档。
4. `docs/jobs/reports/` 报告必须包含命令、时间/日期、范围和结果关键词。
5. Markdown 文件无尾随空白。
6. 本地相对链接能解析到存在的文件。

## 4. 目标结构

第一阶段不强行改变目录名，只补入口和规则。完成后目标结构如下：

```text
docs/
  README.md                         # docs 总入口和路由
  architecture/                     # 当前架构事实
  ADR/                              # 长期决策
  RFC/
    archive/                        # 历史方案
  plans/
    archive/                        # 历史计划
  optimize/                         # 质量审计和治理建议
  jobs/
    reports/                        # 运行事实
    dagster-backfill-2026.md        # 当前回填 runbook
    dagster-definitions-lineage-*.md# 当前 definitions snapshot
  design/
    dbt_layer/                      # dbt 模型设计
  references/                       # 接口、raw profile、data dict
  skills/                           # agent runbook
```

需要补齐：

1. `docs/README.md`
2. `docs/plans/README.md`
3. `docs/jobs/README.md`
4. `docs/architecture/README.md`
5. `docs/references/README.md`
6. `docs/issues/README.md`

## 5. 文档状态模型

文档状态统一使用中文标签，便于人工扫描：

| 状态 | 含义 | 允许位置 |
|---|---|---|
| `Proposed` | 已提出但尚未执行 | `docs/plans/`、`docs/RFC/`、`docs/ADR/` |
| `In Progress` | 正在执行 | `docs/plans/` |
| `Blocked` | 因明确外部条件暂停 | `docs/plans/` |
| `Completed` | 已完成并有验证证据 | 归档前可短暂停留在 `docs/plans/`，随后移入 archive |
| `Accepted` | 长期决策已接受 | `docs/ADR/` |
| `Superseded` | 被新文档替代 | archive 或文首标注 |
| `Archived` | 历史参考 | archive |

计划文档最低模板：

```text
# Plan NNNN: <title>

日期：YYYY-MM-DD
状态：Proposed | In Progress | Blocked | Completed | Superseded

关联文档：
- ...

## 1. 目标
## 2. 非目标
## 3. 当前事实基线
## 4. 实施阶段
## 5. 禁止模式和例外
## 6. 验证命令
## 7. 完成和归档标准
```

## 6. 实施阶段

### Phase 0: 文档资产盘点

范围：

- `docs/`
- `AGENTS.md`
- `engines/README.md`

动作：

1. 生成 docs inventory，按目录统计文件数、最近修改时间和状态字段。
2. 列出没有被任何入口引用的顶层目录，例如当前 `docs/architecture/`。
3. 检查 `docs/plans/` 顶层状态，标记已完成但未归档的计划。
4. 检查编号重复，先记录不改名，避免破坏历史链接。
5. 检查 `docs/jobs/` 中 runbook、snapshot、report 的混放点。

完成标准：

1. 形成一份 inventory 表，记录到本计划的执行报告或 `docs/issues/archive/optimize/`。
2. 所有待归档、待补入口、待加状态的文档有清单。

验证：

```bash
find docs -type f | sort
rg -n "^状态：|^状态:" docs
find docs/plans -maxdepth 1 -type f | sort
```

### Phase 1: 建立 docs 总入口和目录 README

范围：

- `docs/README.md`
- `docs/plans/README.md`
- `docs/jobs/README.md`
- `docs/architecture/README.md`
- `docs/references/README.md`
- `docs/issues/README.md`
- `AGENTS.md`

动作：

1. 新增 `docs/README.md`，作为人类和 agent 的 docs 总入口。
2. 在 `docs/README.md` 中明确“先看当前事实，再看历史材料”的阅读顺序。
3. 为 `plans/jobs/design/references/optimize` 添加简短 README，说明目录职责、命名规则和归档规则。
4. 更新 `AGENTS.md` 文档入口，补充 `docs/architecture/`，并把细节指向 `docs/README.md`。
5. 不在 `AGENTS.md` 中复制目录 README 的详细规则。

完成标准：

1. 新人能从 `docs/README.md` 判断该看 architecture、ADR、plans、jobs、references 还是 design。
2. `AGENTS.md` 仍保持短地图角色。
3. `docs/architecture/` 不再是隐性目录。

验证：

```bash
rg -n "docs/architecture|docs/README|docs/plans|docs/jobs" AGENTS.md docs/README.md docs/*/README.md
git diff --check
```

### Phase 2: 活跃计划归档和状态收敛

范围：

- `docs/plans/`
- `docs/plans/archive/`

动作：

1. 给所有顶层 plan 补齐 `状态：`。
2. 将 `Completed`、明确已执行完毕或已被替代的 plan 移入 `docs/plans/archive/`。
3. 对重号计划不做静默改名；如果需要改名，新增迁移说明并检查引用。
4. 顶层只保留仍需执行的计划，例如 `Proposed`、`In Progress`、`Blocked`。
5. 在 `docs/plans/README.md` 中维护 active plan 索引。

完成标准：

1. `docs/plans/` 顶层没有 `Completed` 状态计划。
2. 每个 active plan 有明确下一步或阻塞条件。
3. 历史计划仍可通过 archive 找到。

验证：

```bash
rg -n "^状态：Completed|^状态：Superseded" docs/plans/*.md
find docs/plans -maxdepth 1 -type f | sort
find docs/plans/archive -maxdepth 1 -type f | sort
```

### Phase 3: RFC、optimize、jobs 的当前/历史边界

范围：

- `docs/RFC/`
- `docs/RFC/archive/`
- `docs/issues/archive/optimize/`
- `docs/jobs/`
- `docs/jobs/reports/`

动作：

1. 对活跃 RFC 标注当前状态：仍在指导实现、已完成、已被 ADR 接收、或仅历史参考。
2. 已被 ADR 或 plan 完成吸收的 RFC 移入 archive，文首保留当前入口链接。
3. `docs/issues/archive/optimize/` 保留质量审计和治理建议，已转为执行计划的文档要指向对应 plan。
4. `docs/jobs/README.md` 区分：
   - runbook：可复用操作步骤。
   - snapshot：某一时点的 definitions、lineage、状态盘点。
   - reports：实际运行结果。
5. 运行报告必须继续包含命令、时间、范围和结果，不改造成泛化说明。

完成标准：

1. 读者能判断每个 RFC/optimize/jobs 顶层文档是否代表当前建议。
2. `docs/jobs/reports/` 中报告不承担 runbook 角色。
3. snapshot 文档标明生成命令和生成日期。

验证：

```bash
rg -n "^状态：|^记录日期：|^日期：" docs/RFC docs/issues docs/jobs
rg -n "命令|范围|结果" docs/jobs/reports
git diff --check
```

### Phase 4: 引入 docs governance validator

范围：

- `scripts/validate_docs_governance.py` 或 `pipeline/contract_tools` 外的轻量脚本
- `Makefile`
- `docs/README.md`
- `docs/plans/README.md`

动作：

1. 新增轻量 Python 脚本，默认只依赖标准库。
2. 检查 plan 编号重复。
3. 检查顶层 active plan 状态。
4. 检查 Markdown 尾随空白。
5. 检查相对链接文件存在。
6. 可选检查 `docs/jobs/reports/*.md` 是否包含命令、日期/时间、范围和结果关键词。
7. 在 `Makefile` 中新增 `docs-check` 目标，运行该脚本。

完成标准：

1. `make docs-check` 可在仓库根目录运行。
2. 文档治理不依赖外部服务或完整 pipeline 环境。
3. 校验失败信息能直接指向文件和规则。

验证：

```bash
make docs-check
git diff --check
```

### Phase 5: 文档变更纳入日常开发

范围：

- `AGENTS.md`
- `docs/skills/fleur-harness/SKILL.md`
- 相关 runbook

动作：

1. 把文档-only 变更的最小验证更新为：

```bash
make docs-check
git diff --check
```

2. 对涉及 Dagster/dbt/contract/Rust 的文档，保留对应领域验证命令。
3. 在 `fleur-harness` 中补充：新增 active plan 后必须更新 `docs/plans/README.md`。
4. 在 review/checklist 中加入“当前事实入口是否唯一”“历史材料是否归档”两项。

完成标准：

1. 后续新增计划、RFC、job report 有明确落点和校验。
2. 文档治理规则不再只依赖人工记忆。

验证：

```bash
make docs-check
git diff --check
```

## 7. 禁止模式和允许例外

禁止模式：

1. 不把新长期规则只写在聊天回复里。
2. 不在 `AGENTS.md` 复制长篇文档规范。
3. 不让已完成 plan 长期留在 `docs/plans/` 顶层。
4. 不把历史 RFC 当作当前事实引用，除非同时有当前代码或 ADR 证明。
5. 不新增无状态、无日期、无验证命令的执行计划。
6. 不用“TODO 文档”替代可执行计划。
7. 不把运行报告写成营销式总结；必须保留命令、范围和结果。

允许例外：

1. 大型迁移完成当天，`Completed` plan 可以短暂停留在顶层，等待同一 PR 归档。
2. 历史文档可以保留原格式，只要 archive 入口或文首状态说明足够清楚。
3. 一次性调查笔记可以先落 `docs/issues/archive/optimize/`，但若产生执行动作，应升级为 plan。
4. 外部接口样例、图片和生成数据字典不强制使用 plan 模板。

## 8. 验证命令

本计划本身的文档-only 验证：

```bash
git diff --check
```

执行 Phase 4 后，文档治理标准验证：

```bash
make docs-check
git diff --check
```

涉及领域事实的文档必须追加领域验证：

```bash
cd pipeline
uv run dg check defs --target-path scheduler
uv run dbt parse --project-dir elt --profiles-dir elt
uv run fleur-contracts validate
```

Rust 相关文档追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 9. 完成和归档标准

本计划完成条件：

1. `docs/README.md` 和关键目录 README 已建立。
2. `docs/plans/` 顶层只保留 active plans。
3. 完成、废弃、被替代的计划和 RFC 已归档或显式标注状态。
4. `make docs-check` 已存在并通过。
5. `AGENTS.md` 和 `fleur-harness` 指向新的 docs governance 入口。
6. 至少一次执行 docs governance validator，并在最终报告中记录结果。

完成后动作：

1. 将本计划状态改为 `Completed`，记录完成日期和验证命令。
2. 把本计划移入 `docs/plans/archive/`。
3. 若执行中形成长期稳定规则，必要时新增 ADR 或更新 `docs/skills/fleur-harness/SKILL.md`。
