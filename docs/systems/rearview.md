# System: Rearview

状态：当前事实入口（2026-06-16）

## 代码根

| 路径 | 角色 |
|---|---|
| [engines/crates/rearview-core/](../../engines/crates/rearview-core/) | Rust Rearview 核心库，包含 config、repository、API、ClickHouse 查询和组合计算共享逻辑 |
| [engines/crates/rearview-server/](../../engines/crates/rearview-server/) | Rust 规则选股 HTTP 服务入口 |
| [engines/crates/rearview-portfolio-worker/](../../engines/crates/rearview-portfolio-worker/) | Rust 组合净值异步 worker 入口 |
| [engines/crates/rearview-core/config/metric_policy.yml](../../engines/crates/rearview-core/config/metric_policy.yml) | metric policy overlay |
| [pipeline/migrate/](../../pipeline/migrate/) | PostgreSQL `rearview` database migration 入口 |

## 职责

1. 提供规则集、不可变规则版本、区间运行、股票池和买入信号 HTTP API。
2. 校验规则 AST 和 metric catalog，编译受控 ClickHouse 查询。
3. 消费 ClickHouse `fleur_marts` 指标 mart，并把运行状态、股票池和买入信号写入 PostgreSQL `rearview` database。
4. 保存 rule hash、compiled SQL hash、ClickHouse query id、chunk 状态和结果解释快照。
5. 提供 `GET /rearview/runs/{run_id}/securities/{security_code}/analysis`，在校验 run result membership 后组合 PostgreSQL result snapshot 与 ClickHouse mart 当前查询值。
6. 提供虚拟账户模板、默认市场费率模板、组合运行、组合净值和目标/订单/成交/持仓/事件明细 API。
7. 通过 PostgreSQL outbox 和 NATS JetStream 分发组合净值计算任务，由 `rearview-portfolio-worker` 幂等写回组合账本。

## 非职责

1. 不重算 KDJ、MA、RSI、BOLL、MACD 或价格行为结构指标；这些由 Furnace/dbt 维护。
2. 不绕过 mart 层读取 raw、staging、intermediate 或 calculation 表。
3. 不提供前端交互；Racingline 承担 UI 工作台。
4. 不自动执行 PostgreSQL DDL migration；迁移由 `pipeline/migrate` 管理。
5. 不把当前 mart 查询值写回 PostgreSQL run snapshot。

## 主要依赖

| 依赖 | 用途 |
|---|---|
| PostgreSQL `rearview` database | 规则、版本、运行、chunk、day、pool、signal 和 metric catalog 状态 |
| ClickHouse `fleur_marts` | 日频行情、趋势、动量、成交量和价格行为结构指标 |
| NATS JetStream | 组合净值计算任务的 at-least-once 分发 |
| dbt mart YAML | metric catalog 基础字段事实校验来源 |
| Furnace/dbt | 指标计算和 mart 物化 |

## 运行入口

本地开发复用根目录 `.env` 和 `deploy/docker-compose.yml`。快速启动 Rearview + Racingline：

```bash
make racingline-dev
```

该命令会先清理 `REARVIEW_HTTP_BIND` 和 Racingline Vite 端口上的既有监听进程，再启动 Docker dev 依赖服务、等待 PostgreSQL/ClickHouse、执行 PostgreSQL migrations、同步 Rearview metric catalog，最后启动 Rearview HTTP 服务、portfolio worker 和前端 dev server。只启动 Rearview HTTP 服务：

```bash
make rearview-dev
```

`make racingline-dev-stop` 只清理前后端 dev server 端口；停止 Docker 依赖服务仍使用 `make dev-down`。

手动展开步骤：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d postgres clickhouse nats

cd pipeline
uv run alembic -c migrate/alembic.ini -x target=pipeline upgrade head
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head

cd ../engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog sync
cargo run -p rearview-server -- serve
cargo run -p rearview-portfolio-worker -- run
```

`rearview-server` 启动时会幂等 ensure portfolio NATS stream，并运行进程内 outbox dispatcher；`rearview-portfolio-worker` 启动时会幂等 ensure stream 和 durable consumer。

## 质量门禁

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 migration 时追加：

```bash
cd pipeline/migrate
uv run alembic upgrade head
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../../engines/README.md](../../engines/README.md) | Rust engines 工作区地图和 Rearview HTTP API 入口 |
| [../RFC/0018-rust-stock-screening-service.md](../RFC/0018-rust-stock-screening-service.md) | Rearview 后端服务设计 |
| [../RFC/0019-racingline-rearview-frontend-workbench.md](../RFC/0019-racingline-rearview-frontend-workbench.md) | Racingline 前端工作台设计 |
| [../RFC/0020-racingline-run-result-security-analysis-page.md](../RFC/0020-racingline-run-result-security-analysis-page.md) | Run result 个股分析页已实现 RFC |
| [../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md) | 虚拟账户、NATS 分发、`rearview-server` / `rearview-portfolio-worker` crate 拆分和组合净值计算 Proposed RFC |
| [../RFC/0024-racingline-strategy-selection-step1.md](../RFC/0024-racingline-strategy-selection-step1.md) | 从 `/strategies` Step 1 接通 metric catalog、RuleVersionSpec、crossing operator 和 explain 的 Proposed RFC |
| [../RFC/0025-racingline-strategy-weight-configuration-step2.md](../RFC/0025-racingline-strategy-weight-configuration-step2.md) | 从 `/strategies` Step 2 接通 `RuleVersionSpec.scoring.rules`，并定义点击股池预览时才执行选股、评分和排名的 Implemented RFC |
| [../RFC/0026-racingline-strategy-pool-preview-step3.md](../RFC/0026-racingline-strategy-pool-preview-step3.md) | 从 `/strategies` Step 3 股池预览切入，定义 preview snapshot、全池分页、证券显示和 preview-only 个股上下文的 Proposed RFC |
| [../plans/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md](../plans/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md) | 虚拟账户、组合运行、worker 和 Racingline 组合页面当前实施计划 |
| [../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md](../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md) | Rearview preview-only API、`[0, 100]` scoring clamp 和策略权重配置 Step 2 实施计划归档 |
| [../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md](../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md) | Rearview preview-only API 和 Racingline Step 2/3 闭环验收报告 |
| [../plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md](../plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md) | Rearview metric catalog、crossing operator 和 explain 缺口填补实施计划归档 |
| [../plans/archive/0036-rust-rearview-stock-screening-service-implementation-plan.md](../plans/archive/0036-rust-rearview-stock-screening-service-implementation-plan.md) | Rearview 后端历史实施计划 |
| [../plans/archive/0039-racingline-run-result-security-analysis-page-implementation-plan.md](../plans/archive/0039-racingline-run-result-security-analysis-page-implementation-plan.md) | Rearview analysis API 和 Racingline 个股分析页实施计划 |
| [../jobs/reports/2026-06-12-rearview-n-structure-low-reversal-smoke-run.md](../jobs/reports/2026-06-12-rearview-n-structure-low-reversal-smoke-run.md) | 代表性规则 smoke run 记录 |
| [../jobs/reports/2026-06-15-rearview-low-reversal-ma-chain-run.md](../jobs/reports/2026-06-15-rearview-low-reversal-ma-chain-run.md) | 低位反转追加 MA 链过滤后的区间运行记录 |
| [../jobs/reports/2026-06-15-racingline-security-analysis-page.md](../jobs/reports/2026-06-15-racingline-security-analysis-page.md) | 个股 analysis API、图表和 UI 验收报告 |
| [../jobs/reports/2026-06-16-racingline-portfolio-nav.md](../jobs/reports/2026-06-16-racingline-portfolio-nav.md) | 虚拟账户组合净值、NATS worker、明细 API 和 Racingline 组合页面 smoke 报告 |
| [../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md](../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md) | Rearview metric catalog coverage、crossing explain 和 Racingline Step 1 vertical slice 验收报告 |

## 待决问题

1. Rearview 鉴权、用户隔离和 API 错误响应结构是否应上升为 ADR。
2. 是否新增 `mart_stock_rearview_metric_daily` 作为选股专用宽表。
3. `mart_stock_trend_indicator` 和 `mart_stock_momentum_indicator` 当前排序键以 `trade_date` 优先；个股 analysis API 第一版用日期窗口约束查询，后续如响应变慢再评估专用 chart mart。
