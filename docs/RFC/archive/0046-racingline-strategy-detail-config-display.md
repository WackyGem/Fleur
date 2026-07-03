# RFC 0046: Racingline 策略详情页策略配置展示

状态：Implemented
日期：2026-07-02
领域：Racingline, Rearview, Strategy Portfolio, UX
关联系统：app/racingline, engines/crates/rearview-core
相关文档：
- docs/RFC/archive/0034-racingline-step5-portfolio-publish-dialog-tabs.md
- docs/RFC/archive/0028-racingline-strategy-backtest-step5.md
- docs/RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md
- docs/architecture/racingline.md
- docs/architecture/rearview.md

## 摘要

策略组合发布后，用户在 `/dashboard/strategies/:portfolioId` 查看的是 live 运行结果、净值、账户、对账单、信号、持仓和调仓记录。当前详情页缺少一个稳定位置回答“这个策略到底按什么规则运行”。这会让用户在查看表现时难以回忆 Step 1 过滤、Step 2 评分和 Step 4 建仓参数，也不利于把 live 表现和原始策略配置放在同一上下文里审视。

本 RFC 建议在策略详情页页头增加 `策略配置方案` 按钮，点击后打开配置详情层；详情层展示内容与 `/strategies` Step 5「建立组合」弹层中 `策略配置` tab 保持一致，并且必须由组合真实使用的 canonical 配置派生：

1. `指标过滤`：展示过滤条件分组、组内关系和表达式。
2. `权重得分`：展示评分项顺序、加分值和表达式。
3. `建仓摘要`：展示初始资金、每日候选、候选口径、调仓规则、最大持仓、单票上限、交易成本和风控。

我的修订建议是：不考虑移动端，按桌面工作台设计。不要把完整配置插入详情页主内容流，也不设计 hover/focus 提示层。策略详情页的核心任务是阅读 live 结果，完整配置属于按需核对信息。更合理的交互是页头放一个稳定、可键盘访问的 `策略配置方案` 文字按钮，click 打开 popover 或 dialog 承载完整配置。这样既保留配置可发现性，又不打断信号、业绩、账户和对账单的阅读节奏。

2026-07-02 纠偏完成：0074 第一版已实现入口和 Dialog，但原验收使用本地 PostgreSQL 临时注入手写 display snapshot，这不能证明详情页展示的是组合真实配置。Plan 0075 已完成并收敛为从 `rule_snapshot` 和 `execution_config` 派生展示；不新增 `strategy_config_snapshot`、`strategy_config_display`、period label、benchmark label 或 preview 展示字段到后端。展示模板、标签和上下文文案由前端基于 canonical 字段统一生成，0051 example browser smoke 已通过。

## 当前事实

### Step 5 配置展示

Step 5「建立组合」弹层位于 [strategy-page.tsx](../../../app/racingline/src/routes/strategy-page.tsx)，当前 `Tabs` 包含 `策略配置` 和 `回测业绩`。

`策略配置` tab 的三段内容来源如下：

| 区域 | 当前来源 | 展示内容 |
|---|---|---|
| `指标过滤` | `conditionGroups -> publishConditionRows` | `groupLabel`、`logicLabel`、`expression` |
| `权重得分` | `weightIndicators -> publishScoringRows` | `index`、`score`、`expression` |
| `建仓摘要` | `effectiveSimulationSettings` + `backtestExecutionDraft.summary.enabled_exit_rule_count` | 初始资金、每日候选、候选口径、调仓规则、最大持仓、单票上限、交易成本、风控 |

这些行目前由前端编辑态直接生成：

- `formatComparableIndicator()` 格式化 Step 1 条件表达式。
- `formatWeightIndicator()` 格式化 Step 2 评分表达式。
- `publishBuildSummaryRows` 从 Step 4 simulation settings 生成建仓摘要。

### 策略详情页

策略详情页位于 [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx)。当前页面已读取 `StrategyPortfolioRecord`，并展示：

1. 页头：策略名、建仓日、运行天数、持仓数、删除按钮。
2. `策略信号` 或 `待调入信号`。
3. `策略业绩`、`净值走势`、`持仓记录`。
4. `虚拟资金账户` 和 `账户对账单`。

当前详情页没有独立展示 Step 1/2/4 策略配置。它只在虚拟资金账户区块使用了 `portfolioRecord.execution_config.account.initial_cash` 作为初始资金上下文。

### 持久化数据

前端类型 [rearview.ts](../../../app/racingline/src/types/rearview.ts) 中 `StrategyPortfolioRecord` 已包含以下配置字段：

| 字段 | 含义 |
|---|---|
| `rule_snapshot` | 发布时冻结的 `RuleVersionSpec` JSON |
| `rule_hash` | 策略规则 hash |
| `execution_config` | 发布时冻结的 `BacktestExecutionConfig` |
| `execution_config_hash` | 执行配置 hash |
| `benchmark_security_code` | source backtest 业绩基准代码 |
| `source_period_key` / `source_start_date` / `source_end_date` | source backtest 周期和区间 |
| `initial_signal_date` / `live_start_date` | 信号日和 live 建仓日 |
| `ui_display_snapshot` | 历史展示快照 JSON；策略配置展示不应依赖它，也不应为本需求继续扩展 |

Rearview 创建 strategy portfolio 时会从 source backtest run 复制 `ui_display_snapshot`。这是一条历史展示通道，不应继续用于策略配置详情。period label、benchmark label、preview 文案和配置行都可以由前端通过统一模板从 portfolio/backtest record 的结构字段生成，不需要保存到后端。

这意味着详情页已经具备 canonical 配置事实，但还没有一个稳定、类型化、与 Step 5 一致的前端展示模型。后续实现不能简单假设 `ui_display_snapshot` 已经有完整行数据，也不应该在详情页用任意 JSON 路径猜字段。

## 目标

1. 策略详情页提供 `策略配置方案` 入口，打开后展示内容与 Step 5 发布弹层 `策略配置` tab 保持一致。
2. 用户能在不离开详情页、不打断 live 结果阅读的前提下核对策略规则。
3. 同一套配置展示组件和展示模型可被 Step 5 发布弹层和策略详情页复用，并且展示模型由 `RuleVersionSpec` + `BacktestExecutionConfig` 构造。
4. 对 persisted portfolio 使用稳定数据来源，不从 live 结果、持仓或交易记录反推配置。
5. 对 canonical 配置字段缺失或结构非法的组合显式降级，不写多层字段猜测 fallback。

## 非目标

1. 不在本 RFC 中实现代码、迁移或 API。
2. 不改动策略规则、回测计算、portfolio daily run 或 live facts 口径。
3. 不把 Step 3 个股样本、SQL、hash、run id 或 result attempt 作为普通用户默认配置展示。
4. 不在浏览器内重新计算权威持仓、净值、绩效或交易日历。
5. 不用策略详情页替代 Step 5 发布确认弹层。

## 展示内容契约

### 必须与 Step 5 保持一致

详情页 `策略配置` 详情层必须展示以下三组内容，字段顺序和文案应与 Step 5 `策略配置` tab 对齐：

| 组 | 字段 | 说明 |
|---|---|---|
| `指标过滤` | 分组、组内关系、条件表达式 | 对应 Step 1。无条件时展示 `暂无条件指标`。 |
| `权重得分` | 序号、加分值、评分表达式 | 对应 Step 2。无评分项时展示 `暂无评分项`。 |
| `建仓摘要` | 初始资金、每日候选、候选口径、调仓规则、最大持仓、单票上限、交易成本、风控 | 对应 Step 4。风控只列启用规则，未启用时展示 `未启用`。 |

其中 `候选口径` 和 `调仓规则` 不应省略。它们解释的是当前 live 组合的核心语义：

- `Top N 是每日候选信号，不是目标持仓集合`。
- `仅空位调入；旧持仓由风控退出`。

### 详情页上下文边界

详情页不是发布弹层，用户已经离开 Step 5 创建流程。source backtest 周期、source backtest 区间、业绩基准和 live 建仓日可以用于详情页业绩摘要、标题信息或其他非配置区域，但不进入 `策略配置` 详情层顶部。

| 字段 | 来源 |
|---|---|
| source backtest 周期 | `portfolioRecord.source_period_key`，由前端模板映射为展示文案 |
| source backtest 区间 | `portfolioRecord.source_start_date` / `portfolioRecord.source_end_date` |
| 业绩基准 | `portfolioRecord.benchmark_security_code`，由前端基准选项或代码模板映射为展示文案 |
| live 建仓日 | `portfolioRecord.live_start_date` |

`策略配置` 详情层只展示与 Step 5 `策略配置` tab 对齐的配置事实：`指标过滤`、`权重得分` 和 `建仓摘要`。类似 `example_v1 · 2023-12-29 - 2023-12-29 · 沪深300（000300.SH） · 建仓日 2024-01-02` 的来源文案不应出现在配置弹窗中。

## 推荐展示方案

### 入口位置

建议把 `策略配置方案` 按钮放在详情页页头，靠近策略名称和运行摘要，删除按钮之前：

```text
返回 / 策略名称 / 建仓 / 运行 / 持仓 / [icon 策略配置方案] / 删除

策略信号
策略业绩
净值走势
持仓记录
虚拟资金账户
账户对账单
```

理由：

1. 页头是策略身份和操作入口的自然位置，用户需要核对规则时能立刻找到。
2. 配置内容通常较长，放进主内容流会把 `策略信号` 和 `策略业绩` 推远，降低详情页的结果阅读效率。
3. `删除` 已经是页头动作，`配置` 作为只读信息入口放在同一区域比插入新页面区块更轻。
4. 桌面端通过 click 进入详情层，不设计 hover/focus 提示层。

触发按钮文本固定为 `策略配置方案`，左侧使用配置类图标。按钮采用无可见边框、普通字重的 ghost/text button 形态，避免在页头制造过强的主操作视觉。`!` 图标不建议用于普通策略配置，因为它在 UI 语义上更像警告或异常；除非配置存在缺失、过期或不一致，才应使用 warning icon。

### 交互形态

推荐采用单一显式交互：

1. 页头按钮：左侧配置类图标 + 固定文本 `策略配置方案`，无可见边框，文本普通字重。
2. 详情层：click 打开完整配置，优先使用 popover 或 dialog。若配置行数较多，dialog 比 hover popover 更稳定。

### 桌面效果草图

默认状态下，详情页主内容仍然直接从策略信号开始，配置入口只占用页头一个无边框图标文字按钮：

```text
+--------------------------------------------------------------------------------+
| <  低位反转策略  |  建仓: 2026-07-02  运行: 42 天  持仓: 10 只                 |
|                                                   [icon 策略配置方案] [ 删除 ] |
+--------------------------------------------------------------------------------+

  策略信号
  -------------------------------------------------------------------------------
  历史信号数                         股票             得分项                 得分
  [ signal count chart ]              中航高科 600862   KDJ 低位反转           82.4
                                      ...

  策略业绩
  -------------------------------------------------------------------------------
  业绩日期        2026-07-02          策略净值          1.0832
  业绩基准        沪深300             基准净值          1.0215
```

click 后打开完整配置详情层。第一版更建议用 dialog，而不是 hover popover；dialog 可以稳定承载长表达式、滚动和关闭动作。

```text
+--------------------------------------------------------------------------------+
| <  低位反转策略  |  建仓: 2026-07-02  运行: 42 天  持仓: 10 只                 |
|                                                   [icon 策略配置方案] [ 删除 ] |
+--------------------------------------------------------------------------------+
|                                                                                |
|                 +----------------------------------------------------------+   |
|                 | 策略配置                                      [ 关闭 x ] |   |
|                 |----------------------------------------------------------|   |
|                 | 指标过滤                                                |   |
|                 |   指标组 1     组内起始   close_price > price_ma_20     |   |
|                 |   指标组 1     AND        volume_ratio > 1.2             |   |
|                 |----------------------------------------------------------|   |
|                 | 权重得分                                                |   |
|                 |   #1           +30        turnover_rate between 2 and 8  |   |
|                 |   #2           +20        close_price > price_ma_60      |   |
|                 |----------------------------------------------------------|   |
|                 | 建仓摘要                                                |   |
|                 |   初始资金       ¥1,000,000                              |   |
|                 |   每日候选       Top 10                                  |   |
|                 |   候选口径       Top N 是每日候选信号，不是目标持仓集合  |   |
|                 |   调仓规则       仅空位调入；旧持仓由风控退出            |   |
|                 |   最大持仓       10 只                                   |   |
|                 |   单票上限       10%                                     |   |
|                 |   交易成本       佣金 0.010% / 滑点 0.100%               |   |
|                 |   风控           固定止损 8%，时间止损 20 天             |   |
|                 +----------------------------------------------------------+   |
|                                                                                |
+--------------------------------------------------------------------------------+
```

完整配置不应只放在 hover popover 中，也不需要额外 hover/focus 短提示。原因是：

1. 完整配置包含长表达式和滚动内容，hover 容易误触关闭。
2. 键盘用户需要可聚焦、可关闭、可滚动的详情层。
3. 用户可能需要对照配置和页面结果，点击打开的详情层比 hover 更稳定。
4. 一个明确的文字按钮已经足够表达入口含义，短提示会增加不必要浮层。

### 详情层布局

建议使用当前 Step 5 弹层同风格的 border-y 清单，而不是新增大卡片或嵌套卡片。

桌面端：

```text
策略配置
──────────────────────────────────────────────────────────────────────────────
指标过滤
  指标组 1        组内起始      close_price > price_ma_20
  指标组 1        AND           volume_ratio > 1.2

权重得分
  #1              +30           turnover_rate between 2 and 8
  #2              +20           close_price > price_ma_60

建仓摘要
  初始资金         ¥1,000,000
  每日候选         Top 10
  候选口径         Top N 是每日候选信号，不是目标持仓集合
  调仓规则         仅空位调入；旧持仓由风控退出
  最大持仓         10 只
  单票上限         10%
  交易成本         佣金 0.010% / 滑点 0.100%
  风控             固定止损 8%，时间止损 20 天
```

布局要求：

1. 三组内容纵向堆叠。
2. 标题 `策略配置` 和第一组 `指标过滤` 之间保留分割线。
3. 表达式允许换行，不强行单行截断。
4. 详情层使用可滚动 body，保留明确关闭动作。

### 长列表处理

策略条件和评分项可能很多。建议第一版在详情层内保持内容默认可见，并给规则清单设置合理最大高度，例如每组最多显示约 6 到 8 行高度，超出时区块内部滚动。详情层自身也要有 `max-height`，避免遮挡页面或溢出视口。

不建议把 `指标过滤`、`权重得分` 和 `建仓摘要` 再拆成多层折叠。入口已经是按需打开，详情层内部应直接展示核心内容。

## 不建议的方案

### Hover-only `!` 提示

`!` 或 `Info` icon hover 后弹出完整配置，看起来轻量，但不适合作为主方案。完整配置有长表达式、滚动和误触关闭问题；`!` 还会传达“配置异常”的警告语义，不适合正常状态。

### 把详情页拆成大 Tab

把详情页改成 `策略表现 / 策略配置` 两个大 tab 会打断当前详情页的顺序阅读，也会让配置和 live 数据互相隔离。用户真正需要的是在同一页面先看配置，再看信号和表现。

### 把配置插入页头下方主内容流

主内容流内联配置会占用首屏，尤其在条件和评分较多时会把 `策略信号`、`策略业绩` 和 `净值走势` 推远。详情页的主任务是结果阅读，完整配置应该是可发现但按需打开。

### 放到右侧 sticky aside

右侧 aside 对长条件表达式不友好，也会和当前详情页的净值、业绩、账户阅读节奏竞争。策略配置是宽文本内容，不适合长期占用窄栏。

### 从 live 持仓或调仓记录反推配置

live 持仓、信号和调仓记录是结果事实，不是配置事实。详情页展示配置必须来自 strategy portfolio record 的 `rule_snapshot` 和 `execution_config`，不能根据结果数据反推，也不能用手写展示 JSON 替代真实配置。

## 数据模型建议

后续实现建议引入一个前端共享的展示模型，例如：

```ts
type StrategyConfigDisplayModel = {
  version: 1
  conditionRows: {
    id: string
    groupLabel: string
    logicLabel: string
    expression: string
  }[]
  scoringRows: {
    id: string
    index: number
    score: number
    expression: string
  }[]
  buildSummaryRows: {
    label: string
    value: string
  }[]
}
```

Step 5 发布弹层和策略详情页都渲染这个模型，而不是各自维护一份 JSX 和行构造逻辑。

### 数据来源优先级

建议分两层处理：

1. Step 5 发布弹层可以继续从当前编辑态构造即时展示，但提交前必须能得到同一份 `RuleVersionSpec` 和 `BacktestExecutionConfig`。
2. 策略详情页只从 persisted portfolio 的 `rule_snapshot` 和 `execution_config` 读取真实配置，前端用同一个 canonical formatter 生成 `StrategyConfigDisplayModel`。
3. period label、benchmark label、preview 文案、配置行和空态文案都由前端模板生成，不保存到 `ui_display_snapshot`。
4. `ui_display_snapshot.strategy_config_snapshot` 和 `ui_display_snapshot.strategy_config_display` 都不作为新增 contract；如果历史数据中存在，详情页也不应把它们当成权威来源。

如果未来需要服务端直接返回 typed display model，也应由 Rearview 在 response 层从 `rule_snapshot`、`execution_config` 和受控 catalog 生成，而不是要求前端把展示快照写回后端。

### 历史组合降级

当前已经创建的组合可能没有可解析的 canonical 配置。第一版可以采用以下降级规则：

1. 如果 `rule_snapshot` 可被解析为 `RuleVersionSpec`，且 `execution_config` 可被解析为 `BacktestExecutionConfig`，则展示 canonical 配置。
2. 如果 canonical 字段缺失或结构非法，不做字段猜测，只显示 `策略配置暂不可展示`，配置弹窗内不展示 source backtest 周期、区间、基准和建仓日上下文。
3. 即使存在 `strategy_config_display` 或 `strategy_config_snapshot` 历史 JSON，也不能把它们当成详情页权威来源。

这里不要写 `snapshot.config || snapshot.strategy || snapshot.data` 之类的多路径兼容。缺少明确 contract 时应显式暴露缺口。

## 实施建议

建议后续计划按以下顺序执行：

1. 抽出 `StrategyConfigDisplaySection` 组件，先让 Step 5 发布弹层使用它，确保现有展示不变。
2. 抽出 `buildStrategyConfigDisplayFromCanonical()`，输入为 `RuleVersionSpec`、`BacktestExecutionConfig` 和前端 catalog/label 模板。
3. Step 5 发布弹层在已有编辑态上继续即时展示，同时用同一套 formatter 校验其提交出去的 canonical rule/execution config 能生成一致展示。
4. 策略详情页读取 `portfolioRecord.rule_snapshot` 和 `portfolioRecord.execution_config`，通过 type guard 后派生 `StrategyConfigDisplayModel`。
5. 清理 backtest create request 中为本需求新增或计划新增的 `strategy_config_snapshot`、`strategy_config_display`、period label、benchmark label 和 preview 展示字段写入。
6. 在详情页页头渲染左侧配置图标 + `策略配置方案` 的无边框按钮，click 打开 `StrategyConfigDisplaySection`。
7. 补充前端测试，覆盖 Step 5 和详情页渲染同一份 canonical 展示模型。
8. 用 0051 example 和真实 Step 创建组合做 browser smoke，证明不依赖手写 PostgreSQL JSON。

## 验收标准

1. Step 5 发布弹层和策略详情页使用同一个展示组件或同一个展示模型 contract，且详情页展示必须从 `rule_snapshot` + `execution_config` 派生。
2. 策略详情页页头提供左侧配置图标 + `策略配置方案` 的无边框按钮，click 后能展示 `指标过滤`、`权重得分` 和 `建仓摘要` 三组内容。
3. 三组内容的字段、文案和顺序与 Step 5 `策略配置` tab 一致。
4. 完整配置不占用详情页主内容流，`策略信号` 仍紧跟页头后的详情主内容。
5. 长表达式在桌面视口不溢出；详情层可滚动、可关闭、可通过键盘访问。
6. canonical 配置不可解析时显式展示缺失状态，不从未定义 JSON 路径猜测字段；display-only JSON 不能通过验收。
7. `业绩基准` 不再在策略详情页硬编码为 `沪深300`，应从 portfolio/source context 派生。
8. 策略配置弹窗标题下不展示 source backtest 周期、区间、基准和建仓日拼接文案。

## 待决问题

1. `pool_filters` 的逻辑树在 canonical 展示中如何分组命名，是否统一为 `条件组 1` / `条件组 2`，还是按表达式层级展示？
2. metric 中文标签使用当前前端 catalog 模板即可，还是未来需要 Rearview 返回与 `catalog_hash` 对应的历史 catalog label？
3. 是否需要在详情页配置区块提供 `复制配置` 或 `从该策略新建回测` 的后续动作？本 RFC 第一版不包含这些动作。

## 最小验证

文档阶段最小验证：

```bash
make docs-check
git diff --check
```

实现阶段最小验证：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

若实现改动 Rearview API 或 Rust 类型，追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
