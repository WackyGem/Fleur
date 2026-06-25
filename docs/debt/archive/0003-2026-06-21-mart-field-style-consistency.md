# Debt: Marts 层字段风格与一致性收敛

日期：2026-06-21

## 执行状态

状态更新：2026-06-21。本文档为 marts 层字段风格与一致性梳理的债务登记，尚未落地任何改动。

| # | 债务 | 优先级 | 状态 | 证据 |
|---|------|--------|------|------|
| A1 | 日频 mart 表名 `_daily` 后缀不统一 | 高 | 已关闭 | `mart_stock_momentum_indicator`、`mart_stock_trend_indicator`、`mart_stock_volume_indicator` 缺 `_daily` 后缀。 |
| A2 | stock grain 的 `order_by` 主键顺序不一致 | 高 | 已关闭 | quotes 为 `(security_code, trade_date)`，momentum/trend/volume/price_pattern 为 `(trade_date, security_code)`。 |
| A3 | BOLL 上下轨缩写不对称 | 中 | 已关闭 | 上轨 `boll_up_*`，下轨 `boll_dn_*`。 |
| A4 | 百分比口径字段无统一信号 | 中 | 已关闭 | `pct_change`/`pct_amplitude` 带 `pct_` 前缀，`turnover_rate`/`dy_*` 不带但同为百分数；`roe`/`roa` 为小数比例。 |
| A5 | `a_` 前缀语义模糊（过度设计） | 中 | 已关闭 | `a_market_cap`、`a_shares` 等，整表已是 A 股域。 |
| A6 | `turnover_rate_actual` 命名含糊 | 中 | 已关闭 | "actual" 语义不清，实际为自由流通股本分母。 |
| B1 | KDJ 参数字段只在部分 mart 暴露 | 中 | 已关闭 | momentum 暴露 `kdj_rsv_window/k_smoothing/d_smoothing`，quotes 的 KDJ 部分无参数字段。 |
| B2 | portfolio rank YAML 列文档不完整 | 中 | 已关闭 | `mart_portfolio_performance_metric_rank.yml` 只文档化 3 列，SQL 输出 13 列；trade rank 同样只文档化 3 列，SQL 输出 9 列。 |
| C1 | 描述语言中英文混用 | 低 | 已关闭 | benchmark/risk_free/quotes 中文，portfolio rank/momentum/price_pattern/trend/volume 英文。 |
| D1 | `not_null` 测试覆盖不均 | 低 | 已关闭 | portfolio rank 主键列未加 `not_null`。 |
| D2 | 关键度量字段无范围测试 | 低 | 已关闭 | `return_daily`、`pct_change` 等无范围约束。 |

## 背景

`refactor/int-mart-field-consistency` 分支目标是调整 int 和 mart 层数据字段风格和一致性。本文档先冻结 marts 层现状清单与整理建议，作为后续改动的基线。marts 层共 9 张表，按业务域分为行情宽表、指标、基准/无风险利率、组合绩效排名四组。

字段命名 canonical 规则见 `pipeline/elt/metadata/field_glossary.yml`：`canonical_field_pattern: "^[a-z][a-z0-9_]*$"`，且字段描述以中文 `description_zh` 为事实源。

## Marts 层表清单

### 1. 行情宽表域

#### `mart_stock_quotes_daily` — A 股日频行情宽表

- **grain**: `(security_code, trade_date)`
- **order_by**: `(security_code, trade_date)` · **partition_by**: `toYear(trade_date)`
- **字段**（52 列）：

| 分组 | 字段 |
|---|---|
| 主键 | `security_code`, `trade_date` |
| 未复权 OHLC | `open_price`, `high_price`, `low_price`, `close_price` |
| 前收 | `prev_close_price`, `prev_close_price_unadj` |
| 前复权 OHLC | `open_price_forward_adj`, `high_price_forward_adj`, `low_price_forward_adj`, `close_price_forward_adj`, `prev_close_price_forward_adj` |
| 后复权 OHLC | `open_price_backward_adj`, `high_price_backward_adj`, `low_price_backward_adj`, `close_price_backward_adj`, `prev_close_price_backward_adj` |
| 复权因子 | `forward_adjustment_factor`, `forward_adjustment_ratio`, `backward_adjustment_factor`, `backward_adjustment_ratio` |
| 成交 | `prev_volume`, `volume`, `amount` |
| 换手/振幅/涨跌 | `turnover_rate`, `turnover_rate_actual`, `pct_amplitude`, `pct_change` |
| 涨跌停 | `limit_up_price`, `limit_down_price` |
| 市值 | `a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap` |
| 股本 | `a_shares`, `a_float_shares`, `a_free_float_shares` |
| 估值（as-of） | `pe_static`, `pe_ttm`, `pe_forecast`, `pb_mrq`, `book_value_per_share`, `roe`, `roa`, `roaa`, `roae` |
| 股息率 | `dy_static`, `dy_ttm` |
| 状态 | `is_suspend`, `is_st` |
| KDJ | `kdj_rsv`, `kdj_k_value`, `kdj_d_value`, `kdj_j_value` |

### 2. 指标域

以下四张表 grain 均为 `(security_code, trade_date)`。

#### `mart_stock_momentum_indicator` — 动量指标（RSI + KDJ）

- **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **字段**：`security_code`, `trade_date`, `rsi_6`, `rsi_12`, `rsi_14`, `rsi_24`, `rsi_25`, `rsi_50`, `kdj_rsv_window`, `kdj_k_smoothing`, `kdj_d_smoothing`, `kdj_rsv`, `kdj_k_value`, `kdj_d_value`, `kdj_j_value`

#### `mart_stock_trend_indicator` — 趋势指标（MA + BOLL + MACD）

- **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **字段**：`security_code`, `trade_date`, `price_ma_3/5/6/10/12/14/20/24/28/30/57/60/114/250`, `price_avg_ma_3_6_12_24`, `price_avg_ma_14_28_57_114`, `price_ema2_10`, `boll_mid_10_1p5`, `boll_up_10_1p5`, `boll_dn_10_1p5`, `boll_mid_20_2`, `boll_up_20_2`, `boll_dn_20_2`, `boll_mid_50_2p5`, `boll_up_50_2p5`, `boll_dn_50_2p5`, `macd_dif`, `macd_dea`, `macd_histogram`

#### `mart_stock_volume_indicator` — 均量指标

- **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **字段**：`security_code`, `trade_date`, `volume_ma_5`, `volume_ma_10`, `volume_ma_20`, `volume_ma_60`

#### `mart_stock_price_pattern_daily` — 价格行为与结构

- **order_by**: `(trade_date, security_code)` · **partition_by**: `toYear(trade_date)`
- **字段**：`security_code`, `trade_date`, `close_direction`, `close_up_streak_days`, `close_down_streak_days`, `n_structure_20_valid_bars`, `n_structure_20_high_date`, `n_structure_20_high_price`, `n_structure_20_low_date`, `n_structure_20_low_price`, `n_structure_20_second_low_date`, `n_structure_20_second_low_price`, `n_structure_20_second_low_ratio`, `n_structure_20_is_valid`

### 3. 基准 / 无风险利率域

#### `mart_benchmark_returns_daily` — 基准日收益

- **grain**: `(security_code, trade_date)` · **order_by**: `(security_code, trade_date)` · **partition_by**: `toYear(trade_date)`
- **字段**：`security_code`, `trade_date`, `return_daily`

#### `mart_risk_free_rate_daily` — 无风险利率

- **grain**: `(trade_date, source_tenor)` · **order_by**: `(source_tenor, trade_date)` · **partition_by**: `toYear(trade_date)`
- **字段**：`trade_date`, `source_date`, `source_tenor`, `annual_rate`, `daily_rate`

### 4. 组合绩效排名域（snapshot，无 partition）

#### `mart_portfolio_performance_metric_rank` — 绩效指标排名

- **grain**: `(portfolio_run_id, result_attempt_id, security_code, window_key, metric_name)`
- **order_by**: `(config_hash, security_code, window_key, metric_name, metric_rank, portfolio_run_id, result_attempt_id)`
- **字段**：`portfolio_run_id`, `result_attempt_id`, `source_run_id`, `security_code`, `window_key`, `window_start`, `window_end`, `config_hash`, `metric_name`, `metric_value`, `rank_direction`, `metric_rank`, `reason_code`
- ⚠️ YAML 只文档化了 3 列（`metric_rank`, `metric_name`, `rank_direction`），其余 10 列未在 YAML 列出。

#### `mart_portfolio_trade_metric_rank` — 交易质量指标排名

- **grain**: `(portfolio_run_id, result_attempt_id, window_key, metric_name)`
- **order_by**: `(window_key, metric_name, metric_rank, portfolio_run_id, result_attempt_id)`
- **字段**：`portfolio_run_id`, `result_attempt_id`, `window_key`, `window_start`, `window_end`, `metric_name`, `metric_value`, `rank_direction`, `metric_rank`
- ⚠️ YAML 只文档化了 3 列，其余 6 列未列出。

## 整理建议

### A. 命名风格不一致（高优先级）

#### A1. 表名 `_daily` 后缀不统一

同样是日频 grain，但后缀不一致：

| 表名 | 有 `_daily` |
|---|---|
| `mart_stock_quotes_daily` | ✅ |
| `mart_stock_price_pattern_daily` | ✅ |
| `mart_benchmark_returns_daily` | ✅ |
| `mart_risk_free_rate_daily` | ✅ |
| `mart_stock_momentum_indicator` | ❌ |
| `mart_stock_trend_indicator` | ❌ |
| `mart_stock_volume_indicator` | ❌ |

**建议**：日频 grain 的 mart 统一加 `_daily` 后缀（`mart_stock_momentum_indicator_daily` 等）；snapshot 类（portfolio rank）不加。这样从表名即可识别频率。

#### A2. ClickHouse `order_by` 主键顺序不一致

同样是 `(security_code, trade_date)` grain，但顺序相反：

| 表 | order_by |
|---|---|
| `mart_stock_quotes_daily` | `(security_code, trade_date)` |
| `mart_benchmark_returns_daily` | `(security_code, trade_date)` |
| `mart_stock_momentum_indicator` | `(trade_date, security_code)` |
| `mart_stock_trend_indicator` | `(trade_date, security_code)` |
| `mart_stock_volume_indicator` | `(trade_date, security_code)` |
| `mart_stock_price_pattern_daily` | `(trade_date, security_code)` |

**决策（2026-06-21）**：stock grain 统一为 `(trade_date, security_code)`，理由是下游以日期扫描为主，日期在前有利于分区裁剪和按日期范围扫描的查询计划。

- **需改动的 mart 表**：`mart_stock_quotes_daily`、`mart_benchmark_returns_daily`（当前为 `(security_code, trade_date)`）
- **已是 `(trade_date, security_code)` 的 mart 表**：momentum、trend、volume、price_pattern ✅
- int 层已是 `(trade_date, security_code)`，无需改动。见 `0002-2026-06-21-int-field-style-consistency.md` IA2。

#### A3. BOLL 上下轨缩写不对称

- 上轨 `boll_up_*`（full word）
- 下轨 `boll_dn_*`（abbreviated）

**建议**：统一为 `boll_upper_*` / `boll_lower_*`（或全用 `up`/`dn`，但 full word 更易读）。

#### A4. 百分比口径字段无统一信号

| 字段 | 是否带 `_pct` 后缀 | 实际口径 |
|---|---|---|
| `pct_amplitude`, `pct_change` | ❌（前缀特例） | 百分数 |
| `turnover_rate`, `turnover_rate_actual` | ❌ | 百分数 |
| `dy_static`, `dy_ttm` | ❌ | 百分数 |
| `roe`, `roa`, `roaa`, `roae` | ❌ | 比率（小数） |

**决策（2026-06-21）**：采用 `_pct` 后缀方案，与项目现有主流惯例（staging `free_float_holdnum_ratio_pct`、政府债 `*_yield_pct`）对齐。
- 百分数口径：统一加 `_pct` 后缀（如 `turnover_rate_pct`, `dy_ttm_pct`），并将前缀特例 `pct_amplitude`→`amplitude_pct`、`pct_change`→`change_pct`
- 小数比例口径：不加后缀，但在 YAML description 显式标注"比率口径，不乘以 100"

当前 `roe` 等已在 description 注明比率口径，但 `turnover_rate` 描述说"百分数口径"却无 `_pct` 后缀，最易误导。详见 `0002-2026-06-21-int-field-style-consistency.md` IB2。

#### A5. `a_` 前缀语义模糊（过度设计）

`a_market_cap`、`a_shares` 等，`a_` 指"A 股"，但整张表本身就是 A 股域。

**建议**：去掉 `a_` 前缀，直接用 `market_cap` / `float_market_cap` / `free_float_market_cap` / `shares` / `float_shares` / `free_float_shares`。若未来有 H 股混表需求再考虑前缀，当前是过度设计。

**语义评估（2026-06-21）**：去除 `a_` 前缀安全可行。消费链路 universe 已过滤为纯 A 股域，同表无平行非 A 股市值字段；多市场区分由 staging 层 `listed_a_*`/`b_*`/`h_*` 命名承载，mart 层透传不涉及。详见 `0002-2026-06-21-int-field-style-consistency.md` IB3 的完整评估和影响面清单（6 个文件，int 层先改，mart 透传对齐）。

**⚠️ 注意**：去前缀后 `shares`（A 股股本）与 `total_shares`（全市场总股本，仅存在于 `int_stock_shares_history`，mart 层不输出 `total_shares`）的对照需在 int 层 description 中明确。mart 层只需透传改名后的字段。

#### A6. `turnover_rate_actual` 命名含糊

"actual" 语义不清，实际含义是"以自由流通股本为分母"。

**建议**：改名为 `turnover_rate_free_float`，与 `a_free_float_*` 系列语义对齐，含义自解释。

### B. 字段暴露不一致（中优先级）

#### B1. KDJ 参数字段只在部分 mart 暴露

- `mart_stock_momentum_indicator` 暴露了 `kdj_rsv_window`, `kdj_k_smoothing`, `kdj_d_smoothing`
- `mart_stock_quotes_daily` 的 KDJ 部分只有值字段，无参数字段

**建议**：KDJ 参数属"口径元数据"，要么在所有含 KDJ 的 mart 都暴露（便于自验），要么都不暴露（统一收敛到 `int_stock_kdj_daily` wrapper 文档）。推荐后者：mart 只暴露值，参数口径由 `accepted_values` 测试 + upstream 文档保证，避免重复。

#### B2. portfolio rank YAML 列文档不完整

两张 rank 表 YAML 只写了 3 列，SQL 实际输出 9–13 列，主键字段（`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key` 等）和 `metric_value`, `reason_code`, `window_start/end` 都未文档化。

**建议**：补全 YAML columns，至少覆盖主键、`metric_value`、`rank_direction`、`reason_code`、`window_start/end`、`config_hash`，并加 `not_null` 测试。

### C. 描述语言不统一（低优先级）

#### C1. 中英文混用

| 表 | description 语言 |
|---|---|
| `mart_benchmark_returns_daily` | 中文 |
| `mart_risk_free_rate_daily` | 中文 |
| `mart_stock_quotes_daily` | 中文为主 |
| `mart_portfolio_*_rank` | 英文 |
| `mart_stock_momentum_indicator` | 英文 |
| `mart_stock_price_pattern_daily` | 英文 |
| `mart_stock_trend_indicator` | 英文 |
| `mart_stock_volume_indicator` | 英文 |

**建议**：统一为中文（与 `field_glossary.yml` 的 `description_zh` 主线一致），或统一英文。考虑项目 AGENTS.md 和 glossary 都以中文为事实源，推荐**统一中文**。字段级 description 也建议引用 `{{ doc('field_*') }}` 共享 doc，减少重复。

### D. 测试覆盖一致性（低优先级）

#### D1. `not_null` 测试覆盖不均

- `mart_stock_quotes_daily` 只对 `is_suspend` 加了 `not_null`，但 `security_code`/`trade_date` 也有 not_null 测试 ✅
- portfolio rank 表的 `portfolio_run_id`, `result_attempt_id`, `window_key` 等主键未加 `not_null`

**建议**：所有主键列、`metric_rank`、`rank_direction` 统一加 `not_null`。

#### D2. 关键度量字段无范围测试

`return_daily`、`pct_change` 等关键度量字段无合理区间约束。

**建议**：对 `return_daily`、`pct_change` 等加 `dbt_utils.accepted_range` 或自定义 test 约束合理区间（如 `pct_change` 在 [-20, 20] 内为正常，超限告警）。

## 落地顺序建议

1. **第一批（机械重命名，影响面大但安全）**：A1（表名 `_daily`）、A2（order_by 统一）、A3（boll upper/lower）、A5（去 `a_` 前缀）、A6（`turnover_rate_free_float`）
2. **第二批（口径信号）**：A4（`_pct` 后缀统一，含 `pct_amplitude`/`pct_change` 改为后缀）—— mart 透传 int 层改动
3. **第三批（文档补全）**：B1（KDJ 参数收敛）、B2（portfolio YAML 补列）、C1（描述语言统一）
4. **第四批（测试加固）**：D1、D2

每批落地后运行 dbt parse、`validate_field_glossary.py` 和定向 `dbt build --select` 验证，并同步更新本文档执行状态表。

## 相关位置

- `pipeline/elt/models/marts/*.sql`
- `pipeline/elt/models/marts/*.yml`
- `pipeline/elt/metadata/field_glossary.yml`
- 分支：`refactor/int-mart-field-consistency`
