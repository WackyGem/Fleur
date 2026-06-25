# Plan 0049: Racingline Step 3 股池预览二次漂移修正实施计划

日期：2026-06-22

状态：Completed

关联文档：

- [Debt 0005: Strategies Step 3 股池预览二次实现漂移](../../debt/archive/0005-2026-06-22-strategies-step3-implemennt-drift2.md)
- [RFC 0026: Racingline 股池预览 Step 3 实现方案](../../RFC/archive/0026-racingline-strategy-pool-preview-step3.md)
- [RFC 0025: Racingline 策略权重配置 Step 2 实现方案](../../RFC/archive/0025-racingline-strategy-weight-configuration-step2.md)
- [Plan 0048: Racingline Step 3 股池预览漂移修正实施计划](0048-racingline-strategy-step3-drift-remediation-plan.md)
- [Step3 Drift Remediation 报告](../../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
- [mart_stock_basic_snapshot 设计](../../design/dbt_layer/fleur_marts/mart_stock_basic_snapshot.md)

## 目标

1. 修正 Step 3 二次漂移：补齐板块展示、全复权模式 MA、量柱、动态近一年窗口、Step 3 权重微调和下钻性能。
2. 保持 RFC 0026 的核心边界：Step 3 是 preview execution 结果检查页，不创建正式 run、rule version、portfolio run 或回测结果。
3. 修正 RFC 0026 / Plan 0048 后的过度收缩：Step 3 可以调整 Step 2 权重草稿，但必须通过“更新股池”重新执行 preview 后才替换 applied snapshot。
4. 保持真实接口约束：Step 1/2/3 成功路径全部来自 Rearview 和 marts，不恢复 mock 成功路径。
5. 把性能优化做成可测目标：降低 `security-analysis` payload 和个股切换卡顿，并在 job report 记录优化前后证据。

## 非目标

1. 不恢复“命中指标”面板或 `raw_values` debug 面板。
2. 不把 Step 3 权重滑动变成本地实时重排；排名、得分和股池必须来自 Rearview 重新 preview。
3. 不引入行业、概念、地域或同类分组；本计划只处理 A 股交易板块 `security_board`。
4. 不持久化 preview result，不新增 preview cache 作为第一阶段必需能力。
5. 不改 Step 1 筛选表单和 Step 2 主权重配置页的交互结构。
6. 不把 Step 3 的预览窗口或分页大小暴露为用户主控件。

## 当前事实基线

| 领域 | 当前实现事实 | 证据 |
|---|---|---|
| Step 3 布局 | K 线 + timeline + 10 条分页表格 + 右侧行情/估值。 | `app/racingline_new/src/features/strategy/components/stock-pool-preview-workbench.tsx` |
| 证券显示 | K 线标题和表格使用 `security_name`、`security_code`、`exchange_code`，当前会显示 `000001.SZ / SZ`。 | `buildPreviewStockRow()` 与 `KLinePanel` |
| 板块数据 | `int_stock_basic_snapshot` 有 `security_board`；`mart_stock_basic_snapshot` 只透出 `security_code/security_name/exchange_code`。 | `pipeline/elt/models/marts/mart_stock_basic_snapshot.sql` |
| Preview API | 已有 `strategy-preview/timeline`、`strategy-preview`、`pool-page`、`security-analysis`。 | `engines/crates/rearview-core/src/api/mod.rs` |
| MA 数据 | `security-analysis` 可返回 `ma_windows`，但 `ma_values()` 在非前复权模式返回空 map。 | `engines/crates/rearview-core/src/api/mod.rs` |
| 图表 | 前端只渲染 `CandlestickSeries` 和 `LineSeries`，未渲染 `volume`。 | `CandlestickChart` |
| 近一年窗口 | `defaultPreviewRange` 写死为 `2025-06-01` 到 `2026-06-01`。 | `app/racingline_new/src/routes/strategy-page.tsx` |
| 权重微调 | Step 3 已移除权重配置 props 和右侧权重控件；`更新股池` 只能按当前全局权重草稿执行。 | `PoolPreviewPanel` / `StockPoolPreviewWorkbench` |
| 下钻性能 | 切换个股触发 `security-analysis`，返回 chart、selected quote、完整 `quote_rows` 和 diagnostics；一次响应曾约 578KB。 | `docs/jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md` |

## 预期差异

| 项 | 当前行为 | 预期行为 |
|---|---|---|
| 标题区 | `平安银行` + `000001.SZ / SZ` | `平安银行` + `000001.SZ / 深市主板` |
| 板块来源 | 前端误用 `exchange_code` | Rearview 从 `mart_stock_basic_snapshot.security_board` 返回真实板块 |
| MA | 仅前复权可用 | 除权、前复权、后复权都可显示 MA5/MA10/MA30；MA 基准固定前复权 |
| 量柱 | 无量柱 | K 线底部展示 `chart.series[].volume` 量柱 |
| 近一年 | 固定样例日期 | 每次 preview 根据当前日期生成近一年 timeline，单日 preview 使用 timeline 最新交易日 |
| 权重配置 | Step 3 不可调权重 | 右侧行情下方支持紧凑权重配置，改动标记 stale，点击更新股池后应用 |
| 下钻查询 | 大 payload + 每次重建图表 | 减少 payload、并行查询、缓存/取消过期请求，切换个股不卡顿 |

## 实施缺口

| 缺口 ID | 缺口 | 风险 | 填补方案 |
|---|---|---|---|
| G1 | `security_board` 未进入 mart/API/前端类型。 | 无法真实展示板块，只能继续误用交易所代码。 | mart 透出字段，Rearview display contract 和 TS 类型同步扩展。 |
| G2 | 板块 label 缺少统一映射。 | UI 显示裸枚举值，用户难以识别。 | 前端新增 `formatSecurityBoard()`，覆盖主板、创业板、科创板和缺失回退。 |
| G3 | MA 数据与当前 OHLC adjustment 绑定。 | 非前复权模式 MA 消失。 | 后端始终返回前复权 MA；前端不因 adjustment 切换清空 MA toggle。 |
| G4 | 图表缺少 volume histogram。 | K 线无法观察量价关系。 | 使用 Lightweight Charts `HistogramSeries` 渲染量柱。 |
| G5 | 近一年窗口硬编码。 | 日期推进后 preview 不再近一年，也可能漏最新交易日。 | 用当前日期构造 timeline range，先请求 timeline，再用 latest trade date 请求单日 preview。 |
| G6 | Step 3 权重微调被删除。 | 用户必须离开预览页调整权重，工作流断裂。 | 恢复右侧紧凑权重配置，保持 draft/applied 分离和 stale gate。 |
| G7 | `security-analysis` payload 过大。 | 切换个股卡顿，网络和前端渲染压力高。 | 支持 `include_quote_rows=false` 或 preview 默认瘦身，仅返回 Step 3 所需数据。 |
| G8 | 后端下钻查询串行度高。 | ClickHouse 和 API 延迟叠加。 | 在 chart window 确定后并行 trend/momentum 查询，保留 membership 校验。 |
| G9 | 前端下钻请求缺少交互优化。 | 快速切换个股堆积过期请求，图表闪烁。 | React Query staleTime、keep previous data、取消/忽略过期请求，必要时 debounce。 |
| G10 | RFC 0026 与 debt 0005 边界冲突。 | 后续 agent 可能继续按“Step 3 禁止调权”实现。 | 实施时同步更新 RFC 0026 和系统地图，明确允许 Step 3 权重微调但禁止 debug 面板。 |

## 实施阶段

### 阶段 1：板块 contract 和展示

目标：K 线标题和表格展示真实交易板块，不再显示裸交易所代码作为板块。

任务：

1. dbt mart：
   - 修改 `pipeline/elt/models/marts/mart_stock_basic_snapshot.sql`，从 `int_stock_basic_snapshot` 透出 `security_board`。
   - 更新 `pipeline/elt/models/marts/mart_stock_basic_snapshot.yml`，增加字段描述和 accepted values。
   - 更新 `docs/design/dbt_layer/fleur_marts/mart_stock_basic_snapshot.md`，明确 `security_board` 是交易板块，不是行业。
2. Rearview：
   - `SecurityDisplayRow` 增加 `security_board`。
   - `query_security_display_rows()` 查询 `security_board`。
   - preview rows、pool-page items、security-analysis response 透出 `security_board`。
   - 增加 Rust 测试覆盖 display row JSON 解析和 preview response 字段。
3. Racingline：
   - TS 类型增加 `security_board`。
   - `PreviewStockRow` 增加 `board`。
   - K 线标题第二行和表格股票列显示 `security_code / boardLabel`。
   - 保留 `exchange_code` 作为 diagnostics/fallback，不作为板块主展示。

完成标准：

- `/strategies` Step 3 标题显示类似 `平安银行 / 000001.SZ / 深市主板`。
- `rg "000001.SZ / SZ"` 类似裸交易所展示不再出现在 Step 3 主 UI 验收快照中。

测试策略：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_basic_snapshot
uv run python elt/scripts/validate_field_glossary.py

cd ../engines
cargo test -p rearview-core

cd ../app/racingline_new
npm test
npm run typecheck
```

### 阶段 2：MA 全复权模式可见与量柱

目标：补齐 K 线图预期能力，三种复权模式都能展示 MA，且图表包含成交量柱。

任务：

1. Rearview：
   - 修改 `ma_values()`，无论当前 `adjustment` 是除权、前复权还是后复权，都返回前复权 `price_ma_5/10/30`。
   - 修改 `ChartMaMetadata.status`，当前 OHLC adjustment 不再导致 MA unavailable。
   - 如需避免语义混淆，增加 `basis_adjustment = "forward_adjusted"` 或在 metadata 文档中固定说明。
   - 增加 Rust 测试：`adjustment = unadjusted/backward_adjusted` 时仍返回 MA map。
2. Racingline：
   - `maAvailable` 改为由后端 `available_windows` 决定，不再依赖 `adjustmentMode === "forward_adjusted"`。
   - 切换复权时保留已选择的 MA toggle。
   - 引入 `HistogramSeries`，用 `chart.series[].volume` 渲染量柱。
   - 用同一 chart instance 生命周期内渲染 candlestick、MA lines 和 volume histogram。
3. 浏览器验收：
   - 除权、前复权、后复权都能开关 MA5/MA10/MA30。
   - K 线底部可见量柱。

完成标准：

- 三种复权模式均能显示 MA line series。
- 图表 canvas 中量柱随个股切换仍渲染。

测试策略：

```bash
cd engines
cargo test -p rearview-core

cd ../app/racingline_new
npm test
npm run typecheck
npm run build
```

### 阶段 3：动态近一年 preview window

目标：删除固定样例日期，让 timeline 和单日 preview 始终围绕当前日期与最新交易日工作。

任务：

1. 前端新增 `buildPreviewTimelineRange(now = new Date())`：
   - `end_date = today`
   - `start_date = today - 1 year`
   - 日期格式固定 `YYYY-MM-DD`
2. 调整 `openPreview()`：
   - 先调用 `previewTimelineMutation.mutateAsync()`。
   - 从 `timeline.trade_dates.at(-1)` 取 `latestTradeDate`。
   - 再用 `latestTradeDate` 调用 `previewMutation.mutateAsync({ start_date: latestTradeDate, end_date: latestTradeDate, preview_row_limit: 10 })`。
   - timeline 为空时创建 empty snapshot 或展示明确 empty 状态，不空跑当天自然日。
3. 删除或降级 `defaultPreviewRange`：
   - 不再保存写死日期。
   - `PreviewSnapshot.range` 保存实际 timeline range 和 selected preview date。
4. 增加单元测试：
   - 固定 `now` 检查 range。
   - timeline 最新交易日驱动单日 preview。
   - timeline 为空时不调用 preview。

完成标准：

- Network 中 timeline request 不再固定 `2025-06-01` / `2026-06-01`。
- 周末或非交易日运行时，单日 preview 使用 timeline 最新交易日。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
```

### 阶段 4：恢复 Step 3 权重微调

目标：恢复“看行情和 K 线后微调权重，再更新股池”的工作流，同时不恢复 debug 面板。

任务：

1. 状态和 props：
   - `PoolPreviewPanel` / `StockPoolPreviewWorkbench` 重新接收当前 `weightIndicators` draft 和更新回调。
   - 保留 `previewAppliedWeightIndicators` 作为 applied labels 来源。
   - `PreviewSnapshot.labels.scoringRules` 只来自成功 preview 的 applied weights。
2. UI：
   - 在右侧 `KeyDataPanel` 的“行情”“估值与财务”下方增加紧凑版权重配置。
   - 复用 Step 2 的权重编辑子组件或抽取 compact component，避免复制 adapter。
   - 页面仍不展示“命中指标”和 `raw_values`。
3. 行为：
   - 修改权重只更新 draft，并调用 `markRuleDraftChanged()` 标记 preview stale。
   - `更新股池` 调用 `openPreview(weightIndicators)`，成功后替换 snapshot。
   - stale 状态下表格继续展示 applied result，但后续阶段 gate 阻止继续。
4. 测试：
   - 权重修改后 snapshot stale。
   - 更新成功后 applied score labels 更新。
   - “得分项”列不被未执行的 draft 权重污染。

完成标准：

- Step 3 右侧行情下方可以编辑权重。
- 调整权重后点击“更新股池”会重新请求 timeline、preview、pool-page 和 security-analysis。
- 表格排名和得分项来自更新后的 applied 权重。

测试策略：

```bash
cd app/racingline_new
npm test
npm run typecheck
npm run lint
```

### 阶段 5：下钻查询性能优化

目标：降低切换个股的 API payload、查询耗时和前端渲染成本。

任务：

1. API contract：
   - `PreviewSecurityAnalysisRequest` 增加 `include_quote_rows?: boolean`，Step 3 默认传 `false`。
   - response 保留 `selected_quote` 和 `chart.series`；`quote_rows` 可为空或省略。
   - TS 类型兼容 `quote_rows?: QuoteMartRow[]`。
2. Rearview：
   - preview security-analysis 在 `include_quote_rows=false` 时不返回完整 quote rows。
   - 在 quote rows 确定 chart window 后，用 `tokio::try_join!` 并行 trend/momentum 查询。
   - 保留 preview membership 校验，禁止直接展示非当前 preview 股票。
   - 增加测试覆盖 quote rows omission 和 response shape。
3. Racingline：
   - `usePreviewSecurityAnalysisQuery()` 增加 `staleTime`。
   - 切换个股时保留旧图表直到新数据返回，避免闪烁。
   - 对快速点击的过期响应进行忽略或依赖 TanStack Query abort/cancel。
   - 如仍卡顿，再评估 chart instance 复用。
4. 验收记录：
   - job report 记录优化前后 response size。
   - 记录当前页 10 只股票连续切换观察。

完成标准：

- 单次 preview security-analysis response size 明显低于 578KB 基线。
- 快速切换当前页个股不出现明显页面卡顿。
- Network 不持续堆积过期 analysis 请求。

测试策略：

```bash
cd engines
cargo test -p rearview-core

cd ../app/racingline_new
npm test
npm run typecheck
```

### 阶段 6：文档回写和浏览器验收

目标：把新的边界沉淀为当前事实，避免第三次漂移。

任务：

1. 更新 `docs/RFC/archive/0026-racingline-strategy-pool-preview-step3.md`：
   - 改写 Plan 0048 后的 Step 3 主界面边界。
   - 明确 Step 3 允许权重微调，但只更新 draft，必须点击更新股池才替换 applied preview。
   - 明确 `security_board` 是交易板块，不是行业。
2. 更新系统地图：
   - `docs/systems/racingline.md` 增加 board、volume、dynamic window、weight tuning 和 performance 边界。
   - `docs/systems/rearview.md` 增加 `security_board` 和 `include_quote_rows` contract。
3. 更新 mart 设计：
   - `docs/design/dbt_layer/fleur_marts/mart_stock_basic_snapshot.md` 增加 `security_board`。
4. 新增 job report：
   - 记录命令、API samples、browser observations、response size 对比和未解决限制。
5. 浏览器验收：
   - 使用 `app/racingline_new` `/strategies` 页面。
   - 覆盖板块、MA、量柱、动态窗口、权重微调、更新股池、个股切换性能。

完成标准：

- 文档、实现和浏览器验收对 Step 3 当前边界一致。
- `make docs-check` 和 `git diff --check` 通过，除非工作树已有无关断链；如有无关断链必须在报告中单独说明。

## 禁止模式

1. 禁止用 `exchange_code` 伪装板块。
2. 禁止恢复“命中指标”或 `raw_values` debug 面板。
3. 禁止权重修改后在浏览器本地重算排名或得分。
4. 禁止继续使用固定样例日期表示“近一年”。
5. 禁止为了性能优化绕过 Rearview preview membership 校验。
6. 禁止用 mock K 线、mock 股池或 mock 行情兜底接口失败。
7. 禁止把 Step 3 的 preview result 持久化为正式 run 或回测结果。

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

dbt mart：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select mart_stock_basic_snapshot
uv run python elt/scripts/validate_field_glossary.py
```

文档：

```bash
make docs-check
git diff --check
```

## 完成标准

1. Step 3 K 线标题展示证券名称、证券代码和交易板块。
2. 除权、前复权、后复权下 MA5/MA10/MA30 都可显示。
3. K 线图显示成交量柱。
4. preview timeline 使用当前日期动态近一年窗口。
5. 单日 preview 使用 timeline 返回的最新交易日。
6. Step 3 右侧行情下方支持权重微调。
7. 修改权重后 preview stale，点击“更新股池”后新权重成为 applied snapshot。
8. 表格得分项和排名来自 Rearview 重新 preview，不来自本地重算。
9. 个股切换性能改善，response size 和交互观察写入 job report。
10. Step 1/2/3 成功路径仍全部来自 Rearview 真实接口。
11. RFC、systems、mart design 和 job report 已同步。
