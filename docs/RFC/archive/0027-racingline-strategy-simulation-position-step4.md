# RFC 0027: Racingline 模拟建仓 Step 4 实现方案

状态：Archived（2026-06-25；归档前状态：Implemented）
领域：racingline, rearview
关联系统：racingline, rearview
代码根：app/racingline_new/, app/racingline/, engines/crates/rearview-core/, engines/crates/rearview-server/, engines/crates/rearview-portfolio-worker/
系统地图：docs/architecture/racingline.md
实现报告：docs/jobs/reports/2026-06-23-racingline-strategy-step4-draft-handoff.md

路径说明：本文写于 Plan 0053 迁移前；文中的 `app/racingline_new/` 均为历史实现路径，当前 Racingline 前端代码根为 `app/racingline/`。

## 摘要

本文档定义 `/strategies` Step 4「模拟建仓」的业务边界、现有资源、实现缺口和补齐方案。

前三步当前已经形成真实接口闭环：

```text
Step 1 策略选股
  记录股票池筛选条件 pool_filters
  不真正生成股票池

Step 2 权重配置
  记录候选池内个股评分规则 scoring.rules
  不真正打分和排名

点击「股池预览」
  组合 Step 1 + Step 2 草稿
  执行选股、评分和排名

Step 3 股池预览
  展示一次真实 preview execution 的 applied snapshot
  检查候选池规模、排名、得分项、筛选指标、K 线和个股上下文
```

Step 4 位于 Step 3 和 Step 5 之间。它不再配置选股条件，也不执行回测；它只记录后续回测如何把每日评分排名结果转成虚拟持仓、订单、成交和卖出规则。

正确流程是：

```text
Step 3 applied preview snapshot
  + Step 4 建仓/费用/卖出规则草稿
  -> BacktestExecutionDraft

Step 5 策略回测
  选择回测区间和基准
  用 Step 1/2 applied rule + Step 4 execution config
  重新执行历史选股、评分、TopN 买入信号、组合模拟和绩效计算
```

其中 `Top N` 是 Step 4 的用户输入，表示后续回测每天从评分排名中取多少只作为买入信号来源；它不由 Step 3 提供，也不属于 Step 3 设计范围。

## 背景

[Q&A 0004](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md) 定义了策略创建流程：策略选股、权重配置、股池预览、模拟建仓、策略回测和运行策略。

[RFC 0024](0024-racingline-strategy-selection-step1.md)、[RFC 0025](0025-racingline-strategy-weight-configuration-step2.md) 和 [RFC 0026](0026-racingline-strategy-pool-preview-step3.md) 已把前三步边界收敛为：

- Step 1/2 记录规则草稿。
- Step 3 点击 preview 后才真实执行选股、评分和排名。
- Step 3 不提供 `topN`，也不提供建仓参数。

Step 4 要回答的问题是：

> 如果 Step 3 检查过的规则进入历史回测，系统应按什么资金、仓位、交易成本和卖出规则构建虚拟组合？

## 目标

1. 明确 Step 4 是“记录回测执行参数”，不是执行回测。
2. 盘点当前 Step 4 前端原型、旧 Racingline 账户模板和 Rearview portfolio engine 的可复用能力。
3. 设计 `SimulationSettings -> BacktestExecutionConfig` 的前端 adapter 和后端 contract 形态。
4. 明确 Step 4 的 `buyTopN` 是 Step 5 执行历史信号时的 TopN 参数，不来自 Step 3。
5. 明确 Step 4 只能基于非 stale 的 Step 3 applied preview snapshot 进入。
6. 为 Step 5 回测执行准备稳定输入：applied rule、TopN、资金、仓位上限、费率、滑点、卖出规则和可复现 snapshot。
7. 保持边界：Racingline 不在浏览器内计算权威成交、持仓、费用、滑点、净值或绩效。

## 非目标

1. 不在 Step 4 发起 `POST /rearview/runs`、`POST /rearview/portfolio-runs` 或任何回测执行。
2. 不在 Step 4 创建 rule set、rule version、run、portfolio run 或正式策略。
3. 不在 Step 4 展示回测净值、绩效指标、持仓记录或交易记录。
4. 不把 Step 3 preview response 当作 Step 5 的历史回测数据源；Step 5 必须按回测区间重新执行规则。
5. 不在浏览器内重算 Step 3 结果，也不在浏览器内生成权威买入信号。
6. 不支持自定义公式、任意 SQL、任意脚本或前端自定义指标作为卖出规则。
7. 第一版不实现实盘交易、券商清算、企业行动真实持仓数量调整、多币种或多资产类型。
8. 第一版不把 indicator stop loss 宣称为已可执行能力，除非 Rearview 已提供受控指标退出规则。

## 业务流程定义

### Step 4 的输入

Step 4 必须从一个非 stale 的 Step 3 applied preview snapshot 进入：

```text
PreviewSnapshot {
  preview_id
  applied_rule_spec
  start_date
  end_date
  trade_dates[]
  stale = false
}
```

如果 Step 1 或 Step 2 草稿在 preview 后发生变化，Step 3 snapshot 会进入 stale 状态。此时 Step 4 仍可展示当前草稿，但不能允许进入 Step 5 执行回测，除非用户重新点击「更新股池」并得到新的 applied snapshot。

### Step 4 的输出

Step 4 输出的是一个本地的、可序列化的回测执行配置快照：

```text
BacktestExecutionDraft {
  applied_rule_spec
  preview_context
  position_config_snapshot
  created_at
  stale
}
```

其中 `position_config_snapshot` 包含：

- 初始资金和币种。
- Step 4 用户输入的 `buy_signal_top_n`。
- 单票仓位上限和目标权重策略。
- A 股手数约束。
- 交易费率、最低佣金和滑点。
- 卖出规则。
- 执行时点和价格口径。

### Step 5 的输入边界

Step 5 不应直接读取 Step 1/2 当前 draft，也不应读取 Step 3 表格行作为历史回测数据。它的执行输入应来自：

```text
Step 3 PreviewSnapshot.applied_rule_spec
  + Step 4 BacktestExecutionConfig
  + Step 5 BacktestRange / Benchmark
  -> BacktestExecutionRequest
```

Step 5 可以选择后端实现形态：

1. 新增 first-class strategy backtest API，直接接收 `rule + top_n + execution_config + range`。
2. 或在后端内部创建临时/正式 rule version、run，再创建 portfolio simulation。

无论后端实现选择哪种，Step 4 只负责生成 execution config，不负责持久化和执行。

## 当前资源盘点

### 前端资源

| 资源 | 路径 | 当前能力 | Step 4 价值 | 缺口 |
|---|---|---|---|---|
| Step 流程 | `app/racingline_new/src/routes/strategy-page.tsx` | 已有 `preview -> simulation -> backtest` 导航，Step 4 接收 `previewAppliedWeightIndicators` 和 `conditionGroups` | 可承接 Step 3 applied snapshot 后进入 Step 4 | Step 4 还没有独立 snapshot/gate，也没有生成 Step 5 request |
| Step 4 类型 | `app/racingline_new/src/features/strategy/types.ts` | `SimulationSettings` 覆盖初始资金、TopN、单票上限、费率、滑点、止盈止损、时间止损和指标止损 | 可作为 Step 4 草稿模型起点 | 字段是前端表单语义，不是后端 execution contract |
| Step 4 面板 | `app/racingline_new/src/features/strategy/components/simulation-position-panel.tsx` | 已展示仓位管理、交易费率、风险管理和建仓摘要 | UI 信息结构基本可复用 | 仍使用静态 `indicatorCatalog`；摘要中的信号趋势是本地估算，不是真实 backtest 输入 |
| 共享三栏布局 | `app/racingline_new/src/features/strategy/components/strategy-split-panel.tsx` | Step 3/4/5 已复用同一左右面板布局和分隔线 | 可保持 Step 4 与 Step 5 对齐 | 只是布局能力，不解决数据 contract |
| Preview snapshot | `app/racingline_new/src/features/strategy/preview.ts` | 已有 `PreviewSnapshot`、presentation adapter 和 stale 语义 | Step 4 gate 的事实来源 | Step 4 尚未把非 stale snapshot 固化进 backtest draft |
| 旧账户模板 UI | `app/racingline/src/features/portfolio/components/account-template-card.tsx` | 已把默认市场模板、账户模板、费率、滑点和 max positions 映射到 API request | 可复用 fee/slippage/rebalance/risk policy 命名经验 | 绑定 `rule_set_id` 和正式账户模板，不适合直接用于策略创建流未保存状态 |
| 旧 portfolio 页面 | `app/racingline/src/routes/PortfolioDetailPage.tsx` | 已展示 portfolio run、nav、targets、orders、trades、positions 和 events | Step 5 回测结果展示可复用信息结构 | 依赖已存在 `portfolio_run_id`，Step 4 不应创建 |

### Rearview 后端资源

| 资源 | 路径 | 当前能力 | Step 4/5 价值 | 缺口 |
|---|---|---|---|---|
| 默认市场费率模板 | `GET /rearview/market-fee-templates/default` | 返回 `fee_profile`、`slippage_profile`、币种和市场默认配置 | Step 4 默认值来源 | `app/racingline_new` 尚未接入，当前使用本地默认百分比 |
| Account template API | `GET/POST /rearview/rule-sets/{rule_set_id}/account-templates` | 管理正式 rule set 下的虚拟账户模板 | 运行策略阶段可复用 | Step 4 发生在策略未保存前，不应强依赖 `rule_set_id` |
| Portfolio run API | `POST /rearview/portfolio-runs` | 以 `source_run_id + account_template_id` 创建异步组合运行 | Step 5 可以作为后端内部实现参考 | 无法直接消费 `RuleVersionSpec + Step4 config`，也不能在 Step 4 调用 |
| Portfolio simulation engine | `engines/crates/rearview-core/src/portfolio/mod.rs` | 支持 `initial_cash`、`max_positions`、A 股手数、fee、slippage、fixed stop loss、take profit、time stop loss、signals 和 backward adjusted prices | Step 5 组合账本计算核心 | 不支持单票仓位 cap 作为一等字段；不支持 indicator stop loss |
| Portfolio ClickHouse facts | `engines/crates/rearview-core/src/clickhouse/portfolio_schema.rs`、`portfolio_write.rs` | 已定义 nav、position、trade、order、target、event 等结果事实写入 | Step 5 结果展示和持久化基础 | Step 4 不写这些表；Step 5 需决定是否复用 portfolio_run 或新增 backtest namespace |
| NATS worker | `engines/crates/rearview-portfolio-worker/` | 已承担异步组合计算 | Step 5 可复用异步执行模型 | 当前任务从 `portfolio_run_id` 读取 PostgreSQL 快照，不接受 transient request |

### 数据层资源

| 资源 | 路径 | 当前能力 | Step 4/5 价值 | 缺口 |
|---|---|---|---|---|
| 日频行情 mart | `pipeline/elt/models/marts/mart_stock_quotes_daily.sql` | 提供后复权和原始行情字段 | Step 5 计算成交参考价和持仓估值 | Step 4 只配置，不读取 |
| Step 3 preview APIs | Rearview preview-only endpoints | 已能按规则执行近一年选股、评分和排名 | Step 4 用它作为“已检查过规则”的 gate | Step 5 仍需按回测区间重新执行 |
| Portfolio result facts | `fleur_portfolio` ClickHouse tables | 保存组合 nav、target、order、trade、position 和 event | Step 5 结果持久化候选 | Step 4 只产出 config，不落结果 |

### 文档和验收资源

| 资源 | 当前价值 |
|---|---|
| [Q&A 0004](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md) | 定义 Step 4 在策略创建闭环中的位置 |
| [RFC 0021](0021-racingline-virtual-account-portfolio-rebalancing.md) | 定义虚拟账户、费率、滑点、调仓、止盈止损、T+1 成交和研究型净值边界 |
| [RFC 0026](0026-racingline-strategy-pool-preview-step3.md) | 定义 Step 3 applied snapshot 和 stale gate |
| [Plan 0041](../../plans/archive/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md) | 提供 portfolio run/worker 的实现背景 |
| [System: Racingline](../../architecture/racingline.md) | 确认 `app/racingline_new` 是策略创建工作台 |
| [System: Rearview](../../architecture/rearview.md) | 确认 Rearview 已有 preview-only API 和 portfolio APIs |

## 实施缺口与填充方案

| 缺口 ID | 缺口 | 影响 | 填充方案 |
|---|---|---|---|
| G1 | Step 4 没有独立 `BacktestExecutionDraft`。 | Step 5 容易读取当前表单散落状态，无法保证可复现。 | 新增前端快照模型，包含 applied rule、Step4 config、hash、created_at 和 stale 标记。 |
| G2 | Step 4 进入条件没有绑定非 stale PreviewSnapshot。 | 用户修改 Step1/2 后可能用未预览规则进入回测。 | Step4/Step5 gate 必须要求 `previewSnapshot && !previewSnapshot.stale`。 |
| G3 | `buyTopN` 语义未与 Step5 执行 contract 对齐。 | 可能再次把 TopN 错误归入 Step3。 | 定义 `buy_signal_top_n` 为 Step4 字段；Step5 用它作为 run/backtest 的 `top_n`。 |
| G4 | 单票仓位上限与 backend `max_positions` 模型不一致。 | 前端展示“单票上限”，后端只支持等权 `max_positions` 会造成结果不一致。 | 第一版 contract 增加 `single_position_limit_pct`；Step5 backend 要么实现 cap，要么明确用 adapter 推导 `cash_reserve_pct` 并在 UI 中展示等价约束。 |
| G5 | 费率和滑点默认值来自本地常量。 | 与 Rearview 默认市场模板可能漂移。 | Step4 loading 时读取 `GET /rearview/market-fee-templates/default?market=CN_A_SHARE`，接口失败显示 error，不使用 mock 成功兜底。 |
| G6 | 前端百分比字段和后端小数字段没有 adapter。 | 0.05% 可能被错误提交为 0.05。 | 新增 `simulationSettingsToExecutionConfig()`，统一把 percent 转成 decimal/bps。 |
| G7 | indicator stop loss UI 已存在，但 portfolio engine 不支持。 | 用户可能配置无法执行的卖出规则。 | 第一版禁用或隐藏 indicator stop loss，或后端补受控 `MetricExitRule` 后再开放。 |
| G8 | Step4 摘要中的“近三月信号数”是本地估算。 | 用户可能误以为这是 Step5 的真实历史信号。 | 第一版摘要只显示配置摘要和 Step3 preview summary；真实信号统计留给 Step5 执行结果。 |
| G9 | 正式 `portfolio_run` API 需要 `source_run_id`。 | 策略创建流在 Step5 前没有 source run。 | Step5 需要新增 backtest execution API，或后端内部先创建 run 再创建 portfolio run；Step4 只输出中立 config。 |
| G10 | Account template 绑定 `rule_set_id`。 | 未保存策略不能直接持久化账户模板。 | Step4 使用 transient execution config；运行策略/保存策略时再转换成 account template。 |
| G11 | Step4 config 缺少 hash/version。 | Step5 结果无法证明使用了哪版建仓参数。 | 生成 `execution_config_hash`，Step5 request 和结果 snapshot 都携带。 |
| G12 | 当前测试覆盖不足。 | 后续 UI 改动容易破坏 TopN、百分比和 fee 转换。 | 增加 adapter 单测、gate 单测、默认模板错误态测试和 Playwright smoke。 |

## 设计

### D1: Step 4 输出 BacktestExecutionDraft

前端新增领域模型：

```ts
type BacktestExecutionDraft = {
  previewId: string
  appliedRuleSpec: RuleVersionSpec
  previewRange: {
    startDate: string
    endDate: string
  }
  positionConfig: BacktestExecutionConfig
  executionConfigHash: string
  createdAt: string
  stale: boolean
}
```

`stale` 规则：

1. Step 4 配置变化后，当前 Step 4 draft 需要重新生成 hash。
2. Step 1/2 修改导致 Step 3 stale 后，Step 4 draft 也视为 stale。
3. Step 5 只能消费 `previewSnapshot.stale = false` 且 `BacktestExecutionDraft.stale = false` 的配置。

### D2: Canonical BacktestExecutionConfig

Step4 表单不直接提交。它必须先转换成 canonical config：

```ts
type BacktestExecutionConfig = {
  market: "CN_A_SHARE"
  account: {
    initialCash: number
    currency: "CNY"
  }
  signalPolicy: {
    buySignalTopN: number
    signalTiming: "close_confirm_next_open"
  }
  rebalancePolicy: {
    targetWeighting: "equal_weight_capped"
    maxPositions: number
    singlePositionLimitPct: number
    cashReservePct: number
    lotSize: 100
    minTradeLots: 1
    emptySignalAction: "hold"
  }
  feeProfile: {
    commissionRate: number
    commissionRateMax: number
    minCommission: number
    stampDutyRateSell: number
    transferFeeRate: number
  }
  slippageProfile: {
    mode: "bps"
    buyBps: number
    sellBps: number
  }
  riskExitPolicy: {
    triggerTiming: "close_confirm_next_open"
    exitRules: ExitRuleConfig[]
  }
  priceBasis: "backward_adjusted"
}
```

字段映射：

| Step4 UI 字段 | Canonical 字段 | 转换规则 |
|---|---|---|
| `initialCapital` | `account.initialCash` | 原值，必须 `> 0` |
| `buyTopN` | `signalPolicy.buySignalTopN` 和 `rebalancePolicy.maxPositions` | 整数，必须 `>= 1` |
| `singlePositionLimitPercent` | `rebalancePolicy.singlePositionLimitPct` | 百分比，范围 `(0, 100]` |
| `stampDutyRatePercent` | `feeProfile.stampDutyRateSell` | 除以 100 |
| `transferFeeRatePercent` | `feeProfile.transferFeeRate` | 除以 100 |
| `commissionRatePercent` | `feeProfile.commissionRate` | 除以 100 |
| `slippageRatePercent` | `slippageProfile.buyBps/sellBps` | `percent * 100` |
| `fixedStopLoss.lossPercent` | `ExitRule.FixedStopLoss.lossPct` | 除以 100 |
| `takeProfit.profitPercent` | `ExitRule.TakeProfit.profitPct` | 除以 100 |
| `timeStopLoss` | `ExitRule.TimeStopLoss` | `holdingDays` 原值，`minimumReturnPercent` 除以 100 |

### D3: TopN 只属于 Step 4/5

Step 3 不提供 TopN。Step 4 的 `buySignalTopN` 是 Step 5 执行历史回测时使用的参数：

```text
BacktestExecutionRequest.top_n = BacktestExecutionConfig.signalPolicy.buySignalTopN
```

执行语义：

1. Step5 对每个回测交易日重新执行 `appliedRuleSpec`。
2. 在完整候选池中按 `score DESC, security_code ASC` 排名。
3. 取 Step4 `buySignalTopN` 作为当日买入信号来源。
4. 后续按 Step4 `rebalancePolicy` 转成目标持仓和订单。

Step 3 的 `pool-page limit = 10`、横向交易日窗口和表格展示行数都不能影响 Step5 TopN。

### D4: 单票上限和等权目标

第一版用户输入包含 `buyTopN` 和 `singlePositionLimitPercent`。推荐 canonical 语义：

```text
raw_equal_weight = 1 / buySignalTopN
target_weight_per_position = min(raw_equal_weight, singlePositionLimitPct / 100)
cash_reserve_pct = 1 - target_weight_per_position * buySignalTopN
```

例子：

| TopN | 单票上限 | 单票目标权重 | 现金保留 |
|---:|---:|---:|---:|
| 10 | 10% | 10% | 0% |
| 20 | 10% | 5% | 0% |
| 5 | 10% | 10% | 50% |

如果 Step5 第一版决定复用当前 `PortfolioSimulationInput.max_positions + cash_reserve_pct`，则 backend adapter 必须按上述公式推导 `max_positions = buySignalTopN` 和 `cash_reserve_pct`。如果需要更直接表达，应在 portfolio engine 中新增 `single_position_limit_pct`。

### D5: 默认市场费率模板

Step4 初次加载时调用：

```http
GET /rearview/market-fee-templates/default?market=CN_A_SHARE
```

返回值用于初始化：

- `currency`
- `commission_rate`
- `commission_rate_max`
- `min_commission`
- `stamp_duty_rate_sell`
- `transfer_fee_rate`
- `buy_bps`
- `sell_bps`

规则：

1. 已加载过的用户草稿不被后续 refetch 覆盖。
2. 接口 loading 时费率区域展示 loading。
3. 接口失败时展示 error，不允许用本地假默认值伪装为成功。
4. 单元测试 fixture 可以保留本地默认值，但必须与用户成功路径隔离。

### D6: 卖出规则第一版边界

Step4 第一版允许：

```json
[
  { "type": "fixed_stop_loss", "loss_pct": 0.08 },
  { "type": "take_profit", "profit_pct": 0.2 },
  {
    "type": "time_stop_loss",
    "holding_days": 20,
    "max_return_pct": 0
  }
]
```

Step4 第一版不默认开放：

```json
{ "type": "indicator_stop_loss" }
```

原因是当前 `PortfolioSimulationInput.ExitRule` 只支持 `FixedStopLoss`、`TakeProfit` 和 `TimeStopLoss`。如果需要指标止损，必须另行补充：

1. 指标 catalog allowlist。
2. 后端受控 metric lookup。
3. 与后复权价格口径兼容的触发时点。
4. 失败和缺失指标的事件记录。

### D7: Step5 BacktestExecutionRequest 草案

Step4 不实现 Step5 API，但需要为 Step5 准备稳定输入。推荐 Step5 request 草案：

```json
{
  "rule": {},
  "start_date": "2025-06-22",
  "end_date": "2026-06-22",
  "top_n": 10,
  "execution_config": {
    "market": "CN_A_SHARE",
    "account": {
      "initial_cash": 1000000,
      "currency": "CNY"
    },
    "rebalance_policy": {
      "target_weighting": "equal_weight_capped",
      "max_positions": 10,
      "single_position_limit_pct": 0.1,
      "cash_reserve_pct": 0,
      "lot_size": 100,
      "min_trade_lots": 1,
      "empty_signal_action": "hold"
    },
    "fee_profile": {},
    "slippage_profile": {},
    "risk_exit_policy": {},
    "price_basis": "backward_adjusted"
  },
  "benchmark": "000300.SH"
}
```

Step5 响应应能被现有 portfolio result UI 消费，至少包含：

- execution id / portfolio run id / backtest id。
- status。
- nav。
- summary。
- targets/orders/trades/positions/events 查询入口。
- `rule_hash` 和 `execution_config_hash`。

### D8: 保存策略与运行策略的衔接

Step4 的 transient config 不能自动成为正式账户模板。后续「运行策略」时才需要持久化：

```text
BacktestExecutionConfig
  -> create rule_set
  -> create rule_version
  -> create or update default account_template
  -> create scheduled/observable portfolio strategy
```

因此 Step4 第一版只需要保证 config 可序列化、可 hash、可提交给 Step5，不要求创建或更新 account template。

## API contract 汇总

| API | 状态 | Step 4 用途 |
|---|---|---|
| `GET /rearview/market-fee-templates/default` | 已存在 | 获取 CN_A_SHARE 费率和滑点默认值 |
| `GET /rearview/metrics` | 已存在 | 如果后续开放 indicator stop loss，用于受控指标选择；第一版不必开放 |
| `POST /rearview/strategy-preview/*` | 已存在 | Step4 gate 的上游事实来源，不在 Step4 重新调用 |
| `POST /rearview/runs` | 已存在 | Step5 后端实现可复用，不由 Step4 调用 |
| `POST /rearview/portfolio-runs` | 已存在 | 正式 source run 后的组合运行入口，不由 Step4 调用 |
| `POST /rearview/strategy-backtests` | 待设计 | Step5 推荐新增入口，消费 Step4 execution config |

## 初步实现路径

### Phase 1: Step4 config model 和 adapter

目标：把本地 `SimulationSettings` 转成 canonical `BacktestExecutionConfig`。

任务：

1. 新增 `BacktestExecutionConfig`、`BacktestExecutionDraft` 和 `ExitRuleConfig` 类型。
2. 实现 `simulationSettingsToExecutionConfig(settings, marketTemplate)`。
3. 实现 `hashBacktestExecutionConfig(config)`。
4. 增加 percent/decimal/bps 转换测试。

完成标准：

- Step4 表单字段和 Step5 request 字段一一对应。
- `buyTopN` 只映射到 Step5 top_n / max positions，不影响 Step3。

### Phase 2: 默认市场模板接入

目标：Step4 默认费率和滑点来自 Rearview。

任务：

1. 在 `app/racingline_new` 增加 default market fee template API client/hook。
2. Step4 首次进入时用 Rearview 默认模板初始化草稿。
3. 处理 loading/error/empty 状态。
4. 移除用户成功路径中的本地费率成功 fallback。

完成标准：

- 断开 Rearview 时 Step4 不显示“已加载默认费率”的假成功状态。
- 用户修改后的费率不会被 refetch 覆盖。

### Phase 3: Step4 gate 和 snapshot

目标：让 Step4/Step5 只消费非 stale applied preview。

任务：

1. Step4 读取 `PreviewSnapshot`，没有 snapshot 时展示进入前置条件。
2. `previewSnapshot.stale = true` 时禁用进入 Step5。
3. Step4 配置变化生成新的 `BacktestExecutionDraft`。
4. Step5 按 draft hash 判断是否需要重新执行。

完成标准：

- 修改 Step1/2 后不能直接执行 Step5。
- 点击「更新股池」成功后 gate 恢复。

### Phase 4: UI 语义收敛

目标：Step4 页面只展示真实配置摘要，不展示伪造信号统计。

任务：

1. 保留仓位管理、交易费率、风险管理和建仓摘要。
2. 摘要展示 initial cash、TopN、单票上限、目标单票权重、现金保留、费用摘要和卖出规则。
3. 移除或重命名本地估算的“近三月信号数”，除非由真实 Step3/Step5 数据提供。
4. 禁用或隐藏 indicator stop loss，直到后端支持。

完成标准：

- 页面没有把本地估算伪装成真实历史信号。
- Step4 文案和字段都指向 Step5 回测执行参数。

### Phase 5: Step5 contract 对接准备

目标：让 Step5 可以直接消费 Step4 draft。

任务：

1. 在 Step5 RFC 中固定 backtest execution API。
2. Step4 将 `BacktestExecutionDraft` 传入 Step5。
3. Step5 执行时提交 `rule + top_n + execution_config + range + benchmark`。
4. Step5 结果必须返回 `rule_hash` 和 `execution_config_hash`。

完成标准：

- Step5 不再读取散落表单状态。
- 同一 rule + config + range 可复现同一执行输入。

## 测试和验收建议

文档阶段：

```bash
make docs-check
git diff --check
```

前端实现阶段：

```bash
cd app/racingline_new
npm run typecheck
npm run lint
npm test
npm run build
```

涉及 Rearview backtest/portfolio API 时追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

浏览器验收至少检查：

- Step4 从非 stale Step3 preview 进入。
- Step1/2 修改后 Step4/Step5 gate 被 stale 拦住。
- Step4 默认费率来自 `GET /rearview/market-fee-templates/default`。
- Step4 `buyTopN` 修改后只改变 Step5 execution draft，不改变 Step3 表格。
- Step4 百分比输入转换为后端 decimal/bps 后无 100 倍误差。
- Step4 禁用或隐藏当前不可执行的 indicator stop loss。
- 断开 Rearview 后不会用 mock 默认费率伪装成功。

## 风险与待决问题

1. Step5 是否新增 `POST /rearview/strategy-backtests`，还是内部组合 `runs + portfolio-runs`，需要在 Step5 RFC 固定。
2. `single_position_limit_pct` 是否进入 portfolio engine 原生模型，还是由 adapter 推导为 `cash_reserve_pct`，需要实现阶段选择。
3. indicator stop loss 是否进入第一版，取决于 Rearview 是否能提供受控指标退出规则。
4. Step4 transient config 在页面刷新后是否恢复，不在第一版范围；如果需要，需要草稿持久化策略。
5. `app/racingline_new` 仍是并行策略创建工作台，正式迁移到 `app/racingline` 时需要遵守 ADR 0011 和 ADR 0013。

## 相关文档

- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](0021-racingline-virtual-account-portfolio-rebalancing.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](0023-racingline-frontend-prototype-led-development.md)
- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](0024-racingline-strategy-selection-step1.md)
- [RFC 0025: Racingline 策略权重配置 Step 2 实现方案](0025-racingline-strategy-weight-configuration-step2.md)
- [RFC 0026: Racingline 股池预览 Step 3 实现方案](0026-racingline-strategy-pool-preview-step3.md)
- [Plan 0041: Racingline 虚拟账户与组合调仓净值实施计划](../../plans/archive/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md)
- [System: Racingline](../../architecture/racingline.md)
- [System: Rearview](../../architecture/rearview.md)
