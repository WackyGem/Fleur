# Plan 0040: Racingline 个股分析页交互与规则适配改造计划

日期：2026-06-15

状态：Completed

领域：racingline, rearview

关联系统：racingline, rearview, dbt marts

代码根：

- `app/racingline/`
- `engines/crates/rearview/`

关联文档：

- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
- [RFC 0018: Rust Rearview 规则选股服务与 mart 指标库](../../RFC/0018-rust-stock-screening-service.md)
- [RFC 0020: Racingline Run Result 个股分析页](../../RFC/0020-racingline-run-result-security-analysis-page.md)
- [Plan 0039: Racingline Run Result 个股分析页实施计划](0039-racingline-run-result-security-analysis-page-implementation-plan.md)
- [mart_stock_quotes_daily 设计](../../design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md)
- [mart_stock_trend_indicator 设计](../../design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md)
- [mart_stock_momentum_indicator 设计](../../design/dbt_layer/fleur_marts/mart_stock_momentum_indicator.md)
- [mart_stock_volume_indicator 设计](../../design/dbt_layer/fleur_marts/mart_stock_volume_indicator.md)
- [mart_stock_price_pattern_daily 设计](../../design/dbt_layer/fleur_marts/mart_stock_price_pattern_daily.md)

相关规则：

- `fleur-harness`：计划、系统地图、质量门禁和归档规则。
- `shadcn`：Racingline 组件改造优先组合现有 shadcn/ui 组件；新增 option set 组件时优先使用 `ToggleGroup`。
- 实施阶段涉及 Rust、ClickHouse SQL、Playwright 验收时，再按 AGENTS.md 路由使用对应 skills。

## 目标

1. 优化 `/runs/:runId/securities/:securityCode` 个股分析页的图表和工具栏体验。
2. K 线和成交量颜色切换为 A 股常用口径：红涨、绿跌。
3. 移除页面中不必要的 `current mart query` 和 `indicators forward_adjusted` 文案噪声，同时保留运行快照与 mart 当前查询值的结构区分。
4. 将 MA 开关改造成更适合快速切换的组件，并与前复权、后复权、不复权选择保持在同一工具栏水平线上。
5. 支持 `price_ema2_10`、`price_avg_ma_14_28_57_114`、`price_avg_ma_3_6_12_24` 像 MA 一样可选并叠加到主 K 线图。
6. 将 Rearview 代表规则、metric allowlist、规则表单默认值和结果展示适配新的过滤条件与得分条件。
7. 完成桌面和移动端浏览器验收，确保工具栏不会换行重叠、图表非空、叠加线可切换、规则运行结果可解释。

## 非目标

1. 不在前端重算 KDJ、MA、EMA、BOLL、RSI、价格结构或任何选股规则。
2. 不新增交易、下单、组合调仓或回测能力。
3. 不改变 dbt mart 字段语义；前端和 Rearview 只消费已存在的 mart 字段。
4. 不把历史裸字段名 `ema2_10`、`avg_ma_14_28_57_114`、`avg_ma_3_6_12_24` 作为新契约字段；实现必须使用 ADR 0010 后的 canonical 字段名。
5. 不为前复权收盘价新增 `forward_close_price` 这类逻辑别名；实现和规则输出统一使用 `close_price_forward_adj`，UI 展示文案可写作 `forward close`。
6. 不把趋势指标随 K 线复权口径临时重算。当前趋势指标仍是前复权口径，UI 必须避免暗示后复权或不复权趋势线已经重算。
7. 不修改 shadcn/ui CLI 生成的基础组件文件；业务交互在 Racingline 业务组件内组合实现。

## 当前事实基线

1. 页面入口示例：

```text
http://127.0.0.1:5173/runs/a4470e63-6fd3-46ce-9dab-8802c84cef26/securities/002298.SZ?adjustment=forward_adjusted&source=signals&trade_date=2026-05-26
```

2. 个股分析页主逻辑在 `app/racingline/src/routes/SecurityAnalysisPage.tsx`。
3. K 线图实现位于 `app/racingline/src/features/analysis/components/security-analysis-chart.tsx`，使用 `lightweight-charts`。
4. 当前 `--racingline-chart-up` 为绿色、`--racingline-chart-down` 为红色，和目标红涨绿跌相反。
5. 当前工具栏使用 `FilterSelect` 切换复权口径，使用 checkbox 切换 `MA5`、`MA10`、`MA30`。
6. 当前页面存在 `current mart query` 和 `indicators {adjustment}` badge；其中图表工具栏的这两个 badge 对分析动作帮助有限，右侧指标栏也还有一个可见的 `current mart query` badge。
7. 当前 chart response 的 `ma` map 只覆盖 `5`、`10`、`30`，`TrendIndicatorRow` 也只为图表选择了这三个 MA 和 BOLL/MACD 字段。
8. `mart_stock_trend_indicator` 已有 `price_ema2_10`、`price_avg_ma_14_28_57_114`、`price_avg_ma_3_6_12_24`、`price_ma_20` 和 `price_ma_60`。
9. `mart_stock_quotes_daily` 已有 `pct_amplitude`、`pct_change`、`volume`、`prev_volume`、前复权 OHLC 和 KDJ 当前行字段。
10. `mart_stock_volume_indicator` 已有 `volume_ma_5`。
11. `mart_stock_price_pattern_daily` 已有 `close_down_streak_days` 和 `n_structure_20_second_low_ratio`，但 Rearview metric policy 当前只暴露了 `n_structure_20_is_valid`。
12. 当前 `engines/crates/rearview/src/domain/rule.rs` 的 `representative_rule()` 与新规则不一致：`volume` 方向、KDJ 阈值、BOLL 关系、RSI 分值和输出字段均需调整。
13. `engines/crates/rearview/config/metric_policy.yml` 尚未暴露 `pct_amplitude`、`pct_change`、`close_price_forward_adj` 或 `n_structure_20_second_low_ratio` 作为可过滤或可打分指标。
14. `app/racingline/src/store/workbench.ts` 当前只能从表单生成 1 条简单 pool filter 和 1 条 `weighted_metric`，无法表达本计划需要的多条件过滤、字段间比较、字段乘常数和 `conditional_points` 得分规则。
15. Rearview 既有评分协议要求 `ScoreClamp` 保持在 `[0,99]` 范围内，当前代表规则使用 `clamp: { min: 0, max: 99 }`，且排序使用裁剪后的 `score`。

## 规则口径映射

用户描述中的历史短字段名只作为业务口径输入，实施时统一落到 canonical metric。

| 需求表达 | 实现字段或 logical metric | 条件 |
|---|---|---|
| `KDJ 的 J < 13` | `kdj_j_value` | `< 13` |
| `amplitude 4%` | `pct_amplitude` | `< 4.0`，按百分点值处理 |
| `-2% < pct_change < +2%` | `pct_change` | `> -2.0` 且 `< 2.0` |
| `volume < prev_volume * 0.8` | `volume`, `prev_volume` | 字段乘常数 RHS |
| `ema2_10 > avg_ma_14_28_57_114` | `price_ema2_10`, `price_avg_ma_14_28_57_114` | 字段间比较 |
| `close_down_streak_days < 4` | `close_down_streak_days` | `< 4` |
| `forward_close_price` | `close_price_forward_adj` | 仅作为用户文案映射，规则契约不新增 alias |
| `n_structure_20_second_low_ratio > 1` | `n_structure_20_second_low_ratio` | `> 1` |

`pct_amplitude` 和 `pct_change` 按 mart 文档的百分数口径处理，`1.23` 表示 `1.23%`，不是 `0.0123`。`amplitude 4%` 第一版解释为 `pct_amplitude < 4.0`；如果产品验收要求包含等于 4.0，只需将该条件的 operator 从 `lt` 改为 `lte`，不得改变字段单位。

得分规则采用互斥 KDJ 分段，避免 `J < -15` 同时拿到 `J < -10` 的分数：

| 得分项 | 条件 | 分数 |
|---|---|---:|
| 深度 KDJ 超跌 | `kdj_j_value < -15` | +25 |
| 轻度 KDJ 超跌 | `-15 <= kdj_j_value < -10` | +15 |
| 缩量 | `volume < volume_ma_5 * 0.6` | +20 |
| 跌破短组合均线 | `close_price_forward_adj < price_avg_ma_3_6_12_24` | +15 |
| 位于 MA20 和 MA60 之间 | `price_ma_20 < close_price_forward_adj < price_ma_60` | +15 |
| N 结构次低有效 | `n_structure_20_second_low_ratio > 1` | +15 |
| 跌破 BOLL 下轨 | `close_price_forward_adj < boll_dn_20_2` | +15 |
| RSI6 超跌 | `rsi_6 < 25` | +5 |

## 评分 clamp 方案

本计划沿用之前采用的评分 clamp 协议，不把上限扩展到 100 以上：

1. 新规则的 `scoring.clamp` 固定为 `{ min: 0, max: 99 }`。
2. 深度 KDJ 与轻度 KDJ 互斥后，理论最高原始分为 `110`；轻度 KDJ 路径理论最高原始分为 `100`。
3. 因此第一版会出现 `raw_score >= 99` 被裁剪为 `score = 99` 的情况，这是保留既有评分协议的有意结果，不是实现 bug。
4. `score_breakdown` 必须能解释每个得分项是否命中、各项贡献分、原始合计分和裁剪后的最终分；历史解释不能依赖回查当前 mart。
5. 如果产品后续要求高分段继续保持严格排序，不允许把 clamp 上限改为 `110`；应先在计划中选择一种显式方案：降低各项 points 使理论最高分不超过 `99`，或引入经评审的新排序 tie-break 规则。

## 实施阶段

### Phase 1: Rearview metric policy 与代表规则适配

目标：让新过滤和得分条件能被规则引擎校验、编译、explain 和运行。

任务：

1. 在 `engines/crates/rearview/config/metric_policy.yml` 增加或确认以下 logical metrics：
   - `pct_amplitude`
   - `pct_change`
   - `close_price_forward_adj`
   - `n_structure_20_second_low_ratio`
2. 确认新增指标的 `mart_table`、`column_name`、`value_kind`、`allowed_ops`、`null_policy` 和 `allow_filter`/`allow_scoring`。
3. 更新 `representative_rule()`：
   - pool filters 改为本计划的 6 条过滤条件。
   - scoring 改为本计划的 8 条得分条件。
   - 删除旧的 `overextended` 负分规则。
   - `scoring.clamp` 明确保持 `{ min: 0, max: 99 }`。
   - `top_n_default` 如无产品侧新要求，沿用现有默认值。
4. 在 `app/racingline/src/store/workbench.ts` 中新增一个可直接生成完整低位反转规则的 builder，例如 `buildLowReversalRuleVersionSpec()`；不要尝试用当前单条件 draft 结构硬塞本规则。
5. 在 Rules 工作台中提供明确入口加载或发布该 preset，并保留现有简单规则表单用于临时实验。
6. 更新规则构造、metric collection 和 validation 相关测试，确保字段间比较、字段乘常数、分段互斥条件均被覆盖。
7. 更新评分测试，覆盖 `raw_score > 99` 时最终 `score` 被裁剪为 `99`，且 `score_breakdown` 保留原始贡献信息。
8. 更新 Rules 工作台测试，覆盖 preset 生成的 6 条过滤条件、8 条 `conditional_points`、`clamp: { min: 0, max: 99 }` 和输出指标列表，避免 UI 继续生成旧指标名如 `kdj_j`。
9. 更新输出指标列表，至少包含：
   - `close_price_forward_adj`
   - `kdj_j_value`
   - `pct_amplitude`
   - `pct_change`
   - `volume`
   - `prev_volume`
   - `volume_ma_5`
   - `price_ema2_10`
   - `price_avg_ma_14_28_57_114`
   - `price_avg_ma_3_6_12_24`
   - `price_ma_20`
   - `price_ma_60`
   - `boll_dn_20_2`
   - `rsi_6`
   - `close_down_streak_days`
   - `n_structure_20_second_low_ratio`

完成标准：

1. `representative_rule()` 生成的新规则能通过 metric policy validation。
2. explain 输出能列出所有新增 mart 表依赖。
3. 新规则不再使用历史裸字段名或不复权 `close_price` 替代前复权收盘。
4. Racingline 能发布完整 preset 规则；只支持单条件的旧表单不能作为本计划的规则适配完成标准。
5. 新规则不会把 `ScoreClamp.max` 放宽到 `99` 以上；高分段裁剪行为有测试覆盖。

测试策略：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm run typecheck
npm run test
```

### Phase 2: Analysis API 扩展图表叠加字段

目标：让个股分析页一次请求拿到 MA 和新增趋势叠加线，不让前端拼接额外 ClickHouse 查询。

任务：

1. 扩展 `engines/crates/rearview/src/clickhouse/mod.rs` 的 `TrendIndicatorRow`，补充：
   - `price_ema2_10`
   - `price_avg_ma_14_28_57_114`
   - `price_avg_ma_3_6_12_24`
   - 必要时补充 `price_ma_20`、`price_ma_60`，用于右侧规则指标展示或调试。
2. 扩展 `query_analysis_trend_rows()` 的 SELECT 字段，保持 `WHERE trade_date BETWEEN ... AND security_code = ...` 与 `ORDER BY trade_date ASC`。
3. 在 `engines/crates/rearview/src/api/mod.rs` 中将 chart row 从单一 `ma` map 扩展为更通用的 price overlays：
   - 保留现有 `ma` 字段以降低兼容风险。
   - 新增 `price_overlays` 或等价字段，key 使用 canonical 字段名。
   - metadata 返回可用 overlay、默认开启项和前复权口径状态。
4. 第一版 API 固定返回页面所需的全部主图 overlay，前端切换只改变本地可见性状态；不把 overlay 选择写入 URL，也不新增会影响缓存键的 `price_overlays` query 参数。
5. 对后复权和不复权 K 线，继续返回前复权趋势 overlay metadata，但 UI 必须明确这些 overlay 是前复权趋势口径，或在非前复权 K 线下禁用 overlay。
6. 更新 TypeScript 类型 `app/racingline/src/types/rearview.ts`，并保持旧 response 字段解析兼容。

完成标准：

1. 示例证券 `002298.SZ` 在目标 URL 下能返回新增 overlay 数据或明确的 null/不可用状态。
2. API response 中的字段命名与 `metric_policy.yml` 和 dbt mart 字段一致。
3. 趋势 overlay 的缺失值不会导致整条 chart series 失败。

测试策略：

```bash
cd engines
cargo test -p rearview
```

### Phase 3: 图表颜色与价格叠加线改造

目标：主图支持红涨绿跌和多条可选趋势线，且叠加线切换不改变 K 线布局。

任务：

1. 在 `app/racingline/src/index.css` 调整 chart token：
   - `--racingline-chart-up` 改为红色。
   - `--racingline-chart-down` 改为绿色。
   - `--racingline-chart-volume-up` / `--racingline-chart-volume-down` 同步改为红/绿透明色。
   - dark mode 使用同一语义，避免明暗主题口径相反。
2. 更新 `security-analysis-chart.tsx` 的 fallback colors，防止 CSS token 读取失败时回退到旧颜色。
3. 将 `visibleMaWindows` 抽象为 `visiblePriceOverlays`：
   - `price_ma_5`
   - `price_ma_10`
   - `price_ma_30`
   - `price_ema2_10`
   - `price_avg_ma_14_28_57_114`
   - `price_avg_ma_3_6_12_24`
4. 新增 overlay 定义表，统一管理 label、chart title、颜色 token、可用性判断和取值路径。
5. 图表构造时按 `visiblePriceOverlays` 循环添加 `LineSeries`，不要为每个新增字段复制一段 `addLine` 调用。
6. 将成交量柱颜色从硬编码 fallback 改为当前读取到的 chart colors，确保红绿语义在 CSS token 更新后完整生效。
7. 更新图例，避免只显示 `MA5/10/30`，并确保长字段名不会挤压移动端布局。

完成标准：

1. 收盘价大于等于开盘价的 K 线和成交量柱显示为红色，反之显示为绿色。
2. 新增三个趋势 overlay 可独立开关，关闭后不影响 K 线、KDJ、RSI、MACD 和 BOLL 面板。
3. 移动端宽度下工具栏、图例和 chart canvas 不发生文本重叠。

测试策略：

```bash
cd app/racingline
npm run typecheck
npm run test
```

### Phase 4: 工具栏组件与文案清理

目标：把复权口径与趋势线选择做成一个紧凑、可扫描、适合高频操作的工具栏。

任务：

1. 检查 `app/racingline/src/components/ui` 已安装组件；如无 `toggle-group`，在 `app/racingline/` 下使用 shadcn CLI 添加。
2. 新增业务组件，例如 `PriceChartToolbar`：
   - 左侧为复权口径单选 segmented control：前复权、后复权、不复权。
   - 右侧为趋势 overlay 多选 toggle group：MA5、MA10、MA30、EMA2-10、AVG 3/6/12/24、AVG 14/28/57/114。
   - 同一水平 toolbar，窄屏允许横向滚动或自然换行，但不得遮挡图表。
   - 保留 `Signal day` 快捷按钮，放在同一操作带末尾。
3. 从 `SecurityAnalysisPage.tsx` 移除图表工具栏中的：
   - `current mart query`
   - `indicators {adjustment}`
4. 移除右侧 `IndicatorRail` 中可见的 `current mart query` badge，并保留 `Mart indicators` 标题、`Selected day` 和 `PostgreSQL run snapshot` 分区作为语义区分。
5. 保持现有 query string 行为：切换复权口径只更新 `adjustment`，不丢失 `source` 和 `trade_date`。
6. 使用 Hugeicons 时通过 `RacinglineIcon` 组合，不在按钮内手写 SVG。

完成标准：

1. 复权口径和 overlay 开关在桌面端同一行可见。
2. 移动端不出现按钮文字溢出、控制项重叠或图表被工具栏遮挡。
3. 页面不再出现可见的 `current mart query` 和 `indicators forward_adjusted` 文案。

测试策略：

```bash
cd app/racingline
npm run lint
npm run typecheck
```

### Phase 5: 规则结果展示与解释性补强

目标：让新规则运行后的结果列表、个股侧栏和 snapshot JSON 能帮助确认命中原因。

任务：

1. 确认新的 `output_metrics` 能进入 `selected_metrics`，结果列表 `MetricPreview` 不再只显示旧指标。
2. 如 `selected_metrics` 字段过多，`MetricPreview` 使用显式优先级列表展示与新规则直接相关的前三到五个关键指标，不能继续依赖 JSON insertion order：
   - `kdj_j_value`
   - `close_price_forward_adj`
   - `pct_change`
   - `pct_amplitude`
   - `volume`
   - `volume_ma_5`
3. 在 `RunSnapshotPanel` 中保持 `score_breakdown` 和 `filter_snapshot` 可展开，确保新得分项名称清晰。
4. 避免前端根据当前 mart 值重新判定规则是否命中；所有命中事实以 PostgreSQL run snapshot 为准。

完成标准：

1. 新规则运行结果的 score breakdown 能直接对应本计划中的 8 个得分项。
2. 个股分析页可以同时看到 run snapshot 和当前选中交易日 mart 值，不混淆两者来源。

测试策略：

```bash
cd app/racingline
npm run test
```

### Phase 6: 浏览器验收与报告

目标：用真实页面证明图表、工具栏、路由和规则解释没有回归。

任务：

1. 启动本地 dev 环境：

```bash
make racingline-dev
```

2. 用目标 URL 或实际可用 run/security 样本验收：

```text
/runs/:runId/securities/:securityCode?adjustment=forward_adjusted&source=signals&trade_date=YYYY-MM-DD
```

3. 桌面端检查：
   - 三栏布局仍成立。
   - K 线红涨绿跌。
   - 复权口径和 overlay toggle 同一行。
   - 新增 overlay 可开关且叠加在主图。
   - 无 `current mart query` / `indicators forward_adjusted` 噪声。
4. 移动端检查：
   - tabs 默认进入 chart。
   - 工具栏不遮挡图表。
   - overlay 标签不溢出。
   - 指标侧栏仍能查看 selected quote 与 run snapshot。
5. 检查 browser console 和 network；记录 API error、chart warning 或 layout issue。
6. 将验收结果写入 `docs/jobs/reports/`，并在完成后把本 plan 归档。

完成标准：

1. `npm run lint`、`npm run typecheck`、`npm run test`、`npm run build` 通过。
2. 涉及 Rearview 规则或 API 的 Rust checks 通过。
3. Playwright/CDP 桌面和移动端截图证明图表非空、工具栏可用、文本不重叠。
4. 验收报告记录实际 run id、security code、trade date、浏览器 viewport 和结果。

## 禁止模式

1. 禁止在前端根据 chart rows 或 quote rows 临时重算筛选和得分。
2. 禁止用历史裸字段名绕过 `metric_policy.yml`。
3. 禁止把 `close_price` 当作 `close_price_forward_adj` 使用。
4. 禁止为了容纳新得分项把 `ScoreClamp.max` 放宽到 `99` 以上。
5. 禁止在后复权或不复权 K 线下暗示趋势指标已按同一复权口径重算。
6. 禁止新增只有颜色或文案变化、没有状态管理边界的 ad hoc span/button 组合。
7. 禁止为了展示 overlay 在图表容器外创建会挤压 canvas 的浮层。

## 最小验证命令

前端改动：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run test
npm run build
```

Rearview 改动：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

文档改动：

```bash
make docs-check
git diff --check
```

## 完成标准

1. 目标 URL 对应页面完成红涨绿跌、文案清理、工具栏重组和新增 overlay 叠加。
2. 新规则可被 Rearview 创建、解释、运行，并在结果页展示可解释输出。
3. 前端、Rust 和文档最小质量门禁通过。
4. 生成浏览器验收报告后，将本计划从 `docs/plans/` 移入 `docs/plans/archive/`，并同步更新 `docs/plans/README.md`。
