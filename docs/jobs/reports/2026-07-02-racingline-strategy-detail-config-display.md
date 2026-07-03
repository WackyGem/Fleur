# Racingline Strategy Detail Config Display Run Report

日期：2026-07-02

范围：

- `app/racingline/src/features/strategy/config-display.ts`
- `app/racingline/src/features/strategy/components/strategy-config-display-section.tsx`
- `app/racingline/src/features/strategy/execution.ts`
- `app/racingline/src/routes/strategy-page.tsx`
- `app/racingline/src/routes/strategy-detail-page.tsx`
- `docs/RFC/archive/0046-racingline-strategy-detail-config-display.md`
- `docs/plans/archive/0074-racingline-strategy-detail-config-display-plan.md`

## 变更摘要

以下记录保留 0074 第一轮实现事实；其中 `strategy_config_snapshot` / `strategy_config_display` 写入后端的方向已被后续 RFC/Plan 修订否定，不能作为最终设计。

1. 抽出 `StrategyConfigDisplayModel`、v1 `strategy_config_snapshot` / `strategy_config_display` 序列化和读取 guard、source context 派生函数。
2. 抽出 `StrategyConfigDisplaySection`，由 Step 5 发布弹层和策略详情页共用。
3. Step 5 创建 backtest run 时曾计划在 `ui_display_snapshot.strategy_config_snapshot` 写入真实配置快照，并从该 snapshot 派生 `strategy_config_display`；该方向后续不采纳。
4. 策略详情页页头新增左侧配置图标 + `策略配置方案` 的普通字重无边框按钮，位于运行摘要之后、删除按钮之前；click 打开 `Dialog`，标题下用分割线隔开 `指标过滤`、`权重得分`、`建仓摘要`。
5. 缺失、malformed 或 display-only snapshot 时展示 `策略配置暂不可展示`，不做多路径 JSON fallback。
6. 策略详情页 `业绩基准` 改为从 portfolio/source context 派生，不再硬编码 `沪深300`。

## 验证命令

```bash
cd app/racingline
npm test -- src/features/strategy/config-display.test.ts src/features/strategy/execution.test.ts src/routes/strategy-detail-page.test.tsx
npm run typecheck
npm run lint
npm test
npm run build
```

结果：

- 定向测试：3 files passed, 23 tests passed。
- Typecheck：通过。
- Lint：通过。
- 全量测试：11 files passed, 77 tests passed。
- Build：通过；Vite 仍提示现有 chunk size warning。

## 浏览器 Smoke

纠偏说明：本节最初记录的 smoke 使用本地 dev 示例 portfolio，并临时在 PostgreSQL 中注入手写 `strategy_config_display`。该方式不能证明详情页展示的是组合创建时实际使用的真实配置，因此不再作为完整验收依据。后续完整 browser smoke 必须按 Plan 0075 使用真实 Step 创建路径或受控 example fixture，并证明 Dialog 由 `rule_snapshot` + `execution_config` 派生，不依赖 `ui_display_snapshot` 展示字段。

命令：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

环境：

- Racingline: `http://127.0.0.1:5173/`
- Rearview: `http://127.0.0.1:34057`
- CDP: `http://127.0.0.1:9222`
- Smoke detail URL: `http://127.0.0.1:5173/dashboard/strategies/01KWGR9KZX1H8K0MY8Y7J3EXHH`

操作记录：

1. 使用本地 dev 示例 portfolio `01KWGR9KZX1H8K0MY8Y7J3EXHH`。
2. 该示例 portfolio 原始 `ui_display_snapshot` 没有策略配置展示字段；这不应被视为数据缺失，因为详情页最终设计应从 `rule_snapshot` + `execution_config` 派生展示。临时 DB 注入只暴露了验收偏移，不能作为通过依据。
3. CDP 桌面视口 `1440x900` 校验：
   - 页头存在文本完全为 `策略配置方案` 的按钮。
   - 按钮没有 `title`、`aria-describedby` 或 tooltip 节点。
   - hover 后没有短提示或 tooltip。
   - 主页面默认仍从 `策略信号` 开始，没有内联完整配置区块。
   - click 后打开 `策略配置` Dialog。
   - Dialog 展示 `指标过滤`、`权重得分`、`建仓摘要`。
   - Dialog 可关闭，关闭后 `[role=dialog]` 不存在。

结论：入口和 Dialog 交互形态已验证；真实配置来源未通过 browser smoke 验收，转入 Plan 0075。

## 2026-07-02 纠偏再修订

先前补充曾把纠偏方向写成“保存真实配置快照到 `ui_display_snapshot`”。该方向仍然会造成展示状态双写，不再采纳。当前 RFC 0046 和 Plan 0075 已修订为 canonical 派生展示：

1. 策略详情页从 `portfolioRecord.rule_snapshot` + `portfolioRecord.execution_config` 派生展示模型。
2. period label、benchmark label、preview 文案和配置展示行由前端模板生成，不写入后端。
3. `ui_display_snapshot.strategy_config_snapshot` 和 `ui_display_snapshot.strategy_config_display` 不作为新增 contract。
4. 0051 example 应直接使用已有 canonical 配置展示，不补 mock snapshot。
5. Plan 0075 继续处理代码清理、canonical formatter、0051 example 和真实 browser smoke。

先前补充验证：

```bash
cd app/racingline
npm test -- src/features/strategy/config-display.test.ts src/features/strategy/execution.test.ts src/routes/strategy-detail-page.test.tsx
npm run typecheck
```

结果：

- 定向测试：3 files passed, 25 tests passed。
- Typecheck：通过。

说明：上述验证对应已被修订的 snapshot 方向，不能作为 canonical 派生展示的最终验收。

## 2026-07-02 0075 canonical 纠偏验收

本次最终实现已按 Plan 0075 改为 canonical 派生展示：

1. `config-display.ts` 新增 `buildStrategyConfigDisplayFromCanonical()` 和 `readRuleVersionSpec()`，从 `RuleVersionSpec.pool_filters`、`RuleVersionSpec.scoring.rules` 和 `BacktestExecutionConfig` 生成详情页展示模型。
2. `buildStrategyBacktestCreateRequest()` 不再写入 `ui_display_snapshot.strategy_config_snapshot`、`ui_display_snapshot.strategy_config_display`、period label、benchmark label 或 preview 文案。
3. 策略详情页从 `portfolioRecord.rule_snapshot` + `portfolioRecord.execution_config` 派生 Dialog 内容；`ui_display_snapshot` 中的 display-only JSON 被忽略。
4. period/benchmark/source context 由前端模板和 portfolio 结构字段生成。

前端验证：

```bash
cd app/racingline
npm test -- src/features/strategy/config-display.test.ts src/features/strategy/execution.test.ts src/routes/strategy-detail-page.test.tsx
npm run typecheck
npm run lint
npm test
npm run build
```

结果：

- 定向测试：3 files passed, 24 tests passed。
- Typecheck：通过。
- Lint：通过。
- 全量测试：11 files passed, 78 tests passed。
- Build：通过；Vite 仍提示既有 chunk size warning。

Browser smoke：

```bash
node scripts/check_playwright_cdp.mjs
curl -sS -X POST http://127.0.0.1:34057/rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
playwright-cli --s=default resize 1440 900
playwright-cli --s=default reload
playwright-cli --s=default click <策略配置方案>
```

环境：

- Racingline: `http://127.0.0.1:5173/`
- Rearview: `http://127.0.0.1:34057`
- CDP: `http://127.0.0.1:9222`
- Smoke detail URL: `http://127.0.0.1:5173/dashboard/strategies/01KWGR9KZX1H8K0MY8Y7J3EXHH`
- Portfolio: `Racingline 0051 Low Reversal Example`
- Source: Rearview example ensure API，`created=false`

Browser 结论：

1. 页头存在文本完全为 `策略配置方案` 的按钮，按钮左侧有配置图标、普通字重且无可见边框。
2. 按钮无 `title` / `aria-describedby`，hover 后 `[role=tooltip] = false`。
3. 主页面默认仍从 `策略信号` 开始，没有内联完整配置区块。
4. click 后打开 `策略配置` Dialog。
5. Dialog 标题下有分割线，不展示 source backtest 周期、区间、基准和建仓日拼接文案。
6. Dialog 展示来自 canonical 0051 配置的 `KDJ J < 13`、`振幅 < 4`、`涨跌幅`、`均线`、`成交量`、7 条评分项、`Top 5`、`最大持仓 5 只`、`单票上限 20%`、`止盈 15%，指标止损 MA10`。
7. 本地 dev DB 中该 portfolio 残留旧 `ui_display_snapshot.strategy_config_display`，但 Dialog 未展示旧手写 `volume_ratio` 或 `turnover_rate between 2 and 8` 内容，证明 display-only JSON 不再作为详情页配置来源。

## 文档门禁

文档归档、索引和报告创建后运行：

```bash
make docs-check
git diff --check
```

结果：通过。
