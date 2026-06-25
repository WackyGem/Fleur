# RFC 0025: Racingline 策略权重配置 Step 2 实现方案

状态：Implemented（2026-06-22）
领域：racingline, rearview
关联系统：racingline, rearview
代码根：app/racingline_new/, app/racingline/, engines/crates/rearview-core/
需求入口：docs/intake/racingline.md

路径说明：本文写于 Plan 0053 迁移前；文中的 `app/racingline_new/` 均为历史实现路径，当前 Racingline 前端代码根为 `app/racingline/`。

## 摘要

本文档修正 `/strategies` Step 2「权重配置」的业务边界。

正确流程是：

```text
Step 1 策略选股
  记录股票池筛选条件 pool_filters
  不真正生成股票池

Step 2 权重配置
  记录股票池内个股评分规则 scoring.rules
  不真正打分和排名

点击「股池预览」
  组合 Step 1 + Step 2 草稿
  执行选股、评分、排名
  返回可预览股票池
```

因此 Step 2 的核心不是一个独立的 explain 闭环，而是把用户配置的指标权重转成 Rearview 可执行的 `RuleVersionSpec.scoring.rules`。真正的选股和评分发生在「股池预览」动作中。

`POST /rearview/explain` 仍可作为编译校验工具，但它不是业务执行点，也不产出股票池。

## 背景

`docs/Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md` 定义了 `/strategies` 的完整路径：策略选股、权重配置、股池预览、模拟建仓、策略回测和运行策略。

`docs/RFC/0024-racingline-strategy-selection-step1.md` 和 `docs/jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md` 已完成 Step 1：

- `app/racingline_new` 已接入真实 Rearview metric catalog。
- Step 1 已能生成 `RuleVersionSpec.pool_filters`。
- 组内 `AND/OR` 混排已固定为 `AND` 高于 `OR` 的 nested `all` / `any` AST。
- `POST /rearview/explain` 已能校验普通比较、指标比较、`between`、`is_null` 和趋势 crossing。

Step 1 的 explain 是规则草案的编译校验，不是股票池执行。它不能被解释为“已经筛出股票”。Step 2 同理：权重配置阶段只记录评分条件，不能把 explain 结果当成评分结果。

## 业务流程定义

### Step 1: 策略选股

用户用指标条件描述“哪些股票可以进入候选池”。输出是 `pool_filters` 草稿：

```text
conditionGroups -> RuleVersionSpec.pool_filters
```

完成 Step 1 后，系统只有筛选条件，没有实际股票池结果。

### Step 2: 权重配置

用户描述“进入候选池的股票如何打分和排序”。输出是 `scoring.rules` 草稿：

```text
weightIndicators -> RuleVersionSpec.scoring.rules
```

这些评分规则只作用于 Step 1 筛选后的股票池，不应被理解为独立选股条件。

### 股池预览动作

点击「股池预览」时，前端组合完整规则：

```text
RuleVersionSpec {
  universe,
  pool_filters,   # Step 1
  scoring,        # Step 2
  top_n_default,
  output_metrics
}
```

Rearview 在这个动作中真正执行：

1. 按 `universe` 和 `pool_filters` 筛出候选股票池。
2. 对候选池内每只股票应用 `scoring.rules`。
3. 按 `score DESC, security_code ASC` 排名。
4. 取 `top_n_default` 作为买入信号或预览重点。
5. 返回股票、分数、排名、score breakdown 和 selected metrics。

## 目标

1. 盘点 Step 2 权重配置可复用资源和缺口。
2. 明确 Step 2 是“记录评分规则”，不是执行评分。
3. 设计 `WeightIndicator[] -> RuleVersionSpec.scoring.rules` adapter。
4. 让 Step 2 只使用真实 metric catalog 中 `allow_scoring = true` 的指标。
5. 明确「股池预览」是首次真实执行选股、评分和排名的动作。
6. 为 Step 3 预览执行 API 留出清晰 contract。

## 非目标

1. 不在 Step 2 页面加载或权重编辑时执行选股、评分或排名。
2. 不把 `POST /rearview/explain` 当成股票池预览。
3. 不在浏览器内计算权威分数、排名、成交、持仓或净值。
4. 不在本 RFC 中实现真实模拟建仓、回测或运行策略。
5. 不引入新的指标归一化、横截面 rank、z-score、winsorize 或行业中性化规则。
6. 不把 `app/racingline_new/src/features/strategy/catalog.ts` 当作真实 scoring catalog。
7. 不把 `app/racingline_new` 直接升级为正式工程；正式迁移仍需遵守 RFC 0023、ADR 0011 和 ADR 0013。

## 当前资源盘点

### 前端原型资源

| 资源 | 路径 | 当前价值 | 缺口 |
|---|---|---|---|
| Step 流程 | `app/racingline_new/src/routes/strategy-page.tsx` | 已有 `策略选股 -> 权重配置 -> 股池预览` 的导航节奏 | `股池预览` 仍是 mock，不会真实执行 Step 1 + Step 2 |
| 权重草稿类型 | `app/racingline_new/src/features/strategy/types.ts` | `WeightIndicator = ComparableIndicator + score`，表达“评分条件 + 得分” | 还不是 Rearview `ScoringRule` |
| 权重面板 | `app/racingline_new/src/features/strategy/components/weight-indicators-panel.tsx` | 支持添加/删除权重项、比较字段和得分 slider/input | 未接收真实 scoring catalog，比较控件仍会回退静态 catalog |
| 权重缩放工具 | `app/racingline_new/src/features/strategy/utils.ts` | `getScaledWeightIndicators()` 已按 0-100 口径把多项 score 做总量缩放 | Rearview 当前 clamp validation 仍是 `[0, 99]`，实施 Step 2 时必须调整为 `[0, 100]` |
| Mock 股池贡献 | `app/racingline_new/src/features/strategy/components/stock-pool-preview-workbench.tsx` | 已展示 score item 和权重贡献的体验 | 贡献来自 mock 计算，不是 Rearview 选股和 `score_breakdown` |
| Step 1 adapter | `app/racingline_new/src/features/strategy/adapters.ts` | 已能构造 `pool_filters`、`output_metrics` 和 condition path 映射 | builder 绑定 filter context，不能直接复用为 scoring context |

### Rearview 后端资源

| 资源 | 路径 | 当前能力 | Step 2/预览复用方式 |
|---|---|---|---|
| `RuleVersionSpec.scoring` | `engines/crates/rearview-core/src/domain/rule.rs` | 支持 `ScoringSpec { rules, clamp }` | Step 2 生成完整 `scoring.rules` |
| `conditional_points` | `engines/crates/rearview-core/src/domain/rule.rs` | 条件满足时给固定分数 | 第一版权重配置的默认语义 |
| `weighted_metric` | `engines/crates/rearview-core/src/domain/rule.rs` | `coalesce(metric, 0) * weight` | 暂不作为默认语义，等待归一化设计 |
| scoring validation | `engines/crates/rearview-core/src/domain/rule.rs` | scoring context 会检查 `allow_scoring`、操作符、类型和 crossing 前值 | 前端必须只生成 catalog 允许的 scoring 条件 |
| SQL planner | `engines/crates/rearview-core/src/planner/sql.rs` | 已编译 `pool_filters`、`raw_score`、clamp 后 `score`、排名和 `score_breakdown` | 预览执行可以复用同一条 planner SQL |
| Explain API | `engines/crates/rearview-core/src/api/mod.rs` | 可提交完整 `RuleVersionSpec` 做编译校验 | 只用于校验，不返回股票池 |

### Metric catalog 资源

| 资源 | 路径 | 当前能力 | Step 2 复用方式 |
|---|---|---|---|
| policy overlay | `engines/crates/rearview-core/config/metric_policy.yml` | `numeric_metric` 默认 `allow_scoring: true`，`boolean_metric` 默认不允许 scoring | Step 2 只展示 `allow_scoring = true` 的指标 |
| display hint | `MetricDefinition.display` | 已有 `group`、`label_zh`、`unit`、`sort_order` | 权重面板沿用中文指标类型和中文指标名 |
| crossing 元数据 | `MetricDefinition.cross.previous_metric` | 趋势指标已具备上穿/下穿前值字段 | scoring 条件可复用 crossing，但必须由 catalog 明确允许 |

### 旧 Racingline 可复用资源

| 资源 | 路径 | 当前价值 | 复用方式 |
|---|---|---|---|
| 简单规则 builder | `app/racingline/src/store/workbench.ts` | 已生成 `weighted_metric` 示例 | 作为后续加权原始指标参考，不作为 Step 2 默认实现 |
| 低位反转 preset | `app/racingline/src/store/workbench.ts` | 已生成多条 `conditional_points` scoring rules | 作为 Step 2 conditional scoring 的已验证后端形态参考 |
| Run result 展示 | `app/racingline/src/features/runs/components/run-results.tsx` | 已消费 run result 中的 score、rank、score breakdown 和 selected metrics | Step 3 预览结果展示可复用信息结构 |

## 缺口与填充方案

| 缺口 ID | 缺口 | 影响 | 填充方案 |
|---|---|---|---|
| G1 | Step 2 没有明确“记录规则，不执行”的边界。 | 容易把 explain 或本地 mock 当成真实评分结果。 | 明确 Step 2 只生成 `scoring.rules`；点击「股池预览」才执行选股、评分、排名。 |
| G2 | 权重项没有后端语义选择。 | “权重”可能被误解为原始指标乘权重，导致不同量纲指标直接相加。 | 第一版固定为 `conditional_points`：候选池内股票满足评分条件即获得该项分数；`weighted_metric` 留到归一化 RFC。 |
| G3 | `WeightIndicatorsPanel` 未消费真实 scoring catalog。 | Step 2 会显示静态 mock 字段，无法保证后续执行可通过。 | 增加 scoring catalog adapter，只展示 `allow_scoring = true` 且操作符兼容的指标，并把 `catalogOptions` 传入权重面板和比较控件。 |
| G4 | Step 1 adapter 只按 filter context 校验。 | scoring context 下 `allow_filter` 和 `allow_scoring` 的合法性不同。 | 抽出 compare expr builder，显式传入 `capability = "filter" | "scoring"`；Step 2 使用 scoring context。 |
| G5 | 前后端得分缩放和 clamp 口径不一致。 | 前端按 0-100 展示和缩放，后端当前 validation 仍限制到 `[0, 99]`，会导致用户看到的最高分与后端实际分数不一致。 | 后端 `ScoreClamp` validation、planner 输出和测试统一调整到 `[0, 100]`；adapter 使用 `score_budget = 100` 生成 points，最终 clamp 固定 `[0, 100]`。 |
| G6 | `output_metrics` 未包含评分相关指标。 | 预览结果无法解释得分来源。 | Step 2 adapter 在 Step 1 `output_metrics` 基础上追加所有 scoring 条件中用到的 metrics 和 crossing 前值依赖。 |
| G7 | Score breakdown 名称不可控。 | 预览面板无法稳定映射每条权重贡献。 | Adapter 为每条权重项生成稳定 rule name，并保留 `weightId -> scoring.rules[i]` 映射。 |
| G8 | 当前 mock 股池预览不调用 Rearview。 | 用户点击「股池预览」看不到真实筛选、评分、排名。 | 新增或补齐 preview execution API，输入完整 `RuleVersionSpec` 和日期/topN，返回按交易日的候选池与 score breakdown。 |
| G9 | Preview 执行 API contract 未定义。 | 前端不知道点击「股池预览」应调用 explain、临时 run 还是正式 run。 | RFC 明确推荐 preview-only execution，不发布 rule version，不进入正式 run 状态机。 |
| G10 | Explain 错误没有字段路径。 | scoring 失败时难以定位到具体权重行。 | 先用本地 path 映射和错误摘要定位；字段级错误 contract 作为后续 Rearview error contract 增强项。 |

## 设计

### D1: Step 2 第一版使用 conditional scoring

当前 UI 的权重行不是“选择一个原始指标并乘权重”，而是：

```text
指标 + 比较方式 + 比较对象/值 + 得分
```

这与 Rearview 的 `conditional_points` 对应：

```json
{
  "type": "conditional_points",
  "name": "w01_kdj_j_value",
  "condition": {
    "type": "compare",
    "left": {"type": "metric", "name": "kdj_j_value"},
    "op": "lt",
    "right": {"type": "number", "value": 20}
  },
  "points": 25
}
```

该 scoring rule 的作用范围是 Step 1 已筛出的候选股票池。也就是说，评分条件不负责扩大股票池；它只对候选池内股票加分。

`weighted_metric` 第一版不作为默认语义。原因是 `weighted_metric` 会直接把原始指标值乘以权重，例如价格、成交量、RSI、KDJ、连跌天数的量纲和方向完全不同；在没有归一化、方向、截尾和缺失值策略前直接相加，会产生不可解释分数。

### D2: Scoring catalog 是 Step 2 唯一指标来源

新增 scoring catalog adapter：

```text
MetricDefinition[]
  -> allow_scoring metrics
  -> IndicatorCatalog[]
```

规则：

1. 只展示 `allow_scoring = true` 的指标。
2. 保留 `display.group`、`display.label_zh`、`display.unit` 和 `sort_order`。
3. 操作符仍取 `allowed_ops`，但按 value kind 和 UI 能力裁剪。
4. 指标比较右操作数也必须来自 `allow_scoring = true` 的指标。
5. `crosses_above` / `crosses_below` 只在 metric 暴露 `cross.previous_metric` 时展示。
6. `prev_*` 依赖字段如果 `allow_scoring = false`，不出现在权重选择器中，但可作为 crossing 依赖进入执行查询。

### D3: 权重草稿模型保留，新增 scoring adapter

保留现有 `WeightIndicator` 作为 UI 草稿模型，不把组件状态改成后端 AST。新增 adapter：

```text
buildStrategyWeightScoring(weightIndicators, catalog, options)
  -> scoring
  -> outputMetrics
  -> weightPaths
```

它不直接执行查询，只返回可并入 `RuleVersionSpec` 的 scoring 片段。

输入：

- `weightIndicators`：Step 2 当前权重草稿。
- `catalog`：真实 Rearview metrics。
- `options.scoreBudget`：默认 `100`。

输出：

- `scoring`：`ScoringSpec { rules, clamp }`。
- `outputMetrics`：评分条件所依赖的 metrics。
- `weightPaths`：本地映射，形如 `weightId -> scoring.rules.0.condition`，用于错误定位和预览结果展示。

### D4: 得分缩放规则

用户输入、前端缩放、后端 validation 和最终 score clamp 必须统一为 0-100 区间。前端当前 `clampScore()` 和 `clampWeightTotal()` 已按 0-100 处理；Rearview 当前 `RuleVersionSpec::validate()` 中的 `[0, 99]` 限制需要在 Step 2 实施时调整到 `[0, 100]`。

提交给 Rearview 的 points 按同一预算缩放：

```text
raw_i = clamp(round(input_i), 0, 100)
raw_total = sum(raw_i)
score_budget = 100

if raw_total == 0:
  adapter validation failed
else if raw_total <= score_budget:
  points_i = raw_i
else:
  points_i = raw_i * score_budget / raw_total
```

这里的“生成”不是生成单独配置文件，也不是写入 `metric_policy.yml`。它是在点击「股池预览」时，由前端 adapter 生成完整 `RuleVersionSpec` 请求体中的字段：

```text
rule.scoring.clamp
```

生成的 `RuleVersionSpec.scoring.clamp` 固定为：

```json
{"min": 0, "max": 100}
```

后端执行时仍以 SQL planner 中的 `greatest(clamp_min, least(clamp_max, raw_score))` 产出最终 `score`，但 `clamp_max` 必须允许 100。这样可以避免前端展示 100 分、后端最高只允许 99 分的口径漂移。UI 应同时展示原始得分和提交执行的有效得分。

在 preview-only 阶段，该字段随 `POST /rearview/strategy-preview` 请求提交给 Rearview，不单独持久化。后续如果用户发布为正式 rule version，同一个 `RuleVersionSpec.scoring.clamp` 才会作为规则版本 spec 的一部分持久化，并参与 rule hash。

### D5: 完整 RuleVersionSpec 在股池预览时生成

点击「股池预览」时才组合完整规则：

```text
buildPreviewRuleSpec(selectionDraft, weightDraft, catalog, options)
  -> RuleVersionSpec
```

合成规则：

1. `universe` 来自 Step 1 默认或用户后续配置。
2. `pool_filters` 来自 Step 1。
3. `scoring` 来自 Step 2。
4. `top_n_default` 来自页面 TopN 或默认值。
5. `output_metrics` 合并：
   - metric catalog 中 `default_output = true` 的指标；
   - Step 1 筛选条件使用的指标；
   - Step 2 scoring 条件使用的指标；
   - crossing 所需 `prev_*` 依赖指标。

### D6: 股池预览执行 API

Step 2 本身不新增真实执行。真实执行由「股池预览」动作触发。推荐新增 preview-only API：

```http
POST /rearview/strategy-preview
```

请求体：

```json
{
  "rule": { "universe": {}, "pool_filters": {}, "scoring": {}, "top_n_default": 20, "output_metrics": [] },
  "start_date": "2026-01-01",
  "end_date": "2026-01-31",
  "top_n": 20
}
```

响应体第一版可返回：

```json
{
  "preview_id": "optional-debug-id",
  "sql_hash": "compiled-query-hash",
  "trade_dates": [
    {
      "trade_date": "2026-01-31",
      "pool_count": 128,
      "signals": [
        {
          "security_code": "600000.SH",
          "security_name": "浦发银行",
          "score": 83.5,
          "signal_rank": 1,
          "is_buy_signal": true,
          "score_breakdown": {},
          "selected_metrics": {},
          "raw_values": {}
        }
      ]
    }
  ],
  "required_metrics": [],
  "required_marts": [],
  "required_columns": {}
}
```

API 语义：

1. 不创建 rule set。
2. 不创建 rule version。
3. 不创建正式 run。
4. 可以记录临时 preview id、SQL hash 和执行摘要用于调试。
5. 底层复用 Rearview planner 的选股、评分、排名 SQL。

如果短期不新增 API，也可以用临时 run 方案替代，但必须在 RFC/plan 中明确它不是正式策略运行，避免污染正式 rule/run 生命周期。

### D7: 状态和导航 gate

建议状态模型：

```text
selectionDraft            # Step 1 草稿
weightDraft               # Step 2 草稿
lastPreviewRuleSpec       # 点击股池预览时生成的完整规则
lastPreviewResult         # 真实执行返回的股票池和评分结果
lastPreviewAt
isPreviewStale
```

状态规则：

1. 修改 Step 1 条件：预览结果 stale。
2. 修改 Step 2 权重：预览结果 stale。
3. 点击「股池预览」：生成完整 `RuleVersionSpec`，先做本地 adapter validation，再调用 preview execution API。
4. Preview 成功后进入 Step 3，展示真实股票池。
5. Step 3 后续修改权重只改变草稿，不改变已应用预览结果；需要再次点击「更新股池」才重新执行。

## 初步实现路径

### Phase 1: Scoring catalog adapter

目标：让 Step 2 不再使用静态 catalog。

任务：

1. 新增 `buildStrategyScoringCatalog(metrics)`。
2. 只保留 `allow_scoring = true` 且 UI 支持的指标。
3. 沿用中文 group 和 label。
4. 将 `catalogOptions` 传入 `WeightIndicatorsPanel`、`ComparisonFields`、summary formatter 和 `createWeightIndicator()`。
5. 保留静态 catalog 只作为明确的 prototype fallback，不进入真实执行路径。

完成标准：

- Step 2 权重选择器显示真实 Rearview scoring metrics。
- 不允许选择 `allow_scoring = false` 的 metric。
- 权重摘要使用中文 label，而不是裸 metric id。

### Phase 2: Scoring adapter

目标：把权重草稿转换为 `ScoringSpec`。

任务：

1. 新增 `buildStrategyWeightScoring(weightIndicators, catalog, options)`。
2. 抽出 Step 1/Step 2 共享 compare expr builder，并显式区分 filter/scoring capability。
3. 生成 `conditional_points` scoring rules。
4. 使用 `score_budget = 100` 计算 effective points。
5. 输出 scoring 依赖 metrics 和 `weightId -> scoring rule path` 本地映射。

完成标准：

- 生成的 scoring 可并入 `RuleVersionSpec`。
- 空权重、总分为 0、非法指标、非法操作符、类型不兼容和 crossing 前值缺失都会在 adapter 阶段失败。
- Rearview `ScoreClamp` validation 接受 `{"min": 0, "max": 100}`，最终分数 clamp 到 `[0, 100]`。

### Phase 3: Preview rule composer

目标：点击「股池预览」时生成完整规则。

任务：

1. 新增 `buildStrategyPreviewRuleSpec(selectionDraft, weightDraft, catalog, options)`。
2. 组合 Step 1 `pool_filters` 和 Step 2 `scoring`。
3. 合并 `output_metrics`。
4. 保留 condition/weight path 映射。
5. 在请求 preview API 前可选调用 explain 做编译校验，但 explain 不是最终结果。

完成标准：

- 点击「股池预览」时的请求体包含完整 `RuleVersionSpec`。
- 修改 Step 1 或 Step 2 后，已有 preview 结果明确 stale。

### Phase 4: Preview execution API

目标：让「股池预览」成为第一次真实执行选股、评分和排名的动作。

任务：

1. 新增或确认 preview-only API contract。
2. 后端复用 Rearview planner 编译和执行 SQL。
3. 返回 trade_date、pool_count、signals、score、rank、score_breakdown、selected_metrics 和 raw_values。
4. 前端 Step 3 使用真实 preview response 替换 mock 股池。
5. API 失败时保留草稿，不伪造成功结果。

完成标准：

- Step 1/2 配置不会触发真实选股。
- 点击「股池预览」会真实执行并返回候选池和评分排名。
- score breakdown 能映射到 Step 2 权重项。

### Phase 5: 验收和交接

目标：把 Step 2 到 Preview 的真实状态沉淀为可复查事实。

任务：

1. 增加 adapter 单元测试。
2. 增加 preview API 或临时 run 的后端测试。
3. 增加浏览器验收：真实 catalog、添加权重、点击股池预览、返回真实结果、stale 状态。
4. 增加 job report，记录前端、Rearview、preview 样本和浏览器观察项。

完成标准：

- Step 2 不再依赖聊天记录说明业务边界。
- Step 3 可以直接消费真实 preview result。

## API 依赖矩阵

| API | 当前状态 | Step 2/预览用途 |
|---|---|---|
| `GET /rearview/metrics` | 已存在 | 获取 `allow_scoring`、`allowed_ops`、display hint 和 crossing 能力 |
| `POST /rearview/explain` | 已存在 | 可选编译校验，不执行选股或评分 |
| `POST /rearview/strategy-preview` | 待新增或待确认 | 点击「股池预览」时执行选股、评分和排名 |
| `POST /rearview/rule-sets` | 已存在 | 后续发布阶段使用，Step 2/预览不调用 |
| `POST /rearview/runs` | 已存在 | 后续正式运行或回测阶段使用，preview-only 第一版不直接污染正式 run |

## 测试和验收建议

文档阶段：

```bash
make docs-check
git diff --check
```

前端实现阶段：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build
```

涉及 Rearview preview API、scoring validation 或 planner 行为变化时追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 metric policy scoring 能力变化时追加：

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
```

浏览器验收：

```bash
node scripts/check_playwright_cdp.mjs
playwright-cli attach --cdp="${PLAYWRIGHT_CDP_ENDPOINT:-http://127.0.0.1:9222}"
```

至少检查：

- Step 1 配置条件不会生成真实股票池。
- Step 2 配置权重不会真实打分。
- Step 2 指标类型和指标名使用中文展示。
- `allow_scoring = false` 的指标不出现在权重选择器。
- 点击「股池预览」后才出现真实股票池、score、rank 和 score breakdown。
- 修改 Step 1 或 Step 2 后，已有 preview 结果标记为 stale。

## 风险与待决问题

1. Preview-only API 已采用新 endpoint `POST /rearview/strategy-preview`，不复用临时 run。
2. 本轮不调整 Step 1/Step 2/Step 3 前端可见文案；如需从“指标权重”改为“评分条件”或“评分权重”，需另起 UI 文案设计。
3. Boolean 或 pattern 指标是否应允许 scoring；当前 policy 默认 `boolean_metric.allow_scoring = false`。
4. `weighted_metric` 是否需要一个独立 Step 2.5 RFC，补齐归一化、方向和缺失值策略。
5. Rearview error response 是否需要字段路径，以支持权重行级错误定位。
6. Preview 执行是否需要保存短期快照，以支持用户从 Step 3 进入模拟建仓时复用同一批股票池。

## 相关文档

- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](0023-racingline-frontend-prototype-led-development.md)
- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](0024-racingline-strategy-selection-step1.md)
- [Plan 0045: Racingline 策略选股 Step 1 缺口填补实施计划](../plans/archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md)
- [Racingline Strategy Step 1 Gap Closure 报告](../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md)
- [Plan 0046: Racingline 策略权重配置 Step 2 实施计划](../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md)
- [Racingline Strategy Step 2 Preview Implementation 报告](../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md)
- [System: Racingline](../systems/racingline.md)
- [System: Rearview](../systems/rearview.md)
