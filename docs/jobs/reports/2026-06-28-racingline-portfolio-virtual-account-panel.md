# Racingline Portfolio Virtual Account Panel

日期：2026-06-28

范围：Plan 0061 的 Rearview strategy portfolio virtual account read model、Racingline 组合详情页「虚拟资金账户」区块、pending-first-run 语义、桌面/移动端浏览器验收和质量门禁。

## 实现范围

| 层级 | 结果 |
|---|---|
| Rearview API | 新增 `GET /rearview/strategy-portfolios/{strategy_portfolio_id}/virtual-account` |
| Rearview read model | 返回 `total_equity`、`position_market_value`、`cash_balance`、`holding_unrealized_pnl`、`daily_pnl`、`daily_return` 和 `position_count` |
| ClickHouse 读取 | 最新两条 `fleur_portfolio.live_nav_daily` 计算当日盈亏；同一 `account_date` 聚合 `fleur_portfolio.live_position_day.unrealized_pnl` 作为当前持仓未实现盈亏 |
| pending-first-run | 沿用 live detail endpoint 语义返回 `409 portfolio_pending_first_run`，不回退 source backtest，不生成 0 元账户 |
| Racingline | 在组合详情页「净值走势」和「持仓记录」之间展示「虚拟资金账户」 |
| UI | 无边框、无底色、数字 `text-sm`；金额不展示 `¥`；持仓盈亏只展示当前持仓未实现盈亏 |

## API Smoke

Live succeeded 样本：

| 字段 | 值 |
|---|---|
| Portfolio id | `01KW3K9453WXGX3B1KJFZYQRVC` |
| Daily run id | `01KW3K9CRKA235WB2RKCGDHS7M` |
| Result attempt | `01KW3K9ECN3NZQD641Q65RH9FZ` |
| Account date | `2026-06-25` |
| Total equity | `936250.0031081872` |
| Position market value | `407047.6523553816` |
| Cash balance | `529202.3507528055` |
| Holding unrealized PnL | `-63749.99689181314` |
| Daily PnL | `-10186.835309946327` |
| Daily return | `-0.010763354612202791` |
| Position count | `5` |

Pending-first-run 样本：

| 字段 | 值 |
|---|---|
| Portfolio id | `01KW4FZEE6HVTJW44MZ0F8DYB2` |
| HTTP status | `409 Conflict` |
| `error_type` | `portfolio_pending_first_run` |
| 结论 | 没有 live daily run result 时不返回虚拟账户数值 |

## 浏览器 Smoke

通过 `playwright-cli` 连接现有 CDP 浏览器：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

结果：

| 检查 | 结果 |
|---|---|
| Desktop `1440x900` | 「虚拟资金账户」展示在「净值走势」和「持仓记录」之间，6 个指标完整展示 |
| Mobile `390x844` | 6 个指标无重叠 |
| Pending portfolio | 显示「尚未产生虚拟资金账户」空态，`hasZeroAccount=false` |
| 样式 | 账户指标容器无 `border`、无 `bg-muted` 底色，数值类名为 `text-sm` |
| 金额符号 | 账户区块文本 `containsYen=false` |
| Network | `/rearview/strategy-portfolios/01KW3K9453WXGX3B1KJFZYQRVC/virtual-account` 返回 `200 OK` |
| Console | 无 error / warning；仅 React DevTools info |

截图保存在本地调试目录：

```text
.playwright-cli/virtual-account-no-yen-desktop.png
.playwright-cli/virtual-account-no-yen-mobile.png
```

## 验证命令

已通过：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

```bash
cd app/racingline
npm run typecheck
npm test -- --run
npm run lint
npm run build
```

```bash
make docs-check
git diff --check
```

Vite build 仍输出既有 chunk-size warning；本轮未改变该风险。
