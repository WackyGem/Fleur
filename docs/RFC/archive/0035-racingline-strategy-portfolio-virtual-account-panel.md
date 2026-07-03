# RFC 0035: Racingline 组合详情页虚拟资金账户

状态：Implemented（2026-06-28）
领域：racingline
关联系统：racingline, rearview, data-platform
代码根：
- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/elt/`
架构事实：
- docs/architecture/racingline.md
- docs/architecture/rearview.md
关联文档：
- docs/RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md
- docs/RFC/archive/0034-racingline-step5-portfolio-publish-dialog-tabs.md
- docs/plans/archive/0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md

## 摘要

Racingline 组合详情页 `/dashboard/strategies/:portfolioId` 当前已经展示组合概要、净值走势和持仓记录，但缺少一个面向用户的虚拟资金账户视图。用户能看到净值曲线，却不能直接判断当前虚拟账户里还有多少现金、持股市值是多少、当前持仓浮盈浮亏是多少，以及今天账户层面赚亏了多少。

本 RFC 建议在组合详情页的「净值走势」和「持仓记录」之间新增一个标题为「虚拟资金账户」的区块，展示以下六个字段：

1. 账户资产：可用金额加持仓证券市值后的总资产合计。
2. 持股市值。
3. 可用金额。
4. 持仓盈亏：当前持仓的未实现盈亏金额。
5. 当日盈亏：账户资产相对上一交易日账户资产的变化金额。
6. 当日盈亏比：账户层面的当日收益率。

本文只讨论数据、后端和前端页面闭环，不直接实施代码。

## 当前事实

### 页面现状

组合详情页位于 `app/racingline/src/routes/strategy-detail-page.tsx`。当前页面顺序为：

1. 组合标题和 live 状态摘要。
2. 组合业绩指标。
3. 「净值走势」图表。
4. 「持仓记录」表格。

「净值走势」区块使用 `useStrategyPortfolioNavQuery()` 读取 `/rearview/strategy-portfolios/{strategy_portfolio_id}/nav`，并把返回的 `points` 转成策略净值和基准净值曲线。

「持仓记录」区块使用 `useStrategyPortfolioRebalanceRecordsQuery()`，展示调入、持有和调出的证券明细。页面也会调用 `useStrategyPortfolioPositionsQuery()` 获取 live 持仓数量，但没有把持仓账户金额汇总展示出来。

### 现有数据资源

ClickHouse `fleur_portfolio.live_nav_daily` 已由 Rust worker 写入以下可复用字段：

| 字段 | 可支持的 UI 字段 |
|---|---|
| `cash_balance` | 可用金额 |
| `position_market_value` | 持股市值 |
| `total_equity` | 账户资产 |
| `daily_return` | 当日盈亏比 |
| `trade_date` | 账户快照日期 |
| `position_count` | 当前持仓数量，可作为次级信息 |

ClickHouse `fleur_portfolio.live_position_day` 已由 Rust worker 写入以下可复用字段：

| 字段 | 可支持的 UI 字段 |
|---|---|
| `market_value` | 单只证券市值；可用于核对持股市值 |
| `unrealized_pnl` | 单只证券当前持仓未实现盈亏 |
| `unrealized_return` | 单只证券当前持仓收益率；本 RFC 第一版不展示 |
| `trade_date` | 持仓快照日期 |

Rearview 当前 `query_strategy_portfolio_live_nav()` 已能从 `live_nav_daily` 查询 `cash_balance`、`position_market_value`、`total_equity` 和 `daily_return`。但是现有 strategy portfolio nav API 返回类型复用 `StrategyBacktestNavPoint`，只暴露 `trade_date`、`strategy_nav`、`benchmark_nav` 和 `excess_return`，没有把账户金额字段返回给前端。

Rearview 当前 `query_strategy_portfolio_live_positions()` 已能从 `live_position_day` 查询 `market_value` 和 `unrealized_pnl`。但是现有 positions API 是明细列表，不是面向账户卡片的聚合 read model。

### 数据缺口

| UI 字段 | 现有事实 | 缺口 |
|---|---|---|
| 账户资产 | `live_nav_daily.total_equity` | API 未暴露到组合详情页账户 read model |
| 持股市值 | `live_nav_daily.position_market_value` | API 未暴露到组合详情页账户 read model |
| 可用金额 | `live_nav_daily.cash_balance` | API 未暴露到组合详情页账户 read model |
| 持仓盈亏 | `sum(live_position_day.unrealized_pnl)` | 需要按同一 `trade_date` 和 `result_attempt_id` 聚合；不应由前端分页求和 |
| 当日盈亏 | 当前行 `total_equity` 减上一交易日 `total_equity` | 现有表没有直接字段；后端应从最新 nav 行和前一 nav 行派生 |
| 当日盈亏比 | `live_nav_daily.daily_return` | API 未暴露到组合详情页账户 read model；首个 live 日期可能为 NULL |

## 目标

1. 在组合详情页「净值走势」和「持仓记录」之间新增「虚拟资金账户」区块。
2. 使用 Rearview API 返回的单一 read model 展示账户资产、持股市值、可用金额、持仓盈亏、当日盈亏和当日盈亏比。
3. 所有账户金额和收益率由后端从 `fleur_portfolio.live_*` 事实读取或派生，前端只负责展示和格式化。
4. read model 的日期、attempt 和 latest daily run 必须一致，避免净值来自一个日期、持仓盈亏来自另一个日期。
5. `pending_first_run` 组合不展示伪账户数据，显示明确空态。

## 非目标

1. 不接入真实券商资金账户、实盘持仓或交易回报。
2. 不在浏览器中重算现金、持仓市值、NAV、当日盈亏或持仓盈亏。
3. 不新增 ClickHouse 表；第一版应复用 `live_nav_daily` 和 `live_position_day`。
4. 不改变 worker 的权威净值递推公式。
5. 不在本 RFC 中设计完整账户流水、订单冻结金额、可取金额、保证金或融资融券口径。
6. 不把 source backtest 的账户状态混入正式组合 live 账户展示。

## 口径定义

### 快照日期

`account_date` 取当前 strategy portfolio 最新可见 live result attempt 的最新 `live_nav_daily.trade_date`。该日期应与持仓盈亏聚合使用的 `live_position_day.trade_date` 一致。

如果组合为 `pending_first_run`，或者还没有成功 live daily run，则账户 read model 返回 `409 portfolio_pending_first_run` 或空状态，不返回零值账户。

### 字段口径

| API 字段 | UI 文案 | 口径 |
|---|---|---|
| `total_equity` | 账户资产 | 最新 nav 行的 `total_equity`。等价于可用金额加持股市值；以后端事实为准，不在前端相加覆盖。 |
| `position_market_value` | 持股市值 | 最新 nav 行的 `position_market_value`。 |
| `cash_balance` | 可用金额 | 最新 nav 行的 `cash_balance`。第一版不区分冻结资金和可取资金。 |
| `holding_unrealized_pnl` | 持仓盈亏 | 同一日期所有当前持仓 `unrealized_pnl` 求和。没有持仓时为 `0`。 |
| `daily_pnl` | 当日盈亏 | 最新 nav 行 `total_equity - previous_total_equity`。如果没有上一 nav 行，则为 `NULL`。 |
| `daily_return` | 当日盈亏比 | 最新 nav 行 `daily_return`。如果没有上一 nav 行，则为 `NULL`。 |
| `position_count` | 持仓数量 | 最新 nav 行 `position_count`，可用于次级文案或调试，不作为六个主指标之一。 |

`daily_pnl` 不应使用 `total_equity * daily_return` 反推，因为 `daily_return` 的分母是上一交易日账户资产。正确派生是：

```text
daily_pnl = latest.total_equity - previous.total_equity
daily_return = latest.daily_return
```

如果只拿到 `daily_return` 而缺少上一 nav 行，应返回 `daily_pnl = NULL`，不要在前端猜测。

## 后端设计

### 新增 API

建议新增：

```http
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/virtual-account
```

返回示例：

```json
{
  "source": "live_daily_run",
  "strategy_portfolio_id": "01KW3K9453WXGX3B1KJFZYQRVC",
  "strategy_portfolio_daily_run_id": "01KW...",
  "result_attempt_id": "01KW...",
  "account_date": "2026-06-27",
  "currency": "CNY",
  "total_equity": 1012345.67,
  "position_market_value": 812345.67,
  "cash_balance": 200000.0,
  "holding_unrealized_pnl": 12345.67,
  "daily_pnl": -2345.67,
  "daily_return": -0.0023,
  "position_count": 5
}
```

`pending_first_run` 行为与现有 live detail endpoints 保持一致：返回 `409 portfolio_pending_first_run`，前端映射为空态。该接口不回退 source backtest，也不从 publish preview 生成模拟账户。

### 查询策略

Rearview 应复用现有 `resolve_strategy_portfolio_result()`，先得到：

1. `strategy_portfolio_id`
2. `strategy_portfolio_daily_run_id`
3. `result_attempt_id`
4. `source = live_daily_run`

然后由 ClickHouse read method 完成一个账户 read model 查询。建议查询拆成两个稳定部分：

1. 读取最新 nav 行和上一 nav 行。
2. 用最新 nav 行的 `trade_date` 聚合当前持仓未实现盈亏。

伪 SQL：

```sql
WITH latest_nav AS (
    SELECT *
    FROM fleur_portfolio.live_nav_daily
    WHERE strategy_portfolio_daily_run_id = {run_id}
      AND result_attempt_id = {attempt}
    ORDER BY trade_date DESC
    LIMIT 1
),
previous_nav AS (
    SELECT *
    FROM fleur_portfolio.live_nav_daily
    WHERE strategy_portfolio_daily_run_id = {run_id}
      AND result_attempt_id = {attempt}
      AND trade_date < (SELECT trade_date FROM latest_nav)
    ORDER BY trade_date DESC
    LIMIT 1
),
position_agg AS (
    SELECT
        sum(unrealized_pnl) AS holding_unrealized_pnl
    FROM fleur_portfolio.live_position_day
    WHERE strategy_portfolio_daily_run_id = {run_id}
      AND result_attempt_id = {attempt}
      AND trade_date = (SELECT trade_date FROM latest_nav)
)
SELECT
    latest_nav.trade_date AS account_date,
    latest_nav.cash_balance,
    latest_nav.position_market_value,
    latest_nav.total_equity,
    latest_nav.daily_return,
    latest_nav.position_count,
    position_agg.holding_unrealized_pnl,
    latest_nav.total_equity - previous_nav.total_equity AS daily_pnl
FROM latest_nav
LEFT JOIN previous_nav ON 1
LEFT JOIN position_agg ON 1
```

实现时可按 ClickHouse 语法调整，重点是不在前端用分页持仓聚合，也不跨不同 result attempt 取数据。

### 后端类型

建议新增 Rust response type：

```rust
struct StrategyPortfolioVirtualAccountResponse {
    source: String,
    strategy_portfolio_id: String,
    strategy_portfolio_daily_run_id: String,
    result_attempt_id: String,
    account_date: NaiveDate,
    currency: String,
    total_equity: f64,
    position_market_value: f64,
    cash_balance: f64,
    holding_unrealized_pnl: f64,
    daily_pnl: Option<f64>,
    daily_return: Option<f64>,
    position_count: u32,
}
```

`currency` 第一版固定为 `CNY`，来源是当前 A 股虚拟账户语境；后续如果账户模板显式支持币种，再改为从账户快照读取。

### 数据校验

后端实现阶段应增加单元测试或集成测试覆盖：

1. 最新 nav 有上一 nav 时，`daily_pnl = latest.total_equity - previous.total_equity`。
2. 最新 nav 无上一 nav 时，`daily_pnl = NULL` 且 `daily_return = NULL` 或沿用表中 NULL。
3. `holding_unrealized_pnl` 只聚合同一 `strategy_portfolio_daily_run_id`、`result_attempt_id`、`trade_date` 的 position rows。
4. 无持仓时 `holding_unrealized_pnl = 0`，但 `total_equity` 和 `cash_balance` 仍来自 nav 行。
5. `pending_first_run` 不返回零值账户。

## 前端设计

### 数据接入

新增前端 API helper：

```ts
getStrategyPortfolioVirtualAccount(strategyPortfolioId: string)
```

新增 query hook：

```ts
useStrategyPortfolioVirtualAccountQuery(strategyPortfolioId: string | null)
```

详情页沿用当前 pending-first-run gate：

```text
isKnownPendingFirstRun ? null : strategyPortfolioId
```

这样 pending 组合不会请求 live account endpoint，或者即使请求返回 `portfolio_pending_first_run`，页面也应映射为空态。

TypeScript 类型建议：

```ts
export type StrategyPortfolioVirtualAccount = {
  source: "live_daily_run"
  strategy_portfolio_id: string
  strategy_portfolio_daily_run_id: string
  result_attempt_id: string
  account_date: string
  currency: "CNY"
  total_equity: number
  position_market_value: number
  cash_balance: number
  holding_unrealized_pnl: number
  daily_pnl?: number | null
  daily_return?: number | null
  position_count: number
}
```

### 页面位置

在 `strategy-detail-page.tsx` 中，新增区块位置固定为：

```text
净值走势
---
虚拟资金账户
---
持仓记录
```

该区块不要放在页面顶部的业绩摘要里。业绩摘要回答“策略表现如何”，虚拟资金账户回答“当前账户资金结构如何”，两者语义不同。

### 信息展示

区块标题固定为「虚拟资金账户」。建议在标题右侧展示 `account_date`，例如：

```text
虚拟资金账户        2026-06-27
```

主内容使用 6 个紧凑指标单元：

| 标签 | 值格式 |
|---|---|
| 账户资产 | 货币金额，保留 2 位小数 |
| 持股市值 | 货币金额，保留 2 位小数 |
| 可用金额 | 货币金额，保留 2 位小数 |
| 持仓盈亏 | 带正负号货币金额，按正负着色 |
| 当日盈亏 | 带正负号货币金额，按正负着色；无上一交易日时显示 `--` |
| 当日盈亏比 | 带正负号百分比，按正负着色；无上一交易日时显示 `--` |

线框：

```text
──────────────────────────────────────────────────────────────────────────────
净值走势
[ chart ]
──────────────────────────────────────────────────────────────────────────────
虚拟资金账户                                      2026-06-27

账户资产          持股市值          可用金额
¥1,012,345.67     ¥812,345.67       ¥200,000.00

持仓盈亏          当日盈亏          当日盈亏比
+¥12,345.67       -¥2,345.67        -0.23%
──────────────────────────────────────────────────────────────────────────────
持仓记录
[ records ]
```

移动端可以保持两列或单列，但每个指标单元必须有稳定高度，避免金额正负变化时导致布局跳动。

### 空态和错误态

| 状态 | 展示 |
|---|---|
| `pending_first_run` | 标题下显示空态：「首个 live daily run 成功后展示虚拟资金账户。」 |
| account query loading | 显示同尺寸骨架或加载态，不挤压上下区块。 |
| account query error | 显示「虚拟资金账户加载失败」，不影响净值走势和持仓记录。 |
| `daily_pnl = NULL` | 当日盈亏显示 `--`。 |
| `daily_return = NULL` | 当日盈亏比显示 `--`。 |
| `position_count = 0` | 持股市值和持仓盈亏可为 `0`，账户资产仍显示现金。 |

## 端到端闭环

```text
rearview-portfolio-worker
  -> 写入 fleur_portfolio.live_nav_daily
  -> 写入 fleur_portfolio.live_position_day

Rearview API
  -> resolve_strategy_portfolio_result(strategy_portfolio_id)
  -> 读取最新 live nav、上一 live nav、同日 position unrealized_pnl 聚合
  -> 返回 /rearview/strategy-portfolios/{id}/virtual-account

Racingline
  -> useStrategyPortfolioVirtualAccountQuery(id)
  -> 在净值走势和持仓记录之间展示「虚拟资金账户」
```

该闭环只消费 worker 已产出的 live 事实，不新增 dbt 计算层，也不让前端推导账户状态。

## 验收标准

1. 已成功 live daily run 的组合详情页，在「净值走势」和「持仓记录」之间展示「虚拟资金账户」。
2. 六个主指标均来自 Rearview virtual account API，不从前端 nav points 或 positions pagination 自行聚合。
3. `账户资产 = total_equity`、`持股市值 = position_market_value`、`可用金额 = cash_balance`。
4. `持仓盈亏` 与同日 position rows 的 `unrealized_pnl` 聚合一致。
5. `当日盈亏` 与最新 nav 和上一 nav 的 `total_equity` 差值一致。
6. `当日盈亏比` 与最新 nav 的 `daily_return` 一致。
7. `pending_first_run` 组合显示空态，不展示 `0` 元账户。
8. 前端在桌面和移动宽度下，六个指标不会互相重叠，金额正负变化不会改变区块整体结构。

## 最小验证命令

后端实现阶段：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端实现阶段：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

文档-only 阶段：

```bash
make docs-check
git diff --check
```

## 待决问题

1. 是否需要把 `currency` 写入账户模板或 execution snapshot，而不是第一版固定 `CNY`。
2. 是否需要在 API 中返回 `previous_account_date`，用于解释当日盈亏的比较基准。
3. 是否需要同时在 Dashboard 卡片上展示简化版账户资产；本 RFC 第一版只覆盖详情页。
4. 持仓盈亏是否需要拆分为未实现盈亏和已实现盈亏；本 RFC 第一版只展示当前持仓未实现盈亏。
