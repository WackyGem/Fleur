# Plan 0055: Racingline Step 2 到 Step 3 预览链路瘦身实施计划

日期：2026-06-25

状态：Completed

## 背景

[RFC 0030](../../RFC/archive/0030-racingline-step2-step3-preview-latency-slimming.md) 已确认：`/strategies` 从 Step 2「权重配置」点击「股池预览」到进入 Step 3 时，当前把一次用户动作串成 `timeline -> preview -> security-analysis prefetch -> setActiveStep("preview")`，并在 `previewSnapshot` 写入后自动触发 `strategy-backtests/validate`。

Step 3 首屏的最小必要信息只有近一年 timeline 和最新可展示交易日的前 N 个候选股。个股 K 线分析、KDJ/RSI/MACD/BOLL 宽响应和 Step 5 回测校验都不是进入 Step 3 的必要条件。本计划把 RFC 的减法方案拆成可执行阶段：先建立观测，再降低用户感知延时，再收敛首屏接口和 Step 3 chart context，最后只在数据证明仍慢时做 ClickHouse 查询或表结构优化。

## 目标

- Step 2 点击「股池预览」后，Step 3 shell render 不再等待个股 analysis 或 backtest validate。
- Step 3 首屏短期只 hard-block `timeline + latest preview`，中期只 hard-block `strategy-preview/open`。
- Step 3 个股图表只请求当前页面实际展示字段：OHLC、volume、MA5/MA10/MA30、证券显示信息和右侧行情/估值面板字段。
- Step 3 chart context 不再查询 `mart_stock_momentum_indicator_daily`，除非 Step 1/2 规则本身选择了 momentum 指标参与 preview 筛选或评分。
- `strategy-backtests/validate` 延后到 Step 4/5 gate，不在 Step 3 首屏自动触发。
- 所有性能优化都有 route 和 ClickHouse query 级 baseline，不在未测量前调整 mart 表结构。

## 非目标

- 不改变 Step 3 preview-only 边界；不创建 rule set、rule version、run、portfolio run 或 backtest result。
- 不把 Step 3 preview response 作为 Step 5 回测数据源；Step 5 仍按 applied rule 和 execution config 重新执行。
- 不引入鉴权、用户隔离或多租户 preview cache 语义。
- 不在 Phase 0 baseline 前调整 ClickHouse 表结构、dbt mart 物化策略或排序键。
- 不删除通用 `/rearview/security-analysis` 的 run result 个股分析用途。

## 关联文档

| 文档 | 用途 |
|---|---|
| [RFC 0030](../../RFC/archive/0030-racingline-step2-step3-preview-latency-slimming.md) | 设计依据、依赖分析和目标调用图 |
| [Racingline 系统地图](../../systems/racingline.md) | 前端职责、运行入口和质量门禁 |
| [Rearview 系统地图](../../systems/rearview.md) | Rearview API、ClickHouse mart 依赖和质量门禁 |
| [Plan 0047](0047-racingline-strategy-pool-preview-step3-implementation-plan.md) | Step 3 preview 原始实现计划 |
| [Plan 0049](0049-racingline-strategy-step3-drift2-remediation-plan.md) | Step 3 MA、量柱和 analysis payload 历史修正 |

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| Step 2 到 Step 3 入口 | [strategy-page.tsx](../../../app/racingline/src/routes/strategy-page.tsx) 的 `openPreview()` 串行调用 timeline、preview，并 `await prefetchInitialSecurityAnalysis()` 后才 `setActiveStep("preview")` |
| Step 3 analysis query | [stock-pool-preview-workbench.tsx](../../../app/racingline/src/features/strategy/components/stock-pool-preview-workbench.tsx) 使用 `usePreviewSecurityAnalysisQuery()`，当前请求通用 `/rearview/security-analysis` |
| Step 3 pool-page query | `useStrategyPreviewPoolPageQuery()` 只在本地没有 page 0、切换日期、翻页或搜索时需要 |
| Backtest validate | [strategy-page.tsx](../../../app/racingline/src/routes/strategy-page.tsx) 中 `useStrategyBacktestValidateQuery(backtestValidateDraft.request)` 在 `previewSnapshot` 存在后自动触发 |
| Rearview routes | [api/mod.rs](../../../engines/crates/rearview-core/src/api/mod.rs) 已有 `/rearview/strategy-preview/timeline`、`/rearview/strategy-preview`、`/rearview/strategy-preview/pool-page`、`/rearview/security-analysis` 和 `/rearview/strategy-preview/security-analysis` |
| Analysis 查询 | [clickhouse/mod.rs](../../../engines/crates/rearview-core/src/clickhouse/mod.rs) 当前 analysis helpers 包含 `quote_select_columns()`、`chart_quote_select_columns()`、`trend_select_columns()` 和 `query_analysis_momentum_rows()` |
| Step 3 实际消费字段 | 当前页面消费 `security_name/security_code/security_board`、`chart.series[].ohlc`、`volume`、`ma[5/10/30]`、`chart.ma.available_windows` 和右侧面板 14 个 `selected_quote` 字段 |

## 目标调用图

短期目标：

```text
openPreview()
  -> POST /rearview/strategy-preview/timeline
  -> POST /rearview/strategy-preview
  -> setPreviewSnapshot()
  -> setActiveStep("preview")

Step 3 right panel
  -> POST /rearview/security-analysis
     (background, non-blocking, temporary before chart-context lands)

Step 4/5 gate
  -> POST /rearview/strategy-backtests/validate
```

中期目标：

```text
openPreview()
  -> POST /rearview/strategy-preview/open
  -> setPreviewSnapshot()
  -> setActiveStep("preview")

Step 3 local interactions
  -> POST /rearview/strategy-preview/pool-page
  -> POST /rearview/strategy-preview/chart-context

Step 4/5 gate
  -> POST /rearview/strategy-backtests/validate
```

## 实施阶段

### Phase 0：建立最小观测和 baseline

目标：先确认慢在 route、ClickHouse、JSON 解析、React 渲染还是自动触发 query，后续优化不以体感判断。

实施项：

1. Rearview HTTP route 级耗时：
   - 在 Rearview server/API 层记录 route、method、status、elapsed_ms。
   - 日志必须能区分 `/rearview/strategy-preview/timeline`、`/rearview/strategy-preview`、`/rearview/security-analysis`、`/rearview/strategy-preview/pool-page` 和 `/rearview/strategy-backtests/validate`。
   - 失败响应也记录 elapsed 和 status。
2. ClickHouse query 级耗时：
   - 在 `ClickHouseClient::execute_text()` 记录 query_id、elapsed_ms、response_bytes。
   - 调用方能补充 row count 时，在对应 route 或 helper 日志中补充 parsed row count。
   - 保留 query_id，便于查 ClickHouse `system.query_log`。
3. 前端交互耗时：
   - 在 `openPreview()` 周围增加 dev/debug timing。
   - 至少记录 timeline start/end、preview start/end、analysis prefetch start/end、`setPreviewSnapshot`、`setActiveStep("preview")`。
   - 只在 dev 或显式 debug flag 下输出，不污染生产控制台。
4. 建立代表规则 baseline：
   - 使用同一条 Step 1/2 代表规则连续执行 5 次。
   - 记录 Step 2 click 到 Step 3 shell render、timeline、preview、analysis、validate 的 p50/p95。
   - 将结果写入 `docs/jobs/reports/YYYY-MM-DD-racingline-step2-step3-preview-latency-baseline.md`。

测试策略：

- Rust 单测覆盖耗时记录 helper 或可注入 clock 的格式化逻辑；如果 route middleware 不适合单测，至少通过 dev smoke 报告记录真实日志样例。
- 前端测试覆盖 debug timing 默认不启用；启用时不会改变 `openPreview()` 成功/失败语义。

完成标准：

- 每个 Rearview route 都能看到 elapsed_ms。
- 每个 ClickHouse query_id 都能看到 elapsed_ms 和 response_bytes。
- baseline report 记录 5 次样本、p50/p95、query_id 和环境说明。
- 未出现任何用户可见行为变化。

### Phase 1：降低 Step 2 到 Step 3 的感知延时

目标：不改变后端结果语义，先把非首屏工作移出硬等待链路。

实施项：

1. `openPreview()` 不再 `await prefetchInitialSecurityAnalysis()`：
   - latest preview 成功后立即 `setPreviewSnapshot(nextPreviewSnapshot)` 和 `setActiveStep("preview")`。
   - 使用 `queryClient.prefetchQuery()` 或 fire-and-forget 后台任务预热相同 query key。
   - 后台 prefetch 失败只影响 Step 3 图表区域，不进入 `openPreview()` catch，也不阻止页面跳转。
2. Step 3 analysis 错误边界保持局部化：
   - `StockPoolPreviewWorkbench` 继续在 K 线/右侧数据区域显示 loading、error 和重试。
   - 股池列表、timeline 和分页不依赖 analysis 成功。
3. `strategy-backtests/validate` 延后到 Step 4/5：
   - `useStrategyBacktestValidateQuery()` 增加调用方可控的 enabled gate，或在 `strategy-page.tsx` 只在 active step 进入 simulation/backtest 时传入 request。
   - Step 3 不自动创建 backtest execution draft。
   - 进入 Step 4 时允许后台 validate；进入 Step 5 或发起回测前必须同步 gate，失败则阻止继续。
4. 状态语义收敛：
   - `isOpeningPreview` 只表示 timeline/latest preview hard-blocking。
   - Step 3 analysis pending 不再让 Step 2 按钮保持打开中状态。
   - backtest validate pending 只影响 Step 4/5 的 gate 和提示。

测试策略：

- 前端单测覆盖 `openPreview()` 成功后不等待 analysis promise resolve 即切到 Step 3。
- 前端单测覆盖 analysis prefetch reject 不会阻止 `setActiveStep("preview")`。
- 前端单测覆盖 Step 3 时不触发 `strategy-backtests/validate`，进入 Step 4/5 gate 时才触发。
- 浏览器 smoke 验证 Step 3 列表先出现，图表可以局部 loading/error。

完成标准：

- Step 2 click 到 Step 3 shell render 不等待 `/rearview/security-analysis`。
- Step 3 首屏不触发 `/rearview/strategy-backtests/validate`。
- timeline 或 latest preview 失败仍不进入 Step 3。
- analysis 失败停留在 Step 3，且只影响图表/行情区域。

### Phase 2：收敛 preview 首屏接口

目标：将 Step 2 到 Step 3 首屏 hard-blocking 请求从两个前端 round trip 收敛为一个 `/rearview/strategy-preview/open`。

实施项：

1. 后端新增 `POST /rearview/strategy-preview/open`：
   - request 接收 `RuleVersionSpec`、`start_date`、`end_date`、`preview_row_limit`。
   - response 返回 `preview_id`、`sql_hash`、timeline、latest page 0、`required_metrics`、`required_marts` 和 `required_columns`。
   - 第一版服务端内部可仍执行 timeline，再对 latest non-empty trade date 执行 preview。
   - 不在第一版强行合成单条 ClickHouse SQL。
2. 共享规则校验和 planning 上下文：
   - 抽取 timeline 与 latest preview 的公共 validate/catalog traversal。
   - 若抽取会扩大改动面，先保守只合并前端 round trip，并把共享 planning 作为后续子任务。
3. 前端新增 API wrapper 和 query/mutation：
   - 在 [rearview.ts](../../../app/racingline/src/api/rearview.ts) 增加 `openStrategyPreview()`。
   - 在 query key 中保留 `preview_id`、range 和 rule hash 语义。
   - `openPreview()` 改为调用 `strategy-preview/open`，并复用现有 `PreviewSnapshot` 结构。
4. 保留旧接口兼容期：
   - `/timeline` 和 `/strategy-preview` 暂不删除。
   - 前端不再组合调用旧接口作为主路径。
   - 如果 open endpoint 返回空 timeline，则仍生成空 Step 3 snapshot，行为与当前空结果一致。

测试策略：

- Rust 单测覆盖 open endpoint response 组装：有候选池、无候选池、preview SQL hash/required marts 透传。
- Rust 单测覆盖 timeline 和 latest preview 只 validate 一次或明确记录第一版仍保守执行两段。
- 前端单测覆盖 `openPreview()` 只调用 `strategy-preview/open`，不再先调 timeline 再调 preview。
- 保留旧 timeline/preview API 单测，确保兼容期不回归。

完成标准：

- Step 2 到 Step 3 首屏 hard-blocking 请求中期只有 `POST /rearview/strategy-preview/open`。
- `pool-page` 仍只服务切换日期、翻页和搜索。
- 旧 `timeline + preview` 组合调用从前端主路径移除。
- Phase 0 baseline 有优化前后对比。

### Phase 3：新增 Step 3 chart context endpoint

目标：用页面专用 view model 替代 Step 3 对通用 `/rearview/security-analysis` 的依赖。

实施项：

1. 后端新增 `POST /rearview/strategy-preview/chart-context`：
   - request 只接收 `trade_date`、`security_code`、`adjustment`、`lookback_trading_days`、`ma_windows`。
   - 不接收完整 `RuleVersionSpec`。
   - 不重跑 preview pool-page。
   - 不宣称独立证明 preview membership；membership 来源是 Step 3 当前列表里的已选 row。
2. 定义轻量 response struct：
   - 顶层字段：`security_code`、`security_name`、`security_board`、`chart`、`selected_quote`。
   - `chart.series[]` 只包含 `trade_date`、`ohlc`、`volume`、`ma`。
   - `chart.ma.available_windows` 只包含实际可用 MA windows。
   - `selected_quote` 只包含右侧面板当前展示字段。
3. 复用必要查询能力但不要复用宽 response：
   - security display lookup 可复用现有 helper。
   - chart quote 只取 OHLC/volume。
   - trend rows 只取 MA5/MA10/MA30。
   - selected quote 只取右侧面板 14 个字段。
   - 不调用 `query_analysis_momentum_rows()`。
4. 前端新增类型和 wrapper：
   - 定义 `PreviewChartContextRequest` 和 `PreviewChartContextResponse`。
   - 新增 `previewChartContext()` API wrapper、query key 和 `usePreviewChartContextQuery()`。
   - `StockPoolPreviewWorkbench` 切到新 hook，UI 输出保持不变。
5. 旧入口处理：
   - Step 3 不再调用 `/rearview/security-analysis`。
   - `/rearview/strategy-preview/security-analysis` 暂不启用；后续若无独立 membership 校验需求，单独删除或下线。
   - 通用 `/rearview/security-analysis` 保留给 run result 个股分析页或其他真实宽分析场景。

测试策略：

- Rust 单测覆盖 `chart-context` request validation。
- Rust 单测覆盖 response 不包含 `kdj`、`rsi`、`macd`、`boll`、`price_overlays`、`indicator_panels`、`sources`、`quote_rows`。
- Rust 单测通过 SQL 字符串或 helper 断言 `chart-context` 不调用 momentum 查询，且 trend select 不包含 MACD/BOLL/KDJ 宽字段。
- 前端单测覆盖 Step 3 使用 `usePreviewChartContextQuery()`，不再使用 `usePreviewSecurityAnalysisQuery()`。
- 前端组件测试或类型测试覆盖 K 线图和右侧面板只读取轻量 response 字段。

完成标准：

- Step 3 analysis 调用路径唯一收敛到 `/rearview/strategy-preview/chart-context`。
- Step 3 analysis 不再要求前端重复发送完整 rule。
- API 名称和文档明确 chart context 不是 membership proof。
- Step 3 UI 截图和交互表现不变。

### Phase 4：删除 Step 3 analysis 脏字段和冗余查询

目标：字段没被当前 Step 3 页面使用，就不请求、不返回、不查询底层 mart。

实施项：

1. 后端查询 helper 收敛：
   - 新增或拆分 `chart_context_chart_quote_select_columns()`，只选择当前复权模式 OHLC 和 volume。
   - 新增或拆分 `chart_context_selected_quote_select_columns()`，只选择 `open_price`、`high_price`、`low_price`、`close_price`、`prev_close_price`、`pct_change`、`pct_amplitude`、`volume`、`amount`、`limit_up_price`、`limit_down_price`、`a_market_cap`、`pe_ttm`、`roe`。
   - 新增或拆分 `chart_context_trend_select_columns()`，只选择 MA5/MA10/MA30。
   - `chart-context` 路径不调用 `query_analysis_momentum_rows()`。
2. 前端类型收敛：
   - `StockPoolPreviewWorkbench` 不再依赖 `SecurityAnalysisResponse`。
   - `CandlestickChart` 移除 Step 3 路径对 `price_overlays` 的 fallback；MA 数据必须来自 `chart.series[].ma`。
   - 删除 Step 3 请求里的 `include_quote_rows`。
3. 脏字段清单验收：
   - Step 3 network payload 不出现 `kdj`、`rsi`、`macd`、`boll`。
   - Step 3 network payload 不出现 `price_overlays`、`indicator_panels`、`sources` 和空 `quote_rows`。
   - Step 3 chart context SQL 不包含 KDJ、RSI、MACD、BOLL 宽字段。
4. 保护真实 momentum 依赖：
   - 如果 Step 1/2 规则使用 `kdj_j_value` 等 momentum metric，preview timeline/latest/pool-page 仍可通过 metric catalog 查询 `mart_stock_momentum_indicator_daily`。
   - 本阶段只删除 Step 3 chart context 的固定 momentum 查询，不改 preview 规则执行依赖。

测试策略：

- Rust 单测断言 chart context SQL select 不包含 `kdj_`、`rsi`、`macd`、`boll` 字段。
- Rust 单测断言 preview 规则使用 momentum metric 时，preview planning 仍保留对应 mart 依赖。
- 前端单测或快照断言 Step 3 chart context response 类型没有脏字段。
- 浏览器 network 检查记录 Step 3 chart context payload 字段。

完成标准：

- `mart_stock_momentum_indicator_daily` 不再因为 Step 3 chart context 固定查询而被访问。
- Step 3 trend 查询只选择 MA5/MA10/MA30。
- Step 3 selected quote 查询只选择右侧面板实际展示字段。
- Step 3 payload 和 ClickHouse 查询列明显减少，且 UI 表现不变。

### Phase 5：基于观测做真实瓶颈优化

目标：只有 Phase 0 baseline 和 Phase 3/4 瘦身后的数据仍证明瓶颈存在时，才进入缓存、索引或 mart 结构优化。

候选实施项：

1. Preview 短 TTL 结果缓存：
   - cache key 至少包含 rule hash、range、row limit、offset、selected date、schema/catalog hash。
   - 不引入跨用户权限语义；当前无鉴权条件下只做进程内短 TTL dev/prototype 缓存或明确禁止生产持久化。
2. Security display 短 TTL cache：
   - 缓存 `mart_stock_basic_snapshot` 查询结果，减少同一批 `security_code` 重复查显示信息。
   - cache invalidation 使用短 TTL，不改变权威数据来源。
3. Chart mart 或排序键评估：
   - 只有 `chart-context` 收敛后仍慢，才评估 `mart_stock_trend_indicator_daily` 的个股窗口查询优化。
   - 需要先记录 ClickHouse scanned rows/bytes、read rows、read bytes 和 query elapsed。
4. 选股专用宽表评估：
   - 分别记录 preview timeline 和 preview latest scanned rows/bytes。
   - 只有 preview 规则路径成为主要瓶颈时，再评估 `mart_stock_rearview_metric_daily`。

测试策略：

- 每个优化必须附带优化前后 report。
- 缓存必须有单测覆盖 key 组成、TTL 过期和不同 rule/range 不串用。
- 任何 mart 结构调整必须另开 RFC 或 plan，并补 dbt/ClickHouse 专属验证。

完成标准：

- 每个进入实施的优化项都有 Phase 0 baseline 和优化后对比。
- 不把“删无用字段”和“改表结构”混在同一变更里。
- 若 Phase 3/4 后已达到目标延时，则 Phase 5 不启动。

## 禁止模式

- 不用 `a?.x || b?.x` 之类 fallback 掩盖字段归属不清；新增字段前必须沿源码确认唯一消费方和生产方。
- 不在 chart context 中接收完整 `RuleVersionSpec`。
- 不在 chart context 中重跑 preview pool-page 作为 membership 校验。
- 不为了 Step 3 当前页面保留 KDJ/RSI/MACD/BOLL、`price_overlays`、`indicator_panels`、`sources` 或空 `quote_rows`。
- 不把 Step 3 preview 持久化成正式 run。
- 不在没有 baseline 的情况下改 ClickHouse mart 结构。

## 允许保留的例外

- 通用 `/rearview/security-analysis` 可以继续保留给 run result 个股分析页或未来明确需要 KDJ/RSI/MACD/BOLL 的页面。
- `mart_stock_momentum_indicator_daily` 可以继续作为 preview 规则执行依赖；只要 Step 1/2 规则选择了 momentum metric，就不应被本计划删除。
- 旧 `/rearview/strategy-preview/timeline` 和 `/rearview/strategy-preview` 可以在兼容期保留；前端主路径切换后再决定归档或删除。
- `/rearview/strategy-preview/security-analysis` 可以短期保留但不作为 Step 3 主路径；后续按 membership 校验需求决定删除或重做为可复用 preview cache 的校验接口。

## 验证命令

文档-only 阶段：

```bash
make docs-check
git diff --check
```

前端变更阶段：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

Rust Rearview 变更阶段：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

端到端 smoke：

```bash
make racingline-dev
node scripts/check_playwright_cdp.mjs
```

浏览器验收重点：

- 从 `/strategies` Step 2 点击「股池预览」。
- Step 3 shell 和候选列表先出现。
- K 线区域可局部 loading，并在失败时只显示局部错误。
- Step 3 首屏 network 不触发 `strategy-backtests/validate`。
- 中期目标完成后，Step 2 到 Step 3 首屏只触发 `strategy-preview/open`，Step 3 图表触发 `strategy-preview/chart-context`。

## 完成标准

- Phase 0 baseline report 已写入 `docs/jobs/reports/`。
- Step 2 click 到 Step 3 shell render 不等待 analysis 和 backtest validate。
- 首屏 hard-blocking 请求数达到短期 2 个、中期 1 个的目标。
- Step 3 chart context payload 不包含 RFC 0030 标记的脏字段。
- Step 3 chart context 后端不调用 `query_analysis_momentum_rows()`。
- `strategy-backtests/validate` 只作为 Step 4/5 gate 触发。
- 前端、Rust 和 docs 质量门禁按涉及范围通过。
- 完成后将本计划归档，并新增 `docs/jobs/reports/YYYY-MM-DD-racingline-step2-step3-preview-latency-slimming.md` 记录验收结果、命令、样本和残余风险。

## 回滚策略

- Phase 1 可通过恢复 `openPreview()` 中 analysis prefetch await 和 validate query gate 回滚；回滚后必须记录用户等待链路恢复的原因。
- Phase 2 可保留旧 `timeline + preview` 前端路径作为短期 fallback，但 fallback 必须显式开关或单点代码路径，不能同时自动竞态调用两套首屏接口。
- Phase 3/4 可回滚到 `/rearview/security-analysis`，但不得重新把 analysis 放回 Step 2 到 Step 3 hard wait 链路。
- Phase 5 的缓存或 mart 结构优化必须可单独关闭；关闭后仍应保留 Phase 1 到 Phase 4 的减法成果。
