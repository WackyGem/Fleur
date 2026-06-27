# RFC 0021: Racingline 虚拟账户与组合调仓净值

状态：Archived（2026-06-25；归档前状态：Proposed（2026-06-16））
领域：racingline
关联系统：racingline, rearview, data-platform
代码根：app/racingline/, engines/crates/rearview-core/, engines/crates/rearview-server/, engines/crates/rearview-portfolio-worker/, pipeline/migrate/
系统地图：docs/architecture/racingline.md

## 摘要

本文档定义 Racingline 在现有策略选股能力之后的下一阶段能力：用户创建选股策略时，系统默认为该策略创建一个初始资金为 1,000,000 CNY 的研究型虚拟资金账户模板；用户可以配置交易费率、滑点、止盈和止损条件；系统根据选股策略产生的调仓信号构建虚拟组合。第一版交付边界要求完成组合净值计算、净值曲线展示，并持久化目标组合、虚拟订单、成交和每日持仓明细表；完整回测分析报表、归因和基准比较作为后续增强。

该能力把现有 Rearview “区间选股和买入信号生成”升级为“策略组合模拟”。它仍然不是实盘交易、下单系统或券商账户。Racingline 负责账户和组合工作流 UI；Rearview 负责账户模板、组合运行、成交假设、目标/订单/成交/持仓明细、净值计算、审计状态和 API；ClickHouse mart 仍是行情和指标事实来源。由于后续需要扩展完整回测，第一版就引入 NATS JetStream 作为异步任务分发边界，并把组合计算明细落库，为后续回测分析页面复用同一账本。

关联文档：

- [RFC 0018: Rust Rearview 规则选股服务与 mart 指标库](0018-rust-stock-screening-service.md)
- [RFC 0019: Racingline Rearview 前端工作台](0019-racingline-rearview-frontend-workbench.md)
- [RFC 0020: Racingline Run Result 个股分析页](0020-racingline-run-result-security-analysis-page.md)
- [System: Racingline](../../architecture/racingline.md)
- [System: Rearview](../../architecture/rearview.md)
- [System: Deploy Ops](../../architecture/deploy-ops.md)

## 背景

当前 Racingline 已经支持：

1. 创建规则集和不可变规则版本。
2. 对日期区间发起 Rearview run。
3. 按交易日查看股票池、TopN 买入信号和个股分析页。

这些能力回答了“某个交易日哪些证券满足策略条件”。但策略研究下一步需要回答：

1. 每日信号转成持仓后，组合收益曲线如何。
2. 手续费、滑点和调仓频率对结果影响多大。
3. 止盈和止损条件是否改善回撤或损害收益。
4. 策略在现金、持仓、交易、净值和回撤层面是否可解释。

当前 RFC 0018 和 RFC 0019 都把交易、组合调仓和完整回测列为第一版非目标。本 RFC 明确下一阶段边界：新增研究型虚拟账户和组合调仓净值，不进入实盘交易和真实券商清算。

## 目标

1. 用户创建策略时，默认得到一个 `initial_cash = 1,000,000 CNY` 的虚拟账户模板。
2. 用户可以在策略创建和策略版本发布前配置账户初始资金、交易费率、滑点、仓位分配、调仓规则、止盈和止损条件。
3. 每个组合运行使用不可变的账户和执行参数快照，保证历史结果可复现。
4. 组合运行以 Rearview 选股 run 的 `buy_signal` 为调仓信号来源，构建每日目标持仓。
5. 第一版支持计算现金、持仓市值、总资产、单位净值、累计收益、日收益、最大回撤和费用汇总。
6. 第一版必须持久化 `portfolio_target`、`portfolio_order`、`portfolio_trade` 和 `portfolio_position_day`，让净值结果可追溯到目标组合、订单、成交和持仓账本。
7. 第一版通过 NATS JetStream 分发组合净值计算任务，为后续完整回测、任务重试和 worker 横向扩展预留边界。
8. 明确防止未来函数的默认执行时点：T 日信号默认在 T+1 交易日成交。
9. 保持现有边界：Racingline 不直接访问 ClickHouse 或 PostgreSQL，不在浏览器内计算权威净值。

## 非目标

1. 不实现实盘交易、券商下单、撤单、成交回报、银证转账或真实资金账户。
2. 不实现多用户资金隔离、登录鉴权、权限和审计合规模块；如需要应另起 ADR/RFC。
3. 不模拟盘口撮合、分钟级成交、涨跌停排队、部分成交或真实交易所撮合细节。
4. 不处理融资融券、做空、期权、期货、基金、可转债或多币种资产。
5. 第一版不实现企业行动的真实证券数量调整、分红入账和税务处理；净值口径必须明确标记为研究型价格口径。
6. 不允许用户提交任意 SQL 或自定义 Python/Rust 代码作为回测逻辑。
7. 不在前端重算策略信号、技术指标、成交、持仓或净值。
8. 不把组合运行结果写回选股 run 的 `buy_signal` / `pool_member` 快照。
9. 第一版不要求完整回测报表、持仓归因、基准超额收益或跨策略组合分析；但逐笔成交、订单、目标组合和每日持仓明细必须作为账本表持久化。

## 术语

| 术语 | 说明 |
|---|---|
| 虚拟账户模板 | 策略级默认资金和执行参数配置，例如初始资金、费率、滑点和风控规则。模板可以修改，但修改只影响后续版本或后续运行。 |
| 账户快照 | 某次组合运行实际使用的不可变账户参数。它从模板复制而来，写入 `portfolio_run`，用于复现历史结果。 |
| 组合运行 | 根据一个 Rearview 选股 run 和一个账户快照模拟调仓、成交、持仓和净值的异步任务。 |
| 信号日 | 选股 run 产出 `buy_signal.trade_date` 的日期，表示策略在该日收盘后可知的信号。 |
| 成交日 | 根据执行策略把信号转成虚拟成交的日期。默认是信号日后的下一个交易日。 |
| 研究型净值 | 使用约定价格口径计算的策略研究净值，不代表真实券商账户清算结果。 |
| 组合任务消息 | 写入 NATS JetStream 的组合计算任务，只包含 `portfolio_run_id` 和必要调度 metadata；权威参数仍从 PostgreSQL `portfolio_run` 读取。 |

## 当前事实

当前 Rearview PostgreSQL `rearview` database 已保存：

| 表 | 当前职责 |
|---|---|
| `rule_set` | 策略容器和当前版本指针 |
| `rule_version` | 不可变选股规则版本 |
| `run` | 一次区间选股运行 |
| `run_day` | 选股运行日粒度状态 |
| `pool_member` | 每日股票池成员 |
| `buy_signal` | 每日 TopN 买入信号 |
| `metric_catalog` | 可被规则引用的指标目录 |

当前 Racingline 已有页面：

| 路由 | 当前职责 |
|---|---|
| `/rules` | 规则集和规则版本工作台 |
| `/runs` | 选股运行列表 |
| `/runs/:runId` | 选股运行详情、股票池和买入信号 |
| `/runs/:runId/securities/:securityCode` | 个股分析页 |

本 RFC 在这些事实之上扩展组合模拟，不替换现有选股 run。

当前 `deploy/docker-compose.yml` 已包含 NATS 服务，并以 `-js` 开启 JetStream，数据目录挂载到 `/data`。因此第一版不需要新增基础设施容器，但需要 Rearview 拆分 server / worker crate，增加 NATS 连接配置、stream/consumer 初始化和 worker 消费逻辑。

## 第一版交付边界

第一版验收净值计算和明细账本闭环：

| 范围 | 第一版要求 |
|---|---|
| 账户模板 | 创建策略时默认 `1,000,000 CNY`，支持费率、滑点、止盈止损和调仓参数 |
| 任务分发 | `POST /rearview/portfolio-runs` 创建 `portfolio_run`，通过 NATS JetStream 发布净值计算任务 |
| 计算结果 | 写入 `portfolio_nav`、`portfolio_run.summary`、`portfolio_target`、`portfolio_order`、`portfolio_trade` 和 `portfolio_position_day` |
| UI 展示 | 展示组合运行状态、净值曲线、累计收益、最大回撤和费用汇总；明细 tab 可先轻量展示或作为后续 UI 深化 |
| 明细数据 | 第一版必须持久化目标组合、虚拟订单、成交和每日持仓；warning 和触发事件写入 `portfolio_event` 或 summary |

后续完整回测在同一任务分发边界上扩展：

| 后续能力 | 说明 |
|---|---|
| 明细 UI 深化 | 在已持久化账本上增加分页、筛选、导出、原因解释和证券分析页互跳 |
| 归因和基准 | 增加 benchmark、行业/个股贡献和跨策略比较 |
| 更大规模回测 | 用多个 worker 消费 NATS durable consumer，按日期区间或策略批次拆分任务 |

## 产品工作流

### 1. 创建策略和默认账户

用户在 `/rules` 创建新策略时，前端先读取 Rearview 默认市场模板，再用返回值预填表单。默认预填结果应等价于：

```json
{
  "initial_cash": 1000000,
  "currency": "CNY",
  "fee_profile": {
    "commission_rate": 0.0001,
    "commission_rate_max": 0.003,
    "min_commission": 5,
    "stamp_duty_rate_sell": 0.0005,
    "transfer_fee_rate": 0.00001
  },
  "slippage_profile": {
    "mode": "bps",
    "buy_bps": 10,
    "sell_bps": 10
  },
  "rebalance_policy": {
    "frequency": "signal_day",
    "target_weighting": "equal_weight",
    "max_positions": 10,
    "lot_size": 100,
    "min_trade_lots": 1,
    "cash_reserve_pct": 0
  },
  "risk_exit_policy": {
    "trigger_timing": "close_confirm_next_open",
    "exit_rules": []
  }
}
```

默认值应可编辑，但 UI 必须把默认一百万资金作为创建策略时的明确初始状态。发布规则版本时，影响结果的账户和执行参数必须进入版本快照或组合运行快照。

### 2. 发起选股 run

用户仍按现有方式选择规则版本、日期区间和 `top_n` 发起 Rearview run。选股 run 的职责不变：生成每日股票池和 TopN 买入信号，不计算持仓和净值。

### 3. 发起组合运行

当选股 run 成功后，用户可以在 `/runs/:runId` 点击“构建组合”：

1. 选择账户模板或使用策略默认账户模板。
2. 确认初始资金、费率、滑点、止盈止损、调仓频率和目标权重。
3. 创建 `portfolio_run`。
4. Rearview 写入 PostgreSQL 权威状态，并向 NATS JetStream 发布组合净值计算任务。
5. Racingline 跳转到 `/portfolios/:portfolioRunId` 查看进度和结果。

### 4. 查看组合结果

组合结果页展示：

1. 净值曲线、累计收益、日收益和回撤。
2. 现金、持仓市值、总资产和仓位比例。
3. 费用和滑点成本汇总。
4. 与原始选股 run、规则版本和账户快照的可追溯链接。
5. 第一版提供每日调仓目标、虚拟订单、虚拟成交和持仓明细的轻量查看或 API 查询能力；后续版本再深化筛选、导出、归因和图表联动。

## 核心设计决策

### 账户模板与组合运行分离

策略默认账户是模板，不是会被每次回测消耗的真实账户余额。每次组合运行都复制模板参数，形成不可变账户快照。

原因：

1. 同一个选股 run 可以用不同费率、滑点和止损参数重复模拟。
2. 历史组合运行必须可复现，不能因为用户修改策略默认账户而变化。
3. 用户认知上“策略默认一百万”是研究配置，不是实盘资金池。

账户模板不固化到 `rule_version`。`rule_version` 只保存选股规则、metric dependency、`rule_hash` 和影响选股结果的内容；虚拟账户、费率、滑点、调仓和卖出规则只在 `portfolio_run.account_snapshot` / `portfolio_run.execution_snapshot` 中固化。这样同一个规则版本可以用不同账户模板重复做组合净值实验，同时每次组合运行仍可复现。

### T 日信号默认 T+1 成交

默认执行时点：

```text
T 日收盘后得到 buy_signal
T+1 交易日按配置价格成交
T+1 收盘后计算持仓市值和净值
```

第一版默认不允许同一交易日使用 T 日收盘指标又按 T 日收盘价买入。若后续要支持 `same_day_close` 研究模式，必须在 UI 和 API 中标记为可能包含未来函数的实验模式，不能作为默认。

### 研究型价格口径

第一版组合模拟使用 ClickHouse mart 中的日频价格行。策略回测和长期收益率评估默认使用 `backward_adjusted` 后复权价格，用于把分红、送转和配股等企业行动对历史价格序列的影响累计到当前口径，保持长期收益率评估的连续性。费用和滑点按同一研究价格口径计算成交金额。

第一版不允许用户切换组合净值价格口径。所有组合净值、成交参考价、持仓估值和收益率计算统一使用 `backward_adjusted` 后复权口径。前端不提供价格口径选择控件，API 也不接受组合运行级价格口径覆盖参数。

价格口径分工：

| 价格口径 | 第一版用途 |
|---|---|
| `backward_adjusted` | 策略回测、长期收益率评估和组合净值唯一口径 |
| `forward_adjusted` | 个股图表、形态观察和与现有技术指标口径对齐的分析视图 |
| `unadjusted` | 原始行情检查，不作为组合净值口径 |

该口径仍不是券商真实清算账本。后复权价格适合研究长期收益率，但不等同真实成交价格、真实持仓数量或真实分红入账。

后续如果需要真实持仓数量、分红、送转和除权处理，应新增企业行动和清算模型 RFC，而不是把未处理的非复权价格结果伪装成真实账户。

### TOPN 等权和空闲仓位递补

第一版目标组合从每日 TopN 买入信号构建，只支持 `equal_weight`。`max_positions` 表示组合最多持仓数量，通常等于用户发起选股 run 时的 `top_n`。每个持仓槽位的目标权重为：

```text
target_weight_per_position = (1 - cash_reserve_pct) / max_positions
```

第一版按“空闲仓位递补”而不是每日全量换仓执行：

1. 组合从回测区间内第一个出现买入信号的信号日开始进入建仓尝试；如果该日所有候选都不满足成交条件，则不建仓并继续等待后续信号日。
2. 实际建仓只由第一个可成交买入信号触发。可成交必须同时满足：成交日存在后复权开盘价、买入数量按手数取整后不少于 1 手、可用现金足以覆盖成交金额和买入费用。
3. 已持仓股票不会因为再次出现在 `buy_signal` 中而重复买入或加仓；同一成交日已经生成买入订单的股票也不重复下单。
4. 只有当止盈、止损、时间止损、指标止损或清仓规则卖出后出现空闲仓位，才从后续买入信号中按 `rank` 升序调入新股票。
5. 若某个买入候选因为价格缺失、目标金额不足 1 手或现金不足而无法成交，该候选被跳过且不占用 TOPN 持仓槽位，继续检查下一个 `rank` 候选。
6. 若当日可用买入信号不足以填满 `max_positions`，剩余仓位和资金保持空闲。
7. 建仓不要求一次填满 TOPN；每次只对实际可成交候选下单，空闲仓位保留到后续信号日继续按 rank 顺序递补。
8. 后续可以增加按 `score` 加权、波动率倒数加权、全量再平衡或自定义约束，但第一版不引入优化器。

第一版 A 股下单数量按手约束，默认一手为 100 股：

```text
lot_size = 100
min_trade_lots = 1
min_trade_quantity = lot_size * min_trade_lots
```

买入订单必须至少成交 1 手。若目标金额或可用现金不足以覆盖 1 手成交金额和买入费用，则跳过该候选并记录 `cash_insufficient_for_min_lot` 或 `target_amount_below_min_lot` 事件。

## 调仓模型

### 输入

组合运行输入：

| 输入 | 来源 |
|---|---|
| `run_id` | Rearview 选股 run |
| `rule_version_id` / `rule_hash` | 选股 run 已保存 |
| `buy_signal` | PostgreSQL `buy_signal` |
| 交易日历 | ClickHouse mart 或现有交易日来源 |
| 日频价格 | `fleur_marts.mart_stock_quotes_daily`，通过 Rearview 间接读取 |
| 账户快照 | `portfolio_run.account_snapshot` |
| 费率、滑点、卖出规则 | `portfolio_run.execution_snapshot` |

### 调仓日期

第一版支持：

| `frequency` | 行为 |
|---|---|
| `signal_day` | 每个有 `buy_signal` 的信号日生成目标组合，默认下一交易日成交 |
| `weekly` | 每周最后一个有信号的交易日生成目标组合 |
| `monthly` | 每月最后一个有信号的交易日生成目标组合 |

默认值为 `signal_day`。如果选股 run 某日无信号，不强制清仓；是否清仓由 `empty_signal_action` 控制：

| `empty_signal_action` | 行为 |
|---|---|
| `hold` | 保持现有持仓，只执行止盈止损 |
| `clear` | 目标组合为空，下一成交日清仓 |

第一版默认 `hold`，避免单日无信号导致过度换手。

### 目标组合生成

对一个信号日：

组合在回测区间内第一个出现买入信号的信号日开始执行本流程；在首次可成交买入之前不产生持仓，所有不可成交候选只记录事件或 warning，不占用 TOPN 持仓槽位。

1. 读取该日 `buy_signal`，按 `rank` 升序。
2. 先根据上一交易日日终持仓和当日卖出触发结果，生成卖出订单；卖出执行后释放现金和持仓槽位。
3. 读取卖出后的持仓，构造当前已持仓证券集合。
4. 计算空闲仓位数：

```text
vacant_slots = max_positions - current_position_count_after_sells
```

5. 若 `vacant_slots <= 0`，不生成买入目标，即使当日有更高 rank 的新信号也不主动换仓。
6. 从买入候选中排除已持仓证券和当日已生成买入订单的证券；已持仓股票不会重复买入，也不因再次出现在 `buy_signal` 中生成加仓订单。
7. 按 `rank` 升序遍历剩余买入候选，逐个判断价格、手数和现金条件；只有实际可成交候选才占用空闲仓位，最多成交 `vacant_slots` 个候选。
8. 每个新买入候选按 TOPN 等权槽位生成目标金额：

```text
target_amount = total_equity_after_sells * (1 - cash_reserve_pct) / max_positions
```

9. 根据成交日后复权开盘价、买入滑点和费用估算可成交数量。买入数量必须向下取整到 100 股整数倍，且不少于 1 手：

```text
raw_quantity = target_amount / buy_execution_price
lot_quantity = floor(raw_quantity / lot_size) * lot_size
```

10. 若 `lot_quantity < min_trade_quantity`，跳过该候选并记录事件，不占用空闲仓位，继续尝试下一个 rank 候选。
11. 若现金不足以支付 `lot_quantity * buy_execution_price + fees`，按现金可承受数量重新向下取整到手；仍不足 1 手则跳过该候选并记录事件，不占用空闲仓位。
12. 只有实际成交的候选才占用 TOPN 空闲仓位，并生成 `portfolio_target`、`portfolio_order` 和成交后 `portfolio_trade` 快照；未成交候选不阻止后续 rank 候选继续尝试。

第一版只要求：

| `target_weighting` | 行为 |
|---|---|
| `equal_weight` | 入选证券等权 |

### 卖出优先级

同一个成交日同时存在多条卖出规则和调仓卖出时，卖出原因优先级：

1. `fixed_stop_loss`
2. `indicator_stop_loss`
3. `take_profit`
4. `time_stop_loss`
5. `rebalance`

优先级只影响 `exit_reason` 和审计解释，不应导致重复卖出。同一证券同一成交日最多生成一条净卖出订单。

### 买入现金约束

卖出成交和买入成交在同一成交日按以下顺序处理：

1. 先处理卖出，扣除卖出费用，释放现金。
2. 再按空闲仓位数和买入信号 `rank` 顺序逐个尝试买入。
3. 每个买入候选都必须满足至少 1 手和现金足额条件；不满足则跳过并尝试下一个 rank 候选。
4. 第一版不做多只候选之间的比例缩放；按 rank 顺序逐笔检查并成交。
5. 买入后现金不得为负。

第一版不使用保证金，不允许融资买入。

## 费率和滑点

### 费率模型

第一版费率配置来自系统市场默认模板。Rearview 必须从 PostgreSQL `market_fee_template` 读取当前 active 模板；如果缺少 active 模板，账户模板创建、组合运行创建和前端默认表单加载都应返回明确错误，不能在代码里静默回退硬编码费率。

费率字段：

| 字段 | 说明 |
|---|---|
| `commission_rate` | 买卖双边佣金率 |
| `commission_rate_max` | 佣金率上限 |
| `min_commission` | 每笔最低佣金 |
| `stamp_duty_rate_sell` | 卖出印花税率 |
| `transfer_fee_rate` | 过户费率，买卖双边 |

第一版初始化一条 A 股默认市场模板：

| 费用项目 | 默认费率 | 收取方向 |
|---|---:|---|
| 印花税 | 0.05%（万分之五） | 仅卖出单边收取 |
| 过户费 | 0.001%（十万分之一） | 买卖双向收取 |
| 佣金 | 0.01%（万分之一），最高不超过 0.3%（千分之三） | 买卖双向收取 |

费用计算：

```text
effective_commission_rate = min(commission_rate, commission_rate_max)
commission = max(gross_amount * effective_commission_rate, min_commission)
stamp_duty = sell_gross_amount * stamp_duty_rate_sell
transfer_fee = gross_amount * transfer_fee_rate
total_fee = commission + stamp_duty + transfer_fee
```

如果成交金额过小，最低佣金可能导致该笔交易不经济。第一版可以保留该成交，但需要在交易明细中展示费用占比。

### 滑点模型

第一版默认使用成交日开盘价作为 `reference_price`。策略回测默认价格口径为后复权，因此默认 `reference_price` 是成交日 `open_price_backward_adj`；买入按开盘价上浮，卖出按开盘价下浮，默认滑点为 `0.1%`，也就是 `10 bps`。

第一版支持两种滑点模式：

| `mode` | 行为 |
|---|---|
| `bps` | 买入价上浮 `buy_bps`，卖出价下调 `sell_bps` |
| `fixed_pct` | 等价于百分比滑点，语义上更适合 UI 输入 |

默认：

```text
reference_price = execution_trade_date.open_price_backward_adj
buy_execution_price = reference_price * (1 + buy_slippage)
sell_execution_price = reference_price * (1 - sell_slippage)
```

第一版不实现基于成交量、换手率、涨跌停和市场冲击的动态滑点。后续可以扩展 `volume_participation` 模型。

## 卖出规则

### 默认触发口径

用户可配置多条卖出条件，任一条件触发即生成卖出信号。第一版默认使用收盘确认、下一交易日开盘价成交：

```text
trigger_date = T
execution_date = next_trade_date(T)
sell_reference_price = execution_date.open_price_backward_adj
sell_execution_price = sell_reference_price * (1 - sell_slippage)
```

触发日在 T 日，成交日在 T+1。这样可以避免在只有日频 OHLC 的情况下假设盘中触发顺序。

卖出条件类型：

| 类型 | 规则 |
|---|---|
| 固定止损 | 跌破买入价 N% 则卖出 |
| 指标止损 | 跌破指定指标值则卖出，例如跌破 20 日均线 |
| 止盈 | 涨幅达到 N% 则卖出 |
| 时间止损 | 持仓满 N 天后，若累计收益率低于阈值则卖出 |

建议结构：

```json
{
  "trigger_timing": "close_confirm_next_open",
  "exit_rules": [
    {
      "type": "fixed_stop_loss",
      "loss_pct": 0.08
    },
    {
      "type": "indicator_stop_loss",
      "price_metric": "close_price_backward_adj",
      "indicator_metric": "price_ma_20",
      "operator": "cross_below"
    },
    {
      "type": "take_profit",
      "profit_pct": 0.2
    },
    {
      "type": "time_stop_loss",
      "holding_days": 10,
      "max_return_pct": 0
    }
  ]
}
```

`indicator_stop_loss` 的 `indicator_metric` 必须来自 Rearview metric catalog 或明确 allowlist，不允许用户输入任意 SQL 或前端自定义公式。第一版如果 mart 暂无对应指标字段，应在 explain 或 portfolio run 校验阶段返回字段级错误。

触发公式：

```text
unrealized_return_close = close_price / entry_price - 1
fixed_stop_loss triggered when unrealized_return_close <= -loss_pct
take_profit triggered when unrealized_return_close >= profit_pct
indicator_stop_loss triggered when close_price crosses below indicator_metric
time_stop_loss triggered when holding_days >= N and unrealized_return_close < max_return_pct
```

### 可选 OHLC 触发

后续可以增加 `intraday_ohlc_conservative`：

1. 止损用当日 `low` 判断。
2. 止盈用当日 `high` 判断。
3. 同一天同时命中止盈和止损时，默认按 `stop_loss_first` 保守处理。

该模式第一版不作为默认，因为日频数据无法知道真实盘中先后顺序。

### 持仓天数限制

时间止损的持仓天数按交易日计数。第一版默认只在达到 `holding_days` 且累计收益率低于 `max_return_pct` 时卖出；如果用户需要“持仓满 N 天无条件卖出”，后续可新增 `time_exit` 类型，不复用 `time_stop_loss`。

## 净值计算

### 日终资产

每个交易日写入一条 `portfolio_nav`：

```text
cash_balance = previous_cash + sell_proceeds - buy_cost - fees
position_market_value = sum(position_quantity * close_price)
total_equity = cash_balance + position_market_value
nav = total_equity / initial_cash
daily_return = total_equity / previous_total_equity - 1
```

第一条净值记录：

```text
nav = 1.0
cash_balance = initial_cash
position_market_value = 0
total_equity = initial_cash
```

### 成本和收益字段

组合运行至少汇总：

| 指标 | 说明 |
|---|---|
| `initial_cash` | 初始资金 |
| `ending_equity` | 期末总资产 |
| `total_return` | 期末累计收益 |
| `annualized_return` | 年化收益，按交易日数折算 |
| `max_drawdown` | 最大回撤 |
| `daily_return_volatility` | 日收益波动率 |
| `turnover` | 期间换手率 |
| `trade_count` | 成交笔数 |
| `total_fee` | 总交易费用 |
| `total_slippage_cost` | 滑点成本估算 |
| `win_rate_closed_trades` | 已平仓交易胜率 |

第一版如果不实现夏普比率或基准超额收益，应在 UI 中不展示占位值。

### 缺失价格处理

如果成交日或估值日缺少价格：

| 情况 | 第一版行为 |
|---|---|
| 买入缺少成交价格 | 跳过买入，记录 `price_missing` |
| 卖出缺少成交价格 | 保留持仓，记录 `exit_blocked_price_missing` |
| 持仓估值缺少收盘价 | 使用最近一个可用估值价并标记 `stale_price` |

所有缺失价格处理必须写入运行 warning，不能静默吞掉。

## PostgreSQL 数据模型

新增表建议放在 PostgreSQL `rearview` database，由 `pipeline/migrate` 管理 migration。

| 表 | 职责 |
|---|---|
| `market_fee_template` | 系统级市场默认费率模板，作为账户模板表单和组合运行默认费率来源 |
| `virtual_account_template` | 策略级账户模板，保存默认初始资金、币种、费率、滑点、调仓和卖出规则配置 |
| `portfolio_run` | 一次组合运行，保存选股 `run_id`、账户快照、执行快照、状态、日期范围和汇总指标 |
| `portfolio_task_outbox` | 组合任务发布 outbox，保证 PostgreSQL 状态和 NATS 发布之间可恢复 |
| `portfolio_target` | 每个信号日的目标组合权重快照 |
| `portfolio_order` | 根据目标组合和退出规则生成的虚拟订单 |
| `portfolio_trade` | 虚拟成交结果，含价格、数量、金额、费用、滑点和成交原因 |
| `portfolio_position_day` | 每日持仓快照 |
| `portfolio_nav` | 每日现金、持仓市值、总资产和净值 |
| `portfolio_event` | warning、止盈止损触发、价格缺失、现金不足缩放等审计事件 |

第一版最小持久化表：

| 表 | 第一版要求 |
|---|---|
| `market_fee_template` | 必需，并随 migration 初始化 A 股默认模板 |
| `virtual_account_template` | 必需 |
| `portfolio_run` | 必需 |
| `portfolio_task_outbox` | 必需，用于 NATS 任务发布恢复 |
| `portfolio_target` | 必需，保存每个信号日的目标组合权重和来源信号 |
| `portfolio_order` | 必需，保存根据目标组合和卖出规则生成的虚拟订单 |
| `portfolio_trade` | 必需，保存每笔虚拟成交、费用和滑点 |
| `portfolio_position_day` | 必需，保存每日持仓快照 |
| `portfolio_nav` | 必需 |
| `portfolio_event` | 建议必需，用于 warning、止盈止损触发、价格缺失和现金不足缩放；如第一版暂缓建表，必须把等价结构化 warning 写入 `portfolio_run.summary` |

第一版净值计算可以在 worker 内部先构造内存态目标组合、虚拟订单、成交和持仓账本，但 worker 成功结束前必须把这些账本写入对应明细表。`portfolio_nav` 的每一日结果应能通过同一 `portfolio_run_id` 追溯到当日目标、订单、成交和持仓快照。实现时不得只写 summary 或 nav 而丢失明细。

如果 worker 在计算过程中失败，第一版采用 run 级重算语义：重新消费同一个 `portfolio_run_id` 时，可以先删除或覆盖该 run 的非终态 `portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav` 和 `portfolio_event`，再按不可变快照重算整段净值。第一版不要求从某个交易日断点续算。

### `market_fee_template`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `market_fee_template_id` | 主键 |
| `market` | 市场代码，第一版为 `CN_A_SHARE` |
| `name` | 模板名称，例如 `A-share default` |
| `currency` | 默认 `CNY` |
| `fee_profile` | JSONB，保存佣金、印花税、过户费 |
| `slippage_profile` | JSONB，保存默认买卖滑点 |
| `is_default` | 同一市场只能有一个默认模板 |
| `status` | `active` / `archived` |
| `created_at` / `updated_at` | 审计时间 |

第一版 migration 必须初始化一条 active 默认模板：

```json
{
  "market": "CN_A_SHARE",
  "name": "A-share default",
  "currency": "CNY",
  "fee_profile": {
    "commission_rate": 0.0001,
    "commission_rate_max": 0.003,
    "min_commission": 5,
    "stamp_duty_rate_sell": 0.0005,
    "transfer_fee_rate": 0.00001
  },
  "slippage_profile": {
    "mode": "bps",
    "buy_bps": 10,
    "sell_bps": 10
  },
  "is_default": true,
  "status": "active"
}
```

约束：

1. 同一 `market` 最多一个 `is_default = true and status = active` 模板。
2. `fee_profile` 和 `slippage_profile` 必须通过后端 schema 校验，不能只作为任意 JSON 保存。
3. 初始化数据由 `pipeline/migrate` 的 rearview target 管理，避免本地和部署环境默认费率不一致。
4. 更新市场默认模板只影响后续新建账户模板，不回写已有 `virtual_account_template` 或历史 `portfolio_run` 快照。

### `virtual_account_template`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `account_template_id` | 主键 |
| `rule_set_id` | 关联策略 |
| `market_fee_template_id` | 来源市场费率模板，可空但第一版默认填充 |
| `name` | 模板名称，默认 `Default research account` |
| `initial_cash` | 默认 `1000000` |
| `currency` | 默认 `CNY` |
| `fee_profile` | JSONB |
| `slippage_profile` | JSONB |
| `rebalance_policy` | JSONB |
| `risk_exit_policy` | JSONB |
| `status` | `active` / `archived` |
| `created_at` / `updated_at` | 审计时间 |

约束：

1. 每个 `rule_set` 至少有一个默认 active 模板。
2. `initial_cash > 0`。
3. 费率、滑点和仓位比例不得为负。
4. 模板修改不回写历史 `portfolio_run`。
5. 新建策略默认账户模板时，`fee_profile` 和 `slippage_profile` 从 active `market_fee_template` 复制并允许用户在表单中覆盖。

### `portfolio_run`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `portfolio_run_id` | 主键 |
| `source_run_id` | 关联选股 `run.run_id` |
| `rule_version_id` | 冗余保存，便于查询和审计 |
| `rule_hash` | 冗余保存，便于复现 |
| `account_template_id` | 可空，表示来源模板 |
| `account_snapshot` | JSONB，不可变 |
| `execution_snapshot` | JSONB，不可变 |
| `start_date` / `end_date` | 组合运行日期范围 |
| `status` | 运行状态 |
| `dispatch_status` | NATS 任务分发状态：`pending` / `published` / `publish_failed` |
| `nats_stream_sequence` | 可空，NATS publish ack 中的 stream sequence |
| `summary` | JSONB 汇总指标 |
| `error_type` / `error_message` | 错误摘要 |
| `created_at` / `updated_at` / `completed_at` | 审计时间 |

状态建议：

```text
created
dispatching
queued
validating
loading_signals
building_targets
calculating_nav
writing_results
succeeded
failed_validation
failed_market_data
failed_simulation
failed_write
cancelled
```

第一版可以只把 `queued`、`calculating_nav`、`writing_results` 和终态作为对外可见状态；但目标组合、订单、成交和持仓明细必须落库，不得因为不暴露 `building_targets` 中间状态而跳过明细表写入。

### `portfolio_task_outbox`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `outbox_id` | 主键 |
| `portfolio_run_id` | 组合运行 |
| `subject` | NATS subject，第一版为 `rearview.portfolio_run.requested` |
| `payload` | JSONB，包含 `portfolio_run_id`、`source_run_id`、`created_at` |
| `status` | `pending` / `published` / `failed` |
| `attempt_count` | 发布尝试次数 |
| `last_error` | 最近一次发布错误 |
| `nats_stream_sequence` | NATS publish ack sequence |
| `created_at` / `published_at` / `updated_at` | 审计时间 |

约束：

1. `portfolio_run_id` 对第一版创建任务保持唯一，避免重复发布同一组合运行。
2. HTTP 创建组合运行时，`portfolio_run` 和 `portfolio_task_outbox` 在同一个 PostgreSQL 事务内写入。
3. outbox dispatcher 可以独立重试 NATS 发布；发布成功后标记 `published`，并同步 `portfolio_run.dispatch_status = published`。
4. 即使 NATS 发布成功但 HTTP 请求超时，用户也可以通过 `portfolio_run_id` 查询到任务状态。

### `portfolio_trade`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `portfolio_run_id` | 组合运行 |
| `trade_date` | 成交日 |
| `signal_date` | 来源信号日，可空 |
| `security_code` | 证券代码 |
| `side` | `buy` / `sell` |
| `quantity` | 研究型数量 |
| `reference_price` | 未加滑点前价格 |
| `execution_price` | 加滑点后价格 |
| `gross_amount` | 成交金额 |
| `commission` / `stamp_duty` / `transfer_fee` | 费用拆分 |
| `total_fee` | 总费用 |
| `slippage_cost` | 滑点成本估算 |
| `reason` | `rebalance` / `fixed_stop_loss` / `indicator_stop_loss` / `take_profit` / `time_stop_loss` |

### `portfolio_target`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `portfolio_run_id` | 组合运行 |
| `signal_date` | 来源信号日 |
| `execution_date` | 默认下一交易日 |
| `security_code` | 目标证券 |
| `source_rank` | `buy_signal.rank`，可空 |
| `source_score` | `buy_signal.score`，可空 |
| `target_weight` | 目标权重 |
| `target_amount` | 目标金额 |
| `target_quantity` | 研究型目标数量，可空 |
| `target_reason` | `buy_signal` / `clear_empty_signal` / `rebalance` |

主键建议为 `(portfolio_run_id, signal_date, security_code)`。

### `portfolio_order`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `portfolio_order_id` | 主键 |
| `portfolio_run_id` | 组合运行 |
| `signal_date` | 来源信号日，可空 |
| `execution_date` | 预期成交日 |
| `security_code` | 证券代码 |
| `side` | `buy` / `sell` |
| `order_quantity` | 研究型订单数量 |
| `order_amount` | 目标订单金额 |
| `reference_price` | 下单时参考价，可空 |
| `reason` | `rebalance` / `fixed_stop_loss` / `indicator_stop_loss` / `take_profit` / `time_stop_loss` |
| `status` | `planned` / `filled` / `skipped_price_missing` / `cancelled_cash_scaled` |
| `event_ref` | 可空，关联 `portfolio_event` |

### `portfolio_position_day`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `portfolio_run_id` | 组合运行 |
| `trade_date` | 持仓日期 |
| `security_code` | 证券代码 |
| `quantity` | 研究型持仓数量 |
| `cost_basis` | 研究型成本金额 |
| `average_entry_price` | 平均买入价 |
| `close_price` | 当日估值价，固定为后复权收盘价 |
| `market_value` | 当日市值 |
| `unrealized_pnl` | 未实现收益 |
| `unrealized_return` | 未实现收益率 |
| `holding_days` | 按交易日计的持仓天数 |
| `is_stale_price` | 是否使用最近可用估值价 |

主键建议为 `(portfolio_run_id, trade_date, security_code)`。

### `portfolio_nav`

建议字段：

| 字段 | 类型语义 |
|---|---|
| `portfolio_run_id` | 组合运行 |
| `trade_date` | 估值日 |
| `cash_balance` | 现金 |
| `position_market_value` | 持仓市值 |
| `total_equity` | 总资产 |
| `nav` | 单位净值 |
| `daily_return` | 日收益 |
| `drawdown` | 当前回撤 |
| `gross_exposure` | 总仓位 |
| `position_count` | 持仓数量 |
| `turnover` | 当日换手 |
| `fee_amount` | 当日费用 |
| `warning_count` | 当日 warning 数 |

主键建议为 `(portfolio_run_id, trade_date)`。

## NATS 任务分发

第一版使用 `deploy/docker-compose.yml` 中已有的 NATS JetStream。JetStream 提供持久 stream、durable consumer、ack 和未 ack 消息重投递；因此任务消费语义按 at-least-once 设计，所有 worker 写入必须幂等。

第一版 stream 和 durable consumer 初始化由 Rearview 运行进程负责幂等 ensure，不依赖人工 NATS CLI 或部署脚本预创建。`rearview-server` 启动时 ensure stream；`rearview-portfolio-worker` 启动时 ensure stream 和 worker durable consumer。若 ensure 失败，进程应启动失败并记录明确错误，避免 HTTP 层已接受任务但 worker 永远无法消费。

### Crate 边界与运行入口

第一版把 Rearview 的 HTTP API 和组合净值 worker 拆成两个可部署 Rust binary，并抽出共享 library crate。不要由 Racingline、Dagster job 或 Furnace CLI 充当 worker，也不要让 server 和 worker 两个 binary 互相依赖。

第一版必须拆分为三个 Rust crate：

```text
engines/crates/
├── rearview-core/
├── rearview-server/
└── rearview-portfolio-worker/
```

职责划分：

| Crate | 类型 | 职责 |
|---|---|---|
| `rearview-core` | library | config、error、PostgreSQL repository、ClickHouse query、NATS/outbox model、组合净值计算、目标/订单/成交/持仓账本、shared types |
| `rearview-server` | binary | Axum HTTP API，创建 `portfolio_run`，写 PostgreSQL outbox，发布 NATS 任务，查询组合净值和明细账本 |
| `rearview-portfolio-worker` | binary | 消费 NATS JetStream，读取 `portfolio_run` 快照，查询 ClickHouse mart，计算组合账本和 `portfolio_nav`，写回 PostgreSQL |

依赖方向固定为：

```text
rearview-server ────────────────┐
                                ├── rearview-core
rearview-portfolio-worker ──────┘
```

禁止方向：

```text
rearview-server -> rearview-portfolio-worker
rearview-portfolio-worker -> rearview-server
```

运行入口：

```bash
cargo run -p rearview-server -- serve
cargo run -p rearview-portfolio-worker -- run
```

部署上是两个进程：

```text
rearview-api
rearview-portfolio-worker-1
```

后续回测变重时，优先横向扩展同一个 worker 入口：

```text
rearview-api
rearview-portfolio-worker-1
rearview-portfolio-worker-2
rearview-portfolio-worker-n
```

只有当回测能力发展成独立领域，例如多策略批量回测、参数网格、归因、benchmark 和跨策略 OLAP，再评估拆出新的服务或更专门的 crate，例如 `engines/crates/backtester`。第一版不需要新增第三个可部署服务。

### Stream 和 subject

建议约定：

| 项 | 值 |
|---|---|
| Stream | `REARVIEW_PORTFOLIO` |
| Subjects | `rearview.portfolio_run.*` |
| 创建任务 subject | `rearview.portfolio_run.requested` |
| 可选进度 subject | `rearview.portfolio_run.progress` |
| 可选完成 subject | `rearview.portfolio_run.completed` |
| Worker durable consumer | `rearview-portfolio-worker` |
| Queue group | `rearview-portfolio-workers` |

第一版只要求 `requested` subject。`progress` 和 `completed` 可以先不发布，因为 PostgreSQL `portfolio_run` 和 `portfolio_nav` 是 UI 查询的权威来源。

### 消息 payload

NATS 消息只携带调度信息：

```json
{
  "portfolio_run_id": "portfolio-run-uuid",
  "source_run_id": "run-uuid",
  "requested_at": "2026-06-16T00:00:00Z"
}
```

消息不得携带完整账户参数、规则快照或行情数据。worker 收到消息后必须从 PostgreSQL `portfolio_run` 读取不可变 `account_snapshot` 和 `execution_snapshot`，保证 NATS 消息重投递或延迟消费不会改变计算事实。

### Worker 幂等规则

1. worker 收到消息后，先用 `portfolio_run_id` 查询 PostgreSQL。
2. 若 `status` 已是 `succeeded`、`failed_*` 或 `cancelled`，直接 ack。
3. 若 `status` 是 `queued` 或 `created`，用事务或行锁把状态推进到 `calculating_nav`。
4. 计算写入明细表和 `portfolio_nav` 时必须以 `portfolio_run_id` 为幂等边界；`portfolio_nav` 和 `portfolio_position_day` 使用 `(portfolio_run_id, trade_date, ...)` 主键，`portfolio_target`、`portfolio_order`、`portfolio_trade` 也必须具备可重复写入的唯一键或 run 级清理策略。
5. 只有在 `portfolio_run` 写入终态后才 ack NATS 消息。
6. worker 崩溃或未 ack 时，JetStream 可以重投递；重投递必须依赖上述状态机和主键保持幂等。

### HTTP 与 NATS 的关系

`POST /rearview/portfolio-runs` 不直接执行净值计算。它只负责：

1. 校验 source run 已成功。
2. 写入 `portfolio_run` 和 `portfolio_task_outbox`。
3. 尝试或等待 outbox dispatcher 发布 NATS 任务。
4. 返回 `202 Accepted` 和 `portfolio_run_id`。

第一版 outbox dispatcher 先作为 `rearview-server` 进程内的后台任务实现：server 启动后周期性扫描 `portfolio_task_outbox.status = pending or failed` 且未超过重试间隔的记录，发布到 NATS JetStream，成功后更新 outbox 和 `portfolio_run.dispatch_status`。后续如果 HTTP server 负载和任务分发需要独立伸缩，再把 dispatcher 抽成单独 binary；第一版不新增第三个可部署服务。

如果 NATS 暂不可用，第一版可以选择：

| 策略 | 行为 |
|---|---|
| 推荐 | 保留 `portfolio_task_outbox.status = pending`，返回 `202 Accepted`，页面显示 `dispatch_status = pending`，后台继续重试 |
| 严格 | 返回 `503 Service Unavailable`，但仍保留可恢复 outbox 记录 |

推荐策略更适合后续长回测：HTTP 创建请求和任务分发解耦，NATS 短暂不可用不会丢失组合运行。

## Rearview API

新增 API 以 `VITE_REARVIEW_API_BASE_URL` 为 base URL，仍由 Racingline 通过 Rearview 访问。

| Method | Path | 阶段 | 用途 |
|---|---|---|---|
| `GET` | `/rearview/market-fee-templates/default?market=CN_A_SHARE` | 第一版 | 查询系统默认市场费率和滑点模板，用于账户表单预填 |
| `GET` | `/rearview/rule-sets/{rule_set_id}/account-templates` | 第一版 | 查询策略账户模板 |
| `POST` | `/rearview/rule-sets/{rule_set_id}/account-templates` | 第一版 | 创建账户模板 |
| `PATCH` | `/rearview/account-templates/{account_template_id}` | 第一版 | 更新模板，只影响后续运行 |
| `POST` | `/rearview/portfolio-runs` | 第一版 | 从选股 run 创建组合运行，写入 outbox 并发布 NATS 任务 |
| `GET` | `/rearview/portfolio-runs` | 第一版 | 查询组合运行列表 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}` | 第一版 | 查询组合运行 summary、状态和 `dispatch_status` |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/nav` | 第一版 | 查询净值曲线 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/targets` | 第一版 | 查询调仓目标 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/orders` | 第一版 | 查询虚拟订单 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/trades` | 第一版 | 查询成交明细 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/positions` | 第一版 | 查询每日或当前持仓 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/events` | 第一版 | 查询 warning 和触发事件；若未建 `portfolio_event`，从 summary warning 投影 |

### 创建组合运行请求

```json
{
  "source_run_id": "run-uuid",
  "account_template_id": "template-uuid",
  "account_override": {
    "initial_cash": 1000000
  },
  "execution_override": {
    "fee_profile": {
      "commission_rate": 0.0001,
      "commission_rate_max": 0.003,
      "min_commission": 5,
      "stamp_duty_rate_sell": 0.0005,
      "transfer_fee_rate": 0.00001
    },
    "slippage_profile": {
      "mode": "bps",
      "buy_bps": 10,
      "sell_bps": 10
    },
    "rebalance_policy": {
      "frequency": "signal_day",
      "target_weighting": "equal_weight",
      "max_positions": 10,
      "lot_size": 100,
      "min_trade_lots": 1,
      "cash_reserve_pct": 0,
      "empty_signal_action": "hold"
    },
    "risk_exit_policy": {
      "trigger_timing": "close_confirm_next_open",
      "exit_rules": [
        {
          "type": "fixed_stop_loss",
          "loss_pct": 0.08
        },
        {
          "type": "take_profit",
          "profit_pct": 0.2
        },
        {
          "type": "time_stop_loss",
          "holding_days": 10,
          "max_return_pct": 0
        }
      ]
    }
  }
}
```

后端响应 `202 Accepted`：

```json
{
  "portfolio_run_id": "portfolio-run-uuid",
  "source_run_id": "run-uuid",
  "status": "queued",
  "dispatch_status": "published",
  "start_date": "2021-01-01",
  "end_date": "2025-12-31",
  "summary": null
}
```

### 错误语义

错误响应继续使用 Rearview 统一结构：

```json
{
  "error_type": "validation_error",
  "message": "initial_cash must be greater than 0",
  "field_path": "account_override.initial_cash"
}
```

常见错误：

| `error_type` | 场景 |
|---|---|
| `validation_error` | 参数非法 |
| `source_run_not_succeeded` | 选股 run 尚未成功 |
| `market_data_missing` | 必要价格数据缺失过多 |
| `simulation_error` | 现金、持仓或净值计算异常 |
| `write_error` | PostgreSQL 写入失败 |
| `dispatch_pending` | 组合运行已创建，但 NATS 发布仍在 outbox 重试 |
| `dispatch_error` | NATS 发布失败且超过重试策略 |

## Racingline 页面设计

### `/rules` 策略创建扩展

规则工作台新增“虚拟账户”配置区：

1. 初始资金，默认 `1,000,000`。
2. 手续费配置：佣金率、最低佣金、卖出印花税、过户费。
3. 滑点配置：买入滑点、卖出滑点。
4. 调仓配置：最大持仓数、权重方式、现金保留比例、无信号日行为。
5. 卖出规则配置：固定止损、指标止损、止盈、时间止损。

创建新策略或新账户模板时，前端必须先调用 `GET /rearview/market-fee-templates/default?market=CN_A_SHARE`，用后端返回的默认市场模板预填费率和滑点字段。用户可以在表单中修改这些默认值；提交后，实际使用值保存到 `virtual_account_template`，后续组合运行再复制到 `portfolio_run` 快照。

这些控件应使用数字输入、百分比输入、开关和选择器，不要求用户编辑 JSON。JSON 只可作为只读预览或排查辅助。

### `/runs/:runId` 结果页扩展

选股 run 成功后新增组合入口：

1. 显示当前 run 是否已有组合运行。
2. 支持从当前 run 创建组合运行。
3. 创建前弹出或进入配置面板，展示账户模板和可覆盖参数。
4. 创建成功后跳转 `/portfolios/:portfolioRunId`。

### `/portfolios`

新增组合运行列表：

| 字段 | 说明 |
|---|---|
| `portfolio_run_id` | 可复制 |
| `rule_set_name` | 策略名称 |
| `source_run_id` | 来源选股 run |
| `date_range` | 日期范围 |
| `initial_cash` | 初始资金 |
| `status` | 状态 |
| `total_return` | 累计收益 |
| `max_drawdown` | 最大回撤 |
| `total_fee` | 总费用 |
| `created_at` | 创建时间 |

支持按策略、状态、日期区间和关键词筛选。

### `/portfolios/:portfolioRunId`

组合详情页采用工作台布局：

1. 顶部 summary：策略、来源 run、日期区间、状态、初始资金、期末资产、累计收益、最大回撤。
2. 净值图：单位净值、回撤、可选现金/仓位曲线。
3. 第一版 tabs：`净值`、`持仓`、`成交`、`调仓目标`、`参数`。
4. `事件` tab 可以第一版轻量展示 warning，也可以先把 warning 合并到 `净值` 或 `参数` 区域。
5. 右侧或顶部参数摘要：费率、滑点、调仓规则、卖出规则。
6. 与选股 run 和个股分析页互跳：成交或持仓行可打开对应证券分析页。

状态处理：

- 运行中时轮询 `GET /rearview/portfolio-runs/{portfolio_run_id}` 和局部结果接口。
- `dispatch_status = pending` 时展示任务已创建、等待 NATS 分发的状态，不重复创建组合运行。
- 失败时展示 `error_type`、`error_message` 和已写出的事件。
- 成功后停止轮询。

## 数据边界

### Racingline

Racingline 负责：

1. 表单化配置账户和执行参数。
2. 发起组合运行。
3. 第一版展示组合状态、NATS 分发状态、净值曲线、summary、持仓、成交和调仓目标。
4. 后续深化明细筛选、导出、归因、benchmark 和跨策略分析。
5. 明确标记研究型净值和价格口径。

Racingline 不负责：

1. 直接读取 ClickHouse 或 PostgreSQL。
2. 在浏览器内计算权威成交、持仓或净值。
3. 生成或修改选股 run 结果。

### Rearview

Rearview 负责：

1. 校验账户、费率、滑点、调仓和卖出规则参数。
2. 从选股 run 读取 `buy_signal`。
3. 通过 ClickHouse mart 读取价格数据。
4. 创建 `portfolio_run` 和 outbox，并通过 NATS JetStream 分发净值计算任务。
5. 第一版计算并写入净值曲线、summary、目标组合、订单、成交和每日持仓。
6. 后续在已持久化账本上扩展完整事件、归因、benchmark 和跨策略分析。
7. 将组合运行状态和结果写入 PostgreSQL。
8. 提供 UI 友好的分页和筛选 API。

Rearview 不负责：

1. 重算 Furnace/dbt 指标。
2. 真实交易所撮合和券商清算。
3. 修改历史选股 run 快照。

### ClickHouse marts

第一版需要稳定消费：

| Mart | 用途 |
|---|---|
| `mart_stock_quotes_daily` | 交易日价格、收盘估值、ST/停牌、涨跌停辅助字段 |
| `mart_stock_trend_indicator` | 可选，用于后续组合归因，不是第一版净值必需 |
| `mart_stock_momentum_indicator` | 可选，用于后续组合归因，不是第一版净值必需 |

如后续组合计算需要更高性能，可以新增面向 Rearview 的宽表或 ClickHouse 结果事实表，但第一版先使用 PostgreSQL 保存组合结果，保证应用查询和审计简单。

### NATS JetStream

NATS 负责：

1. 持久化组合计算任务消息。
2. 通过 durable consumer 和 queue group 支持 `rearview-portfolio-worker` 重启、重投递和横向扩展。
3. 解耦 HTTP 创建请求和长时间净值计算。

NATS 不负责：

1. 保存账户参数、规则快照、行情数据或净值结果。
2. 充当权威任务状态库；权威状态仍是 PostgreSQL `portfolio_run`。
3. 替代 PostgreSQL outbox；outbox 用于恢复 HTTP 事务和 NATS 发布之间的不一致。

## 验证要求

文档阶段：

```bash
make docs-check
git diff --check
```

后端实现阶段至少运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 PostgreSQL migration 时追加：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
```

涉及 `market_fee_template` 初始化时追加数据核验，确认 `CN_A_SHARE` active 默认模板存在且费率值符合本 RFC：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
```

涉及 NATS 任务分发时追加：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d nats
curl -fsS "http://127.0.0.1:${NATS_MONITOR_PORT:-34056}/healthz"
```

涉及组合净值 worker 时追加进程入口验证：

```bash
cd engines
cargo run -p rearview-portfolio-worker -- run --help
```

前端实现阶段至少运行：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run test
npm run build
```

浏览器验收使用：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## 验收标准

1. 创建新策略时，Racingline 默认展示 `1,000,000 CNY` 虚拟账户配置。
2. Racingline 从 Rearview 默认市场模板预填费率和滑点字段；用户可以修改费率、滑点、卖出规则和调仓参数，并保存为策略账户模板。
3. 一个成功的选股 run 可以创建组合运行。
4. 创建组合运行时，Rearview 写入 `portfolio_run` 和 `portfolio_task_outbox`，并通过 NATS JetStream 发布 `rearview.portfolio_run.requested` 消息。
5. `rearview-portfolio-worker` 能消费任务，并以幂等方式推进 `portfolio_run.status`。
6. 组合运行完成后，API 返回 summary、`portfolio_nav` 净值曲线、目标组合、订单、成交和每日持仓明细。
7. 净值曲线从 `1.0` 开始，现金、持仓市值和总资产每日可追溯到持久化明细账本。
8. Racingline 组合详情页能在桌面和移动端查看核心净值结果，无文本重叠或关键数据遮挡。
9. 历史组合运行不受后续账户模板修改影响。
10. UI 明确标记该结果是研究型虚拟组合，不是实盘账户。
11. 第一版必须持久化成交明细、持仓明细、虚拟订单和调仓目标；对应 API 不得返回未实现。完整事件列表如暂未单独建表，必须能从 summary warning 查询到等价结构化信息。
12. 第一版前端不提供组合净值价格口径切换，后端也不接受组合运行级价格口径覆盖参数。
13. Rearview migration 初始化 `CN_A_SHARE` active 默认市场模板，且新策略账户模板默认从该模板复制费率和滑点。

## 待决问题

1. 是否需要基准指数净值和超额收益；如果需要，需要先确定可用 benchmark mart。
2. 是否需要把组合结果同步到 ClickHouse 以支持跨策略 OLAP；第一版建议暂不做。
3. 涨跌停、停牌和无法成交是否在第一版作为硬约束处理，还是先作为 warning 和跳过成交处理。
