# System: Rearview

状态：当前事实入口；Strategy Backtest Step 5 已通过 dev live smoke 和 worker 重投递验收（2026-06-23）

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
6. 提供 preview-only 策略检查 API：`POST /rearview/strategy-preview/timeline`、`POST /rearview/strategy-preview`、`POST /rearview/strategy-preview/pool-page` 和 `POST /rearview/strategy-preview/security-analysis`；这些接口不创建 rule set、rule version、run 或 portfolio run。
7. Preview rows、pool page 和 preview security analysis 通过 `mart_stock_basic_snapshot` 补齐 `security_name`、`exchange_code` 和交易板块 `security_board`。
8. Preview security analysis 支持 `include_quote_rows=false`，在保留 membership 校验、`selected_quote` 和 chart series 的同时省略完整 `quote_rows` payload；MA5/MA10/MA30 固定使用前复权指标基准并可叠加到任意 OHLC 复权模式。
9. 提供 draft-only 策略回测校验 API：`POST /rearview/strategy-backtests/validate`，接收 transient `RuleVersionSpec + BacktestExecutionConfig`，返回 canonical config、`rule_hash`、`execution_config_hash` 和仓位/退出规则摘要；该接口不创建 rule set、rule version、run、portfolio run，不写结果事实，也不发 NATS。第一版支持受控 trend indicator stop loss，只接受 `source = "trend"`、allowlisted trend metric 和 `operator = "close_below_metric"`，主图指标集合为 MA、MA 组合和 EMA。
10. 提供 Step 5 strategy backtest control plane：`GET /rearview/strategy-backtests/options`、`POST /rearview/strategy-backtests`、`GET /rearview/strategy-backtests/{id}`。Create API 只落 PostgreSQL `strategy_backtest_run` 与 outbox 并返回 `202 Accepted`，服务端动态解析 `1y/2y/3y` 区间，固化 benchmark、range、rule/config hash、preflight snapshot 和 `client_request_id` 幂等语义。
11. 提供 Step 5 result wrapper API：`/nav`、`/rebalance-records`、`/targets`、`/orders`、`/trades`、`/positions`、`/events`、`/performance`、`/closed-trades` 和 `/trade-metrics`，先解析 `current_result_attempt_id`，再按 `portfolio_run_id = strategy_backtest_run_id` 和 `result_attempt_id` 读取现有 ClickHouse portfolio/calculation data plane。
12. 提供虚拟账户模板、默认市场费率模板、组合运行、组合净值和目标/订单/成交/持仓/事件明细 API。
13. Portfolio simulation engine 支持 `single_position_limit_pct` 一等字段；当该字段存在时，后端使用 `min((1 - cash_reserve_pct) / max_positions, single_position_limit_pct)` 计算单票目标权重，cap 留下的资金保留为现金。模拟器校验 `execution_date > signal_date`，Step 5 worker 负责把收盘确认信号映射到下一交易日开盘成交。
14. 通过 PostgreSQL outbox 和 NATS JetStream 分发组合净值和 strategy backtest 计算任务，由 `rearview-portfolio-worker` 消费 typed task；strategy backtest 路径从 transient rule snapshot 重新生成 signals，复用现有 ClickHouse 结果事实表并 append-only 写入新 result attempt。

## 非职责

1. 不重算 KDJ、MA、RSI、BOLL、MACD 或价格行为结构指标；这些由 Furnace/dbt 维护。
2. 不绕过 mart 层读取 raw、staging、intermediate 或 calculation 表。
3. 不提供前端交互；Racingline 承担 UI 工作台。
4. 不自动执行 PostgreSQL DDL migration；迁移由 `pipeline/migrate` 管理。
5. 不把当前 mart 查询值写回 PostgreSQL run snapshot。

## 主要依赖

| 依赖 | 用途 |
|---|---|
| PostgreSQL `rearview` database | 规则、版本、运行、chunk、day、pool、signal、strategy backtest control plane 和 metric catalog 状态 |
| ClickHouse `fleur_marts` | 日频行情、趋势、动量、成交量和价格行为结构指标 |
| `fleur_marts.mart_stock_basic_snapshot` | preview rows、pool page 和 preview security analysis 的证券名称、交易所代码、交易板块显示信息 |
| NATS JetStream | 组合净值与 strategy backtest 计算任务的 at-least-once 分发 |
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

只启动 `app/racingline_new` 和 Rearview HTTP 服务、用于策略创建工作台手工复验：

```bash
make racingline-new-rearview-dev
```

该命令会清理前后端端口并停止残留 portfolio worker，但不会启动新的 `rearview-portfolio-worker`。

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
| [../RFC/0026-racingline-strategy-pool-preview-step3.md](../RFC/0026-racingline-strategy-pool-preview-step3.md) | 从 `/strategies` Step 3 股池预览切入，定义 preview snapshot、全池分页、证券显示和 preview-only 个股上下文的 Implemented RFC |
| [../RFC/0027-racingline-strategy-simulation-position-step4.md](../RFC/0027-racingline-strategy-simulation-position-step4.md) | `/strategies` Step 4 模拟建仓、BacktestExecutionDraft 和 Step 5 handoff 边界 |
| [../RFC/0028-racingline-strategy-backtest-step5.md](../RFC/0028-racingline-strategy-backtest-step5.md) | `/strategies` Step 5 策略回测异步执行、backtest run control plane、NATS worker 和组合绩效指标已实现 RFC |
| [../plans/archive/0051-racingline-strategy-backtest-step5-implementation-plan.md](../plans/archive/0051-racingline-strategy-backtest-step5-implementation-plan.md) | Step 5 strategy backtest control plane、typed outbox、worker transient signal materialization、result wrapper 和 live smoke 已完成计划 |
| [../jobs/reports/2026-06-23-racingline-strategy-step5-backtest.md](../jobs/reports/2026-06-23-racingline-strategy-step5-backtest.md) | Step 5 默认动态近一年、period/benchmark rerun、wrapper API、ClickHouse/PG 和 worker 重投递验收报告 |
| [../plans/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md](../plans/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md) | 虚拟账户、组合运行、worker 和 Racingline 组合页面当前实施计划 |
| [../plans/archive/0050-racingline-strategy-simulation-position-step4-implementation-plan.md](../plans/archive/0050-racingline-strategy-simulation-position-step4-implementation-plan.md) | Racingline Step 4 模拟建仓 execution draft、Rearview validate contract 和前端 gate 已完成计划 |
| [../jobs/reports/2026-06-23-racingline-strategy-step4-draft-handoff.md](../jobs/reports/2026-06-23-racingline-strategy-step4-draft-handoff.md) | Strategy backtest validate contract、Step 4 handoff 和浏览器验收报告 |
| [../debt/0006-2026-06-23-strategies-step4-implemennt-drift.md](../debt/0006-2026-06-23-strategies-step4-implemennt-drift.md) | Step 4 模拟建仓实现漂移和修复方案，已 resolved |
| [../jobs/reports/2026-06-23-racingline-strategy-step4-drift-remediation.md](../jobs/reports/2026-06-23-racingline-strategy-step4-drift-remediation.md) | Rearview trend indicator stop loss validation、worker 转换和 portfolio engine 执行验收报告 |
| [../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md](../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md) | Rearview preview-only API、`[0, 100]` scoring clamp 和策略权重配置 Step 2 实施计划归档 |
| [../plans/archive/0047-racingline-strategy-pool-preview-step3-implementation-plan.md](../plans/archive/0047-racingline-strategy-pool-preview-step3-implementation-plan.md) | Step 3 preview snapshot、全池分页、证券显示和 preview security analysis 实施计划归档 |
| [../plans/archive/0048-racingline-strategy-step3-drift-remediation-plan.md](../plans/archive/0048-racingline-strategy-step3-drift-remediation-plan.md) | Step 3 preview timeline、10 条分页、K 线复权/MA 和 UI 职责收缩实施计划归档 |
| [../plans/archive/0049-racingline-strategy-step3-drift2-remediation-plan.md](../plans/archive/0049-racingline-strategy-step3-drift2-remediation-plan.md) | Step 3 二次漂移修正：交易板块、量柱、动态窗口、权重微调和 analysis payload 瘦身 |
| [../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md](../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md) | Rearview preview-only API 和 Racingline Step 2/3 闭环验收报告 |
| [../jobs/reports/2026-06-22-racingline-strategy-step3-preview.md](../jobs/reports/2026-06-22-racingline-strategy-step3-preview.md) | Step 1/2/3 真实接口闭环、preview pool page 和 preview security analysis 浏览器验收报告 |
| [../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md](../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md) | Step 3 漂移修正后的 Rearview timeline、pool-page 和 security-analysis 验收报告 |
| [../jobs/reports/2026-06-22-racingline-strategy-step3-drift2-remediation.md](../jobs/reports/2026-06-22-racingline-strategy-step3-drift2-remediation.md) | Step 3 二次漂移修正后的 Rearview security display、MA、analysis payload 和质量门禁验收报告 |
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
