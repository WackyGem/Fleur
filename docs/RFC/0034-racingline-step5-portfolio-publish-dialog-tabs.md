# RFC 0034: Racingline Step 5 建立策略组合弹层分 Tab 信息架构

状态：Proposed（讨论稿，2026-06-27；2026-07-02 更新盘中/收盘后建仓日期规则）
领域：racingline
关联系统：racingline, rearview, data-platform
代码根：
- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/migrate/`
- `pipeline/elt/`
架构事实：
- docs/architecture/racingline.md
- docs/architecture/rearview.md
- docs/architecture/data-platform.md
关联文档：docs/RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md

## 摘要

`/strategies` Step 5 成功回测后，用户点击「建立组合」会打开「建立策略组合」弹层。当前弹层把策略名称、条件指标、评分项、建仓摘要、回测业绩和回测快照纵向堆叠在同一页面里，信息密度高，用户很难先确认“这个组合将按什么规则运行”，再确认“这次回测表现是否足够好”。

本 RFC 讨论两个层面的目标态：一是「建立策略组合」弹层的信息架构，二是点击确定后组合发布、T+1 建仓、pending 看板和 backtest/live 数据隔离的业务口径。本文仍是设计文档，不直接改代码。目标态是：

1. 弹层拆成多个 tab。
2. tab 公共顶部固定展示策略名称和建仓日期。
3. 建仓日期采用 T+1 交易日口径，并按交易阶段区分盘中和收盘后可用信号日。
4. Tab 1 展示 Step 1 指标过滤、Step 2 权重得分和 Step 4 建仓摘要。
5. Tab 2 展示 Step 5 回测业绩。
6. 发布后的组合把回测依据和建仓后跟踪拆成两段数据，Dashboard 默认只展示 live 段。
7. ClickHouse 结果事实层强制拆成 `fleur_backtest` 和 `fleur_portfolio.live_*` 两套物理表；PostgreSQL 控制面先审计现状，再按职责拆分或新增表。

## 当前事实

当前「建立策略组合」弹层位于 [strategy-page.tsx](../../app/racingline/src/routes/strategy-page.tsx) 第 2458 行附近。按钮只在 `activeBacktestRun.status === "succeeded"`、存在 `current_result_attempt_id` 且配置未 stale 时启用。

弹层现有数据来源：

| 区域 | 当前来源 |
|---|---|
| 策略名称 | `portfolioName` 本地状态 |
| 指标过滤 | `conditionGroups -> publishConditionRows` |
| 权重得分 | `weightIndicators -> publishScoringRows` |
| 建仓摘要 | `effectiveSimulationSettings` + `backtestExecutionDraft.summary.enabled_exit_rule_count` |
| 回测上下文 | `activeBacktestRun`、`backtestPeriod`、`backtestBenchmark`；只服务业绩简报上下文和发布入参，不作为快照区块展示 |
| 回测业绩 | `useStrategyBacktestNavQuery()` + `useStrategyBacktestPerformanceQuery()` |

当前直接可取的日期字段包括：

| 字段 | 含义 | 是否足够作为建仓日期 |
|---|---|---|
| `activeBacktestRun.end_date` | 回测区间结束日 | 否，只是 T |
| `publishLatestNetValuePoint.time` | 回测结果最新净值交易日 | 否，只是 T |
| `previewSnapshot.range.selectedTradeDate/endDate` | Step 3 预览交易日 | 否，不能代表发布组合建仓日 |

因此，建仓日期如果要严格采用 T+1 交易日，不应由前端按自然日 `+1` 推测。RFC 建议定义为：

```text
T = 成功回测的最新有效交易日，优先取 latest nav trade_date，等价时可落到 activeBacktestRun.end_date
建仓日期 = Rearview 交易日历中 T 的下一个交易日
```

2026-07-02 讨论补充：T 不能只按服务器日期判断，还要结合 A 股收盘时间。15:00 前当天行情尚未完成，允许使用上一已完成交易日作为 T；15:00 后按当前建仓规则，要求当天收盘数据已经更新，否则不允许发布。实现阶段应由 Rearview 发布预检接口提供 `source_signal_date`、`required_source_signal_date`、`server_current_date`、`market_phase` 和 `planned_live_start_date`；前端只展示 `source_signal_date` 和 `planned_live_start_date`，其余字段只用于校验、禁用确认和 create expected payload。

## 目标

1. 把当前长弹层重组为有公共顶部的 tab 弹层。
2. 让用户先在 Tab 1 确认发布后的策略组合配置，再在 Tab 2 确认回测业绩。
3. 公共顶部保留策略名称输入，增加建仓日期展示，避免每个 tab 重复。
4. 保留「取消 / 确定」底部操作，确定按钮仍只创建 strategy portfolio，不在前端计算权威组合结果。
5. 明确发布后 pending-first-run 看板只能展示 T 日生成、T+1 调入的买入信号，不能用 source backtest 业绩填充 live 业绩空缺。
6. 明确正式组合的净值、基准净值和业绩指标从 `live_start_date` 重新归 1 并重新计算入库，不延续回测曲线、持仓、订单、成交或风控状态。

## 非目标

1. 不在本文直接实施前端、Rearview、worker、ClickHouse 或 PostgreSQL 代码变更；实施路径见关联 plan。
2. 不新增前端计算净值、绩效、交易日历或组合持仓的逻辑。
3. 不在本 RFC 中决定最终视觉样式、具体 CSS class 或组件拆分。
4. 不引入真实券商交易、实盘下单、成交回报或资金账户。

## 信息架构

### 公共顶部

公共顶部在 tab 之上，所有 tab 都保持可见。建议只包含发布动作必需的身份信息：

| 字段 | 说明 |
|---|---|
| 策略名称 | 必填输入，默认仍可为「策略组合」。 |
| 建仓日期 | 只展示后端确认的日期；`T+1 交易日`作为日期后的次级说明，不单独做重点卡片。 |

线框：

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ 建立策略组合                                                                 │
│ 确认策略配置和回测表现，发布后返回看板。                                      │
├──────────────────────────────────────────────────────────────────────────────┤
│ 策略名称                                                                     │
│ ┌────────────────────────────────────────────┐  建仓日期                    │
│ │ 策略组合                                   │  2026-06-29  T+1 交易日      │
│ └────────────────────────────────────────────┘                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ 策略配置 ] [ 回测业绩 ]                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Tab 1：策略配置

Tab 1 回答的问题是：发布后的组合会按照哪些规则选股、打分和建仓？

建议分三段竖直清单：

1. `指标过滤`：每条清单展示分组、组内关系和条件表达式。
2. `权重得分`：每条清单展示序号、加分值和评分条件。
3. `建仓摘要`：用清单展示资金、TopN、最大持仓、单票上限、交易成本和已启用风控。

线框：

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ 公共顶部：策略名称 + 建仓日期                                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ 策略配置 ] [ 回测业绩 ]                                                     │
├──────────────────────────────────────────────────────────────────────────────┤
│ 指标过滤                                                                     │
│  01  指标组 1 · 组内起始                                                      │
│      price_ma_5 > price_ma_10                                                 │
│  02  指标组 1 · AND                                                           │
│      volume > avg_volume_20                                                   │
│  03  指标组 2 · OR                                                            │
│      turnover_rate between 2% and 8%                                           │
│                                                                              │
│ 权重得分                                                                     │
│  01  +30                                                                      │
│      turnover_rate > 3%                                                        │
│  02  +20                                                                      │
│      close_price > price_ma_20                                                 │
│  03  +10                                                                      │
│      volume_ratio > 1.2                                                        │
│                                                                              │
│ 建仓摘要                                                                     │
│  · 初始资金：¥1,000,000                                                       │
│  · 每日候选：Top 5                                                            │
│  · 最大持仓：5 只                                                             │
│  · 单票上限：10%                                                              │
│  · 交易成本：佣金 0.010%，滑点 0.100%，印花税 0.050%，过户费 0.001%          │
│  · 风控：固定止损 8%；时间止损 20 天后收益低于 0%                            │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                         [ 取消 ] [ 确定 ]    │
└──────────────────────────────────────────────────────────────────────────────┘
```

Tab 1 的内容取舍：

| 内容 | 建议 |
|---|---|
| Step 1 条件表达式 | 使用竖直清单展示，不使用明显表格布局；长表达式允许在条目内换行或区域内滚动。 |
| Step 2 权重得分 | 使用竖直清单展示得分和表达式，不在此处展示 Step 3 个股样本。 |
| Step 4 建仓摘要 | 使用竖直清单展示当前字段，避免摘要网格或表格化布局；风控合并为一条，只列启用规则，未启用项不展示。 |
| 回测 Run ID / result attempt | 默认不放在 Tab 1，可放到后续“诊断/快照”折叠区；普通发布确认不需要先看 UUID。 |
| 股池预览日期 | 默认不放在 Tab 1，除非后续决定要展示“规则检查日期”。 |

### Tab 2：回测业绩

Tab 2 回答的问题是：这套配置的历史回测表现是否足够支持发布？

建议做成一份竖直简报，而不是指标矩阵或调试快照：

1. `简报上下文`：先展示周期、起止日期和业绩基准。
2. `业绩表现`：先展示业绩日期，再把策略净值、基准净值、超额收益、持仓收益、年化收益和日胜率逐条竖向展示。
3. `指标要点`：风险、性价比和相对市场指标也逐条竖向展示。

线框：

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ 公共顶部：策略名称 + 建仓日期                                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ 策略配置 ] [ 回测业绩 ]                                                     │
├──────────────────────────────────────────────────────────────────────────────┤
│ 回测业绩简报                                                                 │
│  周期：近一年（2025-06-26 - 2026-06-26）                                      │
│  业绩基准：中证A100                                                           │
│                                                                              │
│  业绩表现                                                                     │
│  · 业绩日期：2026-06-26                                                       │
│  · 策略净值：1.1842                                                           │
│  · 基准净值：1.0615                                                           │
│  · 超额收益：+12.27%                                                          │
│  · 持仓收益：+18.42%                                                          │
│  · 年化收益：+17.95%                                                          │
│  · 日胜率：56.8%                                                              │
│                                                                              │
│  风险                                                                         │
│  · 最大回撤：-7.81%                                                           │
│  · 年化波动率：21.30%                                                         │
│  · 下行波动率：13.40%                                                         │
│                                                                              │
│  性价比                                                                       │
│  · Sharpe：0.84                                                               │
│  · Sortino：1.12                                                              │
│  · Calmar：2.30                                                               │
│  · Treynor：0.18                                                              │
│                                                                              │
│  相对市场                                                                     │
│  · Alpha：+6.20%                                                              │
│  · Beta：0.91                                                                 │
│  · Information Ratio：0.77                                                    │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                         [ 取消 ] [ 确定 ]    │
└──────────────────────────────────────────────────────────────────────────────┘
```

Tab 2 的内容取舍：

| 内容 | 建议 |
|---|---|
| 周期、起止日期、业绩基准 | 放在简报最前面，不做卡片。 |
| 业绩日期 | 放在 `业绩表现` 段落下，作为第一条清单项。 |
| 策略净值、基准净值、超额收益、持仓收益、年化收益、日胜率 | 保留，逐条竖向展示，不横向串联。 |
| 风险 / 性价比 / 相对市场 | 保留关键指标，逐条竖向展示，不做矩阵。 |
| 净值曲线 | 本次不建议放进发布弹层；Step 5 主页面已经展示，弹层只做确认。 |
| 调仓记录/持仓明细 | 不放进弹层；信息量过大，应留在 Step 5 页面或后续详情页。 |
| 回测快照、回测 Run ID、result attempt | 不放进 Tab 2；如需排查，后续另放诊断入口，不进入发布确认主流程。 |

## 后端接口和数据口径校对

本节补充 2026-06-27 对现有前后端实现的代码级核对，重点确认「点击确定后建仓」的交易日、信号和 dashboard 口径。

### 目标业务语义

以 2026-06-27 周六发布为例：

```text
T = 2026-06-26，最近一个已经完成并可生成收盘信号的交易日
T+1 = 2026-06-29，交易日历中的下一个交易日

Step 5 succeeded backtest
  -> 用户点击「建立组合」
  -> 发布预检确认 T=2026-06-26、建仓日=2026-06-29
  -> 用户点击「确定」
  -> 创建 strategy_portfolio，状态为 pending_first_run
  -> 返回 /dashboard
  -> 看板只展示 T 日生成、T+1 调入的买入信号
  -> 看板不展示正式组合净值、收益、风险、回撤、业绩曲线
  -> T+1 daily run 成功后，才展示正式组合业绩数据
```

关键口径：

| 概念 | 定义 |
|---|---|
| `source_signal_date` | 发布时用于生成首批买入信号的 T 日，必须等于发布预检解析出的 `required_source_signal_date`。 |
| `required_source_signal_date` | 后端根据交易日历和 `Asia/Shanghai` 当前时间解析出的本次发布允许信号日。 |
| `planned_live_start_date` / `live_start_date` | `source_signal_date` 的下一个交易日，不允许用自然日 `+1`。 |
| `pending_buy_signals` | T 日规则生成的 TopN 买入信号，`execution_date = live_start_date`。 |
| `live performance` | 只来自 succeeded strategy portfolio daily run；pending 首次运行时为空。 |

### 盘中/收盘后信号日期规则

发布预检必须使用交易阶段感知的信号日期规则，而不是简单要求 `source_signal_date == server_current_date`。核心目标是：盘中允许用户使用上一已完成交易日的收盘信号建立组合，收盘后继续要求当天行情更新后才能发布。

建议规则：

| 场景 | `required_source_signal_date` | 允许发布条件 | `planned_live_start_date` | 用户文案 |
|---|---|---|---|---|
| 交易日 15:00 前 | `previous_trade_date(server_current_date)` | `source_signal_date == required_source_signal_date` | `next_trade_date(source_signal_date)`，通常为当天交易日 | 展示最后信号日和计划建仓日。 |
| 交易日 15:00 后 | `server_current_date` | `source_signal_date == required_source_signal_date` | `next_trade_date(source_signal_date)` | 当天数据未更新时提示“最后信号日与最新行情日存在缺口，请先回填行情数据到最新。”，禁止确定。 |
| 非交易日 | 最近一个已完成交易日 | `source_signal_date == required_source_signal_date` | `next_trade_date(source_signal_date)` | 按周末/节假日正常 T+1 发布。 |
| 数据落后超过一个允许信号日 | 不变 | 禁止发布 | 不返回 | 提示“最后信号日与最新行情日存在缺口，请先回填行情数据到最新。”，避免把多日旧信号误认为可建仓。 |

我的建议是只做“一个交易阶段”的前推/回退，不做无限向前寻找可用信号。也就是说，15:00 前最多允许用上一交易日信号；如果数据仍停在更早日期，说明数据链路已经落后，不应让用户继续创建新组合。

示例：

```text
2026-07-02 14:30，server_current_date = 2026-07-02
required_source_signal_date = previous_trade_date(2026-07-02) = 2026-07-01
source_signal_date = 2026-07-01 时允许发布
planned_live_start_date = 2026-07-02

2026-07-02 15:30，server_current_date = 2026-07-02
required_source_signal_date = 2026-07-02
source_signal_date = 2026-07-01 时禁止发布，提示最后信号日与最新行情日存在缺口
source_signal_date = 2026-07-02 时允许发布
planned_live_start_date = 2026-07-03
```

这样会带来一个必须在 UI 上解释清楚的差异：15:00 前建立组合时，计划建仓日可能是当天；15:00 后建立组合时，计划建仓日通常是下一交易日。这个差异不是前端特例，而是交易信号生成时点不同导致的结果。

实现状态：盘中/收盘后信号日期规则已由 [Plan 0070](../plans/archive/0070-racingline-strategy-publish-market-phase-entry-rule-plan.md) 落地，验收见 [2026-07-02 实施报告](../jobs/reports/2026-07-02-racingline-strategy-publish-market-phase-entry-rule.md)。RFC 0034 其他 backtest/live 数据隔离讨论仍按本文后续章节跟踪。

### 策略组合两段数据模型

正式策略组合必须显式区分两段数据：

```text
strategy_portfolio
  ├─ backtest_segment：发布依据
  │    source_strategy_backtest_run_id
  │    source_result_attempt_id
  │    source_start_date
  │    source_end_date
  │    source_period_key
  │    benchmark_security_code
  │
  └─ live_segment：建仓后跟踪
       live_start_date
       initial_signal_date
       latest_daily_run_id
       current_live_result_attempt_id
       live_status
```

两段数据的职责不同：

| 数据段 | 时间范围 | 权威来源 | 页面用途 | 禁止混用 |
|---|---|---|---|---|
| `backtest_segment` | Step 5 回测区间，例如 `2025-06-26 - 2026-06-26` | source strategy backtest result attempt | 发布弹层 Tab 2 展示“回测业绩简报”；组合详情中可作为“发布依据/回测依据”展示 | 不能冒充建仓后的真实组合业绩。 |
| `live_segment` | `live_start_date` 起的正式跟踪区间，例如 `2026-06-29` 起 | strategy portfolio daily run result attempt | Dashboard、组合详情默认展示的净值、收益、风险、持仓、调仓和跟踪信号 | pending 首次运行时不能用 backtest result 填充 live 业绩空缺。 |

因此，一个 strategy portfolio 不是“把 Step 5 backtest 改名为组合”，而是保存：

1. 一段不可变的 `backtest_segment`，说明这个组合为什么被发布。
2. 一段从 `live_start_date` 开始逐日产生的 `live_segment`，说明发布后真实跟踪结果。

Dashboard 默认展示 `live_segment`。在 `live_segment` 尚未产生第一条 succeeded daily run 之前，Dashboard 只能展示建仓日和 pending 买入信号；如果需要展示 `backtest_segment`，必须用明确标签，例如“回测依据”，不能放进“最新净值”“收益指标”“风险指标”或“净值与基准”这些 live 区域。

### Backtest 与 Live 状态隔离原则

回测业绩是历史区间内的策略表现，只能作为发布依据和用户决策参考。正式组合在建仓日后进入新的 live tracking 账本，不能延续回测区间内的任何运行状态。

隔离规则：

| Backtest 数据 | Live 组合是否继承 | 说明 |
|---|---|---|
| 回测净值、基准净值、收益、风险、回撤、绩效指标 | 否 | 仅用于 Tab 2「回测业绩」和“回测依据”标签区域。 |
| 回测持仓、现金、成本价、盈亏、仓位权重 | 否 | live 组合从 `live_start_date` 的新账本开始，初始状态是空持仓和配置中的初始资金。 |
| 回测订单、成交、调仓记录、事件、止损触发记录 | 否 | 这些只描述历史模拟过程，不能成为 live 组合的账本延续。 |
| 回测 source rule/config/hash/benchmark/period | 是，作为发布依据复制 | 复制的是规则和配置，不是回测账本状态。 |
| T 日 pending buy signals | 是，作为首批待调入信号 | 这批信号由发布预检基于 T 日规则重新生成，不从回测持仓或 latest target 继承。 |

因此，正式组合的第一条 live 记录不是 source backtest 的下一条记录。它是一个新 result attempt：使用同一策略规则、同一执行配置和 T 日信号，在 `live_start_date` 后重新跟踪。

### 表级隔离设计

数据模型必须拆成两段，结果事实层也必须物理拆表。原因不是性能优先，而是业务语义优先：回测是历史业绩，正式组合是建仓后的跟踪账本，两者不应在同一套裸 facts 中依靠约定过滤来避免混用。

| 层级 | 隔离方式 | 设计结论 |
|---|---|---|
| PostgreSQL control plane | 先审计，再决定拆分 | 当前已有 `strategy_backtest_run`、`strategy_portfolio`、`strategy_portfolio_daily_run` 等表，表面上已分开，但仍需要梳理字段归属、状态机、current attempt、outbox、client_request_id、summary/progress/signal_summary 和 API resolver 调用链，再决定是否进一步拆字段、拆表或加 publish context 表。 |
| ClickHouse result facts | 强制物理拆表 | 回测结果写入 `fleur_backtest`；正式组合 live 结果写入 `fleur_portfolio` 的 `live_*` 表。两者不再共用旧 `fleur_portfolio.portfolio_*` / `fleur_calculation.calc_portfolio_*` 裸 facts。 |
| API / repository | 强类型读取隔离 | Backtest endpoint 只能读 backtest result family；portfolio live endpoint 只能读 live result family。禁止通用 resolver 跨 family fallback。 |
| dbt / analytics wrapper | 两套 source / wrapper | dbt 或分析层分别声明 backtest result source 和 live portfolio result source，不允许用同一 source 再靠 `source_kind` 过滤。 |

目标 ClickHouse 结构：

```text
fleur_backtest.*
  backtest_run_snapshot
  backtest_nav_daily
  backtest_target
  backtest_order
  backtest_trade
  backtest_position_day
  backtest_event
  backtest_performance_metric
  backtest_performance_metric_status
  backtest_closed_trade
  backtest_trade_metric
  ...

fleur_portfolio.*
  live_run_snapshot
  live_nav_daily
  live_target
  live_order
  live_trade
  live_position_day
  live_event
  live_performance_metric
  live_performance_metric_status
  live_closed_trade
  live_trade_metric
  ...
```

命名可以在实施阶段微调，但必须满足：

1. 回测事实表和 live 事实表是不同物理表。
2. 回测查询代码不能访问 live 表，live 查询代码不能访问回测表。
3. 写入 worker 必须按任务类型选择目标 writer。
4. dbt sources 必须分开声明，避免裸表混读。
5. 旧统一 facts 表如需保留，只作为历史兼容或迁移来源，不作为新写入目标。

ClickHouse 设计约束：

1. Per `schema-pk-plan-before-creation`，拆表时不能直接复制旧 `ORDER BY` 当作最终答案；必须分别列出 backtest result 和 live result 的主要查询模式后再定排序键。
2. Per `schema-pk-filter-on-orderby`，两类 API 仍应以 run id + result attempt 为主要过滤前缀，避免扫描跨 run 数据。
3. Per `schema-partition-low-cardinality` 和 `decision-partitioning-timeseries`，nav/position/trade/event 等时序事实优先使用月分区；禁止按 run id 或 portfolio id 做高基数分区。
4. Per `insert-mutation-avoid-update`，两套表都继续使用 append-only result attempt，重算生成新 attempt，不用 mutation 修正旧 attempt。
5. Per `insert-batch-size`，两套 writer 都继续按 run 批量写入，不引入逐行补写。

PostgreSQL 控制平面不在本 RFC 中直接宣布“再拆哪张表”，必须先完成现状梳理：

1. 列出 `strategy_backtest_run`、`strategy_backtest_task_outbox`、`strategy_portfolio`、`strategy_portfolio_daily_run`、`strategy_portfolio_daily_task_outbox` 的字段和状态机。
2. 标注哪些字段属于发布依据、哪些属于 live tracking、哪些只是 UI snapshot 或 worker 诊断。
3. 追踪 create、publish、daily run、dashboard、detail endpoints 对这些字段的读取和写入。
4. 找出仍然把 source backtest 和 live portfolio 绑定过紧的现有字段或 resolver，例如 `source_result_attempt_id`、`current_result_attempt_id`、summary/progress 复用和 dashboard fallback；目标态中 live 当前 attempt 应表达为 `current_live_result_attempt_id`。
5. 基于审计结果再决定 PostgreSQL 是否需要新增 `strategy_portfolio_publish_context`、`strategy_portfolio_live_state` 或拆分 outbox/attempt registry；如果审计确认发布依据和 live 状态仍混在同一字段或同一 JSON 快照中，必须拆出职责明确的字段或表，不能继续用模糊 JSON 承载权威状态。

### Live 净值和业绩重算口径

`live_segment` 不是把 source backtest 的尾部曲线接到组合上，也不是把 `initial_signal_date` 当作组合净值起点。正式组合的净值、基准净值和业绩指标必须以 `live_start_date` 为建仓基准日重新归 1、重新计算并写入 live daily run 的 result attempt。

关键口径：

| 概念 | 口径 |
|---|---|
| `initial_signal_date` | T 日，只用于生成首批信号和驱动 T+1 买入；不作为 live 净值基准日。 |
| `live_start_date` | T+1 交易日，正式建仓日，也是 live 策略净值和基准净值的归一化基准日。 |
| 策略净值 | live result attempt 入库前按 `live_start_date` 重新归 1；Dashboard 和详情读取的第一条 live nav 应为 `1.0`。 |
| 基准净值 | 与策略净值同口径，以 `live_start_date` 的基准价格或基准收益序列为 `1.0` 重新计算。 |
| 业绩指标 | 收益、超额收益、年化收益、风险、回撤、Sharpe/Sortino/Calmar、Alpha/Beta/IR 等只基于 `live_start_date..trade_date` 的归一化 live 序列计算。 |
| T 日模拟种子 nav | 如果模拟器内部为了执行 T 信号而产生 `initial_signal_date` 的初始现金 nav，该行只能作为计算种子；不得作为 live nav/performance 写入或展示。 |

示例：

```text
source_signal_date = 2026-06-26
live_start_date = 2026-06-29

worker simulation window:
  2026-06-26 -> 2026-06-29

ClickHouse live result attempt:
  nav rows start at 2026-06-29
  strategy_nav(2026-06-29) = 1.0
  benchmark_nav(2026-06-29) = 1.0
  performance window = 2026-06-29..trade_date
```

### 当前实现事实

| 链路 | 当前实现 |
|---|---|
| 前端发布 | [strategy-page.tsx](../../app/racingline/src/routes/strategy-page.tsx) 的 `publishPortfolio()` 只提交 `source_strategy_backtest_run_id`、`source_result_attempt_id`、`name`，成功后立即跳转 `/dashboard`。 |
| 创建组合 | [api/mod.rs](../../engines/crates/rearview-core/src/api/mod.rs) 的 `create_strategy_portfolio()` 要求 source backtest `succeeded` 且 attempt 匹配，然后写入 `strategy_portfolio`。 |
| 建仓日期 | `resolve_strategy_portfolio_live_start_date()` 从 `source_run.end_date + 1` 到 `+45` 天查询交易日，取第一个大于 source end date 的交易日。 |
| 建仓日期失败兜底 | 如果查询不到未来交易日，当前实现 `unwrap_or(source_end_date)`，会把建仓日退回 T 日。 |
| Dashboard pending 状态 | `get_strategy_portfolio_dashboard()` 在没有 `latest_daily_run_id` 时设置 `live_status = pending_first_run`、`curve_source = source_backtest`。 |
| Dashboard 数据读取 | `resolve_strategy_portfolio_result()` 在没有 daily run 结果时回退到 source backtest result attempt；dashboard 因此读取 source backtest 的 nav、performance、curve 和 latest targets。 |
| Daily run 创建 | `create_strategy_portfolio_daily_runs_for_trade_date()` 只为 `live_start_date <= trade_date` 的组合创建 run，并把 `run_start_date` 写成 `portfolio.live_start_date`。 |
| Daily run 信号 | worker 的 `materialize_strategy_portfolio_daily_run_signals()` 用 `run_start_date..trade_date` 查询交易日，再把信号日映射到下一交易日。 |
| Backtest 尾日信号 | backtest worker 会丢弃 `execution_date > run.end_date` 的信号，因此 source backtest 的 `portfolio_target` 不会包含 T 日生成、T+1 执行的尾日信号。 |

### 实现偏差

| 偏差 | 影响 | 修正方向 |
|---|---|---|
| Dashboard pending 首次运行时回退到 source backtest 业绩。 | 新建组合看板会显示回测净值、收益、风险和曲线，用户会误认为组合已经建仓并产生业绩。 | pending 首次运行时 dashboard 的 live 业绩字段必须为空，只允许展示待调入信号。 |
| source backtest `portfolio_target` 不能代表发布日 T 的买入信号。 | backtest 结果会丢弃 `execution_date > end_date` 的尾日信号；直接取 latest target 可能是 T-1 信号、T 日调入，不是 T 信号、T+1 调入。 | 发布前单独编译 T 日信号，并把 execution date 设为后端交易日历解析出的 T+1。 |
| `resolve_strategy_portfolio_live_start_date()` 找不到未来交易日时退回 T。 | 极端情况下会违反 T+1 建仓日语义。 | 找不到 T+1 交易日时返回 validation/conflict 错误，前端禁用确定。 |
| 首个 daily run 的 `run_start_date = live_start_date`。 | 在 T+1 当天创建 run 时交易日列表只有一个日期，worker 会因少于两个交易日失败；即使放宽校验，也拿不到 T 日信号。 | daily run 的信号窗口起点应是 `source_signal_date`，而不是 `live_start_date`。 |
| live 组合没有明确禁止继承回测账本状态。 | 实现容易把回测持仓、现金、订单、成交或绩效当成组合建仓后的初始状态。 | live daily run 必须从空持仓和配置初始资金开始，只继承策略规则、执行配置、benchmark 和发布信号快照。 |
| Dashboard signal payload 只有 code/name/score。 | 看板无法表达“信号 T 日产生，T+1 调入”。 | `today_signals` 或 `pending_buy_signals` 需要包含 `signal_date`、`execution_date`、`source_rank`、`source_score`。 |
| 详情页 `/nav`、`/performance`、`/positions` 等 portfolio result endpoint 也会回退到 source backtest。 | pending 组合详情可能展示回测账本，而不是正式组合账本。 | live result endpoints 在没有 daily run result 时返回 `409 portfolio_pending_first_run`，不再用 source backtest 伪装 live result；Dashboard 仍返回 200 pending card。 |

### 改进接口设计

#### 发布预检接口

新增一个只读预检接口，供弹层打开时使用：

```http
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/portfolio-publish-preview?source_result_attempt_id=...
```

响应建议：

```json
{
  "can_publish": true,
  "blockers": [],
  "source_strategy_backtest_run_id": "01J...",
  "source_result_attempt_id": "01J...",
  "server_current_date": "2026-06-27",
  "server_current_time": "14:30:00+08:00",
  "market_phase": "before_close",
  "publish_cutoff_time": "15:00:00+08:00",
  "required_source_signal_date": "2026-06-26",
  "source_signal_date": "2026-06-26",
  "planned_live_start_date": "2026-06-29",
  "source_period_key": "1y",
  "source_start_date": "2025-06-26",
  "source_end_date": "2026-06-26",
  "benchmark_security_code": "000903.SH",
  "pending_buy_signals": [
    {
      "security_code": "600000.SH",
      "security_name": "浦发银行",
      "source_rank": 1,
      "source_score": 91.4,
      "signal_date": "2026-06-26",
      "execution_date": "2026-06-29"
    }
  ]
}
```

预检接口职责：

1. 校验 source backtest 已 succeeded 且 `source_result_attempt_id` 匹配当前 attempt。
2. 取 `source_signal_date = source_run.end_date`。
3. 基于 `Asia/Shanghai` 当前时间、交易日历和 `publish_cutoff_time = 15:00` 解析 `required_source_signal_date`：
   - 交易日 15:00 前：上一交易日。
   - 交易日 15:00 后：当前交易日。
   - 非交易日：最近一个已完成交易日。
4. 校验 `source_signal_date == required_source_signal_date`；不相等时返回 `can_publish=false` 和 blocker。15:00 后当天数据未更新、或盘中数据落后超过上一交易日，都应提示“最后信号日与最新行情日存在缺口，请先回填行情数据到最新。”。
5. 用 Rearview/ClickHouse 交易日历解析 `planned_live_start_date = next_trade_date(source_signal_date)`；查不到时返回 `can_publish=false` 和 blocker，不回退到 T。
6. 针对 `source_signal_date` 单日编译 Step 1/2 规则，生成 TopN `pending_buy_signals`，并为每条信号填入 `execution_date = planned_live_start_date`。
7. 不读取 source backtest 的 latest `portfolio_target` 作为发布信号来源，因为 backtest 会丢弃尾日 T+1 信号。

#### 创建组合接口

保留当前：

```http
POST /rearview/strategy-portfolios
```

请求建议追加预检校验字段：

```json
{
  "source_strategy_backtest_run_id": "01J...",
  "source_result_attempt_id": "01J...",
  "name": "策略组合",
  "expected_required_source_signal_date": "2026-06-26",
  "expected_source_signal_date": "2026-06-26",
  "expected_live_start_date": "2026-06-29",
  "client_request_id": "..."
}
```

创建接口职责：

1. 重新执行发布预检，不能信任前端传入日期。
2. 如果后端解析出的 `required_source_signal_date`、`source_signal_date` 或 `live_start_date` 与 expected 字段不一致，返回 `409 Conflict`，要求前端刷新弹层。这样可以处理用户在 15:00 前打开弹层、15:00 后才点击确定的边界。
3. 创建 `strategy_portfolio` 时保存：
   - `live_start_date = planned_live_start_date`
   - `initial_signal_date = source_signal_date`
   - `pending_buy_signal_snapshot`
   - source backtest 的 period、benchmark 和 result attempt 元数据
   第一版可以落在新增 PostgreSQL 表，也可以落在职责受控的列；如果暂存在 JSONB，也必须属于 `strategy_portfolio_publish_context` 语义，不能放进可被 UI 随意覆盖的 display snapshot。
4. 创建组合只复制 source backtest 的规则、执行配置、benchmark、period 和展示快照；不得复制 source backtest 的持仓、现金、成本、订单、成交、事件、净值或绩效状态。
5. 创建成功后 `latest_daily_run_id` 和 `current_live_result_attempt_id` 仍为空，`live_status = pending_first_run`。

#### Dashboard read model

`GET /rearview/strategy-portfolios/dashboard` 的目标态：

Dashboard response 应显式暴露两段来源，避免前端用 `source_backtest` 补 live 空缺：

```json
{
  "backtest_segment": {
    "source_strategy_backtest_run_id": "01J...",
    "source_result_attempt_id": "01J...",
    "period_key": "1y",
    "start_date": "2025-06-26",
    "end_date": "2026-06-26",
    "benchmark_security_code": "000903.SH"
  },
  "live_segment": {
    "live_status": "pending_first_run",
    "live_start_date": "2026-06-29",
    "latest_daily_run_id": null,
    "current_live_result_attempt_id": null,
    "performance_source": "none",
    "signal_source": "publish_preview"
  }
}
```

| 状态 | 业绩字段 | 信号字段 |
|---|---|---|
| `pending_first_run` | `latest_nav = null`、`recent_change = null`、`returns/risk/efficiency/relative = []`、`curve = []` | 返回 `pending_buy_signals` 或 `today_signals`，每条包含 `signal_date` 和 `execution_date`。 |
| `queued/running` | 同 pending，可展示运行状态 | 继续展示 pending/last available signals。 |
| `succeeded` | 只读取 latest daily run result attempt 的 nav/performance/curve | 信号读取 latest daily run result attempt 的 latest target。 |
| `failed` | 不回退 source backtest 业绩 | 展示错误状态和可用的 pending/last signal。 |

字段建议：

```json
{
  "live_status": "pending_first_run",
  "performance_source": "none",
  "signal_source": "publish_preview",
  "live_start_date": "2026-06-29",
  "latest_nav": null,
  "returns": [],
  "risk": [],
  "efficiency": [],
  "relative": [],
  "curve": [],
  "pending_buy_signals": [
    {
      "code": "600000.SH",
      "name": "浦发银行",
      "score": 91.4,
      "rank": 1,
      "signal_date": "2026-06-26",
      "execution_date": "2026-06-29"
    }
  ]
}
```

前端看板规则：

1. `performance_source = none` 时，不展示“最新净值”“收益指标”“风险指标”“净值与基准”里的真实数值；展示 pending 文案或空态。
2. `pending_buy_signals` 标题不写“今日信号”，改为“待调入信号”或“买入信号”。
3. 每条信号展示 `signal_date -> execution_date`，表达 T 日产生、T+1 调入。

#### Daily run 数据修正

`strategy_portfolio_daily_run` 第一版仍可保留全窗口重算，但窗口起点需要区分：

| 字段 | 建议语义 |
|---|---|
| `strategy_portfolio.live_start_date` | 正式建仓日，T+1。 |
| `strategy_portfolio.initial_signal_date` | 首批买入信号日，T。 |
| `strategy_portfolio_daily_run.run_start_date` | 信号/模拟窗口起点，首版建议固定为 `initial_signal_date`，确保 T 信号能在 T+1 执行。 |
| `strategy_portfolio_daily_run.trade_date` | 本次 daily run 截止交易日。 |

示例：

```text
source_signal_date = 2026-06-26
live_start_date = 2026-06-29

daily run for 2026-06-29:
  run_start_date = 2026-06-26
  trade_date = 2026-06-29
  initial portfolio state = cash only, no inherited holdings
  signal 2026-06-26 -> execution 2026-06-29
```

这样可以复用现有 `simulate_portfolio()` 的信号执行行为：它先在 `run_start_date` 写入初始现金 nav，再从下一个交易日开始执行买入；首个执行日正好是 `live_start_date`。但写入 live result attempt 前必须做 live 段归一化：丢弃或隔离 `initial_signal_date` 的种子 nav，并以 `live_start_date` 重新计算策略净值、基准净值和全部业绩指标。

### ClickHouse 数据面校验

本设计不建议在 ClickHouse 中写入“尚未建仓”的假 nav、假 performance 或假 target result attempt。pending 信号必须在创建组合时冻结为 PostgreSQL control plane 的发布快照；发布预检接口的即时计算只服务创建前展示，创建成功后 Dashboard 读取持久化快照，避免 metric catalog 或规则解释变化导致 pending 信号漂移。

首个 daily run succeeded 后，写入 ClickHouse 的 live result attempt 必须已经是基于 `live_start_date` 归 1 后的事实数据。API 层不负责临时把 source backtest 或模拟种子 nav 重标为 live nav；查询只读取已经入库的 live result attempt。

规则校对：

| 规则 | 判断 |
|---|---|
| Per `schema-pk-plan-before-creation` | 本设计要求物理拆成 backtest/live 两套 ClickHouse result family；实施前必须分别列出各自 top query patterns，再设计 ORDER BY，不能复制旧统一 `portfolio_*` 表当作最终答案。 |
| Per `schema-pk-prioritize-filters` | `fleur_backtest.backtest_target` 的主过滤应是 `strategy_backtest_run_id + result_attempt_id`；`fleur_portfolio.live_target` 的主过滤应是 `strategy_portfolio_daily_run_id + result_attempt_id`。两套 target 表都应围绕实际 API filter 选排序键。 |
| Per `schema-pk-filter-on-orderby` | Backtest API 查询必须使用 `strategy_backtest_run_id` 和 `result_attempt_id` 前缀；portfolio live API 查询必须使用 `strategy_portfolio_daily_run_id` 和 `result_attempt_id` 前缀。禁止 live 查询先扫 backtest family 再在 API 层过滤。 |
| Per `schema-partition-low-cardinality` | `backtest_*` 和 `live_*` 的 nav、target、position、trade、event 等时序事实优先按月分区；禁止按 run id、portfolio id 或 strategy id 做高基数分区。 |
| Per `insert-mutation-avoid-update` | pending 状态不写假事实；daily run succeeded 后以新的 result attempt append-only 写入已归一化的 live nav/performance，避免用 ClickHouse mutation 修正占位业绩或旧归一化口径。 |
| Per `insert-batch-size` | backtest 和 live writer 都按 run 批量写入 nav、target、order、trade、position、event 和 performance facts，不引入单行补写或逐指标补写。 |

## 交互规则

| 场景 | 建议行为 |
|---|---|
| 打开弹层 | 默认进入 Tab 1「策略配置」。 |
| 策略名称为空 | 底部「确定」禁用。 |
| 15:00 前，上一交易日信号可用 | 显示最后信号日和计划建仓日。 |
| 15:00 后，当天信号可用 | 显示计划建仓日，并按当前 T+1 建仓规则允许确定。 |
| 行情数据未达到允许信号日 | 显示“最后信号日与最新行情日存在缺口，请先回填行情数据到最新。”，禁用「确定」。 |
| T+1 建仓日期未返回 | 显示 `待交易日历确认`，并禁用「确定」或在创建 API 中由后端最终校验。建议优先禁用，避免发布时口径不透明。 |
| 弹层打开后跨过 15:00 | 创建接口重新预检；如 required date 已变化，返回 `409 Conflict`，前端提示刷新发布预检。 |
| 回测业绩读取中 | Tab 2 展示 skeleton；Tab 1 仍可浏览配置。 |
| 回测业绩读取失败 | Tab 2 展示错误，并禁用「确定」；发布确认必须能看到回测业绩简报。 |
| 创建失败 | 底部上方展示当前错误 alert，保留在当前 tab。 |
| 创建成功 | 沿用当前行为，关闭弹层并跳转 `/dashboard`。 |

## 已定稿问题

1. 发布预检生成的 `pending_buy_signals` 创建成功后必须持久化为发布时快照；Dashboard 不重新计算 pending 信号。
2. 回测业绩读取失败时不允许用户创建组合；弹层必须能展示 Tab 2 简报后才能确认发布。
3. 第一版不加第三个 tab「运行设置」；当前需求只保留 `策略配置` 和 `回测业绩`，避免把发布弹层扩成完整详情页。
4. 盘中只允许回退到上一交易日信号，收盘后要求当天信号；不支持在数据落后多日时继续向前顺延。

## 实施提示

后续改代码时，建议拆成前端信息架构和后端口径修正两条线：

1. 前端弹层复用现有 `Tabs`、`TabsList`、`TabsTrigger`、`TabsContent`。
2. 前端复用当前 `publishConditionRows`、`publishScoringRows`、`publishPerformanceGroups` 和 `effectiveSimulationSettings`。
3. 将弹层内重复 card 拆成局部 render helper 或小组件，避免继续扩大 `strategy-page.tsx` 的单文件体积。
4. T+1 建仓日期不要在前端按自然日推导；从发布预检接口读取 `planned_live_start_date`，缺字段时禁用确定。
5. 后端新增发布预检接口，按 `Asia/Shanghai` 和 15:00 cutoff 解析 `required_source_signal_date`；创建组合时重新校验 `required_source_signal_date`、`source_signal_date` 和 `live_start_date`。前端 UI 只展示最后信号日和计划建仓日。
6. Dashboard pending 首次运行时不要消费 source backtest nav/performance/curve；只展示 `pending_buy_signals` 和建仓日。
7. Daily run 创建时把信号窗口起点设为 `initial_signal_date`，确保首批 T 日信号能在 T+1 执行。
8. Live daily run 写入前必须以 `live_start_date` 将策略净值、基准净值和业绩指标重新归 1 并重新计算入库；`initial_signal_date` 的种子 nav 不进入 live 展示事实。
9. Live daily run 的初始组合状态必须是空持仓和配置中的初始资金，不继承 source backtest 的持仓、现金、成本、订单、成交、事件或绩效状态。

## 最小验证

本文是文档-only RFC。提交前至少运行：

```bash
make docs-check
git diff --check
```
