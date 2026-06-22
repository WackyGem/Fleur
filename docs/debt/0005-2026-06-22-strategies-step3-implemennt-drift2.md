# Debt 0005: Strategies Step 3 股池预览二次实现漂移

状态：Resolved（2026-06-22）
日期：2026-06-22
领域：racingline, rearview, dbt-marts
关联代码：`app/racingline_new/`, `engines/crates/rearview-core/`, `pipeline/elt/models/marts/mart_stock_basic_snapshot.sql`
关联设计：`docs/RFC/0026-racingline-strategy-pool-preview-step3.md`, `docs/debt/0004-2026-06-22-strategies-step3-implemennt-drift.md`, `docs/plans/archive/0048-racingline-strategy-step3-drift-remediation-plan.md`

## 摘要

Plan 0048 修正了 Step 3 从 debug 页扩张的问题，但又产生了新的实现偏移：部分被删除的能力其实属于用户预览工作流，部分图表和数据 contract 没有按产品预期补齐。

新的偏移集中在 6 个方面：

1. K 线标题区应展示股票板块，但当前展示交易所代码。
2. 除权、前复权、后复权模式下都应能显示 MA5/MA10/MA30；MA 计算口径固定为前复权，但不应只在前复权模式可见。
3. K 线图缺少量柱。
4. “近一年”窗口被写死为 `2025-06-01` 到 `2026-06-01`，没有根据当天日期动态计算。
5. 行情下方的权重配置被误删；用户需要在 Step 3 调整权重并点击“更新股池”刷新预览。
6. 切换个股时下钻查询明显卡顿，需要拆解性能瓶颈并优化。

本 debt 登记问题和解决方案；已按 [Plan 0049](../plans/archive/0049-racingline-strategy-step3-drift2-remediation-plan.md) 实施并完成验收，见 [2026-06-22 Step3 Drift2 Remediation report](../jobs/reports/2026-06-22-racingline-strategy-step3-drift2-remediation.md)。

## 业务基线

Step 3 的正确边界应调整为：

```text
Step 3 股池预览
  展示真实 preview execution 结果
  支持检查近一年股池、个股 K 线、行情和评分排序
  支持在当前页面微调 Step 2 权重草稿
  点击“更新股池”后重新执行 Step 1 + 当前权重草稿
  不展示“命中指标”和 raw debug 面板
```

权重配置需要回到 Step 3，但必须满足：

1. 权重调整只改变 draft，不直接改写已展示 applied preview。
2. 调整权重后 Step 3 进入 stale 状态。
3. 点击“更新股池”后才重新调用 Rearview preview/timeline/pool-page/security-analysis。
4. 更新成功后新的权重成为 applied snapshot，表格得分项和排名同步刷新。
5. 不恢复 `selected_metrics` / `raw_values` debug 面板。

## 当前实现事实

| 偏移点 | 当前事实 | 代码位置 | 影响 |
|---|---|---|---|
| 标题区展示交易所而非板块 | K 线标题第二行使用 `[stock.code, stock.exchangeCode]`，结果是 `000001.SZ / SZ`。 | `app/racingline_new/src/features/strategy/components/stock-pool-preview-workbench.tsx` | 用户看到重复交易所信息，无法确认股票所属板块。 |
| Rearview display contract 不含板块 | `SecurityDisplayRow` 只有 `security_name` 和 `exchange_code`；`query_security_display_rows()` 只查这两列。 | `engines/crates/rearview-core/src/clickhouse/mod.rs` | 前端没有真实 `security_board` 可展示，只能误用 `exchange_code`。 |
| mart display 表不含板块 | `mart_stock_basic_snapshot.sql` 只输出 `security_code`、`security_name`、`exchange_code`；而 `int_stock_basic_snapshot` 已有 `security_board`。 | `pipeline/elt/models/marts/mart_stock_basic_snapshot.sql`, `docs/design/dbt_layer/fleur_intermediate/int_stock_basic_snapshot.md` | 板块字段停在 intermediate，Rearview 无法通过 mart 层读取。 |
| 非前复权模式禁用 MA | `maAvailable = adjustmentMode === "forward_adjusted" && status !== "forward_adjusted_only"`；后端 `ma_values()` 在非前复权直接返回空 map。 | `stock-pool-preview-workbench.tsx`, `engines/crates/rearview-core/src/api/mod.rs` | 除权和后复权模式下 MA5/MA10/MA30 不可用，违背“MA 固定前复权口径但所有复权模式可展示”。 |
| K 线图缺少量柱 | `ChartSeriesRow` 已有 `volume`，但前端只 `addSeries(CandlestickSeries)` 和 `LineSeries`。 | `stock-pool-preview-workbench.tsx`, `app/racingline_new/src/types/rearview.ts` | 图表无法观察量价配合，右侧成交量字段不能替代时间序列量柱。 |
| 近一年窗口写死 | `defaultPreviewRange` 固定为 `2025-06-01` 到 `2026-06-01`；`openPreview()` 直接用该 range。 | `app/racingline_new/src/routes/strategy-page.tsx` | 随日期推进后横轴不再代表“近一年”，也可能错过最新交易日。 |
| Step 3 权重配置被删 | `StockPoolPreviewWorkbenchProps` 只接收 `appliedWeightIndicators`，没有 draft 权重和更新回调；`KeyDataPanel` 只显示行情、估值。 | `pool-preview-panel.tsx`, `stock-pool-preview-workbench.tsx`, `strategy-page.tsx` | 用户无法在预览页微调权重，必须返回 Step 2，破坏原本“看 K 线和行情后微调权重再更新股池”的工作流。 |
| 个股切换卡顿 | 每次选股都会触发 `POST /rearview/strategy-preview/security-analysis`；后端会重新校验 preview membership，再查 240 条 quote、trend、momentum，并返回 `chart.series` 和完整 `quote_rows`。浏览器验收中单次 response 约 578KB。 | `stock-pool-preview-workbench.tsx`, `api/hooks.ts`, `rearview-core/src/api/mod.rs`, `clickhouse/mod.rs` | 切换个股时网络、ClickHouse 查询和前端 chart 重建叠加，造成明显卡顿。 |

## 根因判断

### R1: Plan 0048 把“debug 收缩”和“权重微调”混在一起处理

“命中指标”和 `raw_values` 属于 debug/解释面板，确实不应作为 Step 3 主 UI 展示；但 Step 3 右侧权重配置属于用户试错工作流，不应被一并删除。

需要恢复权重配置，但恢复方式必须保持 applied/draft 分离。

### R2: 展示字段 contract 只补了 security display 最小字段

之前为了避免行业字段扩张，Rearview 只补了 `security_name` 和 `exchange_code`。本次需求中的“板块”不是行业，而是 A 股交易板块 `security_board`，已有上游字段事实：

- `int_stock_basic_snapshot.security_board`
- 可取值：`sse_main_board`, `szse_main_board`, `chinext`, `star_market`

缺口在 mart 和 Rearview display contract，没有在数据源本身。

### R3: MA 可用性被错误绑定到当前复权模式

后端 metadata 当前表达为 `forward_adjusted_only`，前端据此在非前复权模式禁用 MA。用户真实要求是：

```text
K 线 OHLC 跟随当前复权模式切换
MA 始终使用前复权口径计算和展示
```

因此 MA 的计算基准和 K 线复权模式应解耦。

### R4: 图表只补了价格主图，没有补量价结构

Rearview 已返回 `ChartSeriesRow.volume`，但前端没有用 `HistogramSeries` 渲染量柱。该缺口是前端图表组合不完整，不需要新增后端字段。

### R5: “近一年”被实现成固定样例日期

当前 `defaultPreviewRange` 是为 2026-06-01 数据样本服务的硬编码。正确实现应根据当前日期动态生成 timeline 请求范围，并根据 Rearview 返回的最新交易日选择 preview 单日。

### R6: 下钻接口承担了过多实时工作

`security-analysis` 同时做 membership 校验、quote/trend/momentum 查询、完整 chart payload 和完整 quote rows 返回。Step 3 主 UI 只需要：

1. 图表序列。
2. 当前交易日 selected quote。
3. 少量 metadata。

完整 `quote_rows` 和 diagnostics 对 Step 3 主 UI 不是必需数据。当前 response 过大，且前端每次切换个股都重建图表。

## 解决方案

### D1: 股票板块展示 contract

目标：K 线标题区展示 `证券名称 + 证券代码 + 板块`，不再把交易所代码当板块。

后端和数据层：

1. 更新 `pipeline/elt/models/marts/mart_stock_basic_snapshot.sql`：
   - 从 `int_stock_basic_snapshot` 透出 `security_board`。
   - 类型建议 `LowCardinality(Nullable(String))` 或保持上游 nullable 枚举语义后 cast 为可序列化字符串。
2. 更新 `pipeline/elt/models/marts/mart_stock_basic_snapshot.yml` 和设计文档：
   - 增加 `security_board` 字段说明。
   - 明确它是 A 股交易板块，不是行业分类。
3. 更新 Rearview:
   - `SecurityDisplayRow` 增加 `security_board: Option<String>`。
   - `query_security_display_rows()` 查询 `security_board`。
   - `StrategyPreviewSignal`、`StrategyPreviewPoolPageItem`、`SecurityAnalysisResponse` 透出 `security_board`。
4. 更新前端类型：
   - `StrategyPreviewSignal.security_board?: string | null`
   - `SecurityAnalysisResponse.security_board?: string | null`
   - `PreviewStockRow.board?: string | null`

前端展示：

1. K 线标题第二行改为 `security_code / boardLabel`。
2. 表格股票列可保留 `security_code / boardLabel`，不再显示裸 `SZ`。
3. 增加 board label map：
   - `sse_main_board` -> `沪市主板`
   - `szse_main_board` -> `深市主板`
   - `chinext` -> `创业板`
   - `star_market` -> `科创板`
4. `security_board` 缺失时回退为 `exchange_code` 或 `--`，但 UI 不把回退值标注为板块。

### D2: MA 与 K 线复权模式解耦

目标：除权、前复权、后复权都能切换 MA5/MA10/MA30；MA 固定使用前复权指标。

后端：

1. 修改 `ma_values()`：
   - 移除 `adjustment != ForwardAdjusted` 时返回空 map 的逻辑。
   - 始终从 `TrendIndicatorRow.price_ma_5/10/30` 返回前复权 MA。
2. 修改 `price_overlay_values()`：
   - 如 Step 3 只需要 MA，可先保证 `price_ma_5/10/30` 始终返回。
   - 其他 forward-adjusted-only overlays 可保持 metadata 区分，避免误导。
3. 修改 `ChartMaMetadata`：
   - `status = "available"` 不再随当前 OHLC adjustment 变为 unavailable。
   - 增加或明确 `adjustment = "forward_adjusted"` 表示 MA 计算基准。
   - 可增加 `basis = "forward_adjusted"` 字段，避免把它误读为当前 K 线复权模式。

前端：

1. `maAvailable` 不再依赖 `adjustmentMode === "forward_adjusted"`。
2. MA toggle 始终可用，只在后端 `available_windows` 为空时禁用。
3. `visibleTrendLines` 不因切换除权或后复权被清空。
4. 测试覆盖：
   - `adjustment = unadjusted` 时 request 仍包含 `ma_windows = "5,10,30"`。
   - response 中 MA 数据仍渲染 line series。

注意：MA 与非前复权 OHLC 叠加时，价格尺度可能不完全同口径。这是本需求指定的展示口径，验收以“MA 固定前复权基准且所有复权模式可显示”为准。

### D3: K 线图增加量柱

目标：在 K 线主图底部展示成交量柱，复用后端已有 `chart.series[].volume`。

前端：

1. 从 `lightweight-charts` 引入 `HistogramSeries`。
2. 在 `CandlestickChart` 中新增 volume series：
   - `priceScaleId = "volume"` 或 overlay scale。
   - `scaleMargins.top` 约 `0.78`，`scaleMargins.bottom` 约 `0`，避免遮挡蜡烛。
3. volume bar 数据：
   - `time = trade_date`
   - `value = volume`
   - `color` 根据 `close >= open` 使用上涨/下跌颜色。
4. 保持 layout 稳定：
   - 图表容器高度不因量柱出现而变化。
   - MA、candlestick、volume 同步 fitContent。

测试与验收：

1. 浏览器 canvas 非空。
2. `chart.series[].volume` 有值时页面出现量柱。
3. 切换个股和复权时量柱仍显示。

### D4: 动态近一年窗口

目标：根据当天日期计算近一年窗口，不再写死 `2025-06-01` / `2026-06-01`。

前端推荐流程：

```text
today = 当前本地日期或 UTC 日期
timelineStart = today - 1 year
timelineEnd = today

POST /strategy-preview/timeline(start=timelineStart, end=timelineEnd)
latestTradeDate = timeline.trade_dates[-1]?.trade_date

if latestTradeDate exists:
  POST /strategy-preview(start=latestTradeDate, end=latestTradeDate, preview_row_limit=10)
else:
  展示空 timeline / empty state
```

实现要点：

1. 删除硬编码 `defaultPreviewRange`，改为 `buildDefaultPreviewRange(now = new Date())`。
2. `openPreview()` 不能并发请求 timeline 和单日 preview，因为单日 preview 的 end date 应来自 timeline 返回的最新交易日。
3. timeline range 使用自然日近一年即可；Rearview 会只返回有数据的交易日。
4. 如果当天没有数据、周末或假日，使用 timeline 返回的最后一个交易日作为 selected date。
5. 单元测试固定 `now`，避免日期相关测试不稳定。

验收：

1. 请求体不再固定为 `2025-06-01` / `2026-06-01`。
2. 在任意日期运行时，timeline end 接近当天。
3. 单日 preview 使用 timeline 最新交易日，而不是当天自然日硬打。

### D5: 恢复 Step 3 权重微调并保持 applied/draft 分离

目标：在右侧行情/估值下方支持权重配置，调整后点击“更新股池”刷新股池。

状态模型：

```text
weightIndicators
  当前 Step 2 / Step 3 共享权重草稿

previewSnapshot.labels.scoringRules
  上一次成功 preview 的 applied 权重标签

previewAppliedWeightIndicators
  上一次成功 preview 的 applied 权重列表
```

前端实现：

1. `PoolPreviewPanel` / `StockPoolPreviewWorkbench` 重新接收：
   - `draftWeightIndicators`
   - `onDraftWeightIndicatorAdd`
   - `onDraftWeightIndicatorUpdate`
   - `onDraftWeightIndicatorRemove`
   - `onUpdatePreview`
   - `strategyScoringCatalog`
2. 在 `KeyDataPanel` 的“行情”“估值与财务”下方渲染紧凑版权重配置。
3. Step 3 权重控件复用 Step 2 的 `WeightIndicatorsPanel` 子组件或抽取一个 compact 组件，不复制 scoring adapter 逻辑。
4. 修改权重时：
   - 调用现有 `updateWeightIndicator()` / `addWeightIndicator()` / `removeWeightIndicator()`。
   - 标记 preview stale。
   - 不立即改写 `previewSnapshot`。
5. 点击“更新股池”时：
   - 调用 `openPreview(weightIndicators)`。
   - 重新生成 `RuleVersionSpec.scoring.rules`。
   - 先跑 timeline，再用最新交易日跑单日 preview。
   - 成功后更新 `previewAppliedWeightIndicators` 和 `previewSnapshot.labels.scoringRules`。
6. “得分项”列继续只用 applied snapshot 的 score labels，避免 draft 权重未执行时污染当前表格。

禁止恢复：

1. 不恢复“命中指标”面板。
2. 不展示 `raw_values` debug 面板。
3. 不让权重滑动直接实时重排表格。

验收：

1. Step 3 右侧行情下方能看到权重配置。
2. 调整权重后页面显示 stale 或等效状态。
3. 点击“更新股池”后网络重新请求 timeline、preview、pool-page 和 security-analysis。
4. 表格排名和得分项来自更新后的 applied 权重。
5. 返回 Step 2 时权重草稿与 Step 3 调整保持一致。

### D6: 下钻查询性能优化

目标：切换个股时减少卡顿，降低网络 payload、ClickHouse 查询延迟和前端 chart 重建成本。

性能瓶颈拆解：

| 层 | 当前问题 | 优化方案 |
|---|---|---|
| API payload | `security-analysis` 同时返回 `chart.series` 和完整 `quote_rows`，Step 3 主 UI 只使用 `selected_quote` 和 chart。 | 增加请求参数 `include_quote_rows=false` 或 preview security-analysis 默认不返回完整 `quote_rows`。 |
| 后端查询 | membership 校验、quote、trend、momentum 查询串联；trend/momentum 可在确定 chart window 后并行。 | 使用 `tokio::try_join!` 并行 trend/momentum；保留 membership 校验但只查必要字段。 |
| 前端请求 | 每次点击行立即触发 analysis；快速连续点击会产生多次无用请求。 | 对 selected security 增加短 debounce，或在 React Query 中取消过期请求。 |
| 前端缓存 | query key 包含 `previewId/tradeDate/security/adjustment/ma_windows`，但没有设置有效 `staleTime`。 | 对当前 preview 内的 analysis 设置合理 `staleTime`，切回同一股票不重复请求。 |
| 图表渲染 | 每次 response 都销毁并重建 chart。 | 用 `useMemo` 减少数据重算；如仍卡顿，再评估持久 chart instance + setData 更新。 |
| 预取策略 | 用户通常在当前页 10 只股票内切换。 | 可在首屏空闲时预取下一只或 hovered row 的 analysis，但必须限制并发。 |

推荐实施顺序：

1. 先减 payload：preview `security-analysis` 默认不返回完整 `quote_rows`，只返回 `selected_quote` 和 `chart.series`。
2. 后端并行 trend/momentum 查询。
3. 前端给 analysis query 设置 `staleTime`，并在切换个股时保留旧图表直到新数据返回，避免空白闪烁。
4. 再评估 chart instance 复用和 hover/prefetch。

验收指标：

1. 单次 `security-analysis` response size 明显低于当前约 578KB。
2. 切换当前页 10 只个股时无明显主线程卡顿。
3. Network 中快速切换不会持续堆积已过期请求。
4. 图表 loading 不遮挡已加载的旧图，除非当前证券无数据。

## 实施阶段

### Phase 1: 数据 contract 补齐板块

任务：

1. mart 透出 `security_board`。
2. Rearview display row、preview row、pool-page row、security-analysis response 透出 `security_board`。
3. 前端显示 board label，移除标题区裸 `exchange_code`。

完成标准：

- K 线标题区显示 `平安银行 / 000001.SZ / 深市主板` 或同等板块表达。

### Phase 2: 图表能力补齐

任务：

1. MA 与 K 线复权解耦。
2. 前端所有复权模式允许切换 MA5/MA10/MA30。
3. K 线图增加成交量柱。

完成标准：

- 除权、前复权、后复权三种模式下 MA 控件均可用。
- 图表中同时可见蜡烛、MA 折线和量柱。

### Phase 3: 动态近一年窗口

任务：

1. 删除固定 `defaultPreviewRange` 日期。
2. timeline 使用当天动态近一年窗口。
3. 单日 preview 使用 timeline 返回的最新交易日。

完成标准：

- 任意日期启动时，请求窗口随当天日期变化。
- 周末/假日不导致单日 preview 空跑当天自然日。

### Phase 4: Step 3 权重微调恢复

任务：

1. 右侧行情下方恢复紧凑权重配置。
2. 权重调整只更新 draft 并标记 stale。
3. “更新股池”用当前权重草稿重新执行 preview。

完成标准：

- Step 3 可完成“看个股上下文 -> 调整权重 -> 更新股池 -> 查看新排名”的闭环。

### Phase 5: 下钻性能优化

任务：

1. 缩减 `security-analysis` payload。
2. 并行后端指标查询。
3. 增加前端 cache/staleTime/cancel/debounce。
4. 必要时优化 chart instance 复用。

完成标准：

- 切换当前页个股不再出现明显页面卡顿。
- job report 记录优化前后 response size 和交互观察。

## 禁止模式

1. 禁止用 `exchange_code` 伪装成板块。
2. 禁止恢复“命中指标”或 `raw_values` debug 面板。
3. 禁止权重滑动后直接在浏览器本地重算排名。
4. 禁止把固定样例日期继续作为“近一年”实现。
5. 禁止为了降低卡顿改回 mock K 线或 mock 行情。
6. 禁止跳过 Rearview membership 校验直接展示任意股票 analysis。

## 最小验证命令

文档阶段：

```bash
make docs-check
git diff --check
```

涉及前端实现时：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

涉及 Rearview 时：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 mart 字段时：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_basic_snapshot
uv run python elt/scripts/validate_field_glossary.py
```

## 浏览器验收清单

1. K 线标题展示证券名称、证券代码和板块，不展示裸交易所代码作为板块。
2. 除权、前复权、后复权三种模式下 MA5/MA10/MA30 都可以开关并显示。
3. K 线图底部显示成交量柱。
4. preview timeline 请求日期随当天变化，不再固定 `2025-06-01` / `2026-06-01`。
5. 当前日期不是交易日时，preview 使用 timeline 返回的最新交易日。
6. 行情和估值下方显示权重配置。
7. 修改 Step 3 权重后，旧 preview 进入 stale 状态。
8. 点击“更新股池”后，timeline、preview、pool-page、security-analysis 均重新请求。
9. 得分项和排名来自更新后的 applied 权重。
10. 快速切换当前页个股无明显卡顿；network 不堆积大量过期 analysis 请求。

## 后续文档动作

实施已同步更新：

1. `docs/RFC/0026-racingline-strategy-pool-preview-step3.md`：修正 Step 3 允许权重微调的边界。
2. `docs/systems/racingline.md`：加入 Step 3 板块、量柱、动态窗口和权重微调事实。
3. `docs/systems/rearview.md`：如新增 `security_board` 或 `include_quote_rows` contract，同步 API 事实。
4. `docs/design/dbt_layer/fleur_marts/mart_stock_basic_snapshot.md`：记录 `security_board` 字段。
5. `docs/jobs/reports/`：补充实现报告和性能优化前后证据。

## 关闭记录

已完成：

1. `security_board` 从 `int_stock_basic_snapshot` 透传到 `mart_stock_basic_snapshot`，并进入 Rearview preview rows、pool-page 和 preview security analysis response。
2. Step 3 K 线标题和表格展示中文交易板块，例如 `000001.SZ / 深市主板`。
3. MA5/MA10/MA30 固定使用前复权指标基准，除权、前复权、后复权三种 OHLC 模式下都可显示。
4. K 线图新增成交量柱，使用 `chart.series[].volume`。
5. Preview timeline 改为动态近一年窗口；单日 preview 使用 timeline 最新交易日。
6. Step 3 右侧恢复紧凑权重微调；修改只标记 stale，点击「更新股池」后才替换 applied snapshot。
7. Preview security analysis 支持 `include_quote_rows=false`，Step 3 payload 从约 578KB 降至约 193-198KB。
8. 后端 trend/momentum 查询在 chart window 确定后并行执行，前端 analysis query 使用 `staleTime`、`placeholderData` 和 abort signal。

验证摘要：

- `app/racingline_new`: lint、typecheck、test、build 通过。
- `engines`: fmt、clippy、workspace tests 通过。
- `pipeline`: `mart_stock_basic_snapshot` dbt build 和 field glossary lint 通过。
- 浏览器验收覆盖 board、MA、量柱、动态窗口、权重 stale/update 和 payload 瘦身。
