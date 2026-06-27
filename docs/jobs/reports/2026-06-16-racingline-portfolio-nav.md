# Racingline Portfolio NAV Smoke Report

日期：2026-06-16

范围：Plan 0041 第一版组合净值链路 smoke，覆盖 Rearview server、PostgreSQL migration、NATS JetStream outbox dispatch、`rearview-portfolio-worker`、Racingline `/portfolios` 页面和明细 API。

## 环境

- PostgreSQL：`fleur-postgres`，healthy
- ClickHouse：`fleur-clickhouse`，healthy
- NATS：`fleur-nats`，healthy，`GET http://127.0.0.1:34056/healthz` 返回 `{"status":"ok"}`
- Rearview server：`cargo run -p rearview-server -- serve`
- Rearview portfolio worker：`cargo run -p rearview-portfolio-worker -- run`
- Racingline：`npm run dev -- --host 127.0.0.1 --port 5173`

启动入口：

```bash
make racingline-dev
```

## 样本

Source run：

```text
run_id = a4470e63-6fd3-46ce-9dab-8802c84cef26
rule_set_id = fa457364-0c32-4e27-82dd-edc60601420b
date_range = 2026-05-20 / 2026-06-01
top_n = 3
status = succeeded
summary.signal_count = 27
```

历史样本 rule set 缺少默认虚拟账户模板，先通过 API 创建默认模板：

```text
account_template_id = c4d292a5-2916-46d6-9d81-7f0d919a3860
initial_cash = 1,000,000 CNY
fee_template = CN_A_SHARE active default
max_positions = 3
slippage = 10 bps buy/sell
exit_rules = fixed_stop_loss(8%), take_profit(20%)
```

组合运行：

```text
portfolio_run_id = 39d740f8-993c-465a-b1b9-dbe5d48ca1e0
status = succeeded
dispatch_status = published
nats_stream_sequence = 1
price_basis = backward_adjusted
```

## API 验证

创建组合运行：

```bash
curl -fsS -X POST "http://127.0.0.1:34057/rearview/portfolio-runs" \
  -H "content-type: application/json" \
  -d '{"source_run_id":"a4470e63-6fd3-46ce-9dab-8802c84cef26"}'
```

终态 summary：

```json
{
  "ending_equity": 939305.980573453,
  "initial_cash": 1000000.0,
  "max_drawdown": -0.07947923946727053,
  "total_fee": 322.9883940101826,
  "total_return": -0.06069401942654695,
  "trade_count": 5,
  "warning_count": 0
}
```

明细计数：

| 明细 | 结果 |
|---|---:|
| `portfolio_nav` | 9 rows |
| `portfolio_target` | 4 rows |
| `portfolio_order` | 5 rows |
| `portfolio_trade` | 5 rows |
| latest `portfolio_position_day` | 3 rows |
| `portfolio_event` | 0 rows |

一致性核验：

| 检查 | 结果 |
|---|---|
| first nav | `trade_date=2026-05-20`, `nav=1.0`, `cash_balance=1000000.0`, `position_count=0` |
| `summary.trade_count` vs trades | `5 == 5` |
| `summary.total_fee` vs trade fee sum | `322.9883940101826` vs `322.9883`，仅展示精度差异 |
| last nav position count vs latest positions | `3 == 3` |

## UI 验收

Playwright CDP：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

页面检查：

- `/portfolios/39d740f8-993c-465a-b1b9-dbe5d48ca1e0` 桌面宽度显示 `succeeded`、`published`、summary、NAV tab、Trades、Orders、Targets。
- `/portfolios/39d740f8-993c-465a-b1b9-dbe5d48ca1e0` 移动宽度 `390x844` 显示组合 run 标题、`succeeded`、`published`、Summary、NAV、Trades、Orders、Targets。
- `/portfolios` 桌面宽度显示组合列表行：`39d740f8`、source run `a4470e63`、`succeeded`、`published`、ending equity `939,305.98`。

截图：

- [assets/2026-06-16-racingline-portfolio-detail.png](assets/2026-06-16-racingline-portfolio-detail.png)
- [assets/2026-06-16-racingline-portfolio-list.png](assets/2026-06-16-racingline-portfolio-list.png)

## 已知限制

1. 本次 smoke 未模拟 NATS 停止后恢复的 pending outbox 重试路径。
2. 本地未安装 NATS CLI，未手工重复发布同一个 `portfolio_run_id` 消息；当前覆盖来自 worker 终态 ack 逻辑单元测试和 run 级结果替换实现。
3. `indicator_stop_loss` 第一版按 worker validation error 处理，尚未接入后复权兼容指标输入。
