# Racingline Step2-Step3 Preview Latency Slimming

日期：2026-06-25

范围：Plan 0055，Racingline `/strategies` 从 Step 2「权重配置」进入 Step 3「股池预览」的接口链路瘦身、脏字段清理和延时验收。

## 实现摘要

Rearview:

- 新增 route 级 HTTP timing 日志，记录 `method`、matched `route`、`status`、`elapsed_ms`。
- 新增 ClickHouse query timing 日志，记录 `query_id`、`status`、`elapsed_ms`、`response_bytes`。
- 新增 `POST /rearview/strategy-preview/open`，把 Step 3 首屏 hard-blocking 的 timeline + latest preview 合并为一个前端 round trip。
- 新增 `POST /rearview/strategy-preview/chart-context`，只返回 Step 3 当前页面消费的证券显示信息、OHLC、volume、MA5/MA10/MA30 和右侧行情/估值字段。
- `chart-context` 不接收 `RuleVersionSpec`，不重跑 preview pool-page，不调用 `query_analysis_momentum_rows()`。

Racingline:

- `openPreview()` 改为调用 `strategy-preview/open`，成功后立即 `setPreviewSnapshot()` 和 `setActiveStep("preview")`。
- 删除 Step2->3 hard wait 中的 analysis prefetch；Step 3 图表由本地 hook 在页面 render 后局部 loading。
- `strategy-backtests/validate` 增加 enabled gate，只在 Step 4/5 需要 execution draft 时触发。
- Step 3 从 `usePreviewSecurityAnalysisQuery()` 切到 `usePreviewChartContextQuery()`。
- 删除 Step 3 请求中的 `include_quote_rows`，K 线图不再从 `price_overlays` fallback 取 MA。
- Preview rule adapter 不再把 catalog `default_output` 全量加入 `output_metrics`；只输出 Step 1/2 规则实际使用的指标，避免 close-only 规则携带 KDJ/RSI 脏依赖。

## After Samples

环境：

| 项 | 值 |
|---|---|
| Rearview | `http://127.0.0.1:34057` |
| Racingline | `http://127.0.0.1:5173/strategies` |
| CDP endpoint | `http://127.0.0.1:9222` |
| Browser | Docker `vnc-mini-desktop` Chromium `Chrome/148.0.7778.178` |

代表规则同 baseline：

- Step 1：`close_price >= 0`
- Step 2：`close_price >= 0`，分数 `50`

| Iteration | Shell render ms | Rearview requests during Step2->3 | Preview `output_metrics` | Chart context fields |
|---:|---:|---|---|---|
| 1 | 743 | `strategy-preview/open`, `strategy-preview/chart-context` | `close_price` | `chart`, `security_board`, `security_code`, `security_name`, `selected_quote` |
| 2 | 725 | 同上 | `close_price` | 同上 |
| 3 | 722 | 同上 | `close_price` | 同上 |
| 4 | 769 | 同上 | `close_price` | 同上 |
| 5 | 723 | 同上 | `close_price` | 同上 |

统计：

| Metric | Before | After |
|---|---:|---:|
| p50 shell render | 2759 ms | 725 ms |
| p95 shell render | 2774 ms | 769 ms |
| Hard-blocking frontend round trips | 3 before shell (`timeline`, `preview`, `security-analysis`) | 1 before shell (`strategy-preview/open`) |
| Step 3 first-screen `strategy-backtests/validate` | triggered | not triggered |
| Step 3 first-screen `security-analysis` | triggered and awaited | not triggered |

## Network Contract Evidence

Final Step2->3 `strategy-preview/open` request used:

```json
{
  "rule": {
    "pool_filters": {"type": "all"},
    "scoring": {"rules": [{"type": "conditional_points"}]},
    "output_metrics": ["close_price"]
  },
  "start_date": "2025-06-25",
  "end_date": "2026-06-25",
  "preview_row_limit": 10
}
```

Final `chart-context` response keys:

```text
chart, security_board, security_code, security_name, selected_quote
```

Final `selected_quote` keys:

```text
a_market_cap, amount, close_price, high_price, limit_down_price,
limit_up_price, low_price, open_price, pct_amplitude, pct_change,
pe_ttm, prev_close_price, roe, volume
```

Observed absent from final Step 3 chart context payload:

```text
kdj, rsi, macd, boll, price_overlays, indicator_panels, sources, quote_rows
```

## Backend Log Evidence

Representative final logs:

```text
rearview http request method=POST route=/rearview/strategy-preview/open status=200 elapsed_ms=123
clickhouse query query_id="rearview-preview-open-timeline-..." status=200 elapsed_ms=48 response_bytes=10396
clickhouse query query_id="rearview-preview-open-latest-..." status=200 elapsed_ms=50 response_bytes=3067
clickhouse query query_id="rearview-preview-open-latest-...-display" status=200 elapsed_ms=10 response_bytes=1151

rearview http request method=POST route=/rearview/strategy-preview/chart-context status=200 elapsed_ms=205
clickhouse query query_id="rearview-preview-chart-context-...-display" status=200 elapsed_ms=10 response_bytes=117
clickhouse query query_id="rearview-preview-chart-context-...-date-window" status=200 elapsed_ms=72 response_bytes=28
clickhouse query query_id="rearview-preview-chart-context-...-selected-quote" status=200 elapsed_ms=13 response_bytes=367
clickhouse query query_id="rearview-preview-chart-context-...-chart-quotes" status=200 elapsed_ms=72 response_bytes=52075
clickhouse query query_id="rearview-preview-chart-context-...-trend" status=200 elapsed_ms=29 response_bytes=36754
```

这些日志证明 route 和 ClickHouse query 级 `elapsed_ms`、`query_id`、`response_bytes` 已可观测。

## Phase 5 Decision

Phase 5 不启动。

理由：

- p50 shell render 从 2759 ms 降到 725 ms，p95 从 2774 ms 降到 769 ms。
- 当前主要用户感知延时已由删除硬等待和接口收敛解决。
- `chart-context` 后台请求约 200-280 ms，且不再阻塞 Step 3 shell。
- 没有证据需要调整 ClickHouse mart 表结构、排序键或引入 preview cache；按计划不在无必要时做表结构优化。

## 已执行检查

Rust Rearview:

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：通过。`rearview-core` 114 个单测通过，workspace 单测和 doctest 通过。

Frontend:

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

结果：通过。Vitest 5 个测试文件、42 个测试通过。Build 通过；Vite 保留 chunk size warning，未阻塞构建。

Docs:

```bash
make docs-check
git diff --check
```

结果：通过。

CDP smoke:

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

结果：连接成功。

## 残余风险

- 通用 `/rearview/security-analysis` 和 `/rearview/strategy-preview/security-analysis` 仍保留兼容用途；本计划只移除 Step 3 主路径依赖。
- React dev `StrictMode` 下，快速 reload 或热更新可能出现已取消的 chart-context 请求在后端已开始执行部分查询；生产构建不启用 StrictMode 双挂载。本次用户感知链路和完成响应已收敛。
- `strategy-preview/open` 第一版服务端内部仍执行 timeline query 后再执行 latest preview query；本次只减少前端 round trip，不把两段 ClickHouse SQL 强行合并。

## Cleanup

- baseline 临时后端和前端已停止。
- 当前 `make racingline-app-dev` smoke 服务在报告采样后停止。
