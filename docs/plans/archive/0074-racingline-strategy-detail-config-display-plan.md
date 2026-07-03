# Plan 0074: Racingline 策略详情页策略配置方案实施计划

日期：2026-07-02

状态：Completed

领域：Racingline, Rearview, Strategy Portfolio, Frontend UX

关联系统：

- `app/racingline/`
- `engines/crates/rearview-core/`

关联文档：

- [RFC 0046: Racingline 策略详情页策略配置展示](../../RFC/archive/0046-racingline-strategy-detail-config-display.md)
- [RFC 0034: Racingline Step 5 建立策略组合弹层分 Tab 信息架构](../../RFC/archive/0034-racingline-step5-portfolio-publish-dialog-tabs.md)
- [Racingline Architecture](../../architecture/racingline.md)
- [Rearview Architecture](../../architecture/rearview.md)

## 背景

RFC 0046 已确认：策略详情页需要提供 `策略配置方案` 入口，用于展示与 Step 5「建立组合」弹层 `策略配置` tab 一致的内容，但不应把完整配置插入详情页主内容流，也不应通过 hover/focus 提示承载完整配置。

目标交互是桌面工作台方案：

```text
<  低位反转策略  |  建仓: 2026-07-02  运行: 42 天  持仓: 10 只  [ 策略配置方案 ] [ 删除 ]
```

点击 `策略配置方案` 后打开完整配置详情层，展示三组内容：

1. `指标过滤`
2. `权重得分`
3. `建仓摘要`

当前阻塞点不是 UI 按钮本身，而是 Step 5 的可读配置行来自前端编辑态，发布后的 `StrategyPortfolioRecord.ui_display_snapshot` 还没有稳定保存完整展示模型。实施必须先收敛展示模型，再接入详情页，避免详情页从未定义 JSON 路径猜字段。

## 目标

1. 抽出可复用 `StrategyConfigDisplayModel` 和 `StrategyConfigDisplaySection`，供 Step 5 发布弹层和策略详情页共用。
2. Step 5 创建 backtest run 时，把三组配置展示行写入 `ui_display_snapshot.strategy_config_display`。
3. 发布 portfolio 时继续复用 Rearview 现有行为：从 source backtest run 复制 `ui_display_snapshot` 到 strategy portfolio。
4. 策略详情页页头新增 `策略配置方案` 按钮，位置在运行摘要之后、删除按钮之前。
5. 点击按钮打开配置详情层，展示与 Step 5 `策略配置` tab 一致的 `指标过滤`、`权重得分` 和 `建仓摘要`。
6. 对缺失 `strategy_config_display` 的历史组合显式展示缺失状态，不做多路径 JSON fallback。
7. 删除详情页 `业绩基准` 的硬编码 `沪深300`，改为从 portfolio/source context 派生。

## 非目标

1. 不实现移动端布局或 bottom sheet。
2. 不设计 hover/focus tooltip 或 hover-only 完整配置展示。
3. 不把完整配置插入详情页页头下方主内容流。
4. 不把详情页拆成 `策略表现 / 策略配置` 大 Tab。
5. 不修改策略规则、回测计算、portfolio daily run、live facts 或绩效口径。
6. 不新增 PostgreSQL column 或 Alembic migration；第一版继续使用现有 `ui_display_snapshot` JSONB。
7. 不为历史组合做数据库 backfill；历史组合第一版允许显示配置快照缺失。
8. 不在详情页从 live 持仓、调仓记录或信号结果反推配置。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| Step 5 发布弹层 | [strategy-page.tsx](../../../app/racingline/src/routes/strategy-page.tsx) 中 `策略配置` tab 已展示 `指标过滤`、`权重得分`、`建仓摘要`。 |
| Step 5 展示行来源 | `publishConditionRows` 来自 `conditionGroups`，`publishScoringRows` 来自 `weightIndicators`，`publishBuildSummaryRows` 来自 `effectiveSimulationSettings` 和 `backtestExecutionDraft.summary`。 |
| 表达式 formatter | [utils.ts](../../../app/racingline/src/features/strategy/utils.ts) 已有 `formatComparableIndicator()` 和 `formatWeightIndicator()`。 |
| Backtest create snapshot | `buildStrategyBacktestCreateRequest()` 已写入 `ui_display_snapshot`，但当前只包含 `benchmark`、`period`、`preview` 和 `simulation` 摘要。 |
| Portfolio create | Rearview `create_strategy_portfolio()` 从 source backtest run 复制 `ui_display_snapshot` 到 `strategy_portfolio`。 |
| 策略详情页 | [strategy-detail-page.tsx](../../../app/racingline/src/routes/strategy-detail-page.tsx) 已读取 `StrategyPortfolioRecord`，但没有展示 Step 1/2/4 配置。 |
| 策略详情页按钮区 | 页头已有返回、策略名称、建仓/运行/持仓摘要和删除 Dialog，可在删除按钮之前新增只读配置入口。 |
| 前端测试 | 现有测试包括 `strategy-detail-page.test.tsx`、`features/strategy/execution.test.ts`、`features/strategy/utils.test.ts`、`features/strategy/adapters.test.ts`。 |

## 设计约束

1. 按桌面工作台实现，不做移动端专项适配。
2. `策略配置方案` 是明确文字按钮，不依赖图标表达含义，不设计 hover/focus 提示层。
3. 完整配置通过 click 打开详情层；第一版优先使用 existing `Dialog` 组件。
4. 展示内容必须与 Step 5 `策略配置` tab 的字段、顺序和文案一致。
5. `strategy_config_display` 必须是 versioned shape；详情页只能消费通过 type guard 的数据。
6. 缺失或 shape 不匹配时显示缺失状态，不从 `ui_display_snapshot` 的其他候选路径猜测字段。
7. Step 5 发布弹层和策略详情页不维护两套 JSX 清单结构。
8. 不新增 Rearview typed API 字段；除非实现中发现现有 `ui_display_snapshot` 无法满足 contract，再另开 RFC/plan。

## 实施阶段

### Phase 0: Characterization And Model Shape

目标：先定义展示模型和当前行为测试边界，避免后续 UI 重构改变 Step 5 文案或顺序。

实施项：

1. 新增 `app/racingline/src/features/strategy/config-display.ts`。
2. 定义 `StrategyConfigDisplayModel`、`StrategyConfigConditionRow`、`StrategyConfigScoringRow` 和 `StrategyConfigSummaryRow`。
3. 定义 `strategy_config_display.version = 1` 的 JSON shape 和 type guard。
4. 将 RFC 0046 中的三组字段顺序写成测试 fixture：
   - `指标过滤`
   - `权重得分`
   - `建仓摘要`
5. 给 type guard 添加单测，覆盖：
   - valid v1 shape
   - missing version
   - wrong row shape
   - unrelated JSON object

测试策略：

```bash
cd app/racingline
npm test -- src/features/strategy/config-display.test.ts
```

完成标准：

1. 展示模型有明确类型和 runtime guard。
2. 没有任何 `snapshot.config || snapshot.strategy || snapshot.data` 式 fallback。

### Phase 1: Extract Shared Display Builder And Section

目标：把 Step 5 发布弹层当前散落的行构造逻辑收敛为共享 builder 和共享展示组件。

实施项：

1. 从 `strategy-page.tsx` 抽出 builder：
   - `buildStrategyConfigDisplayFromDraft()`
   - 输入为 `conditionGroups`、`weightIndicators`、`strategyCatalogOptions`、`strategyScoringCatalog`、`effectiveSimulationSettings`、`backtestExecutionDraft.summary`。
   - 输出 `StrategyConfigDisplayModel`。
2. 复用现有 `formatComparableIndicator()` 和 `formatWeightIndicator()`，不重写表达式格式化逻辑。
3. 新增 `StrategyConfigDisplaySection` 组件：
   - 渲染 `指标过滤`
   - 渲染 `权重得分`
   - 渲染 `建仓摘要`
   - 使用 Step 5 现有 border-y 清单视觉风格。
4. 替换 Step 5 发布弹层 `策略配置` tab 内部 JSX，使其消费 `StrategyConfigDisplaySection`。
5. 保持 Step 5 当前可见文案和顺序不变。

测试策略：

1. Builder 单测覆盖条件行、评分行和建仓摘要行生成。
2. 如果现有 route 测试不覆盖 Step 5 发布弹层，新增轻量组件测试覆盖 `StrategyConfigDisplaySection` 的三段标题和典型行。

完成标准：

1. Step 5 `策略配置` tab 不再维护独立三段 JSX。
2. 共享 builder 输出与当前 Step 5 展示内容一致。

### Phase 2: Persist Display Snapshot Through Backtest And Portfolio Publish

目标：让新发布组合的详情页能从 persisted snapshot 读取完整配置展示模型。

实施项：

1. 修改 `buildStrategyBacktestCreateRequest()` 的 `ui_display_snapshot`：
   - 保留当前 `benchmark`、`period`、`preview`、`simulation` 字段。
   - 新增 `strategy_config_display`。
2. `strategy_config_display` 写入 Phase 1 builder 的输出，且包含 `version: 1`。
3. 在 `features/strategy/execution.test.ts` 或新增测试中断言 create request 包含完整 display snapshot。
4. 确认 Rearview create portfolio 现有实现继续复制 `source_run.ui_display_snapshot`，不需要后端改动。
5. 不修改 Alembic schema；继续使用 `ui_display_snapshot` JSONB。

测试策略：

```bash
cd app/racingline
npm test -- src/features/strategy/execution.test.ts src/features/strategy/config-display.test.ts
```

完成标准：

1. 新 Step 5 backtest run 的 `ui_display_snapshot.strategy_config_display.version` 为 `1`。
2. 发布后的 `StrategyPortfolioRecord.ui_display_snapshot` 可以携带同一份展示模型。
3. 没有后端 migration。

### Phase 3: Add Strategy Detail Header Button And Dialog

目标：在策略详情页接入 `策略配置方案` 按钮和完整配置详情层。

实施项：

1. 在 `strategy-detail-page.tsx` 新增 state：`configDialogOpen`。
2. 在页头运行摘要之后、删除按钮之前新增 Button：
   - 文本固定为 `策略配置方案`。
   - 不加 hover/focus tooltip。
   - 不使用 `!` 图标。
3. 点击按钮打开 Dialog。
4. Dialog header 使用 `策略配置` 标题，并展示 source context：
   - source period label 或 `source_period_key`
   - `source_start_date` - `source_end_date`
   - benchmark label 或 `benchmark_security_code`
   - `live_start_date`
5. Dialog body 渲染 `StrategyConfigDisplaySection`。
6. 如果 `strategy_config_display` 缺失或 type guard 失败，显示 `策略配置展示快照缺失` 空态，并保留 source context。
7. 将详情页 `业绩基准` 从硬编码 `沪深300` 改为 source context 派生展示。

测试策略：

1. 更新 `strategy-detail-page.test.tsx`：
   - 有 valid display snapshot 时，点击 `策略配置方案` 后看到 `指标过滤`、`权重得分`、`建仓摘要`。
   - 缺失 display snapshot 时，点击后看到 `策略配置展示快照缺失`。
   - 页头不出现 hover tooltip 相关 DOM。
   - `业绩基准` 使用 portfolio/source context，不再硬编码。
2. 继续覆盖删除按钮逻辑，避免新增按钮影响 delete dialog。

完成标准：

1. 详情页默认主内容仍从 `策略信号` 开始。
2. `策略配置方案` 按钮可打开、关闭配置 Dialog。
3. 配置 Dialog 内容与 Step 5 共享组件一致。

### Phase 4: Quality Gate And Browser Smoke

目标：完成前端质量门禁，并用桌面浏览器确认交互形态符合 RFC。

实施项：

1. 运行前端 lint、typecheck、test、build。
2. 启动或复用 Racingline dev 环境。
3. 使用 CDP/Playwright 在桌面视口打开一个策略详情页。
4. 截图确认：
   - 页头按钮文本为 `策略配置方案`。
   - hover/focus 不出现短提示层。
   - 点击后出现配置 Dialog。
   - Dialog 中有三组内容，长表达式不溢出。
   - 主页面默认不插入完整配置区块。

验证命令：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

浏览器 smoke 可使用：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
```

完成标准：

1. 前端质量门禁全部通过。
2. 桌面浏览器 smoke 截图或记录证明 `策略配置方案` 按钮和 Dialog 工作。
3. 如实现需要变更 Rearview Rust API，追加 `cargo fmt --check`、`cargo clippy` 和 `cargo test`；按当前计划预计不需要。

## 禁止模式

1. 禁止 hover-only 完整配置。
2. 禁止 hover/focus 短提示层。
3. 禁止使用 `!` 作为正常配置入口。
4. 禁止把完整配置内联插到页头下方主内容流。
5. 禁止从 live 持仓、调仓、信号或绩效结果反推配置。
6. 禁止对 `ui_display_snapshot` 写多路径猜测 fallback。
7. 禁止维护 Step 5 和详情页两套配置展示 JSX。

## 最小验证

文档阶段：

```bash
make docs-check
git diff --check
```

实施阶段：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

如果实现修改 Rearview Rust 代码：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## 完成标准

1. `docs/plans/README.md` active plan 指向本计划。
2. Step 5 发布弹层和策略详情页共用配置展示模型或组件。
3. 新发布组合的 `ui_display_snapshot` 持久化 `strategy_config_display.version = 1`。
4. 策略详情页页头展示 `策略配置方案` 按钮。
5. 点击按钮打开配置 Dialog，并展示 `指标过滤`、`权重得分` 和 `建仓摘要`。
6. 缺失展示快照时显式展示缺失状态。
7. 前端质量门禁和桌面浏览器 smoke 通过。
8. 完成后将本计划移入 `docs/plans/archive/`，并新增运行报告到 `docs/jobs/reports/`。
