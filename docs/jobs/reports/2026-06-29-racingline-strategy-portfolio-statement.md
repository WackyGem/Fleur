# 2026-06-29 Racingline 策略组合对账单验收

状态：Completed

范围：Plan 0062 对账单 read model、Racingline 详情页展示、Dagster 清算终态核验、2025 年初 T+1 建仓验收样本。

## 结论

已用真实 Rearview、Rearview portfolio worker、Dagster 和 ClickHouse 完成对账单端到端验收。验收样本按用户调整从 `2025-01-02` 起查找首个真实买入信号，实际信号日为 `2025-01-07`，建仓日为 T+1 `2025-01-08`。dev seed 只写 PostgreSQL 控制面，未写入 ClickHouse `live_*` facts；`live_*` facts 由 worker 清算生成。

组合与 attempt：

| 字段 | 值 |
|---|---|
| `strategy_portfolio_id` | `01KW8EY4YZ847G8P2EY4WWMBM2` |
| `source_strategy_backtest_run_id` | `5cfbe7c6-99a8-4f31-a8c7-b55ca2d9bc72` |
| `source_result_attempt_id` | `dev-statement-source-attempt-first-signal-v1` |
| `initial_signal_date` | `2025-01-07` |
| `live_start_date` | `2025-01-08` |
| `latest_daily_run_id` | `01KW8FS5AVZEVD592AED99X0J3` |
| `latest_result_attempt_id` | `01KW8FSC6TY66E35C626ETW5Y1` |
| latest trade date | `2026-06-26` |

## Seed

执行命令：

```bash
set -a; source /storage/program/fleur/.env; set +a
cd engines
cargo run -q -p rearview-server -- dev seed-statement-portfolio
```

关键输出：

```json
{
  "signal_search_start_date": "2025-01-02",
  "signal_search_end_date": "2025-12-31",
  "initial_signal_date": "2025-01-07",
  "live_start_date": "2025-01-08",
  "live_facts_written_by_seed": false,
  "pending_buy_signal_count": 1
}
```

## Dagster

执行命令：

```bash
set -a; source /storage/program/fleur/.env; set +a
export DAGSTER_HOME=/storage/program/fleur/.dagster
cd pipeline
uv run dg launch --target-path scheduler \
  --job strategy_portfolio__daily_run_job \
  --partition 2026-06-26 \
  --config-json '{"ops":{"rearview__strategy_portfolio_daily_runs":{"config":{"start_date":"2026-06-26","end_date":"2026-06-26","strategy_portfolio_id":"01KW8EY4YZ847G8P2EY4WWMBM2","wait_for_completion":true,"poll_interval_seconds":5,"timeout_seconds":1800,"chunk_size":20}}}}'
```

Dagster run `7dea2dfc-c2a2-4435-86c3-f5f63e60b91a` 成功。materialization metadata：

| 字段 | 值 |
|---|---|
| `created_run_count` | `0` |
| `skipped_run_count` | `1` |
| `succeeded_run_count` | `1` |
| `failed_run_count` | `0` |
| `timeout_run_count` | `0` |
| `latest_daily_run_id` | `01KW8FS5AVZEVD592AED99X0J3` |
| `latest_result_attempt_id` | `01KW8FSC6TY66E35C626ETW5Y1` |
| `nav_row_count` | `353` |
| `trade_row_count` | `914` |
| `closed_trade_row_count` | `457` |

该 run 对已有 daily run 做幂等跳过和 fact-count 核验；不是只验证“创建 daily run 成功”。

## Data Plane

ClickHouse 直接核验：

| 查询 | 结果 |
|---|---|
| `live_nav_daily` | `2025-01-08..2026-06-26`, `353` rows |
| `live_trade` | `2025-01-08..2026-05-27`, `914` rows |
| 非 100 股整数倍 trade | `0` rows |
| `live_closed_trade` | `2025-01-13..2026-05-27`, `457` rows |
| `2026-01-02` 后 nav 交易日 | `114` rows |

Daily run status API 返回 `status=succeeded`、`dispatch_status=published`、`worker_attempt_no=1`，worker summary 中 `trade_count=914`、`total_return=0.0627042642`。

## Statement API

`period=all`：

| 字段 | 值 |
|---|---|
| range | `2025-01-08..2026-06-26` |
| `average_position_pct` | `0.2663091821` |
| `traded_security_count` | `403` |
| `trade_count` | `914` |
| `trade_win_rate` | `0.3719912473` |
| `winning_security_count` | `147` |
| `losing_security_count` | `256` |
| `holding_days` | `270` |

`period=three_months`：

| 字段 | 值 |
|---|---|
| range | `2026-03-26..2026-06-26` |
| `average_position_pct` | `0.0655752697` |
| `traded_security_count` | `28` |
| `trade_count` | `56` |
| `trade_win_rate` | `0.3214285714` |
| `winning_security_count` | `9` |
| `losing_security_count` | `19` |
| `holding_days` | `20` |

首条 operation row 包含价格、数量、手数、金额、费用、成交后持仓余额和实现盈亏：

```json
{
  "trade_date": "2026-05-27",
  "security_code": "002183.SZ",
  "security_name": "怡亚通",
  "side": "sell",
  "quantity": 1000.0,
  "lot_size": 100,
  "lot_count": 10.0,
  "total_fee": 62.99416532359582,
  "position_balance_quantity": 0.0,
  "realized_pnl": -2411.741372608866
}
```

## Browser Evidence

浏览器环境：

| 项 | 值 |
|---|---|
| CDP endpoint | `http://127.0.0.1:9222` |
| Browser | `Chrome/148.0.7778.178` |
| Dev server | `http://127.0.0.1:5173/` |
| Rearview | `http://127.0.0.1:34057/` |

截图保存于 `docs/jobs/reports/assets/2026-06-29/statement/`：

| 证据 | 文件 |
|---|---|
| 桌面对账单 summary 和 period | [statement-acceptance-06-summary-periods-desktop.png](assets/2026-06-29/statement/statement-acceptance-06-summary-periods-desktop.png) |
| 桌面全部区间操作记录 | [statement-acceptance-07-operations-all-desktop.png](assets/2026-06-29/statement/statement-acceptance-07-operations-all-desktop.png) |
| 桌面近三月操作记录 | [statement-acceptance-08-operations-period-desktop.png](assets/2026-06-29/statement/statement-acceptance-08-operations-period-desktop.png) |
| 移动端全部区间 summary/table | [statement-acceptance-09-summary-mobile.png](assets/2026-06-29/statement/statement-acceptance-09-summary-mobile.png) |
| 移动端近三月 summary/table | [statement-acceptance-10-operations-mobile.png](assets/2026-06-29/statement/statement-acceptance-10-operations-mobile.png) |

本次验收路径使用 dev seed 替代 Step 1/2/4/publish UI 重跑，因此不生成 `statement-acceptance-01` 到 `05`。对应证据由 seed 命令输出、Dagster materialization metadata 和 ClickHouse SQL 结果覆盖，不伪造 UI 截图。截图通过 `mini-desktop` 的 CDP 页面 target 采集，未启动本机新浏览器。

Playwright/浏览器检查：

| 检查 | 结果 |
|---|---|
| console errors | `0` |
| console warnings | `0` |
| statement network | `GET /statement?period=month` 和 `GET /statement?period=all` 均为 `200 OK` |
| desktop viewport | `1440x1100` |
| mobile viewport | `390x844` |

## Validation

已完成验证：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests -q
cd scheduler
uv run dg check defs

cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build

make docs-check
git diff --check
```

结果：全部通过。`npm run build` 仍输出 Vite chunk size warning，这是当前前端 bundle 的既有提示，不影响构建成功。
