# RFC 0026: Racingline 股池预览 Step 3 实现方案

状态：Archived（2026-06-25；归档前状态：Implemented（2026-06-22））
领域：racingline, rearview
关联系统：racingline, rearview
代码根：app/racingline_new/, app/racingline/, engines/crates/rearview-core/, pipeline/elt/
系统地图：docs/systems/racingline.md

路径说明：本文写于 Plan 0053 迁移前；文中的 `app/racingline_new/` 均为历史实现路径，当前 Racingline 前端代码根为 `app/racingline/`。

## 摘要

本文档定义 `/strategies` Step 3「股池预览」的业务边界、现有资源、实施缺口和补齐方案。

实现状态：已按 [Plan 0047](../../plans/archive/0047-racingline-strategy-pool-preview-step3-implementation-plan.md) 落地，按 [Plan 0048](../../plans/archive/0048-racingline-strategy-step3-drift-remediation-plan.md) 完成首次漂移修正，并按 [Plan 0049](../../plans/archive/0049-racingline-strategy-step3-drift2-remediation-plan.md) 完成二次漂移修正。验收见 [2026-06-22 Step3 Preview report](../../jobs/reports/2026-06-22-racingline-strategy-step3-preview.md)、[2026-06-22 Step3 Drift Remediation report](../../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md) 和 [2026-06-22 Step3 Drift2 Remediation report](../../jobs/reports/2026-06-22-racingline-strategy-step3-drift2-remediation.md)。

完整流程仍按 RFC 0025 修正后的语义：

```text
Step 1 策略选股
  记录股票池筛选条件 pool_filters
  不真正生成股票池

Step 2 权重配置
  记录候选池内个股评分规则 scoring.rules
  不真正打分和排名

点击「股池预览」
  组合 Step 1 + Step 2 草稿
  执行选股、评分和排名
  进入 Step 3 查看结果

Step 3 股池预览
  展示一次 preview execution 的结果快照
  支持检查候选池规模、评分排名、Step 2 得分项和 Step 1 指标列
  展示动态近一年交易日股池概览、交易板块、K 线、成交量柱和选中股票行情上下文
  允许微调 Step 2 权重草稿，但必须点击「更新股池」重新 preview 后才替换 applied snapshot
```

Step 3 不是新的规则编辑阶段，也不是正式 run、回测或组合运行。它是用户在进入模拟建仓前，对 Step 1 筛选条件和 Step 2 评分规则的一次真实执行检查。

当前 Plan 0046 已经落地最小链路：`POST /rearview/strategy-preview` 能执行完整 `RuleVersionSpec`，`app/racingline_new` 能展示按交易日组织的 `pool_count`、ranked preview rows、score 和 `score_breakdown`。Plan 0047 补齐了 preview snapshot、全池分页和 preview security analysis；Plan 0048 进一步把 Step 3 主 UI 收缩为“股池预览结果检查页”，不再展示 raw debug 面板；Plan 0049 修正过度收缩，恢复 Step 3 内的权重微调草稿能力，同时保持 applied/draft 分离。

## 背景

[Q&A 0004](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md) 定义了从看板进入 `/strategies` 的主流程：策略选股、权重配置、股池预览、模拟建仓、策略回测和运行策略。

[RFC 0024](0024-racingline-strategy-selection-step1.md) 已明确 Step 1 只生成 `RuleVersionSpec.pool_filters` 草稿，并通过 `POST /rearview/explain` 做编译校验。

[RFC 0025](0025-racingline-strategy-weight-configuration-step2.md) 已明确 Step 2 只生成 `RuleVersionSpec.scoring.rules` 草稿，点击「股池预览」时才执行真实选股、评分和排名。对应实现报告见 [2026-06-22 Step2 Preview report](../../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md)。

这意味着 Step 3 的任务不再是“是否要执行 preview”，而是要回答：

> 当前规则实际命中了哪些股票，为什么这些股票排在前面，这批结果是否足够可靠，可以继续进入模拟建仓？

## 业务流程定义

### Step 3 的输入

Step 3 的输入必须来自一次成功的 preview execution：

```text
selectionDraft + weightDraft + previewRange
  -> buildStrategyPreviewRuleSpec()
  -> POST /rearview/strategy-preview/timeline
  -> POST /rearview/strategy-preview
  -> StrategyPreviewTimelineResponse + StrategyPreviewResponse
  -> Step 3 applied preview snapshot
```

其中：

- `selectionDraft` 来自 Step 1 当前筛选条件。
- `weightDraft` 来自 Step 2 当前权重规则。
- `previewRange` 是前端内部 preview window，当前固定为近一年横轴窗口；Step 3 不向用户暴露开始日期、结束日期或展示行数输入。

Step 3 的输入只限于筛选草稿、评分草稿和内部预览窗口。建仓参数不进入本阶段。

### Step 3 的输出

Step 3 输出的不是正式策略，也不是可交易组合，而是一个 applied preview snapshot：

```text
PreviewSnapshot {
  preview_id
  applied_rule_spec
  start_date
  end_date
  trade_dates[]
  selected_trade_date
  selected_security
  stale
}
```

后续「模拟建仓」应读取这个 snapshot 中的 `applied_rule_spec` 和预览结果摘要，作为用户已经检查过规则命中情况的上下文。真正的回测仍应由后续 backtest execution 按规则重新执行，不应把 Step 3 的短期 preview response 当成长期历史结果。

### Step 3 的状态边界

Step 3 必须区分两类状态：

| 状态 | 含义 | 行为 |
|---|---|---|
| Draft rule | 用户正在编辑的 Step 1/Step 2 草稿 | 修改后标记 preview stale，不直接改变已展示结果 |
| Applied preview | 上一次点击「股池预览」或「更新股池」成功执行的规则快照 | Step 3 表格、分数解释和后续模拟建仓的规则上下文都基于它 |

修改 Step 1 条件或 Step 2 权重后，已有结果必须进入 stale 状态。只有再次执行 preview 成功后，stale 才能清除。

## 目标

1. 明确 Step 3 是一次 preview execution 的结果检查页，不是 Step 1/2 的实时联动计算页。
2. 盘点当前前端、Rearview、数据层和旧 Racingline 中可复用资源。
3. 列出 Step 3 从最小实现到可用工作流之间的缺口。
4. 设计补齐方案，覆盖结果 contract、股票显示信息、分数解释、关键指标、K 线/个股上下文、stale 状态和进入模拟建仓的 gate。
5. 保持 preview-only 边界：不创建 rule set、rule version、run、portfolio run 或回测结果。
6. 保证 Step 1、Step 2、Step 3 落地后都使用 Rearview 真实接口数据，不保留 mock 成功路径。
7. 为后续实施计划提供可拆分阶段和验收标准。

## 非目标

1. 不在 Step 3 发布正式策略或持久化 rule version。
2. 不在 Step 3 发起正式 `POST /rearview/runs`、回测 run 或 portfolio run。
3. 不在浏览器内计算权威股票池、分数、排名、成交、持仓、费用、滑点或净值。
4. 不把 Step 3 preview response 当成后续回测的权威历史数据源；回测必须重新执行规则。
5. 不引入新的评分归一化、仓位优化或交易撮合规则。
6. 不在本 RFC 中调整 Step 1、Step 2、Step 3 的前端可见文案；如需改文案，应另起 UI 文案设计。
7. 不把 `app/racingline_new/src/features/strategy/catalog.ts` 重新作为真实数据来源。
8. 不允许 Step 1、Step 2 或 Step 3 在 Rearview 接口不可用时用 mock 数据伪造成成功状态。

## 真实接口数据落地约束

Step 3 实施完成后，`/strategies` 的前三步必须全部使用 Rearview 真实接口数据。mock 数据只能用于单元测试 fixture、Storybook-like 开发样例或明确标记的离线原型，不得出现在用户可操作成功路径中。

| 步骤 | 用户可见数据 | 必须使用的真实来源 | 禁止模式 |
|---|---|---|---|
| Step 1 策略选股 | 指标类型、指标名、操作符、中文 label、规则校验结果 | `GET /rearview/metrics` 和 `POST /rearview/explain` | 使用静态 `catalog.ts` 作为成功 fallback；接口失败时仍展示可提交的 mock 指标 |
| Step 2 权重配置 | 可评分指标、操作符、中文 label、scoring rule adapter 输入 | `GET /rearview/metrics` 中 `allow_scoring = true` 的真实 catalog | 使用静态 scoring catalog 或本地假指标生成可执行 scoring rules |
| Step 3 股池预览 | 近一年交易日股池数量、rank、score、Step 2 得分项、Step 1 指标列 | `POST /rearview/strategy-preview/timeline`、`POST /rearview/strategy-preview` 和 `POST /rearview/strategy-preview/pool-page` 返回的真实 preview response | 本地生成股票池、mock 分数、mock 排名或 mock score breakdown |
| Step 3 个股上下文 | 证券名称、近一年 K 线、复权 OHLC、MA5/MA10/MA30、行情和估值字段 | security display lookup 和 preview security analysis 真实接口 | 固定股票、mock K 线或 mock 行情快照 |

实现规则：

1. Rearview 接口 loading 时展示 loading 状态；接口失败时展示 error 状态；接口返回空列表时展示 empty 状态。
2. 不得把 mock 数据作为接口失败后的 success fallback。
3. `app/racingline_new/src/features/strategy/catalog.ts` 只能作为测试 fixture 或明确的 prototype-only 开发材料，不能被 `/strategies` 的正式成功路径引用。
4. 浏览器验收必须检查 Step 1/2/3 的 network 请求来自 Rearview，并确认断开 Rearview 后不会出现 mock 成功结果。
5. 实施计划必须包含一项显式任务：移除或隔离 Step 1/2/3 中仍参与用户成功路径的 mock 数据。

## Plan 0049 后的 Step 3 主界面边界

[Debt 0004](../../debt/archive/0004-2026-06-22-strategies-step3-implemennt-drift.md) 确认 RFC 0026 和 Plan 0047 的后半段把 Step 3 扩张成了“候选池分页 + 个股 analysis + 指标解释 + debug 展示”的复合页面。Plan 0048 已修正首次漂移。[Debt 0005](../../debt/archive/0005-2026-06-22-strategies-step3-implemennt-drift2.md) 又确认 Plan 0048 过度删除了 Step 3 权重微调、动态窗口、量柱和交易板块展示。Plan 0049 后，本节覆盖当前边界。

当前 Step 3 主界面只承担以下职责：

1. 展示一次真实 preview execution 的 applied snapshot。
2. 用 `POST /rearview/strategy-preview/timeline` 展示按当前日期动态计算的近一年交易日股池概览。
3. 用 `POST /rearview/strategy-preview/pool-page` 按选中交易日分页展示候选股，前端固定 `limit = 10`。
4. 表格“得分项”只来自 Step 2 `score_breakdown` 与 applied weight labels。
5. 表格“指标”只来自 Step 1 condition mapping 对应的筛选指标值；在 Rearview 补齐 `condition_hits` 前，不把该列宣称为 boolean 命中。
6. 用 `POST /rearview/strategy-preview/security-analysis` 展示选中股票近一年 K 线、复权 OHLC、前复权基准 MA5/MA10/MA30、成交量柱、行情和估值字段。
7. 在右侧行情/估值下方允许微调 Step 2 权重草稿；修改只标记 preview stale，不改写当前表格排名和得分项。
8. 点击「更新股池」后重新执行 timeline、单日 preview、pool-page 和 security-analysis；成功后新的权重草稿成为 applied snapshot。

当前 Step 3 主界面禁止：

1. 展示开始日期、结束日期或展示行数输入。
2. 展示“命中指标”或 `raw_values` debug 面板。
3. 权重修改后在浏览器本地重算排名、得分或股池。
4. 把 Step 2 权重指标混入 Step 1 指标列。
5. 用 mock 股池、mock 排名、mock K 线或 mock 行情兜底真实接口失败。
6. 用 `exchange_code` 伪装交易板块；交易板块字段是 `security_board`，不是行业、概念或地域字段。

`security-analysis` 响应仍可保留 `result_snapshot`、`selected_metrics` 和 `raw_values` 作为 backend diagnostics，但 `/strategies` Step 3 主 UI 不渲染这些 debug 字段。后续如需要完整指标解释页，应另起 run result 或 diagnostics 设计，不回填到 Step 3 主流程。

## 当前资源盘点

### 前端资源

| 资源 | 路径 | 当前能力 | Step 3 价值 | 缺口 |
|---|---|---|---|---|
| Step 流程和 preview 状态 | `app/racingline_new/src/routes/strategy-page.tsx` | 已有 `lastPreviewRuleSpec`、`lastPreviewResult`、`lastPreviewAt`、`isPreviewStale` 和 `openPreview()` | 能表达 applied preview 与 draft 之间的关系 | `preview_id`、applied rule、range 和后续阶段 gate 还没有形成独立 snapshot 模型；当前展示行数状态需要重命名为 preview row limit |
| Preview API hook | `app/racingline_new/src/api/hooks.ts` | `useStrategyPreviewMutation()` 调用 Rearview preview API | Step 3 真实数据入口 | 失败、stale、空结果和超范围错误还只是通用提示 |
| Preview 类型 | `app/racingline_new/src/types/rearview.ts` | 定义 `StrategyPreviewRequest/Response` | 可直接消费 `score_breakdown`、`selected_metrics` 和 `raw_values` | signal 缺少 `security_name`、解释标签和分页元数据 |
| Preview panel | `app/racingline_new/src/features/strategy/components/pool-preview-panel.tsx` | 支持日期范围、展示行数、pending/error/stale/empty 状态 | Step 3 页面容器已存在 | 展示行数需要按 preview row limit 处理；Step 3 还缺 preview 摘要、applied rule 摘要、结果元信息和继续模拟建仓 gate |
| Preview workbench | `app/racingline_new/src/features/strategy/components/stock-pool-preview-workbench.tsx` | 能把真实 preview response 转成按交易日展示的股票池表格 | 已展示日期、pool count、rank、score 和 score items | 仍混用 mock K 线、mock 股票名称和 mock 行情快照；未展示 selected metrics 和 raw values |
| Scoring adapter | `app/racingline_new/src/features/strategy/adapters.ts` | 生成稳定 scoring rule name，例如 `weight:<id>:<index>` | 可把 score breakdown 映射回 Step 2 权重项 | Step 3 映射逻辑散落在 workbench 内，还没有可复用 result presenter adapter |
| 旧 run result 表格 | `app/racingline/src/features/runs/components/run-results.tsx` | 展示 pool/signals、动态 selected metrics 列、分页和详情入口 | 可复用表格信息结构和动态指标列思路 | 依赖正式 run_id，不适合直接用于 preview-only |
| 旧 signal detail sheet | `app/racingline/src/features/runs/components/signal-detail-sheet.tsx` | 展示 rank、score、score_breakdown、selected_metrics | 可复用分数解释展示方式 | 语义是 PostgreSQL run snapshot，需要改成 Preview snapshot |
| 旧个股分析页 | `app/racingline/src/routes/SecurityAnalysisPage.tsx` | 展示 run result membership 下的 K 线、MA、指标和 result snapshot | Step 3 需要类似的个股上下文 | 当前 analysis API 只接受正式 run membership，不支持 preview-only |

### Rearview 后端资源

| 资源 | 路径 | 当前能力 | Step 3 价值 | 缺口 |
|---|---|---|---|---|
| Preview route | `engines/crates/rearview-core/src/api/mod.rs` | `POST /rearview/strategy-preview` 执行完整 `RuleVersionSpec` | Step 3 真实执行入口 | 不持久化 preview，不支持基于 `preview_id` 的后续分页或个股分析 |
| Preview response builder | `engines/crates/rearview-core/src/api/mod.rs` | 按交易日聚合 rows，返回 `pool_count` 和 ranked preview rows | 已能支撑首屏排名检查 | 只返回证券代码，没有证券简称、分页和全池浏览；现有展示行数参数命名不够准确 |
| Planner | `engines/crates/rearview-core/src/planner/sql.rs` | 生成 pool、raw_score、clamp score、rank、score_breakdown、selected_metrics、raw_values | Step 3 结果解释的权威来源 | SQL 返回全池行给 API，再由 API 裁剪展示行数；多日大池可能有传输和内存压力 |
| Metric catalog | `engines/crates/rearview-core/config/metric_policy.yml` | 定义 default output、中文标签、allow_filter/allow_scoring | 可把 selected metrics 展示成中文指标 | Preview response 中只有 metric id/value，没有 display label 快照 |
| Security analysis API | `engines/crates/rearview-core/src/api/mod.rs` | `GET /rearview/runs/{run_id}/securities/{security_code}/analysis` | 已有 K 线和指标查询经验 | 只接受 run membership；Step 3 preview 没有 run_id |
| ClickHouse query helpers | `engines/crates/rearview-core/src/clickhouse/mod.rs` | 已有 screening rows、analysis quote/trend/momentum 查询 | 可复用查询能力 | 没有 preview security display lookup 和 preview membership analysis helper |

### 数据层资源

| 资源 | 路径 | 当前能力 | Step 3 价值 | 缺口 |
|---|---|---|---|---|
| 行情 mart | `pipeline/elt/models/marts/mart_stock_quotes_daily.sql` | 提供价格、成交量、复权价格和涨跌幅等字段 | selected metrics、K 线和右侧行情上下文 | Preview workbench 当前未用真实行情数据驱动 K 线 |
| 趋势/动量/量能/形态 mart | `pipeline/elt/models/marts/` | 提供 MA、BOLL、MACD、RSI、KDJ、量能和价格结构指标 | 分数解释和个股上下文 | Step 3 尚未按 selected security 拉取这些指标 |
| 股票基础信息 snapshot | `pipeline/elt/models/intermediate/int_stock_basic_snapshot.sql` | 有 `security_code`、`security_name`、交易所和上市状态等当前快照 | 能补足证券名称和基础显示信息 | 位于 intermediate 层；Rearview 当前不应直接把它当作 marts API 数据源 |

### 文档和验收资源

| 资源 | 路径 | 当前价值 |
|---|---|---|
| Q&A 0004 | `docs/Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md` | 定义 Step 3 在完整策略创建闭环中的位置 |
| RFC 0024 | `docs/RFC/archive/0024-racingline-strategy-selection-step1.md` | 明确 Step 1 只记录筛选条件 |
| RFC 0025 | `docs/RFC/archive/0025-racingline-strategy-weight-configuration-step2.md` | 明确 Step 2 只记录评分规则，preview 才执行 |
| Plan 0046 | `docs/plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md` | 已归档 Step 2 到真实 preview 的实施事实 |
| Step2 report | `docs/jobs/reports/2026-06-22-racingline-strategy-step2-preview.md` | 记录 live smoke 和浏览器验收，证明最小 preview 链路已通 |
| RFC 0020 | `docs/RFC/archive/0020-racingline-run-result-security-analysis-page.md` | 提供 run result 个股分析页的数据边界和 API 约束，可作为 preview analysis 的设计参考 |

## 实施缺口与填充方案

| 缺口 ID | 缺口 | 影响 | 填充方案 |
|---|---|---|---|
| G1 | Step 3 没有独立 PreviewSnapshot 模型。 | 页面状态依赖分散的 `lastPreview*` 变量，后续阶段难以判断是否可继续。 | 引入前端层 `PreviewSnapshot`：包含 `preview_id`、applied rule、range、result、created_at 和 stale 标记；后续阶段只消费非 stale snapshot。 |
| G2 | 结果语义混用“候选池”和“预览行”。 | 用户可能以为表格展示的是完整候选池，或把展示行数误解成后续建仓参数。 | 明确 `pool_count` 表示完整候选池数量，Step 3 表格展示 ranked preview rows；若需要全池浏览，补充单日分页查询 contract。 |
| G3 | 当前 preview response 没有证券名称。 | Step 3 表格只能显示证券代码，原型用代码代替名称，影响检查效率。 | 增加 security display lookup：从 marts 层证券基础快照或 Rearview 同步表返回 `security_name` 和 `exchange_code`；preview response 内嵌 display snapshot。 |
| G4 | Preview workbench 仍使用 mock K 线和行情快照。 | 用户看到的个股上下文可能和真实 preview signal 不一致。 | 新增 preview security analysis contract，复用 run analysis 的 chart/indicator 查询，但 source 标记为 `preview`，并校验该证券属于 applied preview。 |
| G5 | `selected_metrics` 和 `raw_values` 未成为一等展示。 | 用户只能看到总分和部分 score breakdown，难以判断筛选条件和评分条件为何命中。 | Step 3 新增 result presenter adapter：把 `selected_metrics`、`raw_values`、metric catalog label 和 scoring rule label 组合成可展示 rows。 |
| G6 | score breakdown 只按 rule name 粗略映射。 | 权重顺序或规则名变化后，展示容易退化为裸 key。 | 在 preview snapshot 中保留 `weightId -> ruleName -> label` 映射；展示时优先用 snapshot label，缺失时才回退 rule name。 |
| G7 | Preview API 返回全池 rows 到 API 层后再裁剪展示行数。 | 多日大候选池会增加 ClickHouse 到 Rearview 的传输和后端内存压力，也会放大展示行数参数的业务误解。 | Planner 支持 preview mode：首屏只返回页面所需 ranked rows，同时用窗口或聚合返回 `pool_count`；全池分页走单日 detail query。 |
| G8 | `preview_id` 只是调试 ID，不可查询。 | 无法用 `preview_id` 拉取分页、个股分析或复用结果。 | 第一版保持 preview stateless；分页和个股分析请求携带 applied `RuleVersionSpec`。后续如需要性能优化，再引入短期 preview cache。 |
| G9 | Step 3 修改权重或日期后 stale 与 applied result 的差异不够可解释。 | 用户可能继续使用旧结果进入模拟建仓。 | 任何 draft/range 变化只更新草稿并标记 stale；「更新股池」成功后才替换 applied snapshot；模拟建仓按钮必须 gate stale 状态。 |
| G10 | 后续模拟建仓缺少明确输入边界。 | 后续阶段可能错误读取当前草稿，或错误继承 Step 3 的展示行数作为建仓参数。 | 后续阶段只读取 `PreviewSnapshot.applied_rule_spec` 和 preview range/summary 作为上下文；建仓参数不能从 Step 3 继承。 |
| G11 | 错误和空状态粒度不足。 | 用户无法区分规则无命中、日期无数据、后端超范围、catalog 缺失或 ClickHouse 执行失败。 | 规范 Step 3 错误类型：adapter validation、preview validation、empty pool、empty preview rows、backend execution、catalog unavailable。 |
| G12 | Step 3 的测试只覆盖最小链路。 | 后续重构容易回到 mock 或破坏 stale/snapshot 边界。 | 增加 adapter 单测、preview response presenter 单测、React 组件状态测试和 Playwright live smoke。 |
| G13 | Step 1/2/3 仍可能保留 prototype fallback 或 mock 展示路径。 | 用户在接口失败时可能看到假成功结果，误以为策略条件、评分规则或股池预览已经真实执行。 | 实施时显式移除或隔离 mock 成功路径；Step 1/2/3 成功状态都必须由 Rearview 接口数据驱动，接口不可用时只能显示 loading/error/empty。 |

## 设计

### D1: Step 3 使用 applied PreviewSnapshot

前端新增独立的 Step 3 快照模型，不直接让各组件读取散落状态：

```ts
type PreviewSnapshot = {
  previewId: string
  appliedRuleSpec: RuleVersionSpec
  createdAt: string
  range: {
    startDate: string
    endDate: string
  }
  result: StrategyPreviewResponse
  labels: {
    scoringRules: Record<string, string>
    metrics: Record<string, string>
  }
  stale: boolean
}
```

状态规则：

1. 点击「股池预览」成功后创建新的 `PreviewSnapshot`，并进入 Step 3。
2. 点击「更新股池」成功后替换 `PreviewSnapshot`。
3. 修改 Step 1、Step 2 或 Step 3 日期后，只标记 `PreviewSnapshot.stale = true`。
4. Step 3 表格和右侧解释继续展示 applied snapshot，不实时混入 draft。
5. 后续阶段只能从 `stale = false` 的 snapshot 进入；否则必须先重新执行 preview。

### D2: 明确候选池、预览行和全池浏览

Step 3 第一屏的默认展示对象是 ranked preview rows，不一定是全量候选池表格：

```text
trade_date.pool_count = 当日完整候选池数量
trade_date.signals = 当日按 score DESC, security_code ASC 排序后的预览行
```

为了避免语义漂移，前端 adapter 应把后端字段映射为领域名：

```text
StrategyPreviewTradeDate.signals -> rankedPreviewRows
StrategyPreviewTradeDate.pool_count -> candidatePoolCount
```

现有 preview API 的展示行数参数只能作为短期兼容的技术返回行数限制。后续实施应在 Step 3 侧重命名为 `preview_limit` 或用分页 `limit` 替代。

如需要检查完整候选池，新增 stateless 单日分页 contract：

```http
POST /rearview/strategy-preview/pool-page
```

请求：

```json
{
  "rule": {},
  "trade_date": "2026-06-01",
  "limit": 50,
  "offset": 0,
  "sort": "score_desc",
  "security_code": null
}
```

响应：

```json
{
  "trade_date": "2026-06-01",
  "pool_count": 4959,
  "items": [
    {
      "security_code": "600000.SH",
      "security_name": "浦发银行",
      "raw_score": 83.5,
      "score": 83.5,
      "signal_rank": 1,
      "is_buy_signal": true,
      "score_breakdown": {},
      "selected_metrics": {},
      "raw_values": {}
    }
  ],
  "limit": 50,
  "offset": 0,
  "has_more": true
}
```

该 endpoint 不依赖 `preview_id`，也不持久化 preview。它用同一个 `RuleVersionSpec` 在单个 `trade_date` 上重新编译执行，并只返回分页范围。后续如性能需要，可以引入短期 preview cache，但不作为第一版前置条件。

### D3: 补齐证券显示信息

Step 3 表格和个股上下文至少需要：

- `security_code`
- `security_name`
- `exchange_code`

当前可用事实在 `int_stock_basic_snapshot`，但 Rearview 系统边界要求前端通过 Rearview 消费数据，Rearview 不应随意绕过 marts 读取 intermediate 模型。推荐补齐方式：

1. 数据层新增或确认 `fleur_marts.mart_stock_basic_snapshot`，从 `int_stock_basic_snapshot` 暴露稳定展示字段。
2. Rearview 增加 security display lookup helper，按 `security_code IN (...)` 批量查询该 mart。
3. `POST /rearview/strategy-preview` 和 `pool-page` 在返回 rows 前合并 display snapshot。
4. 如果 display lookup 失败，preview 主结果仍可返回，但 `security_name` 回退为 `security_code`，并在 diagnostics 中标记 display partial。

显示字段是上下文信息，不参与筛选、评分或排名，也不进入 rule hash。

### D4: Preview security analysis contract

Step 3 中点击某只股票后，应展示真实 K 线、指标和 preview result snapshot。不能继续使用 mock `previewStock`。

新增 preview-only analysis contract：

```http
POST /rearview/strategy-preview/security-analysis
```

请求：

```json
{
  "rule": {},
  "trade_date": "2026-06-01",
  "security_code": "600000.SH",
  "source": "signals",
  "adjustment": "forward_adjusted",
  "lookback_trading_days": 240
}
```

响应复用 RFC 0020 的主要结构，但去掉 `run_id`：

```json
{
  "source": "preview",
  "trade_date": "2026-06-01",
  "security_code": "600000.SH",
  "security_name": "浦发银行",
  "result_snapshot": {
    "rank": 1,
    "score": 83.5,
    "score_breakdown": {},
    "selected_metrics": {},
    "raw_values": {}
  },
  "chart_window": {
    "start_date": "2025-06-01",
    "end_date": "2026-06-01"
  },
  "chart": {
    "series": [],
    "ma": {},
    "price_overlays": {}
  },
  "quote_rows": [],
  "indicator_sections": []
}
```

Backend 必须校验该 `security_code` 属于当前 rule 在 `trade_date` 的 preview 结果：

- `source = "preview"`：要求满足当前 rule 的 pool filter，返回该证券在当日的 score、rank 和解释快照。
- `source = "pool"`：同样要求满足 pool filter，用于后续完整候选池分页入口。

如果不属于 preview 结果，应返回 validation error 或 404，避免展示与当前规则无关的个股上下文。

### D5: Result presenter adapter

Step 3 新增纯前端 adapter，把 backend response 转成展示模型：

```text
StrategyPreviewResponse
  + MetricDefinition[]
  + WeightIndicator[] applied labels
  -> PreviewPresentation
```

输出：

```ts
type PreviewPresentation = {
  tradeDates: Array<{
    date: string
    candidatePoolCount: number
    rankedPreviewRows: PreviewSignalRow[]
    averagePreviewScore: number
  }>
  metricLabels: Record<string, string>
  scoringLabels: Record<string, string>
}

type PreviewSignalRow = {
  securityCode: string
  securityName: string
  rank: number
  rawScore: number
  score: number
  scoreItems: Array<{ key: string; label: string; points: number }>
  selectedMetrics: Array<{ key: string; label: string; value: JsonValue }>
  rawValues: Array<{ key: string; label: string; value: JsonValue }>
}
```

规则：

1. `score_breakdown` 按 applied scoring rule label 展示。
2. `selected_metrics` 按 metric catalog `display.label_zh` 展示。
3. `raw_values` 默认折叠，仅用于解释和调试。
4. 数值格式按 metric `unit` 和 value kind 格式化。
5. 未识别 key 保留原始 key，不丢弃数据。

### D6: 后续阶段 gate

Step 3 进入模拟建仓必须满足：

1. 有成功的 `PreviewSnapshot`。
2. `PreviewSnapshot.stale = false`。
3. 至少一个交易日有 `candidatePoolCount > 0`。
4. 至少一个交易日有可展示的 ranked preview rows。
5. `PreviewSnapshot.appliedRuleSpec.scoring.clamp` 已固定为 `{ "min": 0, "max": 100 }`。

后续阶段不重新读取 Step 1/2 draft，而是接收：

```text
appliedRuleSpec
previewStartDate
previewEndDate
previewSummary
```

Step 3 只提供已预览的规则和结果上下文，不提供建仓参数。这能避免用户在 Step 3 或返回 Step 2 修改草稿后，后续阶段错误使用未预览规则，也避免把预览展示行数误当成建仓参数。

### D7: Preview diagnostics

Step 3 应把以下诊断信息保存到 snapshot，用于错误排查和复现：

- `preview_id`
- `sql_hash`
- `required_metrics`
- `required_marts`
- `required_columns`
- `start_date`
- `end_date`
- `preview_limit`
- `created_at`

这些信息默认可以折叠，不参与用户主决策，但在 catalog 缺失、数据缺失或执行失败时有助于定位。

### D8: Preview-only 持久化策略

第一版不持久化 preview result。

原因：

1. Step 3 是策略创建过程中的检查动作，不是正式 run result。
2. 当前 `POST /rearview/strategy-preview` 已能 stateless 执行规则。
3. 持久化 preview 会引入清理策略、过期策略和用户隔离问题。
4. 后续回测仍需按回测区间重新执行规则，不能依赖短期 preview result。

允许保留的例外：

- `preview_id` 和 `query_id` 用于进程日志和 ClickHouse 查询追踪。
- 前端内存保存 `PreviewSnapshot`，页面刷新后丢失可以接受。
- 如果后续需要支持刷新恢复或多人协作，再新增短期 preview cache RFC。

## API contract 汇总

| API | 状态 | Step 3 用途 | 是否必须 |
|---|---|---|---|
| `GET /rearview/metrics` | 已存在 | 提供 metric label、unit、default output 和解释映射 | 必须 |
| `POST /rearview/strategy-preview/timeline` | 已存在 | 执行 Step 1 + Step 2，返回近一年 `trade_date + pool_count` 轻量横轴 | 必须 |
| `POST /rearview/strategy-preview` | 已存在 | 执行 Step 1 + Step 2，返回选中日期 ranked preview rows 和解释字段 | 必须 |
| `POST /rearview/strategy-preview/pool-page` | 已存在 | 单日完整候选池分页浏览，Step 3 固定 `limit = 10` | 必须 |
| `POST /rearview/strategy-preview/security-analysis` | 已存在 | 预览结果中单只股票的真实 K 线、复权、MA 和行情上下文 | 必须 |
| `GET /rearview/runs/{run_id}/securities/{security_code}/analysis` | 已存在 | 设计参考，不直接服务 preview-only | 不用于 Step 3 |
| `POST /rearview/runs` | 已存在 | 后续正式 run 或回测阶段使用 | Step 3 不调用 |

## 初步实现路径

### Phase 1: 真实接口基线收敛

目标：保证 Step 1、Step 2、Step 3 在用户成功路径中全部由 Rearview 真实接口数据驱动。

任务：

1. 审计 `/strategies` Step 1 的指标 catalog、操作符和 explain 校验路径，确认成功状态只来自 `GET /rearview/metrics` 和 `POST /rearview/explain`。
2. 审计 Step 2 的 scoring catalog 和 scoring adapter 输入，确认成功状态只来自 `allow_scoring = true` 的真实 metric catalog。
3. 审计 Step 3 的股票池、分数、排名、score breakdown、selected metrics 和 raw values，确认成功状态只来自 `POST /rearview/strategy-preview`。
4. 移除或隔离 `catalog.ts`、本地生成股票池、mock 分数、mock K 线和 mock 行情快照在用户成功路径中的引用。
5. 对接口 loading/error/empty 状态分别保留明确 UI，不用 mock 数据兜底。

完成标准：

- 断开 Rearview 后，Step 1/2/3 都不会展示 mock 成功结果。
- Step 1/2/3 的成功状态都能在 browser network 中对应到 Rearview 请求。
- 静态 fixture 只存在于测试或明确 prototype-only 代码路径。

### Phase 2: PreviewSnapshot 和 presenter adapter

目标：把现有 Step 3 最小链路整理成稳定的前端状态和展示模型。

任务：

1. 新增 `PreviewSnapshot` 类型，收敛 `lastPreviewRuleSpec/Result/At/isPreviewStale`。
2. 新增 `buildPreviewPresentation()`，把 response、metric catalog 和 applied weights 转成展示 rows。
3. 表格展示 `securityCode`、`securityName` fallback、rank、score、score breakdown、selected metrics 摘要。
4. 保留 raw values 为折叠诊断信息。
5. 后续阶段 gate 改为读取非 stale snapshot。

完成标准：

- Step 3 不再从 mock 计算股票池。
- Step 3 明确展示 applied result，即使 draft 已修改也不会静默联动。
- 修改 Step 1/2/日期后，后续阶段 gate 被 stale 状态拦住。

### Phase 3: Security display lookup

目标：让 preview rows 具备可读证券信息。

任务：

1. 确认或新增 marts 层证券基础快照。
2. Rearview 增加按证券代码批量查询 display fields 的 helper。
3. `POST /rearview/strategy-preview` 返回 `security_name` 和 `exchange_code`。
4. 前端去掉用 `security_code` 充当名称的逻辑。

完成标准：

- Step 3 表格可以同时展示证券名称和证券代码。
- display lookup 失败不影响 preview 主结果。
- Rearview 不直接依赖 intermediate 模型作为长期 API 数据源。

### Phase 4: Preview SQL 裁剪和全池分页

目标：降低首屏 preview 传输成本，并支持完整候选池检查。

任务：

1. Planner 支持 ranked preview rows + `pool_count` 的 SQL 形态，避免首屏拉全池。
2. 新增 `POST /rearview/strategy-preview/pool-page`。
3. 前端在 selected trade date 下支持预览行与完整候选池分页视图。
4. 保持排序稳定：`score DESC, security_code ASC`。

完成标准：

- 首屏 preview 返回 ranked preview rows 和 pool count。
- 单日 full pool 可以分页、按代码过滤、按 score 排序。
- 大候选池不会把所有 rows 传回浏览器。

### Phase 5: Preview security analysis

目标：替换 Step 3 中的 mock K 线和右侧行情快照。

任务：

1. 新增 `POST /rearview/strategy-preview/security-analysis`。
2. 复用 RFC 0020 的 chart、quote、trend、momentum 查询结构。
3. 后端校验 selected security 属于 applied preview。
4. 前端点击 Step 3 表格行后加载真实 K 线、行情和指标上下文。

完成标准：

- Step 3 不再展示固定 `贵州茅台` mock K 线。
- 图表和右侧数据来自 selected security + selected trade date。
- 如果个股 analysis 查询失败，表格和 preview result 仍可用。

### Phase 6: 验收和沉淀

目标：把 Step 3 的真实股池预览能力沉淀为可复查事实。

任务：

1. 增加 presenter adapter 单元测试。
2. 增加 Rearview preview pool-page 和 security-analysis 测试。
3. 增加浏览器验收：Step 1/2/3 真实接口请求、断开 Rearview 不出现 mock 成功结果、成功 preview、stale gate、空结果、排名表格、selected metrics、真实 K 线。
4. 增加 job report，记录 API smoke、浏览器观察项和限制。

完成标准：

- Step 3 可以支撑用户判断当前规则是否值得进入模拟建仓。
- 文档、测试和运行报告都能证明 preview-only 边界没有污染正式 run 生命周期。

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

涉及 Rearview preview API、planner 或 analysis 行为变化时追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 metric catalog 或 security display mart 时追加：

```bash
cd engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog coverage
```

若新增 marts 层证券基础快照，按 dbt 变更范围追加：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select <security-basic-mart-selector>
uv run python elt/scripts/validate_field_glossary.py
```

浏览器验收至少检查：

- Step 1 指标类型、指标名和操作符来自 `GET /rearview/metrics`。
- Step 1 校验规则调用 `POST /rearview/explain`，接口失败时不显示 mock 校验成功。
- Step 2 可评分指标来自真实 `allow_scoring = true` catalog，接口失败时不显示静态 mock 权重指标。
- Step 3 候选池数量、rank、score、score breakdown、selected metrics 和 raw values 来自 `POST /rearview/strategy-preview`。
- 断开 Rearview 或指向错误 API base URL 后，Step 1/2/3 都进入 loading/error/empty 状态，不出现 mock 成功结果。
- Step 1/2 修改后 Step 3 preview 标记 stale。
- stale 状态不能直接进入模拟建仓。
- 点击「更新股池」后 stale 清除，表格使用新 applied result。
- Step 3 展示交易日、candidate pool count、rank、score 和 score breakdown。
- selected metrics 使用中文 metric label 展示。
- 证券名称来自真实 display lookup；缺失时回退证券代码。
- 选中股票后 K 线和右侧数据来自真实 preview security analysis，不再是固定 mock。
- 后端 preview 不创建 rule set、rule version、run 或 portfolio run。

## 风险与待决问题

1. Preview security analysis 的 membership 校验会重复执行单日规则；如性能不足，再评估短期 preview cache。
2. Preview result 页面刷新后是否需要恢复，不在第一版范围；如果需要恢复，必须设计 preview cache、过期和用户隔离。
3. Step 3 进入后续阶段当前要求存在非 stale snapshot、至少一个交易日有候选池且至少一个交易日有 ranked preview rows；更细的 Step 4 建仓准入规则仍由后续 RFC 固定。

## 相关文档

- [Q&A 0004: Racingline 原型看板到策略创建闭环用户故事](../../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md)
- [RFC 0020: Racingline Run Result 个股分析页](0020-racingline-run-result-security-analysis-page.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](0023-racingline-frontend-prototype-led-development.md)
- [RFC 0024: Racingline 策略选股 Step 1 实现切入方案](0024-racingline-strategy-selection-step1.md)
- [RFC 0025: Racingline 策略权重配置 Step 2 实现方案](0025-racingline-strategy-weight-configuration-step2.md)
- [Plan 0046: Racingline 策略权重配置 Step 2 实施计划](../../plans/archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md)
- [Plan 0047: Racingline 股池预览 Step 3 实施计划](../../plans/archive/0047-racingline-strategy-pool-preview-step3-implementation-plan.md)
- [Plan 0048: Racingline Step 3 股池预览漂移修正实施计划](../../plans/archive/0048-racingline-strategy-step3-drift-remediation-plan.md)
- [Plan 0049: Racingline Step 3 股池预览二次漂移修正实施计划](../../plans/archive/0049-racingline-strategy-step3-drift2-remediation-plan.md)
- [Racingline Strategy Step 2 Preview Implementation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md)
- [Racingline Strategy Step 3 Preview Implementation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step3-preview.md)
- [Racingline Strategy Step 3 Drift Remediation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md)
- [Racingline Strategy Step 3 Drift2 Remediation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step3-drift2-remediation.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
