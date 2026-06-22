# Debt 0004: Strategies Step 3 股池预览实现漂移

状态：Resolved（2026-06-22）
日期：2026-06-22
领域：racingline, rearview
关联代码：`app/racingline_new/`, `engines/crates/rearview-core/`
关联设计：`docs/Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md`, `docs/RFC/0024-racingline-strategy-selection-step1.md`, `docs/RFC/0025-racingline-strategy-weight-configuration-step2.md`, `docs/RFC/0026-racingline-strategy-pool-preview-step3.md`
修正计划：`docs/plans/archive/0048-racingline-strategy-step3-drift-remediation-plan.md`
验收报告：`docs/jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md`

## 摘要

`/strategies` Step 3 当前实现与股池预览的设计预期发生明显偏移。偏移不是单点 bug，而是 RFC 0026 和 Plan 0047 在落地时把 Step 3 从“用户检查 Step 1 + Step 2 执行结果的股池预览”扩展成了“候选池分页 + 个股 analysis + 指标解释 + 局部权重调整”的复合页面。

需要保留的正确方向：

1. Step 1 只记录股票池筛选条件。
2. Step 2 只记录候选池内评分规则。
3. 只有点击股池预览时，Rearview 才执行选股、评分和排名。
4. Step 3 只展示这次执行后的股池预览结果，帮助用户判断规则是否命中了预期股票。
5. Step 3 不接管 Step 1/2 编辑职责，不引入建仓参数，也不展示 mock 成功数据。

当前需要纠偏的核心方向：

1. 收缩 Step 3 UI，不暴露开始日期、结束日期和展示行数。
2. K 线默认展示近一年，并让复权切换和 MA5/MA10/MA30 趋势线真正生效。
3. 右侧不请求、不展示“命中指标”详情面板。
4. K 线下方的横轴展示近一年交易日维度的股票池，而不是短日期区间。
5. 股票池分页固定为 10 只一页。
6. 股票池“得分项”只来自 Step 2 权重配置。
7. 股票池“指标列”只来自 Step 1 指标组的真实筛选命中，不混入 Step 2 权重指标或 raw debug values。

## 设计基线

### 业务流程基线

按 Q&A 0004、RFC 0024 和 RFC 0025 修正后的流程：

```text
Step 1 策略选股
  用户记录股票池筛选条件
  不真正生成股池

Step 2 权重配置
  用户记录候选池内评分规则
  不真正评分和排名

点击股池预览
  组合 Step 1 + Step 2 草稿
  执行真实选股、评分和排名

Step 3 股池预览
  展示执行结果
  检查命中的股票、评分排序和关键行情上下文
  不继续编辑筛选/评分规则
```

### Step 3 页面预期

Step 3 应是一屏预览页，而不是规则编辑页或 debug 页：

1. 顶部或主区展示选中股票的近一年 K 线。
2. K 线支持除权、前复权、后复权切换；切换后蜡烛图价格序列必须变化。
3. K 线支持 MA5、MA10、MA30 叠加；开关必须影响图表上的折线。
4. K 线下方横向时间轴展示近一年交易日的股票池概览，用户可切换交易日。
5. 下方/左侧股票池表格展示当日候选股，分页固定 10 个个股一页。
6. 表格得分项展示 Step 2 权重配置命中后的贡献。
7. 表格指标列展示 Step 1 指标组的真实筛选命中。
8. 右侧只保留行情、估值和必要上下文；不展示“命中指标”和 raw debug 信息。

## 当前实现事实

| 偏移点 | 当前事实 | 代码位置 | 影响 |
|---|---|---|---|
| Step 3 暴露开始日期、结束日期和展示行数 | `PoolPreviewPanel` 直接渲染三个输入框，并把变化写入 `previewRange`。 | `app/racingline_new/src/features/strategy/components/pool-preview-panel.tsx` | 技术执行参数泄露到产品主流程，用户会把 Step 3 理解成再次配置 preview 范围。 |
| 默认预览范围只有数天 | `defaultPreviewRange` 是 `2026-05-26` 到 `2026-06-01`。 | `app/racingline_new/src/routes/strategy-page.tsx` | K 线下方横轴和股池日期只覆盖很短区间，不能满足近一年预览。 |
| K 线复权切换只部分接通 | `adjustmentMode` 会传给 `security-analysis` 请求；后端能按 adjustment 选择 OHLC，但前端没有明确显示数据源状态，也没有验收约束。 | `stock-pool-preview-workbench.tsx`, `rearview-core/src/api/mod.rs` | 用户感知为切换无效时，缺少可定位机制；后续容易再次退化。 |
| MA5/MA10/MA30 开关无效 | `trendLines` 只保存在组件 state，既不传 `ma_windows`，也没有渲染任何 line series。 | `stock-pool-preview-workbench.tsx` | UI 提供了趋势线控件，但图表只画蜡烛。 |
| 右侧展示“命中指标”和“原始值” | `KeyDataPanel` 展示 `selectedMetricRows` 和 `rawValueRows`。 | `stock-pool-preview-workbench.tsx` | Step 3 变成 debug/解释页，并且额外依赖 selected metrics/raw values。 |
| 右侧可以调整指标权重 | `WeightControlSection` 在 Step 3 右侧渲染 Step 2 权重滑杆。 | `stock-pool-preview-workbench.tsx` | Step 3 接管了 Step 2 编辑职责，破坏“Step 2 记录规则，Step 3 预览结果”的边界。 |
| 股票池分页是 50 | `pageSize = 50`，pool-page 请求 limit 也是 50。 | `stock-pool-preview-workbench.tsx` | 与验收预期的 10 只一页不一致。 |
| 得分项和指标列语义混杂 | `scoreItems` 来自 `score_breakdown`；`selectedMetricRows` 来自 `RuleVersionSpec.output_metrics`。当前 `buildStrategyPreviewRuleSpec()` 会把 Step 1 条件指标和 Step 2 权重指标都放入 `output_metrics`。 | `preview.ts`, `adapters.ts`, `planner/sql.rs` | 表格“指标”列不是 Step 1 指标组真实命中，容易混入评分指标或 raw 依赖指标。 |
| RFC 0026 对 Step 3 授权过宽 | RFC 0026 把“分数解释、关键指标、K 线/个股上下文、全池分页、preview security analysis”都放进 Step 3 完整版。 | `docs/RFC/0026-racingline-strategy-pool-preview-step3.md` | 实现按 RFC 扩张后，超过当前产品设计预期。 |

## 根因判断

### R1: RFC 0026 把能力补齐误写成主路径产品要求

RFC 0026 的目标原本是去 mock、接真实接口、补齐 preview snapshot。但文档后半段把 run result 个股 analysis 页的能力迁移到了 Step 3，导致 Step 3 主界面承担了太多职责。

需要调整为：

1. Step 3 主路径只保留股池预览需要的展示。
2. `security-analysis` 可以作为后端数据来源继续存在，但前端只消费 K 线和行情上下文，不展示命中指标/raw debug。
3. 全池分页可以继续存在，但默认分页大小和展示方式要按股池预览设计收敛。

### R2: 技术执行参数泄露到 UI

`start_date`、`end_date`、`preview_row_limit` 是 preview execution 的技术输入，但当前直接暴露在 Step 3。用户预期不是在 Step 3 配置执行区间，而是看到近一年预览。

需要调整为：

1. 前端内部固定 preview window 为近一年交易日，或由更高层策略设置统一决定。
2. Step 3 不显示开始日期、结束日期和展示行数。
3. `preview_row_limit` 不再作为用户可编辑字段；首屏和分页使用固定策略。

### R3: Step 3 混入 Step 2 编辑

右侧 `WeightControlSection` 允许在 Step 3 修改权重并重新 preview。这个交互会让 Step 3 变成“权重微调器”，与 Step 2 的职责重叠。

需要调整为：

1. Step 3 不显示权重滑杆。
2. 用户要改权重时返回 Step 2。
3. 修改 Step 1/Step 2 后，已有 preview 标记 stale；再次点击股池预览才生成新结果。

### R4: 图表 contract 有数据，但前端没有真正渲染叠加层

后端 `SecurityAnalysisResponse.chart.series[].ma` 和 `price_overlays` 已存在，`chart.ma` 元数据也描述了 MA 可用性。但前端 `CandlestickChart` 只渲染 `CandlestickSeries`，没有根据 `trendLines` 添加 line series，也没有把 `ma_windows` 传给请求。

需要调整为：

1. 前端请求 `security-analysis` 时显式传 `ma_windows=5,10,30` 或后端默认返回这三个窗口。
2. `CandlestickChart` 接收 `visibleTrendLines`，为 MA5/MA10/MA30 分别渲染 line series。
3. 当 adjustment 不是前复权且后端返回 `chart.ma.status = "forward_adjusted_only"` 时，UI 要么禁用 MA 开关，要么提示 MA 只支持前复权，不能表现为“开关无效”。

### R5: “指标列”缺少 Step 1 命中语义

当前后端 `selected_metrics` 来自 `RuleVersionSpec.output_metrics`。前端构建 preview rule 时把 Step 1 条件指标和 Step 2 评分指标合并进 `output_metrics`。这只能说明“这些指标被输出”，不能说明“这些指标组命中了”。

需要调整为：

1. 前端 `PreviewSnapshot.labels` 保留 Step 1 `conditionId/groupId -> metric` 映射。
2. 表格“指标列”只从 Step 1 condition paths 生成展示项。
3. 后端如需权威命中状态，应补充 `filter_breakdown` 或 `condition_hits` 字段，按 condition path 返回 boolean/value/operator/threshold。
4. 在后端字段未补齐前，前端只能展示 Step 1 条件指标的返回值，不得标注为“命中”。

## 调整方案

### D1: 收缩 Step 3 产品职责

Step 3 的最终职责定义为：

```text
展示一次真实 preview execution 的股池结果
展示近一年 K 线和近一年交易日股池概览
展示当日候选股排名、Step 2 得分项和 Step 1 指标列
允许用户切换日期、切换个股、翻页、切换复权和趋势线
不允许用户在本页编辑 Step 1/Step 2 规则
不展示命中指标/raw debug 面板
```

需要从 Step 3 移除：

1. 开始日期输入。
2. 结束日期输入。
3. 展示行数输入。
4. 右侧命中指标区块。
5. 右侧原始值区块。
6. 右侧指标权重滑杆。

允许保留：

1. K 线复权切换。
2. MA5/MA10/MA30 趋势线开关。
3. 股票池日期切换。
4. 股票池分页。
5. 行情、估值和基础报价字段。

### D2: 固定 Step 3 预览窗口为近一年

前端进入 Step 3 时按交易日近一年构造 preview 请求：

```text
preview_end_date = 最近可用交易日或当前策略上下文交易日
preview_start_date = preview_end_date 往前约 1 年的交易日边界
preview_row_limit = 10
```

如果后端当前 `max_range_days` 不允许一年范围，需要二选一：

1. 提升 preview API 的 range 上限，并用 SQL 层 per-date top rows 控制返回量。
2. 新增 `POST /rearview/strategy-preview/timeline`，只返回近一年每个交易日的 `pool_count` 和必要 top rows，单日候选股仍走 `pool-page`。

推荐第二种，避免 `strategy-preview` 一次返回过大 payload。

### D3: 重设 API 使用边界

保留真实接口，不回退 mock，但调整每个接口的 UI 用途：

| API | 调整后用途 | UI 展示边界 |
|---|---|---|
| `POST /rearview/strategy-preview` | 执行 Step 1 + Step 2，生成近一年 preview snapshot 或首屏摘要。 | 不把 `preview_row_limit` 暴露给用户。 |
| `POST /rearview/strategy-preview/pool-page` | 按 selected trade date 拉取候选股分页。 | 固定 `limit = 10`。 |
| `POST /rearview/strategy-preview/security-analysis` | 拉取选中个股近一年 K 线、复权 OHLC、MA 和行情。 | 不展示 `selected_metrics`、`raw_values`、`result_snapshot` debug 信息。 |
| `GET /rearview/metrics` | 提供 Step 1/Step 2 指标中文 label 和单位。 | 用于表格 label，不作为 mock fallback。 |

### D4: K 线和趋势线实现口径

K 线默认请求：

```json
{
  "adjustment": "forward_adjusted",
  "lookback_trading_days": 240,
  "ma_windows": "5,10,30"
}
```

前端渲染规则：

1. `unadjusted` 使用未复权 OHLC。
2. `forward_adjusted` 使用前复权 OHLC。
3. `backward_adjusted` 使用后复权 OHLC。
4. MA5/MA10/MA30 开关控制 line series 的显示。
5. 如果后端只支持前复权 MA，则非前复权模式下禁用 MA 开关或显示不可用状态。
6. 验收时必须用同一只股票切换复权，确认蜡烛图价格序列发生变化；切换 MA，确认 line series 增删。

### D5: 股票池横轴改为近一年交易日概览

当前横轴绑定 `StrategyPreviewResponse.trade_dates`，所以只展示 preview request 的短区间。调整后：

1. 横轴数据源必须覆盖近一年交易日。
2. 每个日期展示 `trade_date` 和 `pool_count`。
3. 横轴不展示股票个数以外的 debug 信息。
4. 点击日期后，调用 `pool-page` 拉取该日候选股第一页。
5. 如果某日 `pool_count = 0`，表格展示空状态，但日期仍可见。

后端可选 contract：

```http
POST /rearview/strategy-preview/timeline
```

请求：

```json
{
  "rule": {},
  "start_date": "2025-06-01",
  "end_date": "2026-06-01"
}
```

响应：

```json
{
  "preview_id": "01...",
  "start_date": "2025-06-01",
  "end_date": "2026-06-01",
  "trade_dates": [
    {"trade_date": "2026-06-01", "pool_count": 4959}
  ]
}
```

如果不新增 endpoint，`strategy-preview` 必须支持一年范围并只返回轻量 timeline，不返回每一天大量 signals。

### D6: 分页固定 10 只个股

前端固定：

```ts
const pageSize = 10
```

请求：

```json
{
  "limit": 10,
  "offset": 0,
  "sort": "score_desc"
}
```

验收口径：

1. 每页最多 10 行。
2. 页码范围显示与 10 条分页一致。
3. 下一页 offset 按 10 增加。
4. 后端仍可保留通用 `limit` 参数，但 Step 3 UI 不提供 limit 输入。

### D7: 得分项只映射 Step 2 权重配置

表格“得分项”列只展示 Step 2 权重项贡献：

```text
Step 2 WeightIndicator.id
  -> scoring.rules[].name = weight:<id>:<index>
  -> score_breakdown[ruleName]
  -> 表格得分项
```

规则：

1. 如果某个 `score_breakdown` key 无法映射到 Step 2 权重项，显示为诊断 fallback，但不作为正常验收通过标准。
2. 得分项 label 使用 Step 2 权重配置里的中文指标和操作符。
3. 得分项不展示 Step 1 筛选条件。
4. 得分项不展示 raw required metrics。

### D8: 指标列只映射 Step 1 指标组

表格“指标”列改为 Step 1 筛选条件视图。

第一阶段可接受方案：

1. `buildStrategySelectionRuleSpec()` 保留 `conditionPaths`。
2. `PreviewSnapshot` 保存 applied Step 1 condition mapping。
3. 表格按 Step 1 条件顺序展示对应 metric value。
4. 文案使用“筛选指标”，不写“命中指标”，避免把 value 展示误读为后端 boolean hit。

完整方案：

后端 preview row 增加 `condition_hits`：

```json
{
  "condition_hits": {
    "pool_filters.groups[0].conditions[0]": {
      "group_id": "g1",
      "condition_id": "c1",
      "metric": "price_ma_20",
      "label": "MA20",
      "operator": "gte",
      "left_value": 12.34,
      "right_value": 10.0,
      "matched": true
    }
  }
}
```

前端表格指标列优先用 `condition_hits`，后端未提供时退回 Step 1 metric value 展示。

禁止：

1. 用 `selected_metrics` 直接当作 Step 1 命中指标。
2. 把 Step 2 权重指标混入“筛选指标”列。
3. 在接口失败时用 mock 指标列兜底。

## 实施建议

### Phase 1: 前端职责收缩

1. 移除 Step 3 的开始日期、结束日期、展示行数输入。
2. 移除 Step 3 右侧命中指标、原始值和指标权重区块。
3. 固定 pool-page `pageSize = 10`。
4. Step 3 权重修改入口改为返回 Step 2，而不是本页滑杆。

完成标准：

1. Step 3 页面没有日期范围和行数输入。
2. Step 3 右侧不出现“命中指标”“原始值”“指标权重”。
3. 股票池每页最多 10 只。

### Phase 2: K 线能力补齐

1. security-analysis 请求带上 `ma_windows=5,10,30`。
2. `CandlestickChart` 渲染 MA5/MA10/MA30 line series。
3. 复权切换后重新请求并更新图表。
4. 非前复权模式下明确处理 MA 可用性。

完成标准：

1. 除权、前复权、后复权切换会改变 K 线 OHLC。
2. MA5/MA10/MA30 开关会改变图表叠加线。
3. 默认展示近一年 K 线。

### Phase 3: 近一年股池时间轴

1. 确定最近可用交易日来源。
2. 生成近一年 preview/timeline 范围。
3. 横轴展示近一年交易日和每日期候选池数量。
4. 点击日期后用 `pool-page limit=10` 拉取候选股。

完成标准：

1. 横轴不再只展示 `2026-05-26` 到 `2026-06-01` 这类短区间。
2. 横轴覆盖近一年交易日。
3. 每个日期的候选池数量来自 Rearview 真实接口。

### Phase 4: Step 1/Step 2 展示语义拆分

1. Step 2 得分项继续用 `score_breakdown`，但只映射权重项。
2. Step 1 指标列从 condition mapping 生成。
3. 如需真实 boolean 命中，后端补 `condition_hits`。
4. 前端禁止把 `selected_metrics` 整包展示成“命中指标”。

完成标准：

1. 得分项与 Step 2 权重配置一一对应。
2. 指标列与 Step 1 指标组一一对应。
3. Step 2 指标不会混入 Step 1 指标列。

### Phase 5: 文档回写

1. 更新 RFC 0026，把 Step 3 主路径从“个股 analysis/debug”收缩为“股池预览”。
2. 更新 `docs/systems/racingline.md`，删除 Step 3 展示 raw values、完整 debug analysis 的职责表述。
3. 如新增 timeline 或 condition_hits API，更新 `docs/systems/rearview.md`。
4. 完成后新增 job report，记录浏览器验收。

## 验收清单

1. `/strategies` Step 3 不展示开始日期、结束日期和展示行数。
2. K 线默认近一年。
3. 复权切换实际改变图表数据。
4. MA5、MA10、MA30 开关实际控制叠加线。
5. 右侧不请求、不展示“命中指标”面板。
6. K 线下方横轴展示近一年股票池日期。
7. 股票池每页 10 个个股。
8. 得分项来自 Step 2 权重配置。
9. 指标列来自 Step 1 指标组。
10. Step 1/2/3 成功状态全部来自 Rearview 真实接口，不出现 mock 成功路径。

## 最小验证命令

文档阶段：

```bash
make docs-check
git diff --check
```

实现阶段：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

涉及 Rearview API 或 planner 时追加：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

浏览器验收需要记录：

1. network 中 Step 1/2/3 都来自 Rearview。
2. `strategy-preview`、`pool-page`、`security-analysis` 的请求参数符合本 debt 的调整口径。
3. 截图覆盖默认 Step 3、复权切换、MA 切换、分页和断开 Rearview 的失败态。

## Resolution 2026-06-22

已按 [Plan 0048](../plans/archive/0048-racingline-strategy-step3-drift-remediation-plan.md) 完成修正，并在 [Step3 Drift Remediation report](../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md) 中记录命令和浏览器证据。

实际落地：

1. Step 3 移除了开始日期、结束日期和展示行数输入。
2. Step 3 移除了“命中指标”“原始值”和 Step 2 权重滑杆。
3. 新增 `POST /rearview/strategy-preview/timeline`，近一年横轴只返回 `trade_date + pool_count`。
4. Step 3 preview 首屏只拉取 selected/end date，`preview_row_limit = 10`。
5. `pool-page` 在 Step 3 固定 `limit = 10`。
6. `security-analysis` 请求固定 `lookback_trading_days = 240` 和 `ma_windows = "5,10,30"`。
7. K 线复权切换会触发新的 Rearview 请求，前复权与除权 OHLC 已在浏览器验收中确认不同。
8. MA5/MA10/MA30 使用 Lightweight Charts line series 渲染；非前复权模式按后端 metadata 禁用。
9. 表格“得分项”只来自 Step 2 `score_breakdown`，表格“指标”只来自 Step 1 condition mapping。
10. `/strategies` 成功路径仍全部来自 Rearview 真实接口，不恢复 mock 成功路径。

保留限制：

1. Rearview 尚未返回 condition-level `condition_hits`；当前指标列展示 Step 1 筛选指标值，不宣称 boolean 命中。
2. `security-analysis` 响应仍保留 diagnostics 字段，但 Step 3 主 UI 不展示这些 debug 字段。
