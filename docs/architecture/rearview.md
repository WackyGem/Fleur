# Architecture: Rearview

状态：当前事实入口；Strategy Backtest Step 5 已通过 dev live smoke、worker 重投递验收和 Step 4/5 延时优化验收（2026-06-25）；Step 5 worker latency 已完成 1y/2y/3y live smoke、query_log、结果事实 hash、overview HTTP 计时和 bounded queue smoke（2026-06-26）；Strategy Portfolio publish 已完成 T+1 预检、pending 首次运行语义、backtest/live ClickHouse 事实族拆分和端到端 smoke（2026-06-27）；Strategy Portfolio detail 已提供 live 虚拟资金账户和对账单 read model（2026-06-29）；Strategy Portfolio publish preview 已按 15:00 前/后交易阶段解析允许信号日（2026-07-02）；Strategy Portfolio archive 已通过 `PATCH status=archived` 接入，archived detail 和手动指定 archived portfolio 创建 daily run 返回 `410 Gone`（2026-07-02）；0051 低位反转 example portfolio 已按 RFC/Plan 0044/0072 接入 example ensure API 与共享 strategy portfolio snapshot persistence service（2026-07-02）；Dagster 已将 production portfolio live NAV 清算收敛为 `daily__fetch_history_sources_to_marts_schedule_job` 的 `rearview/daily__portfolio_nav_liquidation` terminal step，继续调用现有 Rearview settlement-target、single-day daily-runs、status 和 fact-count APIs，不新增 batch liquidation API（2026-07-02）

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
10. 提供 Step 5 strategy backtest control plane：`GET /rearview/strategy-backtests/options`、`POST /rearview/strategy-backtests`、`GET /rearview/strategy-backtests/{id}` 和 `GET /rearview/strategy-backtests/{id}/status`。Create API 只落 PostgreSQL `strategy_backtest_run` 与 outbox 并返回 `202 Accepted`，服务端动态解析 `1y/2y/3y` 区间，固化 benchmark、range、rule/config hash、preflight snapshot 和 `client_request_id` 幂等语义；status view 只返回 Step 5 gate/status 必需字段。
11. 提供 Step 5 result wrapper API：`/overview`、`/nav`、`/rebalance-records`、`/targets`、`/orders`、`/trades`、`/positions`、`/events`、`/performance`、`/closed-trades` 和 `/trade-metrics`，先解析 backtest `current_result_attempt_id`，再按 `strategy_backtest_run_id` 和 `result_attempt_id` 读取 `fleur_backtest.backtest_*` 与 backtest calculation facts；`/overview?view=ui` 返回 Step 5 首屏 compact status、nav points、performance 和 rebalance read model，`/nav`、`/rebalance-records` 和 `/performance` 支持 `view=ui` compact response，full response 保留诊断字段。
12. 提供 Strategy Portfolio publish API：`GET /rearview/strategy-backtests/{id}/portfolio-publish-preview` 解析 source result attempt、T 日信号，并通过 `fleur_marts.mart_trade_calendar` 按 `Asia/Shanghai` 和 15:00 cutoff 解析 `required_source_signal_date` 与下一交易日；交易日 15:00 前允许上一交易日信号，15:00 后要求当天信号，非交易日使用最近完成交易日，数据多日落后继续阻断。`POST /rearview/strategy-portfolios` 要求提交 `expected_required_source_signal_date`、`expected_source_signal_date` 和 `expected_live_start_date`，创建时重新预检并固化 `initial_signal_date`、`live_start_date` 和 `pending_buy_signal_snapshot`。
13. 提供 Strategy Portfolio dashboard/detail live API。`pending_first_run` 组合的 dashboard 不回退 source backtest 曲线或绩效，nav/performance/positions/rebalance/virtual-account/statement 返回 `409 portfolio_pending_first_run`，signals 和 signal timeline 返回 publish snapshot；daily run 成功后读取 `fleur_portfolio.live_*` 事实族并通过 `current_live_result_attempt_id` 指向最新 live attempt。`PATCH /rearview/strategy-portfolios/{id}` 只接受 `{ "status": "archived" }`，归档只更新 PostgreSQL `strategy_portfolio.status` 和 `archived_at`，不删除 ClickHouse backtest/live facts；archived portfolio 的 detail 和详情 live 子资源返回 `410 Gone`。
14. 提供 `GET /rearview/strategy-portfolios/{id}/virtual-account`，按最新 live daily result 读取 `live_nav_daily` 的 `total_equity`、`position_market_value`、`cash_balance` 和 `daily_return`，并按同一 `account_date` 聚合 `live_position_day.unrealized_pnl` 为当前持仓未实现盈亏；当日盈亏使用最新与上一 nav 行 `total_equity` 差值，前端不反推。
15. 提供 `GET /rearview/strategy-portfolios/{id}/statement`，按最新 live daily result 解析 period key，返回区间 summary 和 operation rows。summary 中交易成功率按区间内卖出 trade row 的 realized PnL 胜率计算，盈利/亏损股票数只统计已卖出实现盈亏，持股天数按区间内 `live_nav_daily.position_count > 0` 的交易日计数；operation rows 的成交后持仓余额由后端基于同一 attempt 的全历史 trade window 派生。
16. 提供 Strategy Portfolio daily run 控制面 API：单日创建、range 创建、status、fact-counts 和 settlement-target。Production Dagster `rearview/daily__portfolio_nav_liquidation` 先调用 settlement-target，再调用 single-day daily-runs API 为所有 eligible active portfolios 创建或复用 latest target run；range API 保留给受控回补、验收和 0051 example 手动入口。手动指定 archived portfolio 创建 daily run 时返回 `410 Gone`，不创建 daily run 或 outbox。Fact-counts 返回最新 attempt 的 `live_nav_daily`、`live_trade` 和 `live_closed_trade` 行数，供 Dagster 判断 worker 写入完成。
17. 提供 `POST /rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure`，把固化 0051 data config 转换为正常 Rearview request DTO，复用 validate/canonicalization 和 `create_strategy_portfolio_from_snapshot` 持久化边界，按 `case_id + version + fixture_hash` 幂等创建或复用 `source_kind = example` 的 strategy portfolio；该 API 不直接触发 worker，后续清算仍走正式 daily-run range API。
18. 提供虚拟账户模板、默认市场费率模板、组合运行、组合净值和目标/订单/成交/持仓/事件明细 API。
19. Portfolio simulation engine 支持 `single_position_limit_pct` 一等字段；当该字段存在时，后端使用 `min((1 - cash_reserve_pct) / max_positions, single_position_limit_pct)` 计算单票目标权重，cap 留下的资金保留为现金。模拟器校验 `execution_date > signal_date`，Step 5 worker 负责把收盘确认信号映射到下一交易日开盘成交。模拟器内部使用私有 `PriceStore` 和 `TradeCalendarPlan` 降低价格索引 clone 与下一交易日扫描成本，外部 `PortfolioSimulationInput` / `PortfolioSimulationOutput` contract 不变。
20. 通过 PostgreSQL outbox 和 NATS JetStream 分发组合净值和 strategy backtest 计算任务，由 `rearview-portfolio-worker` 消费 typed task；strategy backtest 路径从 transient rule snapshot 重新生成 signals，默认使用 TopN-only worker signal SQL，只读取 `security_code`、`trade_date`、`score`、`signal_rank`，不拉回 Step 3 preview/explain 的 `score_breakdown`、`selected_metrics` 或 `raw_values`；按 stop-loss indicator metrics 动态投影 price bars 趋势列，写入 `fleur_backtest.backtest_*`；live daily run 以 `initial_signal_date` 作为 `run_start_date` 计算 T 日信号，归一化输出时从 `live_start_date` 开始净值归一并写入 `fleur_portfolio.live_*`，保留 target 的 T -> T+1 语义。
21. `rearview-server` 的 outbox dispatcher 在 create accepted 后可被进程内 notify 唤醒，保留 PG outbox 事务边界，并记录 pending scan、publish success/fail、NATS sequence 和 created-to-published elapsed；`GET /rearview/strategy-backtests/diagnostics/stale-active` 提供只读 stale active run 诊断。
22. `rearview-portfolio-worker` 使用 `REARVIEW_MAX_CONCURRENT_RUNS` 限制单进程内 task 并发，先获取 permit 再拉取 JetStream 消息，避免无界 delivered-but-unacked；ack 仍只在任务 handler 完成后执行。
23. Strategy backtest worker summary 写入 `worker_timing.version = 2`，包含 `stages_ms`、`simulation_ms`、`row_counts`、`query_ids` 和 `total_ms`；细粒度 simulation timing 只进入诊断 summary，不进入 Step 5 compact status 的必需字段。

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
| `fleur_marts.mart_trade_calendar` | 发布预检和 worker 日期边界使用的 A 股交易日历入口，支持解析上一交易日、最近完成交易日和 T 日下一交易日 |
| `fleur_marts.mart_stock_basic_snapshot` | preview rows、pool page 和 preview security analysis 的证券名称、交易所代码、交易板块显示信息 |
| `fleur_backtest.backtest_*` | strategy backtest result facts，按 `strategy_backtest_run_id` 和 `result_attempt_id` 读取 |
| `fleur_portfolio.live_*` | strategy portfolio live daily result facts，按 `strategy_portfolio_daily_run_id` 和 `result_attempt_id` 读取 |
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

`rearview-server` 启动时会幂等 ensure portfolio NATS stream，并运行进程内 outbox dispatcher；dispatcher 在无 pending 任务时等待 create notify 或 2s idle timeout，避免 busy loop。`rearview-portfolio-worker` 启动时会幂等 ensure stream 和 durable consumer，并按 `REARVIEW_MAX_CONCURRENT_RUNS` 限制拉取和处理中的任务数。

对账单验收可使用 dev-only seed 命令创建控制面样本。该命令从 `2025-01-02` 起查找首个真实低位反转买入信号日，并用交易日历取 T+1 作为 `live_start_date`；命令不写入 ClickHouse `live_*` facts，live facts 仍必须由 daily run worker 生成：

```bash
cd engines
cargo run -p rearview-server -- dev seed-statement-portfolio
```

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
| [../RFC/archive/0018-rust-stock-screening-service.md](../RFC/archive/0018-rust-stock-screening-service.md) | Rearview 后端服务设计 |
| [../RFC/archive/0019-racingline-rearview-frontend-workbench.md](../RFC/archive/0019-racingline-rearview-frontend-workbench.md) | Racingline 前端工作台设计 |
| [../RFC/archive/0020-racingline-run-result-security-analysis-page.md](../RFC/archive/0020-racingline-run-result-security-analysis-page.md) | Run result 个股分析页已实现 RFC |
| [../RFC/archive/0021-racingline-virtual-account-portfolio-rebalancing.md](../RFC/archive/0021-racingline-virtual-account-portfolio-rebalancing.md) | 虚拟账户、NATS 分发、`rearview-server` / `rearview-portfolio-worker` crate 拆分和组合净值计算 Proposed RFC |
| [../RFC/archive/0024-racingline-strategy-selection-step1.md](../RFC/archive/0024-racingline-strategy-selection-step1.md) | 从 `/strategies` Step 1 接通 metric catalog、RuleVersionSpec、crossing operator 和 explain 的 Proposed RFC |
| [../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md](../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md) | 从 `/strategies` Step 2 接通 `RuleVersionSpec.scoring.rules`，并定义点击股池预览时才执行选股、评分和排名的 Implemented RFC |
| [../RFC/archive/0026-racingline-strategy-pool-preview-step3.md](../RFC/archive/0026-racingline-strategy-pool-preview-step3.md) | 从 `/strategies` Step 3 股池预览切入，定义 preview snapshot、全池分页、证券显示和 preview-only 个股上下文的 Implemented RFC |
| [../RFC/archive/0027-racingline-strategy-simulation-position-step4.md](../RFC/archive/0027-racingline-strategy-simulation-position-step4.md) | `/strategies` Step 4 模拟建仓、BacktestExecutionDraft 和 Step 5 handoff 边界 |
| [../RFC/archive/0028-racingline-strategy-backtest-step5.md](../RFC/archive/0028-racingline-strategy-backtest-step5.md) | `/strategies` Step 5 策略回测异步执行、backtest run control plane、NATS worker 和组合绩效指标已实现 RFC |
| [../RFC/archive/0031-racingline-step4-step5-backtest-latency-slimming.md](../RFC/archive/0031-racingline-step4-step5-backtest-latency-slimming.md) | Step 4 到 Step 5 回测延时瘦身、字段审计、outbox 和 worker 性能治理依据 |
| [../plans/archive/0051-racingline-strategy-backtest-step5-implementation-plan.md](../plans/archive/0051-racingline-strategy-backtest-step5-implementation-plan.md) | Step 5 strategy backtest control plane、typed outbox、worker transient signal materialization、result wrapper 和 live smoke 已完成计划 |
| [../jobs/reports/2026-06-23-racingline-strategy-step5-backtest.md](../jobs/reports/2026-06-23-racingline-strategy-step5-backtest.md) | Step 5 默认动态近一年、period/benchmark rerun、wrapper API、ClickHouse/PG 和 worker 重投递验收报告 |
| [../plans/archive/0056-racingline-step4-step5-backtest-latency-optimization-plan.md](../plans/archive/0056-racingline-step4-step5-backtest-latency-optimization-plan.md) | Step 4/5 handoff、status/compact API、worker timing、动态 price bars 和 outbox 唤醒实施计划 |
| [../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md](../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md) | Step 4/5 回测延时优化验收报告 |
| [../plans/archive/0058-racingline-step5-backtest-worker-latency-optimization-plan.md](../plans/archive/0058-racingline-step5-backtest-worker-latency-optimization-plan.md) | Step 5 worker 热路径、MarketDataDemand、首屏读取和 pickup wait 治理完成计划 |
| [../jobs/reports/2026-06-26-racingline-step5-backtest-worker-latency-optimization.md](../jobs/reports/2026-06-26-racingline-step5-backtest-worker-latency-optimization.md) | Step 5 worker latency live smoke、query_log、结果事实 hash、MarketDataDemand 结论和 queue smoke 报告 |
| [../plans/archive/0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md](../plans/archive/0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md) | Step 5 建立组合弹层、T+1 publish preview、pending 首次运行、backtest/live 事实族拆分和 live run 信号窗口修正完成计划 |
| [../jobs/reports/2026-06-27-racingline-portfolio-control-plane-audit.md](../jobs/reports/2026-06-27-racingline-portfolio-control-plane-audit.md) | Strategy Portfolio 控制面字段归属、读写链路和迁移审计 |
| [../jobs/reports/2026-06-27-racingline-portfolio-publish-tplus1-smoke.md](../jobs/reports/2026-06-27-racingline-portfolio-publish-tplus1-smoke.md) | T+1 publish、pending endpoint、daily run、ClickHouse split 和 performance success 端到端验收报告 |
| [../plans/archive/0061-racingline-strategy-portfolio-virtual-account-panel-plan.md](../plans/archive/0061-racingline-strategy-portfolio-virtual-account-panel-plan.md) | Strategy Portfolio 虚拟资金账户 read model、API 和 Racingline 详情页区块完成计划 |
| [../jobs/reports/2026-06-28-racingline-portfolio-virtual-account-panel.md](../jobs/reports/2026-06-28-racingline-portfolio-virtual-account-panel.md) | 虚拟资金账户 API、pending 语义、桌面/移动端 UI 和质量门禁验收报告 |
| [../plans/archive/0062-racingline-strategy-portfolio-statement-plan.md](../plans/archive/0062-racingline-strategy-portfolio-statement-plan.md) | Strategy Portfolio 对账单 read model、Dagster 清算 fact-count 核验、T+1 建仓验收和 Racingline 详情页区块完成计划 |
| [../jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md](../jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md) | 对账单 API、first-signal T+1 seed、Dagster metadata、ClickHouse facts 和 CDP 截图验收报告 |
| [../RFC/archive/0043-racingline-strategy-detail-delete-action.md](../RFC/archive/0043-racingline-strategy-detail-delete-action.md) | Strategy Portfolio archive API、archived detail 410 和 Racingline 删除按钮设计 |
| [../plans/archive/0071-racingline-strategy-detail-delete-action-plan.md](../plans/archive/0071-racingline-strategy-detail-delete-action-plan.md) | Strategy Portfolio archive、manual daily run 410 和前后端联调完成计划 |
| [../jobs/reports/2026-07-02-racingline-strategy-detail-delete-action.md](../jobs/reports/2026-07-02-racingline-strategy-detail-delete-action.md) | Strategy Portfolio archive 和 Racingline 删除按钮质量门禁验收报告 |
| [../RFC/archive/0044-racingline-0051-low-reversal-regression-case.md](../RFC/archive/0044-racingline-0051-low-reversal-regression-case.md) | 0051 低位反转 data config、example ensure API 和 `example__portfolio_live_job` 设计 |
| [../plans/archive/0072-racingline-0051-low-reversal-example-live-job-plan.md](../plans/archive/0072-racingline-0051-low-reversal-example-live-job-plan.md) | 0051 example live job、共享 snapshot/persistence service 和清算验收实施计划 |
| [../jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md](../jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md) | 0051 example live job 建仓日、扩展窗口 T+1 买入 smoke 和默认 latest settlement target 修正记录 |
| [../RFC/archive/0045-strategy-portfolio-daily-nav-liquidation.md](../RFC/archive/0045-strategy-portfolio-daily-nav-liquidation.md) | Strategy Portfolio 日度 NAV 清算入口收敛设计 |
| [../plans/archive/0073-strategy-portfolio-daily-nav-liquidation-plan.md](../plans/archive/0073-strategy-portfolio-daily-nav-liquidation-plan.md) | Strategy Portfolio 日度 NAV 清算入口收敛实施计划 |
| [../jobs/reports/2026-07-02-strategy-portfolio-daily-nav-liquidation.md](../jobs/reports/2026-07-02-strategy-portfolio-daily-nav-liquidation.md) | `daily__portfolio_nav_liquidation` dry-run、direct smoke 和质量门禁报告 |
| [../plans/archive/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md](../plans/archive/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md) | 虚拟账户、组合运行、worker 和旧 Racingline 组合页面 Superseded 计划 |
| [../plans/archive/0050-racingline-strategy-simulation-position-step4-implementation-plan.md](../plans/archive/0050-racingline-strategy-simulation-position-step4-implementation-plan.md) | Racingline Step 4 模拟建仓 execution draft、Rearview validate contract 和前端 gate 已完成计划 |
| [../jobs/reports/2026-06-23-racingline-strategy-step4-draft-handoff.md](../jobs/reports/2026-06-23-racingline-strategy-step4-draft-handoff.md) | Strategy backtest validate contract、Step 4 handoff 和浏览器验收报告 |
| [../issues/archive/debt/0006-2026-06-23-strategies-step4-implemennt-drift.md](../issues/archive/debt/0006-2026-06-23-strategies-step4-implemennt-drift.md) | Step 4 模拟建仓实现漂移和修复方案，已 resolved |
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
