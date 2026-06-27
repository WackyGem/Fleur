# Plan 0048: Racingline Step 3 股池预览漂移修正实施计划

日期：2026-06-22

状态：Completed

关联文档：

- [Debt 0004: Strategies Step 3 股池预览实现漂移](../../issues/archive/debt/0004-2026-06-22-strategies-step3-implemennt-drift.md)
- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)
- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](../../RFC/archive/0024-racingline-strategy-selection-step1.md)
- [RFC 0025: Racingline 策略权重配置 Step 2 实现方案](../../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md)
- [RFC 0026: Racingline 股池预览 Step 3 实现方案](../../RFC/archive/0026-racingline-strategy-pool-preview-step3.md)
- [Plan 0047: Racingline 股池预览 Step 3 实施计划](0047-racingline-strategy-pool-preview-step3-implementation-plan.md)
- [Racingline Strategy Step 3 Preview Implementation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step3-preview.md)
- [System: Racingline](../../architecture/racingline.md)
- [System: Rearview](../../architecture/rearview.md)
- [Step3 Drift Remediation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md)

## 目标

1. 将 `/strategies` Step 3 从“候选池分页 + 个股 analysis + 指标解释 + 局部权重调整”的复合页面，收缩回“股池预览结果检查页”。
2. 移除 Step 3 用户可见的开始日期、结束日期和展示行数配置；这些参数只能作为内部 execution/timeline 策略存在。
3. 让 K 线默认展示近一年行情，并确保除权、前复权、后复权切换真实改变图表数据。
4. 让 MA5、MA10、MA30 趋势线叠加真实渲染到 K 线图，而不是只改变前端 state。
5. 让 K 线下方横轴展示近一年交易日的股票池概览，而不是短日期区间。
6. 固定股票池分页为 10 个个股一页。
7. 明确表格语义：得分项只来自 Step 2 权重配置，指标列只来自 Step 1 指标组。
8. 保持 Step 1/2/3 的成功路径全部来自 Rearview 真实接口，不恢复任何 mock 成功路径。

## 非目标

1. 不重新设计 Step 1 和 Step 2 的表单交互。
2. 不在 Step 3 编辑 Step 1 筛选条件或 Step 2 权重规则。
3. 不在 Step 3 引入建仓、回测、运行策略或组合参数。
4. 不创建 rule set、rule version、run、portfolio run 或持久化 preview result。
5. 不在浏览器内计算权威股票池、分数、排名、K 线、复权价格或指标命中。
6. 不展示行业、板块、同类分组或其他与本 debt 无关字段。
7. 不把 `selected_metrics` 整包作为“命中指标”面板重新展示。
8. 不把 Step 3 的内部分页数量、preview execution range 或 debug 字段暴露为用户主控件。

## 当前事实基线

### 已有能力

| 领域 | 当前事实 |
|---|---|
| Step 1 真实 catalog | `app/racingline_new` 已使用 `GET /rearview/metrics` 构造筛选指标选项。 |
| Step 1 rule adapter | `buildStrategySelectionRuleSpec()` 已生成 `RuleVersionSpec.pool_filters`，并保留 `conditionPaths`。 |
| Step 2 scoring adapter | `buildStrategyWeightScoring()` 已将 `WeightIndicator[]` 转为 `scoring.rules`，并固定 clamp 到 `[0, 100]`。 |
| Preview snapshot | `PreviewSnapshot` 已存在，能保存 applied rule、range、labels、result 和 stale 状态。 |
| Preview API | `POST /rearview/strategy-preview` 已返回 `preview_id`、required metrics/marts/columns、按交易日组织的 pool count 和 signals。 |
| Pool page API | `POST /rearview/strategy-preview/pool-page` 已按单日分页返回候选股。 |
| Security analysis API | `POST /rearview/strategy-preview/security-analysis` 已返回选中个股的 quote rows、chart series、result snapshot 和行情上下文。 |
| 证券显示信息 | preview rows 已包含 `security_name` 和 `exchange_code`，且不包含行业/板块字段。 |

### 漂移事实

| 漂移 | 当前实现 | 需要修正为 |
|---|---|---|
| Step 3 有日期和行数输入 | `PoolPreviewPanel` 暴露开始日期、结束日期、展示行数。 | Step 3 不展示这些输入；内部固定近一年窗口和分页策略。 |
| 横轴日期过短 | 默认 range 为 `2026-05-26` 到 `2026-06-01`。 | 横轴覆盖近一年交易日。 |
| 分页过大 | `pageSize = 50`。 | 固定 10 只一页。 |
| 趋势线无效 | `trendLines` 只在前端 state 中变化，没有渲染 line series。 | MA5/MA10/MA30 可见状态驱动图表叠加线。 |
| 右侧展示 debug 信息 | `KeyDataPanel` 展示“命中指标”“原始值”和权重滑杆。 | 右侧只保留行情、估值和必要上下文。 |
| Step 3 可调权重 | `WeightControlSection` 在 Step 3 允许改 Step 2 权重。 | 用户回 Step 2 改权重，Step 3 只看 applied preview。 |
| 指标列语义混杂 | `selectedMetricRows` 来自合并后的 `output_metrics`。 | 指标列只来自 Step 1 指标组，得分项只来自 Step 2 权重。 |

## 实现缺口

| 缺口 ID | 缺口 | 影响 | 填补方案 |
|---|---|---|---|
| G1 | Step 3 UI 职责未收缩。 | 页面继续暴露执行参数和 Step 2 编辑控件。 | 移除日期/行数输入、命中指标、原始值、权重滑杆；只保留预览查看控件。 |
| G2 | 近一年 preview window 缺少稳定来源。 | 横轴只能跟随短 range。 | 前端根据最近可用交易日构造近一年窗口；若后端 range 上限不足，新增 timeline API。 |
| G3 | 后端缺少轻量 timeline contract。 | 直接用 `strategy-preview` 拉一年数据可能 payload 过大。 | 新增 `POST /rearview/strategy-preview/timeline`，只返回 trade date + pool count。 |
| G4 | `pool-page` 分页大小与设计不一致。 | 用户一次看到 50 行，破坏目标密度。 | Step 3 固定请求 `limit = 10`，后端通用 limit 可保留。 |
| G5 | MA 请求参数未接入。 | 后端可能不返回所需 MA 窗口，前端无法稳定渲染。 | `security-analysis` 请求显式携带 `ma_windows = "5,10,30"`。 |
| G6 | MA 图表渲染缺失。 | 趋势线按钮看似可用但不改变图表。 | 在 `CandlestickChart` 中为 MA5/MA10/MA30 增加 line series。 |
| G7 | 复权切换缺少可观测验收。 | 切换失败时容易被误认为数据没变化。 | 图表数据按 adjustment 更新，并用测试/浏览器验收校验 OHLC 有差异。 |
| G8 | Step 1 指标列缺少 condition-level 数据模型。 | 当前只能展示混合 `selected_metrics`。 | 前端先用 applied condition mapping + metric value 展示；后端补 `condition_hits` 后切换为权威命中。 |
| G9 | `output_metrics` 同时承载筛选指标和评分指标。 | 表格指标列混入 Step 2 权重指标。 | 拆分 presentation：`filterMetricRows` 来自 Step 1，`scoreItems` 来自 Step 2。 |
| G10 | `security-analysis` 响应包含 debug 字段但 UI 不该展示。 | Step 3 被误用为解释页。 | 前端只消费 chart、selected quote 和基础行情；不渲染 result snapshot、selected metrics、raw values。 |
| G11 | 现有测试覆盖的是扩张后的 Step 3。 | 修正后可能缺少职责边界回归测试。 | 增加组件测试和 Playwright 验收，断言控件不存在、分页 10、MA/复权有效。 |
| G12 | RFC 0026 和架构事实文档仍描述扩张版 Step 3。 | 后续 agent 可能按旧文档再次实现偏。 | 实施完成后更新 RFC 0026、architecture/racingline、architecture/rearview 和 job report。 |

## 填补原则

1. 先收缩职责，再补能力：先移除错误 UI，再实现 timeline、MA 和指标语义。
2. 保留真实接口：不因为收缩 UI 而恢复 mock 或前端本地计算。
3. 用户不可见参数内聚：日期范围、row limit、lookback 和 page size 由代码策略控制，不作为 Step 3 主控件。
4. Step 1/Step 2 语义分离：筛选指标和评分得分项在展示模型中分开。
5. Chart 行为可验证：复权和 MA 都必须有自动测试或浏览器验收样本。
6. 后端 contract 向轻量化收敛：近一年横轴优先返回 timeline，不拉取每个交易日全量 signals。

## 实施阶段

### 阶段 1：Step 3 UI 职责收缩

目标：先把页面恢复为股池预览结果检查页，移除越权控件。

任务：

1. 修改 `PoolPreviewPanel`：
   - 移除开始日期输入。
   - 移除结束日期输入。
   - 移除展示行数输入。
2. 修改 `StockPoolPreviewWorkbench` / `KeyDataPanel`：
   - 移除“命中指标”区块。
   - 移除“原始值”区块。
   - 移除 `WeightControlSection` 和 Step 3 权重滑杆。
3. 调整 props：
   - 移除 Step 3 不再需要的 `draftWeightIndicators`、`onDraftWeightScoreChange` 等编辑型 prop。
   - 保留 applied weight labels 仅用于得分项展示。
4. 保持 stale 行为：
   - 修改 Step 1/Step 2 后仍标记 preview stale。
   - Step 3 不提供本地编辑入口。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

完成标准：

1. Step 3 页面不存在开始日期、结束日期和展示行数输入。
2. Step 3 右侧不存在“命中指标”“原始值”“指标权重”。
3. Step 3 不能直接修改 Step 2 权重。
4. Step 1/Step 2 修改后 stale gate 仍生效。

### 阶段 2：分页收敛到 10 只一页

目标：让股票池分页密度符合设计预期。

任务：

1. 将 `stock-pool-preview-workbench.tsx` 中的 `pageSize` 从 50 改为 10。
2. 确认 `useStrategyPreviewPoolPageQuery()` 请求参数固定传 `limit = 10`。
3. 更新分页文案/范围计算，确保显示范围按 10 条递增。
4. 增加测试或组件断言，覆盖 next/previous offset 变化。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
```

浏览器验收：

1. network 中 `POST /rearview/strategy-preview/pool-page` 请求 `limit = 10`。
2. 表格每页最多 10 行。
3. 下一页 offset 为 10、20、30 递增。

完成标准：

- Step 3 不再出现 50 条分页。

### 阶段 3：近一年 preview timeline contract

目标：让 K 线下方横轴展示近一年交易日股票池概览，同时避免一次拉取过大 signals payload。

任务：

1. 评估当前 `POST /rearview/strategy-preview` 的 range 限制和一年数据返回体积。
2. 推荐新增 `POST /rearview/strategy-preview/timeline`：
   - 请求：`rule`、`start_date`、`end_date`。
   - 响应：`preview_id`、`start_date`、`end_date`、`trade_dates[]`。
   - `trade_dates[]` 只包含 `trade_date` 和 `pool_count`。
3. Rearview planner 增加 timeline query：
   - 复用 `RuleVersionSpec` 编译和 pool filter。
   - 按交易日聚合 `count(*) AS pool_count`。
   - 不返回 `score_breakdown`、`selected_metrics`、`raw_values` 和 signals。
4. 前端新增 timeline hook 和 query key。
5. `PreviewSnapshot` 增加 timeline 数据，横轴从 timeline 渲染。
6. 点击横轴日期后调用 `pool-page limit=10` 拉取该日候选股。
7. 如 timeline API 暂缓，必须至少把 `strategy-preview` 改为近一年轻量模式，不能返回每个交易日大量 rows。

测试策略：

```bash
cd engines
cargo fmt --check
cargo test -p rearview-core strategy_preview

cd ../app/racingline_new
npm test
npm run typecheck
```

完成标准：

1. 横轴覆盖近一年交易日。
2. 横轴 pool count 来自 Rearview。
3. 横轴不再受短期 `defaultPreviewRange` 限制。
4. 大候选池不会把一年内所有 signals 传回浏览器。

### 阶段 4：近一年 K 线、复权和 MA 叠加

目标：让图表控件和后端数据 contract 真正闭环。

任务：

1. 前端 `security-analysis` 请求固定：
   - `lookback_trading_days = 240`。
   - `ma_windows = "5,10,30"`。
   - `adjustment` 使用当前复权选择。
2. 更新 TS 类型：
   - 补齐 `chart.ma`、`chart.price_overlays` 等后端已返回但前端类型缺失的 metadata。
3. 修改 `CandlestickChart`：
   - 保留 candlestick series。
   - 为 MA5、MA10、MA30 添加 line series。
   - line series 数据来自 `chart.series[].ma` 或 `price_overlays` 中对应字段。
4. 处理 MA 可用性：
   - 如果后端标记 MA 只支持前复权，则非前复权模式下禁用 MA 开关或展示不可用状态。
   - 不允许按钮可点但图表不变。
5. 增加图表单元测试或浏览器像素/DOM 验收：
   - 切换复权后请求参数和图表数据变化。
   - 开关 MA 后 line series 增删。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run build
```

浏览器验收：

1. `security-analysis` 请求包含 `lookback_trading_days = 240`。
2. `security-analysis` 请求包含 `ma_windows = 5,10,30`。
3. 复权切换触发新的 Rearview 请求。
4. MA5/MA10/MA30 开关能改变图表上的折线。

完成标准：

- K 线默认近一年，复权和趋势线控件均有效。

### 阶段 5：Step 1 指标列与 Step 2 得分项拆分

目标：修正表格“指标”和“得分项”的来源语义。

任务：

1. `PreviewSnapshot` 增加 applied Step 1 condition mapping：
   - `conditionId`
   - `groupId`
   - `path`
   - `metric`
   - `label`
   - `operator`
2. `buildPreviewPresentation()` 输出拆分字段：
   - `scoreItems`：只来自 Step 2 `score_breakdown` 和 applied weight labels。
   - `filterMetricRows`：只来自 Step 1 condition mapping 对应的 metric values。
3. 修改表格：
   - “得分项”列只渲染 `scoreItems`。
   - “指标”列只渲染 `filterMetricRows`。
4. 短期实现：
   - 继续从 row 的 metric value JSON 中取值，但只取 Step 1 condition mapping 中的 metric。
   - 不把该列命名或解释为后端 boolean 命中。
5. 完整实现：
   - Rearview preview row 增加 `condition_hits`。
   - `condition_hits` 返回每个 Step 1 condition 的 `matched`、left/right value、operator 和 label。
   - 前端优先用 `condition_hits` 渲染指标列。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck

cd ../engines
cargo test -p rearview-core strategy_preview
```

关键测试用例：

1. Step 2 权重项只出现在得分项列。
2. Step 1 筛选指标只出现在指标列。
3. Step 2 独有指标不会混入指标列。
4. 未识别 score breakdown key 只作为 fallback，不影响正常映射。

完成标准：

- 表格得分项和指标列分别对应 Step 2 与 Step 1，不再混用 `selected_metrics` 整包。

### 阶段 6：Mock 回归防线和错误态

目标：确保修正过程中不恢复 mock 成功路径。

任务：

1. 审计 `/strategies` 成功路径引用：
   - 不允许使用 `catalog.ts` 作为真实 success fallback。
   - 不允许本地生成股票池、排名、K 线或行情。
2. 保留明确状态：
   - metrics loading/error。
   - preview loading/error/empty。
   - timeline loading/error/empty。
   - pool-page loading/error/empty。
   - security-analysis loading/error/empty。
3. 断开 Rearview 时：
   - Step 1/2/3 不展示 mock 成功。
   - Step 3 不展示假 K 线。
4. 增加 Playwright negative smoke 记录。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
npm run build
```

完成标准：

- Rearview 不可用时，页面只进入 loading/error/empty，不出现真实结果外观。

### 阶段 7：浏览器验收和文档回写

目标：把漂移修正沉淀为可复查事实，避免 RFC 0026 的扩张表述继续误导后续实现。

任务：

1. 浏览器验收：
   - 默认打开 `/strategies`。
   - 点击股池预览。
   - 检查 Step 3 控件收缩。
   - 检查近一年横轴。
   - 检查 10 条分页。
   - 检查复权切换。
   - 检查 MA5/MA10/MA30。
   - 检查得分项/指标列来源。
   - 检查断开 Rearview 的失败态。
2. 更新 RFC 0026：
   - 将 Step 3 主路径从 analysis/debug 收缩为股池预览。
   - 将 selected metrics/raw values 定义为 backend diagnostics，不作为主 UI 面板。
3. 更新 `docs/architecture/racingline.md`：
   - 删除 Step 3 展示 raw values、完整 debug analysis 的职责描述。
   - 增加 Step 3 近一年股池预览、10 条分页、K 线复权和 MA 叠加边界。
4. 如新增 timeline 或 condition_hits API，更新 `docs/architecture/rearview.md`。
5. 新增 job report：
   - 记录命令。
   - 记录 API samples。
   - 记录浏览器观察。
   - 记录无法验证项或后续限制。
6. 完成后归档本计划并更新 `docs/plans/README.md`。

验证命令：

```bash
make docs-check
git diff --check
```

完成标准：

- 文档、实现和浏览器验收对 Step 3 职责给出一致结论。

## 禁止模式

1. 禁止在 Step 3 重新加入开始日期、结束日期或展示行数主控件。
2. 禁止在 Step 3 调整 Step 2 权重。
3. 禁止把 `selected_metrics` 整包渲染成“命中指标”面板。
4. 禁止用 mock K 线、mock 行情、mock 股票池或 mock 排名兜底真实接口失败。
5. 禁止把 Step 2 权重指标混入 Step 1 指标列。
6. 禁止把 Step 3 的分页大小开放给用户配置。
7. 禁止把 Step 3 preview result 当成后续回测或建仓的权威历史结果。

## 允许保留的例外

1. `strategy-preview/security-analysis` 可以继续返回 `result_snapshot`、`selected_metrics` 和 `raw_values`，但 Step 3 主 UI 不展示这些 debug 字段。
2. `pool-page` 后端可以继续支持通用 `limit`，但 Step 3 前端固定传 10。
3. `PreviewSnapshot` 可以继续保存 diagnostics，用于排错和 job report，但不作为主 UI 面板。
4. 后端未提供 `condition_hits` 前，前端可以用 Step 1 condition mapping + metric value 展示筛选指标值，但不得宣称为 boolean 命中。

## 最小验证命令

前端：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

Rearview：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

文档：

```bash
make docs-check
git diff --check
```

## 完成标准

1. `/strategies` Step 3 不展示开始日期、结束日期和展示行数。
2. Step 3 不展示“命中指标”“原始值”或权重滑杆。
3. K 线默认近一年。
4. 除权、前复权、后复权切换真实改变图表数据。
5. MA5、MA10、MA30 开关真实控制图表叠加线。
6. K 线下方横轴展示近一年交易日股票池概览。
7. 股票池分页固定 10 只一页。
8. 得分项只来自 Step 2 权重配置。
9. 指标列只来自 Step 1 指标组。
10. Step 1/2/3 成功状态全部来自 Rearview 真实接口。
11. RFC 0026、systems 文档和 job report 已同步修正后的 Step 3 边界。
