# RFC 0041: Racingline 最近信号建仓日期与空位补仓规则

状态：Implemented（最近信号发布 gate + 空位补仓命名解释，2026-07-02）
领域：racingline, rearview
关联系统：racingline, rearview, rearview-portfolio-worker, ClickHouse marts
代码根：
- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-portfolio-worker/`
架构事实：
- `docs/architecture/racingline.md`
- `docs/architecture/rearview.md`
关联文档：
- `docs/RFC/archive/0027-racingline-strategy-simulation-position-step4.md`
- `docs/RFC/archive/0028-racingline-strategy-backtest-step5.md`
- `docs/RFC/archive/0031-racingline-step4-step5-backtest-latency-slimming.md`
- `docs/RFC/archive/0034-racingline-step5-portfolio-publish-dialog-tabs.md`
- `docs/plans/archive/0069-racingline-strategy-entry-rule-implementation-plan.md`
- `docs/jobs/reports/2026-07-02-racingline-strategy-entry-rule-implementation.md`

## 摘要

本文只收敛两个建仓规则决策：

1. 调整最近信号建仓日期规则：发布组合时，如果最后信号日早于服务端当前日期，视为行情或 mart 数据滞后，不允许发布组合，不生成 pending 首仓信号，也不把首个建仓日顺延到未来交易日。
2. 保持当前空位补仓执行规则：回测和首仓继续采用“每日候选信号 Top N + 仅空位调入 + 旧持仓由风控退出”的模型，不改为每日 Top N 目标持仓再平衡。

这两个问题需要分开表达：最近信号日期是发布前置 gate；空位补仓是组合运行期间的买入执行模型。前者要新增阻断规则，后者保持当前执行口径，但需要强化命名和解释，避免用户把 Top N 误解为每日目标持仓集合。

## 目标

本 RFC 的目标是：

1. 固化最近信号建仓日期规则，避免用滞后的历史信号在当前日期发布组合。
2. 固化当前空位补仓执行规则，明确本轮不改变买入、持有和退出模型。
3. 明确前端文案、后端 publish preview/create API 和测试需要覆盖的实施点。

非目标：

- 不设计每日 Top N 全量换仓、rank band、score 加权或现金再分配算法。
- 不修改历史归档 RFC。
- 不评价具体策略收益表现。

## 决策一：调整最近信号建仓日期规则

### 问题

当前 Step 5「建立组合」的 publish preview 使用 source run 的 `end_date` 作为 `source_signal_date`，再把首个建仓日解析为 `next_trade_date_after(source_signal_date)`。

如果行情数据只更新到 `2026-06-29`，用户在 `2026-07-02` 建立组合，当前实现可能仍基于 `2026-06-29` 的最后信号生成 pending 首仓，并把执行日解析到 `2026-06-30` 这类历史日期。这个行为与讨论口径不一致。

讨论口径是：旧信号不能在当前日期发布成新组合首仓。数据滞后时，正确动作是更新行情并重跑回测，而不是把 6/29 信号顺延到 7/3，也不是创建历史执行日的 pending buy signal。

### 接受规则

发布组合时以后端服务端日期作为判断基准：

```text
source_signal_date = 最后一个可用行情信号日
server_current_date = 服务端当前市场日期

if source_signal_date < server_current_date:
  can_publish = false
  blockers += "行情数据未更新，最后信号日早于当前日期；请先更新行情并重跑回测"
else if source_signal_date == server_current_date:
  can_publish = true
  planned_live_start_date = next_trade_date_after(source_signal_date)
else:
  can_publish = false
  blockers += "最后信号日晚于当前日期，请检查行情日期或系统时间"
```

示例：

| 场景 | 最后信号日 | 当前日期 | 结果 |
|---|---:|---:|---|
| 行情滞后 | `2026-06-29` | `2026-07-02` | 阻止发布，不生成首仓 |
| 当日信号 | `2026-07-02` | `2026-07-02` | 允许发布，首个建仓日为下一交易日 |
| 未来信号 | `2026-07-03` | `2026-07-02` | 阻止发布，提示日期异常 |

### API 和 UI 口径

publish preview 需要明确返回：

- `source_signal_date`：首仓信号使用的最后信号日。
- `server_current_date`：后端用于 stale 判断的当前日期。
- `planned_live_start_date`：仅在允许发布时返回可确认的计划建仓日。
- `can_publish`：是否允许建立组合。
- `blockers`：不能发布时的原因。

create API 不能只信任前端提交的 preview 日期。创建时必须重新按服务端当前日期解析 preview，并再次校验 stale；如果 `source_signal_date < server_current_date`，即使前端提交了匹配的 `expected_source_signal_date` 和 `expected_live_start_date`，也要拒绝创建。

UI 需要把这个规则表达为“信号新鲜度”问题，而不是“首个建仓日选择”问题。数据滞后时，按钮应不可确认，并提示先更新行情、重跑回测。

## 决策二：保持当前空位补仓执行规则

### 当前执行模型

当前回测建仓口径保持不变：

```text
每日按策略规则筛出股票池
  -> 按 score DESC, security_code ASC 排名
  -> 每日只取 Top N 作为候选买入信号
  -> 信号日 T 收盘确认
  -> 下一交易日 T+1 开盘价加买入滑点成交
  -> 只在有空仓位时买入，不因跌出 Top N 主动卖出旧持仓
  -> 旧持仓只通过止损、止盈、时间止损或指标止损退出
  -> 每只目标金额 = 卖出后总权益 * min((1 - cash_reserve_pct) / max_positions, single_position_limit_pct)
  -> 按 100 股整数手、最小 1 手、现金和费用约束下调数量
```

因此当前模型是“信号驱动的空位补仓 + 风控退出”，不是“每日 Top N 目标持仓再平衡”。

`buy_signal_top_n` 和 `max_positions` 继续保持独立：

- `buy_signal_top_n`：每日候选买入信号数量。
- `max_positions`：组合最多持仓只数。

当 `buy_signal_top_n > vacant_slots` 时，只会按 rank 优先填满空位；当 `buy_signal_top_n < vacant_slots` 时，空位可能继续保留。已有持仓如果再次出现在候选信号中，不会重复买入或加仓。

### 命名和解释

前端文案需要强化这个模型：

| 当前文案 | 调整后文案 |
|---|---|
| 买入信号 Top N | 每日候选信号 Top N |
| 调仓规则：仓位空余按信号调入 | 调仓规则：仅空位调入；旧持仓由风控退出 |

Step 4 摘要、Step 5 结果页和发布弹层需要解释：

- Top N 是每日候选信号数量，不是每日目标持仓集合。
- 当前持仓未必等于最新 Top N。
- 股票跌出最新 Top N 不会自动卖出。
- 旧持仓由风控规则退出，退出后腾出的空位再由后续候选信号补入。

### 本轮不改变的行为

本轮明确不改：

- 不因为旧持仓跌出最新 Top N 而卖出。
- 不把每日 Top N 作为目标持仓集合。
- 不引入 rank band 换仓参数。
- 不按 score 加权分配资金。
- 不把单票上限产生的隐含现金重新分配给其他股票。
- 不合并 `buy_signal_top_n` 和 `max_positions`。

## 当前实现事实

### Step 4 配置入口

前端 Step 4 配置位于 `app/racingline/src/features/strategy/components/simulation-position-panel.tsx` 和 `app/racingline/src/features/strategy/execution.ts`。

当前固定口径：

- `signal_policy.signal_timing = "close_confirm_next_open"`。
- `rebalance_policy.target_weighting = "equal_weight_capped"`。
- `cash_reserve_pct = 0`。
- `lot_size = 100`。
- `min_trade_lots = 1`。
- `empty_signal_action = "hold"`。
- `price_basis = "backward_adjusted"`。
- 买入和卖出滑点来自同一个前端滑点百分比，转换成 bps。

### 后端 execution config

Rearview 的 `BacktestExecutionConfig` 定义和校验位于 `engines/crates/rearview-core/src/strategy_backtest.rs`。

后端 summary 中的单票目标权重为：

```text
target_weight_per_position_pct =
  min((1 - cash_reserve_pct) / max_positions, single_position_limit_pct)
```

`implicit_cash_reserve_pct` 由目标权重和最大持仓反推：

```text
1 - target_weight_per_position_pct * max_positions
```

当单票上限低于等权目标时，剩余资金自然留作现金。

### 信号生成和 T+1 映射

Worker 的回测信号物化位于 `engines/crates/rearview-portfolio-worker/src/main.rs` 的 `materialize_strategy_backtest_signals()`。

信号 SQL 由 `engines/crates/rearview-core/src/planner/sql.rs` 的 `compile_backtest_signals()` 生成：

```sql
row_number() OVER (
  PARTITION BY trade_date
  ORDER BY score DESC, security_code ASC
) AS signal_rank
...
WHERE signal_rank <= {top_n}
ORDER BY trade_date ASC, signal_rank ASC, security_code ASC
```

当前事实：

- 每个交易日独立排名。
- 排名先按 `score` 降序，再按 `security_code` 升序稳定打破同分。
- Worker SQL 只返回 `signal_rank <= buy_signal_top_n` 的行。
- Worker 把每条 T 日信号映射到交易日历中的下一个交易日作为 `execution_date`。
- 如果没有下一个交易日，或下一个交易日超过回测 `end_date`，该信号会被丢弃。
- 信号进入模拟器前按 `(execution_date, rank, security_code)` 排序。

### 模拟器买入和退出

Portfolio simulation engine 位于 `engines/crates/rearview-core/src/portfolio/mod.rs`。

每个交易日按以下顺序处理：

1. 执行前一交易日收盘后触发并排队到今天的卖出单。
2. 计算卖出后的现金和持仓市值。
3. 根据今天 `execution_date` 的买入信号，在空余仓位内逐只买入。
4. 用收盘价估值，写 position day 和 nav。
5. 收盘后评估风控退出规则，若触发则排队到下一交易日开盘卖出。

买入时按 rank 从小到大处理信号；已持仓股票跳过；开盘价缺失、低于最小手数或现金不足时写 skipped order。成功成交后写 target、order、trade 和 position state。

当前退出只来自：

- 固定止损。
- 止盈。
- 时间止损。
- 指标止损。

### 发布组合当前缺口

Step 5 publish preview 位于 `engines/crates/rearview-core/src/api/mod.rs`。

当前事实：

- `source_signal_date = source_run.end_date`。
- `planned_live_start_date` 由交易日历解析为 `source_signal_date` 后的下一交易日。
- pending buy signals 使用 source run 的 `rule_snapshot` 和 `execution_config.signal_policy.buy_signal_top_n` 重新编译单日信号。
- create API 只校验前端提交的 expected 日期与重新计算的 preview 一致。
- 当前没有校验 `source_signal_date` 是否早于服务端当前日期。

这正是决策一需要补上的发布 gate。

## 实施待办

1. Rearview publish preview：新增 `server_current_date`、`can_publish`、`blockers`；当 `source_signal_date < server_current_date` 时返回 blocked，不生成可确认的首仓。
2. Rearview create API：重新按服务端当前日期校验 stale，拒绝旧 preview 提交。
3. Racingline 发布弹层：展示“最后信号日”“当前日期”“计划建仓日”；stale 时禁用确认并提示更新行情、重跑回测。
4. Racingline Step 4：把 `买入信号 Top N` 改为“每日候选信号 Top N”，把调仓规则改为“仅空位调入；旧持仓由风控退出”。
5. Racingline Step 4/Step 5：补充解释 Top N 是每日候选信号数量，不是每日目标持仓集合。
6. 测试：覆盖 stale signal blocked、同日 signal allowed、未来 signal blocked、create 二次校验、UI 文案不改变 execution config。

## 暂不实施

以下方案作为未来可能的策略模式保留，不进入本轮：

- Top N 目标持仓再平衡：每日把 Top N 视为目标持仓集合，卖出跌出 Top N 的旧持仓。
- Rank band 换仓：买入 Top N，跌出 Top M 才卖出，用更宽的持有区间减少边界抖动。
- 现金再分配或 score 加权：对当天可买信号重新分配现金，或按 score 加权分配目标金额。

## 最小验证入口

当前规则已有以下源码和测试覆盖点：

- `app/racingline/src/features/strategy/execution.test.ts`：Step 4 设置到 execution config 的序列化。
- `engines/crates/rearview-core/src/strategy_backtest.rs`：execution config canonicalization、hash 和 summary。
- `engines/crates/rearview-core/src/planner/sql.rs`：TopN-only worker signal SQL。
- `engines/crates/rearview-core/src/portfolio/mod.rs`：simulation engine 的买入、卖出、手数、费用和风控规则。
- `engines/crates/rearview-portfolio-worker/src/main.rs`：回测 worker 信号物化、T+1 映射和 simulation input 装配。

后续实现至少应补充：

- publish preview/create API 测试：stale signal blocked、同日 signal allowed、未来 signal blocked。
- portfolio 单元测试：旧持仓跌出 TopN 不卖出、空位补仓、已有持仓不重复买入。
- 前端 execution 测试：文案调整不改变 execution config。
