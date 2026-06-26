# Racingline Step2-Step3 Preview Latency Baseline

日期：2026-06-25

范围：Plan 0055 的优化前基线。使用临时 worktree `451620f`（`docs: add racingline preview latency slimming plan`）复现旧实现，采集 `/strategies` 从 Step 2「股池预览」到 Step 3 shell render 的请求链路和耗时。

## 环境

| 项 | 值 |
|---|---|
| Baseline worktree | `/storage/program/mono-fleur-worktrees/0055-baseline` |
| Baseline commit | `451620f` |
| Rearview baseline | `http://127.0.0.1:34058` |
| Racingline baseline | `http://127.0.0.1:5174/strategies` |
| CDP endpoint | `http://127.0.0.1:9222` |
| Browser | Docker `vnc-mini-desktop` Chromium `Chrome/148.0.7778.178` |

临时 worktree 复用主工作区 `.env`，并用 `VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34058` 指向 baseline 后端。

## 启动命令

```bash
git worktree add --detach /storage/program/mono-fleur-worktrees/0055-baseline 451620f
ln -s /storage/program/mono-fleur/.env /storage/program/mono-fleur-worktrees/0055-baseline/.env
ln -s /storage/program/mono-fleur/app/racingline/node_modules /storage/program/mono-fleur-worktrees/0055-baseline/app/racingline/node_modules
```

```bash
cd /storage/program/mono-fleur-worktrees/0055-baseline/engines
REARVIEW_HTTP_BIND=127.0.0.1:34058 cargo run -p rearview-server -- serve
```

```bash
cd /storage/program/mono-fleur-worktrees/0055-baseline/app/racingline
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34058 npm run dev -- --host 127.0.0.1 --port 5174
```

CDP 连通性：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

结果：CDP 连接成功。

## 代表规则

通过真实 UI 构造：

- Step 1：创建一个指标组，默认条件 `close_price >= 0`。
- Step 2：新增一个权重指标，默认评分条件 `close_price >= 0`，分数 `50`。
- 点击 Step 2 底部「股池预览」。

## 5 次样本

| Iteration | Shell render ms | Rearview requests | Preview `output_metrics` |
|---:|---:|---|---|
| 1 | 2773 | `timeline` -> `strategy-preview` -> `security-analysis` -> `strategy-backtests/validate` | `close_price`, `close_price_forward_adj`, `kdj_j_value`, `n_structure_20_is_valid`, `n_structure_20_second_low_ratio`, `rsi_6`, `volume` |
| 2 | 2774 | `timeline` -> `strategy-preview` -> `security-analysis` -> `strategy-backtests/validate` | 同上 |
| 3 | 2385 | `timeline` -> `strategy-preview` -> `security-analysis` -> `strategy-backtests/validate` | 同上 |
| 4 | 2759 | `timeline` -> `strategy-preview` -> `security-analysis` -> `strategy-backtests/validate` | 同上 |
| 5 | 2481 | `timeline` -> `strategy-preview` -> `security-analysis` -> `strategy-backtests/validate` | 同上 |

统计：

| Metric | Value |
|---|---:|
| p50 shell render | 2759 ms |
| p95 shell render | 2774 ms |

## Payload 观察

旧 `security-analysis` response 顶层字段：

```text
adjustment, chart, chart_window, exchange_code, quote_rows, security_board,
security_code, security_name, selected_quote, source, sources, trade_date
```

旧 `chart` 字段：

```text
indicator_panels, ma, price_overlays, series
```

旧链路即使请求 `include_quote_rows=false`，response 仍返回空 `quote_rows` 字段；Step 3 未展示的 `sources`、`price_overlays`、`indicator_panels` 也随通用 analysis contract 返回。

## 结论

- 旧 Step2->3 硬等待链路包含 4 个接口：`timeline`、`strategy-preview`、`security-analysis`、`strategy-backtests/validate`。
- `security-analysis` 在 Step 3 shell render 前被 await，直接拉长用户感知延时。
- `strategy-backtests/validate` 在 Step 3 首屏自动触发，属于 Step 4/5 gate，旧链路触发过早。
- close-only 代表规则仍被默认 `output_metrics` 污染，携带 `kdj_j_value` 和 `rsi_6`，会把 momentum mart 依赖带入 preview planning。
- baseline worktree 前端因为 symlink 复用 `node_modules` 出现 Vite font allow-list warning；页面和网络采样可用，未影响本次 Step2->3 数据。

## Cleanup

baseline 后端和前端已用 Ctrl-C 停止。采样完成后，临时 worktree 已通过 `git worktree remove /storage/program/mono-fleur-worktrees/0055-baseline` 删除。
