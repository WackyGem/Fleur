# Racingline Strategy Step 3 Drift Remediation

日期：2026-06-22

范围：

- Debt 0004 的 Step 3 股池预览漂移修正。
- Racingline `/strategies` Step 3 职责收缩。
- Rearview preview-only timeline API。
- K 线复权、MA5/MA10/MA30、10 条分页和 Step1/Step2 展示语义拆分。

## 环境

服务：

- Rearview: `http://127.0.0.1:34057`
- Racingline new: `http://127.0.0.1:5174/strategies`
- Playwright CDP: `http://127.0.0.1:9222`

启动命令：

```bash
make racingline-dev

cd app/racingline_new
npm run dev -- --host 127.0.0.1 --port 5174
```

说明：`make racingline-dev` 启动的是 `app/racingline` 默认前端和 Rearview；本次目标页面在 `app/racingline_new`，所以另起 5174 端口验收。

浏览器连接：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## 实现摘要

Racingline:

- Step 3 移除开始日期、结束日期、展示行数输入。
- Step 3 移除“命中指标”“原始值”和权重滑杆。
- Step 3 表格分页固定 10 条。
- Step 3 横轴使用 timeline trade dates，不再依赖短 preview response。
- 表格“得分项”只映射 Step 2 `score_breakdown`。
- 表格“指标”只映射 Step 1 condition metric values。
- `security-analysis` 请求携带 `lookback_trading_days = 240` 和 `ma_windows = "5,10,30"`。
- K 线使用 Lightweight Charts candlestick series 和 MA line series。

Rearview:

- 新增 `POST /rearview/strategy-preview/timeline`。
- Timeline SQL 只返回 `trade_date` 和 `count() AS pool_count`。
- Timeline 不返回 signals、`score_breakdown`、`selected_metrics` 或 `raw_values`。
- Timeline range 上限支持近一年窗口。

## API Samples

`POST /rearview/strategy-preview/timeline` request:

```json
{
  "start_date": "2025-06-01",
  "end_date": "2026-06-01",
  "scoring": {"min": 0, "max": 100}
}
```

`POST /rearview/strategy-preview/timeline` response summary:

```json
{
  "start_date": "2025-06-01",
  "end_date": "2026-06-01",
  "dates": 242,
  "first": {"trade_date": "2025-06-03", "pool_count": 4943},
  "last": {"trade_date": "2026-06-01", "pool_count": 4959}
}
```

`POST /rearview/strategy-preview` request summary:

```json
{
  "start_date": "2026-06-01",
  "end_date": "2026-06-01",
  "preview_row_limit": 10,
  "scoring": {"min": 0, "max": 100}
}
```

`POST /rearview/strategy-preview/pool-page` request and response summary:

```json
{
  "request": {"trade_date": "2026-06-01", "limit": 10, "offset": 0, "sort": "score_desc"},
  "response": {"items": 10, "has_more": true, "first": {"security_code": "000001.SZ", "rank": 1, "score": 50.0}}
}
```

`POST /rearview/strategy-preview/security-analysis` request:

```json
{
  "trade_date": "2026-06-01",
  "security_code": "000001.SZ",
  "adjustment": "forward_adjusted",
  "lookback_trading_days": 240,
  "ma_windows": "5,10,30"
}
```

`security-analysis` response summary:

```json
{
  "source": "preview",
  "adjustment": "forward_adjusted",
  "chart_window": {"start_date": "2025-06-05", "end_date": "2026-06-01", "lookback_trading_days": 240},
  "chart_series": 240,
  "ma": {
    "requested_windows": [5, 10, 30],
    "available_windows": [5, 10, 30],
    "status": "available"
  }
}
```

复权切换验收：

```json
{
  "forward_adjusted_first_ohlc": {"open": 11.280144679788693, "high": 11.30862989362654, "low": 11.071253111644456, "close": 11.080748182923738},
  "unadjusted_first_ohlc": {"open": 11.88, "high": 11.91, "low": 11.66, "close": 11.67}
}
```

非前复权 MA metadata:

```json
{
  "adjustment": "unadjusted",
  "ma": {
    "available_windows": [],
    "adjustment": "forward_adjusted",
    "status": "forward_adjusted_only"
  }
}
```

## Browser Observations

正常路径：

- `/strategies` Step 1 从 `GET /rearview/metrics => 200` 加载指标 catalog。
- 指标类型和指标名显示中文，例如“行情与涨跌”“收盘价”。
- `POST /rearview/explain => 200` 校验规则成功。
- Step 2 需要至少一个评分权重；无权重时显示“至少需要一个评分权重”，不进入 mock success。
- 点击股池预览后出现真实请求：
  - `POST /rearview/strategy-preview/timeline => 200`
  - `POST /rearview/strategy-preview => 200`
  - `POST /rearview/strategy-preview/pool-page => 200`
  - `POST /rearview/strategy-preview/security-analysis => 200`
- Step 3 横轴显示 2025-06 到 2026-06 的交易日和股池数量。
- Step 3 表格显示 10 行，底部范围为 `1 - 10`。
- 表格“得分项”显示 Step 2 权重项，例如 `close_price >= 0 50.0`。
- 表格“指标”显示 Step 1 指标值，例如 `收盘价 10.99`。
- 右侧只展示行情和估值字段。
- 页面文本检查未发现 `命中指标`、`原始值`、`展示行数`、`开始日期`、`结束日期`。
- 复权切换触发新的 `security-analysis` 请求，前复权与除权 OHLC 不同。
- 非前复权时 MA5/MA10/MA30 按后端 metadata 禁用。
- 前复权下切换 MA 使图表 canvas 数据发生变化。

## Verification Commands

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

结果：通过。`npm run build` 仅保留既有 Vite chunk size warning。

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：通过。

```bash
make docs-check
git diff --check
```

结果：通过。

## Result

Debt 0004 的主要漂移已修正。Step 3 已恢复为股池预览结果检查页，使用 Rearview 真实接口展示近一年股池横轴、10 条分页、Step 2 得分项、Step 1 指标列和选中股票行情上下文。

保留限制：

1. Rearview 仍未返回 condition-level `condition_hits`；当前指标列展示 Step 1 筛选指标值，不宣称 boolean 命中。
2. `security-analysis` 响应仍保留 diagnostics 字段，但 Step 3 主 UI 不渲染这些字段。
