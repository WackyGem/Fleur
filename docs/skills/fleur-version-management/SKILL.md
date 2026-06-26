---
name: fleur-version-management
description: mono-fleur 的多工程版本管理技能。用于判断和执行组件 SemVer bump、dataset contract version bump、Alembic revision/version 记录、Rust crate 独立版本维护、deploy release manifest、集成 tag、release note 和运行时版本暴露；适用于用户要求处理版本号、发布版本、组件 tag、集成 tag、release manifest、changelog/release note、运行时版本展示或版本一致性校验时。
---

# Fleur Version Management

使用此 skill 维护 mono-fleur 的版本事实和发布追溯。skill 只保留日常操作流程、决策规则、验证命令和停止条件。

## 先读事实

1. 先按变更范围读取系统地图：
   - `docs/systems/data-platform.md`
   - `docs/systems/data-governance.md`
   - `docs/systems/furnace.md`
   - `docs/systems/rearview.md`
   - `docs/systems/racingline.md`
   - `docs/systems/deploy-ops.md`
2. 用工具确认当前版本源，不凭记忆改版本：

```bash
rg -n "^(name|version|version\\.workspace|\\[workspace\\.package\\]|release:|components:|database_heads:)" \
  pipeline/pyproject.toml \
  pipeline/scheduler/pyproject.toml \
  pipeline/contract_tools/pyproject.toml \
  pipeline/elt/dbt_project.yml \
  engines/Cargo.toml \
  engines/crates/*/Cargo.toml \
  app/racingline/package.json \
  deploy/release-manifest.yml 2>/dev/null

rg -n "^version:" pipeline/contracts/datasets
rg -n "revision =|down_revision =" pipeline/migrate/versions
```

如果用户要求打 tag，先读：

```bash
git status --short
git tag --list | sort
git rev-parse --short HEAD
```

不要在脏工作区里创建 tag，除非用户明确要求 tag 指向当前脏工作区对应的已提交 commit；Git tag 只能指向 commit，不能包含未提交改动。

## 版本主体

| 主体 | 版本源 | 类型 |
|---|---|---|
| `pipeline` workspace meta | `pipeline/pyproject.toml` | 仅 meta package SemVer |
| `scheduler` | `pipeline/scheduler/pyproject.toml` | Component SemVer |
| `contract-tools` | `pipeline/contract_tools/pyproject.toml` | Component SemVer |
| `contracts` datasets | `pipeline/contracts/datasets/*.yml` | Dataset integer version |
| `elt` | `pipeline/elt/dbt_project.yml` | Component SemVer |
| `migrate` | `pipeline/migrate/versions/**` | Alembic revision |
| Rust crates | `engines/crates/*/Cargo.toml` | Component SemVer |
| `racingline` | `app/racingline/package.json` | Component SemVer |
| deploy snapshot | `deploy/release-manifest.yml` | Integration manifest |

`pipeline` root version 不代表数据平台整体版本。数据平台状态由 `scheduler`、`contract-tools`、`elt`、dataset contract versions 和 Alembic heads 共同表达。

## 判断 bump

先确定影响主体，再确定 bump 级别。

### Component SemVer

`MAJOR.MINOR.PATCH`：

- `MAJOR`：进入 `1.0.0` 后用于破坏性 API/CLI/HTTP/dbt model/deploy contract 变化。
- `MINOR`：新增向后兼容能力、字段、命令、模型或可选配置。
- `PATCH`：bug fix、性能优化、内部重构、文档修正，不改变外部 contract。

当前 `0.x` 阶段：

- 破坏性变化 bump `MINOR`。
- 兼容修复 bump `PATCH`。
- 新增兼容能力也 bump `MINOR`。

### Dataset contract version

只 bump 被修改 dataset 的整数 `version`。

必须 bump：

- source/raw/parquet/clickhouse_raw 字段集合变化。
- 字段类型、nullable、required 变化。
- grain、partition、raw table contract 变化。

不 bump：

- 只改中文描述、validation notes、拼写、排序。
- 只改 dbt staging 字段描述；staging 属于 `pipeline/elt`。

### Alembic revision

迁移不使用 SemVer。新增 schema 变化时新增 Alembic revision，并在 release manifest 记录 target database 的 head。

不要为了版本号新增 `pipeline/migrate/pyproject.toml`。

### 跨组件变更

- 生产者破坏性变化：生产者 bump 破坏性版本，消费者同一 PR 或后续兼容 PR bump。
- 生产者新增兼容字段：生产者 bump minor；消费者只有开始依赖该字段时才 bump。
- 消费者只适配既有 contract：只 bump 消费者。

## 编辑规则

### Python/uv

修改：

- `pipeline/scheduler/pyproject.toml`
- `pipeline/contract_tools/pyproject.toml`
- 必要时才修改 `pipeline/pyproject.toml`

保持 `pipeline/pyproject.toml` 版本保守：只有 Python baseline、workspace 成员拓扑、共享依赖策略或 root meta package 行为变化时才 bump。

### dbt

修改 `pipeline/elt/dbt_project.yml` 的 `version`。

公开 model、column、grain、materialization 或 mart 指标口径变更时 bump。新增兼容 model/nullable column/tests/docs 时 bump minor。只改生成的 `sources.yml` 不单独 bump `elt`，除非 dbt 公开语义也变化。

### Rust

目标态是每个 crate 显式维护自己的 version：

```toml
[package]
name = "rearview-server"
version = "0.1.0"
```

不要继续依赖：

```toml
version.workspace = true
```

`engines/Cargo.toml` 可以保留 `edition`、`license`、`publish` 等 workspace 共享元数据。当前内部 path dependency 不加 version constraint；只有发布 crate 或私有 registry 制品时才补：

```toml
rearview-core = { version = "0.2.0", path = "../rearview-core" }
```

### npm/Racingline

修改 `app/racingline/package.json` 的 `version`。不要手工改 `package-lock.json` 里的派生版本；如 package metadata 需要同步，使用 npm 命令或让锁文件由 npm 生成。

### Release manifest

如果涉及部署快照，新增或更新：

```text
deploy/release-manifest.yml
```

至少记录：

```yaml
release: mono-fleur-2026.06.1
components:
  scheduler: 0.1.0
  contract-tools: 0.1.0
  elt: 1.0.0
  furnace: 0.1.0
  rearview-server: 0.1.0
  rearview-portfolio-worker: 0.1.0
  racingline: 0.0.1
database_heads:
  pipeline: 0008_strategy_portfolio_cp
  rearview: 0008_strategy_portfolio_cp
target_schema_heads:
  pipeline: 0001_jiuyan_industry_images
  rearview: 0008_strategy_portfolio_cp
contracts:
  registry_commit: <git-sha>
  changed_datasets: []
```

清单只记录一次部署快照，不替代源文件中的组件版本。

当前 Alembic revision graph 是全局线性链，`database_heads` 记录 target 数据库实际 revision head；`target_schema_heads` 记录该 target 最后一个会执行 DDL 的 migration。

### Release note

优先维护集成 release note，不为每个组件提前拆 changelog。建议路径：

```text
docs/releases/README.md
docs/releases/mono-fleur-2026.06.1.md
```

release note 至少包含：

- 集成 tag。
- Git commit。
- 组件版本清单。
- Alembic head。
- dbt project version。
- contract registry 变更摘要。
- 验证命令和结果。

只有当某组件高频独立发布、对外提供稳定 API 或被其他仓库消费时，才新增 `docs/releases/components/<component>.md`。

## Tag 规则

组件 tag：

```text
scheduler-v0.2.0
contract-tools-v0.2.0
elt-v1.1.0
furnace-v0.2.0
rearview-server-v0.2.0
racingline-v0.1.0
```

集成 tag：

```text
mono-fleur-YYYY.MM.N
```

集成 tag 不使用 SemVer，因为它表达整仓验收快照，不表达公共 API 兼容性。

打 tag 前确认：

1. 工作区干净或用户明确指定已提交 commit。
2. 版本源文件已经提交。
3. 对应验证命令已通过。
4. tag 不与既有 tag 重名。

## 运行时版本暴露

只有可执行或可部署组件需要运行时版本入口：

| 组件 | 最低要求 |
|---|---|
| `furnace` | `furnace --version`，JSON summary 可选带 `engine_version` |
| `rearview-server` | `rearview-server --version`，`GET /rearview/version` 或 health payload |
| `rearview-portfolio-worker` | `rearview-portfolio-worker --version`，启动日志记录 version |
| `racingline` | build metadata：app version、git sha、build time |
| `scheduler` | Dagster resource/run metadata 记录 scheduler package version |
| `contract-tools` | `fleur-contracts --version` |

库 crate 不需要单独运行时入口；通过调用它们的 binary/service 进入日志、summary 或 release manifest。

## 验证

文档-only 或 skill 变更：

```bash
make docs-check
git diff --check
```

版本源、release manifest 或运行时版本暴露变更：

```bash
make versions-check
```

Python/scheduler/contract tools 版本或代码变更：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests
```

dbt 变更：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
```

contract 变更：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```

Rust 变更：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Racingline 变更：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

## 停止条件

停止并说明阻塞，不要猜测：

- 无法确认版本源文件。
- 无法确认变更影响的生产者/消费者边界。
- 要打 tag 但目标 commit、版本 bump 或验证状态不清楚。
- release manifest 中的组件版本和源文件不一致。
- 用户要求的版本策略与本 skill 的版本规则冲突。
