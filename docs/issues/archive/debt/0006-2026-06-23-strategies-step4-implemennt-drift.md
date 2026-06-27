# Debt 0006: Strategies Step 4 模拟建仓实现漂移

状态：Resolved（2026-06-23）
日期：2026-06-23
领域：racingline, rearview
关联代码：`app/racingline_new/`, `engines/crates/rearview-core/`, `engines/crates/rearview-portfolio-worker/`
关联设计：`docs/RFC/archive/0027-racingline-strategy-simulation-position-step4.md`, `docs/plans/archive/0050-racingline-strategy-simulation-position-step4-implementation-plan.md`
关联报告：`docs/jobs/reports/2026-06-23-racingline-strategy-step4-draft-handoff.md`, `docs/jobs/reports/2026-06-23-racingline-strategy-step4-drift-remediation.md`

## 摘要

Step 4 第一版实现完成了 Rearview draft validation、canonical hash 和 Step 5 handoff，但与当前产品预期发生了新的实现偏移。偏移集中在两个方向：

1. 后端 draft contract 的技术细节过早暴露到 Step 4 UI，导致“模拟建仓”看起来像后端校验面板。
2. 为了对齐 Plan 0050，删除或禁用了原型中仍应保留的用户输入能力，包括近三月票池数折线图、趋势指标止损、单一滑点输入，以及更轻量的费用表单。

本 debt 记录这些偏移，并给出修复方案。已按本文方案完成实现和验收，见 [Step 4 Drift Remediation report](../../../jobs/reports/2026-06-23-racingline-strategy-step4-drift-remediation.md)。修复时不应简单回退到旧实现，也不应把后端能力缺口强行转嫁给前端。正确方向是：

- Step 4 面向用户表达建仓参数和风险管理，不展示 Rearview draft/hash/preview debug 信息。
- Step 4 编辑期间不自动触发后端 backtest validate；只有点击「策略回测」或「进入回测」时才生成回测条件并校验。
- 后端仍保留 canonical validate endpoint，但调用时机从 live query 改成显式 transition action。
- 指标止损要补成受控趋势指标能力，而不是继续禁用 UI。

## 实施结果

2026-06-23 已完成以下修复：

1. Step 4 摘要恢复为用户参数视角，不再展示 Rearview draft、Draft ready、Preview debug、草稿 hash、账户币种、现金保留、佣金上限和最低佣金。
2. 交易费率 UI 收敛为佣金率、卖出印花税、过户费和单一成交滑点；adapter 提交时把单一滑点映射为相同 `buy_bps` / `sell_bps`。
3. Step 4 编辑期间不再自动调用 `POST /rearview/strategy-backtests/validate`；点击「进入回测」时才通过 mutation 生成 canonical draft。
4. Step 4 恢复 `近三月票池数` 折线图，数据直接来自 Step 3 applied preview snapshot 的 `timeline.trade_dates[*].pool_count`。该图不使用信号数、不读取 `buyTopN`，也不进入 Step 5 request、hash 或后端结果。
5. 指标止损恢复为受控趋势指标能力：前端只能选择 trend 指标，后端只接受 `{ source: "trend", operator: "close_below_metric" }` 形态，并由 portfolio engine 执行。
6. Step 5 继续展示 canonical `rule_hash` 和 `execution_config_hash`，但不展示静态净值、持仓或绩效样例。

验证结果：

- `cd app/racingline_new && npm run lint`
- `cd app/racingline_new && npm run typecheck`
- `cd app/racingline_new && npm test`
- `cd app/racingline_new && npm run build`
- `cd engines && cargo fmt --check`
- `cd engines && cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cd engines && cargo test --workspace`
- `make docs-check`
- `git diff --check`

浏览器验收已确认 Step 4 显示 `近三月票池数`、趋势指标止损、单一成交滑点，不显示 Rearview draft/hash/Preview debug 字段；编辑 Step 4 不触发 validate，点击「进入回测」仅触发一次 validate 并进入 Step 5。

## 预期基线

Step 4 在用户流程里的定位应是：

```text
Step 3 股池预览
  用户确认规则和权重生成的候选池

Step 4 模拟建仓
  用户配置资金、TopN、单票上限、交易费率、滑点和卖出规则
  页面展示建仓摘要、近三月票池数和风险规则摘要
  用户可以继续编辑，不因每次编辑触发后端回测条件生成

点击「策略回测」/「进入回测」
  前端把 Step 3 applied snapshot + Step 4 当前表单转换为 request
  调用 Rearview validate/canonicalize
  成功后进入 Step 5
```

Step 4 不是后端调试页，不展示 canonical hash、preview id、preview range 或 Draft ready 状态。Step 5 可以消费后端 canonical draft，但 Step 4 用户摘要应保持产品语义。

## 偏移清单

| ID | 偏移 | 当前事实 | 影响 | 修复方向 |
|---|---|---|---|---|
| D1 | 建仓摘要展示 `Rearview 回测草稿`、`规则和建仓参数已由 Rearview 校验`、`Draft ready`。 | `SimulationPositionPanel` 的右侧摘要直接渲染 `BacktestDraftState` 和 draft-ready badge。 | Step 4 被理解成后端校验状态页，不是模拟建仓配置页。 | Step 4 删除这些状态文案；校验状态只作为按钮 loading/error 或 Step 5 前置结果。 |
| D2 | 建仓摘要展示 `账户币种 CNY` 和 `现金保留 0%`。 | 摘要从 canonical response 中读取 currency 和 implicit cash reserve。 | 技术字段占用摘要空间；现金保留不是当前用户主控件。 | Step 4 摘要不展示账户币种和现金保留；如后续开放现金保留输入，再作为用户字段。 |
| D3 | 建仓摘要展示 Preview、Preview 状态、Preview 区间和草稿 Hash。 | 摘要直接展示 `previewSnapshot` 和 `execution_config_hash`。 | preview/debug 边界泄漏到用户主流程。 | Step 4 隐藏这些字段；Step 5 或 dev/debug 面板可展示 hash。 |
| D4 | `佣金上限` 被实现成独立输入框和摘要字段。 | `transactionFees.commissionRateMaxPercent` 作为可编辑字段展示。 | 用户被迫配置低频技术约束，违背“佣金上限只作为输入文本校验”的预期。 | Step 4 只保留佣金率输入；佣金上限来自模板或常量，只用于校验和 helper，不作为独立输入。 |
| D5 | `最低佣金` 被实现成独立输入框和摘要字段。 | `transactionFees.minCommission` 作为可编辑字段展示并进入摘要。 | 第一版不需要用户设置最低佣金，增加表单噪声。 | Step 4 移除最低佣金 UI；后端 request 可继续携带模板默认值，但不展示、不允许编辑。 |
| D6 | 前端拆分买入滑点和卖出滑点。 | `buySlippageRatePercent` 和 `sellSlippageRatePercent` 两个输入分别展示。 | 当前预期是单一成交滑点输入，前端不应拆分。 | UI 恢复单一 `slippageRatePercent`；adapter 提交时映射为相同 `buy_bps` / `sell_bps`。 |
| D7 | 编辑风险管理时持续生成/校验回测条件。 | `useStrategyBacktestValidateQuery(request)` 在 request 变化后自动运行，修改 fee/risk/TopN 都可能触发 validate。 | 用户编辑过程中出现隐式后端请求，且错误态会打断配置体验。 | 改为点击「进入回测」时使用 mutation 校验；Step 4 编辑期间不调用 validate endpoint。 |
| D8 | 原有近三月票池数和折线图被删除。 | Plan 0050 将其视作本地伪统计并移除。 | 用户丢失 Step 3 股票池规模在近三个月内的走势反馈。 | 恢复近三月票池数折线图，直接展示 Step 3 股票池 timeline 中最近三个月的 `pool_count` 走势。 |
| D9 | 指标止损被禁用。 | 前端禁用 indicator stop loss，adapter 和后端 validation 均拒绝。 | 用户无法配置预期中的趋势指标止损。 | 第一版支持趋势指标止损，前端恢复指标选择，后端补受控 `indicator_stop_loss` contract 和执行语义。 |

## 初始漂移事实

| 领域 | 初始实现 | 证据 |
|---|---|---|
| Step 4 draft live query | `strategy-page.tsx` 根据 `previewSnapshot + defaultMarketTemplate + effectiveSimulationSettings` 自动构造 validate request，并用 React Query 自动调用 `POST /rearview/strategy-backtests/validate`。 | `app/racingline_new/src/routes/strategy-page.tsx` |
| Step 4 摘要 | `SimulationPositionPanel` 展示 Rearview draft state、Preview 信息、草稿 Hash、现金保留、账户币种、佣金上限、最低佣金和买卖滑点。 | `app/racingline_new/src/features/strategy/components/simulation-position-panel.tsx` |
| 前端 fee/slippage state | `SimulationSettings.transactionFees` 已从旧的单滑点改成 `commissionRateMaxPercent`、`minCommission`、`buySlippageRatePercent`、`sellSlippageRatePercent`。 | `app/racingline_new/src/features/strategy/types.ts` |
| 指标止损 UI | `RiskRuleRow` 中指标止损 disabled，并提示 Rearview 当前只开放固定止损、固定止盈和时间止损。 | `simulation-position-panel.tsx` |
| 指标止损 adapter | `simulationSettingsToBacktestExecutionConfig()` 遇到 `indicatorStopLoss.enabled` 直接 throw。 | `app/racingline_new/src/features/strategy/execution.ts` |
| 后端 draft validation | `ExitRuleConfig::IndicatorStopLoss` 存在枚举 shape，但 `validate()` 直接返回 unsupported。 | `engines/crates/rearview-core/src/strategy_backtest.rs` |
| portfolio worker | `RiskExitPolicy::exit_rules()` 对 `indicator_stop_loss` 返回 unsupported，因为 worker 当前没有指标输入。 | `engines/crates/rearview-portfolio-worker/src/main.rs` |
| portfolio engine | `ExitRule` 只支持 fixed stop loss、take profit 和 time stop loss。 | `engines/crates/rearview-core/src/portfolio/mod.rs` |
| 被删除的旧 UI | `c1b7aa3` 版本中有 `IndicatorStopLossFields`、`SignalCountTrendChart` 和 `buildSignalTrendData()`。 | `git show c1b7aa3:app/racingline_new/src/features/strategy/components/simulation-position-panel.tsx` |

## 根因判断

### R1: Plan 0050 过度偏向后端 canonical draft 可见性

Plan 0050 的核心纠偏是“前端不生成权威 hash，后端 canonicalize”。这个方向是正确的，但实现时把 canonical draft 状态展示到了 Step 4 摘要里。产品预期不是隐藏后端校验能力，而是把它变成 transition-time guard：

1. Step 4 页面内编辑时只显示用户参数。
2. 点击进入 Step 5 时才校验并生成 canonical draft。
3. Step 5 内可以展示 request boundary 和 hash。

### R2: 后端 contract 字段被一比一搬进 UI

Rearview contract 需要 `commission_rate_max`、`min_commission`、`buy_bps`、`sell_bps`，但这不代表 Step 4 必须有同名输入。UI 应按用户心智收敛：

1. 佣金上限用于校验佣金率，不是独立条件。
2. 最低佣金第一版使用模板默认值，不暴露给用户。
3. 滑点第一版只给一个成交滑点输入，提交时映射为双边相同值。

### R3: “本地伪统计”被误判为必须删除的假结果

近三月票池数不应伪装成真实回测信号或绩效结果；它只表达 Step 3 股票池在最近三个月的规模走势。更合适的处理是：

1. 恢复折线图的信息结构。
2. 数据源直接使用 Step 3 applied preview snapshot 的 timeline，取最近三个月交易日的 `pool_count`。
3. 不按 `buyTopN` 推导信号数，不新增 signal count endpoint，也不使用表单数量做本地估算。
4. 图表必须保持为 Step 4 辅助摘要，不进入 Step 5 request、hash 或后端结果。

### R4: 指标止损缺口被 UI 禁用掩盖

当前 portfolio engine 没有指标输入，因此禁用 indicator stop loss 是 Plan 0050 的短期保守处理。但用户预期是支持“趋势指标止损”。更优路线是补后端最小受控能力，而不是长期禁用：

1. 第一版只允许 trend 指标。
2. 不开放任意指标、公式或 SQL。
3. 后端 validation 和 worker 都要理解该规则。
4. portfolio simulation 输入要能在持仓日期读取所选 trend metric，并和收盘价比较。

## 修复方案

### Phase 0: 固定新的 Step 4 UI 边界

目标：先从文档和测试上明确 Step 4 不展示后端 draft/debug 信息，但仍使用后端 validate 作为进入 Step 5 的 guard。

要求：

1. Step 4 摘要标题保持“建仓摘要”，描述改回“当前模拟参数”或等价产品文案。
2. 删除摘要中的：
   - `Rearview 回测草稿`
   - `规则和建仓参数已由 Rearview 校验`
   - `Draft ready`
   - `账户币种`
   - `现金保留`
   - `Preview`
   - `Preview 状态`
   - `Preview 区间`
   - `草稿 Hash`
3. Step 4 仍可以在缺少 preview、preview stale、模板加载失败时禁用进入 Step 5，但这些状态只用于 gate，不进入常规摘要。
4. Step 5 可以继续展示 Rule hash 和 Execution config hash，因为 Step 5 是回测输入确认页。

完成标准：

1. Step 4 页面看不到 Rearview draft/hash/Preview debug 字段。
2. Step 5 仍能接收 canonical draft。
3. 没有 preview 或 stale preview 时不能进入 Step 5。

### Phase 1: 收敛交易费率 UI

目标：把费用输入改回用户预期的轻量形态，同时保留后端 canonical contract 所需字段。

前端 state 建议：

```ts
transactionFees: {
  commissionRatePercent: number
  slippageRatePercent: number
  stampDutyRatePercent: number
  transferFeeRatePercent: number
}
```

模板映射：

1. `commissionRatePercent = fee_profile.commission_rate * 100`
2. `stampDutyRatePercent = fee_profile.stamp_duty_rate_sell * 100`
3. `transferFeeRatePercent = fee_profile.transfer_fee_rate * 100`
4. `slippageRatePercent = max(slippage_profile.buy_bps, slippage_profile.sell_bps) / 100`，如果两边不同，第一版以前端单值为准。
5. `commissionRateMaxPercent` 和 `minCommission` 保存在 internal template snapshot 或 adapter 局部变量，不进入用户可编辑表单。

提交到后端：

```ts
fee_profile: {
  commission_rate: percentToDecimal(commissionRatePercent)
  commission_rate_max: template.fee_profile.commission_rate_max
  min_commission: template.fee_profile.min_commission
  stamp_duty_rate_sell: percentToDecimal(stampDutyRatePercent)
  transfer_fee_rate: percentToDecimal(transferFeeRatePercent)
}
slippage_profile: {
  mode: "bps"
  buy_bps: percentToBps(slippageRatePercent)
  sell_bps: percentToBps(slippageRatePercent)
}
```

佣金上限校验：

1. 如果用户输入 `commissionRatePercent > template.fee_profile.commission_rate_max * 100`，前端显示字段级错误并禁止进入 Step 5。
2. 后端仍做同样 validation，防止绕过前端。
3. UI 不展示单独“佣金上限”输入框。

完成标准：

1. Step 4 交易费率区不再有“佣金上限”“最低佣金”“买入滑点”“卖出滑点”四个独立输入。
2. Step 4 只保留一个“成交滑点”输入。
3. Adapter 单测覆盖单一滑点映射为相同 buy/sell bps。
4. Template error 时仍不能进入 Step 5。

### Phase 2: 改成点击进入 Step 5 时才生成回测条件

目标：Step 4 编辑过程不自动调用后端 validate；进入 Step 5 是唯一生成 canonical draft 的动作。

前端实现：

1. 删除 `useStrategyBacktestValidateQuery(request)` 的 live validate 用法。
2. 保留 `validateStrategyBacktest()` 或 `useStrategyBacktestValidateMutation()`。
3. `canEnterBacktest` 改为只检查本地前置条件：
   - `previewSnapshot && !previewSnapshot.stale`
   - market fee template loaded
   - local form validation passed
4. 点击「进入回测」时：
   - 构造 validate request。
   - 调用 `validateStrategyBacktest` mutation。
   - pending 时按钮显示 loading。
   - success 后保存 `backtestExecutionDraft` 并 `setActiveStep("backtest")`。
   - error 时停留 Step 4，在摘要或字段附近显示错误。
5. 修改风险管理、费用、TopN、单票上限时：
   - 只更新本地表单。
   - 清空旧 `backtestExecutionDraft` 或标记其 dirty。
   - 不发起网络请求。

网络验收：

1. 进入 Step 4 后修改固定止损、时间止损、指标止损、费用或 TopN，Network 不出现 `POST /rearview/strategy-backtests/validate`。
2. 点击「进入回测」后才出现一次 validate request。
3. validate 成功后进入 Step 5。
4. validate 失败时停留 Step 4。

### Phase 3: 恢复近三月票池数和折线图

目标：恢复 Step 4 用户需要的股票池规模走势反馈，同时不把它当作真实回测结果或买入信号统计。

数据路线：

1. 复用 Step 3 `previewSnapshot.timeline.trade_dates`。
2. 从 timeline 中截取最近三个月交易日。
3. 每个点使用 `trade_date` 和 `pool_count`，不读取 Step 4 `buyTopN`。
4. 可以按周聚合以降低折线密度；聚合值使用该周最后一个交易日的 `pool_count`，或在 UI 文案中明确使用周均值。
5. 不恢复完全基于表单数量的旧 `buildSignalTrendData()`。

前端实现：

1. 恢复折线图组件，并重命名为 `PoolCountTrendChart` 或等价名称。
2. 从 `previewSnapshot.timeline.trade_dates` 构造 `{ label, count }` 数据，其中 `count = pool_count`。
3. 摘要中恢复：
   - 标题：`近三月票池数`
   - badge：最近一个交易日的股票池数量，或近三个月平均票池数，文案必须明确。
   - 折线图：固定高度，不能撑开布局。
4. 图表不进入 execution config、hash 或 Step 5 request。

完成标准：

1. Step 4 建仓摘要重新显示近三月票池数和折线图。
2. 图表来源可追溯到 Step 3 timeline 的 `pool_count`。
3. Step 5 仍不把该图当作回测结果或买入信号统计展示。

### Phase 4: 支持趋势指标止损

目标：恢复指标止损 UI，并让后端支持第一版受控趋势指标退出规则。

前端 UI：

1. 恢复 `IndicatorStopLossFields`，但只允许趋势指标。
2. 指标来源固定为 `trend` 或从 catalog 中筛选 `mart_stock_trend_indicator_daily` 且 value kind 为 numeric 的指标。
3. 第一版语义固定为：

```text
收盘价跌破 selected_trend_metric 时卖出
```

4. 不开放任意 operator、阈值、公式或跨指标比较。

前端 request:

```json
{
  "type": "indicator_stop_loss",
  "source": "trend",
  "metric": "price_ma_10",
  "operator": "close_below_metric"
}
```

后端 contract：

1. 扩展 `ExitRuleConfig::IndicatorStopLoss`：
   - `source: "trend"`
   - `metric: String`
   - `operator: "close_below_metric"`
2. validation:
   - source 必须是 `trend`
   - metric 必须属于 allowlisted trend metrics
   - operator 必须是 `close_below_metric`
3. `rearview-portfolio-worker` 不再拒绝 `indicator_stop_loss`，而是转换成 portfolio engine exit rule。
4. `PortfolioSimulationInput` 增加指标输入：

```rust
pub struct IndicatorBar {
    pub security_code: String,
    pub trade_date: NaiveDate,
    pub values: BTreeMap<String, Option<f64>>,
}
```

或在 `PriceBar` 中增加受控 trend metric 字段。实现时优先选择更简单且类型安全的结构。

5. `ExitRule` 增加：

```rust
IndicatorStopLoss {
    metric: String,
}
```

6. exit evaluation:
   - 对持仓证券，在评估日读取 close price 和 selected metric。
   - 当 close price 有值、metric 有值且 close < metric 时触发卖出。
   - 缺失指标不触发卖出，并记录 event reason 或 warning。

数据读取：

1. Step 5/backtest worker 需要在加载价格序列时同步加载所需 trend metric。
2. 第一版只支持 `mart_stock_trend_indicator_daily` 中已存在的趋势指标。
3. 指标调整口径必须和用于比较的 close price 明确一致；如果暂时无法做到全复权一致，必须在后端 validation 或 result metadata 中记录 basis。

完成标准：

1. Step 4 可以启用指标止损并选择趋势指标。
2. 启用指标止损后点击「进入回测」能通过 validate。
3. 后端拒绝非 trend 指标或未知 metric。
4. portfolio engine 单测覆盖 close below MA 触发卖出、metric 缺失不触发卖出。
5. worker 反序列化测试覆盖 indicator stop loss。

### Phase 5: Step 5 handoff 保持 canonical，但不提前污染 Step 4

目标：保留后端 canonical draft 的价值，但把它放到正确阶段。

实现要求：

1. Step 4 点击进入时生成 `BacktestExecutionDraft`。
2. Step 5 接收 draft 并展示：
   - 回测区间
   - benchmark
   - TopN
   - rule hash
   - execution config hash
   - 费用和卖出规则摘要
3. Step 5 不展示真实净值、持仓或绩效结果，直到真实回测 API 接入。
4. 如果用户从 Step 5 返回 Step 4 并修改任意配置，旧 draft 作废；再次进入 Step 5 时重新 validate。

## 禁止模式

1. 不在 Step 4 编辑期间自动调用 `POST /rearview/strategy-backtests/validate`。
2. 不在 Step 4 摘要展示 hash、preview id、preview range 或 Draft ready。
3. 不把佣金上限、最低佣金、买入滑点、卖出滑点作为第一版独立用户输入。
4. 不用前端任意公式实现指标止损；趋势指标止损必须经过后端 validation 和 portfolio engine 支持。
5. 不支持任意指标、任意 SQL、任意脚本或跨 mart 自定义条件作为退出规则。
6. 不把近三月票池数折线图写入 execution config、hash 或 Step 5 request。
7. 不恢复 Step 5 静态净值、持仓和绩效样例。

## 最小测试策略

前端：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

需要新增或更新的测试：

1. Adapter 单测：单一 `slippageRatePercent` 映射为相同 `buy_bps` 和 `sell_bps`。
2. Adapter 单测：佣金上限来自模板，不来自用户可编辑 state。
3. Gate 单测或组件测试：修改风险管理不触发 validate query。
4. Mutation 流程测试：点击进入 Step 5 后才调用 validate。
5. Indicator stop loss adapter 单测：trend metric 序列化为 `indicator_stop_loss`。
6. Pool count chart 单测：从 preview timeline 的 `pool_count` 构造近三月票池数折线数据。

后端：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

需要新增或更新的测试：

1. `strategy_backtest` validation 接受 trend indicator stop loss。
2. `strategy_backtest` validation 拒绝非 trend source、未知 metric 和 unsupported operator。
3. `rearview-portfolio-worker` 能反序列化 indicator stop loss。
4. `portfolio` engine 在 close below selected trend metric 时触发卖出。
5. metric 缺失时不触发卖出，并保持 simulation 不 panic。

文档：

```bash
make docs-check
git diff --check
```

## 浏览器验收

环境仍使用现有 CDP 流程：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

验收点：

1. Step 4 建仓摘要不显示 Rearview draft/hash/Preview debug 字段。
2. Step 4 交易费率区只有佣金、印花税、过户费和单一成交滑点；没有佣金上限、最低佣金、买入滑点、卖出滑点输入。
3. 修改固定止盈、固定止损、指标止损、时间止损、TopN、单票上限和费用时，Network 不出现 strategy-backtests validate 请求。
4. 点击「进入回测」后才出现一次 `POST /rearview/strategy-backtests/validate`。
5. validate 成功后进入 Step 5，并在 Step 5 展示 canonical hash。
6. Step 4 显示近三月票池数和折线图。
7. Step 4 可启用指标止损，并只能选择趋势指标。
8. 启用趋势指标止损后仍能进入 Step 5；非趋势指标没有入口或会被后端拒绝。
9. Template error path 仍阻止进入 Step 5。
10. Step 1/2 修改导致 preview stale 后仍不能进入 Step 4/Step 5。

## 实施落点

本 debt 未再新增独立 Plan 0051；修复范围足够明确，直接按本文阶段顺序实施，并用 job report 固化结果：

1. Step 4 UI summary 和交易费率表单收敛。
2. validate 调用时机从 live query 改为 click mutation。
3. 恢复近三月票池数折线图。
4. 补趋势指标止损的前后端 contract、worker 和 portfolio engine。
5. 完成浏览器验收和 job report。

完成记录见 [2026-06-23 Step4 Drift Remediation report](../../../jobs/reports/2026-06-23-racingline-strategy-step4-drift-remediation.md)。后续真实回测执行 API、worker 结果落表、benchmark 绩效和回测结果页不属于本 debt，继续由组合净值和回测执行相关计划承接。
