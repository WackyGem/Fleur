# Plan 0046: Racingline 策略权重配置 Step 2 实施计划

日期：2026-06-22

状态：Completed

关联文档：

- [RFC 0025: Racingline 策略权重配置 Step 2 实现方案](../../RFC/0025-racingline-strategy-weight-configuration-step2.md)
- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](../../RFC/0024-racingline-strategy-selection-step1.md)
- [Plan 0045: Racingline 策略选股 Step 1 缺口填补实施计划](0045-racingline-strategy-selection-step1-gap-closure-plan.md)
- [Racingline Strategy Step 1 Gap Closure 报告](../../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md)
- [Racingline Strategy Step 2 Preview Implementation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md)
- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)

## 目标

1. 将 `/strategies` Step 2 权重配置从本地 mock 推进为真实 `RuleVersionSpec.scoring.rules` 草稿。
2. 保持业务边界：Step 1 只记录股票池筛选条件，Step 2 只记录候选池内评分规则，点击「股池预览」时才执行选股、评分和排名。
3. 让前后端评分口径统一为 `[0, 100]`，包括前端缩放、`RuleVersionSpec.scoring.clamp`、Rearview validation 和 planner 输出。
4. 新增 preview-only API，执行完整 `RuleVersionSpec` 并返回按交易日组织的股票池、score、rank、score breakdown 和 selected metrics。
5. 用真实 Rearview metric catalog 驱动 Step 2 scoring 指标选择，只允许 `allow_scoring = true` 的指标进入评分规则。
6. 让 Step 3 股池预览页面消费真实 preview response，不再把 mock 股池当作成功路径。

## 非目标

1. 不发布 rule set 或 rule version，不调用正式 rule version 创建 API。
2. 不创建正式 `run`，不污染 Rearview run 状态机、PostgreSQL run snapshot 或 portfolio run 链路。
3. 不实现模拟建仓、回测、运行策略或看板回写。
4. 不在浏览器内计算权威股票池、score、rank、成交、持仓或净值。
5. 不引入 `weighted_metric` 的原始指标线性加权；第一版只使用 `conditional_points`。
6. 不新增指标归一化、方向、行业中性化、z-score、rank 或 winsorize 逻辑。
7. 不把 `app/racingline_new/src/features/strategy/catalog.ts` 当作真实 scoring catalog。
8. 不把 `app/racingline_new` 升级为正式工程；正式迁移仍需另起计划并遵守 ADR 0011/0013。
9. 不调整 Step 1/Step 2/Step 3 的前端可见文案；本计划只改变规则构造、预览执行和状态接入。

## 当前事实基线

### 已有能力

| 领域 | 当前事实 |
|---|---|
| Step 1 adapter | `app/racingline_new/src/features/strategy/adapters.ts` 已能把 `StrategyConditionGroup[]` 转成 `RuleVersionSpec.pool_filters`，并支持组内 AND/OR 混排、指标比较、between、is_null 和 crossing。 |
| Step 1 catalog | `buildStrategyMetricCatalog()` 已从 `GET /rearview/metrics` 生成中文指标分组和 filter 指标选项。 |
| Step 1 explain | `/strategies` 已能调用 `POST /rearview/explain` 做编译校验，并展示 required metrics/marts/columns。 |
| Step 2 UI | `WeightIndicatorsPanel` 已有权重项增删、比较字段、slider/input 和缩放得分摘要。 |
| 前端缩放 | `clampScore()` 和 `clampWeightTotal()` 已按 0-100 区间处理；`getScaledWeightIndicators()` 已能把总分缩放到 100。 |
| Rearview scoring DSL | `RuleVersionSpec.scoring` 支持 `conditional_points` 和 `weighted_metric`，planner 已能编译 `raw_score`、clamp 后 `score`、`score_breakdown` 和 rank。 |
| Rearview run 执行 | `service/runner.rs` 已有正式 run 执行路径：compile SQL、查询 ClickHouse、写 PostgreSQL run pool/signal snapshot。 |
| Metric policy | `metric_policy.yml` 已提供 `allow_scoring`、`display.label_zh`、`display.group` 和 crossing metadata。 |

### 已确认缺口

| 缺口 ID | 缺口 | 影响 | 填补方向 |
|---|---|---|---|
| G1 | Rearview `ScoreClamp` validation 仍限制 `max <= 99`。 | 前端 100 分和后端最高 99 分不一致。 | 将后端 validation、代表性规则、测试和前端默认 spec 统一到 `[0, 100]`。 |
| G2 | Step 2 仍用 filter catalog 或静态 fallback，未按 `allow_scoring` 裁剪。 | 用户可能选择后端不允许评分的指标。 | 新增 scoring catalog adapter，Step 2 只消费 `allow_scoring = true`。 |
| G3 | Step 1 compare builder 绑定 filter context。 | scoring context 下右侧 metric、操作符和能力校验不同。 | 抽出 compare expr builder，显式传入 `capability = filter/scoring`。 |
| G4 | `WeightIndicator[]` 尚未转换成 `ScoringSpec`。 | 权重配置无法进入后端 planner。 | 新增 `buildStrategyWeightScoring()`，生成 `conditional_points` rules、effective points 和 path map。 |
| G5 | 点击「股池预览」当前只复制 mock 权重并进入本地 mock 页面。 | 没有真实选股、评分和排名。 | 新增 `buildStrategyPreviewRuleSpec()` 和 preview mutation，点击后调用后端 preview-only API。 |
| G6 | Rearview 没有 preview-only API。 | 前端只能选择 explain 或正式 run，二者都不符合股池预览语义。 | 新增 `POST /rearview/strategy-preview`，复用 planner 和 ClickHouse 查询，但不写正式 run。 |
| G7 | Preview response 类型和 UI 数据映射不存在。 | Step 3 不能展示真实 trade date、pool count、score breakdown。 | 新增 TS/Rust response contract，并替换 `StockPoolPreviewWorkbench` 的 mock 成功路径。 |
| G8 | score breakdown 名称和权重行没有稳定映射。 | 用户无法理解每只股票为什么得分。 | Adapter 生成稳定 scoring rule name，并保留 `weightId -> scoring.rules[i]` 映射。 |
| G9 | 修改 Step 1/Step 2 后 preview stale 状态缺失。 | 用户可能查看旧股票池结果。 | 新增 `lastPreviewRuleSpec/Result/At` 和 `isPreviewStale` 状态。 |
| G10 | 缺少 Step 2 到 Preview 的验收报告和样本。 | 后续阶段无法判断真实闭环是否完成。 | 新增 job report，记录 backend/API/frontend/browser 验收。 |

## 填补原则

1. 记录和执行分离：Step 1/2 只编辑草稿；点击「股池预览」才执行数据查询。
2. 后端是评分真相：前端不计算权威 score/rank，只提交结构化 `RuleVersionSpec`。
3. 前后端同一分数区间：所有评分 clamp 统一为 `[0, 100]`。
4. Preview 不污染正式状态：第一版 preview 不创建 rule set、rule version、run 或 portfolio run。
5. Adapter 先行：组件只编辑 UI 草稿，不直接拼后端 AST。
6. 每个阶段都有测试，不把 adapter、planner 和浏览器验证堆到最后。

## 实施阶段

### 阶段 1：Rearview score clamp 统一到 `[0, 100]`

目标：消除后端 `[0, 99]` 与前端 `[0, 100]` 的评分口径漂移。

任务：

1. 修改 `engines/crates/rearview-core/src/domain/rule.rs`：
   - `RuleVersionSpec::validate()` 允许 `scoring.clamp.max <= 100.0`。
   - validation error 文案更新为 `[0, 100]`。
   - `representative_rule()` 中 `ScoreClamp.max` 从 `99.0` 调整为 `100.0`。
2. 修改 `engines/crates/rearview-core/src/planner/sql.rs` 和相关测试 fixture 中的默认 clamp 期望。
3. 修改 `app/racingline_new/src/features/strategy/adapters.ts` 中 Step 1 placeholder scoring clamp，避免继续生成 max 99。
4. 补充或更新 Rust/TS 测试，覆盖 max 100 被接受、max 100.1 被拒绝。

测试策略：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core rule
cargo test -p rearview-core planner
```

前端涉及 adapter fixture 时追加：

```bash
cd app/racingline_new
npm test
npm run typecheck
```

完成标准：

- 后端接受 `{"min": 0, "max": 100}`。
- 后端拒绝超过 100 的 clamp。
- 前端生成的所有 `RuleVersionSpec.scoring.clamp` 默认都是 `[0, 100]`。

### 阶段 2：Step 2 scoring catalog adapter

目标：让 Step 2 权重配置只消费真实 scoring 指标。

任务：

1. 在 `app/racingline_new/src/features/strategy/adapters.ts` 新增 `buildStrategyScoringCatalog(metrics)`。
2. 复用现有中文 group/label/order 规则，但过滤条件改为：
   - `allow_scoring = true`。
   - `value_kind` 与 UI operator 支持兼容。
   - crossing operator 仅在 `cross.previous_metric` 存在时保留。
3. `WeightIndicatorsPanel` 增加 `catalogOptions` prop，并传给 `ComparisonFields`。
4. `createWeightIndicator(strategyScoringCatalog)` 用 scoring catalog 初始化新权重项。
5. summary formatter 使用 catalog label 渲染，避免权重摘要显示裸 metric id。
6. 当真实 scoring catalog 为空或 metrics 加载失败时，Step 2 显示明确状态，不把静态 catalog 当成功路径。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

关键测试用例：

- `allow_scoring = false` 的 metric 不进入 Step 2 catalog。
- `display.label_zh` 能进入权重面板 label。
- crossing operator 只对具备 `previous_metric` 的 scoring metric 展示。
- date 或不支持 UI operator 的 metric 被裁掉。

完成标准：

- Step 2 指标类型和指标名来自真实 Rearview metrics。
- Step 2 不允许选择后端不允许评分的指标。

### 阶段 3：Scoring adapter 和共享 compare builder

目标：把 `WeightIndicator[]` 转成 `ScoringSpec`，并保持 filter/scoring 两种能力边界。

任务：

1. 抽出共享 compare builder：
   - 输入 `ComparableIndicator`、catalog、`capability`。
   - `capability = "filter"` 时校验 `allow_filter`。
   - `capability = "scoring"` 时校验 `allow_scoring`。
2. 保留 Step 1 混排 AST 逻辑不变，只替换 leaf compare 构造实现。
3. 新增 `buildStrategyWeightScoring(weightIndicators, catalog, options)`：
   - 生成 `ScoringSpec { rules, clamp: { min: 0, max: 100 } }`。
   - 每条权重项生成一条 `conditional_points`。
   - 按 `score_budget = 100` 计算 effective points。
   - 拒绝空权重、总分为 0、非法数值、非法操作符、类型不兼容和无效 crossing。
   - 返回 `outputMetrics` 和 `weightPaths`。
4. 生成稳定 scoring rule name：
   - 建议格式：`w{index}_${metric}`，必要时追加短 id 防重复。
   - rule name 必须能映射回 UI 权重行。
5. 明确不生成 `weighted_metric`。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

关键测试用例：

- 单条权重生成一条 `conditional_points`，clamp 为 `[0, 100]`。
- 多条权重总分小于等于 100 时 points 保持原值。
- 多条权重总分大于 100 时 points 按比例缩放到总和 100。
- score 为 0 或无有效权重时报错。
- scoring 条件使用 `allow_scoring = false` metric 被拒绝。
- metric-vs-metric scoring 条件右侧也要求 `allow_scoring = true`。
- crossing scoring 条件补充 `prev_*` 依赖到 output metrics。

完成标准：

- Step 2 草稿能生成后端可校验的 `ScoringSpec`。
- Step 1 adapter 的既有单元测试不回退。

### 阶段 4：Preview rule composer

目标：点击「股池预览」时组合 Step 1 和 Step 2，生成完整 `RuleVersionSpec`。

任务：

1. 新增 `buildStrategyPreviewRuleSpec(conditionGroups, weightIndicators, catalog, options)`。
2. 复用 Step 1 `pool_filters` builder 和 Step 2 `scoring` builder。
3. 生成默认 universe：
   - `base = "all_a_shares"`
   - `exclude_st = true`
   - `exclude_suspend = true`
   - include/exclude 证券代码第一版留空。
4. 生成 `top_n_default`：
   - 默认 20。
   - 后续由 Preview UI 的 TopN 控件覆盖。
5. 合并 `output_metrics`：
   - catalog `default_output = true`。
   - Step 1 使用指标。
   - Step 2 使用指标。
   - crossing `prev_*` 依赖。
6. 返回 condition/weight path 映射，用于错误和 score breakdown 定位。
7. 不在 composer 中执行 API 请求。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
```

关键测试用例：

- 组合后的 rule 同时包含 Step 1 `pool_filters` 和 Step 2 `scoring.rules`。
- `output_metrics` 包含 filter/scoring/crossing 依赖且去重排序。
- 空 Step 1 或空 Step 2 会在本地 adapter 阶段失败，不发送 preview 请求。

完成标准：

- 点击「股池预览」时可以生成完整、可提交的 `RuleVersionSpec`。

### 阶段 5：Rearview preview-only API

目标：新增「股池预览」真实执行入口，复用 planner 和 ClickHouse 查询，但不写正式 run 状态。

任务：

1. 在 `engines/crates/rearview-core/src/api/mod.rs` 注册：

```http
POST /rearview/strategy-preview
```

2. 新增 request/response 类型：
   - request：`rule`、`start_date`、`end_date`、`top_n`。
   - response：`preview_id`、`sql_hash`、`required_metrics`、`required_marts`、`required_columns`、`trade_dates`。
3. 请求校验：
   - `start_date <= end_date`。
   - `top_n > 0`。
   - date range 第一版限制在小窗口，避免误跑长区间；默认建议不超过 `chunk_small_range_trading_days` 或明确配置上限。
   - `RuleVersionSpec` 必须通过 validation。
4. 复用 `QueryPlanner::compile()` 生成 SQL。
5. 复用 ClickHouse `query_screening_rows()` 执行 SQL。
6. 不调用 PostgreSQL run 写入，不创建 chunks/days/pool/signal snapshots。
7. 在内存中按 `trade_date` 聚合 rows：
   - `pool_count` 为当天候选池总数。
   - `signals` 第一版返回 `signal_rank <= top_n` 的行，必要时支持返回候选池前 N 行。
   - 保留 `score_breakdown`、`selected_metrics`、`raw_values`。
8. `preview_id` 可用 hash 或短 UUID，仅用于本次响应和日志，不作为持久化主键。
9. 错误响应沿用 Rearview 现有 error model；字段级 error path 留到后续 error contract 增强。

测试策略：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core rule
cargo test -p rearview-core planner
cargo test -p rearview-core api
```

若 API 测试当前没有精确 target，使用：

```bash
cd engines
cargo test --workspace
```

Smoke 验收：

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
```

完成标准：

- `POST /rearview/strategy-preview` 能执行完整 rule 并返回真实 score/rank。
- preview 不写 PostgreSQL run 相关表。
- invalid clamp、invalid scoring metric、超大日期范围和非法 top_n 会失败。

### 阶段 6：前端 preview API runtime

目标：让 `app/racingline_new` 能调用 Rearview preview-only API。

任务：

1. 在 `app/racingline_new/src/types/rearview.ts` 增加 preview request/response 类型。
2. 在 `app/racingline_new/src/api/rearview.ts` 增加 `previewStrategy()`。
3. 在 `app/racingline_new/src/api/hooks.ts` 增加 `useStrategyPreviewMutation()`。
4. 处理 loading/error/success/stale 状态。
5. 预览请求第一版使用页面默认区间和 TopN：
   - 若 UI 尚无日期控件，先在 Step 3 preview panel 提供明确的 date range/TopN 控件。
   - 不允许静默用无限日期范围。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

完成标准：

- 前端能构造 preview request 并调用真实 API。
- API 失败时保留草稿并显示错误，不回退 mock 成功状态。

### 阶段 7：`/strategies` Step 2/3 状态和 UI 接入

目标：把 Step 2 记录评分规则、Step 3 执行 preview 的业务流程落到页面状态。

任务：

1. 在 `StrategyPage` 增加 preview 状态：
   - `lastPreviewRuleSpec`
   - `lastPreviewResult`
   - `lastPreviewAt`
   - `isPreviewStale`
   - `previewAdapterError`
2. 修改 Step 1/Step 2 草稿时将已有 preview 标记为 stale。
3. Step 2 点击「股池预览」时：
   - 先执行 `buildStrategyPreviewRuleSpec()`。
   - 再调用 `previewStrategy()`。
   - 成功后进入 Step 3，展示真实 preview result。
4. Step 3 的「更新股池」使用当前草稿重新生成 rule 并再次调用 preview API。
5. `PoolPreviewPanel` 和 `StockPoolPreviewWorkbench` 增加真实 response props：
   - 成功时显示真实 trade dates、pool count、signals、score、rank、score breakdown。
   - 无真实 response 时显示“尚未预览”状态。
   - mock 数据只能作为显式 dev fallback 或测试 fixture，不作为成功状态。
6. 保持 responsive layout 和当前 Step 页面信息密度，不引入新首页或无关装饰。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build
```

浏览器验收：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

完成标准：

- Step 1/2 编辑不会触发真实选股。
- 点击 Step 2「股池预览」后才出现真实股票池。
- 修改 Step 1 或 Step 2 后已有 preview 被标记为 stale。
- score breakdown 可追溯到 Step 2 权重项。

### 阶段 8：验收报告和文档交接

目标：把 Step 2 到 Preview 的完成状态沉淀为可复查记录。

任务：

1. 新增 `docs/jobs/reports/YYYY-MM-DD-racingline-strategy-step2-preview.md`。
2. 报告记录：
   - Rearview clamp `[0, 100]` 测试结果。
   - `POST /rearview/strategy-preview` 请求/响应样本。
   - 前端 adapter 单元测试结果。
   - 前端 lint/typecheck/build 结果。
   - 浏览器验收路径和关键观察项。
3. 更新本计划状态为 `Completed` 后移入 `docs/plans/archive/`。
4. 更新 `docs/plans/README.md` 的 active/completed 索引。
5. 必要时补充系统地图中的报告链接。

完成标准：

- Step 2 到 Preview 的真实状态不依赖聊天记录。
- 后续 Step 4 模拟建仓/回测可以消费真实 preview result 或后续持久化 preview snapshot。

## 依赖顺序

| 先决项 | 解锁项 | 原因 |
|---|---|---|
| 阶段 1 | 阶段 3、4、5 | clamp 口径必须先统一，否则 adapter 和后端 preview 会继续漂移。 |
| 阶段 2 | 阶段 3、7 | Step 2 builder 和 UI 都依赖 scoring catalog。 |
| 阶段 3 | 阶段 4 | preview rule composer 需要 scoring builder。 |
| 阶段 4 | 阶段 5、6、7 | 后端和前端 preview 请求都需要完整 rule contract。 |
| 阶段 5 | 阶段 6、7 | 前端真实 preview 依赖 API。 |
| 阶段 7 | 阶段 8 | 验收报告必须基于真实 UI/API 闭环。 |

## 禁止模式

1. 禁止把 Step 1 explain 结果描述为“已生成股票池”。
2. 禁止 Step 2 编辑权重时触发真实选股或评分。
3. 禁止用 `POST /rearview/explain` 替代股池预览结果。
4. 禁止 preview-only API 创建 rule set、rule version、run 或 portfolio run。
5. 禁止在浏览器内计算权威 score/rank。
6. 禁止使用静态 catalog 作为真实 scoring 成功路径。
7. 禁止直接生成 `weighted_metric` 作为第一版权重语义。
8. 禁止继续生成或接受默认 `[0, 99]` scoring clamp。
9. 禁止为了 preview 绕过 Rearview metric catalog validation 或 planner。

## 允许保留的例外

1. Step 1 的 `POST /rearview/explain` 可继续作为编译校验和字段依赖展示，但不得把 explain 结果作为股票池执行结果。
2. `StockPoolPreviewWorkbench` 可保留 mock fixture 用于 Story/test/dev fallback，但真实 API 可用时不得把 mock 当成功结果。
3. Preview response 第一版可以只返回每个交易日 topN signals 和 pool_count，不必返回完整候选池全量行；全量候选池分页可后续补 API。
4. Preview result 第一版可以不持久化；如果后续模拟建仓需要稳定复用同一批股票池，再设计 preview snapshot。

## 总体验证命令

文档-only 变更：

```bash
make docs-check
git diff --check
```

Rearview：

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

前端：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build
```

浏览器：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

## 完成标准

1. Rearview scoring clamp 全链路统一为 `[0, 100]`。
2. Step 2 权重选择器只展示真实 `allow_scoring = true` 指标。
3. `WeightIndicator[]` 能生成 `conditional_points` scoring rules 和有效 points。
4. 点击「股池预览」时生成完整 `RuleVersionSpec`，包含 Step 1 `pool_filters`、Step 2 `scoring`、`top_n_default` 和完整 `output_metrics`。
5. `POST /rearview/strategy-preview` 能真实执行选股、评分和排名，并返回 score breakdown。
6. Preview 不创建正式 rule/run/portfolio 状态。
7. Step 3 UI 使用真实 preview response 替代 mock 成功路径。
8. 修改 Step 1/Step 2 后，已有 preview 结果标记 stale。
9. 后端、前端和浏览器验收结果写入 job report。
10. 完成后归档本计划，并同步 `docs/plans/README.md` 和相关系统地图。
