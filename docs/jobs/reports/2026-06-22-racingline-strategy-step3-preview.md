# Racingline Strategy Step 3 Preview Implementation

日期：2026-06-22

范围：

- Racingline `/strategies` Step 1/2/3 真实接口闭环。
- Rearview preview-only API：`POST /rearview/strategy-preview`、`POST /rearview/strategy-preview/pool-page`、`POST /rearview/strategy-preview/security-analysis`。
- 新增证券显示 mart：`fleur_marts.mart_stock_basic_snapshot`。

## 环境

服务：

- Rearview: `http://127.0.0.1:34057`
- Racingline normal smoke: `http://127.0.0.1:5174/strategies`
- Racingline negative smoke: `http://127.0.0.1:5175/strategies`
- Playwright CDP: `http://127.0.0.1:9222`

启动命令：

```bash
cd engines
cargo run -p rearview-server -- serve

cd app/racingline_new
npm run dev -- --host 127.0.0.1 --port 5174
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:9 npm run dev -- --host 127.0.0.1 --port 5175
```

浏览器连接：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## API Samples

`GET /rearview/metrics`：

```json
{"metric_count":81,"allow_scoring_count":51}
```

`POST /rearview/strategy-preview`，`preview_row_limit = 3`：

```json
{
  "preview_row_limit": 3,
  "dates": [
    {"trade_date": "2026-05-27", "pool_count": 4955, "first": {"security_code": "000001.SZ", "security_name": "平安银行", "exchange_code": "SZ", "score": 50, "signal_rank": 1}},
    {"trade_date": "2026-06-01", "pool_count": 4959, "first": {"security_code": "000001.SZ", "security_name": "平安银行", "exchange_code": "SZ", "score": 50, "signal_rank": 1}}
  ]
}
```

`POST /rearview/strategy-preview/pool-page`：

```json
{"trade_date":"2026-06-01","pool_count":4959,"limit":2,"offset":0,"has_more":true,"first":{"security_code":"000001.SZ","security_name":"平安银行","exchange_code":"SZ","score":50,"signal_rank":1}}
```

`POST /rearview/strategy-preview/security-analysis`：

```json
{"source":"preview","trade_date":"2026-06-01","security_code":"000001.SZ","security_name":"平安银行","chart_points":60,"quote_rows":60,"selected_quote":{"close_price":10.99,"pct_change":0.5489478499542589,"a_market_cap":213271040996.02}}
```

## Browser Observations

Normal smoke at `5174`:

- Step 1 loaded metrics from `GET http://127.0.0.1:34057/rearview/metrics => 200`.
- Step 1 rule validation called `POST /rearview/explain => 200`.
- Step 2 scoring options were available from the same real metric catalog; 51 metrics had `allow_scoring = true`.
- Step 3 preview called `POST /rearview/strategy-preview => 200`.
- Step 3 full-pool table called `POST /rearview/strategy-preview/pool-page => 200`.
- Selecting the first row loaded `POST /rearview/strategy-preview/security-analysis => 200`.
- Page displayed `平安银行 / 000001.SZ / SZ`, candidate pool counts, score, score breakdown, selected metrics, raw values, quote fields and chart context from Rearview.
- `rg` over the browser snapshot found no `行业`、`板块`、`industry`、`sector`、`security_board` display text.
- Changing the preview start date from `2026-05-26` to `2026-05-27` showed `股池预览已过期`, kept the previous applied rows visible, and disabled `模拟建仓`.
- Clicking `更新股池` cleared the stale state; latest request tail was:

```text
POST /rearview/strategy-preview => 200
POST /rearview/strategy-preview/pool-page => 200
POST /rearview/strategy-preview/security-analysis => 200
```

Negative smoke at `5175`:

- Frontend was started with `VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:9`.
- Browser requests showed `GET http://127.0.0.1:9/rearview/metrics => FAILED`.
- Page displayed `指标加载失败 / Failed to fetch`.
- `校验规则` and `配置权重` were disabled.
- No Step 1/2/3 mock success state appeared.

## Fixes During Smoke

Live smoke exposed ClickHouse mart field drift in the preview security analysis query:

- `mart_stock_quotes_daily` uses `turnover_rate_pct`, `turnover_rate_free_float_pct`, `amplitude_pct`, `change_pct`, `market_cap`, `float_market_cap`, `free_float_market_cap`, `shares`, `float_shares_a`, `free_float_shares`, `dy_static_pct`, and `dy_ttm_pct`.
- `mart_stock_trend_indicator_daily` uses `boll_upper_20_2` and `boll_lower_20_2`.

Rearview now aliases those current mart fields back to the existing analysis response contract. The API contract exposed to Racingline remains unchanged.

## Verification Commands

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build

cd ../../engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage

cd ../pipeline
uv run dbt parse --project-dir elt --profiles-dir elt --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_basic_snapshot --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run python elt/scripts/validate_field_glossary.py

make docs-check
git diff --check
```

## Result

Step 1/2/3 now have a verified real-interface success path. Step 3 uses an applied preview snapshot, supports stale handling, full pool pagination, security display fields, selected metric/raw value explanation and preview-only security analysis. Rearview failure no longer falls back to mock success data.

Remaining limitation: preview pagination and security analysis are stateless and resend the applied `RuleVersionSpec`; short-lived preview cache remains a separate future RFC if response time or payload size becomes an issue.
