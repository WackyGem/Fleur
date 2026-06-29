# Plan 0062: Racingline 策略组合对账单实施计划

日期：2026-06-28

状态：Completed

完成日期：2026-06-29

领域：racingline, rearview, data-platform

关联系统：racingline, rearview, scheduler, clickhouse portfolio facts

代码根：

- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/scheduler/`

关联文档：

- [RFC 0036: Racingline 策略详情页账户对账单](../../RFC/0036-racingline-strategy-portfolio-statement.md)
- [RFC 0029: Racingline 回测结果发布为策略组合与 Dagster 日运行资产](../../RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md)
- [Plan 0061: Racingline 组合详情页虚拟资金账户实施计划](0061-racingline-strategy-portfolio-virtual-account-panel-plan.md)
- [Plan 0052: Racingline 策略组合发布、看板真实数据与 Dagster 日运行实施计划](0052-racingline-strategy-portfolio-publish-dashboard-dagster-plan.md)
- [Plan 0051: Racingline 策略回测 Step 5 异步执行实施计划](0051-racingline-strategy-backtest-step5-implementation-plan.md)
- [Racingline 系统地图](../../architecture/racingline.md)
- [Rearview 系统地图](../../architecture/rearview.md)
- [数据平台地图](../../architecture/data-platform.md)
- [验收报告](../../jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md)

## 背景

RFC 0036 确认在 Racingline 策略组合详情页 `/dashboard/strategies/:portfolioId` 增加“对账单”功能，包含：

1. 账户盈亏面板：支持 `本月`、`近三月`、`近半年`、`今年`、`全部` 区间；展示平均仓位、交易股票数、交易笔数、交易成功率、盈利股票数、亏损股票数和持股天数。
2. 操作记录：按调仓日展示买入或卖出的个股、价格、数量、金额、费用、成交后持仓余额和实现盈亏。

该功能必须读取 strategy portfolio live daily run 的最新成功 attempt。前端不能自行拼接交易流水、计算持仓余额或从 source backtest 回退展示。

本计划把 RFC 0036 拆为可实施的后端 statement read model、Dagster 清算作业改造、前端页面接入和生产化验收步骤。

## 文档 Review 结论

已 review RFC 0036、当前 Racingline 详情页、Rearview strategy portfolio API、ClickHouse live facts 和 Dagster `strategy_portfolio_daily_runs` asset，补充以下实现缺口：

| 缺口 | 处理结论 |
|---|---|
| 现有 `rebalance-records` 是调仓摘要，不包含价格、金额、费用、持仓余额和逐笔实现盈亏 | 新增 `/statement` read model，不复用 `rebalance-records` response |
| 交易成功率容易被误算成 closed lot 胜率 | 后端按区间内卖出 trade row 计算，卖出行 realized PnL 先按 `exit_trade_seq` 聚合 closed rows |
| 持股天数口径不是 closed trade 平均周期 | 后端按区间内 `live_nav_daily.position_count > 0` 的交易日数计算 |
| 持仓余额不能由当前页或当前区间前端累计 | ClickHouse 查询必须先覆盖当前 attempt 全量历史，再在外层按 period 分页过滤 |
| 盈利/亏损股票数只统计已卖出股票实现盈亏 | 后端按区间内 `live_closed_trade` 先聚合到 `security_code`，再统计正负股票数 |
| 第一版前端不做证券过滤 | API 可以预留后端参数空间，但页面第一版不暴露证券筛选入口 |
| Dagster 当前 asset 成功只代表 daily run 创建成功 | 调度链路必须等待 worker 终态并核验 ClickHouse facts，否则对账单验收会假阳性 |
| Rearview 当前没有 daily run 状态 HTTP 查询接口 | 新增 daily run status/batch status API，供 Dagster 等待终态和输出失败原因 |
| 长周期验收数据依赖 2025 年初建仓样本 | 验收改为从 `2025-01-02` 起查找首个真实买入信号日，再取该信号日的下一交易日作为 `live_start_date`；实际样本为 `initial_signal_date = 2025-01-07`、`live_start_date = 2025-01-08` |
| 当前 Dagster partition 起点为 `2026-06-24` | 新增 range/backfill 清算能力，不能依赖现有分区自然覆盖 2025 |
| 生产 schedule 不能按自然日盲跑 | 清算作业需要 settlement target resolver，跳过非交易日和依赖数据未齐日期 |
| 当前 daily run finalize 会无条件更新 portfolio latest pointer | 长周期并发回补前必须改为“只在完成 trade date 不早于当前 latest trade date 时更新 latest 指针”，避免较早交易日后完成而覆盖较晚结果 |
| Dagster 需要 settlement target，但当前 scheduler resource 只调用 Rearview HTTP | 优先由 Rearview 暴露 settlement target API，Dagster 复用该结果；避免 scheduler 直接复制 ClickHouse 依赖探测逻辑 |
| statement 验收依赖真实 live facts | 不新增 mock、fixture response 或前端静态 fallback；dev/test fixture 只允许用于受控创建 portfolio seed，不允许替代 worker 清算结果 |

## 目标

1. Rearview 新增 strategy portfolio statement endpoint，返回区间 summary 和 operation rows。
2. 后端统一解析 period key，返回 resolved `start_date/end_date/latest_live_trade_date`。
3. Summary 计算平均仓位、交易股票数、交易笔数、交易成功率、盈利股票数、亏损股票数和持股天数。
4. Operation rows 展示买入/卖出、价格、数量、手数、金额、费用、成交后持仓余额和实现盈亏。
5. 所有 statement 字段绑定同一个 `strategy_portfolio_daily_run_id` 和 `result_attempt_id`。
6. Dagster 清算作业支持 2025 长周期 backfill，并确认 worker 已成功写入 ClickHouse。
7. Racingline 详情页在虚拟资金账户之后增加对账单区块，支持 period 切换和分页。
8. 使用低位反转样例从 `2025-01-02` 起查找首个信号日并取 T+1 建仓，清算到晚于 `2026-01-02` 的交易日，完成端到端验收。

## 非目标

1. 不新增 raw source、dbt staging、dbt mart 或数据契约。
2. 不新增 ClickHouse 事实表；第一版读时聚合。
3. 不修改 worker 的撮合、费用、净值递推或 FIFO realized PnL 计算口径。
4. 不实现真实券商对账、资金流水、冻结金额、可取金额或融资融券字段。
5. 不让 Racingline 直接访问 PostgreSQL 或 ClickHouse。
6. 不从 source backtest 回退展示对账单。
7. 不在第一版前端实现操作记录按证券过滤。
8. 不把 Dagster 创建 daily run 成功视为清算完成。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| 详情页路由 | [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx) 渲染 `/dashboard/strategies/:portfolioId`。 |
| 已有详情页 query | 页面已接入 portfolio、nav、performance、virtual-account、signals、positions 和 rebalance-records queries。 |
| 当前对账单缺口 | `rebalance-records` 不包含成交价格、成交金额、费用、成交后持仓余额和逐笔 realized PnL。 |
| 前端 API 结构 | [rearview.ts](../../../app/racingline/src/api/rearview.ts)、[hooks.ts](../../../app/racingline/src/api/hooks.ts)、[queryKeys.ts](../../../app/racingline/src/api/queryKeys.ts) 已有 strategy portfolio API/hook/key 模式。 |
| Rearview live 结果解析 | [api/mod.rs](../../../engines/crates/rearview-core/src/api/mod.rs) 的 `resolve_strategy_portfolio_result()` 返回 latest daily run 和 current live attempt；pending-first-run 返回 `PortfolioPendingFirstRun`。 |
| live trade 查询 | [clickhouse/mod.rs](../../../engines/crates/rearview-core/src/clickhouse/mod.rs) 已有 `query_strategy_portfolio_live_trades()`，但没有 statement 专用余额和区间 summary 查询。 |
| live closed trade 查询 | [clickhouse/mod.rs](../../../engines/crates/rearview-core/src/clickhouse/mod.rs) 已有 `query_strategy_portfolio_live_closed_trades()` 和 `query_strategy_portfolio_live_trade_metrics()`。 |
| Dagster daily run asset | [assets.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/assets.py) 的 `strategy_portfolio_daily_runs` 只 POST 单个 `trade_date`，partition 起点为 `2026-06-24`。 |
| Dagster resource | [resources.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/resources.py) 只有 `create_strategy_portfolio_daily_runs()`，没有 range 创建、状态查询或 ClickHouse 写入核验。 |
| Dagster schedule | [definitions.py](../../../pipeline/scheduler/src/scheduler/defs/rearview/definitions.py) 每天 20:00 触发 partitioned job。 |
| worker daily run | `rearview-portfolio-worker` 对每个 daily run 从 `run_start_date` 到 `trade_date` 全窗口重算，再按 `live_start_date` 归一化输出。 |

## API Contract

新增：

```http
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/statement?period=month&limit=100&offset=0
```

`period` 允许值：

| key | UI 文案 |
|---|---|
| `month` | 本月 |
| `three_months` | 近三月 |
| `six_months` | 近半年 |
| `ytd` | 今年 |
| `all` | 全部 |

Response：

```json
{
  "source": "live_daily_run",
  "strategy_portfolio_id": "01KW...",
  "strategy_portfolio_daily_run_id": "01KW...",
  "result_attempt_id": "01KW...",
  "period": {
    "key": "three_months",
    "label": "近三月",
    "start_date": "2026-03-26",
    "end_date": "2026-06-26",
    "latest_live_trade_date": "2026-06-26"
  },
  "summary": {
    "average_position_pct": 0.7421,
    "traded_security_count": 12,
    "trade_count": 18,
    "trade_win_rate": 0.5833,
    "winning_security_count": 7,
    "losing_security_count": 4,
    "holding_days": 57
  },
  "operations": {
    "items": [
      {
        "portfolio_trade_id": "01KW...",
        "trade_seq": 12,
        "trade_date": "2026-06-26",
        "security_code": "000001.SZ",
        "security_name": "平安银行",
        "side": "sell",
        "execution_price": 12.34,
        "quantity": 1000,
        "lot_size": 100,
        "lot_count": 10,
        "gross_amount": 12340.0,
        "commission": 5.0,
        "stamp_duty": 12.34,
        "transfer_fee": 0.12,
        "total_fee": 17.46,
        "position_balance_quantity": 0,
        "realized_pnl": 245.6,
        "reason": "risk_exit"
      }
    ],
    "limit": 100,
    "offset": 0,
    "has_more": false
  }
}
```

错误语义：

| 场景 | 语义 |
|---|---|
| portfolio 尚无 live daily run | 保持 `409 portfolio_pending_first_run` |
| period key 非法 | `400 validation` |
| limit/offset 非法 | `400 validation` |
| latest attempt 没有 nav row | 显式 data error，不返回空 summary |
| 区间无卖出 | `trade_win_rate = null` |
| 区间无持仓交易日 | `holding_days = 0` |

## 实施阶段

### Phase 1: Rearview statement period 和 response contract

目标：先固定 API 类型、period 解析和错误语义。

实施项：

1. 在 `engines/crates/rearview-core/src/api/mod.rs` 注册 `GET /rearview/strategy-portfolios/{strategy_portfolio_id}/statement`。
2. 新增 request query type：`period`、`limit`、`offset`。
3. 新增 response types：`StrategyPortfolioStatementResponse`、`StrategyPortfolioStatementPeriod`、`StrategyPortfolioStatementSummary`、`StrategyPortfolioStatementOperation`、`StatementOperationsPage`。
4. 新增 period enum/helper，使用 latest live nav date 和 portfolio `live_start_date` 解析自然日起点，并裁剪到 live start。
5. `limit` 默认 `100`，最大值建议 `200`；`offset` 默认 `0`。
6. endpoint 复用 `resolve_strategy_portfolio_result()`，pending-first-run 不特殊兜底。

测试策略：

1. Rust 单测覆盖 `month`、`three_months`、`six_months`、`ytd`、`all` 的 start/end 解析。
2. 覆盖 live start 裁剪：当 period 起点早于 `live_start_date` 时使用 `live_start_date`。
3. 覆盖非法 period、limit 超限、offset 为负的 validation。

完成标准：

1. API contract 编译通过，错误语义明确。
2. 不修改现有 `/nav`、`/virtual-account`、`/rebalance-records` response。

### Phase 2: ClickHouse statement summary 和 operation queries

目标：新增 statement 专用 read queries，避免前端或通用分页 API 做业务派生。

实施项：

1. 在 `clickhouse/mod.rs` 新增 summary query，绑定 `strategy_portfolio_daily_run_id`、`result_attempt_id`、`start_date`、`end_date`。
2. Summary 字段口径：
   - `average_position_pct = avg(live_nav_daily.gross_exposure)`。
   - `traded_security_count = countDistinct(live_trade.security_code)`。
   - `trade_count = count(*) from live_trade`。
   - `trade_win_rate = 区间内 realized_pnl > 0 的 sell trade row 数 / 区间内 sell trade row 数`。
   - `winning_security_count` 和 `losing_security_count` 先按 `security_code` 汇总区间内 closed rows 的 realized PnL。
   - `holding_days = countIf(live_nav_daily.position_count > 0)`。
3. 新增 operations query：
   - 先对当前 attempt 全量 trade history 按 `security_code, trade_date, trade_seq` 窗口累计持仓余额。
   - 再按 period 过滤、倒序分页。
   - 卖出行 realized PnL 按 `live_closed_trade.exit_trade_seq` 和 `security_code` 汇总。
   - 买入行 `realized_pnl = null`。
4. 补证券名称：沿用现有 `required_security_display_map()` / `security_display_name()` 模式。
5. `lot_size` 第一版返回 `100`，`lot_count = quantity / 100`；若后续从 execution config 读取，则必须用同一 portfolio 的固化 execution config。
6. `has_more` 使用 `limit + 1` 查询。

测试策略：

1. SQL builder/string 单测：必须包含 run id、attempt id、period filter 和全历史余额窗口。
2. Rust helper 单测：同一 sell trade 关闭多个 closed lots 时 realized PnL 合计后只算一笔成功或失败。
3. Rust helper 单测：区间首行前有历史买入时 `position_balance_quantity` 不从零开始。
4. Rust helper 单测：无卖出时 `trade_win_rate = None`，无持仓交易日时 `holding_days = 0`。

完成标准：

1. Summary 和 operations 都来自同一 latest live attempt。
2. 前端不需要也不能重算持仓余额、实现盈亏、交易成功率和股票盈亏数。

### Phase 3: Rearview daily run range/status APIs

目标：补齐 Dagster 清算作业等待 worker 完成所需的控制面 API。

实施项：

1. 新增 `POST /rearview/strategy-portfolios/daily-runs/range`。
2. Request 包含 `start_date`、`end_date`、可选 `client_request_id`、可选 `max_trade_dates`。
3. Range API 从交易日历解析真实交易日列表，只为交易日创建 daily runs。
4. 返回 created/skipped daily run ids、resolved trade dates、active portfolio count 和 client request id。
5. 新增 `GET /rearview/strategy-portfolios/daily-runs/{daily_run_id}` 或 batch status endpoint。
6. Status response 至少包含 `status`、`dispatch_status`、`current_result_attempt_id`、`error_type`、`error_message`、`signal_summary`、`data_coverage_summary`、`created_at`、`updated_at`、`completed_at`。
7. 新增 settlement target API，例如 `GET /rearview/strategy-portfolios/daily-runs/settlement-target`，返回交易日历、行情、required marts、benchmark returns 和无风险利率共同可用上限，以及每类依赖的 latest date。
8. 对 range 请求设置最大交易日数保护；长周期验收可通过 chunk 方式执行。
9. 修改 daily run finalize 逻辑：只有当前 run 的 `trade_date` 不早于 portfolio 当前 latest daily run 的 `trade_date` 时，才更新 `strategy_portfolio.latest_daily_run_id` 和 `current_live_result_attempt_id`。较早日期回补成功仍应标记 daily run succeeded，但不得把组合 latest attempt 回退。

测试策略：

1. Postgres 单测或集成测试覆盖 range 创建幂等：重复执行 created 为 0、skipped 增加。
2. 覆盖非交易日被过滤，不创建 daily run。
3. 覆盖 status endpoint 返回 failed error details。
4. 覆盖 `run_start_date = portfolio.initial_signal_date`，保证 seed signal 的 T+1 建仓日能正确执行。
5. 覆盖 daily run finalize 顺序：较晚 trade date 先成功后，较早 trade date 再成功不能覆盖 portfolio latest pointer。
6. 覆盖 settlement target API 返回每类依赖 latest date，且 target date 取共同下限。

完成标准：

1. Dagster 不再需要直接查询 Rearview PostgreSQL 才能等待 daily run 终态。
2. Range API 对重复调用幂等，不重复写 outbox。
3. 并发或乱序完成的 range backfill 不会把 strategy portfolio latest pointer 回退。

### Phase 4: Dagster 清算作业改造

目标：让 Dagster 清算成功代表 worker 已完成并写入 ClickHouse，而不是只代表 daily run 创建成功。

实施项：

1. 在 `pipeline/scheduler/src/scheduler/defs/rearview/resources.py` 增加：
   - `create_strategy_portfolio_daily_runs_range()`。
   - `get_strategy_portfolio_daily_run_status()` 或 batch status 方法。
   - `get_strategy_portfolio_settlement_target()`。
   - 必要的 ClickHouse fact verification 查询资源或 Rearview verification API client。
2. 保留现有单日 daily asset，同时新增 range/backfill 作业入口，供从 2025 年初信号样本的 T+1 建仓日到目标日的验收回补使用。
3. `StrategyPortfolioDailyRunConfig` 增加 `start_date`、`end_date`、`wait_for_completion`、`poll_interval_seconds`、`timeout_seconds`、`chunk_size`。
4. 清算作业执行顺序：
   - 解析 settlement target date。
   - 展开交易日列表。
   - 按 chunk 调用 Rearview range API。
   - 轮询 daily run status 到 succeeded/failed/timeout。
   - 查询最新 attempt 的 ClickHouse live facts row count。
   - 写 Dagster materialization metadata。
5. 生产 schedule 仍可每天 20:00 触发，但目标日期应解析为“最近可清算交易日”；非交易日或依赖未齐时写 skip metadata。
6. 失败策略：
   - 任一 newly created daily run failed，Dagster run fail。
   - 超时仍 queued/running，Dagster run fail with timeout metadata。
   - 只有 skipped 且已有更新或同日 succeeded，可 materialize success，并记录 skipped原因。
7. metadata 至少包含 `requested_start_date`、`requested_end_date`、`resolved_trade_dates`、`settlement_target_date`、`created_run_count`、`skipped_run_count`、`daily_run_ids`、`succeeded_run_count`、`failed_run_count`、`timeout_run_count`、`latest_result_attempt_id`、`nav_row_count`、`trade_row_count`、`closed_trade_row_count`。

测试策略：

1. Scheduler unit tests 覆盖 range config 解析、metadata 汇总、failed status 转 Dagster failure、timeout 转 failure。
2. Resource tests 覆盖新 API path 和 payload。
3. 对 production schedule 的 target resolver 做单测：非交易日 skip，数据未齐 skip/fail with reason。

完成标准：

1. `strategy_portfolio__daily_run_job` 或 range/backfill 变体能证明 daily runs 进入 `succeeded`。
2. 验收报告能从 Dagster metadata 直接看到 latest attempt 和 ClickHouse row counts。

### Phase 5: 2025 信号日 T+1 建仓验收数据生成

目标：产生可支撑对账单验收的超过一年 live facts。

实施项：

1. 用 0051 的低位反转 Step 1、Step 2 和 Step 4 配置创建 source backtest。
2. 从 `2025-01-02` 起查找低位反转规则的首个真实买入信号日，再通过交易日历取下一交易日作为建仓日。
3. 验收实现明确标记为 dev/test 的 portfolio seed 命令；该命令只能创建控制面 seed，不能伪造 ClickHouse live facts。
4. 发布 portfolio 后，通过 Dagster range/backfill 清算从 T+1 建仓日到 settlement target date。
5. settlement target date 应取交易日历、行情、required marts、`000300.SH` benchmark 和无风险利率共同可用上限。

验收数据标准：

1. `strategy_portfolio.initial_signal_date` 为 `2025-01-02` 之后首个真实信号日；本次验收为 `2025-01-07`。
2. `strategy_portfolio.live_start_date` 为该信号日的 T+1 交易日；本次验收为 `2025-01-08`。
3. 最新成功 daily run trade date 晚于 `2026-01-02`。
4. 最新 attempt 的 `live_nav_daily` 覆盖 T+1 建仓日到最终清算日。
5. 最新 attempt 的 `live_trade` 有 buy/sell rows，且 quantity 均为 100 的整数倍。
6. 最新 attempt 的 `live_closed_trade` 有 realized PnL。

### Phase 6: Racingline statement API client and UI

目标：在策略详情页展示对账单 summary 和操作记录。

实施项：

1. 在 `app/racingline/src/types/rearview.ts` 增加 statement response/types。
2. 在 `app/racingline/src/api/rearview.ts` 增加 `getStrategyPortfolioStatement(strategyPortfolioId, query)`。
3. 在 `app/racingline/src/api/queryKeys.ts` 增加 `strategyPortfolioStatement(strategyPortfolioId, period, limit, offset)`。
4. 在 `app/racingline/src/api/hooks.ts` 增加 `useStrategyPortfolioStatementQuery()`。
5. 在 `strategy-detail-page.tsx` 中新增“对账单”区块，建议放在“虚拟资金账户”之后、“持仓记录”之前。
6. 账户盈亏面板用 period segmented control 切换 `本月/近三月/近半年/今年/全部`。
7. 操作记录表展示调仓日、买卖方向、个股、价格、数量、手数、金额、费用、持仓余额、实现盈亏。
8. 第一版不展示证券筛选入口。
9. Pending-first-run 显示空态，不显示 0 或 mock rows。
10. Loading/error/empty 状态不能影响已有净值、虚拟资金账户和持仓记录区块。

测试策略：

1. API path 单测覆盖 `/rearview/strategy-portfolios/{id}/statement?period=...&limit=...&offset=...`。
2. UI 单测或组件测试覆盖 period 切换、pending-first-run、无操作记录、卖出 realized PnL 正负显示。
3. 使用真实 API response 做浏览器 smoke，不引入前端 mock 成功路径。

完成标准：

1. 前端只展示后端返回的 summary 和 operations。
2. period 切换只传 key，不在前端解析日期区间。
3. 文本在 desktop/mobile 下不重叠。

### Phase 7: 端到端验收和报告

目标：用真实 Rearview、worker、Dagster 和 ClickHouse 完成生产化验收。

验收动作：

1. 启动完整 dev 环境：`make racingline-dev`。
2. 确认 `rearview-portfolio-worker` 正常消费 daily run task。
3. 用 dev seed 命令从 `2025-01-02` 起查找首个真实信号日，并取 T+1 作为建仓日；seed 只创建 PostgreSQL 控制面，不写入 ClickHouse `live_*` facts。
4. 通过 Dagster range/backfill 清算到 settlement target date。
5. 使用 Playwright 连接现有 CDP 浏览器完成截图证据链，不使用本机新开浏览器：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

6. 截图保存到 `docs/jobs/reports/assets/<date>/statement/`，验收报告使用相对路径 `assets/<date>/statement/...` 链接，文件名固定为：
   - `statement-acceptance-06-summary-periods-desktop.png`：桌面视口下对账单 summary 和 period 切换。
   - `statement-acceptance-07-operations-all-desktop.png`：桌面视口下全部区间操作记录。
   - `statement-acceptance-08-operations-period-desktop.png`：桌面视口下非全部区间操作记录。
   - `statement-acceptance-09-summary-mobile.png`：移动视口下账户盈亏面板。
   - `statement-acceptance-10-operations-mobile.png`：移动视口下操作记录。
   - `statement-acceptance-01` 到 `05` 的 UI 流程截图仅适用于从 Step 1/2/4/publish UI 重跑的验收路径；本次 T/T+1 调整使用 dev seed，因此用 seed 命令输出、Dagster metadata 和 ClickHouse SQL 结果替代，不伪造 UI 截图。
7. Playwright 验收必须记录 desktop 和 mobile viewport，检查页面无文本重叠、无 horizontal overflow、console 无未处理错误、statement API network response 使用真实 Rearview 数据。
8. 写入 `docs/jobs/reports/` 验收报告，链接 Playwright 截图证据链，并记录 CDP endpoint、viewport、portfolio id、period key、daily run ids 和 result attempt id。

完成标准：

1. Dagster metadata 显示 daily runs 已进入 `succeeded`，不是只显示 created。
2. 最新 attempt 的 ClickHouse facts 覆盖超过一年。
3. 对账单 `全部` 区间和至少一个非全部区间展示正确。
4. 无 mock 成功路径。
5. Playwright 截图证据链完整，覆盖 desktop/mobile，且验收报告逐张链接。

### Phase 8: 文档收敛

目标：实现完成后更新项目事实。

实施项：

1. 更新 [racingline.md](../../architecture/racingline.md) 的详情页对账单事实。
2. 更新 [rearview.md](../../architecture/rearview.md) 的 statement endpoint 和 daily run range/status API。
3. 更新 [data-platform.md](../../architecture/data-platform.md) 或 scheduler 文档中的清算作业语义。
4. 归档本计划到 `docs/plans/archive/`，并把完成报告链接写入 `docs/plans/README.md`。

## 验证命令

文档：

```bash
make docs-check
git diff --check
```

Rust：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Scheduler：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
cd scheduler
uv run dg check defs
```

Frontend：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

Live smoke：

```bash
make racingline-dev
cd engines
cargo run -p rearview-portfolio-worker -- run --once
cd ../pipeline
uv run dg launch --job strategy_portfolio__daily_run_job
```

Playwright 证据链：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

实际 Dagster range/backfill 命令以实施后的 job/asset 名称为准，验收报告必须记录最终命令、portfolio id、daily run ids、result attempt id 和 ClickHouse row counts。

## 完成标准

1. `GET /rearview/strategy-portfolios/{id}/statement` 返回 RFC 0036 定义的 summary 和 operation rows。
2. Period 由后端解析，前端只传 period key。
3. 交易成功率按区间卖出 trade row 计算，不按 closed lot 计算。
4. 持股天数按区间内有仓位交易日数计算。
5. 持仓余额由后端基于全历史 trade window 派生。
6. Dagster 清算作业能创建 range daily runs、等待 worker succeeded 并核验 ClickHouse facts。
7. 从 `2025-01-02` 起查到首个信号日并取 T+1 建仓的验收样例产生超过一年 live facts。
8. Racingline 详情页展示对账单 summary 和 operation rows，pending/loading/error/empty 状态明确。
9. Rust、scheduler、frontend 和 docs 验证通过。
10. 验收报告包含命令、数据范围、portfolio id、daily run ids、attempt id、Dagster metadata、SQL 验证结果和 Playwright 截图证据链。

## 计划 Review 补充缺口

本计划草案按 RFC 0036 拆分后，再按实现链路 review，额外补充以下必须处理的缺口：

1. Daily run 状态 API 是 Dagster 等待终态的前置，不应留到验收脚本里直接查 PostgreSQL。
2. Range/backfill 清算和生产 schedule 是两种入口：前者服务 2025 长周期验收，后者服务日常清算；两者都必须复用 settlement target resolver。
3. ClickHouse 写入核验必须成为 Dagster 成功条件，否则 statement API 仍可能读不到 facts。
4. 如果用 dev/test fixture 创建 seed portfolio，fixture 只能创建控制面，不允许写入或伪造 `live_*` facts。
5. Frontend 第一版不做证券过滤，但后端 operation query 需要稳定分页；否则后续加筛选会破坏余额口径。
6. `live_trade_metric.full_period` 不能用于 period summary；所有 period 指标第一版都要 read-time 重算。
7. 对账单 summary 和 operation rows 必须使用同一个 resolved latest attempt，不能 summary 读最新、operations 读另一 attempt。
8. 验收样例不能只证明 UI 可见；必须证明 Dagster、worker、PostgreSQL control plane 和 ClickHouse data plane 已闭环。
9. Daily run finalize 当前无条件更新 portfolio latest pointer；range/backfill 支持并发或乱序完成前必须修正，避免最新可见 attempt 被较早 trade date 覆盖。
10. Settlement target resolver 不应在 Rearview 和 Dagster 各写一套；第一版优先由 Rearview 暴露 API，Dagster 只消费结果并写 metadata。
