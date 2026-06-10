# 当前 int 层指标盘点（临时）

状态：Temporary

日期：2026-06-10
范围：`pipeline/elt/models/intermediate/*.sql` 和同目录 YAML 文档。

本文记录当前 `fleur_intermediate` / dbt `intermediate` 层已经具备的指标字段。这里的“指标”包括行情派生指标、复权指标、技术指标、估值指标、股本/权益组成指标；基础维度字段单独列为支撑模型。

## 总览

当前 int 层共有 13 个模型：

| 模型 | 粒度 | 定位 |
|---|---|---|
| `int_stock_quotes_daily_unadj` | 每证券、交易日一行 | 未复权日行情和日频派生指标 |
| `int_stock_adjustment_factor` | 每证券、交易日一行 | 前复权/后复权比例和累计因子 |
| `int_stock_quotes_daily_adj` | 每证券、交易日一行 | 前复权/后复权 OHLC 与昨收价格 |
| `int_stock_ma_daily` | 每证券、交易日一行 | MA、EMA、均量指标 wrapper |
| `int_stock_boll_daily` | 每证券、交易日一行 | BOLL 指标 wrapper |
| `int_stock_rsi_daily` | 每证券、交易日一行 | RSI 指标 wrapper |
| `int_stock_kdj_daily` | 每证券、交易日一行 | RSV/KDJ 指标 wrapper |
| `int_stock_price_pattern_daily` | 每证券、交易日一行 | 价格方向、连涨连跌和 20-bar N 型结构 |
| `int_stock_financial_valuation` | 每证券、财报报告期一行 | PE、PB、每股净资产、ROE/ROA |
| `int_stock_shares_history` | 每证券、股本区间生效日一行 | 股本区间、A 股流通股本和自由流通股本估算 |
| `int_stock_exrights_event` | 每证券、除权除息日一行 | 分红、送股、转增、配股组成和事件标签 |
| `int_stock_basic_snapshot` | 每证券一行 | 股票 universe、板块、上市状态 |
| `int_trade_calendar` | 每交易日一行 | 前一交易日 |

## 日频行情与市场指标

来源模型：`int_stock_quotes_daily_unadj`

| 指标族 | 字段 | 当前口径 |
|---|---|---|
| 基础行情 | `open_price`, `high_price`, `low_price`, `close_price`, `prev_close_price`, `prev_close_price_unadj`, `volume`, `amount`, `prev_volume` | 未复权日行情；`prev_close_price` 沿用 BaoStock preclose，`prev_close_price_unadj` 为前一 A 股交易日未复权收盘价 |
| 换手 | `turnover_rate`, `turnover_rate_actual` | 成交量分别除以 A 股流通股本、A 股自由流通股本，再乘以 100 |
| 涨跌与波动 | `pct_change`, `pct_amplitude` | 涨跌幅和振幅，使用 BaoStock preclose 口径昨收作为分母，百分数口径 |
| 涨跌停理论价 | `limit_up_price`, `limit_down_price` | 基于板块、ST 状态和 preclose 派生，四舍五入到分 |
| 市值 | `a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap` | 未复权收盘价乘以 A 股总股本、A 股流通股本、A 股自由流通股本 |
| 日频股本口径 | `a_shares`, `a_float_shares`, `a_free_float_shares` | 从 `int_stock_shares_history` 按交易日 as-of 到日频 |
| 股息率 | `dy_static`, `dy_ttm` | 静态股息率使用最近年度现金分红每股金额；TTM 股息率使用过去 12 个月现金分红每股金额合计；均除以未复权收盘价再乘以 100 |
| 交易状态 | `is_suspend`, `is_st` | 停牌和 ST/风险警示状态 |

## 复权价格与复权因子

来源模型：`int_stock_adjustment_factor`, `int_stock_quotes_daily_adj`

| 指标族 | 字段 | 当前口径 |
|---|---|---|
| 复权单步比例 | `backward_adjustment_ratio`, `forward_adjustment_ratio` | 使用 `prev_close_price_unadj` 与 BaoStock `prev_close_price` 的有效价格对计算；无效价格对取 1.0 |
| 复权累计因子 | `backward_adjustment_factor`, `forward_adjustment_factor` | 按证券和交易日累计单步比例 |
| 后复权价格 | `open_price_backward_adj`, `high_price_backward_adj`, `low_price_backward_adj`, `close_price_backward_adj`, `prev_close_price_backward_adj` | 未复权价格乘以后复权累计因子 |
| 前复权价格 | `open_price_forward_adj`, `high_price_forward_adj`, `low_price_forward_adj`, `close_price_forward_adj`, `prev_close_price_forward_adj` | 未复权价格乘以前复权累计因子 |

## 技术指标

以下模型是 dbt intermediate wrapper，直接暴露 Furnace/Dagster 物化的 `fleur_calculation.calc_*` 结果；dbt 不重写指标公式。

| 模型 | 指标字段 | 当前口径 |
|---|---|---|
| `int_stock_ma_daily` | `price_ma_3`, `price_ma_5`, `price_ma_6`, `price_ma_10`, `price_ma_12`, `price_ma_14`, `price_ma_20`, `price_ma_24`, `price_ma_28`, `price_ma_57`, `price_ma_60`, `price_ma_114`, `price_ma_250` | 基于 `close_price_forward_adj` 的有效收盘价简单移动平均 |
| `int_stock_ma_daily` | `price_avg_ma_3_6_12_24`, `price_avg_ma_14_28_57_114` | 指定 MA 组合的均值，任一组件为空时输出 NULL |
| `int_stock_ma_daily` | `price_ema2_10` | `EMA(EMA(close_price_forward_adj, 10), 10)`，SMA 启动 |
| `int_stock_ma_daily` | `volume_ma_5`, `volume_ma_10`, `volume_ma_20`, `volume_ma_60` | 基于未复权日行情 `volume` 的均量；0 成交量是有效输入 |
| `int_stock_boll_daily` | `boll_mid_10_1p5`, `boll_up_10_1p5`, `boll_dn_10_1p5` | BOLL(10, 1.5)，基于前复权收盘价；STD 为总体标准差 `ddof=0` |
| `int_stock_boll_daily` | `boll_mid_20_2`, `boll_up_20_2`, `boll_dn_20_2` | BOLL(20, 2)，基于前复权收盘价 |
| `int_stock_boll_daily` | `boll_mid_50_2p5`, `boll_up_50_2p5`, `boll_dn_50_2p5` | BOLL(50, 2.5)，基于前复权收盘价 |
| `int_stock_rsi_daily` | `rsi_6`, `rsi_12`, `rsi_14`, `rsi_24`, `rsi_25`, `rsi_50` | 基于前复权收盘价，使用 Wilder smoothing；非空值应在 `[0, 100]` |
| `int_stock_kdj_daily` | `rsv_window`, `k_smoothing`, `d_smoothing`, `rsv`, `k_value`, `d_value`, `j_value` | canonical KDJ(9,3,3)，固定使用前复权价格口径 |
| `int_stock_price_pattern_daily` | `close_direction`, `close_up_streak_days`, `close_down_streak_days` | 未复权收盘价与 BaoStock preclose 逐行比较；方向为空会打断连续性 |
| `int_stock_price_pattern_daily` | `n_structure_20_valid_bars`, `n_structure_20_high_date`, `n_structure_20_high_price`, `n_structure_20_low_date`, `n_structure_20_low_price`, `n_structure_20_second_low_date`, `n_structure_20_second_low_price`, `n_structure_20_second_low_ratio`, `n_structure_20_is_valid` | 基于前复权 high/low 的最近 20 根有效价格柱，识别前低、次低结构；`n_structure_20_is_valid` 表示次低/前低比例严格大于 1 |

## 财务估值指标

来源模型：`int_stock_financial_valuation`

该模型不是日频模型，而是每证券、财报报告期一行。估值使用报告期当日或之前最近交易日未复权收盘价、as-of 总股本、利润表和资产负债表。

| 指标 | 字段 | 当前口径 |
|---|---|---|
| 静态市盈率 | `pe_static` | 报告期 as-of 最近年度年报归母净利润 |
| TTM 市盈率 | `pe_ttm` | 当前及之前连续四个单季度归母净利润合计 |
| 动态市盈率 | `pe_forecast` | 当前 YTD 归母净利润按报告期季度数年化 |
| MRQ 市净率 | `pb_mrq` | 报告期 as-of 最近归母权益 |
| 每股净资产 | `book_value_per_share` | 报告期 as-of 最近归母权益除以实收资本 |
| 盈利能力 | `roe`, `roa`, `roaa`, `roae` | ROE/ROA 使用期末分母；ROAA/ROAE 使用期初和期末平均分母；比率口径，不乘以 100 |

## 股本与权益事件指标

来源模型：`int_stock_shares_history`, `int_stock_exrights_event`

| 模型 | 指标字段 | 当前口径 |
|---|---|---|
| `int_stock_shares_history` | `total_shares`, `float_shares`, `a_shares`, `a_float_shares`, `a_free_float_shares` | 股本区间字段；A 股股本为已上市流通 A 股加限售 A 股；A 股自由流通股本为 A 股流通股本扣减超过 5% 大股东流通 A 股持股 |
| `int_stock_shares_history` | `major_holder_a_float_shares`, `major_holder_count` | 纳入自由流通股本扣减的大股东持股数量合计和股东数 |
| `int_stock_exrights_event` | `cash_dividend_per_share`, `bonus_share_per_share`, `transfer_share_per_share`, `allotment_share_per_share`, `allotment_price_yuan` | 分红送转和配股组成统一转换为每 1 股口径 |
| `int_stock_exrights_event` | `event_tag`, `has_cash_dividend`, `has_share_right` | `XR` 表示除权，`XD` 表示除息，`DR` 表示除权除息同时发生 |

## 支撑模型

| 模型 | 可用字段 | 用途 |
|---|---|---|
| `int_stock_basic_snapshot` | `security_code`, `exchange_code`, `security_name`, `ipo_date`, `out_date`, `listing_status`, `is_listed`, `security_board` | 股票 universe、板块分类、上市状态和涨跌停规则输入 |
| `int_trade_calendar` | `trade_date`, `prev_trade_date` | 前一交易日补齐，用于未复权前收、前量和时间序列指标 |

## 当前未覆盖或需下游补充

- 日频 PE/PB 尚未在 int 层直接输出；当前 PE/PB 在 `int_stock_financial_valuation` 中按财报报告期输出。
- MACD、WR、CCI、ATR、OBV 等常见技术指标当前没有 int 模型。
- 涨跌停只提供理论价，未提供全市场涨停/跌停事件事实。
- 盘口、逐笔、内外盘、主动买卖方向等 intraday/level-2 指标当前没有输入和 int 模型。

## 事实来源

- `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`
- `pipeline/elt/models/intermediate/int_stock_adjustment_factor.sql`
- `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
- `pipeline/elt/models/intermediate/int_stock_ma_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_boll_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_rsi_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_kdj_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_price_pattern_daily.sql`
- `pipeline/elt/models/intermediate/int_stock_financial_valuation.sql`
- `pipeline/elt/models/intermediate/int_stock_shares_history.sql`
- `pipeline/elt/models/intermediate/int_stock_exrights_event.sql`
- `pipeline/elt/models/intermediate/int_stock_basic_snapshot.sql`
- `pipeline/elt/models/intermediate/int_trade_calendar.sql`
