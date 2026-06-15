# Racingline Security Analysis Page

日期：2026-06-15

## 基本信息

```text
Frontend URL = http://127.0.0.1:5173
Rearview API base URL = http://127.0.0.1:34057
CDP endpoint = http://127.0.0.1:9222
Browser = Chrome/148.0.7778.178
Desktop viewport = 1440x1000
Mobile viewport = 390x844
```

样本：

```text
run_id = e5f59cae-69b4-4a4a-a102-86c77f25848e
trade_date = 2026-06-01
signal security = 002208.SZ
pool/security switch sample = 002272.SZ
```

## 启动和前置检查

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
```

结果摘要：

```text
GET http://127.0.0.1:34057/healthz -> 200
GET http://127.0.0.1:5173/ -> 200 text/html
run status = succeeded
run summary = day_count 1, pool_count 51, signal_count 5
```

Analysis API smoke：

```bash
curl -fsS "http://127.0.0.1:34057/rearview/runs/e5f59cae-69b4-4a4a-a102-86c77f25848e/securities/002208.SZ/analysis?trade_date=2026-06-01&source=signals&adjustment=forward_adjusted" | jq '{security_code, trade_date, adjustment, source, series_count:(.chart.series|length), quote_count:(.quote_rows|length), ma:.chart.ma}'
```

结果：

```text
security_code = 002208.SZ
trade_date = 2026-06-01
adjustment = forward_adjusted
source = signals
series_count = 240
quote_count = 240
MA requested/default/available = [5, 10, 30]
```

## 截图

| 文件 | 视口 | URL / 状态 | 结论 |
|---|---|---|---|
| [assets/2026-06-15-racingline-security-analysis-desktop.png](assets/2026-06-15-racingline-security-analysis-desktop.png) | desktop | `/runs/e5f59cae-69b4-4a4a-a102-86c77f25848e/securities/002208.SZ?trade_date=2026-06-01&source=signals&adjustment=forward_adjusted` | 三栏工作台可见，左侧结果列表、中间 K 线和右侧 mart 指标同时可扫描 |
| [assets/2026-06-15-racingline-security-analysis-mobile.png](assets/2026-06-15-racingline-security-analysis-mobile.png) | mobile | 同上 | 默认展示 Chart tab，没有三栏挤压或页面级横向溢出 |

## 浏览器验收

| 项 | 证据 | 结论 |
|---|---|---|
| signals `Open` | `/runs/:runId/securities/002208.SZ?trade_date=2026-06-01&source=signals&adjustment=forward_adjusted` 正常加载 | 通过 |
| pool `Open` | 从 `/runs/:runId?trade_date=2026-06-01&source=pool` 点击第一行 `Open`，进入 `source=pool` 分析页 | 通过 |
| 页面刷新恢复 | reload 后 `GET /runs/{run_id}`、`/days`、`/signals`、`/analysis` 均 200 | 通过 |
| 图表非空 | canvas 像素采样显示主图和指标 pane 均有绘制内容；主图 canvas 624x463，painted 481 | 通过 |
| 图表日期选择 | 点击 K 线后 `Selected day` 从 `2026-06-01` 变为 `2025-09-12`，URL 不改变 | 通过 |
| 价格口径切换 | UI 从 forward 切到 backward，URL 更新 `adjustment=backward_adjusted`，analysis API 200 | 通过 |
| 价格口径保留 selected day | 切换 backward 后 `Selected day` 保持 `2025-09-12` | 通过 |
| 非前复权 MA 状态 | backward adjusted 下 MA 控件禁用，并显示 `MA forward-adjusted only` | 通过 |
| 证券切换 | 从 `002208.SZ` 切到 `002272.SZ` 后 `Selected day` 回到 signal day `2026-06-01` | 通过 |
| MA 开关 | forward adjusted 下点击 MA10 后，MA10 关闭，MA5/MA30 保持开启 | 通过 |
| 右侧边界 | 右侧展示 `current mart query`，run snapshot 区块标记 `PostgreSQL run snapshot` | 通过 |
| 移动端默认视图 | 390x844 下 active tab 为 `Chart`，`body.scrollWidth=375`、`window.innerWidth=390` | 通过 |

## Network 和 Console

关键请求：

```text
GET /rearview/runs/{run_id}/signals?limit=50&offset=0&sort=rank_asc&trade_date=2026-06-01 -> 200
GET /rearview/runs/{run_id}/pool?limit=50&offset=0&sort=score_desc&trade_date=2026-06-01 -> 200
GET /rearview/runs/{run_id} -> 200
GET /rearview/runs/{run_id}/days -> 200
GET /rearview/runs/{run_id}/securities/002208.SZ/analysis?adjustment=forward_adjusted&source=signals&trade_date=2026-06-01 -> 200
GET /rearview/runs/{run_id}/securities/002208.SZ/analysis?adjustment=backward_adjusted&source=signals&trade_date=2026-06-01 -> 200
GET /rearview/runs/{run_id}/securities/002272.SZ/analysis?adjustment=backward_adjusted&source=signals&trade_date=2026-06-01 -> 200
GET /healthz -> 200
```

Console：

```text
Errors: 0
Warnings: 0
Info: React DevTools development hint only
```

## 工程门禁

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd app/racingline
npm run lint
npm run typecheck
npm run test
npm run build

cd ../..
make docs-check
git diff --check
```

结果：

```text
Rust fmt/clippy/test -> passed
Racingline lint/typecheck/test/build -> passed
docs-check and git diff --check -> passed
Vite build warning: client chunk is larger than 500 kB after minification
```

## 遗留问题

1. Vite production build 仍有 chunk size warning。当前不是功能阻塞；后续页面继续增长时优先做 route-level dynamic import。
2. 第一版仍只展示日线；分钟线、分时和任意指标公式不在 RFC 0020 范围内。

## 验收结论

通过。RFC 0020 的独立个股分析页、Rearview analysis API、K 线和指标展示、Signal day / Selected day 状态边界、价格口径切换、MA 开关、signals/pool 入口、桌面三栏和移动 Chart tab 均已完成并通过本地浏览器验收。
