# Plan 0039: Racingline Run Result 个股分析页实施计划

日期：2026-06-15

状态：Completed

领域：racingline, rearview

关联系统：racingline, rearview, dbt marts

代码根：

- `app/racingline/`
- `engines/crates/rearview/`

关联文档：

- [RFC 0020: Racingline Run Result 个股分析页](../../RFC/0020-racingline-run-result-security-analysis-page.md)
- [RFC 0019: Racingline Rearview 前端工作台](../../RFC/0019-racingline-rearview-frontend-workbench.md)
- [RFC 0018: Rust Rearview 规则选股服务与 mart 指标库](../../RFC/0018-rust-stock-screening-service.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
- [Plan 0038: MA30 and adjusted quotes mart implementation](0038-ma30-and-adjusted-quotes-mart-implementation-plan.md)
- [mart_stock_quotes_daily 设计](../../design/dbt_layer/fleur_marts/mart_stock_quotes_daily.md)
- [mart_stock_trend_indicator 设计](../../design/dbt_layer/fleur_marts/mart_stock_trend_indicator.md)
- [mart_stock_momentum_indicator 设计](../../design/dbt_layer/fleur_marts/mart_stock_momentum_indicator.md)

相关规则：

- `fleur-harness`：计划、系统地图、验收报告和归档规则。
- `clickhouse-best-practices`：Rearview mart 查询必须遵守 `schema-pk-filter-on-orderby`、`query-join-filter-before`、`query-join-use-any`、`query-join-choose-algorithm` 和 `query-index-skipping-indices`。
- 实施阶段涉及 Rust、前端、shadcn 或 Playwright 时，再按 AGENTS.md 路由使用对应 skills。

## 目标

1. 将 `/runs/:runId` 结果表中的 `Open` 从打开 `SignalDetailSheet` 改为导航到 RFC 0020 定义的独立个股分析页。
2. 新增可刷新、可分享、可回退的路由：

```text
/runs/:runId/securities/:securityCode?trade_date=YYYY-MM-DD&source=signals|pool&adjustment=forward_adjusted|backward_adjusted|unadjusted
```

3. 在 Rearview 中新增页面级 analysis API，组合 PostgreSQL run snapshot 与 ClickHouse mart 当前查询值，并显式返回来源和价格口径 metadata。
4. 左侧展示同一 run、交易日和来源下的结果列表，并高亮当前证券。
5. 中间展示日 K 线、成交量、MA5/MA10/MA30 开关、KDJ、RSI、MACD 和 BOLL 指标面板。
6. K 线支持前复权、后复权和不复权切换，默认 `forward_adjusted`。
7. 右侧展示 `mart_stock_quotes_daily` 当前选中日期的一行字段，并和 PostgreSQL run snapshot 明确区分。
8. 固化符合用户分析逻辑的 UX：从 run result 进入后默认查看图表，信号日锚点和图表选中日期清晰分离，切换证券默认回到信号日，切换价格口径不丢失当前上下文。
9. 完成桌面三栏和移动端 tabs/分段视图验收，记录 Playwright CDP 截图、console、network 和关键交互证据。

## 非目标

1. 不新增交易、下单、风控、组合调仓或完整回测能力。
2. 不在前端直接访问 ClickHouse 或 PostgreSQL。
3. 不在前端重算 MA、KDJ、RSI、MACD、BOLL 或复权价格。
4. 不把当前 mart 查询值写回 PostgreSQL run snapshot。
5. 不新增 PostgreSQL 表，除非实现时发现现有 run result schema 无法表达 RFC 0020 的 result snapshot 边界。
6. 不新增 raw、staging、intermediate 或 calculation 层访问路径；Rearview 只能读取 mart 层。
7. 不把 `price_ma_28` 或其他窗口近似为 `price_ma_30`。
8. 不默认展示信号日之后的数据；第一版默认 K 线结束日期仍为 URL `trade_date`。
9. 不支持任意自定义指标公式、分钟线、分时图或任意 MA 窗口编辑。

## 当前事实基线

1. Racingline 当前路由只有 `/runs`、`/runs/:runId`、`/rules` 和 `/metrics`，尚无 `/runs/:runId/securities/:securityCode`。
2. `app/racingline/src/features/runs/components/run-results.tsx` 中 `Buy signals` 的 `Open` 当前打开 `SignalDetailSheet`；`Pool` 表当前没有独立 `Open` 入口。
3. `SignalDetailSheet` 展示的是 PostgreSQL run snapshot 中的 `score_breakdown` 和 `selected_metrics`，不读取 ClickHouse mart。
4. `app/racingline/package.json` 已包含 React Router、TanStack Query、Zustand 和 `lightweight-charts`，可以复用现有前端工程栈。
5. Rearview 当前已有：
   - `GET /rearview/runs/{run_id}`
   - `GET /rearview/runs/{run_id}/days`
   - `GET /rearview/runs/{run_id}/pool`
   - `GET /rearview/runs/{run_id}/signals`
6. Rearview 尚无 `GET /rearview/runs/{run_id}/securities/{security_code}/analysis` 或等价页面级组合接口。
7. Rearview 已有 ClickHouse HTTP client、mart database 配置和 readiness 检查，但当前 ClickHouse 查询层主要服务选股运行与交易日发现，需要新增面向单证券时间窗的 typed query。
8. `mart_stock_quotes_daily` 已按 Plan 0038 暴露未复权、前复权和后复权 OHLC，排序键为 `(security_code, trade_date)`。
9. `mart_stock_trend_indicator` 已暴露 `price_ma_5`、`price_ma_10`、`price_ma_30`、BOLL 和 MACD，排序键为 `(trade_date, security_code)`，指标口径基于 `close_price_forward_adj`。
10. `mart_stock_momentum_indicator` 已暴露 RSI 和 KDJ，排序键为 `(trade_date, security_code)`，指标口径基于 `close_price_forward_adj`。
11. RFC 0020 已补充 UX 交互原则，明确 `Signal day` 与 `Selected day` 分离、证券切换重置 selected quote date、移动端从 `Open` 进入默认展示 `Chart`、以及分析工作台视觉约束。
12. 当前实现尚未建立这些 UX 状态边界；后续实现不能只完成 API 和图表渲染后就视为通过。

## 实施阶段

### Phase 0: 冻结接口和数据语义

目标：在写代码前确认 API contract、价格口径和验收数据，避免前后端并行实现时出现语义漂移。

任务：

1. 将 RFC 0020 的 response 草案收敛为 Rust/TypeScript 可共享的字段清单，明确必填、可空、enum 和 metadata。
2. 确认 `adjustment` enum 只允许 `forward_adjusted`、`backward_adjusted`、`unadjusted`；未知值返回 validation error。
3. 确认 `source` enum 只允许 `signals` 和 `pool`，且 requested security 必须属于该 run、trade date 和 source。
4. 确认 `ma_windows` 第一版只接受 `5`、`10`、`30` 子集。
5. 明确趋势/动量指标当前只有前复权口径；当 K 线切换到后复权或不复权时，API 和 UI 必须标记指标口径，不暗示指标随 K 线重算。
6. 准备一个可用于验收的本地 run id、trade date 和 security_code；如果 RFC 中示例 run 不存在，应在实施报告中记录实际使用样本。
7. 将 RFC 0020 的 UX 交互原则转成实现 checklist，至少覆盖：进入页面默认视图、证券切换、图表日期选择、价格口径切换、MA 开关、右侧指标同步、移动端 tabs 和局部 loading。
8. 确认 `Signal day`、`Selected day`、`当前 mart 查询值` 和 `PostgreSQL run snapshot` 的 UI 标签命名，避免实现阶段临时写出互相矛盾的文案。

完成标准：

1. 后端 response struct、前端 TypeScript type 和 RFC 0020 没有字段命名冲突。
2. 数据口径约定写入 implementation checklist 或实施报告。
3. 确认 Plan 0038 的 MA30 和复权 OHLC 前置项不再阻塞本计划。
4. UX checklist 已进入 issue/PR checklist 或 job report 模板，不能只保留在 RFC 段落中。

验证命令：

```bash
make docs-check
git diff --check
```

### Phase 1: Rearview analysis API 骨架和 PostgreSQL snapshot 校验

目标：先让新页面能以 run result 为锚点恢复，不允许脱离 run 上下文展示任意证券行情。

任务：

1. 在 `engines/crates/rearview/src/api/mod.rs` 增加：

```text
GET /rearview/runs/{run_id}/securities/{security_code}/analysis
```

2. 增加 query 参数类型：
   - `trade_date`
   - `source`
   - `adjustment`
   - `quote_end_date`
   - `lookback_trading_days`
   - `quote_start_date`
   - `ma_windows`
3. 增加 domain/API response 类型：
   - `SecurityAnalysisResponse`
   - `ResultSnapshot`
   - `SourceMetadata`
   - `ChartWindow`
   - `ChartSeriesRow`
   - `QuoteRow`
   - `MaSeries`
   - `KdjSeries`
   - `RsiSeries`
   - `MacdSeries`
   - `BollSeries`
4. 在 PostgreSQL gateway 增加单行 snapshot 读取能力，或复用现有 list query 但必须精确过滤 `run_id`、`trade_date`、`security_code` 和 `source`。
5. `source=signals` 时返回 `rank`、`score`、`score_breakdown` 和 `selected_metrics`。
6. `source=pool` 时返回 `score`、`signal_rank` 和 `selected_metrics`；没有 TopN 信号时 `signal_rank` 可为空。
7. 当 requested security 不属于该 run result 时返回 404 或 validation error，不继续查询 ClickHouse mart。
8. 保持现有 `/signals` 和 `/pool` 列表接口向后兼容。

完成标准：

1. 新 endpoint 能只返回 result snapshot 和空 mart payload 的临时结构，便于前端尽早联调。
2. 不存在“URL 证券不属于 run 仍展示 K 线”的路径。
3. Rust 单元测试覆盖 `source`、`adjustment`、`ma_windows` 和 result membership 校验。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy -p rearview --all-targets --all-features -- -D warnings
cargo test -p rearview
```

### Phase 2: Rearview ClickHouse mart 时间窗查询

目标：从 mart 层读取单证券时间窗数据，并在 Rust 中组装成图表友好的序列。

任务：

1. 在 ClickHouse query layer 增加 typed 查询方法，读取：
   - `fleur_marts.mart_stock_quotes_daily`
   - `fleur_marts.mart_stock_trend_indicator`
   - `fleur_marts.mart_stock_momentum_indicator`
2. `mart_stock_quotes_daily` 查询必须按 `security_code = ?` 和 `trade_date` 窗口过滤，符合 `schema-pk-filter-on-orderby`。
3. `mart_stock_trend_indicator` 和 `mart_stock_momentum_indicator` 至少按 `trade_date` 窗口过滤，再按 `security_code` 过滤；不要做全市场 date-less 或 security-less 大扫描。
4. 优先用三个小查询在 Rust 中按 `(security_code, trade_date)` merge，避免为了页面响应把三张大 mart 直接 join 后再过滤。
5. 如果实现选择 ClickHouse join，必须先用 filtered subquery 再 join，符合 `query-join-filter-before`；只需要单匹配时使用 `LEFT ANY JOIN`，符合 `query-join-use-any`。
6. `adjustment` 只能映射到固定 OHLC 列：
   - `forward_adjusted` -> `*_forward_adj`
   - `backward_adjusted` -> `*_backward_adj`
   - `unadjusted` -> 未复权 `open_price`、`high_price`、`low_price`、`close_price`
7. 默认 `quote_end_date = trade_date`，默认 `lookback_trading_days = 240`，最大值为 1000。
8. 如果提供 `quote_start_date`，优先使用显式日期窗口；否则先按 end date 反向取最近 N 个 quote rows，再得到 start date。
9. `chart.series` 按 `trade_date` 升序返回；OHLC 任一关键字段为空时，该日不输出 candlestick OHLC，或用明确 nullable 字段让前端跳过。
10. `quote_rows` 至少包含右侧指标面板需要的全部 `mart_stock_quotes_daily` 字段，`selected_quote` 对应当前选中日期，缺失时为 `null`。
11. API response 的 `sources` 必须标记：
    - source database/table
    - `value_semantics = current_mart_query`
    - trend/momentum 指标口径为 `forward_adjusted`
12. 记录代表性 query 的 `EXPLAIN indexes = 1` 或 query log 观测；如发现 trend/momentum 单证券时间窗查询仍过慢，再评估是否需要后续专用 chart mart，而不是在本计划中临时改 mart `ORDER BY`。

完成标准：

1. analysis API 对存在的 run result 返回非空 `chart.series`。
2. K 线 OHLC 会随 `adjustment` 参数实际切换。
3. MA5、MA10、MA30 均来自 `mart_stock_trend_indicator` 字段，缺失时按缺失返回，不用其他窗口代替。
4. RSI、KDJ、MACD 和 BOLL 均来自 mart 字段，不在 Rust 或前端重算。

验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

本地服务启动后追加 smoke：

```bash
make racingline-dev
curl -fsS "http://127.0.0.1:34057/rearview/runs/<run_id>/securities/<security_code>/analysis?trade_date=<date>&source=signals&adjustment=forward_adjusted" | jq .
```

### Phase 3: Racingline API types、路由和结果页入口

目标：让前端可以通过 URL 恢复个股分析页，并把现有 `Open` 行为切到导航。

任务：

1. 在 `app/racingline/src/types/rearview.ts` 增加 analysis response 和 query 类型。
2. 在 `app/racingline/src/api/rearview.ts` 增加 `getSecurityAnalysis`。
3. 在 `app/racingline/src/api/queryKeys.ts` 和 hooks 中增加 analysis query key/hook。
4. 在 `App.tsx` 增加路由：

```text
runs/:runId/securities/:securityCode
```

5. 新增 route component，例如 `SecurityAnalysisPage.tsx`，负责 URL 参数规范化、query 调用和页面编排。
6. `Buy signals` 表的 `Open` 改为导航到 RFC 0020 URL，不再作为主行为打开 `SignalDetailSheet`。
7. `Pool` 表增加 `Open` 入口，导航时使用 `source=pool`。
8. URL 缺少 `adjustment` 时补为 `forward_adjusted`；缺少 `trade_date` 或 `source` 时，先尝试回到 run detail 默认值并跳转到规范 URL。
9. 保留或移除 `SignalDetailSheet` 需在实现时明确：如果保留，只能作为辅助查看 run snapshot 的入口，不能继续承担 `Open` 主行为。
10. 新页面返回 `/runs/:runId` 时应保留用户进入时的 `trade_date` 和 `source` 语境，避免用户返回后丢失原列表上下文。
11. 证券切换必须更新 URL；图表 selected date 不写入 URL，且证券切换后重置为 URL `trade_date`。

完成标准：

1. 从 `/runs/:runId` 的 signals 和 pool 都能进入新页面。
2. 刷新新页面可以重新加载，不依赖上一页 React state。
3. 浏览器前进/后退能在不同证券和 adjustment 间恢复。
4. 从新页面返回 run detail 后，用户能继续看到原 trade date/source 的结果语境。
5. API client、query key 和 hook 有定向测试覆盖 query 参数序列化和默认值。

验证命令：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run test
```

### Phase 4: 个股分析页 UI 和图表实现

目标：实现 RFC 0020 的三栏工作流和移动端降级，不把图表塞进临时调试页面。

任务：

1. 页面采用桌面三栏布局：
   - 左侧 result list：`18rem` 到 `22rem`
   - 中间 chart workspace：占剩余主空间
   - 右侧 mart indicators：`22rem` 到 `26rem`
2. 移动端使用 `Results`、`Chart`、`Indicators` tabs 或分段视图；从 `Open` 进入默认落在 `Chart`。
3. 左侧列表复用现有 `/signals` 和 `/pool` hooks，按当前 `source`、`trade_date` 和分页参数加载，并高亮当前证券。
4. 中间主图使用 `lightweight-charts` 渲染 candlestick、volume 和结果日期 marker。
5. 主图默认叠加 MA5、MA10、MA30，并提供三个独立开关；开关只控制显示，不改变后端请求口径。
6. `adjustment` 使用分段控件或 select 控件切换，并更新 URL。
7. KDJ、RSI、MACD 和 BOLL 面板和主图共享横向时间轴；NULL 点不补 0、不前向填充。
8. 右侧指标面板按 RFC 0020 分组展示 `mart_stock_quotes_daily` 字段，并在头部标记 `当前 mart 查询值`。
9. 右侧另设 run snapshot 区块，标记 `PostgreSQL run snapshot`，展示 score、rank/signal_rank、score_breakdown 和 selected_metrics。
10. 图表选中日期第一版保存在本地状态，默认 URL `trade_date`；crosshair/click 改变选中日期时，同步右侧 `selected_quote`。
11. 加载、空状态、错误状态和图表容器必须有稳定尺寸，避免 hover、开关、resize 或空数据造成布局跳动。
12. 不使用嵌套卡片堆叠页面主体；三栏应是工作台布局，重复项、面板和错误状态再按现有组件体系处理。
13. 图表工具栏顺序固定为：价格口径、MA 开关、回到信号日、数据口径标签；控件必须贴近图表，不放到右侧指标或页面顶端远离上下文的位置。
14. `Signal day` marker 和 `Selected day` marker 必须使用不同视觉样式；右侧顶部同时显示两者。
15. 证券切换时保留 MA 开关状态，但 selected quote date 重置为 signal day；价格口径切换时保留 selected quote date。
16. 列表搜索、分页和来源标签保持在左栏顶部；当前证券不在当前分页时，需要显示当前证券摘要或请求包含它的页。
17. `score_breakdown` 和 `selected_metrics` 默认折叠，避免长 JSON 挤压行情指标。
18. 右侧 quote 字段空值显示短横线或空状态，不显示 `null` 字符串；布尔状态使用状态标签。
19. 主图、成交量和指标面板的 legend、tooltip、y 轴和 loading overlay 不得遮挡关键走势。
20. 页面视觉保持 Racingline 工作台风格：紧凑、可扫描、少装饰；禁止营销式 hero、装饰背景、大面积渐变、漂浮 section card 和嵌套卡片。
21. 所有长文本、按钮、图例和表格字段必须在桌面和移动端不溢出、不互相遮挡；颜色表达涨跌和状态时需要文本、标签或线型辅助。

完成标准：

1. 桌面首屏同时可见结果列表、图表和右侧指标。
2. 移动端不出现三栏挤压、文本溢出或控件重叠。
3. 图表在有数据时非空，切换 adjustment 后 candlestick 数据变化。
4. MA 开关可以独立隐藏/恢复，对缺失 MA 不做替代。
5. 当前 mart 查询值和 run snapshot 视觉上分开。
6. 用户能明确区分 signal day 和 selected day，且证券切换/价格口径切换的状态变化符合 RFC 0020。
7. 页面截图能体现分析工作台而非营销页或卡片堆叠页。

验证命令：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm run test
npm run build
```

### Phase 5: 错误状态、性能和可用性收口

目标：覆盖 RFC 0020 的空状态和失败路径，避免页面在部分数据缺失时整体不可用。

任务：

1. run 不存在时展示 run not found，并提供返回 `/runs`。
2. result row 不存在时展示该证券不属于当前 run/trade/source。
3. mart 无 K 线数据时保留 run snapshot，图表显示无行情数据。
4. 复权 OHLC 缺失时保留右侧 quote panel，并提示该价格口径暂无 mart 数据。
5. 技术指标缺失时只让对应面板为空，不影响主 K 线。
6. ClickHouse 查询失败时，后端尽量返回 result snapshot 和 mart 错误 metadata；如果实现复杂度过高，前端至少要把错误边界限制在图表/指标区域。
7. PostgreSQL snapshot 查询失败时作为页面主错误，不展示脱离 run 的 mart 数据。
8. 后端限制 payload 大小，`lookback_trading_days` 最大 1000；前端不允许构造无限窗口请求。
9. 记录 network payload 大小和首屏渲染观测；如图表响应过大，进入后续优化计划，而不是扩大默认窗口。
10. 切换证券、切换口径、打开/关闭 MA 和点击图表日期时，只显示局部 loading 或状态更新，不整页闪白。
11. 错误状态必须保留用户的 run/source/trade/security 上下文，并提供返回结果页或重试当前区域的动作。

完成标准：

1. RFC 0020 空状态表中的场景都有明确 UI 行为。
2. API validation error 和 ClickHouse error 不会造成前端未处理 promise 或 console runtime error。
3. 默认窗口下页面交互没有明显卡顿；如有性能风险，实施报告记录后续优化项。
4. 失败和空状态不会破坏三栏或移动端 tabs 的稳定布局。

验证命令：

```bash
cd engines
cargo test -p rearview

cd ../app/racingline
npm run test
npm run build
```

### Phase 6: 本地联调、浏览器验收和文档收口

目标：以真实 Rearview API 和浏览器截图完成 RFC 0020 验收。

任务：

1. 启动本地 dev 环境：

```bash
make racingline-dev
```

2. 检查 CDP 连接：

```bash
node scripts/check_playwright_cdp.mjs
```

3. 通过 Playwright CDP 验收：
   - `/runs/:runId` signals `Open` 导航
   - `/runs/:runId` pool `Open` 导航
   - 新页面刷新恢复
   - adjustment 三种口径切换
   - MA5/MA10/MA30 开关
   - 证券切换后 selected day 回到 signal day
   - 图表日期点击后右侧 selected quote 更新，signal day 标签仍保留
   - 价格口径切换后 selected day 保持不变
   - KDJ、RSI、MACD、BOLL 面板非空或正确空状态
   - 右侧 quote 字段和 run snapshot 区分
   - 移动端从 `Open` 进入默认展示 Chart tab
   - 左侧列表、图表工具栏、右侧指标没有文本溢出或控件遮挡
   - 桌面视口截图
   - 移动视口截图
   - console 无 runtime error
   - network analysis API 返回 200 或预期错误
4. 更新系统地图：
   - [System: Racingline](../../systems/racingline.md)：新增个股分析页入口和验收报告链接。
   - [System: Rearview](../../systems/rearview.md)：新增 analysis API 事实。
5. 新增 job report 到 `docs/jobs/reports/`，记录命令、样本 run、截图路径、network/console 结果和遗留问题。
6. 完成后将本计划移入 `docs/plans/archive/`，并同步 `docs/plans/README.md`。

完成标准：

1. RFC 0020 验收标准全部通过，或未通过项有明确 blocker、owner 和后续文档落点。
2. Rust、前端和 docs 最小质量门禁通过。
3. 浏览器验收有截图、console 和 network 证据链。
4. 计划归档前，系统地图和 job report 已更新。

最终验证命令：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm run lint
npm run typecheck
npm run test
npm run build

cd ../..
make docs-check
git diff --check
```

## 禁止模式

1. 前端不得直接读取 ClickHouse 或 PostgreSQL。
2. Rearview 不得绕过 mart 层读取 raw、staging、intermediate 或 calculation 表。
3. 不得在 React、Rust API 层或 SQL 查询层重算 RFC 要求的技术指标。
4. 不得用 `price_ma_28` 替代 `price_ma_30`。
5. 不得把当前 mart 查询值写回 run snapshot，或在 UI 中暗示它们是运行时快照。
6. 不得构造全市场、无限日期窗口或 join 后再过滤的大查询。
7. 不得把 `adjustment`、`source` 或 `ma_windows` 当成未校验字符串拼进 SQL。
8. 不得在缺少 `trade_date` 或 `source` 时随意猜测证券归属。
9. 不得默认展示信号日之后的数据。
10. 不得把图表验收只停留在 lint/build；必须做真实浏览器截图和 network/console 检查。
11. 不得把 `Signal day` 和 `Selected day` 混用，或在证券切换后沿用上一只证券的 selected quote date。
12. 不得用整页 loading、页面闪白或重置左侧列表来响应局部交互。
13. 不得使用营销式 hero、装饰性背景、大面积渐变、卡片套卡片或使图表退居次要位置的布局。
14. 不得让按钮、tooltip、legend、表格文本或指标字段在移动端/桌面端溢出、遮挡或改变固定布局尺寸。

## ClickHouse 查询规则检查

1. Per `schema-pk-filter-on-orderby`，`mart_stock_quotes_daily` 查询必须使用 `security_code = ?` 和 `trade_date` 范围，匹配 `(security_code, trade_date)` 排序前缀。
2. Per `schema-pk-filter-on-orderby`，trend/momentum 当前排序为 `(trade_date, security_code)`，查询至少必须使用 `trade_date` 范围；`security_code` 过滤不能替代日期窗口。
3. Per `query-join-filter-before`，如使用 ClickHouse join，必须先在子查询中按 security/date window 过滤各 mart。
4. Per `query-join-use-any`，只需要唯一 `(security_code, trade_date)` 匹配时使用 `LEFT ANY JOIN`，避免重复行扩大图表序列。
5. Per `query-join-choose-algorithm`，如单 SQL join 在代表性窗口下出现内存风险，应改回多个小查询在 Rust 中 merge，或显式评估 join algorithm。
6. Per `query-index-skipping-indices`，如果后续需要优化非排序键过滤，应先用 `EXPLAIN indexes = 1` 在真实数据上证明收益，再新增 skip index；本计划第一版不改 mart schema。

## 最小完成标准

1. `Open` 主行为已经变成导航到独立个股分析页。
2. 新页面刷新后能用 URL 和 Rearview API 恢复完整状态。
3. Rearview analysis API 只在确认 result membership 后读取 mart。
4. K 线、MA、KDJ、RSI、MACD、BOLL 和右侧 quote 字段均来自 mart/API。
5. 当前 mart 查询值和 PostgreSQL run snapshot 在 API metadata 和 UI 中都明确分离。
6. 证券切换、图表日期选择、价格口径切换、MA 开关、返回 run detail 和移动端 tabs 的用户路径符合 RFC 0020 的 UX 交互原则。
7. 桌面和移动端浏览器验收通过，截图证明页面可扫描、控件不遮挡、图表是主工作区。
8. 相关系统地图、job report 和计划归档动作完成。
