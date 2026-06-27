# RFC 0033: 多工程版本管理方案

状态：Proposed（讨论稿，2026-06-26）
领域：repository governance, release management, pipeline, engines, racingline
关联系统：data-platform, data-governance, furnace, rearview, racingline, deploy-ops
代码根：pipeline/, engines/, app/racingline/, deploy/
关联文档：docs/README.md, docs/architecture/README.md, docs/architecture/data-platform.md, docs/architecture/data-governance.md, docs/architecture/furnace.md, docs/architecture/rearview.md, docs/architecture/racingline.md, docs/architecture/deploy-ops.md

## 摘要

mono-fleur 是一个 monorepo，但不是单一可发布产品。当前仓库同时包含 Dagster 调度项目、dbt 转换项目、数据契约工具、数据契约注册表、Alembic 迁移、Rust CLI/服务/worker、Racingline 前端和部署配置。它们的发布节奏、兼容性边界和回滚方式不同，不适合只用一个仓库级版本号表达全部状态。

本 RFC 建议采用“组件独立版本 + 仓库集成发布标记”的模式：

1. 每个可独立构建、部署、调用或被其他组件依赖的软件单元维护自己的版本号。
2. 非软件包但有兼容性语义的对象使用自己的原生版本机制，例如 dataset contract `version`、Alembic revision、dbt project version。
3. 仓库级 tag 只表达一次集成快照，不替代组件版本。
4. 版本 bump 必须和对应行为变更、schema 变更、API 变更或发布记录一起提交，避免版本号成为装饰字段。

## 目标

1. 给当前仓库内主要工程单元定义清晰的版本归属和版本源文件。
2. 允许 `scheduler`、`contract-tools`、`elt`、Furnace、Rearview、Racingline 等组件按真实变更独立发布。
3. 区分 SemVer 版本、数据契约版本、数据库迁移 revision 和集成发布 tag 的用途。
4. 为后续自动校验、changelog 和发布脚本留下稳定规则。
5. 降低多工程文件同时存在时的版本混乱：不再依赖“所有包都跟随同一个 0.1.0”来表达状态。

## 非目标

1. 本 RFC 不立即批量修改所有版本号。
2. 本 RFC 不引入外部发布平台、私有包仓库或制品仓库。
3. 本 RFC 不要求每个目录都拥有 SemVer；只有具备发布或兼容性边界的单元才维护版本。
4. 本 RFC 不改变现有 Python、Cargo、npm、dbt、Alembic 的构建方式。
5. 本 RFC 不把 lockfile 中的第三方依赖版本当作 mono-fleur 自身版本。

## 当前版本基线

截至 2026-06-26，仓库内已经存在以下版本字段或版本机制：

| 单元 | 当前版本源 | 当前值 | 当前问题 |
|---|---|---:|---|
| Python uv workspace meta package | `pipeline/pyproject.toml` | `0.1.0` | 作为 workspace/meta package 的语义未定义 |
| Dagster scheduler package | `pipeline/scheduler/pyproject.toml` | `0.1.0` | 可独立版本，但 bump 规则未定义 |
| Contract tools CLI/package | `pipeline/contract_tools/pyproject.toml` | `0.1.0` | 可独立版本，但需和 contract schema 兼容性区分 |
| dbt elt project | `pipeline/elt/dbt_project.yml` | `1.0.0` | 已有 dbt project version，但和模型兼容性规则未绑定 |
| Dataset contracts | `pipeline/contracts/datasets/*.yml` | `version: 1` | 已有逐 dataset 整数版本，需明确 bump 触发条件 |
| Alembic migrations | `pipeline/migrate/versions/**` | revision chain | 已有 revision 机制，不应再叠加无意义 SemVer |
| Rust engines workspace | `engines/Cargo.toml` | workspace `0.1.0` | Furnace 与 Rearview 共享一个 workspace version，不能表达独立发布 |
| Rust crates | `engines/crates/*/Cargo.toml` | `version.workspace = true` | 每个 crate 无独立版本号 |
| Racingline frontend | `app/racingline/package.json` | `0.0.1` | 可独立版本，但和 Rearview API 兼容性未绑定 |
| Deploy compose/config | `deploy/` | 无组件版本 | 部署快照缺少组件版本清单 |

## 版本类型

### Component SemVer

适用于可构建、可部署、可调用或可被其他工程依赖的软件单元。版本格式使用 SemVer：

```text
MAJOR.MINOR.PATCH
```

规则：

1. `MAJOR`：公共 API、CLI 参数、HTTP contract、数据模型读取 contract 或部署 contract 出现破坏性变化。
2. `MINOR`：新增向后兼容能力、字段、命令、模型或可选配置。
3. `PATCH`：修复 bug、性能优化、内部重构、文档修正，不改变外部 contract。

在 `0.x` 阶段采用更保守规则：破坏性变化 bump `MINOR`，兼容修复 bump `PATCH`；进入 `1.0.0` 后遵循标准 SemVer 的 `MAJOR` 破坏性规则。

### Dataset Contract Version

适用于 `pipeline/contracts/datasets/*.yml`。继续使用整数 `version`，不改成 SemVer。

规则：

1. raw/source/parquet/clickhouse_raw 字段集合、字段类型、nullable、required、grain 或 partition 语义变化时 bump。
2. 只改 `description_zh`、`external_description_zh`、`validation_notes`、文档拼写或排序时不 bump。
3. 一个 dataset 的版本只描述该 dataset 自己的 contract，不代表整个 contract registry。
4. `contract-tools` 自身的解析、生成、校验能力由 `pipeline/contract_tools/pyproject.toml` 的 SemVer 表达。

### Alembic Revision

适用于 `pipeline/migrate/versions/**`。数据库 schema 版本以 Alembic head revision 表达，不额外引入 SemVer。

规则：

1. 每个数据库目标以自己的 revision head 表达当前 schema：当前至少有 `pipeline` 和 `rearview` 两类迁移目录。
2. 新增 DDL 只新增 revision，不修改历史 revision。
3. 需要跨服务声明兼容性时，在发布说明或部署版本清单中记录所需 Alembic head。
4. 应避免新增 `pipeline/migrate/pyproject.toml` 只为了版本号；迁移不是可发布包。

### Integration Release Tag

适用于整个仓库的一次集成快照。仓库级 tag 不表达单个组件的 API 兼容性，只表达“这些组件版本在同一提交上共同验证过”。

建议格式：

```text
mono-fleur-YYYY.MM.N
```

示例：

```text
mono-fleur-2026.06.1
```

组件 tag 使用组件名加 SemVer：

```text
scheduler-v0.2.0
contract-tools-v0.2.0
elt-v1.1.0
furnace-v0.2.0
rearview-server-v0.2.0
racingline-v0.1.0
```

## 版本主体矩阵

| 主体 | 版本源文件 | 类型 | 独立版本策略 |
|---|---|---|---|
| `pipeline` workspace meta | `pipeline/pyproject.toml` | Component SemVer | 仅当 workspace 依赖拓扑、共享质量门禁、Python baseline 或 meta package 行为变化时 bump |
| `scheduler` | `pipeline/scheduler/pyproject.toml` | Component SemVer | Dagster assets/resources/schedules/sensors、CLI 可见行为、Furnace 调用 contract 变化时独立 bump |
| `contract-tools` | `pipeline/contract_tools/pyproject.toml` | Component SemVer | CLI、contract schema adapter、生成物格式、校验规则变化时独立 bump |
| `contracts` datasets | `pipeline/contracts/datasets/*.yml` | Dataset Contract Version | 每个 dataset 独立整数版本；字段 contract 变化时 bump |
| `elt` dbt project | `pipeline/elt/dbt_project.yml` | Component SemVer | dbt 模型 contract、materialization、公开 marts/intermediate wrapper 语义变化时 bump |
| `migrate` | `pipeline/migrate/versions/**` | Alembic Revision | 按 target database 的 revision head 管理，不使用 SemVer |
| `furnace-core` | `engines/crates/furnace-core/Cargo.toml` | Component SemVer | 指标参数、输入输出模型、纯计算 API 变化时独立 bump |
| `furnace-io` | `engines/crates/furnace-io/Cargo.toml` | Component SemVer | ClickHouse DDL/SQL、RowBinary、写入模式、summary contract 变化时独立 bump |
| `furnace` CLI | `engines/crates/furnace/Cargo.toml` | Component SemVer | CLI 参数、退出码、JSON summary、命令行为变化时独立 bump |
| `rearview-core` | `engines/crates/rearview-core/Cargo.toml` | Component SemVer | HTTP/domain DTO、repository contract、portfolio simulation engine、metric catalog 语义变化时独立 bump |
| `rearview-server` | `engines/crates/rearview-server/Cargo.toml` | Component SemVer | HTTP API、catalog command、serve behavior、outbox dispatcher contract 变化时独立 bump |
| `rearview-portfolio-worker` | `engines/crates/rearview-portfolio-worker/Cargo.toml` | Component SemVer | NATS task contract、worker execution semantics、result write contract 变化时独立 bump |
| `racingline` | `app/racingline/package.json` | Component SemVer | 前端路由、API 消费 contract、构建产物、用户可见工作流变化时独立 bump |
| `deploy` bundle | `deploy/release-manifest.yml`（建议新增） | Integration Manifest | 记录一次部署采用的组件版本、image tag、migration head、dbt project version；不替代组件版本 |

## Rust engines 独立版本迁移

当前 Rust workspace 使用：

```toml
[workspace.package]
version = "0.1.0"
```

并且各 crate 使用：

```toml
version.workspace = true
```

这会把 Furnace 与 Rearview 的发布节奏绑定在一起。目标态应改为：

1. `engines/Cargo.toml` 的 `[workspace.package]` 保留 `edition`、`license`、`publish` 等共享元数据。
2. 移除 workspace 级 `version`，或不再让 crate 继承 workspace version。
3. 每个 `engines/crates/*/Cargo.toml` 显式写自己的 `version = "..."`。
4. 内部 path dependency 不强制指定版本；monorepo 内构建仍以 path 为准。
5. 对外发布 crate 或二进制 artifact 时，再由 release 流程把 crate version 写入 artifact metadata。

建议初始拆分版本：

| Crate | 初始独立版本 | 理由 |
|---|---:|---|
| `furnace-core` | `0.1.0` | 当前计算 API 仍处于 pre-1.0 |
| `furnace-io` | `0.1.0` | 当前写入和 summary contract 仍处于 pre-1.0 |
| `furnace` | `0.1.0` | 当前 CLI 已可用但仍随指标族扩展演进 |
| `rearview-core` | `0.1.0` | 当前 API/portfolio engine 仍快速演进 |
| `rearview-server` | `0.1.0` | HTTP API 和 control plane 仍快速演进 |
| `rearview-portfolio-worker` | `0.1.0` | Worker task/result contract 仍快速演进 |

后续变更只 bump 受影响 crate。例如只优化 `furnace-core` 纯计算性能且不改输出语义时 bump `furnace-core` patch；如果 `furnace` CLI JSON summary 加字段且向后兼容，则 bump `furnace` minor，必要时 bump `scheduler` minor 以记录消费端已支持新字段。

## dbt 与数据 contract 版本边界

`pipeline/contracts` 和 `pipeline/elt` 是两个不同版本边界：

1. `contracts` 的 `version` 到 ClickHouse raw 层为止，描述 source payload、Parquet 和 raw table contract。
2. `elt` 的 `version` 描述 dbt 项目公开模型、tests、materialization 和 marts/intermediate wrapper 语义。
3. raw contract 变更不自动要求 `elt` bump；只有当 dbt 模型输入/输出 contract 或消费语义变化时，`elt` 才 bump。
4. dbt `sources.yml` 和 data dictionary 是由 contract 生成或校验的派生产物，不单独拥有版本。
5. `pipeline/elt/metadata/field_glossary.yml` 是 dbt canonical 字段治理入口；字段命名规则变化如果影响模型输出，应 bump `elt`。

dbt `elt` 的破坏性变化包括：

1. 删除或重命名公开 model。
2. 删除、重命名或改变公开 column 语义。
3. 改变 grain、primary business key 或 materialization 造成消费查询不兼容。
4. 改变 mart 指标口径。

兼容 minor 变化包括：

1. 新增 model。
2. 新增 nullable column。
3. 新增 tests、exposures、docs 或 metadata。
4. 新增向后兼容的 wrapper model。

## Deploy 版本清单

`deploy/` 本身不是可发布库，但一次部署需要知道采用了哪些组件版本。建议在实施阶段新增：

```text
deploy/release-manifest.yml
```

建议字段：

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
  pipeline: 0001_jiuyan_industry_images
  rearview: 0008_strategy_portfolio_cp
contracts:
  registry_commit: <git-sha>
  changed_datasets:
    - baostock__query_history_k_data_plus_daily: 1
```

该文件只记录部署快照，不作为源码版本的唯一事实源。组件版本仍以各自工程文件为准。

## Bump 触发规则

每次代码或 contract 变更应先确定影响主体，再决定版本 bump：

| 变更类型 | 必须 bump |
|---|---|
| 修改 Dagster asset/resource/schedule 可见行为 | `scheduler` |
| 修改 `fleur-contracts` CLI、contract parser、schema adapter 或生成格式 | `contract-tools` |
| 修改 dataset raw/parquet/clickhouse_raw contract | 对应 dataset `version` |
| 修改 dbt 公开模型 contract 或指标口径 | `elt` |
| 新增 Alembic migration | 新增 revision；发布清单记录 head |
| 修改 Furnace 计算公式、输入输出模型或 CLI summary | `furnace-core`、`furnace-io`、`furnace` 中受影响者 |
| 修改 Rearview HTTP API、NATS task、worker result contract | `rearview-core`、`rearview-server`、`rearview-portfolio-worker` 中受影响者 |
| 修改 Racingline 用户可见 workflow 或 API 消费 contract | `racingline` |
| 仅文档、测试或内部重构且无行为变化 | 通常不 bump；若该文档是公开 runbook 或部署 contract，可 bump 对应组件 patch |

跨组件变更必须同时记录生产者和消费者：

1. 生产者 contract 破坏性变化：生产者 bump 破坏性版本，消费者同一 PR 或后续兼容 PR bump。
2. 生产者新增兼容字段：生产者 bump minor；消费者只有实际依赖该字段时才 bump。
3. 消费者仅适配已有 contract：只 bump 消费者。

## 发布与 tag 流程

建议流程：

1. 开发 PR 中修改代码、测试、文档和受影响组件版本。
2. PR 描述列出 `Version impact`：组件、旧版本、新版本、bump 原因。
3. 合并后按需创建组件 tag，例如 `racingline-v0.1.0`。
4. 当一组组件共同完成验收后创建集成 tag，例如 `mono-fleur-2026.06.1`。
5. 如果涉及部署，更新 `deploy/release-manifest.yml` 并记录 migration head、dbt version、组件版本和镜像 tag。

组件 tag 指向实际完成该版本 bump 的 commit。集成 tag 指向完成集成验证的 commit。

## 校验建议

实施阶段应新增一个轻量版本校验脚本，建议路径：

```text
scripts/check_component_versions.py
```

第一阶段校验：

1. 所有版本字段符合格式。
2. Rust crate 不再继承 workspace version。
3. `pipeline/contracts/datasets/*.yml` 都有整数 `version`。
4. `deploy/release-manifest.yml` 中列出的组件版本和源文件一致。
5. `docs/RFC/README.md` 中 active RFC 链接存在。

文档-only 阶段最小校验：

```bash
make docs-check
git diff --check
```

涉及工程版本落地时追加对应质量门禁：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests

cd ../engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

## 分阶段实施建议

### Phase 1：记录规则和基线

1. 接受本 RFC。
2. 保持当前版本字段不变。
3. 新增版本影响说明模板到 PR/release 文档。

完成标准：

1. RFC 已在 `docs/RFC/README.md` 登记。
2. 文档检查通过。

### Phase 2：Rust crate 独立版本

1. 将 `engines/crates/*/Cargo.toml` 从 `version.workspace = true` 改成显式 `version = "0.1.0"`。
2. 调整 `engines/Cargo.toml`，不再用 workspace version 表达所有 crate。
3. 运行 Rust 质量门禁。

完成标准：

1. 每个 Rust crate 都有独立版本字段。
2. `cargo metadata` 能读取每个 crate 的独立 version。
3. `cargo fmt --check`、`cargo clippy`、`cargo test` 通过。

### Phase 3：部署版本清单

1. 新增 `deploy/release-manifest.yml`。
2. 记录当前组件版本和 Alembic head。
3. 在部署 runbook 或架构事实文档中链接该清单。

完成标准：

1. 清单版本与源文件一致。
2. 清单能回答“一次部署包含哪些组件版本”。

### Phase 4：机械校验

1. 新增 `scripts/check_component_versions.py`。
2. 将版本格式、清单一致性和 Rust 独立版本规则纳入 `make docs-check` 或单独 `make versions-check`。
3. 在 AGENTS 或 docs 入口中加入最小命令指针。

完成标准：

1. 本地可一键检查版本元数据。
2. 后续版本漂移能被脚本发现，而不是靠人工审查。

## 建议结论

### Changelog 策略

当前不建议立刻为每个组件维护独立 changelog。更合适的第一步是新增一个统一 release 入口，按集成发布记录组件版本和变更摘要：

```text
docs/releases/README.md
docs/releases/mono-fleur-2026.06.1.md
```

单次 release note 应包含：

1. 集成 tag。
2. Git commit。
3. 组件版本清单。
4. Alembic head。
5. dbt project version。
6. contract registry 变更摘要。
7. 验证命令和结果。

只有当某个组件开始高频独立发布、对外提供稳定 API，或被其他仓库消费时，才拆出组件独立 changelog，例如：

```text
docs/releases/components/rearview-server.md
docs/releases/components/racingline.md
```

理由：当前 mono-fleur 的主要痛点是“整套系统用了哪些版本”不可追溯，而不是“每个组件都需要公开发布说明”。先维护集成 release note，可以用更少文件覆盖主要运维和回溯需求。

### 集成 tag 格式

集成 tag 建议使用：

```text
mono-fleur-YYYY.MM.N
```

不建议使用 SemVer。

理由：

1. 集成 tag 不是公共 API contract，不表达向后兼容性。
2. 集成 tag 表达的是“一组组件版本在同一 commit 上通过验收”。
3. 日期序号更适合运维回溯、运行报告和部署记录。
4. 组件自身的破坏性/兼容性语义已经由组件 SemVer 表达。

示例：

```text
mono-fleur-2026.06.1
mono-fleur-2026.06.2
mono-fleur-2026.07.1
```

如果同一天有多次集成验收，可以继续递增 `N`，不需要把 patch/minor/major 语义强加到整仓快照上。

### Rust path dependency 版本约束

当前不需要给 Rust 内部 path dependency 补充 version constraint。保留：

```toml
rearview-core = { path = "../rearview-core" }
```

理由：

1. 当前 `engines` 是 monorepo 内部 workspace，构建解析以 path dependency 为准。
2. `engines/Cargo.toml` 中 `publish = false`，当前没有 crates.io 或私有 registry 发布需求。
3. 给内部 path dependency 过早添加 version constraint，不能提高本地构建安全性，反而增加同步维护成本。

只有进入“对外发布 Rust crate 或把 crate 推送到私有 registry”的阶段，才考虑改为：

```toml
rearview-core = { version = "0.2.0", path = "../rearview-core" }
```

届时需要配套发布脚本校验依赖版本和实际 crate version 一致。

### `pipeline` workspace meta package 版本

`pipeline/pyproject.toml` 的版本不应被当作数据平台整体发布版本使用。它当前更像 uv workspace/meta package 的版本字段。

建议：

1. 短期保留 `pipeline/pyproject.toml` 的 `version = "0.1.0"`，避免为了治理版本引入无关构建变更。
2. 不为普通 `scheduler`、`contract-tools`、`elt` 或 contract registry 变化 bump `pipeline`。
3. 只有 Python workspace baseline 或 meta package 自身语义变化时才 bump，例如 Python 版本基线、workspace 成员拓扑、共享依赖策略或根项目构建语义变化。
4. 文档和 release manifest 中不要把 `pipeline` version 当作“数据平台版本”。数据平台状态应由 `scheduler`、`contract-tools`、`elt`、dataset contract versions 和 Alembic heads 共同表达。

长期是否移除 root `[project]` 或降级为纯 workspace 配置，应作为单独技术清理评估，不放在版本治理第一阶段。

### 运行时版本暴露

需要暴露，但只要求可执行或可部署组件暴露版本；纯库和数据 contract 不需要单独运行时端点。

建议最低要求：

| 组件 | 暴露方式 | 用途 |
|---|---|---|
| `furnace` CLI | `furnace --version`，JSON summary 可选带 `engine_version` | Dagster 日志、回填报告、故障回溯 |
| `rearview-server` | `rearview-server --version` 和 `GET /rearview/version` 或 health payload | 前端/后端 API 兼容性排查 |
| `rearview-portfolio-worker` | `rearview-portfolio-worker --version`，启动日志记录 version | NATS task 处理和异步结果排查 |
| `racingline` | build metadata，例如 app version、git sha、build time | 浏览器截图、用户问题和部署回溯 |
| `scheduler` | Dagster resource/run metadata 记录 scheduler package version | asset run、回填和 job report 回溯 |
| `contract-tools` | `fleur-contracts --version` | contract 生成物和校验结果回溯 |

不建议在第一阶段给 `furnace-core`、`furnace-io`、`rearview-core` 这类库单独做运行时入口。它们的版本应通过调用它们的 binary/service 进入日志、summary 或 release manifest。

## 后续实施优先级

1. 先接受本 RFC，并保持现有版本字段不动。
2. 再拆 Rust crate 独立版本。
3. 再新增 `deploy/release-manifest.yml` 和统一 release note。
4. 再补版本校验脚本。
5. 最后为可执行组件补 `--version`、health/build metadata 等运行时版本暴露。
