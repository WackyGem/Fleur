# Plan 0059: 版本信息治理实施计划

日期：2026-06-26

状态：Completed

## 背景

[RFC 0033](../../RFC/0033-project-version-management.md) 已经提出 mono-fleur 的版本治理方案：采用“组件独立版本 + 仓库集成发布标记”，把 Component SemVer、dataset contract version、Alembic revision、deploy release manifest、集成 tag 和运行时版本暴露分开维护。

当前仓库已经具备部分版本事实：

| 主体 | 当前版本源 | 当前事实 |
|---|---|---|
| `pipeline` workspace meta | `pipeline/pyproject.toml` | `version = "0.1.0"` |
| `scheduler` | `pipeline/scheduler/pyproject.toml` | `version = "0.1.0"` |
| `contract-tools` | `pipeline/contract_tools/pyproject.toml` | `version = "0.1.0"` |
| `elt` | `pipeline/elt/dbt_project.yml` | `version: '1.0.0'` |
| dataset contracts | `pipeline/contracts/datasets/*.yml` | 每个 dataset 已有整数 `version` |
| Alembic migrations | `pipeline/migrate/versions/**` | 以 revision chain 表达 schema 版本 |
| Rust engines | `engines/Cargo.toml` + `engines/crates/*/Cargo.toml` | workspace 统一 `0.1.0`，crate 使用 `version.workspace = true` |
| Racingline | `app/racingline/package.json` | `version: "0.0.1"` |
| Deploy snapshot | `deploy/` | 尚无 `deploy/release-manifest.yml` |

本计划只安排后续实施步骤，不在本计划中直接修改组件版本号、打 tag 或发布 release。

## 目标

1. 把 RFC 0033 的版本治理方案拆成可执行、可验证、可回滚的阶段。
2. 建立当前组件版本、dataset contract versions、Alembic heads 和集成发布快照的统一记录方式。
3. 让 Rust crates 从 workspace 统一版本迁移为 crate 独立版本。
4. 新增 deploy release manifest 和统一 release note 机制，用于回答“一次部署包含哪些组件版本”。
5. 新增机械校验，防止版本字段、release manifest 和源文件长期漂移。
6. 为可执行/可部署组件补充运行时版本暴露。
7. 在实施前 review 并补齐 RFC 0033 未完全展开的信息缺口。

## 非目标

1. 本计划不直接执行组件版本 bump。
2. 本计划不创建任何 Git tag。
3. 本计划不引入外部包仓库、私有 registry、制品仓库或发布平台。
4. 本计划不改变 Python、Cargo、npm、dbt、Alembic 的包管理器。
5. 本计划不把 `pipeline/pyproject.toml` 的版本升级为“数据平台整体版本”。
6. 本计划不为每个组件立即创建独立 changelog；第一阶段只维护统一 release note。

## 关联文档

| 文档 | 用途 |
|---|---|
| [RFC 0033](../../RFC/0033-project-version-management.md) | 多工程版本管理方案、版本主体矩阵和后续实施优先级 |
| [fleur-version-management skill](../../skills/fleur-version-management/SKILL.md) | 后续执行版本 bump、manifest、tag、release note 和运行时版本暴露时的操作手册 |
| [docs/README.md](../../README.md) | docs 生命周期和最小文档校验 |
| [systems/README.md](../../systems/README.md) | 系统地图入口 |
| [deploy-ops 系统地图](../../systems/deploy-ops.md) | 部署、迁移和运行记录入口 |
| [furnace 系统地图](../../systems/furnace.md) | Furnace Rust CLI/crate 边界 |
| [rearview 系统地图](../../systems/rearview.md) | Rearview server/worker/crate 边界 |
| [racingline 系统地图](../../systems/racingline.md) | Racingline frontend 版本和质量门禁 |
| [data-platform 系统地图](../../systems/data-platform.md) | scheduler/dbt/migrate 边界 |
| [data-governance 系统地图](../../systems/data-governance.md) | contracts 和 contract-tools 边界 |

## Review 信息缺口

以下缺口必须在对应阶段补齐，不应在实现时用猜测或多路 fallback 绕过。

| 缺口 | 当前风险 | 补充要求 |
|---|---|---|
| `deploy/release-manifest.yml` 的 schema 未固化 | 后续 release manifest 可能只变成手写说明，无法机械校验 | Phase 2 先定义 YAML 字段、必填项、可选项和排序规则，再创建首个 manifest |
| Alembic head 获取方式未机械化 | 人工填 head 容易填错，尤其 `pipeline` 与 `rearview` 分支迁移并存 | Phase 2/3 明确用 `uv run alembic -c migrate/alembic.ini -x target=<target> heads` 或等价只读命令采集 head |
| Rust crate version 迁移影响 `Cargo.lock` 未确认 | 改 `Cargo.toml` version 可能导致 lockfile 元数据变化，漏提交会造成构建漂移 | Phase 1 必须运行 `cargo metadata` 并检查 `Cargo.lock` 是否需要随版本字段更新 |
| npm version 与 lockfile 同步策略未定 | 手工改 `package.json` 可能使 `package-lock.json` root package version 漂移 | Phase 3 校验脚本必须检查 `app/racingline/package.json` 与 `package-lock.json` root package version 是否一致；若需修改，使用 npm 生成锁文件 |
| release note 目录职责未纳入 docs 入口 | 新增 `docs/releases/` 后如果 docs 索引不挂入口，会成为孤立目录 | Phase 2 需要同步 `docs/README.md` 或相关系统地图中的入口 |
| 组件 tag 与集成 tag 的创建人/时机未写入流程 | 未验证就打 tag 会把错误快照固化 | Phase 2 release note 模板必须包含验证状态；Phase 4 tag runbook 必须要求工作区干净、目标 commit 明确、验证通过 |
| 运行时版本来源未统一 | CLI、HTTP、frontend build metadata 如果各自读取不同来源，会出现版本不一致 | Phase 4 每个运行时版本入口必须从对应源文件或构建注入的同一版本事实读取 |
| `scheduler` 版本如何进入 Dagster metadata 未定 | 版本可见性无法覆盖 asset run 和回填报告 | Phase 4 需确认 Dagster metadata 挂载点：resource metadata、run tag、asset materialization metadata 或 job report 记录 |
| dataset contract version 校验只检查存在，不检查 bump 合理性 | 字段 contract 改了但 version 未 bump，或只改描述却误 bump | Phase 3 先做格式和 manifest 一致性校验；后续可追加 diff-aware 检查，读取 Git diff 判断字段事实变化 |
| release manifest 中 contract registry 的表达粒度未定 | 只写 commit 不够回答哪些 dataset 版本变化；只写全量 dataset 又太重 | Phase 2 首版使用 `registry_commit` + `changed_datasets`；是否生成全量 contract snapshot 放到后续评估 |
| `pipeline` root version 的长期角色仍未决定 | root `[project]` 版本可能继续被误读成数据平台版本 | Phase 2/3 在 manifest 和校验脚本中明确排除 `pipeline` root version 作为业务组件版本，除非 root meta package 自身变化 |
| 现有 active RFC/plan 是否需要版本影响说明模板 | 后续 PR 没有统一 `Version impact` 字段，会回到人工判断 | Phase 2 新增 release note 模板时同步提供 PR/变更说明中的 `Version impact` 模板 |

## 实施阶段

### Phase 0：版本事实基线与计划确认

目标：冻结当前版本信息事实，只做只读盘点和执行计划确认。

实施项：

1. 运行只读版本扫描：

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

2. 记录当前基线：
   - Python/uv package versions。
   - dbt project version。
   - Rust workspace/crate version 继承状态。
   - Racingline package version。
   - dataset contract version 覆盖率。
   - Alembic target heads。
3. 确认本计划只安排后续实施，不做版本 bump。
4. 把本计划登记到 `docs/plans/README.md`。

验证命令：

```bash
make docs-check
git diff --check
```

完成标准：

1. 本计划和索引通过 docs governance 校验。
2. 当前版本事实清单可从计划追溯到真实文件。
3. 信息缺口已有阶段归属，没有未落点的关键问题。

### Phase 1：Rust crate 独立版本迁移

目标：解除 Rust workspace 统一版本对 Furnace 与 Rearview 发布节奏的绑定。

实施项：

1. 修改 `engines/Cargo.toml`：
   - 保留 `[workspace.package]` 中的 `edition`、`license`、`publish` 等共享元数据。
   - 移除或停止使用 workspace 级 `version`。
2. 修改每个 crate：
   - `engines/crates/furnace-core/Cargo.toml`
   - `engines/crates/furnace-io/Cargo.toml`
   - `engines/crates/furnace/Cargo.toml`
   - `engines/crates/rearview-core/Cargo.toml`
   - `engines/crates/rearview-server/Cargo.toml`
   - `engines/crates/rearview-portfolio-worker/Cargo.toml`
3. 将 `version.workspace = true` 改为显式：

```toml
version = "0.1.0"
```

4. 保持内部 path dependency 不加 version constraint。
5. 运行 `cargo metadata` 确认每个 crate 版本可读。
6. 检查 `Cargo.lock` 是否因 package version 元数据变化需要更新。

验证命令：

```bash
cd engines
cargo metadata --format-version 1 --no-deps
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

完成标准：

1. `rg -n "version\\.workspace" engines/crates` 无结果。
2. 每个 Rust crate 都有显式 `version = "0.1.0"`。
3. `cargo metadata` 输出中每个 crate 都有独立 version。
4. Rust 质量门禁通过。
5. 如 `Cargo.lock` 发生版本元数据变化，随同提交。

### Phase 2：Release manifest 与 release note 骨架

目标：建立一次集成发布快照的可追溯记录方式。

实施项：

1. 新增 `deploy/release-manifest.yml`。
2. 定义首版 manifest schema：
   - `release`
   - `commit`
   - `components`
   - `database_heads`
   - `contracts.registry_commit`
   - `contracts.changed_datasets`
   - `verification`
3. 记录当前组件版本：
   - `scheduler`
   - `contract-tools`
   - `elt`
   - `furnace`
   - `rearview-server`
   - `rearview-portfolio-worker`
   - `racingline`
4. 明确不把 `pipeline/pyproject.toml` 的 root version 当作数据平台整体版本。
5. 记录 Alembic target heads：
   - `pipeline`
   - `rearview`
6. 新增统一 release note 入口：
   - `docs/releases/README.md`
   - 首个模板或首个 release note，例如 `docs/releases/mono-fleur-2026.06.1.md`
7. 同步 docs 入口：
   - `docs/README.md`
   - 必要时更新 `docs/systems/deploy-ops.md`
8. 提供 `Version impact` 模板，用于后续 PR/变更说明。

验证命令：

```bash
make docs-check
git diff --check
```

如果 manifest 读取 Alembic head，需要追加只读命令：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=pipeline heads
uv run alembic -c migrate/alembic.ini -x target=rearview heads
```

完成标准：

1. `deploy/release-manifest.yml` 能回答“一次部署包含哪些组件版本”。
2. release note 能追溯 commit、组件版本、migration head、contract 变化和验证结果。
3. docs 入口能找到 release note 和 manifest 规则。
4. manifest 中组件版本与源文件一致。

### Phase 3：版本机械校验

目标：把版本格式、manifest 一致性和 Rust 独立版本规则变成可重复检查。

实施项：

1. 新增：

```text
scripts/check_component_versions.py
```

2. 第一版校验范围：
   - Python package versions 符合 SemVer。
   - `pipeline/elt/dbt_project.yml` version 符合 SemVer。
   - `app/racingline/package.json` version 符合 SemVer。
   - `app/racingline/package-lock.json` root package version 与 package.json 一致。
   - Rust crate 不使用 `version.workspace = true`。
   - Rust crate versions 符合 SemVer。
   - dataset contracts 都有整数 `version`。
   - release manifest 中列出的组件版本与源文件一致。
   - release manifest 中不得把 `pipeline` root version 当作业务组件版本。
   - active RFC/plan 链接存在。
3. 根据脚本结果决定是否加入：
   - `make docs-check`。
   - 或新增单独目标 `make versions-check`。
4. 在 `AGENTS.md`、`docs/README.md` 或 `docs/skills/fleur-version-management/SKILL.md` 中挂最小命令指针。

验证命令：

```bash
python3 scripts/check_component_versions.py
make docs-check
git diff --check
```

完成标准：

1. 本地一条命令可以发现版本格式和 manifest 漂移。
2. 错误信息能指出具体文件、字段和期望值。
3. 校验不访问外部服务，不依赖数据库连接。
4. 后续版本变更可先跑脚本再进入领域质量门禁。

### Phase 4：运行时版本暴露

目标：让运行、回填、浏览器问题和后台 worker 排查能直接看到对应组件版本。

实施项：

1. Furnace：
   - 增加 `furnace --version`。
   - JSON summary 可选带 `engine_version`。
   - 版本来源为 `engines/crates/furnace/Cargo.toml` 的 package version。
2. Rearview server：
   - 增加 `rearview-server --version`。
   - 增加 `GET /rearview/version` 或在 health payload 中包含 version。
   - 版本来源为 `engines/crates/rearview-server/Cargo.toml`。
3. Rearview portfolio worker：
   - 增加 `rearview-portfolio-worker --version`。
   - 启动日志记录 version。
   - 版本来源为 `engines/crates/rearview-portfolio-worker/Cargo.toml`。
4. Racingline：
   - 注入 build metadata：app version、git sha、build time。
   - UI 或 debug endpoint 中可读取；不要求面向普通用户可见。
   - 版本来源为 `app/racingline/package.json`。
5. scheduler：
   - 确认版本进入 Dagster run 或 asset materialization metadata 的挂载点。
   - 回填/运行报告能记录 scheduler package version。
6. contract-tools：
   - 增加或确认 `fleur-contracts --version`。
   - 版本来源为 `pipeline/contract_tools/pyproject.toml`。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm run lint
npm run typecheck
npm test
npm run build

cd ../../pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests
```

完成标准：

1. 每个可执行/可部署组件都能在运行时或构建产物中暴露版本。
2. 版本值来自对应源文件或构建注入的同一事实源。
3. 运行报告和故障排查材料可以引用这些版本字段。
4. 库 crate 不额外新增运行时入口。

### Phase 5：Tag runbook 与首次集成快照

目标：在版本信息和校验机制稳定后，建立可复用 tag 流程。

实施项：

1. 明确组件 tag 格式：

```text
scheduler-v0.2.0
contract-tools-v0.2.0
elt-v1.1.0
furnace-v0.2.0
rearview-server-v0.2.0
racingline-v0.1.0
```

2. 明确集成 tag 格式：

```text
mono-fleur-YYYY.MM.N
```

3. 编写 tag 前检查：
   - `git status --short` 必须干净。
   - 目标 commit 明确。
   - 对应版本源文件已提交。
   - release manifest 已更新。
   - release note 已记录验证命令和结果。
   - `git tag --list` 不存在同名 tag。
4. 首次创建集成 tag 前，先完成 Phase 2-4。
5. 如果用户要求立即 tag，但验证状态不清楚，按 `fleur-version-management` 停止条件说明阻塞。

验证命令：

```bash
git status --short
git rev-parse --short HEAD
git tag --list | sort
python3 scripts/check_component_versions.py
```

完成标准：

1. tag 流程不会指向未验证或未提交版本信息。
2. 集成 tag 可以追溯到 release manifest 和 release note。
3. 组件 tag 和集成 tag 的用途不会混淆。

## 版本影响说明模板

后续涉及版本信息的 PR 或变更说明应包含：

```text
Version impact:
- Components:
  - <component>: <old> -> <new>, <major|minor|patch|none>, <reason>
- Dataset contracts:
  - <dataset>: <old> -> <new>, <reason>
- Alembic heads:
  - <target>: <old> -> <new>
- Release manifest: <updated|not updated>, <reason>
- Runtime version exposure: <updated|not updated>, <reason>
- Tags: <none|component tag|integration tag>, <reason>
```

## 验证命令

本计划文档阶段：

```bash
make docs-check
git diff --check
```

后续执行阶段按变更范围追加：

```bash
python3 scripts/check_component_versions.py
```

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests
uv run dbt parse --project-dir elt --profiles-dir elt
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

## 完成标准

1. Rust crates 已从 workspace 统一版本迁移到显式独立版本。
2. `deploy/release-manifest.yml` 已存在，并能追溯组件版本、migration head、contract 变化和验证状态。
3. `docs/releases/` 已建立统一 release note 入口。
4. `scripts/check_component_versions.py` 能检查版本格式、manifest 一致性和 Rust 独立版本规则。
5. 可执行/可部署组件具备运行时版本暴露。
6. tag runbook 明确组件 tag 与集成 tag 的用途、格式和前置条件。
7. `fleur-version-management` skill 与计划执行结果保持一致。
8. 每个阶段都有对应验证命令和必要的运行/变更记录。

## 风险与控制

| 风险 | 控制 |
|---|---|
| 版本治理变成大量手写文档 | Phase 3 必须新增机械校验，release manifest 与源文件不一致时失败 |
| 过早引入组件独立 changelog | 第一阶段只维护统一 release note；独立 changelog 需满足高频独立发布或外部消费条件 |
| tag 被用作未验证快照 | Phase 5 要求工作区干净、目标 commit 明确、release note 和 manifest 完整、验证通过 |
| Rust crate version 与 lockfile 漂移 | Phase 1 必须检查 `Cargo.lock` 是否需要更新 |
| npm package version 与 lockfile 漂移 | Phase 3 脚本必须检查 root package lock version |
| `pipeline` root version 被误读 | manifest 和脚本显式排除 root version 作为业务组件版本 |
| 运行时版本来源分裂 | 每个运行时入口从对应 package metadata 或构建注入事实读取 |

## 实施结果

完成日期：2026-06-26

1. Rust workspace 已移除统一 package version，6 个 crate 都显式维护 `version = "0.1.0"`。
2. `furnace`、`rearview-server` 和 `rearview-portfolio-worker` 已提供 `--version`；Rearview HTTP `GET /rearview/version` 和 `/healthz` 从 server service identity 暴露版本；worker 启动日志记录版本。
3. `contract-tools` 已提供 `fleur-contracts --version`。
4. scheduler 已通过 Dagster materialization metadata 暴露 `scheduler_version`。
5. Racingline 已在 Vite build 中注入 `window.__RACINGLINE_BUILD_METADATA__`，包含 app version、git sha 和 build time。
6. 已新增 `deploy/release-manifest.yml`、`docs/releases/README.md` 和 `docs/releases/mono-fleur-2026.06.1.md`。
7. 已新增 `scripts/check_component_versions.py` 和 `make versions-check`，覆盖 SemVer、Rust 独立版本、dataset contract version、Racingline lockfile、manifest 一致性和计划/RFC 索引链接。
8. 已将版本校验入口同步到 `AGENTS.md`、`docs/README.md`、`docs/systems/deploy-ops.md` 和 `docs/skills/fleur-version-management/SKILL.md`。

## 已补齐的信息缺口

| 缺口 | 处理结果 |
|---|---|
| `deploy/release-manifest.yml` schema 未固化 | 已在 `docs/releases/README.md` 固化首版 schema，并由 `make versions-check` 校验组件版本一致性 |
| Alembic head 获取方式未机械化 | 已验证当前 Alembic graph 是全局线性 head；manifest 使用 `database_heads` 记录实际 revision head，使用 `target_schema_heads` 记录每个 target 最后执行 DDL 的 migration |
| Rust crate version 迁移影响 `Cargo.lock` 未确认 | 已运行 `cargo metadata --format-version 1 --no-deps`，`Cargo.lock` 未发生变更 |
| npm version 与 lockfile 同步策略未定 | `make versions-check` 已校验 `package.json` 与 `package-lock.json` root version 一致 |
| release note 目录职责未纳入 docs 入口 | 已同步 `docs/README.md` 和 `docs/systems/deploy-ops.md` |
| 运行时版本来源未统一 | Python 从 package metadata 或源 `pyproject.toml` 读取；Rust 从 `CARGO_PKG_VERSION` 读取；Racingline 从 `package.json` 构建注入 |
| `scheduler` 版本如何进入 Dagster metadata 未定 | 已写入 Furnace assets 和 Rearview daily run asset materialization metadata |
| release manifest 中 contract registry 表达粒度未定 | 首版采用 `registry_commit` + `changed_datasets` |
| `pipeline` root version 的长期角色仍未决定 | manifest 和校验脚本明确排除 `pipeline` root version 作为业务组件版本 |
| active RFC/plan 是否需要版本影响说明模板 | 已在 `docs/releases/README.md` 和 release note 中提供 `Version impact` 模板 |

## 验证结果

详见 [mono-fleur-2026.06.1 release note](../../releases/mono-fleur-2026.06.1.md)。
