# Q&A 0002: Portfolio Metrics 基础数据缺口

状态：Proposed

日期：2026-06-16

## 问题

需要计算组合绩效指标，包括区间收益率、年化收益率、年化波动率、夏普比率、索提诺比率、最大回撤、卡尔玛比率、信息比率、下行波动率、Alpha、Beta 和特雷诺比率。基于当前 mono-fleur 的组合运行结果，还欠缺哪些基础数据？

## 结论

当前已经具备“绝对收益和绝对风险”所需的核心净值序列，但这些数据仍在 PostgreSQL 第一版结果表中，尚未按 Q&A 0001 的方向进入 ClickHouse portfolio data plane。

当前最主要缺口是：

1. benchmark 日频收益率序列。
2. risk-free rate 日频序列和期限选择。
3. 指标计算配置：年化天数、收益率口径、MAR、benchmark 选择、日期对齐规则。
4. ClickHouse 中的 portfolio result fact tables 和 result attempt 维度。
5. 闭仓交易级 realized PnL / realized return，用于胜率、盈亏比等交易质量指标。
6. 组合 run 维度快照落到 ClickHouse，支持跨策略、跨版本、跨参数分析。

不补齐 benchmark 和 risk-free rate 时，只能可靠计算区间收益率、年化收益率、年化波动率、最大回撤、卡尔玛比率和 MAR=0 的索提诺比率；夏普、Alpha、Beta、特雷诺和信息比率都只能作为占位或使用临时假设，不能作为权威指标。

## 当前已具备的数据

### 组合净值序列

当前 Rust 计算层已经生成每日 NAV 行，字段定义见 `PortfolioNavRow`：

- `trade_date`
- `cash_balance`
- `position_market_value`
- `total_equity`
- `nav`
- `daily_return`
- `drawdown`
- `gross_exposure`
- `position_count`
- `turnover`
- `fee_amount`
- `warning_count`

当前 PostgreSQL migration 中 `portfolio_nav` 也已经持久化这些字段。它足以支撑：

- 区间收益率。
- 年化收益率。
- 年化波动率。
- 最大回撤。
- 卡尔玛比率。
- MAR=0 的下行波动率和索提诺比率。
- 基础费用和换手率统计。

### 每日持仓快照

当前 `portfolio_position_day` 已有：

- `security_code`
- `quantity`
- `cost_basis`
- `average_entry_price`
- `close_price`
- `market_value`
- `unrealized_pnl`
- `unrealized_return`
- `holding_days`
- `is_stale_price`

这些字段足以支持持仓暴露、未实现收益、持仓天数和简单持仓分布分析，但还不足以做完整归因。

### 虚拟成交

当前 `portfolio_trade` 已有：

- `trade_date`
- `signal_date`
- `security_code`
- `side`
- `quantity`
- `reference_price`
- `execution_price`
- `gross_amount`
- `commission`
- `stamp_duty`
- `transfer_fee`
- `total_fee`
- `slippage_cost`
- `reason`

这些字段足以汇总成交金额、费用和滑点，但尚不足以直接计算闭仓交易胜率，因为缺少买卖 lot 的配对关系和 realized PnL。

## 指标数据依赖矩阵

| 指标 | 当前是否可权威计算 | 已有基础数据 | 欠缺基础数据 |
|---|---:|---|---|
| 区间收益率 | 可算 | `portfolio_nav.nav` 或 `daily_return` | 需要迁移到 ClickHouse 结果事实层 |
| 年化收益率 | 基本可算 | `daily_return`、起止日期 | 年化天数配置、有效交易日计数口径 |
| 年化波动率 | 可算 | `daily_return` | 最小样本数和空值处理规则 |
| 最大回撤 | 可算 | `portfolio_nav.drawdown` 或 `nav` | 回撤正负号展示规则 |
| 卡尔玛比率 | 基本可算 | 年化收益率、最大回撤 | 最大回撤为 0 时的返回规则 |
| 下行波动率 | 基本可算 | `daily_return` | MAR 配置，默认是 0 还是 risk-free daily |
| 索提诺比率 | 部分可算 | `daily_return` | MAR 配置；如 MAR=无风险利率，则还缺 risk-free rate |
| 夏普比率 | 不应权威计算 | `daily_return` | risk-free rate 日频序列和期限选择 |
| 信息比率 | 不应权威计算 | `daily_return` | benchmark 日频收益率、benchmark 选择和日期对齐规则 |
| Beta | 不应权威计算 | `daily_return` | benchmark 日频收益率、同日对齐后的协方差样本 |
| Alpha | 不应权威计算 | `daily_return` | benchmark 日频收益率、risk-free rate、Alpha 口径 |
| 特雷诺比率 | 不应权威计算 | `daily_return` | Beta、risk-free rate、Beta 为 0 的处理规则 |
| 成交胜率 | 不应权威计算 | `portfolio_trade` | 闭仓 lot ledger、realized PnL、realized return |
| 盈亏比 | 不应权威计算 | `portfolio_trade` | 闭仓交易配对和 realized PnL |
| 持仓归因 | 不应权威计算 | `portfolio_position_day` | benchmark 成分权重、行业分类快照、组合权重、收益贡献拆分 |

## 必须补齐的数据集

### 1. ClickHouse Portfolio Result Facts

按 Q&A 0001，组合结果事实应迁移到 ClickHouse。至少需要：

| 表 | 必要字段 |
|---|---|
| `portfolio_nav_daily` | `portfolio_run_id`, `result_attempt_id`, `trade_date`, `nav`, `daily_return`, `drawdown`, `total_equity`, `cash_balance`, `position_market_value`, `turnover`, `fee_amount` |
| `portfolio_position_day` | `portfolio_run_id`, `result_attempt_id`, `trade_date`, `security_code`, `quantity`, `market_value`, `cost_basis`, `unrealized_pnl`, `unrealized_return`, `holding_days` |
| `portfolio_trade` | `portfolio_run_id`, `result_attempt_id`, `trade_seq`, `trade_date`, `security_code`, `side`, `quantity`, `execution_price`, `gross_amount`, `total_fee`, `slippage_cost`, `reason` |
| `portfolio_run_snapshot` | `portfolio_run_id`, `result_attempt_id`, `source_run_id`, `rule_version_id`, `rule_hash`, `start_date`, `end_date`, `initial_cash`, `price_basis`, `created_at` |
| `fleur_calculation.calc_portfolio_performance_metric` | `portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`, `window_start`, `window_end`, 12 个核心指标宽表列、`computed_at`, `config_hash` |

`result_attempt_id` 是关键字段。没有它，重算、口径变更和历史审计会混在同一个 `portfolio_run_id` 下。

### 2. Benchmark Return Daily

Alpha、Beta、信息比率和相对收益都需要 benchmark 日频收益率。当前仓库没有稳定的指数行情 mart；现有 `int_stock_basic_snapshot` 虽能标记 `security_type = 'index'`，但当前行情模型以股票 universe 为主，并不等价于 benchmark 数据集。

建议新增：

| 字段 | 说明 |
|---|---|
| `benchmark_code` | 例如 `000300.SH`、`000905.SH`、`000852.SH` |
| `benchmark_name` | 沪深300、中证500、中证1000 等 |
| `trade_date` | 交易日 |
| `close_price` | 收盘点位 |
| `prev_close_price` | 前收盘点位 |
| `daily_return` | 日收益率 |
| `price_basis` | 指数点位或总收益指数口径 |
| `source` | 数据来源 |

如果要计算更严谨的 Alpha，优先使用总收益指数或明确说明普通价格指数不含分红。

### 3. Risk-Free Rate Daily

夏普、索提诺、Alpha 和特雷诺都需要无风险利率。当前仓库没有无风险利率数据源。

建议新增：

| 字段 | 说明 |
|---|---|
| `rate_date` | 日期 |
| `tenor` | 例如 `1Y`, `3M`, `ON` |
| `annual_rate` | 年化利率，小数口径 |
| `daily_rate` | 按配置折算后的日利率 |
| `day_count_basis` | 例如 `ACT/365`、`ACT/360` 或 `252_trading_days` |
| `source` | 数据来源 |

第一版可以使用固定年化无风险利率配置作为临时输入，但必须写入 `metric_config`，不能隐式写死在公式里。

### 4. Metric Config

指标口径必须作为数据保存，否则同一个指标值无法复现。

建议保存：

| 配置 | 默认建议 | 用途 |
|---|---|---|
| `annualization_days` | `252` | 年化收益、波动率、下行波动率 |
| `return_type` | `simple` | 简单收益或 log return |
| `risk_free_tenor` | `1Y` 或固定配置 | 夏普、Alpha、特雷诺 |
| `sortino_mar_type` | `zero` 或 `risk_free_daily` | 下行波动阈值 |
| `benchmark_code` | 策略或 run 指定 | Alpha、Beta、信息比率 |
| `alignment_policy` | `inner_join_trade_dates` | portfolio / benchmark / risk-free 日期对齐 |
| `min_observations` | 例如 `20` | 避免短窗口指标失真 |
| `zero_division_policy` | `null` | 波动率、Beta、MDD 为 0 时处理 |

### 5. Closed Trade Ledger

当前 `portfolio_trade` 是成交流水，不是闭仓交易结果。要计算胜率、盈亏比、平均盈利、平均亏损和单笔最大亏损，需要新增闭仓 ledger 或 lot ledger：

| 字段 | 说明 |
|---|---|
| `position_lot_id` | 开仓 lot 标识 |
| `entry_trade_seq` | 买入成交序号 |
| `exit_trade_seq` | 卖出成交序号 |
| `security_code` | 证券代码 |
| `entry_date` | 开仓日期 |
| `exit_date` | 平仓日期 |
| `quantity` | 平仓数量 |
| `entry_amount` | 开仓成本 |
| `exit_amount` | 平仓收入 |
| `total_fee` | 开平仓费用合计 |
| `realized_pnl` | 已实现盈亏 |
| `realized_return` | 已实现收益率 |
| `holding_days` | 持仓交易日数 |
| `exit_reason` | 卖出原因 |

没有该 ledger 时，只能用每日持仓的 `unrealized_return` 做持仓表现分析，不能替代闭仓胜率。

### 6. Calendar And Alignment

当前已有 `int_trade_calendar`，可以提供交易日和前一交易日。但绩效指标还需要更明确的对齐规则：

- 组合首日 `daily_return = NULL` 是否排除。
- portfolio、benchmark 和 risk-free 是否取交易日交集。
- benchmark 缺失时跳过、补前值还是失败。
- risk-free 非交易日是否前值填充到最近交易日。
- 年化交易日数用固定 252，还是用当年实际交易日数。

这些规则应写入 `metric_config`，并随 `fleur_calculation.calc_portfolio_performance_metric` 保存配置 hash。

## 第一阶段最小可交付

如果目标是先计算一版可用的核心指标，最小数据闭环是：

1. 将 `portfolio_nav` 等价数据写入 ClickHouse `portfolio_nav_daily`。
2. 新增 `portfolio_run_snapshot`，把 run 维度和 `result_attempt_id` 带入 ClickHouse。
3. 新增一个 benchmark 日频收益率表，先支持一个默认 benchmark，例如沪深300。
4. 新增 risk-free rate 配置或日频表，第一版允许固定年化值，但必须显式保存。
5. 新增 `metric_config` 和 `fleur_calculation.calc_portfolio_performance_metric`。

完成后可权威计算：

- 区间收益率。
- 年化收益率。
- 年化波动率。
- 最大回撤。
- 卡尔玛比率。
- 下行波动率。
- 索提诺比率。
- 夏普比率。
- 信息比率。
- Alpha。
- Beta。
- 特雷诺比率。

成交胜率、盈亏比和交易级指标应放在第二阶段，前提是补齐 closed trade ledger。

## 公式口径

以下定义基于日频收益率数据，默认采用 252 个交易日作为年化基数。

| 符号 | 含义 |
|---|---|
| `R_p` | 投资组合日收益率序列 |
| `R_b` | 业绩基准日收益率序列 |
| `R_f` | 无风险利率，年化或日频取决于配置 |
| `n` | 有效交易日样本数 |
| `MAR` | 最低可接受收益率，常用 0 或 risk-free daily |

### 区间收益率

```text
holding_period_return = product(1 + portfolio_daily_return) - 1
```

### 年化收益率

```text
annualized_return = (1 + holding_period_return) ^ (annualization_days / n) - 1
```

### 年化波动率

```text
annualized_volatility = stddevSamp(portfolio_daily_return) * sqrt(annualization_days)
```

### 夏普比率

```text
sharpe_ratio = (annualized_return - annual_risk_free_rate) / annualized_volatility
```

### 下行波动率和索提诺比率

```text
downside_deviation =
  sqrt(sum(pow(least(portfolio_daily_return - mar_daily, 0), 2)) / (n - 1)) * sqrt(annualization_days)

sortino_ratio = (annualized_return - annual_risk_free_rate) / downside_deviation
```

### 最大回撤和卡尔玛比率

```text
drawdown = nav / running_max(nav) - 1
max_drawdown = abs(min(drawdown))
calmar_ratio = annualized_return / max_drawdown
```

### 信息比率

```text
active_return_daily = portfolio_daily_return - benchmark_daily_return
information_ratio =
  annualized_active_return / (stddevSamp(active_return_daily) * sqrt(annualization_days))
```

### Beta 和 Alpha

```text
beta = covarSamp(portfolio_daily_return, benchmark_daily_return)
       / varSamp(benchmark_daily_return)

alpha = annualized_return
        - (annual_risk_free_rate + beta * (benchmark_annualized_return - annual_risk_free_rate))
```

### 特雷诺比率

```text
treynor_ratio = (annualized_return - annual_risk_free_rate) / beta
```

## 相关文档和事实来源

- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md)
- [engines/crates/rearview-core/src/portfolio/mod.rs](../../engines/crates/rearview-core/src/portfolio/mod.rs)
- [pipeline/migrate/versions/rearview/0003_create_rearview_portfolio_schema.py](../../pipeline/migrate/versions/rearview/0003_create_rearview_portfolio_schema.py)
