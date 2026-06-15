# RFC 0020: Racingline Run Result 个股分析页

状态：Proposed（2026-06-14）

## 摘要

本文档定义 Racingline 在 run 结果页中点击 `Open` 后进入的个股分析页。当前第一版实现中，`/runs/:runId` 的买入信号 `Open` 打开的是右侧详情抽屉，只展示 PostgreSQL run snapshot 中保存的 `score_breakdown` 和 `selected_metrics`。本 RFC 将该交互升级为独立页面：左侧保留当前 run result 个股列表，中间展示支持前复权、后复权和不复权切换的个股日 K 线图，主图叠加可开关 MA，图表下方展示 KDJ、RSI、MACD 和 BOLL 指标面板；右侧展示 `fleur_marts.mart_stock_quotes_daily` 中该证券、该日期的行情和指标字段。

新页面仍由 Racingline 承担 UI；Rearview 负责提供 UI 友好的 HTTP API，并作为前端访问 ClickHouse mart 的唯一入口。前端不得直接连接 ClickHouse 或 PostgreSQL，不在浏览器内重算技术指标。

关联文档：

- [RFC 0018: Rust Rearview 规则选股服务与 mart 指标库](0018-rust-stock-screening-service.md)
- [RFC 0019: Racingline Rearview 前端工作台](0019-racingline-rearview-frontend-workbench.md)
- [System: Racingline](../systems/racingline.md)
- [System: Rearview](../systems/rearview.md)

## 背景

第一版结果页已经支持按交易日查看 `buy_signal` 和 `pool_member`，并把 `selected_metrics` 作为运行时快照展示。这个视图能解释“为什么该证券在某个 run 中入选”，但不能高效回答后续分析问题：

1. 该证券在信号日之前的价格结构是什么样。
2. 信号日对应的成交量、换手率、涨跌幅、市值、估值、ST/停牌和 KDJ 等 mart 字段是多少。
3. 在同一个交易日的结果列表中，如何快速切换证券并保持图表和指标联动。

`mart_stock_quotes_daily` 当前是 `fleur_marts` 中的日频行情 mart，粒度是每证券、交易日一行，包含未复权 OHLC、前复权/后复权 OHLC、成交量、金额、换手率、涨跌幅、市值、估值、股息率、ST/停牌和 KDJ 字段。其 ClickHouse 表排序为 `(security_code, trade_date)`，适合单证券日期区间 K 线查询。

图表增强需要额外读取或补齐 mart 层指标：

- `mart_stock_trend_indicator` 当前提供 MA、BOLL 和 MACD，指标输入以 `close_price_forward_adj` 为主。
- `mart_stock_momentum_indicator` 当前提供 RSI 和 KDJ，指标输入以复权价格口径为主。
- `mart_stock_quotes_daily` 已透传 `int_stock_quotes_daily_adj` 的前复权和后复权 OHLC、复权前收和复权因子；Rearview 不需要绕过 mart 读取 intermediate。

## 目标

1. 将 `/runs/:runId` 结果表中的 `Open` 改为进入可分享、可刷新、可回退的独立个股分析页面。
2. 页面左侧展示当前 run、当前交易日、当前结果来源下的个股列表，并高亮当前证券。
3. 页面中间展示当前证券的日 K 线图，支持 `forward_adjusted`、`backward_adjusted`、`unadjusted` 三种价格口径切换。
4. K 线主图叠加 MA 线并提供开关，默认展示 MA5、MA10、MA30。
5. K 线图下方展示 KDJ、RSI、MACD 和 BOLL 指标面板，所有指标来自 Furnace/dbt 已物化字段，不由前端重算。
6. 页面右侧展示 `mart_stock_quotes_daily` 中当前证券、当前选中日期的字段，按行情、流动性、市值股本、估值、状态和 KDJ 分组。
7. 保持 run 结果快照和当前 mart 查询值的边界清晰：`score_breakdown` / `selected_metrics` 是 PostgreSQL run snapshot；K 线、技术指标面板和右侧指标是当前 ClickHouse mart 查询值。
8. 支持桌面三栏工作流和移动端可用的分栏降级。

## 非目标

1. 不实现交易、下单、风控、组合调仓或完整回测。
2. 不在前端直接访问 ClickHouse 或 PostgreSQL。
3. 不把 K 线和 `mart_stock_quotes_daily` 当前查询值写回 PostgreSQL run snapshot。
4. 不在前端重算 KDJ、MA、RSI、MACD、BOLL 或价格行为结构指标；这些仍由 Furnace/dbt 维护并通过 mart/API 暴露。
5. 不展示分钟线或分时图；第一版只展示日线。
6. 不要求新增 PostgreSQL 表；第一版可以通过 Rearview 查询 PostgreSQL run result 和 ClickHouse mart 组合响应。
7. 不把当前 mart 值伪装成运行当时的历史快照。
8. 不支持任意自定义指标公式或任意 MA 窗口编辑；第一版只要求 MA5、MA10、MA30 的显示开关。

## 当前事实

当前 Racingline 结果页：

- `/runs/:runId` 按交易日展示 `Buy signals` 和 `Pool` tabs。
- `Buy signals` 的 `Open` 打开 `SignalDetailSheet`，展示 `BuySignalRecord.score_breakdown` 和 `selected_metrics`。
- `Pool` 表当前没有独立详情入口。
- 前端已经引入 TradingView Lightweight Charts，可复用为日 K 线渲染基础。

当前 Rearview API：

- 已有 `GET /rearview/runs/{run_id}/signals` 和 `GET /rearview/runs/{run_id}/pool`，支持 `trade_date`、分页、证券代码过滤和排序。
- 尚无面向单证券 K 线和 `mart_stock_quotes_daily` 字段的 HTTP API。
- Rearview 已经持有 ClickHouse client 和 mart database 配置，适合作为前端读取 mart 的服务边界。

当前 `mart_stock_quotes_daily` 字段包括：

- 主键：`security_code`、`trade_date`
- K 线：`open_price`、`high_price`、`low_price`、`close_price`
- 前值和成交：`prev_close_price`、`prev_close_price_unadj`、`prev_volume`、`volume`、`amount`
- 复权价格：`open_price_forward_adj`、`high_price_forward_adj`、`low_price_forward_adj`、`close_price_forward_adj`、`prev_close_price_forward_adj`、`open_price_backward_adj`、`high_price_backward_adj`、`low_price_backward_adj`、`close_price_backward_adj`、`prev_close_price_backward_adj`、`forward_adjustment_factor`、`forward_adjustment_ratio`、`backward_adjustment_factor`、`backward_adjustment_ratio`
- 交易指标：`turnover_rate`、`turnover_rate_actual`、`pct_amplitude`、`pct_change`
- 涨跌停：`limit_up_price`、`limit_down_price`
- 市值股本：`a_market_cap`、`a_float_market_cap`、`a_free_float_market_cap`、`a_shares`、`a_float_shares`、`a_free_float_shares`
- 估值：`pe_static`、`pe_ttm`、`pe_forecast`、`pb_mrq`、`book_value_per_share`、`roe`、`roa`、`roaa`、`roae`
- 股息和状态：`dy_static`、`dy_ttm`、`is_suspend`、`is_st`
- KDJ：`kdj_rsv`、`kdj_k_value`、`kdj_d_value`、`kdj_j_value`

当前图表相关 mart 字段事实：

- `mart_stock_quotes_daily` 输出未复权 `open_price`、`high_price`、`low_price`、`close_price`，以及前复权和后复权 OHLC。
- `mart_stock_trend_indicator` 输出 `price_ma_5`、`price_ma_10`、`price_ma_20`、`price_ma_30`、`price_ma_60` 等 MA 字段，以及 `boll_mid_20_2`、`boll_up_20_2`、`boll_dn_20_2`、`macd_dif`、`macd_dea`、`macd_histogram` 等字段。
- `mart_stock_momentum_indicator` 输出 `rsi_6`、`rsi_12`、`rsi_14`、`rsi_24`、`rsi_25`、`rsi_50` 和 KDJ 字段。
- `int_stock_quotes_daily_adj` 输出 `open_price_forward_adj`、`high_price_forward_adj`、`low_price_forward_adj`、`close_price_forward_adj`、`open_price_backward_adj`、`high_price_backward_adj`、`low_price_backward_adj`、`close_price_backward_adj` 等复权 OHLC，但它不是 Rearview 应直接读取的 mart 边界。

本 RFC 的数据平台前置项已由 Plan 0038 完成：

1. 在 mart 层提供图表可消费的前复权和后复权 OHLC，字段命名必须显式包含 `forward_adj` 或 `backward_adj`。
2. 在 Furnace/dbt MA 输出和 `mart_stock_trend_indicator` 中补齐 `price_ma_30`；不得用 `price_ma_28` 替代 MA30。

## 页面入口与路由

新增路由：

```text
/runs/:runId/securities/:securityCode?trade_date=YYYY-MM-DD&source=signals|pool&adjustment=forward_adjusted|backward_adjusted|unadjusted
```

参数语义：

| 参数 | 必填 | 说明 |
|---|---|---|
| `runId` | 是 | 当前 Rearview run id |
| `securityCode` | 是 | 当前证券代码 |
| `trade_date` | 是 | 当前 run result 交易日，也是默认 K 线锚点日期 |
| `source` | 是 | `signals` 表示来自 TopN 买入信号；`pool` 表示来自完整股票池 |
| `adjustment` | 否 | K 线价格口径，默认 `forward_adjusted`；可选 `forward_adjusted`、`backward_adjusted`、`unadjusted` |

从 `/runs/:runId` 的 `Buy signals` tab 点击 `Open` 时，进入：

```text
/runs/:runId/securities/:securityCode?trade_date=:tradeDate&source=signals&adjustment=forward_adjusted
```

从 `Pool` tab 点击 `Open` 时，进入：

```text
/runs/:runId/securities/:securityCode?trade_date=:tradeDate&source=pool&adjustment=forward_adjusted
```

如果用户直接访问缺少 `trade_date` 或 `source` 的 URL，前端应优先回到 run detail 的默认交易日和默认结果来源，再跳转到规范 URL。缺少 `adjustment` 时使用 `forward_adjusted`。第一版不依赖浏览器内存状态，页面刷新后必须能通过 URL 和后端 API 恢复。

## 页面布局

### 桌面布局

桌面采用三栏工作台布局：

```text
┌────────────────┬──────────────────────────────┬────────────────────┐
│ Result list     │ Daily K-line chart            │ mart indicators    │
│                │                              │                    │
│ run/trade date │ candlestick + MA + volume     │ selected date row  │
│ rows           │ KDJ/RSI/MACD/BOLL panels      │ grouped fields     │
└────────────────┴──────────────────────────────┴────────────────────┘
```

建议宽度：

- 左侧结果列表：`18rem` 到 `22rem`，适合扫描 rank、security_code、score。
- 中间图表：占剩余主要空间，优先保证 K 线可读。
- 右侧指标：`22rem` 到 `26rem`，按字段组纵向滚动。

### 移动布局

移动端不强行三栏挤压，使用 tabs 或分段视图：

- `Results`
- `Chart`
- `Indicators`

页面仍保留同一路由。切换证券或图表日期时，三个视图共享同一个 selected security 和 selected quote date。

## 左侧结果列表

左侧列表表示当前 run、当前交易日、当前结果来源的证券集合。

`source=signals` 时，列表来自 `buy_signal`，默认按 `rank_asc` 排序。

字段：

| 字段 | 来源 | 展示 |
|---|---|---|
| `rank` | `buy_signal.rank` | 排名，固定宽度 |
| `security_code` | `buy_signal.security_code` | 主标识 |
| `score` | `buy_signal.score` | 保留合理小数 |
| `selected_metrics` 摘要 | `buy_signal.selected_metrics` | 只展示 1 到 3 个关键字段，完整内容不在左栏展开 |

`source=pool` 时，列表来自 `pool_member`，默认按 `score_desc` 排序。

字段：

| 字段 | 来源 | 展示 |
|---|---|---|
| `security_code` | `pool_member.security_code` | 主标识 |
| `score` | `pool_member.score` | 可为空 |
| `signal_rank` | `pool_member.signal_rank` | 入选 TopN 时展示 |
| `selected_metrics` 摘要 | `pool_member.selected_metrics` | 只展示 1 到 3 个关键字段 |

交互：

1. 当前证券高亮。
2. 点击其他证券时，仅替换 `:securityCode` 并保留 `runId`、`trade_date` 和 `source`。
3. 列表支持分页或无限滚动；第一版可沿用现有 `limit` / `offset`。
4. 列表为空时，页面保留图表和指标区域的空状态，不自动切换来源。

## 中间图表区

图表区通过 Rearview API 读取 ClickHouse mart 数据。主 K 线使用用户选择的价格口径，默认 `forward_adjusted`，因为当前 MA、BOLL、MACD、RSI 和 KDJ 指标主要基于前复权输入。`unadjusted` 用于检查原始行情口径，`backward_adjusted` 用于长期历史尺度观察。

默认窗口：

- 锚点日期：URL 中的 `trade_date`。
- 默认结束日期：`trade_date`，避免默认展示信号日之后的数据。
- 默认回看：240 个交易行，或后端可用数据不足时返回更短窗口。

### 主 K 线和成交量

| 图层 | 字段 |
|---|---|
| 前复权 K 线 | mart 层 `open_price_forward_adj`、`high_price_forward_adj`、`low_price_forward_adj`、`close_price_forward_adj` |
| 后复权 K 线 | mart 层 `open_price_backward_adj`、`high_price_backward_adj`、`low_price_backward_adj`、`close_price_backward_adj` |
| 不复权 K 线 | `mart_stock_quotes_daily.open_price`、`high_price`、`low_price`、`close_price` |
| 成交量 | `mart_stock_quotes_daily.volume` |
| 结果日期标记 | URL `trade_date` |
| 选中日期标记 | 用户在图表 crosshair 或点击选择的 `trade_date` |

行为要求：

1. 图表按 `trade_date` 升序渲染。
2. OHLC 任一关键字段为空时，该日不渲染 K 线实体；成交量可以独立显示或留空。
3. 默认选中日期为 URL `trade_date`。
4. 用户切换 `adjustment` 时，主 K 线 OHLC 必须切换到对应价格口径，并保持同一证券、同一日期窗口和同一选中日期。
5. 用户在图表上选择其他日期时，右侧指标面板切换到该日期的 `mart_stock_quotes_daily` 行，图表下方指标面板也同步 crosshair。
6. 如果用户主动扩展到信号日之后的日期，UI 必须标记这些值是“当前 mart 查询值”，不是 run 发起时已知信息。

### MA 叠加

主 K 线默认叠加 MA5、MA10、MA30，并在图表工具栏提供三个独立开关。开关只控制显示，不改变后端计算口径。

| MA | 字段要求 | 默认 |
|---|---|---|
| MA5 | `price_ma_5` | 开 |
| MA10 | `price_ma_10` | 开 |
| MA30 | `price_ma_30` | 开 |

约束：

1. MA 线应与当前 K 线的价格口径一致；不得在未标记的情况下把前复权 MA 叠加到不复权或后复权 K 线上。
2. 当前 `mart_stock_trend_indicator` 已有 `price_ma_5` 和 `price_ma_10`，但没有 `price_ma_30`；实现前必须补齐 MA30。
3. 如果第一阶段只具备前复权 MA，则 MA 开关仅在 `adjustment=forward_adjusted` 时可用；切换到其他价格口径时应禁用或隐藏 MA 叠加，并说明该口径暂无同口径 MA 数据。
4. 不允许用 `price_ma_28` 或其他近似窗口替代 MA30。

### 下方技术指标面板

K 线下方按共享时间轴展示四个指标面板。第一版不在前端重算指标，只渲染 API 返回的 mart 字段。

| 面板 | 默认字段 | 来源 |
|---|---|---|
| KDJ | `kdj_k_value`、`kdj_d_value`、`kdj_j_value`，可辅助展示 `kdj_rsv` | `mart_stock_momentum_indicator`；右侧 quote 面板也可展示 `mart_stock_quotes_daily` 中的 KDJ 当前行 |
| RSI | `rsi_6`、`rsi_12`、`rsi_24` | `mart_stock_momentum_indicator` |
| MACD | `macd_dif`、`macd_dea`、`macd_histogram` | `mart_stock_trend_indicator` |
| BOLL | `boll_mid_20_2`、`boll_up_20_2`、`boll_dn_20_2` | `mart_stock_trend_indicator` |

显示规则：

1. 四个面板位于主 K 线和成交量下方，和主图共享横向时间轴。
2. BOLL 是价格带指标，第一版按用户要求放在下方独立面板，使用独立纵轴；不默认叠加到主 K 线。
3. 指标字段为 `NULL` 时对应点或柱留空，不填 0，不前向填充。
4. API 响应必须携带指标来源表和价格口径 metadata。当前趋势/动量指标以 `forward_adjusted` 口径为主；如果用户切换到其他 K 线口径，而同口径指标尚未物化，UI 必须明确标记指标口径，不能暗示这些指标随 K 线口径重算。

## 右侧 mart 指标面板

右侧展示当前证券、当前选中图表日期在 `mart_stock_quotes_daily` 中的一行字段。默认日期等于 URL `trade_date`。

面板头部：

- `security_code`
- 选中 `trade_date`
- 数据来源：`fleur_marts.mart_stock_quotes_daily`
- 明确标签：`当前 mart 查询值`

字段分组：

| 分组 | 字段 |
|---|---|
| OHLC | `open_price`、`high_price`、`low_price`、`close_price`、`prev_close_price`、`prev_close_price_unadj` |
| 成交与交易 | `volume`、`prev_volume`、`amount`、`turnover_rate`、`turnover_rate_actual`、`pct_amplitude`、`pct_change` |
| 涨跌停 | `limit_up_price`、`limit_down_price` |
| 市值股本 | `a_market_cap`、`a_float_market_cap`、`a_free_float_market_cap`、`a_shares`、`a_float_shares`、`a_free_float_shares` |
| 估值 | `pe_static`、`pe_ttm`、`pe_forecast`、`pb_mrq`、`book_value_per_share`、`roe`、`roa`、`roaa`、`roae` |
| 股息和状态 | `dy_static`、`dy_ttm`、`is_suspend`、`is_st` |
| KDJ | `kdj_rsv`、`kdj_k_value`、`kdj_d_value`、`kdj_j_value` |

面板还应保留一个 run snapshot 区块，用于展示当前证券在该 run result 中的 `score`、`rank` / `signal_rank`、`score_breakdown` 和 `selected_metrics`。该区块必须标记为 `PostgreSQL run snapshot`，并与 `当前 mart 查询值` 视觉上分开。

右侧面板不替代下方技术指标面板。右侧只展示当前选中日期的一行 quote mart 字段；KDJ、RSI、MACD 和 BOLL 的历史序列仍在中间图表区渲染。

## 数据边界

本页面有两类事实源：run snapshot 和当前 mart 查询值。当前 mart 查询值又分为行情、复权价格和技术指标序列。

| 数据 | 来源 | 语义 |
|---|---|---|
| `buy_signal`、`pool_member`、`score_breakdown`、`selected_metrics` | PostgreSQL `rearview` database | run 执行时保存的结果快照 |
| 不复权 K 线、成交量和右侧 quote 字段 | ClickHouse `fleur_marts.mart_stock_quotes_daily`，经 Rearview API 查询 | 当前 mart 查询值 |
| 前复权和后复权 K 线 | ClickHouse mart 层复权行情消费接口，具体表待数据平台补齐 | 当前 mart 查询值 |
| MA、BOLL、MACD | ClickHouse `fleur_marts.mart_stock_trend_indicator`，经 Rearview API 查询 | 当前 mart 查询值，当前主要为前复权指标口径 |
| RSI、KDJ | ClickHouse `fleur_marts.mart_stock_momentum_indicator`，经 Rearview API 查询 | 当前 mart 查询值，当前主要为复权指标口径 |

约束：

1. 运行结果解释默认使用 PostgreSQL run snapshot，不用当前 mart 值重算分数。
2. 当前 mart 值只用于行情上下文和补充分析，不能覆盖 `selected_metrics`。
3. 页面上必须明确标记当前 mart 值，避免用户误认为这些字段是 run 当时写入的快照。
4. 后端 API 响应应包含 `source_database`、`source_table`、`adjustment` 和 `value_semantics` 或等价 metadata，便于前端展示来源和价格口径。
5. Rearview 不应为了本页面绕过 mart 层直接读取 raw、staging、intermediate 或 calculation 表；复权 OHLC 和缺失 MA30 需要先在数据平台补齐 mart 消费边界。

## 后端 API 草案

第一版推荐新增页面级组合接口，减少前端在刷新页面时拼装多个事实源的复杂度。

### `GET /rearview/runs/{run_id}/securities/{security_code}/analysis`

用途：返回单个 run result 证券的页面主数据，包括 run result snapshot、图表窗口、当前选中日期的 quote mart 行、K 线序列、MA 序列和下方技术指标序列。

Query：

| 参数 | 必填 | 默认 | 说明 |
|---|---|---|---|
| `trade_date` | 是 | - | 当前 run result 日期 |
| `source` | 是 | - | `signals` 或 `pool` |
| `adjustment` | 否 | `forward_adjusted` | K 线价格口径：`forward_adjusted`、`backward_adjusted`、`unadjusted` |
| `quote_end_date` | 否 | `trade_date` | K 线窗口结束日期 |
| `lookback_trading_days` | 否 | `240` | 回看交易行数 |
| `quote_start_date` | 否 | - | 显式开始日期；提供后优先于 lookback |
| `ma_windows` | 否 | `5,10,30` | 第一版只接受 `5,10,30` 子集；不支持任意窗口 |

响应草案：

```json
{
  "run_id": "e5f59cae-69b4-4a4a-a102-86c77f25848e",
  "trade_date": "2026-06-12",
  "security_code": "sh.600000",
  "source": "signals",
  "adjustment": "forward_adjusted",
  "result_snapshot": {
    "rank": 1,
    "score": 92.5,
    "score_breakdown": {},
    "selected_metrics": {}
  },
  "sources": {
    "quote": {
      "database": "fleur_marts",
      "table": "mart_stock_quotes_daily",
      "value_semantics": "current_mart_query",
      "adjustment": "unadjusted"
    },
    "adjusted_quote": {
      "database": "fleur_marts",
      "table": "mart_stock_quotes_adjusted_daily",
      "value_semantics": "current_mart_query",
      "status": "required_before_implementation"
    },
    "trend": {
      "database": "fleur_marts",
      "table": "mart_stock_trend_indicator",
      "value_semantics": "current_mart_query",
      "adjustment": "forward_adjusted"
    },
    "momentum": {
      "database": "fleur_marts",
      "table": "mart_stock_momentum_indicator",
      "value_semantics": "current_mart_query",
      "adjustment": "forward_adjusted"
    }
  },
  "chart_window": {
    "start_date": "2025-06-20",
    "end_date": "2026-06-12",
    "lookback_trading_days": 240
  },
  "chart": {
    "ma": {
      "requested_windows": [5, 10, 30],
      "default_visible_windows": [5, 10, 30],
      "available_windows": [5, 10, 30],
      "adjustment": "forward_adjusted"
    },
    "indicator_panels": ["kdj", "rsi", "macd", "boll"],
    "series": [
      {
        "trade_date": "2026-06-12",
        "ohlc": {
          "open": 10.1,
          "high": 10.5,
          "low": 9.9,
          "close": 10.3
        },
        "volume": 1234567,
        "ma": {
          "5": 10.05,
          "10": 9.97,
          "30": 9.42
        },
        "kdj": {
          "k": 65.1,
          "d": 58.4,
          "j": 78.5
        },
        "rsi": {
          "6": 61.2,
          "12": 54.8,
          "24": 49.9
        },
        "macd": {
          "dif": 0.12,
          "dea": 0.08,
          "histogram": 0.04
        },
        "boll": {
          "mid_20_2": 9.8,
          "up_20_2": 10.6,
          "dn_20_2": 9.0
        }
      }
    ]
  },
  "quote_rows": [
    {
      "security_code": "sh.600000",
      "trade_date": "2026-06-12",
      "open_price": 10.1,
      "high_price": 10.5,
      "low_price": 9.9,
      "close_price": 10.3,
      "volume": 1234567,
      "pct_change": 2.1,
      "turnover_rate": 1.2,
      "pe_ttm": 8.5,
      "pb_mrq": 0.9,
      "kdj_k_value": 65.1
    }
  ],
  "selected_quote": {
    "security_code": "sh.600000",
    "trade_date": "2026-06-12",
    "source_table": "mart_stock_quotes_daily"
  }
}
```

`chart.series` 是图表消费形态，OHLC 字段已经按请求的 `adjustment` 归一化为 `open`、`high`、`low`、`close`。`quote_rows` 中每一行应包含 `mart_stock_quotes_daily` 全部可展示字段，至少包含右侧指标面板需要的字段。`selected_quote` 是 `quote_rows` 中 `trade_date` 等于请求 `trade_date` 的行；如果该日无 mart 行，应返回 `null`，并保留 result snapshot。

如果后端无法提供与当前 `adjustment` 一致的 MA 序列，应把 `chart.ma.available_windows` 返回为空或缺少对应窗口，并提供能力 metadata；前端不得自己根据 OHLC 重算 MA。

### 左侧列表 API

左侧结果列表第一版可以复用现有接口：

```text
GET /rearview/runs/{run_id}/signals?trade_date=YYYY-MM-DD&limit=50&offset=0&sort=rank_asc
GET /rearview/runs/{run_id}/pool?trade_date=YYYY-MM-DD&limit=50&offset=0&sort=score_desc
```

如果后续发现前端需要在 `signals` 和 `pool` 间复用大量列表逻辑，可以新增归一化接口：

```text
GET /rearview/runs/{run_id}/result-securities?trade_date=YYYY-MM-DD&source=signals|pool
```

该归一化接口不是第一版强制项。

## API 约束

1. Rearview 查询 `mart_stock_quotes_daily` 时必须按 `security_code` 和日期窗口过滤，避免全市场 date-only 大扫描。
2. `lookback_trading_days` 应有上限，建议第一版最大 1000，防止浏览器和 API 返回过大 payload。
3. 查询 `mart_stock_trend_indicator`、`mart_stock_momentum_indicator` 和复权行情 mart 时，同样必须先按 `security_code` 和日期窗口过滤，再和 quote 窗口按 `security_code`、`trade_date` 对齐。该约束符合 ClickHouse `schema-pk-filter-on-orderby` 和 `query-join-filter-before` 规则。
4. 多 mart 对齐时应利用每个 mart 的 `(security_code, trade_date)` 唯一性；需要 join 时优先使用 filtered subquery，且在只需要单匹配时使用 `ANY JOIN` 语义，避免重复行扩大结果集。
5. 返回的 `chart.series` 和 `quote_rows` 按 `trade_date` 升序。
6. `security_code` 必须使用现有证券代码格式校验。
7. `adjustment` 必须按 enum 校验；未知值返回 validation error，不回退到默认口径。
8. `ma_windows` 第一版只允许 `5`、`10`、`30`，不得接受任意窗口，也不得把 `30` 映射到 `28`。
9. 如果 requested `security_code` 不属于该 run 的对应 `trade_date` 和 `source`，API 应返回 404 或 validation error，避免页面展示与 run 无关的证券。
10. ClickHouse 查询失败时，页面仍可展示 result snapshot，并把 K 线和指标区域置为错误状态。

## 前端状态与交互

页面状态来源：

- URL：`runId`、`securityCode`、`trade_date`、`source`、`adjustment`
- TanStack Query：run result list、security analysis payload
- 本地 UI 状态：当前图表选中日期、左侧分页 offset、图表范围选择、MA 开关状态

状态规则：

1. 证券切换必须更新 URL，支持浏览器前进/后退。
2. `adjustment` 切换必须更新 URL 并重新请求 analysis payload；缺省值为 `forward_adjusted`。
3. MA 开关第一版可以只保存在本地状态，不必写入 URL；默认 MA5、MA10、MA30 为打开。
4. 图表选中日期第一版可以只保存在本地状态，不必写入 URL。
5. `source` 不应在页面内自动从 `signals` 切到 `pool`；用户从哪个结果 tab 进入，就保留哪个结果来源。
6. 如果 `source=signals` 但该证券不是买入信号，应显示明确错误，不自动回退到 pool。
7. 页面刷新时必须能重新加载主数据，不依赖从上一页传入的 React state。

## 空状态与错误状态

| 场景 | UI 行为 |
|---|---|
| run 不存在 | 显示 run not found，并提供返回 `/runs` |
| result row 不存在 | 显示该证券不属于当前 run/trade/source |
| mart 无 K 线数据 | 保留 result snapshot，图表显示无行情数据 |
| 复权口径缺少 OHLC | 保留 result snapshot 和右侧 quote 面板，图表提示该价格口径暂无 mart 数据 |
| MA30 或同口径 MA 缺失 | 保留 K 线，禁用缺失 MA 开关，不使用其他窗口替代 |
| 技术指标面板缺失数据 | 对应面板显示空状态，不在前端重算或填充 |
| selected quote 缺失 | 右侧指标面板显示该日无 mart row |
| ClickHouse 查询失败 | 保留左侧列表和 run snapshot，图表/指标区域显示可重试错误 |
| PostgreSQL result 查询失败 | 页面主错误，不展示 K 线，避免脱离 run 结果上下文 |

## 响应式与可用性要求

1. 桌面首屏应同时可见左侧列表、中间图表和右侧指标面板。
2. 结果列表、图表和指标面板的滚动区域应独立，避免页面整体滚动导致上下文丢失。
3. 主 K 线、成交量和四个下方指标面板必须有稳定尺寸，加载、空状态、hover、MA 开关和 resize 不应造成布局跳动。
4. 移动端使用 tabs 或分段控件，不强行三栏展示。
5. `Open` 行为应使用导航，不再打开抽屉；如果保留详情抽屉，应作为页面内的辅助入口，而不是 `Open` 的主行为。

## 验收标准

1. 在 `/runs/:runId` 的买入信号行点击 `Open`，进入 `/runs/:runId/securities/:securityCode?trade_date=...&source=signals&adjustment=forward_adjusted`。
2. 新页面刷新后仍能恢复当前证券、交易日、结果来源、K 线价格口径和指标。
3. 左侧列表展示同一 run、同一交易日、同一结果来源下的证券，并高亮当前证券。
4. 中间日 K 线图默认使用 `forward_adjusted`，并可切换到 `backward_adjusted` 和 `unadjusted`；切换后 OHLC 口径实际变化。
5. 主 K 线默认展示 MA5、MA10、MA30，三个 MA 开关可独立隐藏和恢复；缺失 MA30 时不得用 MA28 代替。
6. K 线下方展示 KDJ、RSI、MACD 和 BOLL 四个指标面板；有数据时非空，缺数据时显示对应空状态。
7. 右侧指标面板展示 `mart_stock_quotes_daily` 的分组字段，并标记为当前 mart 查询值。
8. 页面同时展示当前证券的 run snapshot，并与当前 mart 查询值区分。
9. 前端不直接访问 ClickHouse 或 PostgreSQL，不在浏览器内重算 MA、KDJ、RSI、MACD 或 BOLL。
10. 桌面和移动端 Playwright CDP 截图确认图表非空、三栏或 tabs 布局无重叠。

## 最小验证

涉及前端实现时运行：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run test
npm run build
```

涉及 Rearview API 实现时运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及页面验收时运行：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
```

并通过 Playwright CDP 检查：

- `/runs/:runId/securities/:securityCode?...` 桌面截图
- 同一路由移动端截图
- 图表 canvas 或 SVG 内容非空
- console 无前端错误
- network 中 Rearview analysis API 返回 200

## 已决事项

1. `Open` 的主行为改为导航到独立个股分析页，而不是打开抽屉。
2. 日 K 线支持 `forward_adjusted`、`backward_adjusted`、`unadjusted` 三种价格口径，默认 `forward_adjusted`。
3. 默认 K 线窗口以结果 `trade_date` 为结束日期，不默认展示信号日之后的数据。
4. 主 K 线默认叠加 MA5、MA10、MA30，并提供开关；MA30 不得用 MA28 替代。
5. K 线下方展示 KDJ、RSI、MACD 和 BOLL 指标面板，指标来自 mart/API，不在前端重算。
6. 当前 mart 查询值必须和 PostgreSQL run snapshot 明确区分。
7. 前端通过 Rearview HTTP API 读取 mart 数据，不直接访问 ClickHouse。
8. 复权 OHLC 扩展到 `mart_stock_quotes_daily`，不新增单独 chart mart 作为第一阶段前置项。

## 待决问题

1. 是否需要在第一版展示证券名称；当前 run result 和 `mart_stock_quotes_daily` 只有 `security_code`。
2. 非 `forward_adjusted` 口径下，是否需要补齐同口径 MA/BOLL/RSI/MACD/KDJ；若不补齐，UI 应只展示前复权指标口径并显式标记。
3. 页面级组合接口和多个细粒度接口的边界是否需要在实现前进一步压测确认。
4. 是否需要把图表选中日期写入 URL，支持分享到具体 chart date。
5. 是否需要在图表上叠加 run 信号点、买卖点或价格行为结构标记。
