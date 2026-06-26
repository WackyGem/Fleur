# RFC 0030: Racingline Step 2 到 Step 3 预览链路瘦身与延时治理

状态：Proposed（当前分析记录，2026-06-25）
领域：racingline, rearview
关联系统：racingline, rearview, clickhouse marts
代码根：app/racingline/, engines/crates/rearview-core/, engines/crates/rearview-server/
系统地图：docs/systems/racingline.md, docs/systems/rearview.md

## 摘要

`/strategies` 从 Step 2「权重配置」点击「股池预览」到进入 Step 3 的当前路径，把一次用户动作扩展成多段串行和自动触发请求：

```text
openPreview()
  -> POST /rearview/strategy-preview/timeline
  -> POST /rearview/strategy-preview
  -> await POST /rearview/security-analysis
  -> setPreviewSnapshot()
  -> setActiveStep("preview")
  -> useStrategyBacktestValidateQuery 自动触发 POST /rearview/strategy-backtests/validate
```

其中真正阻塞 Step 3 展示的是前三段；第四段不阻塞 `setActiveStep("preview")`，但会在同一次交互后占用网络、React Query 状态和后端校验资源。第一性原理下，Step 3 首屏只需要回答两个问题：

1. 当前 Step 1/2 规则在近一年哪些交易日有候选池，候选池规模是多少。
2. 最新可展示交易日的前 N 个候选股、排名、分数和解释是什么。

个股 K 线分析和 Step 5 回测校验都不是进入 Step 3 的必要条件，应从首屏硬等待链路中移出。

## 当前事实

### 前端触发链路

| 动作 | 当前实现 | 用户路径角色 |
|---|---|---|
| 点击「股池预览」 | `app/racingline/src/routes/strategy-page.tsx` 的 `openPreview()` | Step 2 到 Step 3 唯一入口 |
| timeline 请求 | `previewTimelineMutation.mutateAsync()` | 串行第一段，返回近一年交易日和 `pool_count` |
| preview 请求 | `previewMutation.mutateAsync()` | 串行第二段，只请求 timeline 最后一个交易日 |
| 个股分析预取 | `await prefetchInitialSecurityAnalysis()` | 串行第三段；失败被吞掉，Step 3 会再由 query 重试 |
| Step 3 页面查询 | `StockPoolPreviewWorkbench` 的 `usePreviewSecurityAnalysisQuery()` 和必要时 `useStrategyPreviewPoolPageQuery()` | 进入 Step 3 后的首屏/交互查询 |
| 回测草稿校验 | `useStrategyBacktestValidateQuery(backtestValidateDraft.request)` | `previewSnapshot` 写入后自动触发，不属于 Step 3 展示必需 |

首屏通常不会立即触发 `pool-page`：latest trade date 的 page 0 已包含在 `strategy-preview` response 中，`StockPoolPreviewWorkbench` 的 `hasLocalPoolPage` 为 true 时会跳过 `pool-page`。用户切换日期、翻页或本地 page 0 不存在时才需要 `POST /rearview/strategy-preview/pool-page`。

### 后端接口职责

| 接口 | 当前职责 | 必要性判断 |
|---|---|---|
| `POST /rearview/strategy-preview/timeline` | 校验 `RuleVersionSpec`，编译 timeline SQL，扫描相关 marts，按 `trade_date` 聚合 `pool_count` | Step 3 横轴和最新可展示交易日需要，但不一定要单独成一次前端 round trip |
| `POST /rearview/strategy-preview` | 对指定日期/区间执行筛选、评分、rank、score breakdown、selected metrics、raw values，并查证券显示信息 | Step 3 latest page 0 需要，但只需要最新交易日前 N 行 |
| `POST /rearview/strategy-preview/pool-page` | 按交易日分页重跑同一规则并查证券显示信息 | 非首屏必需；只在日期切换、翻页或搜索时需要 |
| `POST /rearview/security-analysis` | 不校验 preview membership，查证券显示、交易日窗口、K 线、selected quote、trend 和 momentum 指标 | 当前 Step 3 只渲染 K 线、成交量和 MA5/MA10/MA30；momentum 返回不被页面消费，是瘦身候选 |
| `POST /rearview/strategy-preview/security-analysis` | preview 专用 analysis，先重跑 `compile_preview_pool_page(... security_code)` 做 membership 校验，再查分析数据 | 当前前端未使用；与通用 analysis 形成重复实现 |
| `POST /rearview/strategy-backtests/validate` | 只做 rule/config 校验、canonicalization、hash 和 summary；不访问 ClickHouse | Step 4/5 gate 需要，Step 3 首屏不需要 |

## 性能瓶颈拆解

### 1. 用户等待链路过长

`openPreview()` 现在等待 timeline、latest preview 和 security analysis prefetch 全部结束后才 `setActiveStep("preview")`。其中 security analysis 的异常被捕获并忽略，说明它不是进入 Step 3 的正确门禁。它仍然被 `await`，因此慢图表查询会直接拉长 Step 2 到 Step 3 的按钮等待时间。

### 2. 相同规则被重复校验、编译和执行

timeline、preview、pool-page、preview security analysis 都接收完整 `RuleVersionSpec`，后端每次重新 `rule.validate()`、构造 mart CTE、编译 SQL 并访问 ClickHouse。当前 `preview_id` 只在响应和前端 query key 中使用，后端不持久化 preview，也不能基于 `preview_id` 复用已计算的 timeline/latest rows。

### 3. timeline 与 latest preview 是两次网络往返

前端必须先等待 timeline 才知道最新有候选池的交易日，再发 latest preview。两者都是 Step 3 首屏需要的同一 preview execution 上下文，拆成两个前端 round trip 会放大延时和失败面。

### 4. analysis 查询天然重

`include_quote_rows=false` 已减少 payload，但通用 analysis 仍会执行：

```text
security display lookup
trade date lookback start
chart quote rows
selected quote row
trend indicator rows
momentum indicator rows
```

其中 chart quote 与 selected quote 并行，trend 与 momentum 并行，但 display 和 date-window 仍在前置链路中。当前 Step 3 `KLinePanel` 只渲染蜡烛图、成交量和 MA5/MA10/MA30，未渲染 KDJ/RSI；因此 analysis 中固定查询 `mart_stock_momentum_indicator_daily` 是后端历史 payload 对当前页面需求的污染，不应作为 Step 3 图表的真实数据依赖。该接口适合 Step 3 右侧图表懒加载，不适合作为 Step 3 页面进入门禁。

### 4.1 momentum mart 的真实依赖边界

`mart_stock_momentum_indicator_daily` 在 Step 3 有两种完全不同的触发来源：

1. 如果 Step 1/2 的规则实际选择了 momentum 指标，例如默认权重草稿里的 `kdj_j_value`，preview timeline / latest preview / pool-page SQL 会通过 metric catalog 依赖该 mart。这是规则执行所需的真实依赖。
2. 即使规则没有选择 momentum 指标，当前 analysis 后端也会固定查询 momentum rows 并填充 response 的 `kdj` / `rsi` 字段。但当前页面没有消费这些字段，这是冗余依赖。

### 4.2 当前 Step 3 analysis 的脏字段清单

当前 `StockPoolPreviewWorkbench` 实际消费的 analysis 字段只有：

1. 标题和副标题：`security_name`、`security_code`、`security_board`。
2. K 线图：`chart.series[].ohlc`、`chart.series[].volume`、`chart.series[].ma["5"|"10"|"30"]`，以及 `chart.ma.available_windows` 用于启用/禁用 MA toggle。
3. 右侧行情/估值：`selected_quote.open_price`、`high_price`、`low_price`、`close_price`、`prev_close_price`、`pct_change`、`pct_amplitude`、`volume`、`amount`、`limit_up_price`、`limit_down_price`、`a_market_cap`、`pe_ttm`、`roe`。

对当前页面没有消费价值的字段和查询包括：

| 类型 | 当前多余项 | 依据 |
|---|---|---|
| chart series 指标 | `kdj`、`rsi`、`macd`、`boll` | Step 3 只渲染蜡烛图、成交量和 MA 线 |
| chart metadata | `chart.price_overlays`、`chart.indicator_panels` | UI 不展示 overlay 列表或 indicator panel；`price_overlays` 仅作为 MA fallback，但 `ma` 已按请求窗口返回 |
| trend 查询列 | `price_ma_20`、`price_ma_60`、`price_ma_250`、`price_ema2_10`、`price_avg_ma_3_6_12_24`、`price_avg_ma_14_28_57_114`、MACD、BOLL | Step 3 analysis 当前只需要 MA5/MA10/MA30 |
| momentum 查询 | 整个 `query_analysis_momentum_rows()` | Step 3 analysis 当前不展示 KDJ/RSI |
| chart quote 查询列 | `kdj_rsv`、`kdj_k_value`、`kdj_d_value`、`kdj_j_value` | 只用于填充未展示的 KDJ fallback |
| selected quote 宽字段 | 复权 OHLC、复权因子、`prev_volume`、换手率、float/free 市值、股本、`pe_static`、`pe_forecast`、`pb_mrq`、`book_value_per_share`、`roa`、`roaa`、`roae`、股息率、停牌/ST、KDJ 字段等 | `KeyDataPanel` 只读取 14 个行情/估值字段 |
| response 元信息 | `sources`、`chart_window`、`adjustment`、`source`、空 `quote_rows` | 当前 Step 3 UI 不展示这些诊断元信息 |

因此，Step 3 analysis 的目标响应可以收敛为一个页面专用 view model，而不是复用历史个股分析页的宽响应。

### 5. 回测 validate 过早

`strategy-backtests/validate` 当前不访问 ClickHouse，通常不应是最大后端耗时来源。但它在用户刚进入 Step 3 时自动触发，语义上属于 Step 4/5 的回测草稿 gate。提前触发会让 Step 3 首屏同时承担 preview 和 backtest 两个阶段的后端工作，也让用户误以为 Step 3 还在等待回测准备。

### 6. 缺少可执行耗时基线

Rearview ClickHouse client 当前给每个查询设置 `query_id`，但 `execute_text()` 没有记录 HTTP elapsed、response bytes、row count 或 ClickHouse profile event。历史 smoke report 证明接口打通，但没有足够数据判断每个接口的 p50/p95 和 ClickHouse 扫描瓶颈。

## 减法方案

### Phase 0: 先建立最小观测

目标：先知道慢在哪里，再做数据库层优化。

1. 在 Rearview HTTP 层记录 route、status、elapsed_ms。
2. 在 ClickHouse `execute_text()` 记录 query_id、elapsed_ms、response_bytes；可选记录 parsed row count，由调用方补充。
3. 前端在 `openPreview()` 周围记录 timeline、preview、analysis prefetch、active step 切换的相对耗时，只在 dev 或 debug flag 下输出。
4. 对同一条代表规则连续跑 5 次，记录 p50/p95，并保留 query_id 便于查 ClickHouse `system.query_log`。

完成标准：

- 能区分慢在网络、ClickHouse、JSON 解析、React 渲染还是自动触发的后续 query。
- RFC 后续实现不再以“感觉慢”作为优化依据。

### Phase 1: 立即降低用户感知延时

目标：不改变后端结果语义，先把非首屏工作移出硬等待链路。

1. `prefetchInitialSecurityAnalysis()` 改为非阻塞后台预取。
   - `openPreview()` 在 latest preview 成功后立即 `setPreviewSnapshot()` 和 `setActiveStep("preview")`。
   - analysis query 由 Step 3 右侧区域显示局部 loading。
   - 背景 prefetch 使用同一个 query key，但不阻塞页面切换。
2. `strategy-backtests/validate` 延后到 Step 4 或 Step 5。
   - Step 3 不自动创建 backtest draft request。
   - 进入 Step 4 时可后台校验；点击进入 Step 5 或发起回测时必须同步 gate。
3. Step 3 首屏只依赖 timeline + latest preview。
   - timeline 失败：不进入 Step 3。
   - latest preview 失败：不进入 Step 3。
   - analysis 失败：留在 Step 3，图表区域展示错误和重试。

预期收益：

- 最坏情况下用户不再等待 K 线/指标分析接口。
- Step 3 和 Step 5 的后端工作解耦，界面状态更符合阶段语义。

### Phase 2: 收敛 preview 首屏接口

目标：减少 Step 2->3 的 round trip 和重复编译。

新增或改造一个首屏 preview endpoint：

```text
POST /rearview/strategy-preview/open
```

请求：

```json
{
  "rule": "...RuleVersionSpec...",
  "start_date": "YYYY-MM-DD",
  "end_date": "YYYY-MM-DD",
  "preview_row_limit": 10
}
```

响应：

```json
{
  "preview_id": "deterministic-or-ulid",
  "sql_hash": "...",
  "timeline": {
    "start_date": "YYYY-MM-DD",
    "end_date": "YYYY-MM-DD",
    "trade_dates": [{"trade_date": "YYYY-MM-DD", "pool_count": 123}]
  },
  "latest": {
    "trade_date": "YYYY-MM-DD",
    "pool_count": 123,
    "signals": []
  },
  "required_metrics": [],
  "required_marts": [],
  "required_columns": {}
}
```

实现选择：

1. 保守实现：服务端内部仍执行 timeline，再对 latest non-empty trade date 执行 preview。收益是减少前端 round trip 和集中错误处理；查询成本基本不变。
2. 进一步实现：抽取一次 `rule.validate()` 和 mart plan，timeline 与 latest preview 共享编译上下文，避免重复 catalog traversal 和 SQL planning。
3. 不建议第一版强行合成单条 ClickHouse SQL，因为 latest rank/score 需要 scoring、JSON breakdown 和 window rank；timeline 只需要 count。把两类不同成本的工作合成一条全区间 rank 查询，可能让原本轻的 timeline 变重。

完成标准：

- Step 2->3 首屏只需要一个 preview-open 请求。
- `pool-page` 仍只服务日期切换、翻页和搜索。
- 旧 `timeline`/`preview` 可以保留一段兼容期，但前端不再组合调用。

### Phase 3: 收敛 Step 3 analysis 到 chart context

目标：消除通用 analysis 与 preview analysis 的重复分叉，只保留 Step 3 当前页面需要的图表上下文。

当前选择：

1. 前端实际使用 `/rearview/security-analysis`，不做 preview membership 校验。
2. 后端还保留 `/rearview/strategy-preview/security-analysis`，会重跑 preview pool-page 做 membership 校验。

建议收敛为一个明确策略：

- Step 3 图表只服务“当前已展示的 preview row”，因此第一版使用页面专用 `chart-context` endpoint，不在图表查询里重跑完整筛选规则。
- `chart-context` 不接受完整 `RuleVersionSpec`，也不宣称自己能独立证明 membership；membership 来自 Step 3 列表中的已选 row 这一前端状态事实。
- 如果 API 将来必须独立校验 membership，需要先让 preview result 可被后端按 `preview_id` 复用。否则每次选股图表都重跑规则，会把安全性校验变成性能热点。

第一版建议：

1. 前端保持不阻塞 analysis。
2. 新增 `/rearview/strategy-preview/chart-context`，只接收图表和行情面板需要的参数。
3. 前端 Step 3 切到 `chart-context` 后，停止在 Step 3 使用 `/rearview/security-analysis`。
4. 暂不启用 `/rearview/strategy-preview/security-analysis`；若后续没有独立 membership 校验需求，则删除或下线该重复入口。

完成标准：

- Step 3 不再有两个功能相近的 analysis 调用路径。
- Step 3 analysis 不再要求前端重复发送完整 rule。
- API 名称和文档明确表达它是 chart context，不是 membership proof。

### Phase 4: 清理 Step 3 analysis 脏字段

目标：如果字段没有被当前页面使用，就不向后端请求；如果后端不需要返回，就不查询底层 mart。

执行原则：

1. 前端请求表达页面真实需求，不通过宽 analysis contract 隐式索取历史字段。
2. 后端为 Step 3 提供页面专用 view model，不复用历史个股分析页的全量响应。
3. 删除查询优先于只删除响应字段；否则 ClickHouse 成本仍然存在。
4. 删除字段前用 `rg` 和前端类型检查确认没有其他当前页面消费；历史文档和归档测试不作为保留依据。

`chart-context` 接口契约：

```text
POST /rearview/strategy-preview/chart-context
```

请求只保留当前页面需要的参数：

```json
{
  "trade_date": "YYYY-MM-DD",
  "security_code": "600000.SH",
  "adjustment": "forward_adjusted",
  "lookback_trading_days": 240,
  "ma_windows": "5,10,30"
}
```

响应只返回当前 Step 3 需要的字段：

```json
{
  "security_code": "600000.SH",
  "security_name": "...",
  "security_board": "...",
  "chart": {
    "series": [
      {
        "trade_date": "YYYY-MM-DD",
        "ohlc": {"open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0},
        "volume": 1000,
        "ma": {"5": 1.0, "10": 1.0, "30": 1.0}
      }
    ],
    "ma": {"available_windows": [5, 10, 30]}
  },
  "selected_quote": {
    "open_price": 1.0,
    "high_price": 1.0,
    "low_price": 1.0,
    "close_price": 1.0,
    "prev_close_price": 1.0,
    "pct_change": 0.0,
    "pct_amplitude": 0.0,
    "volume": 1000,
    "amount": 1000000,
    "limit_up_price": 1.1,
    "limit_down_price": 0.9,
    "a_market_cap": 100000000,
    "pe_ttm": 10.0,
    "roe": 0.1
  }
}
```

后端查询收敛：

| 查询 | 当前行为 | 清理后 |
|---|---|---|
| chart quote rows | 查 OHLC、volume 和 KDJ 字段 | 只查所选复权模式 OHLC 和 volume |
| selected quote row | 查 `quote_select_columns()` 宽字段 | 只查右侧面板 14 个字段 |
| trend rows | 查 MA5/10/20/30/60/250、EMA、组合均线、BOLL、MACD | 只查 MA5/10/30 |
| momentum rows | 固定查 RSI/KDJ | 删除查询 |
| response chart series | 返回 OHLC、volume、MA、price_overlays、KDJ、RSI、MACD、BOLL | 只返回 OHLC、volume、MA |
| response metadata | 返回 sources、chart_window、indicator_panels、price_overlays、空 quote_rows | 删除，除非当前页面明确展示 |

实施步骤：

1. 在前端定义 Step 3 专用 `PreviewChartContextResponse` 类型，替代复用 `SecurityAnalysisResponse`。
2. 新增后端轻量 response struct 和查询 helper，不改旧 analysis endpoint。
3. 前端 Step 3 切到轻量 endpoint，保持 UI 表现不变。
4. 给旧 `SecurityAnalysisResponse` 的 Step 3 使用路径加删除标记；确认没有当前页面依赖后，再移除前端 wrapper 中的 Step 3 调用。
5. 补测试：前端断言 Step 3 图表不读取 `kdj/rsi/macd/boll/price_overlays`；后端测试断言轻量 endpoint 不调用 momentum 查询，并且 SQL select 不包含 MACD/BOLL/KDJ 宽字段。

完成标准：

- Step 3 analysis 网络响应中不再出现 `kdj`、`rsi`、`macd`、`boll`、`price_overlays`、`indicator_panels`、`sources` 和空 `quote_rows`。
- Step 3 analysis 后端不再调用 `query_analysis_momentum_rows()`。
- Step 3 trend 查询只选择 MA5/MA10/MA30。
- Step 3 selected quote 查询只选择右侧面板实际展示字段。
- Step 3 UI 截图和交互不变，接口 payload 和 ClickHouse 查询列明显变少。

### Phase 5: 基于观测再做 ClickHouse 查询优化

目标：在 Phase 0 有耗时基线、Phase 4 已删除无用字段后，再优化真实瓶颈。

候选方向：

1. 对 preview 规则引入短 TTL 结果缓存，key 使用 rule hash、range、row limit、offset、selected date 和 schema/catalog hash。
2. 对 security display 查询做进程内短 TTL cache，减少同一批 `security_code` 重复查 `mart_stock_basic_snapshot`。
3. 如果 `chart-context` 收敛后仍慢，再评估 analysis 的 date-window 和 trend 查询是否需要专用 chart mart 或排序键调整。Rearview 系统地图已记录：`mart_stock_trend_indicator` 当前排序键以 `trade_date` 优先，个股 analysis 若变慢再评估专用 chart mart。
4. 对 preview timeline 和 preview latest 分别记录 ClickHouse scanned rows/bytes，再决定是否需要面向选股的宽表 `mart_stock_rearview_metric_daily`。

完成标准：

- 每个优化项都有 Phase 0 baseline 和优化后对比数据。
- 不再把“删无用字段”和“改表结构”混在同一阶段决策。

不建议：

- 不先测量就改 ClickHouse 表结构。
- 不把 Step 3 preview 持久化成正式 run；Step 3 仍是 preview-only，不创建 rule set、rule version、run、portfolio run 或 backtest result。

## 目标调用图

短期目标：

```text
openPreview()
  -> POST /rearview/strategy-preview/timeline
  -> POST /rearview/strategy-preview
  -> setPreviewSnapshot()
  -> setActiveStep("preview")

Step 3 right panel
  -> POST /rearview/security-analysis
     (background, non-blocking, temporary before chart-context lands)

Step 4/5 gate
  -> POST /rearview/strategy-backtests/validate
```

中期目标：

```text
openPreview()
  -> POST /rearview/strategy-preview/open
  -> setPreviewSnapshot()
  -> setActiveStep("preview")

Step 3 local interactions
  -> POST /rearview/strategy-preview/pool-page
  -> POST /rearview/strategy-preview/chart-context

Step 4/5 gate
  -> POST /rearview/strategy-backtests/validate
```

## 验收指标

必须先建立 baseline，再以同一条代表规则对比：

| 指标 | 目标 |
|---|---|
| Step 2 click 到 Step 3 shell render | 不等待 analysis 和 backtest validate |
| 首屏 preview hard-blocking 请求数 | 短期 2 个；中期 1 个 |
| Step 3 图表加载失败影响 | 只影响图表区域，不阻塞股池列表 |
| 后端重复 rule payload | 中期首屏只发送一次 |
| 回测 validate 触发时机 | Step 4/5 gate，不在 Step 3 首屏自动触发 |
| 接口耗时可观测性 | 每个 Rearview route 和 ClickHouse query_id 有 elapsed_ms |

## 非目标

1. 本 RFC 不修改代码。
2. 不改变 Step 3 preview-only 边界。
3. 不把 Step 3 preview response 作为 Step 5 回测数据源；Step 5 仍按 applied rule 和 execution config 重新执行。
4. 不在未测量前调整 ClickHouse 表结构或 dbt mart 物化策略。
5. 不新增鉴权、用户隔离或多租户 preview cache 语义。

## 关联代码与文档

| 资源 | 用途 |
|---|---|
| `app/racingline/src/routes/strategy-page.tsx` | `openPreview()`、analysis prefetch、backtest validate 自动 query |
| `app/racingline/src/features/strategy/components/stock-pool-preview-workbench.tsx` | Step 3 pool-page 和 analysis query 触发点 |
| `app/racingline/src/api/rearview.ts` | 前端 Rearview endpoint wrapper |
| `engines/crates/rearview-core/src/api/mod.rs` | Rearview preview、analysis、backtest validate routes |
| `engines/crates/rearview-core/src/planner/sql.rs` | preview timeline、preview rows、pool-page SQL 编译 |
| `engines/crates/rearview-core/src/clickhouse/mod.rs` | ClickHouse HTTP query execution 与 analysis query helpers |
| `docs/RFC/archive/0026-racingline-strategy-pool-preview-step3.md` | Step 3 原始边界和历史实现方案 |
| `docs/systems/racingline.md` | 当前 Racingline 系统事实 |
| `docs/systems/rearview.md` | 当前 Rearview 系统事实 |

## 后续计划建议

1. 新增一个 active plan，按 Phase 0 -> Phase 1 -> Phase 2 -> Phase 3/4 -> Phase 5 拆实施任务。
2. Phase 0 完成后，把 baseline 写入 `docs/jobs/reports/`。
3. Phase 1 完成后，补前端测试覆盖：`openPreview()` 不等待 analysis；Step 3 可以在 analysis pending/error 时展示列表。
4. Phase 2 完成后，再决定是否归档旧 `timeline` + `preview` 组合调用路径。
5. Phase 3/4 完成后，补前后端测试证明 Step 3 不再请求或返回 KDJ/RSI/MACD/BOLL 等脏字段。
6. Phase 5 只在 baseline 证明仍有 ClickHouse 瓶颈时启动，避免先改表结构再找问题。
