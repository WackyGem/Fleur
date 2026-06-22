# Racingline Strategy Step 3 Drift2 Remediation

日期：2026-06-22

范围：

- Debt 0005 的 Step 3 股池预览二次漂移修正。
- 交易板块 `security_board` contract。
- K 线 MA 全复权模式可见、成交量柱和动态近一年窗口。
- Step 3 权重微调恢复。
- Preview security analysis payload 瘦身和查询并行化。

## 实现摘要

Racingline:

- Step 3 K 线标题和表格股票列显示 `security_code / boardLabel`，不再用 `exchange_code` 伪装板块。
- 新增 `security_board` 中文映射：沪市主板、深市主板、创业板、科创板。
- K 线图使用 `HistogramSeries` 渲染成交量柱。
- MA5/MA10/MA30 可用性由后端 `available_windows` 决定，不再绑定当前 OHLC 复权模式。
- `openPreview()` 先请求动态近一年 timeline，再使用 timeline 最新交易日请求单日 preview；timeline 为空时不空跑自然日。
- Step 3 右侧行情/估值下方恢复紧凑权重微调；修改权重只更新 draft 并标记 preview stale。
- Preview security analysis 请求传 `include_quote_rows=false`，并使用 React Query `staleTime`、`placeholderData` 和 fetch abort signal。

Rearview:

- `mart_stock_basic_snapshot` 透出 `security_board`。
- Preview rows、pool page 和 preview security analysis response 透出 `security_board`。
- `StrategyPreviewSecurityAnalysisRequest` 增加 `include_quote_rows`，默认兼容为 `true`。
- `include_quote_rows=false` 时 response 仍保留 `selected_quote` 和 `chart.series`，但省略完整 `quote_rows`。
- `ma_values()` 和 `price_overlay_values()` 固定返回前复权指标基准，不随当前 OHLC adjustment 清空。
- 确定 chart window 后并行查询 trend 和 momentum。

## API Contract

`POST /rearview/strategy-preview/security-analysis` request:

```json
{
  "trade_date": "2026-06-01",
  "security_code": "000001.SZ",
  "adjustment": "unadjusted",
  "lookback_trading_days": 240,
  "ma_windows": "5,10,30",
  "include_quote_rows": false
}
```

response shape:

```json
{
  "security_code": "000001.SZ",
  "security_board": "szse_main_board",
  "adjustment": "unadjusted",
  "chart": {
    "ma": {
      "available_windows": [5, 10, 30],
      "adjustment": "forward_adjusted",
      "basis_adjustment": "forward_adjusted",
      "status": "available"
    },
    "series": [{"trade_date": "2026-06-01", "volume": 123456}]
  },
  "selected_quote": {"security_code": "000001.SZ"},
  "quote_rows": []
}
```

说明：`quote_rows` 为空表示 Step 3 主 UI 请求了瘦身 payload；完整行情序列仍通过 `chart.series` 返回。

## 已执行检查

```bash
cd app/racingline_new
npm run typecheck
```

结果：通过。

```bash
cd app/racingline_new
npm run lint
npm test
npm run build
```

结果：通过。`npm run build` 仅保留 Vite chunk size warning。

```bash
cd engines
cargo test -p rearview-core
```

结果：通过，76 个测试通过。

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：通过。

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_basic_snapshot --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run python elt/scripts/validate_field_glossary.py
```

结果：通过。

## 浏览器验收

环境：

- Rearview: `http://127.0.0.1:34057`
- Racingline new: `http://127.0.0.1:5174/strategies`
- Playwright CDP: `http://127.0.0.1:9222`

步骤：

1. 进入 `/strategies`。
2. Step 1 创建默认筛选条件并执行规则校验。
3. Step 2 新增默认权重并点击「股池预览」。
4. Step 3 点击「更新股池」重新请求新版 Rearview。
5. 切换除权和后复权。
6. 修改 Step 3 权重得分，再点击「更新股池」。

观察：

- Step 1 指标类型和指标名显示中文，例如“行情与涨跌”“收盘价”。
- Step 3 标题和表格显示 `000001.SZ / 深市主板`，不再显示 `000001.SZ / SZ`。
- Timeline request 为 `start_date = 2025-06-22`、`end_date = 2026-06-22`，不再固定 `2025-06-01` / `2026-06-01`。
- 单日 preview request 使用 timeline 最新交易日 `2026-06-01`。
- `security-analysis` request 带 `include_quote_rows = false`、`lookback_trading_days = 240`、`ma_windows = "5,10,30"`。
- `security-analysis` response summary：`security_board = szse_main_board`、`quote_rows.length = 0`、`selected_quote = true`、`chart.series.length = 240`、`volume_points = 240`、`ma.available_windows = [5, 10, 30]`。
- response size 从 Plan 0048 验收记录的约 `578699` bytes 降到 `197840` bytes；除权状态下最新一次约 `193288` bytes。
- 前复权、除权、后复权三种模式下 MA5/MA10/MA30 控件都保持可用。
- Canvas 像素抽样非空：主 canvas `818x354`，非透明样本 `8269`，颜色样本 `622`。
- 修改 Step 3 权重得分 `50 -> 60` 后出现“股池预览已过期”，模拟建仓按钮禁用，当前表格仍显示 applied `50.0`。
- 点击「更新股池」后 timeline、preview、pool-page、security-analysis 都重新请求，表格得分项和得分更新为 `60.0`，模拟建仓按钮恢复可用。
- 快速切换/更新期间出现已过期 `security-analysis` 请求被 `net::ERR_ABORTED` 取消，未持续堆积。
- `playwright-cli console error` 返回 0 条 error。

```bash
make docs-check
git diff --check
```

结果：通过。

## 待补验收

无。

## 保留限制

1. Step 3 仍不持久化 preview result；刷新恢复和短期 preview cache 不在本次范围。
2. 权重微调第一版只提供紧凑得分调整；完整指标条件编辑仍在 Step 2 主页面完成。
3. 图表仍按响应变化重建 chart instance；如后续浏览器验收显示仍卡顿，再单独优化持久 chart instance。
