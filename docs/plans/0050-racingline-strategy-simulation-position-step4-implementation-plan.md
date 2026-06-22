# Plan 0050: Racingline 模拟建仓 Step 4 实施计划

日期：2026-06-22

状态：Proposed

领域：racingline, rearview

关联系统：racingline, rearview

代码根：

- `app/racingline_new/`
- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`

关联文档：

- [RFC 0027: Racingline 模拟建仓 Step 4 实现方案](../RFC/0027-racingline-strategy-simulation-position-step4.md)
- [RFC 0026: Racingline 股池预览 Step 3 实现方案](../RFC/0026-racingline-strategy-pool-preview-step3.md)
- [RFC 0025: Racingline 策略权重配置 Step 2 实现方案](../RFC/0025-racingline-strategy-weight-configuration-step2.md)
- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](../RFC/0024-racingline-strategy-selection-step1.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md)
- [Plan 0041: Racingline 虚拟账户与组合调仓净值实施计划](0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md)
- [System: Racingline](../systems/racingline.md)
- [System: Rearview](../systems/rearview.md)
- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)

## 目标

1. 把 `/strategies` Step 4 从前端表单页收敛为 Step 3 和 Step 5 之间的可复现执行草稿生成器。
2. 新增 canonical `BacktestExecutionConfig` 和 `BacktestExecutionDraft`，让 Step 5 只消费 applied preview snapshot + Step 4 execution config，不读取散落 UI 状态。
3. 明确 `buyTopN` 只属于 Step 4/Step 5：Step 3 不提供、不展示、不受它影响。
4. Step 4 默认资金、费率和滑点从 Rearview 默认市场模板读取，成功路径不使用本地假默认值。
5. Step 4 和 Step 5 gate 只允许非 stale `PreviewSnapshot` 进入；Step 1/2 改动后必须重新更新股池。
6. 收敛 Step 4 UI 语义：只展示配置摘要和真实 preview 上下文，不展示本地估算信号数量。
7. 第一版只开放 Rearview portfolio engine 已支持的卖出规则：固定止损、固定止盈和时间止损；指标止损先禁用或隐藏。
8. 为后续 Step 5 backtest API 预留稳定 request 形态，但本计划不实现权威回测执行。
9. 不把后端执行能力缺口强行转嫁给前端 adapter；当修改 Rearview contract 或 portfolio engine 是更短、更一致的路径时，优先补后端。

## 非目标

1. 不在 Step 4 调用 `POST /rearview/runs`、`POST /rearview/portfolio-runs` 或任何会执行、持久化结果、入队 worker 的 backtest endpoint；允许调用 validate/draft-only endpoint 做后端 canonicalization、hash 和 validation。
2. 不在 Step 4 创建 rule set、rule version、run、portfolio run、account template 或正式策略。
3. 不在浏览器内计算权威股票池、买入信号、成交、持仓、费用、滑点、净值或绩效。
4. 不把 Step 3 preview response 当作 Step 5 的历史回测数据源；Step 5 仍需按回测区间重新执行 applied rule。
5. 不在本计划内完成 Step 5 结果页、回测 worker、benchmark 绩效或 portfolio run namespace 选择。
6. 不开放自定义公式、任意 SQL、任意脚本或前端自定义指标作为卖出规则。
7. 不实现 Step 4 草稿跨刷新恢复；需要时另行设计本地持久化或服务端 draft。

## 当前事实基线

### 已有能力

| 领域 | 当前事实 |
|---|---|
| Step 3 snapshot | `app/racingline_new/src/features/strategy/preview.ts` 已有 `PreviewSnapshot`、`buildPreviewSnapshot()`、`markPreviewSnapshotStale()` 和 presentation adapter。 |
| Step 3 gate | `app/racingline_new/src/routes/strategy-page.tsx` 已用 `canEnterSimulation` 阻止没有非 stale preview 或空股池时进入 Step 4。 |
| Step 4 表单状态 | `app/racingline_new/src/features/strategy/types.ts` 已有 `SimulationSettings`，覆盖初始资金、TopN、单票上限、费率、滑点和四类卖出规则。 |
| Step 4 面板 | `simulation-position-panel.tsx` 已有仓位管理、交易费率、风险管理和建仓摘要布局。 |
| 旧默认模板接入 | `app/racingline/src/api/rearview.ts`、`app/racingline/src/api/hooks.ts` 和 `app/racingline/src/types/rearview.ts` 已有 `MarketFeeTemplateRecord`、`getDefaultMarketFeeTemplate()` 和 query hook。 |
| Rearview 默认模板 API | `GET /rearview/market-fee-templates/default?market=CN_A_SHARE` 已在 `engines/crates/rearview-core/src/api/mod.rs` 暴露。 |
| Portfolio engine 能力 | `engines/crates/rearview-core/src/portfolio/mod.rs` 已支持 `initial_cash`、`max_positions`、`cash_reserve_pct`、A 股手数、fee、slippage、固定止损、固定止盈和时间止损。 |

### 已确认缺口

| 缺口 ID | 缺口 | 影响 | 处理方向 |
|---|---|---|---|
| G1 | Step 4 只有 `SimulationSettings`，没有 canonical execution config。 | Step 5 容易读表单状态，无法证明执行输入可复现。 | 新增后端 canonical validation 和 `BacktestExecutionDraft`；UI state 只能作为 thin adapter 输入。 |
| G2 | Step 4 进入 Step 5 的按钮没有绑定 draft/stale/template gate。 | 用户可能用 stale preview 或缺省假费率进入回测页。 | Simulation -> Backtest 只允许有效 draft。 |
| G3 | 默认费率仍在 `defaultSimulationSettings` 和面板常量里。 | 与 Rearview 市场模板漂移，接口失败也可能看起来可继续。 | 迁移旧前端默认模板 API/hook，Step 4 成功路径依赖真实模板。 |
| G4 | `indicatorStopLoss` 使用静态 `indicatorCatalog`。 | 当前后端 portfolio engine 不支持该退出规则，用户可能配置无法执行能力。 | 第一版禁用或隐藏指标止损，并由后端 validation 拒绝进入 canonical config。 |
| G5 | 建仓摘要的“近三月信号数”由本地公式生成。 | 容易被误解为真实历史信号统计。 | 移除本地趋势图，改为 execution config 和真实 preview snapshot 摘要。 |
| G6 | 单票上限与 engine 当前 `max_positions + cash_reserve_pct` 模型不完全一致。 | 如果由前端推导 `cashReservePct`，执行语义会依赖浏览器公式，后端结果难以自证。 | 优先在 `PortfolioSimulationInput` / worker `RebalancePolicy` 中新增 `single_position_limit_pct`，由 Rearview 统一计算 target weight 和现金保留。 |
| G7 | 百分比、decimal 和 bps 转换没有集中测试。 | 费率、滑点、止盈止损容易出现 100 倍误差。 | 后端 validation/hash 测试和前端 thin adapter 单测共同覆盖转换。 |
| G8 | Step 5 当前仍是静态回测样例。 | Step 4 draft 没有明确消费者，后续容易重新散落状态。 | Step 5 先接收 draft 并展示/gate，不发起真实回测。 |
| G9 | 计划原先默认前端生成 authoritative hash。 | Step 5/结果快照最终由 Rearview 执行，前端 hash 只能作为展示或乐观校验。 | Rearview 提供 canonical JSON + `rule_hash` + `execution_config_hash`，前端只展示和携带后端返回值。 |
| G10 | 当前 `POST /rearview/portfolio-runs` 只能消费 `source_run_id + account_template_id`。 | 策略创建流在 Step 5 前没有正式 source run，无法直接执行 transient rule + config。 | 设计 `strategy-backtests` validate/draft contract；真实 Step 5 再决定是 transient backtest namespace，还是后端内部创建临时 run + portfolio run。 |
| G11 | 后端已有 preview planner 能按 rule 排名，但 portfolio worker 只读取 PostgreSQL `buy_signal`。 | Step 5 如果只靠前端 draft，仍缺少“按回测区间重新生成 TopN 信号”的后端路径。 | 在 Rearview 侧复用 planner 生成 backtest signal rows，严禁前端从 Step 3 表格拼历史信号。 |
| G12 | `rebalance_policy` / `risk_exit_policy` 目前多处以 JSON 快照传递，worker 只反序列化部分字段。 | 新字段容易在 API、默认模板、worker 和计算内核之间漂移。 | 为 execution config 增加后端 typed structs 和兼容反序列化测试，再由 API/worker/engine 共用。 |

## 实施策略

这次不按“前端先适配现有后端，再等待 Step 5 补洞”的顺序做。更稳、更短的路线是让 Rearview 先拥有 execution config 的执行语义，前端只做表单表达和状态展示。

```text
SimulationSettings
  -> BacktestExecutionConfig request payload
  -> Rearview validate/draft-only canonicalization
  -> BacktestExecutionDraft(rule_hash, execution_config_hash, canonical_config)
  -> Step5 BacktestExecutionRequest
```

阶段顺序固定为：

1. 先在 Rearview 固定 typed execution config，并补最小 portfolio engine 语义缺口。
2. 再提供 validate/draft-only contract，后端返回 canonical config 和 hash，不执行回测。
3. 前端接默认模板和 draft validation，只保留薄 adapter。
4. Step 4/5 gate 消费后端返回的 draft，而不是散落组件状态。
5. 最后收敛 UI 文案和摘要，移除伪统计。

## 目标 contract

### `BacktestExecutionConfig`

第一版 canonical config 以 Rearview snake_case contract 为权威。前端可以保留 camelCase UI state，但提交、hash 和结果快照都以服务端 canonical JSON 为准：

```ts
type BacktestExecutionConfig = {
  market: "CN_A_SHARE"
  account: {
    initial_cash: number
    currency: "CNY"
  }
  signal_policy: {
    buy_signal_top_n: number
    signal_timing: "close_confirm_next_open"
  }
  rebalance_policy: {
    target_weighting: "equal_weight_capped"
    max_positions: number
    single_position_limit_pct: number
    cash_reserve_pct?: number
    lot_size: 100
    min_trade_lots: 1
    empty_signal_action: "hold"
  }
  fee_profile: {
    commission_rate: number
    commission_rate_max: number
    min_commission: number
    stamp_duty_rate_sell: number
    transfer_fee_rate: number
  }
  slippage_profile: {
    mode: "bps"
    buy_bps: number
    sell_bps: number
  }
  risk_exit_policy: {
    trigger_timing: "close_confirm_next_open"
    exit_rules: ExitRuleConfig[]
  }
  price_basis: "backward_adjusted"
}
```

### 单票上限推导

Step 4 保留用户直觉输入 `buyTopN` 和 `singlePositionLimitPercent`，但不由前端把单票上限折算成唯一执行语义。Rearview 统一推导：

```text
raw_equal_weight_pct = 1 / buyTopN
single_position_limit_pct = singlePositionLimitPercent / 100
target_weight_per_position_pct = min(raw_equal_weight_pct, single_position_limit_pct)
max_positions = buyTopN
implicit_cash_reserve_pct = 1 - target_weight_per_position_pct * buyTopN
```

实现要求：

1. `PortfolioSimulationInput` 新增 `single_position_limit_pct: Option<f64>`，worker `RebalancePolicy` 同步反序列化。
2. 买入目标金额由后端计算 `target_weight_per_position_pct`；因 cap 留下的现金自然保留，不需要前端伪造 `cash_reserve_pct`。
3. `cash_reserve_pct` 仍可作为显式现金保留字段保留，但不能替代单票上限语义。
4. 前端摘要中的“单票目标权重”和“隐含现金保留”来自后端 canonical response 或与后端同名 derived 字段，不作为执行真相。

### `BacktestExecutionDraft`

```ts
type BacktestExecutionDraft = {
  previewId: string
  appliedRuleSpec: RuleVersionSpec
  previewRange: {
    startDate: string
    endDate: string
    selectedTradeDate?: string | null
  }
  positionConfig: BacktestExecutionConfig
  ruleHash: string
  executionConfigHash: string
  createdAt: string
  stale: boolean
}
```

Hash 口径：

1. 使用稳定排序的 canonical JSON。
2. `ruleHash` 只覆盖 `appliedRuleSpec`。
3. `executionConfigHash` 只覆盖 `BacktestExecutionConfig`。
4. `createdAt`、UI label、loading/error 状态不进入 hash。

### Step 5 request 草案

Step 4 不调用执行型 API，但可以调用 validate/draft-only endpoint 准备 Step 5 可直接消费的 request shape：

```ts
type BacktestExecutionRequestDraft = {
  rule: RuleVersionSpec
  start_date: string
  end_date: string
  benchmark: string
  top_n: number
  execution_config: BacktestExecutionConfig
  rule_hash: string
  execution_config_hash: string
}
```

后续 Step 5 API 实现时再决定最终 endpoint 是 `POST /rearview/strategy-backtests`，还是后端内部组合 `runs + portfolio-runs`。

## 实施阶段

### Phase 0: Rearview execution contract 和 portfolio engine 语义缺口

目标：在改页面行为前，先让 Rearview 拥有 Step 4 config 的执行语义，避免前端用 adapter 掩盖后端缺口。

任务：

1. 在 `rearview-core` 新增 typed execution config，覆盖：
   - account / initial cash / currency
   - signal policy / `buy_signal_top_n`
   - rebalance policy / `max_positions` / `single_position_limit_pct` / lot rules
   - fee profile
   - slippage profile
   - risk exit policy
   - price basis
2. `PortfolioSimulationInput` 新增 `single_position_limit_pct: Option<f64>`。
3. 修改买入目标权重计算：
   - 默认仍支持旧 `max_positions + cash_reserve_pct` 快照。
   - 当 `single_position_limit_pct` 存在时，后端使用 `min(equal_weight_after_cash_reserve, single_position_limit_pct)`。
   - cap 造成的剩余资金保留为现金。
4. `rearview-portfolio-worker` 的 `ExecutionSnapshot.RebalancePolicy` 同步读取 `single_position_limit_pct`，并保持旧快照兼容。
5. 默认 account template / `default_rebalance_policy()` 增加 `target_weighting = equal_weight_capped` 和 `single_position_limit_pct = 0.1`，或在 contract 里明确缺省值由服务端补齐。
6. 固定后端 validation：
   - `initial_cash > 0`
   - `buy_signal_top_n >= 1`
   - `max_positions >= 1`
   - `single_position_limit_pct` 在 `(0, 1]`
   - fee/slippage 非负
   - indicator stop loss 第一版返回 validation error，除非同阶段补齐受控指标退出规则。
7. 后端生成 canonical JSON、`rule_hash` 和 `execution_config_hash`，hash 不包含 `created_at`、UI label、loading/error 状态。

测试策略：

```bash
cd engines
cargo fmt --check
cargo clippy -p rearview-core -p rearview-portfolio-worker --all-targets --all-features -- -D warnings
cargo test -p rearview-core -p rearview-portfolio-worker
```

关键测试用例：

1. `TopN=5, singlePositionLimit=10%` 目标单票权重为 `10%`，剩余现金保留。
2. `TopN=20, singlePositionLimit=10%` 目标单票权重为 `5%`。
3. 旧 `rebalance_policy` 没有 `single_position_limit_pct` 时行为保持兼容。
4. 相同 rule + config 生成稳定 hash。
5. 修改 `buy_signal_top_n` 只改变 execution config hash，不改变 rule hash。
6. indicator stop loss enabled 时 validation 失败。

完成标准：

1. 后端能表达 Step 4 的单票上限语义，不要求前端折算成 `cash_reserve_pct` 才可执行。
2. 后端 hash、validation 和 default contract 有单元测试保护。
3. 后续 UI 只需要提交用户配置，不需要复制 engine 目标权重算法。

### Phase 1: Strategy backtest draft validation contract

目标：提供不执行、不持久化结果的 Rearview draft/validate endpoint，让 Step 4 可以拿到服务端 canonical config 和 hash。

任务：

1. 新增 validate/draft-only endpoint，建议路径：
   - `POST /rearview/strategy-backtests/validate`
2. Request 输入：
   - `rule`
   - `preview_id` 和 preview range/context
   - `execution_config`
   - 可选 Step 5 `range` / `benchmark`，如果前端在 Step 5 再选择，也可为空。
3. Response 输出：
   - canonical `execution_config`
   - `rule_hash`
   - `execution_config_hash`
   - validation warnings/errors
   - derived summary：目标单票权重、隐含现金保留、max positions、启用 exit rules。
4. Endpoint 不创建 run、portfolio run、rule version、account template，不写 portfolio facts，不发 NATS。
5. 后端 validation 失败时返回统一错误结构，字段路径覆盖 `execution_config.rebalance_policy.single_position_limit_pct` 等关键字段。
6. 如果实现时判断新增 endpoint 过早，可先把同样的 validator 作为 `rearview-core` public service 暴露给 Step 5 API；但前端 hash 不能成为最终权威。

测试策略：

```bash
cd engines
cargo fmt --check
cargo clippy -p rearview-core --all-targets --all-features -- -D warnings
cargo test -p rearview-core
```

关键测试用例：

1. `0.05%` 印花税提交为 `0.0005`。
2. `0.1%` 滑点提交为 `10 bps`。
3. `TopN=5, singlePositionLimit=10%` 后端 derived summary 显示单票目标 `10%` 和隐含现金保留 `50%`。
4. `TopN=20, singlePositionLimit=10%` 后端 derived summary 显示单票目标 `5%` 和隐含现金保留 `0%`。
5. `buy_signal_top_n` 映射到 `signal_policy.buy_signal_top_n` 和 `rebalance_policy.max_positions`。

完成标准：

1. Step 4 表单字段和 Step 5 request 字段有后端 canonicalization 入口。
2. 100 倍单位错误有测试保护。
3. `single_position_limit_pct` 的执行语义在后端可见、可测。

### Phase 2: Rearview 默认市场模板接入

目标：Step 4 默认费率、滑点和币种来自 Rearview，接口失败不伪装成成功；默认模板只是初始值，最终执行语义仍以后端 validate/draft response 为准。

任务：

1. 在 `app/racingline_new/src/types/rearview.ts` 增加 `MarketFeeTemplateRecord`、`FeeProfile` 和 `SlippageProfile` 类型。
2. 在 `app/racingline_new/src/api/rearview.ts` 增加 `getDefaultMarketFeeTemplate(market = "CN_A_SHARE")`。
3. 在 `app/racingline_new/src/api/hooks.ts` 增加 `useDefaultMarketFeeTemplateQuery()`。
4. 在 `app/racingline_new/src/api/queryKeys.ts` 增加 default market fee template key。
5. Step 4 首次进入时用 template 初始化 fee/slippage 草稿。
6. 用户已经修改过 fee/slippage 后，template refetch 不覆盖用户草稿。
7. Template loading 时，交易费率区展示 loading，Simulation -> Backtest 按钮不可用。
8. Template error 时，展示明确错误和重试入口；不使用 `defaultSimulationSettings` 中的费率作为成功兜底。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

浏览器验收：

1. 正常 Rearview：Network 可见 `GET /rearview/market-fee-templates/default?market=CN_A_SHARE`。
2. 错误 API base URL：Step 4 显示默认模板加载失败，不能进入 Step 5。
3. 用户手动修改费率后触发 refetch，输入值不被覆盖。

完成标准：

1. Step 4 用户成功路径中的默认费率有真实 Rearview response 来源。
2. 本地默认值只保留为测试 fixture 或初始草稿结构，不作为接口失败时的可提交成功状态。

### Phase 3: 前端薄 adapter 和 Step 4/Step 5 gate

目标：前端只把 UI state 映射成后端 request payload，并用后端返回的 `BacktestExecutionDraft` 替代散落状态作为 Step 5 的唯一输入。

任务：

1. 在 `app/racingline_new/src/features/strategy/` 下新增薄 adapter，职责仅限字段命名和 UI percent 输入转换：
   - percent 输入转 decimal/bps request payload。
   - 不推导 engine-only 目标权重。
   - 不生成 authoritative hash。
2. 在 `strategy-page.tsx` 中派生或维护 `backtestExecutionDraft`：
   - 输入必须是 `previewSnapshot && !previewSnapshot.stale`
   - 必须有成功加载的 market fee template
   - 必须通过后端 validate/draft contract
3. Step 1/2 改动触发 `markPreviewSnapshotStale()` 后，当前 draft 同步视为 stale 或不可用。
4. Simulation -> Backtest 按钮只在后端 draft 有效时可用。
5. Step 4 页面如果 snapshot 缺失或 stale，显示前置条件状态，引导用户回 Step 3 更新股池。
6. Step 5 `BacktestPanel` 接收 `BacktestExecutionDraft | null`。
7. Step 5 没有 draft 时只显示 gate 状态，不展示静态成功回测。
8. Step 5 有 draft 时展示后端 canonical config 摘要和 hash；真实回测按钮保持 disabled 或明确为待接入状态。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

关键测试用例：

1. 没有 preview snapshot 时不能从 Step 4 进入 Step 5。
2. Step 1 条件修改后 draft invalid。
3. Step 2 权重修改后 draft invalid。
4. 点击更新股池成功后 draft 恢复可生成。
5. 修改 Step 4 `buyTopN` 后，只更新 draft hash，不改变 Step 3 preview 表格和 snapshot rule hash。
6. 后端 validate 返回错误时，Step 4 显示 error，不能进入 Step 5。

完成标准：

1. Step 5 不再依赖 `simulationSettings`、`conditionGroups`、`weightIndicators` 等散落状态。
2. 同一 applied rule + execution config 可复现同一 Step 5 输入。
3. Stale preview 不能越过 Step 4/Step 5 gate。
4. Frontend hash 和 derived weight 不作为执行真相。

### Phase 4: Step 4 UI 语义收敛

目标：让用户看到的是“回测执行参数”，不是伪造历史信号或未支持能力。

任务：

1. `SimulationPositionPanel` props 增加真实上下文：
   - `previewSnapshot`
   - `executionDraft`
   - market template loading/error/refetch 状态
2. 摘要区改为展示：
   - 初始资金和币种
   - 买入信号 TopN
   - 单票目标权重
   - 现金保留比例
   - 预计最大持仓数
   - 费用和滑点摘要
   - 已启用卖出规则
   - `previewId`、preview 日期范围和 stale 状态
3. 移除 `buildSignalTrendData()` 和“近三月信号数”趋势图，除非未来由 Step 5 返回真实统计。
4. 将“最大持仓”从 `floor(100 / singlePositionLimitPercent)` 改为 `buyTopN` 或 canonical `maxPositions`，避免与执行语义冲突。
5. 指标止损第一版禁用或隐藏：
   - 如果保留可见控件，必须明确 disabled，并且不能进入 `ExitRuleConfig`。
   - Summary 不展示不可执行规则。
6. 交易费率区补齐模板字段：
   - 佣金率
   - 佣金上限
   - 最低佣金
   - 卖出印花税
   - 过户费
   - 买入/卖出滑点
7. 将 Simulation -> Backtest 按钮文案从“执行回测”调整为不误导的进入语义；真实执行留给 Step 5。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

浏览器验收：

1. Step 4 页面没有本地估算信号趋势。
2. Step 4 `TopN` 和单票上限变化时，摘要中的单票目标权重、现金保留和最大持仓同步变化。
3. 指标止损不能被提交为可执行规则。
4. Template error 时用户不能看到“已加载默认费率”的成功态。

完成标准：

1. Step 4 可见信息全部来自表单、Rearview 模板或真实 preview snapshot。
2. 页面不再暗示 Step 4 已经执行历史信号统计或回测。

### Phase 5: Step 5 contract 准备，不执行回测

目标：让 Step 5 接好输入边界，为后续 RFC/plan 实现后端回测留清晰入口。

任务：

1. `BacktestPanel` 接收 draft、period 和 benchmark，生成 `BacktestExecutionRequestDraft`。
2. Step 5 配置区展示：
   - 回测周期
   - benchmark
   - TopN
   - rule hash
   - execution config hash
   - 初始资金、单票目标权重、现金保留、费用和卖出规则摘要
3. 静态净值曲线、持仓记录和绩效指标保留时必须明确隔离为 prototype placeholder，不能在有效 draft 下展示为真实结果；推荐第一版直接隐藏真实结果区域，展示“待执行回测”状态。
4. `重新回测` 按钮不调用后端；后续 Step 5 API 接入时再启用。
5. 在类型或测试 fixture 中固化未来 request 的 snake_case 序列化边界，但不提交 HTTP。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

完成标准：

1. Step 5 的所有执行输入都能从 draft + Step 5 range/benchmark 推导。
2. Step 5 不再读取 Step 1/2 当前 draft 或 Step 3 表格行。
3. UI 不把静态样例误认为真实回测结果。

### Phase 6: 端到端验收和回归门禁

目标：覆盖 Step 3 stale、Step 4 template、adapter 单位转换和 Step 5 handoff 的主路径。

任务：

1. 增加 React/unit tests 覆盖 adapter、draft gate 和 UI 状态。
2. 增加或更新 Playwright/CDP smoke 手册步骤，重点覆盖：
   - 非 stale Step 3 preview 进入 Step 4。
   - Step 1/2 修改后 Step 4/Step 5 gate 被 stale 拦住。
   - Step 4 默认费率来自 Rearview。
   - Step 4 `buyTopN` 不改变 Step 3 preview。
   - 断开 Rearview 后默认模板失败，不能进入 Step 5。
3. 如果实现中触碰 Rearview API 或 portfolio engine contract，追加 Rust 验证。
4. 完成后新增 job report，并在归档计划时同步 `docs/plans/README.md` 和 `docs/systems/racingline.md`。

最小验证命令：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

涉及 Rearview 后端改动时追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

浏览器验收入口：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

完成标准：

1. 质量门禁通过。
2. 浏览器验收记录 Step 4 主路径、stale gate 和 fee template error path。
3. 验收报告落到 `docs/jobs/reports/`。

## 禁止模式

1. Step 4 不允许发起任何回测、portfolio run 或正式策略保存。
2. Step 4 不允许在接口失败时用 mock/default 常量伪造成成功。
3. Step 5 不允许绕过 `BacktestExecutionDraft` 直接读取 Step 1/2/3/4 当前组件状态。
4. Step 3 不允许新增或读取 Step 4 的 `buyTopN`。
5. UI 不允许展示本地估算的信号数量、收益、净值、持仓或绩效为真实结果。
6. Adapter 之外不允许分散做 percent/decimal/bps 转换。
7. 第一版不允许把 `indicator_stop_loss` 序列化进 execution config。

## 允许保留的短期例外

1. `app/racingline_new` 仍作为并行策略创建工作台存在，正式迁移到 `app/racingline` 另行计划。
2. Step 5 可以保留静态 layout 骨架，但有效 draft 下不能展示成真实回测结果。
3. 如果 Phase 0 后端 cap 变更被明确阻塞，前端只能展示“单票上限待后端支持”的 disabled 状态；不能用前端 `cashReservePct` 推导伪装成已支持的执行语义。
4. Step 4 草稿第一版可以不跨页面刷新恢复。

## 待决问题

1. Step 5 最终 API 是新增 `POST /rearview/strategy-backtests`，还是后端内部组合 `runs + portfolio-runs`。
2. `cash_reserve_pct` 是否作为用户可编辑字段进入 Step 4 第一版；当前计划只把它作为后端兼容字段和 derived summary。
3. 指标止损是否进入第一版 Step 5；若进入，需要单独补 Rearview 受控指标退出规则、allowlist、缺失指标事件和后复权口径测试。
4. Step 4 draft 是否需要在保存策略时转换成正式 `account_template`，以及转换发生在“保存策略”还是“运行策略”。
5. Step 5 真实执行结果是否复用 `portfolio_run_id` 结果查询 UI，还是新增 strategy backtest namespace。

## 完成后的维护动作

1. 新增验收报告到 `docs/jobs/reports/`。
2. 更新 [System: Racingline](../systems/racingline.md)，说明 `/strategies` Step 4 已生成 `BacktestExecutionDraft`。
3. 如果后端新增 Step 5 API，更新 [System: Rearview](../systems/rearview.md) 和 `engines/README.md`。
4. 计划完成后移入 `docs/plans/archive/`，并在 `docs/plans/README.md` 的 Recently Completed 中记录报告链接。
