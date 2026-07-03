# RFC 0042: Racingline 交易胜率口径讨论

状态：Proposed
日期：2026-07-02
领域：Racingline, Rearview, Portfolio Metrics, ClickHouse
关联系统：app/racingline, engines/crates/rearview-core, fleur_portfolio, fleur_backtest, fleur_calculation
相关文档：
- docs/RFC/archive/0036-racingline-strategy-portfolio-statement.md
- docs/RFC/archive/0031-racingline-step4-step5-backtest-latency-slimming.md
- docs/Q&A/0002-portfolio-metrics.md
- docs/architecture/rearview.md
- docs/architecture/racingline.md

## 摘要

当前 Racingline 中“胜率”相关指标至少有三种口径：

| 名称 | 字段 | 当前用途 | 当前口径 |
| --- | --- | --- | --- |
| 日胜率 | `daily_win_rate` | Step 5 回测业绩侧栏、发布确认信息 | `daily_return > 0` 的净值观察日数 / 有 `daily_return` 的观察日数 |
| 已平仓交易胜率 | `win_rate_closed_trades` | portfolio trade metric calculation/dbt wrapper | `realized_return > 0` 的 closed trade row 数 / closed trade row 数 |
| 交易成功率 | `trade_win_rate` | 策略组合详情页账户对账单 summary | 区间内 `realized_pnl > 0` 的卖出 trade row 数 / 区间内卖出 trade row 数 |

本 RFC 聚焦用户语义中的“交易胜率”。当前前端详情页展示文案是“交易成功率”，字段是 `trade_win_rate`。它不是订单成交率，也不是 full-period closed lot 胜率；它是对账单所选日期范围内的卖出交易胜率。

需要讨论的问题是：这个口径是否应该继续作为用户可见的“交易胜率”，以及它和 `win_rate_closed_trades`、`daily_win_rate` 的命名、展示和存储边界是否需要收敛。

## 当前事实

### 对账单 `trade_win_rate`

当前策略组合详情页账户对账单 summary 展示：

```tsx
{
  label: "交易成功率",
  value: formatOptionalPercent(statement.summary.trade_win_rate),
  tone: signedTone(statement.summary.trade_win_rate),
}
```

来源：`app/racingline/src/routes/strategy-detail-page.tsx`。

后端 summary query 当前按所选区间计算：

```sql
SELECT if(count() = 0, null, countIf(realized_pnl > 0) / toFloat64(count()))
FROM (
  SELECT
    trade.trade_seq,
    trade.security_code,
    sum(coalesce(closed.realized_pnl, 0.0)) AS realized_pnl
  FROM live_trade AS trade
  LEFT JOIN live_closed_trade AS closed
    ON closed.strategy_portfolio_daily_run_id = trade.strategy_portfolio_daily_run_id
   AND closed.result_attempt_id = trade.result_attempt_id
   AND closed.exit_trade_seq = trade.trade_seq
   AND closed.security_code = trade.security_code
  WHERE lower(trade.side) = 'sell'
    AND trade.trade_date >= start_date
    AND trade.trade_date <= end_date
  GROUP BY trade.trade_seq, trade.security_code
)
```

来源：`engines/crates/rearview-core/src/clickhouse/mod.rs` 的 `statement_summary_sql()`。

因此当前口径是：

- 分母：所选区间内 `live_trade.side = sell` 的卖出成交行数。
- 分子：这些卖出成交行中，按 `live_closed_trade.exit_trade_seq = live_trade.trade_seq` 聚合后的 `sum(realized_pnl) > 0` 的行数。
- 同一卖出成交如果关闭多个 FIFO lot，先汇总到卖出成交行，再判断是否盈利。
- 区间内没有卖出成交时，结果为 `NULL`，不是 `0%`。
- 买入成交不进入分母。
- 未卖出的当前持仓不进入分子或分母。

### `win_rate_closed_trades`

计算层的 `win_rate_closed_trades` 当前来自 closed trade rows：

```rust
win_rate_closed_trades: if closed_trades.is_empty() {
    None
} else {
    Some(wins.len() as f64 / closed_trades.len() as f64)
}
```

其中 `wins` 由 `realized_return(row) > EPSILON` 判断。这个口径以 closed trade row 为单位，不以卖出成交行为单位。若一次卖出关闭多个 FIFO lot，这里可能计为多笔 closed trades；而对账单 `trade_win_rate` 只计为一笔卖出。

dbt intermediate 文档也确认：`win_rate_closed_trades = winning_trade_count / closed_trade_count`，分母为零时为 `NULL`。

### `daily_win_rate`

Step 5 回测 performance wrapper 的 `daily_win_rate` 当前来自 nav rows：

```rust
for row in nav {
    if let Some(daily_return) = row.daily_return {
        observation_count += 1;
        if daily_return > 0.0 {
            winning_day_count += 1;
        }
    }
}
```

它衡量的是净值上涨交易日占比，不是交易级胜率。

## 主要歧义

### 1. UI 文案和字段名不一致

页面文案是“交易成功率”，用户容易理解为订单成交成功率、下单成功率或信号命中率。字段名是 `trade_win_rate`，更接近交易胜率。两者都没有明示“只统计卖出形成的 realized PnL”。

### 2. 卖出成交行口径和 closed lot 口径会产生不同结果

如果一次卖出关闭多个买入 lot：

| entry lot | realized PnL |
| --- | ---: |
| lot A | `+100` |
| lot B | `-80` |

当前 `trade_win_rate` 会把这一笔卖出聚合为 `+20`，计为 1 笔胜利卖出。

`win_rate_closed_trades` 会把它拆成 2 个 closed rows，计为 1 胜 1 负，胜率为 `50%`。

两种口径都可以成立，但回答的问题不同：

- 卖出成交行口径回答：“这一次卖出操作整体是否赚钱？”
- closed lot 口径回答：“被关闭的持仓批次中，有多少批次赚钱？”

### 3. 区间维度和全周期维度不同

`trade_win_rate` 是对账单区间指标，随 `period_key` 和日期范围变化。

`win_rate_closed_trades` 当前是 calculation/dbt wrapper 的 `full_period` 指标。它可以扩展窗口，但现状不是账户对账单所选区间的直接来源。

### 4. PnL 金额胜负和收益率胜负不同

`trade_win_rate` 使用 `realized_pnl > 0` 判断胜负。

`win_rate_closed_trades` 使用 `realized_return > EPSILON` 判断胜负。大多数情况下方向一致，但在费用、极小值、成本基数或精度边界上，金额胜负和收益率胜负不是完全同一个定义。

### 5. 当前 SQL 对无 closed row 的卖出使用 `coalesce(..., 0.0)`

当前 query 是 `LEFT JOIN live_closed_trade`，并使用 `sum(coalesce(closed.realized_pnl, 0.0))`。在正常 FIFO worker 输出下，卖出应有对应 closed rows；但如果出现数据缺口，无 closed row 的卖出会进入分母并以 `0` PnL 计为非胜利。

这可以作为保守处理，也可能掩盖 ledger 缺失。是否应改为内连接、增加数据完整性检查，或在 summary 中暴露异常计数，需要单独决策。

## 待讨论方案

### 方案 A：保留当前口径，只改文案和说明

继续以区间内卖出成交行为单位计算 `trade_win_rate`，但把 UI 文案从“交易成功率”改为更明确的“卖出胜率”或“已实现交易胜率”。

建议文案：

| 位置 | 文案 |
| --- | --- |
| summary label | `卖出胜率` |
| tooltip | `区间内实现盈亏为正的卖出成交笔数 / 区间内卖出成交笔数；未卖出持仓不计入。` |

优点：

- 最贴近对账单用户视角：卖出操作是否整体赚钱。
- 不需要变更 ClickHouse calculation 表。
- 保留当前账户对账单按区间即时聚合的能力。

缺点：

- 与 `win_rate_closed_trades` 仍然存在两个交易胜率口径。
- 后续报表和导出必须持续说明“卖出成交行”粒度。

### 方案 B：对账单改用 closed lot 胜率

把账户对账单 summary 改为按 `live_closed_trade` rows 计算，和 `win_rate_closed_trades` 的单位对齐。

优点：

- 和 calculation/dbt wrapper 的指标命名更一致。
- 适合交易质量归因，能看到关闭批次层面的胜负比例。

缺点：

- 一次卖出拆多个 lot 时，用户看到的“交易笔数”和“胜率分母”不一致。
- 对账单操作记录以成交行为单位展示，summary 胜率却以 lot 为单位，容易在 UI 上解释不清。
- 需要额外明确 `realized_pnl` 还是 `realized_return` 作为胜负标准。

### 方案 C：同时暴露两个指标

账户对账单保留卖出胜率，同时在交易质量明细或诊断面板展示 closed lot 胜率。

建议字段：

| 字段 | 含义 |
| --- | --- |
| `sell_trade_win_rate` | 区间内盈利卖出成交行数 / 卖出成交行数 |
| `closed_lot_win_rate` | 区间内盈利 closed lot rows / closed lot rows |

优点：

- 避免把两个合法口径压成一个字段。
- 适合后续导出、诊断和专业用户解释。

缺点：

- UI 复杂度上升。
- 需要为两个指标建立长期命名规范和 API contract。

## 建议方向

建议采用方案 A 作为第一步：保留当前 `trade_win_rate` 的卖出成交行口径，但把用户可见文案改为“卖出胜率”或“已实现交易胜率”，并补充 tooltip。

理由：

1. 当前详情页账户对账单的其他字段也是区间 summary，例如交易笔数、交易股票数、盈利股票数和持股天数。卖出成交行口径和这些 summary 的交互粒度一致。
2. 操作记录以 `live_trade` 行展示，用户更容易把一行卖出操作和一次胜负判断对应起来。
3. 未卖出持仓仍是浮动盈亏，不应混入 realized trade win-rate。
4. `win_rate_closed_trades` 可以继续作为 calculation/dbt 层的交易质量指标，但不应在 UI 中无说明地复用为“交易胜率”。

同时建议建立命名约束：

- UI 不再使用单独的“交易胜率”泛称。
- 若分母是卖出成交行，命名为 `sell_trade_win_rate` 或展示为“卖出胜率”。
- 若分母是 closed trade/closed lot row，命名为 `closed_trade_win_rate` 或展示为“已平仓批次胜率”。
- 若分母是净值观察日，继续命名为 `daily_win_rate`，展示为“日胜率”。

## 实施草案

第一阶段只做语义收敛，不改变数值：

1. 前端详情页把“交易成功率”改为“卖出胜率”或“已实现交易胜率”。
2. 为该指标增加 tooltip，说明分子、分母和未卖出持仓不计入。
3. 后端 response 暂时保留 `trade_win_rate`，避免破坏现有 API。
4. 在 TypeScript 类型注释或 API 文档中补充当前口径。

第二阶段再讨论字段重命名：

1. 后端新增 `sell_trade_win_rate`，保留 `trade_win_rate` 作为兼容字段。
2. 前端切换读取 `sell_trade_win_rate`。
3. 经过一个兼容周期后删除或降级 `trade_win_rate`。

第三阶段补齐质量约束：

1. 对 statement summary query 增加无 closed row 的卖出计数，确认是否存在 ledger 缺口。
2. 若存在缺口，决定是阻断 summary、显示 partial 状态，还是继续按 0 PnL 计入非胜利。
3. 为卖出成交行胜率和 closed lot 胜率分别补测试样例，覆盖“一次卖出关闭多个 lot”的差异。

## 验收标准

- 用户可见文案不再把订单成交成功率、日胜率和交易级胜率混在一起。
- 对账单“胜率”指标的 tooltip 能明确回答分子、分母、区间、是否包含未卖出持仓。
- 代码层至少保留一处权威说明，说明 `trade_win_rate` 当前是卖出成交行口径。
- 测试覆盖区间无卖出时返回 `NULL`，以及一次卖出关闭多个 closed rows 时按一笔卖出判断胜负。

## 待决问题

1. UI 最终文案选“卖出胜率”还是“已实现交易胜率”？
2. `trade_win_rate` 是否需要重命名为 `sell_trade_win_rate`，以及兼容周期多长？
3. 无 closed row 的卖出是否应继续按 `0` PnL 计入分母，还是应暴露为数据质量异常？
4. closed lot 胜率是否需要在账户对账单中展示，还是只保留在交易质量明细和 dbt mart/rank 中？
