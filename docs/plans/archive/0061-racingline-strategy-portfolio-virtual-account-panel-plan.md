# Plan 0061: Racingline 组合详情页虚拟资金账户实施计划

日期：2026-06-28

状态：Completed

领域：racingline, rearview

关联系统：racingline, rearview, data-platform

代码根：

- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/elt/`

关联文档：

- [RFC 0035: Racingline 组合详情页虚拟资金账户](../../RFC/archive/0035-racingline-strategy-portfolio-virtual-account-panel.md)
- [RFC 0029: Racingline 回测结果发布为策略组合与 Dagster 日运行资产](../../RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md)
- [Plan 0060: Racingline Step 5 建立组合弹层与 T+1 建仓语义实施计划](0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md)
- [Racingline 系统地图](../../architecture/racingline.md)
- [Rearview 系统地图](../../architecture/rearview.md)

## 背景

RFC 0035 确认在组合详情页 `/dashboard/strategies/:portfolioId` 的「净值走势」和「持仓记录」之间新增「虚拟资金账户」区块，展示：

1. 账户资产。
2. 持股市值。
3. 可用金额。
4. 持仓盈亏。
5. 当日盈亏。
6. 当日盈亏比。

用户已确认第一版「持仓盈亏」只展示当前持仓的未实现盈亏，不展示已实现盈亏，也不把已平仓交易收益并入账户卡片。

本计划把 RFC 0035 拆成可实施、可测试、可验收的后端 read model、前端 API hook、页面区块和浏览器验证步骤。

## 文档 review 结论

已 review RFC 0035、当前 Racingline 详情页、Rearview live endpoint 和 ClickHouse portfolio schema，补充以下实现缺口：

| 缺口 | 处理结论 |
|---|---|
| 现有 `/nav` endpoint 已读取账户金额字段，但 response 复用 `StrategyBacktestNavPoint`，没有暴露 `cash_balance`、`position_market_value` 和 `total_equity` | 不扩展 `/nav` 图表 response，新增 `/virtual-account` 专用 read model，避免图表 API 继续膨胀 |
| 现有 `/positions` endpoint 是分页明细，不适合前端求和 | 后端聚合 `live_position_day.unrealized_pnl`，前端不得用分页明细计算持仓盈亏 |
| 当日盈亏金额没有现成列 | 后端用最新 nav 行和上一 nav 行的 `total_equity` 差值派生；不得用 `total_equity * daily_return` 反推 |
| pending-first-run 组合没有 live 账户事实 | endpoint 沿用 live detail 语义返回 `portfolio_pending_first_run`；前端展示空态，不显示 0 元账户 |
| 首个 live nav 日期没有上一交易日 | `daily_pnl` 和 `daily_return` 均允许为 `null`，前端显示 `--` |
| `currency` 目前没有账户字段 | 第一版 response 固定 `CNY`，不新增 migration；后续如账户模板支持币种再升级 |
| 持仓盈亏容易被误解为包含已平仓收益 | API 字段命名使用 `holding_unrealized_pnl`，UI 文案保持「持仓盈亏」，计划和测试明确不包含已实现盈亏 |
| 页面需要插入在两个既有区块之间 | 实施时只能放在「净值走势」后、「持仓记录」前，不能挪到顶部业绩摘要 |
| 前端布局可能在移动端金额过长时溢出 | 指标单元需要稳定高度、tabular nums、换行或缩小策略，并用浏览器截图验证 |

## 目标

1. Rearview 新增 strategy portfolio virtual account endpoint，返回账户 read model。
2. read model 使用同一 `strategy_portfolio_daily_run_id`、`result_attempt_id` 和最新 `account_date` 读取 nav 与 position facts。
3. 后端返回账户资产、持股市值、可用金额、持仓未实现盈亏、当日盈亏和当日盈亏比。
4. Racingline 新增 API client、query hook 和 TypeScript 类型。
5. 组合详情页在「净值走势」和「持仓记录」之间展示「虚拟资金账户」。
6. pending-first-run、首日无上一交易日、无持仓、加载失败等状态均有明确 UI。
7. 完成后通过 Rust、前端和文档最小质量门禁，并进行桌面/移动浏览器验收。

## 非目标

1. 不展示已实现盈亏、已平仓交易收益或交易级归因。
2. 不新增 ClickHouse 表、PostgreSQL migration 或 dbt model。
3. 不修改 worker 的净值递推、现金、成交、费用或持仓计算公式。
4. 不在前端重算现金、持股市值、账户资产、当日盈亏或持仓盈亏。
5. 不把 source backtest 账户数据回退展示到 live strategy portfolio 详情页。
6. 不扩展 Dashboard 卡片；第一版只改组合详情页。
7. 不引入真实券商账户字段，例如可取金额、冻结金额、保证金或融资融券余额。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| 详情页路由 | [App.tsx](../../../app/racingline/src/App.tsx) 定义 `/dashboard/strategies/:portfolioId`，由 [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx) 渲染。 |
| 详情页区块顺序 | [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx) 当前先展示「净值走势」，随后展示「持仓记录」。 |
| 前端 nav API | [app/racingline/src/api/rearview.ts](../../../app/racingline/src/api/rearview.ts) 的 `listStrategyPortfolioNav()` 调用 `/rearview/strategy-portfolios/{id}/nav`。 |
| 前端 nav 类型 | [app/racingline/src/types/rearview.ts](../../../app/racingline/src/types/rearview.ts) 的 `StrategyPortfolioNavResponse.points` 复用 `StrategyBacktestNavPoint`，没有账户金额字段。 |
| 前端 pending 处理 | [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx) 已有 `isPortfolioPendingFirstRunError()`，识别 `portfolio_pending_first_run`。 |
| 后端路由 | [api/mod.rs](../../../engines/crates/rearview-core/src/api/mod.rs) 已有 `/rearview/strategy-portfolios/{strategy_portfolio_id}/nav`、`performance`、`signals`、`positions` 和 `rebalance-records`。 |
| live 结果解析 | [api/mod.rs](../../../engines/crates/rearview-core/src/api/mod.rs) 的 `resolve_strategy_portfolio_result()` 在 pending-first-run 时返回 `RearviewError::PortfolioPendingFirstRun`。 |
| live nav 查询 | [clickhouse/mod.rs](../../../engines/crates/rearview-core/src/clickhouse/mod.rs) 的 `query_strategy_portfolio_live_nav()` 已查询 `cash_balance`、`position_market_value`、`total_equity` 和 `daily_return`。 |
| live position 查询 | [clickhouse/mod.rs](../../../engines/crates/rearview-core/src/clickhouse/mod.rs) 的 `query_strategy_portfolio_live_positions()` 已查询 `market_value`、`unrealized_pnl` 和 `unrealized_return`，但以分页明细返回。 |
| ClickHouse schema | [portfolio_schema.rs](../../../engines/crates/rearview-core/src/clickhouse/portfolio_schema.rs) 的 `live_nav_daily` 包含账户金额字段，`live_position_day` 包含未实现盈亏字段。 |
| worker 写入 | [portfolio_write.rs](../../../engines/crates/rearview-core/src/clickhouse/portfolio_write.rs) 已把 nav 和 position 字段写入 ClickHouse；本计划不改写入路径。 |

## 目标 API contract

新增：

```http
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/virtual-account
```

Response：

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

字段口径：

| 字段 | 口径 |
|---|---|
| `account_date` | 最新 live nav 行的 `trade_date`。 |
| `total_equity` | 最新 live nav 行的 `total_equity`。 |
| `position_market_value` | 最新 live nav 行的 `position_market_value`。 |
| `cash_balance` | 最新 live nav 行的 `cash_balance`。 |
| `holding_unrealized_pnl` | 同一 `account_date` 的 `live_position_day.unrealized_pnl` 聚合；无持仓为 `0`。 |
| `daily_pnl` | 最新 nav 行 `total_equity` 减上一 nav 行 `total_equity`；无上一 nav 行为 `null`。 |
| `daily_return` | 最新 nav 行 `daily_return`；无上一 nav 行为 `null`。 |
| `position_count` | 最新 live nav 行的 `position_count`。 |

## 实施阶段

### Phase 1: Rearview virtual account read model

目标：新增后端只读 read model，不改变 worker 写入和现有 nav/positions endpoints。

实施项：

1. 在 Rearview API router 中注册 `GET /rearview/strategy-portfolios/{strategy_portfolio_id}/virtual-account`。
2. 新增 `StrategyPortfolioVirtualAccountResponse` API response type。
3. 新增 ClickHouse read type，例如 `StrategyPortfolioVirtualAccountRecord`。
4. 新增 `query_strategy_portfolio_virtual_account(strategy_portfolio_daily_run_id, result_attempt_id)`。
5. 查询必须读取最新 nav、上一 nav 和同日 position 未实现盈亏聚合。
6. `holding_unrealized_pnl` 使用 `coalesce(sum(unrealized_pnl), 0)` 语义。
7. endpoint 复用 `resolve_strategy_portfolio_result()`，pending-first-run 保持 `portfolio_pending_first_run`。
8. 没有 live nav 行时返回显式 not found/internal data error，不返回全零账户。

测试策略：

1. 给派生 helper 增加 Rust 单测：有上一 nav、无上一 nav、无持仓、负收益。
2. 给 SQL builder 或查询方法增加字符串级单测：必须过滤 run id、result attempt，并按最新 nav trade_date 聚合 positions。
3. 给 pending-first-run 路径增加 handler 或 resolver 单测，确认错误类型不变。

完成标准：

1. 新 endpoint 返回完整 JSON contract。
2. `daily_pnl` 不通过 `total_equity * daily_return` 派生。
3. 不修改 `/nav`、`/positions` 既有 response contract。

### Phase 2: Racingline API client and state integration

目标：前端能以独立 query 获取虚拟资金账户 read model。

实施项：

1. 在 `app/racingline/src/types/rearview.ts` 新增 `StrategyPortfolioVirtualAccount` 类型。
2. 在 `app/racingline/src/api/rearview.ts` 新增 `getStrategyPortfolioVirtualAccount()`。
3. 在 `app/racingline/src/api/hooks.ts` 新增 query key 和 `useStrategyPortfolioVirtualAccountQuery()`。
4. query enabled 条件沿用 live result gate：`pending_first_run` 时不主动请求 account endpoint。
5. 如果 endpoint 仍返回 `portfolio_pending_first_run`，页面层继续映射为空态。

测试策略：

1. 前端 API 单测覆盖 path：`/rearview/strategy-portfolios/{id}/virtual-account`。
2. hook query key 不复用 nav/performance key，避免缓存串线。

完成标准：

1. TypeScript 类型检查能约束六个主指标字段。
2. account query 不影响 nav、performance、positions 和 rebalance records 的错误处理。

### Phase 3: 详情页「虚拟资金账户」区块

目标：在指定位置展示账户数据，并覆盖 loading、empty 和 error 状态。

实施项：

1. 在 [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx) 的「净值走势」后、「持仓记录」前插入新区块。
2. 区块标题固定为「虚拟资金账户」，标题右侧展示 `account_date`。
3. 展示六个指标：账户资产、持股市值、可用金额、持仓盈亏、当日盈亏、当日盈亏比。
4. 货币金额统一使用人民币格式，保留 2 位小数。
5. `holding_unrealized_pnl`、`daily_pnl` 和 `daily_return` 按正负着色。
6. `daily_pnl = null` 或 `daily_return = null` 时显示 `--`。
7. pending-first-run 空态文案：`首个 live daily run 成功后展示虚拟资金账户。`
8. account query loading 和 error 状态不能挤压上下区块或影响持仓记录展示。

测试策略：

1. 对金额格式化和正负 class helper 增加单测，或复用现有 formatter 并补展示测试。
2. 覆盖 pending、null daily pnl、0 持仓、负当日盈亏四类渲染状态。

完成标准：

1. 区块位置符合 RFC：净值走势和持仓记录之间。
2. 不在页面顶部业绩摘要重复展示账户资产。
3. 不从 `navQuery.data.points` 或 `positionsQuery.data.items` 自行计算六个主指标。

### Phase 4: Browser and live smoke validation

目标：确认页面在真实 dev 环境和不同 viewport 下可用。

实施项：

1. 启动 `make racingline-dev`。
2. 打开一个已有 succeeded live daily run 的组合详情页。
3. 验证「虚拟资金账户」显示六个主指标和日期。
4. 打开 pending-first-run 组合详情页，验证空态。
5. 用 Playwright/CDP 截图检查桌面和移动宽度下文本不重叠。
6. 记录必要 smoke 命令和样本 portfolio id 到 job report。

测试策略：

1. 使用 `playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"` 连接现有浏览器。
2. 检查 console error 和 network error。
3. 如暂无 live succeeded 样本，先用现有 runbook 创建或复用最近 smoke 数据，不手工造假 API response。

完成标准：

1. desktop 和 mobile 截图中六个指标不重叠。
2. pending-first-run 不显示 0 元账户。
3. live 账户金额与 API response 一致。

### Phase 5: 文档和归档

目标：实现完成后收敛当前事实和运行记录。

实施项：

1. 更新 [docs/architecture/racingline.md](../../architecture/racingline.md)，说明组合详情页新增虚拟资金账户区块。
2. 更新 [docs/architecture/rearview.md](../../architecture/rearview.md)，说明新增 virtual account read endpoint。
3. 新增 `docs/jobs/reports/YYYY-MM-DD-racingline-portfolio-virtual-account-panel.md`，记录 API 和浏览器验收。
4. 完成后把本计划移入 `docs/plans/archive/`，并更新 [docs/plans/README.md](README.md)。

测试策略：

1. 文档-only 更新运行 `make docs-check` 和 `git diff --check`。

完成标准：

1. 架构事实文档、job report 和计划归档状态一致。
2. RFC 0035 和本计划作为实施记录归档。

## 禁止模式

1. 禁止前端用 nav points 或 positions pagination 自行聚合账户资产、持股市值、可用金额、持仓盈亏或当日盈亏。
2. 禁止用 `total_equity * daily_return` 反推当日盈亏。
3. 禁止 pending-first-run 显示零值账户或回退 source backtest 账户状态。
4. 禁止把已实现盈亏、closed trade realized pnl 或交易级收益混入 `holding_unrealized_pnl`。
5. 禁止为了账户卡片扩展 `/nav` response，造成图表 endpoint 承担账户 read model 职责。
6. 禁止新增 dbt 模型重算 worker 已经写入的 NAV、现金或持仓盈亏。
7. 禁止修改 worker 计算公式来适配 UI 展示；UI 只消费已有事实和后端只读派生。

## 允许保留的例外

1. `currency` 第一版固定为 `CNY`。
2. 首个 live nav 日期允许 `daily_pnl = null` 和 `daily_return = null`。
3. 无持仓时 `holding_unrealized_pnl = 0`，但账户资产和可用金额仍来自 nav 行。
4. 如果实现阶段发现 ClickHouse 单 SQL 对空 `previous_nav` 处理复杂，可以拆成两次查询，但必须保持同一 run id、result attempt 和 latest account date。

## 最小验证命令

后端：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

文档：

```bash
make docs-check
git diff --check
```

端到端 smoke：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## 完成标准

1. Rearview 提供 `/rearview/strategy-portfolios/{id}/virtual-account`。
2. API 对 live succeeded 组合返回六个账户主指标和 `account_date`。
3. API 对 pending-first-run 组合保持 `portfolio_pending_first_run` 语义。
4. Racingline 详情页在「净值走势」和「持仓记录」之间展示「虚拟资金账户」。
5. 持仓盈亏只展示当前持仓未实现盈亏，不展示已实现盈亏。
6. 当日盈亏金额与最新 nav 和上一 nav 的 `total_equity` 差值一致。
7. 前端不自行聚合或反推账户 read model。
8. Rust、前端、文档和浏览器 smoke 验证通过。
