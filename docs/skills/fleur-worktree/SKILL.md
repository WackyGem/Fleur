---
name: fleur-worktree
description: mono-fleur 的 Git worktree 与多分支协作技能。用于为 Codex/agent 并行任务创建、命名、隔离、验证、合并和清理 worktree；适用于多 agent 同仓库开发、分支实验、并行修复、代码评审和长期任务隔离。
---

# Fleur Worktree

当同一个 mono-fleur 仓库需要同时推进多个 Codex/agent 任务时，使用这个 skill。目标是一任务一 worktree、一任务一分支、一套明确验证和清理规则，避免 agent 互相踩文件、共享运行时状态或把未完成分支混入主工作区。

## 依据

- OpenAI Codex app 公开材料强调并行 agent、内置 worktrees、隔离环境和可审阅 diff。
- OpenAI harness engineering 文章提到让应用可按每个 Git worktree 启动，使 Codex 能为每个变更管理独立实例。
- Git 官方 `git-worktree` 文档定义了 `add`、`list`、`remove`、`prune` 等生命周期命令；清理 worktree 时优先用 Git 命令而不是直接删除目录。

## 原则

- 每个并行任务使用独立 worktree 和独立 branch。
- worktree 放在仓库外部的兄弟目录，不放进 `mono-fleur/` 内部。
- 不从脏工作区创建新任务，除非用户明确要求把当前未提交改动带过去。
- worktree 只隔离文件系统和 Git checkout，不隔离 S3、PostgreSQL、Dagster run storage、端口、后台进程或外部 API。
- 能串行就不伪装并行：两个任务会改同一模块、同一迁移、同一资产契约或同一运行状态时，指定 integrator 分支串行合并。
- 合并前必须在目标分支上重新验证；合并后必须清理 worktree 和 stale metadata。

## 布局

主仓库：

```text
/storage/program/mono-fleur
```

推荐 worktree 根目录：

```text
/storage/program/mono-fleur-worktrees/
```

命名规则：

- worktree 目录：`<yyyymmdd>-<short-task>`
- branch：`codex/<yyyymmdd>-<short-task>`
- slug 只用小写字母、数字和 `-`。

示例：

```text
/storage/program/mono-fleur-worktrees/20260531-scheduler-runner
branch: codex/20260531-scheduler-runner
```

## 创建流程

在主仓库或任意干净 worktree 中执行：

```bash
git status --short
git worktree list
git fetch --all --prune

mkdir -p ../mono-fleur-worktrees
git worktree add -b codex/20260531-topic ../mono-fleur-worktrees/20260531-topic HEAD
```

如果要基于远端主线创建，先确认默认分支名，再显式指定：

```bash
git branch --show-current
git worktree add -b codex/20260531-topic ../mono-fleur-worktrees/20260531-topic origin/main
```

进入新 worktree 后先同步依赖：

```bash
cd ../mono-fleur-worktrees/20260531-topic/pipeline
uv sync --all-packages --all-groups
```

## 环境隔离

`.env` 不提交。需要运行本地命令时，按任务选择：

- 只做文档、静态代码、单测：通常不需要复制 `.env`。
- 需要访问同一套外部 S3/PostgreSQL/API：可以复制或软链接 `.env`，但必须确认任务不会写入生产或共享状态。
- 需要并行运行 Dagster、服务或数据库相关任务：不要共享 `DAGSTER_HOME`、端口、临时目录或本地数据库 schema。

Dagster 运行建议：

- `dg check defs`、ruff、pyright、pytest 可在 feature worktree 内运行。
- 物化、回填、OCR、外部 API 写入类任务默认不要从普通 feature worktree 执行。
- 如果确实要在 worktree 中运行 Dagster，使用该 worktree 自己的 `.dagster`，并在命令前显式设置 `DAGSTER_HOME="$PWD/.dagster"`。
- 任何涉及真实 S3/PostgreSQL 写入的操作，先使用 `docs/skills/dg-backfill-runbook/SKILL.md` 判断是否应该切回主工作区或专用 ops worktree。

## 并行任务选择

适合并行：

- 文档和代码分属不同目录。
- 一个任务只改 `docs/`，另一个只改测试或单个 source bundle。
- 同一架构目标的多个候选实现，需要比较 diff 后择优。
- 独立 bugfix、数据字典更新、skill 维护。

不适合并行：

- 同时改 `AGENTS.md`、`docs/architecture/` 和同一套边界测试。
- 同时改同一个 Dagster asset、repository、migration 或 generated schema。
- 同时运行会写入同一 `.dagster`、数据库 schema、S3 prefix、端口或缓存目录的任务。
- 一个分支依赖另一个分支尚未合并的抽象。

遇到不适合并行的情况，建立 `codex/<topic>-integration` 或指定一个 owner worktree，其他分支只作为候选 patch 来源。

## 聚合分支策略

当多个分支会互相冲突、共享接口、修改同一架构边界或需要一起验证时，先创建聚合分支，不要让 `main` 逐个吸收冲突分支。

推荐命名：

- branch：`codex/integration-<yyyymmdd>-<topic>`
- worktree：`../mono-fleur-worktrees/integration-<yyyymmdd>-<topic>`

创建聚合 worktree：

```bash
git fetch --all --prune
mkdir -p ../mono-fleur-worktrees
git worktree add -b codex/integration-20260531-topic ../mono-fleur-worktrees/integration-20260531-topic origin/main
```

在聚合分支按顺序合并候选分支：

```bash
cd ../mono-fleur-worktrees/integration-20260531-topic
git merge codex/branch-a
git merge codex/branch-b
git merge codex/branch-c
```

合并顺序：

- 架构、接口、公共 helper、数据契约先合并。
- 叶子实现、文档、测试补充后合并。
- migration、generated schema、lockfile、运行报告最后单独处理。
- 每次解决冲突后运行与该冲突相关的最小验证，再继续合并下一个分支。

聚合分支验证通过后，再让目标主线合并聚合分支：

```bash
git checkout main
git merge --no-ff codex/integration-20260531-topic
```

不要在 `main` 上逐个解决同一批冲突。`main` 应只接收已经收敛、可验证、可回滚的聚合结果。

## 日常操作

查看所有 worktree：

```bash
git worktree list
```

在 worktree 内确认身份：

```bash
pwd
git branch --show-current
git status --short
```

把分支同步到最新主线：

```bash
git fetch --all --prune
git rebase origin/main
```

如果分支已经推送并协作，优先用 merge 或按团队规则处理 rebase，避免改写别人正在使用的历史。

## 验证门禁

按改动范围选择最小验证。完整 scheduler 变更使用：

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
git diff --check
```

每个 worktree 的最终汇报必须包含：branch、base、主要改动、验证命令、未验证项、是否有外部副作用。

## 合并流程

单个低冲突分支：

1. 在 worktree 内确认 `git status --short` 只包含本任务改动。
2. `git fetch --all --prune`，再 rebase/merge 最新目标分支。
3. 运行对应验证。
4. 推送分支并开 PR，或按用户要求把 diff 带回目标分支。
5. 合并后在目标分支再跑一次最小验证，不能只相信 feature worktree 的结果。

多个冲突分支：

1. 基于目标主线创建 `codex/integration-<yyyymmdd>-<topic>`。
2. 按冲突风险顺序把 feature branches 合入聚合分支。
3. 每次冲突解决后提交 merge，并运行相关最小验证。
4. 全部合并完成后，在聚合分支运行完整或足够覆盖的验证。
5. 由目标主线合并聚合分支；不要把冲突解决散落到 `main` 的多个 merge 中。
6. 合并后清理 feature worktree、integration worktree 和已合并分支。

## 清理流程

分支合并或废弃后：

```bash
git worktree list
git worktree remove ../mono-fleur-worktrees/20260531-topic
git worktree prune
git branch -d codex/20260531-topic
```

如果 worktree 有未提交内容，不要用 `-f`。先查看：

```bash
git -C ../mono-fleur-worktrees/20260531-topic status --short
git -C ../mono-fleur-worktrees/20260531-topic diff --stat
```

只有用户明确确认废弃时，才允许强制删除。

## 常见风险

- **同一 branch 不能被多个 worktree 同时 checkout**：为每个 agent 建独立 branch。
- **直接 `rm -rf` 会留下 stale metadata**：优先 `git worktree remove`，必要时再 `git worktree prune`。
- **依赖安装不是共享运行时**：每个 worktree 的 `.venv` 独立，uv cache 可共享。
- **外部状态不会自动隔离**：S3、PostgreSQL、Dagster storage、远端 API 写入需要任务级确认。
- **端口冲突很常见**：并行跑服务时为每个 worktree 配置端口或只允许一个服务型任务运行。
- **生成文件容易制造冲突**：generated schema、lockfile、迁移文件由 integrator 分支统一收敛。

## 什么时候更新本 skill

- 发现新的端口、`.env`、Dagster home、S3 prefix 或数据库隔离规则。
- 新增了自动创建/清理 worktree 的脚本。
- 多分支合并出现重复冲突，说明命名、任务拆分或验证门禁需要收紧。
- 团队决定使用 stack PR、merge queue、专用 integration 分支或远端 Codex cloud 工作流。
