# RFC 0036: Racingline 策略详情页账户对账单

状态：Proposed（数据盘点，2026-06-28）
领域：racingline, rearview
关联系统：racingline, rearview, clickhouse portfolio facts
代码根：
- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-portfolio-worker/`
架构事实：
- docs/architecture/racingline.md
- docs/architecture/rearview.md
关联文档：
- docs/RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md
- docs/RFC/archive/0035-racingline-strategy-portfolio-virtual-account-panel.md
- docs/plans/archive/0061-racingline-strategy-portfolio-virtual-account-panel-plan.md

## 摘要

Racingline 策略组合详情页 `/dashboard/strategies/:portfolioId` 已经展示 live 净值、绩效、策略信号、虚拟资金账户和持仓记录。下一步希望在详情页增加“对账单”功能，包含两部分：

1. 账户盈亏面板：支持区间切换，本月、近三月、近半年、今年、全部；展示平均仓位、交易股票数、交易笔数、交易成功率、盈利股票数、亏损股票数和持股天数。
2. 操作记录：按调仓日展示买入或卖出的个股、价格、数量、金额、费用、持仓余额和实现盈亏。A 股数量按一手 100 股展示，最小交易单位仍使用后端 execution config 中的 `lot_size = 100` 和 `min_trade_lots = 1`。

当前 ClickHouse live portfolio 事实已经能支撑大部分字段，但现有 Racingline 和 Rearview API 只提供账户快照和调仓摘要，不提供对账单专用的区间聚合与逐笔流水 read model。第一版建议新增 Rearview statement read model；不需要新增 raw/dbt 数据源，也不需要在前端直接访问 ClickHouse。

本文只记录资产状态、数据支撑和后端缺口，不实施代码。

## 当前资产状态

### Racingline 页面

策略组合详情页位于 `app/racingline/src/routes/strategy-detail-page.tsx`。当前页面已接入以下 Rearview query：

| Query | 后端接口 | 当前用途 |
|---|---|---|
| `useStrategyPortfolioQuery()` | `GET /rearview/strategy-portfolios/{id}` | 读取 portfolio record、live 状态、source backtest/live segment |
| `useStrategyPortfolioNavQuery()` | `GET /rearview/strategy-portfolios/{id}/nav` | 展示策略净值和基准净值曲线 |
| `useStrategyPortfolioPerformanceQuery()` | `GET /rearview/strategy-portfolios/{id}/performance` | 展示收益、风险和相对市场指标 |
| `useStrategyPortfolioVirtualAccountQuery()` | `GET /rearview/strategy-portfolios/{id}/virtual-account` | 展示当前账户资产、持股市值、现金和当日盈亏 |
| `useStrategyPortfolioSignalsQuery()` | `GET /rearview/strategy-portfolios/{id}/signals` | 展示 live signals 或 pending buy signals |
| `useStrategyPortfolioSignalTimelineQuery()` | `GET /rearview/strategy-portfolios/{id}/signal-timeline` | 展示信号日期轴 |
| `useStrategyPortfolioPositionsQuery()` | `GET /rearview/strategy-portfolios/{id}/positions` | 读取最新持仓数量 |
| `useStrategyPortfolioRebalanceRecordsQuery()` | `GET /rearview/strategy-portfolios/{id}/rebalance-records` | 展示调入、持有、调出摘要 |

页面上的“虚拟资金账户”已经展示：

- 账户资产
- 持股市值
- 可用金额
- 持仓盈亏
- 当日盈亏
- 当日盈亏比

页面上的“持仓记录”不是对账单流水。它展示调仓日期、调入/持有/调出分组、股票、调仓理由、持仓天数、涨跌幅、成本价、现价和收益贡献；没有价格、成交金额、费用、成交后持仓余额或逐笔实现盈亏。因此对账单不能直接复用当前 `rebalance-records` response。

### Rearview API

Rearview 当前 strategy portfolio live API 已有：

| API | 状态 | 对账单复用价值 |
|---|---|---|
| `GET /rearview/strategy-portfolios/{id}/nav` | 已实现 | 可支撑区间内仓位、净值、现金、市值和费用聚合，但现有 response 只暴露净值曲线字段 |
| `GET /rearview/strategy-portfolios/{id}/performance` | 已实现 | 可复用整体绩效字段，但不是对账单要求的交易统计 |
| `GET /rearview/strategy-portfolios/{id}/virtual-account` | 已实现 | 可复用当前账户快照口径，不能覆盖区间统计 |
| `GET /rearview/strategy-portfolios/{id}/positions` | 已实现 | 可读取某一交易日持仓明细；不适合作为前端分页聚合来源 |
| `GET /rearview/strategy-portfolios/{id}/rebalance-records` | 已实现 | 可展示调仓摘要；字段不满足操作记录 |
| `GET /rearview/strategy-portfolios/{id}/trades` | 未暴露 | ClickHouse query method 已存在，但 strategy portfolio HTTP route 未暴露 |
| `GET /rearview/strategy-portfolios/{id}/closed-trades` | 未暴露 | ClickHouse query method 已存在，但 strategy portfolio HTTP route 未暴露 |
| `GET /rearview/strategy-portfolios/{id}/trade-metrics` | 未暴露 | ClickHouse query method 已存在，但 strategy portfolio HTTP route 未暴露 |
| `GET /rearview/strategy-portfolios/{id}/statement` | 不存在 | 建议新增对账单 read model |

`resolve_strategy_portfolio_result()` 已经能从 PostgreSQL strategy portfolio control plane 解析当前 live daily run：

- `strategy_portfolio_daily_run_id`
- `current_live_result_attempt_id`
- `benchmark_security_code`
- `live_start_date`
- latest daily run trade date

对账单也应复用这个解析结果，保证读取的是当前组合 live attempt，不回退 source backtest。

### ClickHouse 事实表

Strategy portfolio live daily run 结果写入 `fleur_portfolio.live_*` 和 live calculation facts。已确认可复用的表如下：

| 表 | 关键字段 | 对账单用途 |
|---|---|---|
| `fleur_portfolio.live_nav_daily` | `trade_date`, `cash_balance`, `position_market_value`, `total_equity`, `nav`, `daily_return`, `gross_exposure`, `position_count`, `turnover`, `fee_amount` | 账户区间、平均仓位、费用聚合、区间边界 |
| `fleur_portfolio.live_trade` | `trade_date`, `trade_seq`, `portfolio_trade_id`, `portfolio_order_id`, `security_code`, `side`, `quantity`, `execution_price`, `gross_amount`, `commission`, `stamp_duty`, `transfer_fee`, `total_fee`, `reason` | 操作记录主体、交易笔数、交易股票数 |
| `fleur_portfolio.live_order` | `execution_date`, `security_code`, `side`, `order_quantity`, `order_amount`, `reference_price`, `status`, `event_ref` | 第一版对账单交易成功率不使用；仅订单成交率扩展场景需要审计 order status 语义 |
| `fleur_portfolio.live_position_day` | `trade_date`, `security_code`, `quantity`, `cost_basis`, `average_entry_price`, `close_price`, `market_value`, `unrealized_pnl`, `holding_days` | 期末持仓、当前持仓天数、与持仓余额核对 |
| `fleur_portfolio.live_closed_trade` | `entry_trade_seq`, `exit_trade_seq`, `security_code`, `entry_date`, `exit_date`, `quantity`, `entry_gross_amount`, `exit_gross_amount`, `entry_fee`, `exit_fee`, `realized_pnl`, `holding_days` | 实现盈亏、已卖出股票盈利/亏损统计 |
| `fleur_portfolio.live_trade_metric` | `window_key`, `closed_trade_count`, `winning_trade_count`, `losing_trade_count`, `win_rate_closed_trades`, `average_holding_days` | 当前只写 `full_period`，不能直接覆盖本月/近三月/近半年/今年；对账单第一版不直接读取 |

worker 已在 strategy backtest、strategy portfolio daily run 和旧 portfolio run 三条路径调用 `compute_trade_calculation_outputs()`，按 FIFO lot 生成 closed trade ledger 和 trade metric。`live_closed_trade` 因此可以支撑卖出实现盈亏，但买入行没有 realized PnL，应返回 `NULL` 或 `0` 并在 UI 语义上区分。

## 口径建议

### 区间

建议后端定义稳定 period enum：

| key | UI 文案 | 起止日期 |
|---|---|---|
| `month` | 本月 | `latest_live_trade_date` 所在自然月第一天到 `latest_live_trade_date`，下限裁剪到 `live_start_date` |
| `three_months` | 近三月 | `latest_live_trade_date` 向前回看三个月到 `latest_live_trade_date`，下限裁剪到 `live_start_date` |
| `six_months` | 近半年 | `latest_live_trade_date` 向前回看六个月到 `latest_live_trade_date`，下限裁剪到 `live_start_date` |
| `ytd` | 今年 | `latest_live_trade_date` 所在年份 1 月 1 日到 `latest_live_trade_date`，下限裁剪到 `live_start_date` |
| `all` | 全部 | `live_start_date` 到 `latest_live_trade_date` |

`latest_live_trade_date` 建议来自当前 `result_attempt_id` 的最新 `live_nav_daily.trade_date`，而不是前端本地日期。自然日起点不需要前端猜交易日；ClickHouse 查询使用 `trade_date >= start_date AND trade_date <= end_date`，没有交易的自然日不会产生行。

如果区间内没有交易、卖出或 closed trade：

- 交易股票数为 `0`。
- 交易笔数为 `0`。
- 区间内没有卖出笔数时，交易成功率为 `NULL`，不要展示成 `0%`。
- 盈利股票数和亏损股票数为 `0`。
- 区间内没有持仓交易日时，持股天数为 `0`。

### 账户盈亏面板

建议第一版把“账户盈亏面板”定义为区间交易质量和仓位统计，不重复当前“虚拟资金账户”的实时资产快照。

| UI 字段 | 建议 API 字段 | 数据来源 | 支撑状态 |
|---|---|---|---|
| 平均仓位 | `average_position_pct` | `avg(live_nav_daily.gross_exposure)`，同一 `strategy_portfolio_daily_run_id`、`result_attempt_id` 和区间 | 已有字段，需新增聚合 query |
| 交易股票数 | `traded_security_count` | `countDistinct(live_trade.security_code)` | 已有字段，需新增聚合 query |
| 交易笔数 | `trade_count` | `count(*) FROM live_trade` | 已有字段，需新增聚合 query |
| 交易成功率 | `trade_win_rate` | 所选区间内 `实现盈亏 > 0` 的卖出笔数 / 区间内卖出笔数；卖出行 realized PnL 按 `exit_trade_seq` 汇总 closed rows | 已有 realized PnL，需按卖出 trade row 聚合 |
| 盈利股票数 | `winning_security_count` | 先按 `security_code` 聚合 `sum(realized_pnl)`，再统计 `> 0` 的证券数 | 已有字段，需新增聚合 query |
| 亏损股票数 | `losing_security_count` | 先按 `security_code` 聚合 `sum(realized_pnl)`，再统计 `< 0` 的证券数 | 已有字段，需新增聚合 query |
| 持股天数 | `holding_days` | `countIf(live_nav_daily.position_count > 0)`，同一 attempt 和区间 | 已有字段，需新增聚合 query |

这里的“交易成功率”已经明确不是订单成交率，而是对账单所选日期范围内的卖出交易胜率。分母是区间内 `live_trade.side = sell` 的成交笔数，分子是这些卖出行中实现盈亏为正的笔数；同一卖出 trade 如果关闭多个 FIFO lot，需要先按 `live_closed_trade.exit_trade_seq = live_trade.trade_seq` 汇总 realized PnL，再判断该卖出笔是否成功。

这里的“盈利/亏损股票数”已确认只统计区间内已经卖出、已经形成实现盈亏的股票。统计时先按 `security_code` 汇总区间内 closed rows 的 `realized_pnl`，再判断股票级别盈亏；同一股票区间内多次卖出只计入一个股票级别结果。未卖出的当前持仓仍属于浮动盈亏，继续留在虚拟资金账户和持仓明细，不混入 realized trade win-rate。

这里的“持股天数”已确认不是 closed trade 的平均持仓周期，而是所选区间内账户有仓位的天数。第一版按 live facts 的交易日粒度计算：区间内 `live_nav_daily.position_count > 0` 的交易日数。

### 操作记录

操作记录建议只展示实际成交的买入和卖出，不展示 hold 行。字段映射如下：

| UI 字段 | 建议 API 字段 | 数据来源 | 支撑状态 |
|---|---|---|---|
| 调仓日 | `trade_date` | `live_trade.trade_date` | 已有 |
| 买入/卖出 | `side` | `live_trade.side` | 已有 |
| 个股 | `security_code`, `security_name` | `live_trade.security_code` + `mart_stock_basic_snapshot` display map | code 已有，name 需沿用现有 display map |
| 价格 | `execution_price` | `live_trade.execution_price` | 已有 |
| 数量 | `quantity`, `lot_count`, `lot_size` | `live_trade.quantity`；`lot_count = quantity / 100` | 已有；lot size 来自 execution config |
| 金额 | `gross_amount` | `live_trade.gross_amount` | 已有 |
| 费用 | `total_fee`, `commission`, `stamp_duty`, `transfer_fee` | `live_trade.*fee` | 已有 |
| 持仓余额 | `position_balance_quantity` | 由同一证券所有历史 trade 按 `trade_date, trade_seq` 滚动累计买入减卖出 | 需新增 query 派生，不应前端计算 |
| 实现盈亏 | `realized_pnl` | 对卖出行按 `live_closed_trade.exit_trade_seq = live_trade.trade_seq` 聚合 `sum(realized_pnl)`；买入行为 `NULL` | 已有 closed ledger，需新增 join/聚合 |

持仓余额不直接存储在 `live_trade`，但可以由当前 attempt 内的交易流水确定。该派生必须在后端完成，因为它需要读取区间开始前的历史交易才能得到区间首行之后的余额。只拉取当前页 trade rows 在前端累计会在分页、筛选或区间截断时算错。

实现盈亏也不能从当前 sell trade 的卖出金额直接推测。正确来源是 worker 生成的 FIFO `live_closed_trade`，同一个 sell trade 可能关闭多个买入 lot；operation row 应按 `exit_trade_seq` 汇总这些 closed rows。

## 建议 API

建议新增一个组合对账单 read model：

```http
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/statement?period=month&limit=100&offset=0
```

返回结构建议：

```json
{
  "source": "live_daily_run",
  "strategy_portfolio_id": "01KW...",
  "strategy_portfolio_daily_run_id": "01KW...",
  "result_attempt_id": "01KW...",
  "period": {
    "key": "month",
    "label": "本月",
    "start_date": "2026-06-01",
    "end_date": "2026-06-27",
    "latest_live_trade_date": "2026-06-27"
  },
  "summary": {
    "average_position_pct": 0.7421,
    "traded_security_count": 12,
    "trade_count": 18,
    "trade_win_rate": 0.5833,
    "winning_security_count": 7,
    "losing_security_count": 4,
    "holding_days": 18
  },
  "operations": {
    "items": [
      {
        "portfolio_trade_id": "01KW...",
        "trade_seq": 12,
        "trade_date": "2026-06-27",
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

也可以拆成两个接口：

- `GET /rearview/strategy-portfolios/{id}/statement-summary?period=...`
- `GET /rearview/strategy-portfolios/{id}/statement-operations?period=...`

但第一版页面通常会同时展示两部分，一个接口能减少前端状态组合和 loading/error 分叉。实现阶段如果 operation payload 大，再拆分。

`pending_first_run` 语义应与现有 live endpoints 一致：返回 `409 portfolio_pending_first_run`，前端展示空态，不从 source backtest 拼对账单。

## 查询方案草案

### Summary 聚合

伪 SQL：

```sql
SELECT
  (
    SELECT avg(gross_exposure)
    FROM fleur_portfolio.live_nav_daily
    WHERE strategy_portfolio_daily_run_id = {run_id}
      AND result_attempt_id = {attempt}
      AND trade_date >= {start_date}
      AND trade_date <= {end_date}
  ) AS average_position_pct,
  (
    SELECT countDistinct(security_code)
    FROM fleur_portfolio.live_trade
    WHERE strategy_portfolio_daily_run_id = {run_id}
      AND result_attempt_id = {attempt}
      AND trade_date >= {start_date}
      AND trade_date <= {end_date}
  ) AS traded_security_count,
  (
    SELECT count()
    FROM fleur_portfolio.live_trade
    WHERE strategy_portfolio_daily_run_id = {run_id}
      AND result_attempt_id = {attempt}
      AND trade_date >= {start_date}
      AND trade_date <= {end_date}
  ) AS trade_count,
  (
    SELECT if(count() = 0, null, countIf(coalesce(realized_pnl, 0) > 0) / count())
    FROM (
      SELECT
        trade.trade_seq,
        trade.security_code,
        sum(closed.realized_pnl) AS realized_pnl
      FROM fleur_portfolio.live_trade AS trade
      LEFT JOIN fleur_portfolio.live_closed_trade AS closed
        ON closed.strategy_portfolio_daily_run_id = trade.strategy_portfolio_daily_run_id
       AND closed.result_attempt_id = trade.result_attempt_id
       AND closed.exit_trade_seq = trade.trade_seq
       AND closed.security_code = trade.security_code
      WHERE trade.strategy_portfolio_daily_run_id = {run_id}
        AND trade.result_attempt_id = {attempt}
        AND lower(trade.side) = 'sell'
        AND trade.trade_date >= {start_date}
        AND trade.trade_date <= {end_date}
      GROUP BY trade.trade_seq, trade.security_code
    )
  ) AS trade_win_rate,
  (
    SELECT countIf(realized_pnl_by_security > 0)
    FROM (
      SELECT security_code, sum(realized_pnl) AS realized_pnl_by_security
      FROM fleur_portfolio.live_closed_trade
      WHERE strategy_portfolio_daily_run_id = {run_id}
        AND result_attempt_id = {attempt}
        AND exit_date >= {start_date}
        AND exit_date <= {end_date}
      GROUP BY security_code
    )
  ) AS winning_security_count,
  (
    SELECT countIf(realized_pnl_by_security < 0)
    FROM (
      SELECT security_code, sum(realized_pnl) AS realized_pnl_by_security
      FROM fleur_portfolio.live_closed_trade
      WHERE strategy_portfolio_daily_run_id = {run_id}
        AND result_attempt_id = {attempt}
        AND exit_date >= {start_date}
        AND exit_date <= {end_date}
      GROUP BY security_code
    )
  ) AS losing_security_count,
  (
    SELECT countIf(position_count > 0)
    FROM fleur_portfolio.live_nav_daily
    WHERE strategy_portfolio_daily_run_id = {run_id}
      AND result_attempt_id = {attempt}
      AND trade_date >= {start_date}
      AND trade_date <= {end_date}
  ) AS holding_days
```

实现时可按 ClickHouse 语法拆成多个稳定查询或复用子查询，重点是所有查询都必须绑定同一个 `strategy_portfolio_daily_run_id`、`result_attempt_id` 和 resolved period。

### Operation rows

伪 SQL：

```sql
WITH trade_with_balance AS (
  SELECT
    portfolio_trade_id,
    trade_seq,
    trade_date,
    security_code,
    side,
    quantity,
    execution_price,
    gross_amount,
    commission,
    stamp_duty,
    transfer_fee,
    total_fee,
    reason,
    sum(if(lower(side) = 'buy', quantity, -quantity))
      OVER (
        PARTITION BY security_code
        ORDER BY trade_date, trade_seq
        ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
      ) AS position_balance_quantity
  FROM fleur_portfolio.live_trade
  WHERE strategy_portfolio_daily_run_id = {run_id}
    AND result_attempt_id = {attempt}
),
realized_by_exit_trade AS (
  SELECT
    exit_trade_seq,
    security_code,
    sum(realized_pnl) AS realized_pnl
  FROM fleur_portfolio.live_closed_trade
  WHERE strategy_portfolio_daily_run_id = {run_id}
    AND result_attempt_id = {attempt}
  GROUP BY exit_trade_seq, security_code
)
SELECT
  trade_with_balance.*,
  realized_by_exit_trade.realized_pnl
FROM trade_with_balance
LEFT JOIN realized_by_exit_trade
  ON realized_by_exit_trade.exit_trade_seq = trade_with_balance.trade_seq
 AND realized_by_exit_trade.security_code = trade_with_balance.security_code
WHERE trade_date >= {start_date}
  AND trade_date <= {end_date}
ORDER BY trade_date DESC, trade_seq DESC
LIMIT {limit_plus_one} OFFSET {offset}
```

注意这里的 `trade_with_balance` 必须先覆盖当前 attempt 全量历史，再在外层按 period 过滤，否则区间第一条记录的持仓余额会丢失区间之前的买入数量。

## 数据支撑结论

### 可以直接支撑

1. 平均仓位：`live_nav_daily.gross_exposure`。
2. 交易股票数、交易笔数：`live_trade`。
3. 价格、数量、金额、费用：`live_trade`。
4. A 股一手 100 股：execution config 已有 `lot_size = 100` 和 `min_trade_lots = 1`，模拟器也按 lot size 下单。
5. 实现盈亏：`live_closed_trade.realized_pnl`，按 FIFO lot 由 worker 计算。
6. 已卖出股票的盈利/亏损统计：`live_closed_trade`。
7. 区间内有仓位的天数：`live_nav_daily.position_count`。
7. 证券名称显示：可沿用现有 `security_display_map()`，从 `mart_stock_basic_snapshot` 补齐。

### 需要新增后端 read model

1. 区间解析：后端根据 latest live nav date、`live_start_date` 和 period key 统一解析。
2. 账户盈亏面板聚合：对 `live_nav_daily`、`live_trade` 和 `live_closed_trade` 做同 attempt 区间聚合。
3. 操作记录：从 `live_trade` 读取成交流水，并派生成交后持仓余额。
4. 卖出行实现盈亏：按 `exit_trade_seq` 连接 `live_closed_trade`。
5. Strategy portfolio statement HTTP API：当前只有内部 query method，没有面向 `/strategy-portfolios/{id}` 的 trades/closed-trades/statement route。

### 暂不需要新增

1. 不需要新增 raw source、dbt staging、dbt mart 或数据契约。
2. 不需要新增 ClickHouse 事实表；第一版可以读时聚合。
3. 不需要改 worker 的净值、费用或 FIFO 计算口径。
4. 不需要前端计算持仓余额、实现盈亏或交易统计。

如果对账单打开频率高、历史变长后查询变慢，再评估增加 materialized summary，例如 `live_statement_period_metric` 或按日滚动的 `live_trade_statement_day`。第一版应先复用权威事实，避免提前固化派生表。

## 生产化验收样例

对账单验收建议复用 `docs/plans/archive/0051-racingline-strategy-backtest-step5-implementation-plan.md` 中“前端验收样例：低位反转回测”的规则，因为它覆盖多 mart 过滤、字段比较、字段乘常数、分段打分、TopN、仓位约束、费率和风险退出配置，比较接近真实生产组合。对账单场景需要把 Step 5 的默认“一年期回测”调整为“2025 年首个交易日建仓，并持续每日清算到当前可用数据日”，从而得到超过一年长度的 live 成交、持仓、净值和 closed trade 事实。

### 验收规则沿用范围

Step 1 继续使用低位反转 AND 过滤条件：

```text
kdj_j_value < 13
pct_amplitude < 4
pct_change > -2
pct_change < 2
volume < prev_volume * 0.8
price_ema2_10 > price_avg_ma_14_28_57_114
close_down_streak_days < 4
close_price_forward_adj > price_avg_ma_3_6_12_24
price_ma_60 > price_ma_114
price_ma_114 > price_ma_250
```

Step 2 继续使用 conditional points，并保持 KDJ 两段互斥：

```text
kdj_j_value < -15                                      +25
-15 <= kdj_j_value < -10                               +15
volume < volume_ma_5 * 0.6                             +20
price_ma_20 < close_price_forward_adj < price_ma_60    +15
n_structure_20_second_low_ratio > 1                    +15
close_price_forward_adj < boll_lower_20_2              +15
rsi_6 < 25                                             +5
```

Step 4 继续使用默认执行配置：

| 配置 | 验收值 |
|---|---|
| 初始资金 | `1_000_000` |
| 买入 TopN | `5` |
| 最大持仓数 | `5` |
| 单票仓位上限 | `10%` |
| 佣金、印花税、过户费、滑点 | UI 默认模板值 |
| 风控条件 | 全部启用，参数使用 UI 默认值和 validate API 返回的 canonical config |

### 对账单验收调整

原 Step 5 验收是进入 Step 5 后用默认 `近一年 + 000300.SH` 创建回测 run，并验证 rerun 和结果展示。对账单验收应改为：

1. 使用同一套 Step 1、Step 2 和 Step 4 配置，创建一个可发布为 strategy portfolio 的 source backtest。
2. 让发布预检解析出 `source_signal_date = 2024-12-31`、`planned_live_start_date = 2025-01-02`。当前发布逻辑把 source backtest 的 `end_date` 作为 `initial_signal_date`，并从交易日历解析下一交易日为 `live_start_date`；因此首选方案是重跑一次截至 2024 年最后一个交易日的 source backtest，获取 `2024-12-31` 的买入信号。如果补充需求中的“2014 年”不是笔误，则对应的是 2015 年首个交易日建仓，不能满足本文“2025 年首个交易日建仓”的验收目标。
3. 发布 portfolio 后，从 `2025-01-02` 开始按交易日执行 strategy portfolio daily run，一直执行到当前各依赖数据共同可用的最新交易日。当前本地数据盘点中，`mart_trade_calendar` 的 2025 首个交易日是 `2025-01-02`，2025 年共有 243 个交易日；行情、动量、趋势和 `000300.SH` benchmark 在本轮盘点时共同覆盖到 `2026-06-26`，这能产生超过一年长度的 live 事实。
4. 使用最新成功的 daily run 作为对账单读取源。worker 当前对每个 daily run 采用从 `run_start_date` 到 `trade_date` 的全窗口重算，并在写入前按 `live_start_date` 归一化输出；因此最新成功 attempt 已经包含从 `2025-01-02` 到最终清算日的完整 live 对账历史。
5. 在策略详情页打开对账单，依次切换 `本月`、`近三月`、`近半年`、`今年`、`全部`，确认 summary 和 operation rows 都绑定同一个最新 `strategy_portfolio_daily_run_id` 与 `result_attempt_id`。

### 期望验收证据

验收报告应记录以下事实和截图：

1. `strategy_portfolio.initial_signal_date = 2024-12-31`，`strategy_portfolio.live_start_date = 2025-01-02`。
2. 最新成功 `strategy_portfolio_daily_run.trade_date` 晚于 `2026-01-02`，推荐使用当前共同可用最新交易日，例如本轮盘点中的 `2026-06-26`。
3. `fleur_portfolio.live_nav_daily` 在最新 attempt 下覆盖 `2025-01-02` 到最终清算日，并且行数超过一年交易日长度。
4. `fleur_portfolio.live_trade` 有真实 buy/sell rows，`quantity` 均为 100 的整数倍。
5. `fleur_portfolio.live_closed_trade` 有非空 `realized_pnl`，卖出操作行按 `exit_trade_seq` 汇总后能展示实现盈亏。
6. 对账单 `全部` 区间包含超过一年长度的 operation rows；`本月`、`近三月`、`近半年`、`今年` 的 `start_date/end_date` 由后端返回。
7. operation row 的 `position_balance_quantity` 包含区间开始前历史交易，不因分页或 period 截断而重置。
8. 截图证据链在 0051 的 8 张 Step 1 到 Step 5 截图基础上追加：
   - `statement-01-summary-periods.png`：账户盈亏面板和区间切换。
   - `statement-02-operations-all.png`：全部区间操作记录，覆盖买入、卖出、费用、持仓余额和实现盈亏。
   - `statement-03-operations-period.png`：任一非全部区间，证明余额和实现盈亏没有由前端局部累计。

## 每日清算缺口与改造范围

现有 strategy portfolio daily run 能支撑“最新 attempt 包含完整 live 历史”的对账单读取模型，但要稳定产生 2025 首个交易日建仓、超过一年期的验收数据，还需要补齐以下能力。

### 已有能力

1. Rearview 已有 `POST /rearview/strategy-portfolios/daily-runs`，请求体只有 `trade_date` 和可选 `client_request_id`，每次为一个交易日创建 daily run。
2. `create_strategy_portfolio_daily_runs_for_trade_date()` 会读取 active portfolios，按 `portfolio.live_start_date <= trade_date` 筛选，并用唯一约束跳过已存在的 `(strategy_portfolio_id, trade_date)`。
3. daily run 的 `run_start_date` 当前写入 `portfolio.initial_signal_date`，不是 `live_start_date`。这使 worker 可以看到建仓日前一交易日的 seed signal，并在 `live_start_date` 执行 T+1 买入。
4. worker 在 daily run 中会从 `run.run_start_date` 到 `run.trade_date` 重新编译信号、加载价格、模拟组合、计算绩效和 closed trades，再用 `normalize_live_output()` 裁剪到 `portfolio.live_start_date` 并重设 live nav 基准。
5. Dagster 已有 `strategy_portfolio__daily_run_job` 和 `portfolio__daily_run_schedule`，每天 20:00 触发 `strategy_portfolio_daily_runs` asset。
6. 当前 Dagster asset 只调用 Rearview 创建 daily runs，并把 `created_run_count`、`skipped_run_count` 和 `daily_run_ids` 写入 materialization metadata；worker 完成状态仍在 Rearview/PostgreSQL control plane 中异步推进。

### 实现缺口

1. `POST /rearview/strategy-portfolios/daily-runs` 只能处理单个 `trade_date`，没有 `start_date/end_date` range API。要补 2025-01-02 到 2026-06-26 的验收数据，当前只能通过大量单日请求或 Dagster backfill 绕行。
2. Dagster `STRATEGY_PORTFOLIO_DAILY_PARTITIONS` 当前从 `2026-06-24` 开始，无法自然选择 2025 年分区来生成长周期验收 daily runs。
3. daily run 创建接口本身没有查询交易日历；只要请求日期满足 `live_start_date <= trade_date` 就会建 run。worker 后续会查询 `run_start_date` 到 `trade_date` 的交易日，并要求至少两个交易日。调度层需要避免非交易日和数据未齐日期进入队列。
4. 当前没有“最新可清算交易日”解析器。对账单验收不应使用浏览器日期或系统日期，而应取交易日历、行情、规则依赖 mart、benchmark 和无风险利率等依赖数据的共同可用上限。
5. 发布路径当前通过 source backtest 的 `end_date` 推导 `initial_signal_date`。验收应优先重跑截至 2024 年最后一个交易日的 source backtest 来获得 `2024-12-31` 买入信号。如果现有 backtest options 不能生成 `end_date = 2024-12-31` 的 source run，就需要受控测试种子或测试专用后门；不能手工改生产记录字段来伪造日期。
6. worker 对每个 daily run 都做全窗口重算。长周期 backfill 在功能验收上可接受，但生产回补需要控制并发、队列压力、ClickHouse 查询成本和失败重试观测。
7. 对账单 read model 需要明确读取“最新成功 daily run 的当前 attempt”，而不是跨多个 daily run 拼接历史。现有 `resolve_strategy_portfolio_result()` 已经返回 latest daily run 和 `current_live_result_attempt_id`，可以复用。
8. Dagster 清算作业当前的成功语义不等于清算完成。`strategy_portfolio_daily_runs` asset 在 Rearview 返回 `202 Accepted` 后即 materialize；如果 worker 后续失败、NATS/outbox 未消费、ClickHouse 写入失败或 `strategy_portfolio_daily_run.status` 停留在 queued/running，Dagster 仍可能显示本次 asset 成功。该行为会让本验收用例出现调度成功但对账单无数据的假阳性。
9. 当前 Rearview HTTP route 没有暴露按 `strategy_portfolio_daily_run_id` 查询 daily run 状态的接口；Dagster 若要等待终态，需要新增状态查询 API，或增加一个受控的 scheduler 侧 Postgres 查询资源。

### 改造范围

建议把每日清算改造拆成五块：

1. Rearview range daily-run API：新增 `POST /rearview/strategy-portfolios/daily-runs/range`，接收 `start_date`、`end_date`、可选 `client_request_id`，只按交易日历中的交易日建 run，返回 created/skipped/errors 汇总，并设置最大区间保护。
2. Dagster backfill 能力：要么把 partition 起点调整到 `2025-01-02`，要么新增 range backfill asset/config 来调用 range API。生产 schedule 继续跑自然日触发，但 asset 内应解析最近可清算交易日并跳过非交易日。
3. 数据可用性解析：后端或调度层增加 settlement target resolver，取 `mart_trade_calendar`、`mart_stock_quotes_daily`、策略规则 required marts、`mart_benchmark_returns_daily` 和 performance 所需无风险利率的共同最新交易日。
4. 受控验收种子：提供一种可审计的方式创建 `initial_signal_date = 2024-12-31`、`live_start_date = 2025-01-02` 的 portfolio。优先走真实 source backtest 和 publish preview，即重跑截至 2024 年最后一个交易日的回测并发布；如果数据窗口限制导致 source backtest 无法落在 2024-12-31，则使用明确标记为 dev/test 的 fixture 命令，并在验收报告记录 fixture 输入和生成的 portfolio id。
5. Dagster 清算完成校验：改造调度链路，使 Dagster 能确认 daily run 最终 `succeeded` 且 ClickHouse live facts 已写入。可以在同一个 asset 内轮询 Rearview 状态，也可以拆出下游 `strategy_portfolio_daily_run_results` 观测/校验 asset；无论采用哪种实现，验收不能只以“创建 daily run 成功”作为清算成功。

### Dagster 清算作业适配要求

2025 首个交易日建仓的对账单验收依赖 Dagster 能稳定执行长周期清算。这里的“清算作业成功”应定义为：目标交易日的 active portfolio daily run 已创建、worker 已消费并写入结果、PostgreSQL 状态已 finalized、ClickHouse live facts 可查询。仅有 Dagster materialization 成功或 Rearview 返回 `202 Accepted` 不足以证明对账单数据可用。

建议把 Dagster 改造拆成以下能力：

1. 交易日范围展开：清算作业输入可以是单个 `trade_date`，也可以是 `start_date/end_date`。range 模式必须从 Rearview 或 ClickHouse 交易日历解析真实交易日列表，不能按自然日循环。
2. 数据可用性门禁：作业开始前先解析 settlement target date。目标日期必须不晚于交易日历、行情、策略 required marts、benchmark returns 和 performance 所需无风险利率的共同可用上限；超出上限时应 skip 或 fail with reason，而不是创建必然失败的 daily run。
3. 幂等创建：对同一 portfolio 和 trade date 重跑时，应复用 Rearview 唯一约束，把已存在 run 计入 skipped，并在 Dagster metadata 中保留 created/skipped 的明细。
4. 结果完成等待：Dagster 需要读取被创建或命中的 daily run ids，并等待它们进入终态。成功终态要求 `strategy_portfolio_daily_run.status = succeeded`、`current_result_attempt_id` 非空、`strategy_portfolio.latest_daily_run_id` 指向本次或更晚成功 run。
5. ClickHouse 写入核验：对最新成功 attempt 至少检查 `live_nav_daily` 有 rows，且 `max(trade_date)` 等于本次清算 trade date；对账单验收场景还应检查 `live_trade`、`live_closed_trade` 是否可查询，并把 row count 写入 Dagster metadata。
6. 失败可观测：如果任一 daily run 进入 failed 状态，Dagster 作业应失败并输出 `error_type`、`error_message`、`data_coverage_summary`、`signal_summary` 和对应 `strategy_portfolio_daily_run_id`。如果超时仍处于 queued/running，也应失败或标记为明确的 timeout，而不是静默成功。
7. 长周期回补节流：从 `2025-01-02` 回补到 `2026-06-26` 可能创建数百个交易日 run。Dagster 应支持 chunk size、最大并发、轮询间隔和超时配置，避免一次 materialization 压垮 Rearview worker、NATS/outbox 或 ClickHouse。
8. worker 前置检查：验收运行前必须确认 rearview server、NATS/outbox publisher 和 `rearview-portfolio-worker` 正在运行。若 worker 未运行，Dagster 应能通过状态等待超时暴露问题。
9. 生产 schedule 语义：每天 20:00 的 schedule 仍可保留，但它应该以“最近可清算交易日”为目标，而不是简单使用当天自然日 partition key。非交易日或依赖数据未齐时，作业应产生可解释 skip metadata。

Dagster metadata 建议至少包含：

| 字段 | 说明 |
|---|---|
| `requested_start_date` / `requested_end_date` | 用户或 partition 请求的清算范围 |
| `resolved_trade_dates` | 实际执行的交易日列表或数量 |
| `settlement_target_date` | 依赖数据共同可用上限 |
| `created_run_count` / `skipped_run_count` | Rearview 创建和跳过数量 |
| `daily_run_ids` | 本次创建或命中的 daily run ids |
| `succeeded_run_count` / `failed_run_count` / `timeout_run_count` | worker 终态统计 |
| `latest_result_attempt_id` | 最新成功 attempt |
| `nav_row_count` / `trade_row_count` / `closed_trade_row_count` | ClickHouse 写入核验结果 |

本验收用例的完成标准应包括 Dagster 证据：`strategy_portfolio__daily_run_job` 或其 range/backfill 变体成功完成，metadata 显示目标日期范围内 daily runs 已进入 `succeeded`，且最新 attempt 的 ClickHouse facts 覆盖 `2025-01-02` 到最终清算日。

### 改造后的验收 SQL 方向

验收报告至少应记录这些查询结果，具体 SQL 可按实现时的表字段和 query helper 调整：

```sql
select
  strategy_portfolio_id,
  initial_signal_date,
  live_start_date,
  latest_daily_run_id,
  current_live_result_attempt_id
from strategy_portfolio
where strategy_portfolio_id = {portfolio_id};
```

```sql
select
  min(trade_date) as first_live_date,
  max(trade_date) as latest_live_date,
  count() as nav_rows
from fleur_portfolio.live_nav_daily
where strategy_portfolio_daily_run_id = {latest_daily_run_id}
  and result_attempt_id = {current_live_result_attempt_id};
```

```sql
select
  count() as trade_count,
  countIf(lower(side) = 'buy') as buy_count,
  countIf(lower(side) = 'sell') as sell_count,
  countIf(modulo(quantity, 100) != 0) as non_lot_trade_count
from fleur_portfolio.live_trade
where strategy_portfolio_daily_run_id = {latest_daily_run_id}
  and result_attempt_id = {current_live_result_attempt_id};
```

```sql
select
  count() as closed_trade_count,
  countIf(realized_pnl > 0) as winning_closed_lots,
  countIf(realized_pnl < 0) as losing_closed_lots,
  min(exit_date) as first_exit_date,
  max(exit_date) as latest_exit_date
from fleur_portfolio.live_closed_trade
where strategy_portfolio_daily_run_id = {latest_daily_run_id}
  and result_attempt_id = {current_live_result_attempt_id};
```

## 已确认口径

1. “盈利股票数/亏损股票数”只统计已卖出的实现盈亏，不包含未卖出持仓的浮动盈亏。
2. “持股天数”展示区间内有仓位的天数，第一版按 `live_nav_daily.position_count > 0` 的交易日数计算。
3. 第一版前端不实现操作记录按证券过滤。
4. 2025 首个交易日建仓验收优先重跑截至 2024 年最后一个交易日的 source backtest，获取 `2024-12-31` 买入信号后再发布 portfolio。

## 验收建议

实施阶段至少需要覆盖：

1. `pending_first_run` 返回空态，不回退 source backtest。
2. 每个 period 的 `start_date/end_date` 由后端返回，前端只展示和传 key。
3. 区间无卖出 trade 时，`trade_win_rate` 为 `NULL`；区间无持仓交易日时，`holding_days` 为 `0`。
4. operation row 的 `position_balance_quantity` 包含区间开始前历史交易。
5. 同一 sell trade 拆多个 closed lots 时，operation row 的 `realized_pnl` 等于这些 closed rows 的合计。
6. `trade_win_rate` 的分母是区间内卖出 trade row 数，不是 closed lot 数；同一 sell trade 拆多个 closed lots 时只能算一笔卖出。
7. `holding_days` 的口径是区间内 `position_count > 0` 的交易日数，不是 closed trade 平均持仓周期。
8. operation row 的 `quantity` 始终为 100 的整数倍；`lot_count = quantity / 100`。
9. summary 和 operations 都绑定同一个 `strategy_portfolio_daily_run_id` 与 `result_attempt_id`。
10. 使用低位反转样例创建 `2025-01-02` 建仓 portfolio，并清算到晚于 `2026-01-02` 的交易日。
11. 对账单 `全部` 区间的 nav、trade、closed trade 均来自最新成功 live attempt，不跨 attempt 拼接。
12. range daily-run 创建对同一区间重复执行时应幂等，已存在 run 计入 skipped，不重复写 outbox。
13. Dagster 清算作业成功必须代表 worker 已完成并写入 ClickHouse，不能只代表 daily run 创建请求成功。
14. 验收报告必须包含 Dagster materialization metadata，至少覆盖 resolved trade dates、daily run ids、succeeded/failed/timeout 数量和最新 attempt 的 nav/trade/closed trade row count。
