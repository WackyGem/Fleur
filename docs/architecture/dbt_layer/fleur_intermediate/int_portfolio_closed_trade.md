# int_portfolio_closed_trade 设计

状态：Design

依据：

- Portfolio source：`source('fleur_portfolio', 'portfolio_run_snapshot')`
- Calculation source：`source('fleur_calculation', 'calc_portfolio_closed_trade')`
- 组合 RFC：`docs/RFC/archive/0021-racingline-virtual-account-portfolio-rebalancing.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_portfolio_closed_trade.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_portfolio_closed_trade.yml`

## 1. 模型定位

已平仓交易明细 intermediate wrapper。模型透传 Rust worker 输出的 FIFO 批次配对行，并只派生 `total_fee` 和 `realized_return`，用于交易质量指标和组合运行分析。

本模型不重做 FIFO 撮合、不修正成交金额或费用、不读取 PostgreSQL 控制面业务表。只有能命中 `portfolio_run_snapshot` 的完整 result attempt 才进入本模型。

## 2. 数据粒度与依赖

- 粒度：每个 `closed_trade_id` 一行。
- 候选键：`closed_trade_id`。
- 物化：dbt view。
- Join 策略：`calc_portfolio_closed_trade` inner join `portfolio_run_snapshot`，过滤非完整或不可追溯 attempt。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `portfolio_run_id` | calculation | `String` | 组合 run 标识符。 |
| `result_attempt_id` | calculation | `String` | 仅追加的 result attempt 标识符。 |
| `closed_trade_id` | calculation | `String` | 已平仓交易行稳定标识符。 |
| `closed_trade_seq` | calculation | `UInt32` | 已平仓交易在同一 result attempt 内的序号，按退出顺序递增。 |
| `position_lot_id` | calculation | `String` | FIFO 批次标识，指向建仓成交形成的持仓批次。 |
| `entry_trade_seq` | calculation | `UInt32` | 建仓成交序号，对应 worker trade ledger。 |
| `exit_trade_seq` | calculation | `UInt32` | 平仓成交序号，对应 worker trade ledger。 |
| `security_code` | calculation | `String` | 股票标准连接代码。 |
| `entry_date` | calculation | `Date` | 建仓成交日期。 |
| `exit_date` | calculation | `Date` | 平仓成交日期。 |
| `quantity` | calculation | `Float64` | 本批次平仓数量，单位股。 |
| `entry_gross_amount` | calculation | `Float64` | 建仓成交金额，不含费用。 |
| `exit_gross_amount` | calculation | `Float64` | 平仓成交金额，不含费用。 |
| `entry_fee` | calculation | `Float64` | 建仓成交总费用。 |
| `exit_fee` | calculation | `Float64` | 平仓成交总费用。 |
| `total_fee` | `entry_fee + exit_fee` | `Float64` | 本笔已平仓交易的总费用。 |
| `realized_pnl` | calculation | `Float64` | 本笔已平仓交易实现盈亏金额。 |
| `realized_return` | `realized_pnl / (entry_gross_amount + entry_fee)` | `Nullable(Float64)` | 已平仓交易收益率；分母为 0 时为 NULL。 |
| `holding_days` | calculation | `UInt32` | 持仓天数。 |
| `exit_reason` | calculation | `String` | 平仓原因代码，如 signal、stop_loss、manual 等。 |
| `created_at` | calculation | `DateTime` | worker 写入时间。 |

## 4. 测试建议

- `closed_trade_id`: `not_null`。
- 组合键：`closed_trade_id` 唯一。
- `portfolio_run_id`, `result_attempt_id`, `security_code`: `not_null`。
- `security_code`: `cn_security_code_format`。
- 定向校验：`total_fee = entry_fee + exit_fee`，`realized_return` 分母为 0 时为 NULL。
