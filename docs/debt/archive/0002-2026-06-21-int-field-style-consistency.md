# Debt: Intermediate 层字段风格与一致性收敛

日期：2026-06-21

## 执行状态

状态更新：2026-06-21。本文档为 int 层字段风格与一致性梳理的债务登记，尚未落地任何改动。与 `0003-2026-06-21-mart-field-style-consistency.md` 配套，建议从 int 层开始整改，mart 层随后对齐。

| # | 债务 | 优先级 | 状态 | 证据 |
|---|------|--------|------|------|
| IA1 | 缺失 `config()` 块的模型继承默认配置 | 高 | 已关闭 | `int_government_bond_yields_daily`、`int_stock_basic_snapshot`、`int_trade_calendar` 无 config。 |
| IA2 | stock/index grain `order_by` 主键顺序不一致 | 高 | 已关闭 | quotes/adjustment/adj 为 `(trade_date, security_code)`，benchmark_returns 为 `(trade_date, security_code)`；mart 层 quotes 为 `(security_code, trade_date)`。 |
| IA3 | BOLL 上下轨缩写不对称 | 中 | 已关闭 | `boll_up_*`（full word）vs `boll_dn_*`（abbreviated），与 mart 层 A3 同源。 |
| IA4 | 百分比口径字段无统一信号 | 中 | 已关闭 | `pct_*` 带 `pct_` 前缀，`turnover_rate`/`turnover_rate_actual`/`dy_*` 同为百分数但无前缀；`roe`/`roa` 为小数比例。 |
| IA5 | `a_` 前缀语义模糊（过度设计） | 中 | 已关闭 | `a_market_cap`/`a_shares`/`a_float_shares`/`a_free_float_shares`，整表已是 A 股域。 |
| IA6 | `turnover_rate_actual` 命名含糊 | 中 | 已关闭 | "actual" 实为自由流通股本分母，与 mart 层 A6 同源。 |
| IA7 | portfolio 系列 YAML 列文档严重不完整 | 高 | 已关闭 | closed_trade 缺 17 列、performance_metric 缺 18 列、performance_metric_status 缺 3 列、trade_metric 缺 15 列。 |
| IA8 | 指标 wrapper 字段描述中英文混用 | 低 | 已关闭 | kdj/ma/price_pattern/rsi 的列描述为英文，但模型描述和 meta 为中文。 |
| IA9 | `is_suspend` 在 index_quotes 中语义存疑 | 中 | 已细化 | 已拆分为 IE1a（移除输出）和 IE1b（原 IE1 描述更新）。 |
| IA9b | `int_index_quotes_daily` 不再输出 `is_suspend` | 中 | 已关闭 | 指数无停牌概念，`is_suspend` 透传自 BaoStock 行情无实际语义；下游 `int_benchmark_returns_daily` 和 `mart_benchmark_returns_daily` 均不消费该字段。 |
| IA10 | 配置块缺少 `unique_key` / `partition_by` 不统一 | 中 | 已关闭 | 部分 table 模型未设 `partition_by`（shares_history、exrights_event 有，但 snapshot 系列无）；`unique_key` 全部缺失。 |
| IA11 | 政府债收益率字段命名风格不一致 | 低 | 已关闭 | 决策采用 `_pct` 后缀方案后，政府债 `*_yield_pct` 已符合，无需改动。 |
| IA12 | 关键度量字段无范围测试 | 低 | 已关闭 | `return_daily`、`pct_change`、RSI 值等无范围约束。 |
| IA13 | `int_stock_shares_history.effective_date` 上游日期字段命名不一致且语义混淆 | 中 | 已关闭 | equity_history.stg 用 `report_date`、freeholders.stg 用 `end_date`，两者都来自 raw `END_DATE` 但语义不同；经 raw 数据验证，freeholders.END_DATE 是报告期，equity_history.END_DATE 是股本变动截止日。 |

## 背景

`refactor/int-mart-field-consistency` 分支目标是调整 int 和 mart 层数据字段风格和一致性。因 int 层是 mart 层的上游，字段命名和口径的改动应从 int 层开始，mart 层随后对齐透传，避免在 mart 层用 alias 掩盖 int 层的不一致。

int 层共 25 个模型，按业务域分为：基础快照（6）、日频行情（4）、技术指标 wrapper（6）、复权（2）、财务估值（1）、组合绩效（5）、交易日历（1）。

字段命名 canonical 规则见 `pipeline/elt/metadata/field_glossary.yml`：`canonical_field_pattern: "^[a-z][a-z0-9_]*$"`，字段描述以中文 `description_zh` 为事实源。

## Int 层模型清单

### 1. 基础快照域

#### `int_stock_basic_snapshot` — 股票基础信息快照

- **materialized**: 默认（无 config）
- **order_by**: 默认 · **partition_by**: 无
- **upstream**: `stg_baostock__query_stock_basic`（过滤 `security_type = 'stock'`）
- **字段**（12）：`security_code`, `security_local_code`, `exchange_code`, `security_name`, `ipo_date`, `out_date`, `listing_status_code`, `listing_status`, `is_listed`, `security_type_code`, `security_type`, `security_board`
- **YAML**: 中文描述，列完整

#### `int_index_basic_snapshot` — 指数基础信息快照

- **materialized**: `table` · **order_by**: `security_code` · **partition_by**: 无
- **upstream**: `stg_baostock__query_stock_basic`（过滤 `security_type = 'index'`）
- **字段**（11）：`security_code`, `security_local_code`, `exchange_code`, `index_name`（alias from `security_name`）, `ipo_date`, `out_date`, `listing_status_code`, `listing_status`, `is_listed`, `security_type_code`, `security_type`
- **YAML**: 中文描述，列完整

#### `int_benchmark_basic_snapshot` — benchmark 基础信息快照

- **materialized**: `table` · **order_by**: `security_code` · **partition_by**: 无
- **upstream**: `int_index_basic_snapshot`
- **字段**（6）：`security_code`, `security_local_code`, `exchange_code`, `index_name`, `listing_status`, `is_listed`
- **YAML**: 中文描述，列完整

#### `int_stock_shares_history` — 股本有效区间

- **materialized**: `table` · **order_by**: `(security_code, effective_date)` · **partition_by**: 无
- **upstream**: `stg_eastmoney__equity_history`, `stg_eastmoney__freeholders`
- **字段**（12）：`security_code`, `effective_date`, `expiry_date`, `source_equity_end_date`, `source_freeholders_end_date`, `total_shares`, `float_shares`（alias from `unlimited_shares`）, `a_shares`, `a_float_shares`（alias from `listed_a_shares`）, `a_free_float_shares`, `major_holder_a_float_shares`, `major_holder_count`
- **YAML**: 中文描述，列完整

#### `int_stock_exrights_event` — 除权除息事件

- **materialized**: `table` · **order_by**: `(security_code, ex_dividend_date)` · **partition_by**: `toYear(ex_dividend_date)`
- **upstream**: `stg_eastmoney__dividend_main`, `stg_eastmoney__dividend_allotment`
- **字段**（17）：`security_code`, `ex_dividend_date`, `equity_record_date`, `notice_date`, `report_date`, `report_period_label`, `cash_dividend_per_share`, `bonus_share_per_share`, `transfer_share_per_share`, `allotment_share_per_share`, `allotment_price_yuan`, `event_tag`, `has_cash_dividend`, `has_share_right`, `source_has_dividend_main`, `source_has_allotment`, `source_plan_text`, `source_allotment_text`
- **YAML**: 中文描述，列完整

#### `int_trade_calendar` — 交易日历

- **materialized**: 默认（无 config）
- **order_by**: 默认 · **partition_by**: 无
- **upstream**: `stg_sina__trade_calendar`
- **字段**（2）：`trade_date`, `prev_trade_date`
- **YAML**: 中文描述，列完整

### 2. 日频行情域

#### `int_stock_quotes_daily_unadj` — 未复权日行情（核心事实表）

- **materialized**: `table` · **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **upstream**: `int_stock_basic_snapshot`, `stg_baostock__query_history_k_data_plus_daily`, `int_trade_calendar`, `int_stock_shares_history`, `int_stock_exrights_event`
- **字段**（22）：`security_code`, `trade_date`, `open_price`, `high_price`, `low_price`, `close_price`, `prev_close_price`, `prev_close_price_unadj`, `prev_volume`, `volume`, `amount`, `turnover_rate`, `turnover_rate_actual`, `pct_amplitude`, `pct_change`, `limit_up_price`, `limit_down_price`, `a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap`, `a_shares`, `a_float_shares`, `a_free_float_shares`, `dy_static`, `dy_ttm`, `is_suspend`, `is_st`
- **YAML**: 中文描述，列完整

#### `int_stock_quotes_daily_adj` — 复权价格

- **materialized**: `table` · **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **upstream**: `int_stock_quotes_daily_unadj`, `int_stock_adjustment_factor`
- **字段**（15）：`security_code`, `trade_date`, `open_price_backward_adj`, `high_price_backward_adj`, `low_price_backward_adj`, `close_price_backward_adj`, `prev_close_price_backward_adj`, `open_price_forward_adj`, `high_price_forward_adj`, `low_price_forward_adj`, `close_price_forward_adj`, `prev_close_price_forward_adj`, `backward_adjustment_factor`, `backward_adjustment_ratio`, `forward_adjustment_factor`, `forward_adjustment_ratio`
- **YAML**: 中文描述，列完整

#### `int_stock_adjustment_factor` — 复权因子

- **materialized**: `table` · **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **upstream**: `int_stock_quotes_daily_unadj`
- **字段**（8）：`security_code`, `trade_date`, `prev_close_price_unadj`, `prev_close_price`, `backward_adjustment_ratio`, `backward_adjustment_factor`, `forward_adjustment_ratio`, `forward_adjustment_factor`
- **YAML**: 中文描述，列完整

#### `int_index_quotes_daily` — 指数日行情

- **materialized**: `table` · **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **upstream**: `int_index_basic_snapshot`, `stg_baostock__query_history_k_data_plus_daily`
- **字段**（11）：`security_code`, `trade_date`, `open_price`, `high_price`, `low_price`, `close_price`, `prev_close_price`, `return_daily`（derived: `close_price/prev_close_price - 1`）, `volume`, `amount`, `is_suspend`
- **YAML**: 中文描述，列完整

### 3. 技术指标 wrapper 域（均 `materialized='view'`，直读 `fleur_calculation` source）

#### `int_stock_kdj_daily` — RSV/KDJ

- **upstream**: `source('fleur_calculation', 'calc_stock_kdj_daily')`
- **字段**（9）：`security_code`, `trade_date`, `rsv_window`, `k_smoothing`, `d_smoothing`, `rsv`, `k_value`, `d_value`, `j_value`
- **YAML**: 模型描述/meta 中文，列描述英文

#### `int_stock_ma_daily` — MA/EMA

- **upstream**: `source('fleur_calculation', 'calc_stock_ma_daily')`
- **字段**（20）：`security_code`, `trade_date`, `price_ma_3/5/6/10/12/14/20/24/28/30/57/60/114/250`, `price_avg_ma_3_6_12_24`, `price_avg_ma_14_28_57_114`, `price_ema2_10`, `volume_ma_5/10/20/60`
- **YAML**: 模型描述/meta 中文，列描述英文

#### `int_stock_boll_daily` — BOLL

- **upstream**: `source('fleur_calculation', 'calc_stock_boll_daily')`
- **字段**（11）：`security_code`, `trade_date`, `boll_mid_10_1p5`, `boll_up_10_1p5`, `boll_dn_10_1p5`, `boll_mid_20_2`, `boll_up_20_2`, `boll_dn_20_2`, `boll_mid_50_2p5`, `boll_up_50_2p5`, `boll_dn_50_2p5`
- **YAML**: 模型描述/meta 中文，列描述中文

#### `int_stock_macd_daily` — MACD

- **upstream**: `source('fleur_calculation', 'calc_stock_macd_daily')`
- **字段**（5）：`security_code`, `trade_date`, `macd_dif`, `macd_dea`, `macd_histogram`
- **YAML**: 中文描述，列完整

#### `int_stock_rsi_daily` — RSI

- **upstream**: `source('fleur_calculation', 'calc_stock_rsi_daily')`
- **字段**（8）：`security_code`, `trade_date`, `rsi_6`, `rsi_12`, `rsi_14`, `rsi_24`, `rsi_25`, `rsi_50`
- **YAML**: 模型描述/meta 中文，列描述英文

#### `int_stock_price_pattern_daily` — 价格行为与结构

- **upstream**: `source('fleur_calculation', 'calc_stock_price_pattern_daily')`
- **字段**（14）：`security_code`, `trade_date`, `close_direction`, `close_up_streak_days`, `close_down_streak_days`, `n_structure_20_valid_bars`, `n_structure_20_high_date`, `n_structure_20_high_price`, `n_structure_20_low_date`, `n_structure_20_low_price`, `n_structure_20_second_low_date`, `n_structure_20_second_low_price`, `n_structure_20_second_low_ratio`, `n_structure_20_is_valid`
- **YAML**: 模型描述/meta 中文，列描述英文

### 4. 财务估值域

#### `int_stock_financial_valuation` — 财报期末估值

- **materialized**: `table` · **order_by**: `(security_code, report_date)` · **partition_by**: `toYear(report_date)`
- **upstream**: `stg_eastmoney__income_ytd`, `stg_eastmoney__balance`, `int_stock_quotes_daily_unadj`, `int_stock_shares_history`, `stg_eastmoney__income_sq`
- **字段**（11）：`security_code`, `report_date`, `pe_static`, `pe_ttm`, `pe_forecast`, `pb_mrq`, `book_value_per_share`, `roe`, `roa`, `roaa`, `roae`
- **YAML**: 中文描述，列完整

### 5. 组合绩效域（均 `materialized='view'`，直读 `fleur_portfolio`/`fleur_calculation` source）

#### `int_portfolio_closed_trade` — 已平仓交易明细

- **grain**: `(closed_trade_id)`
- **upstream**: `source('fleur_portfolio', 'portfolio_run_snapshot')`, `source('fleur_calculation', 'calc_portfolio_closed_trade')`
- **字段**（21）：`portfolio_run_id`, `result_attempt_id`, `closed_trade_id`, `closed_trade_seq`, `position_lot_id`, `entry_trade_seq`, `exit_trade_seq`, `security_code`, `entry_date`, `exit_date`, `quantity`, `entry_gross_amount`, `exit_gross_amount`, `entry_fee`, `exit_fee`, `total_fee`, `realized_pnl`, `realized_return`, `holding_days`, `exit_reason`, `created_at`
- **YAML**: 英文描述，**仅文档化 4 列**（缺 17 列）

#### `int_portfolio_performance_metric` — 绩效指标

- **grain**: `(portfolio_run_id, result_attempt_id, security_code, window_key)`
- **upstream**: `source('fleur_portfolio', 'portfolio_run_snapshot')`, `source('fleur_calculation', 'calc_portfolio_performance_metric')`
- **字段**（25）：`portfolio_run_id`, `result_attempt_id`, `source_run_id`, `security_code`, `window_key`, `window_start`, `window_end`, `run_start_date`, `run_end_date`, `config_hash`, `metric_status`, `observation_count`, `holding_period_return`, `annualized_return`, `annualized_volatility`, `max_drawdown`, `calmar_ratio`, `downside_deviation`, `sortino_ratio`, `sharpe_ratio`, `information_ratio`, `beta`, `alpha`, `treynor_ratio`, `computed_at`
- **YAML**: 英文描述，**仅文档化 7 列**（缺 18 列，包括全部 12 个绩效指标值字段）

#### `int_portfolio_performance_metric_status` — 指标状态

- **grain**: `(portfolio_run_id, result_attempt_id, security_code, window_key, metric_name)`
- **upstream**: `source('fleur_portfolio', 'portfolio_run_snapshot')`, `source('fleur_calculation', 'calc_portfolio_performance_metric_status')`
- **字段**（8）：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `metric_name`, `metric_status`, `reason_code`, `computed_at`
- **YAML**: 英文描述，**仅文档化 5 列**（缺 `security_code`, `window_key`, `computed_at`；其中 `security_code`/`window_key` 被 model-level unique test 引用却未文档化）

#### `int_portfolio_performance_metric_rank_catalog` — 排名方向目录

- **materialized**: `ephemeral`
- **grain**: `(metric_name)`
- **upstream**: 无（内联字面量 catalog）
- **字段**（3）：`metric_name`, `rank_direction`, `null_policy`
- **YAML**: 英文描述，列完整

#### `int_portfolio_trade_metric` — 交易质量指标

- **grain**: `(portfolio_run_id, result_attempt_id, window_key)`
- **upstream**: `source('fleur_portfolio', 'portfolio_run_snapshot')`, `source('fleur_calculation', 'calc_portfolio_trade_metric')`
- **字段**（18）：`portfolio_run_id`, `result_attempt_id`, `window_key`, `window_start`, `window_end`, `closed_trade_count`, `winning_trade_count`, `losing_trade_count`, `breakeven_trade_count`, `win_rate_closed_trades`, `average_win_return`, `average_loss_return`, `profit_loss_ratio`, `average_holding_days`, `largest_win_return`, `largest_loss_return`, `computed_at`
- **YAML**: 英文描述，**仅文档化 3 列**（缺 15 列，包括全部交易指标值字段）

### 6. 基准 / 无风险利率域

#### `int_benchmark_returns_daily` — 基准日收益

- **materialized**: `table` · **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **upstream**: `int_benchmark_basic_snapshot`, `int_index_quotes_daily`
- **字段**（5）：`security_code`, `trade_date`, `close_price`, `prev_close_price`, `return_daily`
- **YAML**: 中文描述，列完整

#### `int_government_bond_yields_daily` — 国债收益率曲线

- **materialized**: 默认（无 config）
- **order_by**: 默认 · **partition_by**: 无
- **upstream**: `stg_chinabond__government_bond`
- **字段**（12）：`trade_date`, `three_month_yield_pct`, `six_month_yield_pct`, `one_year_yield_pct`, `two_year_yield_pct`, `three_year_yield_pct`, `five_year_yield_pct`, `seven_year_yield_pct`, `ten_year_yield_pct`, `fifteen_year_yield_pct`, `twenty_year_yield_pct`, `thirty_year_yield_pct`
- **YAML**: 中文描述，列完整

#### `int_risk_free_rate_daily` — 无风险利率

- **materialized**: `table` · **order_by**: `(source_tenor, trade_date)` · **partition_by**: `toYear(trade_date)`
- **upstream**: `int_trade_calendar`, `int_government_bond_yields_daily`
- **字段**（5）：`trade_date`, `source_date`, `source_tenor`, `annual_rate`, `daily_rate`
- **YAML**: 中文描述，列完整

---

## 整理建议

### IA. 命名与配置一致性（高优先级）

#### IA1. 补全缺失的 `config()` 块

三个模型完全缺少 config 块，继承项目默认（默认 view materialization + 默认 order_by），可能导致 ClickHouse 物化行为不符合预期：

| 模型 | 建议 config |
|---|---|
| `int_government_bond_yields_daily` | `table`, `order_by='trade_date'`, `partition_by='toYear(trade_date)'` |
| `int_stock_basic_snapshot` | `table`, `order_by='security_code'` |
| `int_trade_calendar` | `table`, `order_by='trade_date'` |

**建议**：显式声明，与其他同类 snapshot/calendar 模型对齐。

#### IA2. stock/index grain `order_by` 主键顺序统一

int 层日频行情/复权表已统一用 `(trade_date, security_code)`，但 mart 层 `mart_stock_quotes_daily` 用 `(security_code, trade_date)`。int→mart 透传时 order_by 不一致虽然不影响正确性，但影响查询计划和分区裁剪一致性。

**决策（2026-06-21）**：int 和 mart 两层 stock daily grain 统一为 `(trade_date, security_code)`，理由是下游以日期扫描为主，日期在前有利于分区裁剪和按日期范围扫描的查询计划。

- **int 层**：当前已是 `(trade_date, security_code)`，无需改动。✅
- **mart 层**：`mart_stock_quotes_daily` 和 `mart_benchmark_returns_daily` 当前为 `(security_code, trade_date)`，需改为 `(trade_date, security_code)`。见 `0003-2026-06-21-mart-field-style-consistency.md` A2。

### IB. 字段命名口径（中优先级，与 mart 层同源）

#### IB1. BOLL 上下轨缩写对称化（= mart A3）

`boll_up_*` → `boll_upper_*`，`boll_dn_*` → `boll_lower_*`。int 层先改，mart 层透传对齐。

#### IB2. 百分比口径 `_pct` 后缀统一（= mart A4）

**决策（2026-06-21）**：采用 `_pct` 后缀方案，与项目现有主流惯例（staging `free_float_holdnum_ratio_pct`、`change_ratio_pct`，政府债 `*_yield_pct`）对齐。

- 百分数口径：统一加 `_pct` 后缀 → `turnover_rate_pct`, `turnover_rate_free_float_pct`, `dy_static_pct`, `dy_ttm_pct`，并将现有前缀特例 `pct_amplitude`→`amplitude_pct`、`pct_change`→`change_pct`
- 小数比例口径：不加后缀，YAML description 显式标注"比率口径，不乘以 100"（`roe`/`roa`/`roaa`/`roae`/`annual_rate`/`daily_rate`/`return_daily` 已符合）

**理由**：后缀方案与项目 14 个已有字段（staging 2 个 + 政府债 12 个）一致，无需改动政府债字段；主词在前更符合自然阅读；消除了 `pct_amplitude`/`pct_change` 两个前缀特例。

#### IB3. 去除 `a_` 前缀（= mart A5）

`a_market_cap` → `market_cap`，`a_shares` → `shares`，`a_float_shares` → `float_shares`，`a_free_float_shares` → `free_float_shares`，`a_float_market_cap` → `float_market_cap`，`a_free_float_market_cap` → `free_float_market_cap`。

**注意**：`int_stock_shares_history` 中 `a_shares` 是 derived（`a_float_shares + limited_a_shares`），`a_float_shares` 是 alias from upstream `listed_a_shares`。改名需同步 staging 层 alias 映射和 `int_stock_financial_valuation`（引用 `a_shares`/`total_shares`）。

**语义评估（2026-06-21）**：去除 `a_` 前缀是安全的，语义不会丢失。

- **`a_` 前缀在 intermediate/mart 层无实际区分价值**：消费链路（`int_stock_quotes_daily_unadj` → `mart_stock_quotes_daily`）的 universe 已通过 `security_type = 'stock'` + A 股板块（`sse_main_board`/`szse_main_board`/`chinext`/`star_market`）过滤锁定为纯 A 股域，同表内不存在平行的 B 股/H 股市值字段，无混淆风险。
- **多市场区分价值存在于 staging 层**：`stg_eastmoney__equity_history` 透传的 `listed_a_shares`/`limited_a_shares`/`b_free_share`/`h_free_share` 是源端多市场结构，承载市场区分语义，**去前缀不触碰 staging 层**，这些字段保留 `a_`/`b_`/`h_` 标记。
- **`int_stock_shares_history` 的 `a_` 字段确实只统计 A 股口径**：派生时已 `where shares_type = 'A股'` 过滤大股东扣减，只取 `listed_a_shares`，系统性排除了 B/H 股。语义准确，但区分价值由上游 `listed_a_*` 命名承载，intermediate 的 `a_` 前缀是冗余的二次标记。
- **市场域是库级属性**：`field_glossary.yml` 已将整库定义为 A 股域（`security_code` regex 只接受 `SH/SZ/BJ`，`trade_date`/`exchange_code` 均标注 A 股口径），市场域不必下沉到每个度量字段名。

**⚠️ 语义对照点（去前缀后必须在 description 中明确）**：`int_stock_shares_history` 同时输出 `total_shares`（全市场总股本，含 B/H）和 `a_shares`（A 股股本 = 流通 + 限售）。去前缀后 `shares` 与 `total_shares` 并存，仅看字段名可能误以为 `shares` 就是总股本。YAML description 必须明确：
- `shares`：A 股股本（已上市流通 A 股 + 限售 A 股）
- `total_shares`：全市场总股本（含 A/B/H 等所有股份类别）

**影响面（6 个文件，int 层 4 + mart 层 2）**：

| 文件 | 改动 |
|---|---|
| `int_stock_shares_history.sql` | `a_shares`/`a_float_shares`/`a_free_float_shares` 定义改名，上游 alias `listed_a_shares` 保持不变 |
| `int_stock_shares_history.yml` | 同步列名和 description（明确与 `total_shares` 的对照） |
| `int_stock_quotes_daily_unadj.sql` | 引用改名后的 shares 字段；`a_market_cap`/`a_float_market_cap`/`a_free_float_market_cap` 派生改名 |
| `int_stock_quotes_daily_unadj.yml` | 同步 6 个列名 |
| `mart_stock_quotes_daily.sql` | 透传 6 个改名后的字段 |
| `mart_stock_quotes_daily.yml` | 同步 6 个列名 |

**不受影响**：staging 层（`stg_eastmoney__equity_history`）保留源端 `listed_a_*`/`b_*`/`h_*` 多市场标记；`int_stock_financial_valuation` 使用 `total_shares`（全市场口径），不引用 `a_` 前缀字段，无需改动。

#### IB4. `turnover_rate_actual` 改名（= mart A6）

`turnover_rate_actual` → `turnover_rate_free_float`。int 层先改，mart 透传。

### IC. YAML 文档完整性（高优先级）

#### IC1. portfolio 系列 YAML 补全列文档（= mart B2 上游）

四个 portfolio wrapper 的 YAML 严重缺失列文档，共缺 53 列。这是 int 层最严重的文档债务：

| 模型 | SQL 输出 | YAML 已文档化 | 缺失 |
|---|---|---|---|
| `int_portfolio_closed_trade` | 21 | 4 | 17（含 `security_code`, `realized_pnl`, `total_fee` 等核心字段） |
| `int_portfolio_performance_metric` | 25 | 7 | 18（含全部 12 个绩效指标值字段） |
| `int_portfolio_performance_metric_status` | 8 | 5 | 3（含 `security_code`, `window_key`——被 unique test 引用却未文档化） |
| `int_portfolio_trade_metric` | 18 | 3 | 15（含全部交易指标值字段） |

**建议**：补全所有列的 `name`/`data_type`/`description`，主键和 status 列加 `not_null`。这与 mart 层 B2 联动——int 补全后 mart rank 表的列文档也能引用上游描述。

### ID. 描述语言统一（低优先级）

#### ID1. 指标 wrapper 列描述语言统一

`int_stock_kdj_daily`、`int_stock_ma_daily`、`int_stock_rsi_daily`、`int_stock_price_pattern_daily` 的模型描述和 meta 是中文，但列描述是英文。`int_stock_boll_daily` 和 `int_stock_macd_daily` 列描述是中文。

**建议**：统一为中文（与 field_glossary 主线一致），或引用 `{{ doc('field_*') }}` 共享 doc。

#### ID2. portfolio 系列描述语言

portfolio 系列模型描述和列描述全为英文，而 grain_zh/business_logic_zh 为中文。

**建议**：模型描述和列描述统一改为中文，grain_zh/business_logic_zh 保留。

### IE. 其他一致性问题

#### IE1. `is_suspend` 在 index_quotes 中语义存疑（已细化为 IE1a/IE1b，见下）

#### IE1a. `int_index_quotes_daily` 不再输出 `is_suspend`

`int_index_quotes_daily` 当前输出 `is_suspend`（透传自 `stg_baostock__query_history_k_data_plus_daily`），但指数没有停牌概念，该字段无实际语义。

- **上游**：`stg_baostock__query_history_k_data_plus_daily.is_suspend`（BaoStock 行情字段，对指数恒为 false 或无意义）
- **下游消费**：`int_benchmark_returns_daily` 只选取 `security_code`/`trade_date`/`close_price`/`prev_close_price`/`return_daily`，不消费 `is_suspend`；`mart_benchmark_returns_daily` 同样不消费。
- **影响面**：仅需改动 `int_index_quotes_daily.sql`（删除 SELECT 中的 `is_suspend`）和 `int_index_quotes_daily.yml`（删除 `is_suspend` 列定义和 `not_null` 测试）。无下游引用，无破坏性影响。

**建议**：从 `int_index_quotes_daily` 的 SELECT 和 YAML 中移除 `is_suspend`。

#### IE2. 配置块 `partition_by` 和 `unique_key` 不统一

- snapshot 系列无 `partition_by`（合理，数据量小）
- `int_stock_shares_history` 无 `partition_by`（但 order_by 含 `effective_date`，可考虑 `toYear(effective_date)`）
- 所有 table 模型均无 `unique_key`，依赖 `unique_combination_of_columns` data_test 做事后校验

**建议**：table 物化的日频模型统一加 `partition_by='toYear(trade_date)'`（已有的一致）；考虑在 config 中加 `unique_key` 做增量物化保护（可选，低优先级）。

#### IE3. 政府债收益率字段命名风格

`three_month_yield_pct` 使用完整英文数字单词（`three`/`six`/`fifteen`/`twenty`/`thirty`），但 `one`/`two`/`five`/`seven`/`ten` 也是单词。整体一致但冗长。

**决策（2026-06-21）**：已关闭。采用 `_pct` 后缀方案后，政府债 `*_yield_pct` 系列已符合命名约定，无需改动。字段名虽冗长但语义清晰，维持现状。

#### IE4. 关键度量字段无范围测试（= mart D2）

`return_daily`、`pct_change`、RSI 值（`rsi_*` 理论上 [0,100]）等无范围约束。

**建议**：RSI 加 `accepted_range [0, 100]`；`return_daily`/`pct_change` 加合理区间告警测试。

#### IE5. `int_stock_shares_history.effective_date` 上游日期字段命名不一致且语义混淆

`effective_date` 是 union(equity_history 上游日期, freeholders 上游日期) 去重后的"股本区间生效起始日"，混合了两类不同语义的日期。经 raw 层数据验证（2026-06-21，样本 `600519.SH`）：

**freeholders.END_DATE = 报告期** ✅
实际取值为季度末：`2026-03-31`、`2025-12-31`、`2025-09-30`、`2025-06-30`、`2025-03-31`（标准财报报告期），偶尔夹杂非季末日期如 `2025-11-19`（临时公告/权益分派报告期）。契约描述为"前十大流通股东名单对应的报告期截止日期"。

**equity_history.END_DATE = 股本变动截止日** ❌（非报告期）
实际取值为股本变动发生日：`2026-05-28`（回购）、`2015-07-17`（送股上市）、`2009-05-25`（股改限售流通股上市），与季报报告期无关。契约描述为"股本变动截止日"。

**现状问题**：
- `stg_eastmoney__equity_history` 把 raw `END_DATE` 命名为 `report_date`（复用 `field_report_date` glossary），但实际语义是"股本变动截止日"，**命名与事实不符**，误导下游。
- `stg_eastmoney__freeholders` 把 raw `END_DATE` 命名为 `end_date`，语义是报告期，但命名未体现"报告期"语义。
- 两个 stg 模型对同类日期字段命名不一致（`report_date` vs `end_date`），且 equity_history 的 `report_date` 命名是错的。

**建议（分两步）**：
1. **stg 层纠正命名**：
   - `stg_eastmoney__equity_history`：`report_date` → `end_date`（股本变动截止日），不复用 `field_report_date` glossary，description 明确"股本变动截止日，非财报报告期"。
   - `stg_eastmoney__freeholders`：`end_date` → `report_date`（报告期），复用 `field_report_date` glossary，description 明确"前十大流通股东报告期"。
2. **int 层 `effective_date` 保留**：它是 union 去重后的区间生效起始日，是合理的派生语义，不等于任何单一上游的"报告期"。但需同步更新：
   - `int_stock_shares_history.sql` CTE 中 `equity_history` 的 `report_date as end_date` 改为 `end_date as end_date`；`freeholders` 的 `end_date` 改为 `report_date as end_date`。
   - `int_stock_shares_history.yml` 的 `source_equity_end_date` description 保持"股本变动截止日"；`source_freeholders_end_date` description 保持"A 股流通股东报告期"。

**影响面**：
- `stg_eastmoney__equity_history.sql` + `.yml`（rename `report_date` → `end_date`）
- `stg_eastmoney__freeholders.sql` + `.yml`（rename `end_date` → `report_date`）
- `int_stock_shares_history.sql`（同步 CTE alias）
- 检查是否有其他模型引用这两个 stg 字段（grep 确认）

---

## 落地顺序建议

int 层先于 mart 层整改，因为 int 是上游，mart 透传对齐：

1. **第一批（int 机械重命名 + config 补全）**：IA1（config 补全）、IA2（order_by 统一）、IB1（boll upper/lower）、IB3（去 `a_` 前缀）、IB4（`turnover_rate_free_float`）
2. **第二批（int 口径信号）**：IB2（`_pct` 后缀统一，含 `pct_amplitude`/`pct_change` 改为后缀）—— 政府债已符合，无需改动
3. **第三批（int 文档补全）**：IC1（portfolio YAML 补列）、ID1/ID2（描述语言统一）
4. **第四批（int 测试加固 + 语义确认）**：IE1（is_suspend 语义）、IE2（partition/unique_key）、IE4（范围测试）
5. **第五批（mart 层对齐）**：参照 `0003-2026-06-21-mart-field-style-consistency.md` 落地顺序，mart 透传 int 改动后的字段名

每批落地后运行：
```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --select <affected_models> --project-dir elt --profiles-dir elt
```

## 相关位置

- `pipeline/elt/models/intermediate/*.sql`
- `pipeline/elt/models/intermediate/*.yml`
- `pipeline/elt/metadata/field_glossary.yml`
- 配套文档：`docs/debt/archive/0003-2026-06-21-mart-field-style-consistency.md`
- 分支：`refactor/int-mart-field-consistency`
