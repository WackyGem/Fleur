# Racingline Strategy Portfolio Publish Dashboard Dagster

日期：2026-06-24

范围：

- `app/racingline_new` Step 5 建立组合入口、看板首页和策略详情页真实数据接入。
- `rearview` strategy portfolio control plane、dashboard/detail API、NATS outbox 和 portfolio worker daily run。
- `pipeline/migrate` strategy portfolio control-plane migration。
- `pipeline/scheduler` strategy portfolio daily run Dagster asset、job、schedule 和 definitions 注册。

约束：

- UI 业务展示数据只来自 Rearview API、PostgreSQL、ClickHouse 或 worker 计算结果。
- 不使用 mock、fixture、generated fake data、静态 fallback 或前端生成业务数据。
- 不改变 `racingline_new` 看板和详情页现有设计、排版、信息层级和交互骨架；仅替换数据来源并补 loading/error/empty/unavailable 状态。

## 结果

- 新增 `strategy_portfolio`、`strategy_portfolio_daily_run`、`strategy_portfolio_daily_task_outbox` migration，并已执行到 `0008_strategy_portfolio_cp`。
- Step 5 支持基于成功 backtest result 建立正式 strategy portfolio，发布后返回 `/dashboard`。
- Dashboard 首页不再依赖 `portfolioCards`；卡片净值、涨跌、绩效指标、信号和曲线由 Rearview dashboard API 返回。
- Strategy detail 页不再依赖 `holdingsByPortfolioId`、`buildStrategySignalPools()`、`buildDetailRebalanceRecords()`、`detailTradeCandidates` 或 `buildTradingDates()`。
- Dagster 注册稳定日分区 asset `rearview/strategy_portfolio_daily_runs`、job `strategy_portfolio__daily_run_job` 和 schedule `strategy_portfolio__daily_run_schedule`。

## 验证

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

结果：通过。

```bash
cd engines
cargo fmt --check
cargo check -p rearview-core -p rearview-server -p rearview-portfolio-worker
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：通过。

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format --check scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
```

结果：通过，`362 passed`，coverage `75.63%`。

```bash
cd pipeline/scheduler
uv run dg check defs
uv run dg list defs --json
```

结果：通过，已看到 `strategy_portfolio__daily_run_job`、`strategy_portfolio__daily_run_schedule` 和 `rearview_api` resource。

```bash
cd pipeline/migrate
uv run alembic upgrade head
```

结果：通过，执行 `0007_strategy_backtest_cp -> 0008_strategy_portfolio_cp`。

```bash
make docs-check
git diff --check
rg "portfolioCards|holdingsByPortfolioId|buildStrategySignalPools|buildDetailRebalanceRecords|detailTradeCandidates|buildTradingDates" app/racingline_new/src || true
```

结果：通过；mock 清退搜索无命中。

## 未覆盖

- 未做浏览器截图验收；本次通过代码保持现有组件结构、前端 lint/typecheck/test/build 和 mock 清退搜索验证。
- 未实际触发 Dagster schedule 调用 Rearview 服务；已完成 definitions 加载检查和 API resource/job/schedule 注册验证。

## 补充验证：0051 低位反转组合创建

日期：2026-06-24

用例来源：`docs/plans/archive/0051-racingline-strategy-backtest-step5-implementation-plan.md`

输入：

- Step 1：使用 0051 低位反转 AND 条件，包含 `kdj_j_value`、`pct_amplitude`、`pct_change`、`volume < prev_volume * 0.8`、均线结构和连跌天数条件。
- Step 2：使用 0051 评分项，分值为 `25/15/20/15/15/15/5`，KDJ 两段用 `kdj_j_value < -15` 与 `-15 <= kdj_j_value < -10` 互斥表达。
- Step 4：`initial_cash = 1000000`、`buy_signal_top_n = 5`、`max_positions = 5`、`single_position_limit_pct = 0.1`，默认费率、滑点和 4 条风控规则。
- Step 5：`period_key = 1y`、`benchmark_security_code = 000300.SH`，由 Rearview 动态解析区间。

结果：

- Backtest run：`0eeb7f71-028a-43fb-af91-e3ec609e4e4b`
- Result attempt：`01KVXE1TQ10M3S97SPR088PT0G`
- 回测区间：`2025-06-03..2026-06-01`
- Rearview options 最新可用交易日：`2026-06-01`
- Backtest nav 最新交易日：`2026-06-01`
- Strategy portfolio：`01KVXE63VJGPQQKZ4AQKE286WS`
- Portfolio code：`SP-20260624-Q53QK`
- Dashboard curve source：`source_backtest`
- Dashboard live status：`pending_first_run`

结论：0051 用例已使用真实 Rearview API、PostgreSQL、ClickHouse 和 worker 计算数据完成回测并创建策略组合；回测结果取到当前最新完整交易日 `2026-06-01`。当 source backtest 已覆盖最新交易日且尚无下一交易日数据时，组合允许创建并保持 `pending_first_run`，看板先展示 source backtest 曲线。
