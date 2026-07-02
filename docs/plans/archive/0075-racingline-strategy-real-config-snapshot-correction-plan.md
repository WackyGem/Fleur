# Plan 0075: Racingline 策略详情页 canonical 配置展示纠偏计划

日期：2026-07-02

状态：Completed

领域：Racingline, Rearview, Strategy Portfolio, Frontend UX

关联系统：

- `app/racingline/`
- `engines/crates/rearview-core/`

关联文档：

- [RFC 0046: Racingline 策略详情页策略配置展示](../../RFC/0046-racingline-strategy-detail-config-display.md)
- [Plan 0074: Racingline 策略详情页策略配置方案实施计划](0074-racingline-strategy-detail-config-display-plan.md)
- [2026-07-02 Racingline Strategy Detail Config Display Run Report](../../jobs/reports/2026-07-02-racingline-strategy-detail-config-display.md)
- [Racingline Architecture](../../architecture/racingline.md)
- [Rearview Architecture](../../architecture/rearview.md)

## 背景

0074 已完成页头入口、Dialog 和共享展示组件，但实施和验收存在偏移：

1. 0074 第一版允许详情页读取 display-only JSON，导致手写展示行可能被误认为真实策略配置。
2. 浏览器 smoke 使用本地 dev PostgreSQL 临时注入手写 display snapshot 验证 Dialog，这是无效验收；验收必须来自真实 Step 流程或受控 example fixture。
3. 后续曾考虑把 `strategy_config_snapshot`、`strategy_config_display`、period label、benchmark label 和 preview 展示字段写入 `ui_display_snapshot`，但这仍然是双写展示状态，容易和真实配置漂移。
4. `Racingline 0051 Low Reversal Example` 没有前端 Step 1-Step 5 的展示快照，但它已经具备真实 `rule_snapshot` 和 `execution_config`。

纠偏原则：策略详情页展示的配置必须由组合真实执行的 canonical 配置派生。`rule_snapshot` 和 `execution_config` 是权威来源；展示模板、标签、period 文案、benchmark 文案和 preview 文案都由前端统一生成，不保存到后端。

## 目标

1. 策略详情页从 `portfolioRecord.rule_snapshot` + `portfolioRecord.execution_config` 构造 `StrategyConfigDisplayModel`。
2. Step 5 发布弹层和策略详情页复用同一个展示组件，并尽量复用同一个 canonical formatter。
3. 不新增、不依赖 `ui_display_snapshot.strategy_config_snapshot` 或 `ui_display_snapshot.strategy_config_display`。
4. backtest create request 不为本需求写入 period label、benchmark label、preview 文案或配置展示字段。
5. 0051 example portfolio 通过已有 `rule_snapshot` + `execution_config` 自动获得可展示配置；不得用 mock JSON 或手改 PostgreSQL 作为验收。
6. canonical 配置不可解析时显式展示缺失状态，不做多路径 JSON fallback。

## 非目标

1. 不在浏览器内反推持仓、净值或绩效。
2. 不从 live 信号、调仓、持仓结果反推出策略配置。
3. 不把 `ui_display_snapshot` 中的 display-only JSON 当成真实配置来源。
4. 不为历史组合编造配置；没有可解析 canonical 配置时必须显示缺失。
5. 不新增 `sliders` 文案、hover/focus tooltip 或 `!` 图标入口。
6. 不在本计划中移除 Rearview/PostgreSQL 现有 `ui_display_snapshot` 字段；本计划只停止为策略配置展示扩展和依赖它。

## 当前事实基线

| 区域 | 事实 |
|---|---|
| 0074 UI | 详情页已有 `策略配置方案` 按钮和 Dialog。 |
| 0074 display | Step 5 和详情页共用 `StrategyConfigDisplaySection`。 |
| 当前 canonical 字段 | `StrategyPortfolioRecord` 已包含 `rule_snapshot`、`execution_config`、source period/range、benchmark code 和 live start date。 |
| 当前偏移代码 | 中断实现里已出现 `strategy_config_snapshot` v1、序列化、guard 和写入 request 的草稿；这些应被移除或改写为 canonical formatter。 |
| 0051 example | Rearview example ensure 直接写入 `rule_snapshot` 和 `execution_config`，但没有前端展示快照。canonical formatter 能覆盖它。 |
| 无效验收 | 2026-07-02 report 中记录的本地 PostgreSQL 注入 smoke 不能作为完整验收依据。 |

## 实施阶段

### Phase 1: Canonical Display Formatter

目标：建立唯一展示构造入口。

实施项：

1. 定义 `buildStrategyConfigDisplayFromCanonical()`。
2. 输入只允许：
   - `RuleVersionSpec`
   - `BacktestExecutionConfig`
   - source context：period key、source start/end date、benchmark code、live start date
   - 前端 metric catalog / label 模板
3. 从 `rule_snapshot.pool_filters` 生成 `指标过滤` 行。
4. 从 `rule_snapshot.scoring.rules` 生成 `权重得分` 行。
5. 从 `execution_config` 生成 `建仓摘要` 行。
6. 对不支持或结构非法的 canonical config 返回显式缺失状态，不做 `ui_display_snapshot` fallback。

完成标准：

1. `config-display.test.ts` 覆盖 `RuleVersionSpec + BacktestExecutionConfig` 到展示模型的转换。
2. 测试覆盖 conditional points、weighted metric、空过滤、空评分、启用/未启用风控。
3. display-only JSON 无法驱动详情页展示。

### Phase 2: Request And Snapshot Cleanup

目标：停止把展示状态写入后端。

实施项：

1. 移除 backtest create request 中为策略配置展示新增的：
   - `strategy_config_snapshot`
   - `strategy_config_display`
   - period label
   - benchmark label
   - preview 展示文案
2. 如果短期仍需传 `ui_display_snapshot` 兼容既有 API，则只允许传空对象或已存在且另有事实用途的字段；不得为策略配置展示新增字段。
3. 删除中断实现中 `StrategyConfigSnapshotModel`、snapshot guard、snapshot serializer 相关代码，或改名/改写为纯前端 canonical display model。

完成标准：

1. `execution.test.ts` 断言 create request 不包含策略配置展示快照。
2. 新增/更新测试确保 period、benchmark 和 preview 展示文案由前端 source context 模板生成。

### Phase 3: Detail Page Wiring

目标：详情页只用 portfolio canonical 字段展示配置。

实施项：

1. 策略详情页读取 `portfolioRecord.rule_snapshot` 和 `portfolioRecord.execution_config`。
2. 通过 runtime guard 验证 `rule_snapshot` 是 `RuleVersionSpec`。
3. 使用 `buildStrategyConfigDisplayFromCanonical()` 生成 Dialog 内容。
4. 0051 example 不补 mock snapshot，直接验证 canonical formatter 输出。
5. canonical 字段不可解析时展示 `策略配置暂不可展示`。

完成标准：

1. `strategy-detail-page.test.tsx` 覆盖 valid canonical config 可展示。
2. `strategy-detail-page.test.tsx` 覆盖 `ui_display_snapshot` 中存在 display-only JSON 但 canonical 字段非法时仍显示缺失。
3. 0051 fixture 或等价 fixture 能展示 `指标过滤`、`权重得分` 和 `建仓摘要`。

### Phase 4: Real Browser Smoke

目标：重新验收 UI 行为，但数据必须来自真实创建路径。

实施项：

1. 通过 `/strategies` Step 1-Step 5 创建一个 dev portfolio，或通过 0051 example ensure API 创建 portfolio。
2. 使用 CDP 桌面视口验证：
   - `策略配置方案` 按钮存在。
   - hover/focus 不出现短提示。
   - click 打开 Dialog。
   - Dialog 内容由 `rule_snapshot` + `execution_config` 派生。
   - 不依赖 `ui_display_snapshot`、mock JSON 或手写 PostgreSQL 更新。

完成标准：

1. Browser smoke 不依赖手改 PostgreSQL。
2. 运行报告明确记录 portfolio id、创建路径和 canonical 配置来源。

## 最小验证

```bash
cd app/racingline
npm test -- src/features/strategy/config-display.test.ts src/features/strategy/execution.test.ts src/routes/strategy-detail-page.test.tsx
npm run typecheck
npm run lint
npm test
npm run build
```

文档检查：

```bash
make docs-check
git diff --check
```

如果修改 Rearview Rust 代码，追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 完成标准

1. 新发布组合和 0051 example 的详情页展示均来自 `rule_snapshot` + `execution_config`。
2. `ui_display_snapshot` 不再作为策略配置展示来源，也不新增 period/benchmark/preview 展示字段。
3. 0051 example 或其他非 UI 创建组合不使用假数据展示。
4. Browser smoke 使用真实创建路径完成。
5. 更新 RFC 0046、Racingline architecture 和运行报告，明确 0074 的验收偏移及 0075 的 canonical 纠偏结果。

## 验收结果

完成日期：2026-07-02

1. Frontend 定向测试通过：`npm test -- src/features/strategy/config-display.test.ts src/features/strategy/execution.test.ts src/routes/strategy-detail-page.test.tsx`，3 files / 24 tests passed。
2. Frontend 全量门禁通过：`npm run typecheck`、`npm run lint`、`npm test`、`npm run build`。Build 仅保留既有 Vite chunk size warning。
3. Browser smoke 使用 0051 example portfolio `01KWGR9KZX1H8K0MY8Y7J3EXHH`，确认 Dialog 从 canonical `rule_snapshot` + `execution_config` 展示 KDJ、涨跌幅、均线、成交量、评分和建仓摘要；本地 DB 中残留的旧 `ui_display_snapshot.strategy_config_display` 没有进入 Dialog。
4. `策略配置方案` 按钮无 `title` / `aria-describedby`，hover 不出现 `[role=tooltip]`。
