# Project Status

状态：当前架构事实入口（2026-06-27）

本文件记录 mono-fleur 当前项目状态和子域入口。新需求先从这里确认影响范围，再进入对应架构事实文档、ADR、RFC、plan、job report、reference 或代码实现。

## 当前状态

| 子域 | 当前代码根 | 架构事实 | 当前状态 |
|---|---|---|---|
| 数据平台 | `pipeline/scheduler/`、`pipeline/elt/`、`pipeline/migrate/` | [data-platform.md](data-platform.md) | Dagster 采集编排、S3 Parquet、ClickHouse raw sync、dbt 分层建模和 PostgreSQL migration 均已形成稳定入口 |
| 数据治理 | `pipeline/contracts/`、`pipeline/contract_tools/`、`pipeline/elt/metadata/field_glossary.yml` | [data-governance.md](data-governance.md) | raw 层字段事实、data dict 生成、staging readiness 和 field glossary 校验由 contract/tools/dbt 脚本共同维护 |
| Furnace | `engines/crates/furnace*`、`pipeline/scheduler/src/scheduler/defs/furnace/` | [furnace.md](furnace.md) | KDJ、MA、RSI、BOLL、MACD 和价格行为结构指标由 Rust CLI 计算，Dagster 只负责编排调用 |
| Rearview | `engines/crates/rearview-core/`、`engines/crates/rearview-server/`、`engines/crates/rearview-portfolio-worker/` | [rearview.md](rearview.md) | Rust HTTP 服务和 worker 已覆盖 preview、strategy backtest、portfolio publish、pending 首次运行和 live daily result |
| Racingline | `app/racingline/` | [racingline.md](racingline.md) | 单一正式前端工作台，覆盖 dashboard、策略创建 Step 1-5、T+1 建立组合和 pending/live 展示 |
| 部署与运行 | `deploy/`、`pipeline/migrate/`、`docs/jobs/` | [deploy-ops.md](deploy-ops.md) | Docker Compose、本地 dev/smoke 依赖、migration、release manifest 和运行报告入口集中维护 |

## 跨系统边界

| 边界 | 当前约束 |
|---|---|
| Dagster 与 dbt | Dagster 负责采集、S3 Parquet、raw sync 和资产编排；dbt 负责 staging/intermediate/marts 建模和数据测试 |
| contracts 与 dbt | `pipeline/contracts` 只到 ClickHouse raw 层；dbt staging SQL、tests、字段描述和 mart 语义由 `pipeline/elt` 维护 |
| Furnace 与数据平台 | 技术指标公式只在 `furnace-core`；Python asset 和 dbt wrapper 不重写递推公式 |
| Rearview 与 mart | Rearview 只消费 `fleur_marts` 和 backtest/live 结果事实族，不绕过 mart 读取 raw/staging/intermediate/calculation 表 |
| Racingline 与后端 | Racingline 只通过 Rearview API 消费策略、回测、组合和个股分析能力，不直接访问 ClickHouse、PostgreSQL、NATS 或 dbt |
| Deploy/Ops 与业务 | deploy 只维护本地基础设施、migration 和运行前提，不定义业务模型、指标公式、规则 AST 或前端交互 |

## 主要工作流

| 工作流 | 入口 | 最小验证 |
|---|---|---|
| Python / dbt / Dagster | `pipeline/` | `uv run dbt parse --project-dir elt --profiles-dir elt`、`cd scheduler && uv run dg check defs` |
| 数据契约 | `pipeline/contracts/`、`pipeline/contract_tools/` | `uv run fleur-contracts validate`、`uv run fleur-contracts generate --check` |
| Rust engines | `engines/` | `cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`cargo test --workspace` |
| Racingline | `app/racingline/` | `npm run lint`、`npm run typecheck`、`npm test`、`npm run build` |
| 文档治理 | `docs/` | `make docs-check`、`git diff --check` |

## 当前待决方向

1. 是否为 data platform 拆出更高层端到端数据流文档，覆盖 scheduler、dbt、contracts、Furnace、Rearview 和 Racingline 的事实流向。
2. 是否将 Rearview 鉴权、用户隔离和 API 错误响应结构上升为 ADR。
3. 是否建立 mart 字段事实到 Rearview metric catalog 的专用治理入口。
4. 是否为 dev、smoke、production-like 环境补统一环境矩阵和 health check runbook。
