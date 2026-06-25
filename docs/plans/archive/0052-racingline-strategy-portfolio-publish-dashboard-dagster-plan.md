# Plan 0052: Racingline 策略组合发布、看板真实数据与 Dagster 日运行实施计划

日期：2026-06-24

状态：Completed

领域：racingline, rearview, data-platform

关联系统：racingline, rearview, data-platform, deploy-ops

代码根：

- `app/racingline_new/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/migrate/`
- `pipeline/scheduler/`

关联文档：

- [RFC 0029: Racingline 回测结果发布为策略组合与 Dagster 日运行资产](../../RFC/0029-racingline-strategy-portfolio-publish-and-daily-run.md)
- [RFC 0028: Racingline 策略回测 Step 5 异步执行方案](../../RFC/0028-racingline-strategy-backtest-step5.md)
- [RFC 0022: 组合数据面迁移 ClickHouse 与绩效指标分层](../../RFC/0022-portfolio-data-plane-clickhouse-and-metrics.md)
- [Plan 0051: Racingline 策略回测 Step 5 异步执行实施计划](0051-racingline-strategy-backtest-step5-implementation-plan.md)
- [Plan 0043: 组合数据面迁移 ClickHouse 第一阶段实施计划](../0043-portfolio-data-plane-clickhouse-phase1-implementation-plan.md)
- [Plan 0044: 组合绩效指标、dbt 输入与交易级指标实施计划](../0044-portfolio-performance-metrics-implementation-plan.md)
- [System: Racingline](../../systems/racingline.md)
- [System: Rearview](../../systems/rearview.md)
- [System: Data Platform](../../systems/data-platform.md)

## 文档 review 结论

已 review RFC 0029，并补充以下实现缺口，确保方案能进入实际开发：

| 缺口 | 处理结论 |
|---|---|
| 策略 ID 不应使用名称、日期序号或 backtest run id | 新增内部 `strategy_portfolio_id`，使用 UUIDv7 或 ULID；新增用户可读 `portfolio_code = SP-YYYYMMDD-XXXXX` |
| 不需要 strategy portfolio version | 第一版只建 `strategy_portfolio`，发布配置直接固化在组合记录上；修改策略时重新回测并创建新组合 |
| 发布后马上回看板但尚未有 daily run | dashboard 支持 `live_status = pending_first_run`；此时显示 source backtest summary/curve，并标明不是 live 运行结果 |
| 当前 simulation engine 不支持从昨日持仓增量恢复 | 第一版 daily run 使用全窗口重算：从 `live_start_date` 到 `trade_date` 重新生成信号并模拟，最新 daily run 持有完整 live curve |
| 看板不能只给裸列表 | 新增 dashboard view model，直接覆盖 `racingline_new` 当前首页卡片和详情页字段 |
| 今日信号缺证券名称和 score breakdown | 第一版证券名称查询时补，score breakdown 若不落库则前端隐藏得分项列；不伪造 mock |
| Dagster 不应为每个组合生成动态 asset | 新增稳定日分区 asset `rearview/strategy_portfolio_daily_runs`，materialization metadata 记录 portfolio ids 和计数 |

## 目标

1. 从 Step 5 成功 backtest run 发布正式 `strategy_portfolio`。
2. 生成稳定内部 ID 和用户可读 `portfolio_code`。
3. Step 5 下方增加「建立组合」按钮和紧凑确认面板，确认 Step 1、Step 2、Step 4 和 Step 5 source backtest 摘要。
4. 发布成功后返回 `/dashboard`，新策略组合来自 Rearview API，而不是 `portfolioCards` mock。
5. `app/racingline_new` 看板首页和详情页使用真实 dashboard/portfolio detail APIs。
6. Rearview 支持 active strategy portfolios 的 daily run control plane、NATS outbox 和 worker 计算。
7. Dagster 新增稳定日分区 asset/job/schedule，定时触发所有 active strategy portfolios 的 daily run。
8. 第一版 daily run 复用现有全窗口 portfolio simulation，不实现增量持仓恢复。
9. 所有 UI 展示数据必须来自真实 Rearview API、PostgreSQL control plane、ClickHouse portfolio/calculation data plane 或 worker 计算结果，不采用 mock、fixture、前端生成数据或静态 fallback。
10. 前端不改变当前 `racingline_new` 看板和详情页的设计、布局、排版、信息层级和交互骨架；本计划只替换数据来源并补齐必要状态展示。

## 非目标

1. 不引入 `strategy_portfolio_version`。
2. 不实现登录、鉴权、用户隔离或权限审计。
3. 不实现实盘交易、券商下单、撤单、成交回报或真实资金账户。
4. 不让 Racingline 直接访问 PostgreSQL 或 ClickHouse。
5. 不在前端计算权威信号、成交、持仓、净值或绩效指标。
6. 不为每个用户策略创建独立 Dagster asset definition。
7. 不在第一版实现增量账本恢复；后续若 daily run 全窗口重算成为性能瓶颈，再设计 incremental state。
8. 不保证 source backtest 与 live run 可以被混成一个连续收益序列；两者必须在 API 和 UI 中区分。
9. 不重做看板视觉设计，不改变卡片网格、详情页模块顺序、表格列结构和现有紧凑信息密度。

## 硬性实施约束

1. 禁止 mock：开发完成路径中不得继续使用 `portfolioCards`、`holdingsByPortfolioId`、`buildStrategySignalPools()`、`buildDetailRebalanceRecords()` 或任何等价静态/生成数据作为业务展示来源。
2. 禁止 silent fallback：Rearview API 失败、字段缺失或计算结果为空时，UI 必须展示 loading/error/empty/unavailable 状态，不得回退到 mock 曲线、mock 指标、mock 信号或 mock 持仓。
3. 真实数据来源边界：策略组合元数据来自 PostgreSQL `strategy_portfolio` / `strategy_portfolio_daily_run`；净值、持仓、目标、订单、成交、事件来自 ClickHouse `fleur_portfolio`；绩效和交易级指标来自 ClickHouse `fleur_calculation`；证券名称通过 Rearview 查询 `mart_stock_basic_snapshot` 后返回给前端。
4. 前端只做格式化：百分比、货币、日期、badge 颜色和空态文案可在前端格式化；指标值、信号分数、持仓、收益贡献、调仓理由、净值和基准曲线必须由后端返回。
5. 保持 UI 设计：`PortfolioOverviewBoard`、`PortfolioOverviewCard`、`StrategyDetailPage` 的布局结构、卡片排版、图表位置、表格列结构和现有交互骨架保持不变；只允许为真实数据增加必要的 loading/error/empty/pending-first-run 状态。
6. 设计变更隔离：如果实现中发现当前设计无法承载真实数据，不得在本计划内直接重设计页面；必须先补 RFC/plan 说明并单独验收。

## 阶段执行口径

1. 每个阶段交付都必须能说明新增展示字段的真实来源：Rearview API、PostgreSQL control plane、ClickHouse portfolio/calculation data plane、worker 计算结果或证券基础信息查询。
2. 前端开发不得用 mock 数据推进视觉联调；若后端接口尚未完成，应先落 loading/empty/error/pending 状态，等待真实接口联调。
3. 后端缺少可计算字段时，不在 API 中返回占位业务值；字段应返回 null、空数组或 unavailable source marker，并由前端按现有布局展示不可用状态。
4. 所有页面改造以“保持当前 `racingline_new` 视觉和信息结构”为边界；不得借真实数据接入重做卡片密度、详情页模块顺序、表格结构、图表位置或交互路径。
5. 代码 review 时将 `rg "portfolioCards|holdingsByPortfolioId|buildStrategySignalPools|buildDetailRebalanceRecords"` 作为前端 mock 清退检查项；命中生产路径即视为未完成。

## 当前事实基线

1. `app/racingline_new/src/components/racingline/dashboard/portfolio-data.ts` 定义 `portfolioCards` mock，首页卡片字段包括 `name/startDate/simulationDays/latestNav/recentChange/returns/risk/todaySignals/curve`。
2. `app/racingline_new/src/routes/strategy-detail-page.tsx` 仍用 `holdingsByPortfolioId`、`buildStrategySignalPools()` 和 `buildDetailRebalanceRecords()` 生成详情页 mock 数据。
3. Step 5 已有 `StrategyBacktestRunRecord`、nav、performance、targets、orders、trades、positions、events 类型和 API wrapper。
4. Rearview 已有 `strategy_backtest_run` control plane 和 result wrapper，source backtest 结果用 `portfolio_run_id = strategy_backtest_run_id` 写入 `fleur_portfolio` / `fleur_calculation`。
5. ClickHouse portfolio data plane 已有 `portfolio_nav_daily`、`portfolio_position_day`、`portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_event` 和 `portfolio_run_snapshot`。
6. ClickHouse calculation data plane 已有 `calc_portfolio_performance_metric`、`calc_portfolio_performance_metric_status`、`calc_portfolio_closed_trade` 和 `calc_portfolio_trade_metric`。
7. Rearview generic portfolio APIs 和 strategy backtest wrapper APIs 当前按 `portfolio_run_id` 或 `strategy_backtest_run_id` 查询，不按 `strategy_portfolio_id` 查询。
8. `PortfolioSimulationInput` 当前只有 `start_date`、`end_date`、`initial_cash`、signals 和 prices，不支持从已有持仓/现金状态增量恢复。
9. `rearview-portfolio-worker` 已有 transient strategy backtest signal materialization，可复用为 strategy portfolio daily run 的信号生成基础。
10. Dagster 当前 `uv run dg list defs --json` 中有 portfolio/calculation source assets 和 dbt wrappers，但没有 strategy portfolio daily run asset、job 或 schedule。
11. `stock__daily_build_schedule` 当前 cron 为 `30 18 * * *`，strategy portfolio daily run schedule 必须晚于它。
12. 最新 Rearview migration 为 `pipeline/migrate/versions/rearview/0007_create_strategy_backtest_control_plane.py`，新增 strategy portfolio migration 应使用后续 revision。

## 数据模型决策

### `strategy_portfolio`

新增 PostgreSQL control-plane 表，建议 migration：

```text
pipeline/migrate/versions/rearview/0008_create_strategy_portfolio_control_plane.py
```

核心字段：

| 字段 | 说明 |
|---|---|
| `strategy_portfolio_id` | 内部主键，UUIDv7 或 ULID |
| `portfolio_code` | 用户可读编号，`SP-YYYYMMDD-XXXXX`，唯一索引 |
| `name` | 用户填写策略名称 |
| `status` | `active`、`archived` |
| `rule_snapshot` | 从 source `strategy_backtest_run.rule_snapshot` 复制 |
| `rule_hash` | 从 source backtest 复制 |
| `execution_config` | 从 source backtest 复制 |
| `execution_config_hash` | 从 source backtest 复制 |
| `benchmark_security_code` | 从 source backtest 复制 |
| `price_basis` | 第一版固定 `backward_adjusted` |
| `catalog_hash` | 从 source backtest 复制 |
| `required_metrics` | 从 source backtest 复制 |
| `required_marts` | 从 source backtest 复制 |
| `source_strategy_backtest_run_id` | 发布来源 |
| `source_result_attempt_id` | 发布时 pin 的 source result attempt |
| `source_period_key` | source backtest period |
| `source_start_date` / `source_end_date` | source backtest frozen range |
| `live_start_date` | daily run 起始交易日，默认 source end date 后的下一个交易日 |
| `latest_daily_run_id` | 最新成功或最新可见 daily run |
| `current_result_attempt_id` | 最新 live 结果 attempt；没有 daily run 时为空 |
| `ui_display_snapshot` | 发布面板展示快照，不参与计算 |
| `created_at` / `updated_at` / `archived_at` | 控制面时间 |

约束：

1. `portfolio_code` 唯一。
2. `name` 非空。
3. `status in ('active', 'archived')`。
4. `rule_snapshot`、`execution_config`、`required_metrics`、`required_marts`、`ui_display_snapshot` 使用 JSONB 类型并加 JSON type check。
5. `source_result_attempt_id` 必须非空。
6. 第一版不加 FK 到 ClickHouse result attempt；只对 PostgreSQL `strategy_backtest_run` 加 FK。

### `strategy_portfolio_daily_run`

新增 PostgreSQL control-plane 表，表示一次 active portfolio 到某个 `trade_date` 的 full-window live simulation。

核心字段：

| 字段 | 说明 |
|---|---|
| `strategy_portfolio_daily_run_id` | 内部主键，ULID |
| `strategy_portfolio_id` | 所属组合 |
| `run_start_date` | 本次 full-window simulation 起始日期，等于 portfolio `live_start_date` |
| `trade_date` | 本次运行终止交易日 |
| `status` | 复用 backtest worker 状态：`created/queued/compiling_signals/running_clickhouse/loading_market_data/calculating_nav/computing_performance/writing_results/succeeded/failed_*` |
| `dispatch_status` | `pending/published/publish_failed` |
| `worker_attempt_no`、`claimed_at`、`heartbeat_at`、`claim_expires_at` | worker claim/lease |
| `progress`、`summary`、`signal_summary`、`data_coverage_summary` | 可观测状态 |
| `error_type`、`error_message` | 错误 |
| `current_result_attempt_id` | 当前有效 live result attempt |
| `created_at`、`updated_at`、`started_at`、`completed_at` | 时间 |

约束：

```text
unique(strategy_portfolio_id, trade_date)
```

ClickHouse 写入约定：

```text
portfolio_run_id = strategy_portfolio_daily_run_id
result_attempt_id = current_result_attempt_id
portfolio_run_snapshot.execution_snapshot.source_kind = strategy_portfolio_daily_run
```

### `strategy_portfolio_daily_task_outbox`

新增 outbox 表，保持 HTTP/Dagster 创建 daily run 与 NATS 发布之间可恢复：

| 字段 | 说明 |
|---|---|
| `outbox_id` | 主键 |
| `strategy_portfolio_daily_run_id` | daily run id |
| `subject` | NATS subject |
| `payload` | typed task message payload |
| `status` | `pending/published/publish_failed` |
| `attempt_count` | 发布重试次数 |
| `created_at` / `updated_at` / `published_at` | 时间 |

NATS task message 新增：

```json
{
  "kind": "strategy_portfolio_daily_run",
  "daily_run_id": "01J..."
}
```

## API 合同

### Publish

```http
POST /rearview/strategy-portfolios
```

请求：

```json
{
  "source_strategy_backtest_run_id": "uuid",
  "source_result_attempt_id": "01J...",
  "name": "红利低波增强",
  "client_request_id": "optional-idempotency-key"
}
```

响应：

```json
{
  "strategy_portfolio_id": "01J...",
  "portfolio_code": "SP-20260624-K7Q9M",
  "name": "红利低波增强",
  "status": "active",
  "live_status": "pending_first_run",
  "source_strategy_backtest_run_id": "uuid",
  "source_result_attempt_id": "01J...",
  "live_start_date": "2026-06-25",
  "created_at": "2026-06-24T00:00:00Z"
}
```

后端校验：

1. source backtest 存在且 `status = succeeded`。
2. `source_result_attempt_id` 等于 source backtest 的 `current_result_attempt_id`。
3. name 非空。
4. `client_request_id` 幂等：同一 request hash 返回同一 portfolio。
5. `portfolio_code` 生成时遇到唯一冲突最多重试 5 次，仍失败返回 409。

### Dashboard

```http
GET /rearview/strategy-portfolios/dashboard
```

返回 RFC 0029 中定义的 dashboard view model。实现要求：

1. active portfolios 默认按 `created_at desc` 排序。
2. archived portfolios 默认不返回。
3. 有 latest daily run 时，live fields 从 latest daily run 的 ClickHouse result attempt 读取。
4. 没有 latest daily run 时，`live_status = pending_first_run`，live fields 为 null，curve 使用 source backtest curve，并显式标记 `curve_source = source_backtest`。
5. 不把 source backtest metric 写入 `live_summary`。

### Detail

```http
GET /rearview/strategy-portfolios/{strategy_portfolio_id}
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/nav
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/signals
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/signal-timeline
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/rebalance-records
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/positions
PATCH /rearview/strategy-portfolios/{strategy_portfolio_id}
```

实现要求：

1. `nav` 复用 Step 5 benchmark normalization 逻辑，返回 `strategy_nav`、`benchmark_nav`、`excess_return`。
2. `signals` 查询 latest daily run 的 `portfolio_target`；没有 live run 时可查 source backtest target，但响应必须标记 `source = source_backtest`。
3. `signal-timeline` 第一版从 `portfolio_target` 聚合 `target_count`；完整候选池数量缺失时不伪造 `signal_count`。
4. `rebalance-records` 泛化 Step 5 `build_strategy_backtest_rebalance_rows()`。
5. `positions` 查询 latest daily run 或指定 trade date 的 `portfolio_position_day`。
6. `PATCH` 第一版只支持 archive：`{"status":"archived"}`。

### Daily Run Batch

```http
POST /rearview/strategy-portfolios/daily-runs
```

请求：

```json
{
  "trade_date": "2026-06-24",
  "client_request_id": "dagster-2026-06-24"
}
```

响应：

```json
{
  "trade_date": "2026-06-24",
  "active_portfolio_count": 2,
  "created_run_count": 2,
  "skipped_run_count": 0,
  "daily_run_ids": ["01J...", "01J..."]
}
```

语义：

1. Rearview 枚举 `status = active` 且 `live_start_date <= trade_date` 的 portfolios。
2. 对 `(strategy_portfolio_id, trade_date)` 已存在的 daily run 返回 skipped。
3. 新 daily run 进入 outbox，由 worker 异步计算。
4. API 不同步等待 worker 完成。

## 实施阶段

### Phase 0: 合同和测试基线

**目标**：先固定类型和语义，避免前后端并行开发时漂移。

实现：

1. 在 `engines/crates/rearview-core/src/api/mod.rs` 增加 request/response struct 草案。
2. 在 `app/racingline_new/src/types/rearview.ts` 增加对应 TypeScript 类型。
3. 明确 `StrategyPortfolioDashboardCard.live_status`：

```text
pending_first_run | queued | running | succeeded | failed
```

4. 明确 `StrategyPortfolioCurvePoint.curve_source`：

```text
source_backtest | live_daily_run
```

5. 为 `portfolio_code` 生成函数写 Rust 单测：格式、长度、前缀、唯一冲突重试。

完成标准：

1. 类型字段覆盖 RFC 0029 当前看板首页和详情页字段。
2. 不存在前端-only fake 字段。
3. 每个 dashboard/detail 字段在类型注释或 API 合同中能追溯到真实数据来源，不为 UI 联调预留 mock-only 字段。
4. 记录当前首页和详情页布局基线，后续阶段只允许数据源替换和状态展示变化。

### Phase 1: Rearview control plane migration

**目标**：新增 strategy portfolio、daily run 和 outbox control plane。

实现：

1. 新增 Alembic migration `0008_create_strategy_portfolio_control_plane.py`。
2. 创建 `strategy_portfolio`、`strategy_portfolio_daily_run`、`strategy_portfolio_daily_task_outbox`。
3. 增加必要索引：

```text
idx_strategy_portfolio_status_created(status, created_at)
idx_strategy_portfolio_code(portfolio_code)
idx_strategy_portfolio_source_backtest(source_strategy_backtest_run_id)
idx_strategy_portfolio_daily_status_created(status, created_at)
idx_strategy_portfolio_daily_trade_date(trade_date)
idx_strategy_portfolio_daily_claim_expires(claim_expires_at)
idx_strategy_portfolio_daily_outbox_status_created(status, created_at)
```

4. `strategy_portfolio_daily_run` 加唯一约束 `(strategy_portfolio_id, trade_date)`。
5. `strategy_portfolio.source_strategy_backtest_run_id` FK 到 `strategy_backtest_run`。

完成标准：

1. `cd pipeline/migrate && uv run alembic upgrade head` 可执行。
2. migration downgrade 可回滚。

### Phase 2: Publish API

**目标**：从 succeeded Step 5 backtest 创建正式策略组合。

实现：

1. `RearviewPg` 新增：

```rust
create_strategy_portfolio(...)
get_strategy_portfolio(...)
get_strategy_portfolio_by_client_request_id(...)
archive_strategy_portfolio(...)
```

2. `api/mod.rs` 新增：

```text
POST /rearview/strategy-portfolios
GET /rearview/strategy-portfolios/{id}
PATCH /rearview/strategy-portfolios/{id}
```

3. publish API 从 `strategy_backtest_run` 复制 canonical snapshots，不信任前端提交配置。
4. 解析 `live_start_date`：source backtest `end_date` 后的下一个交易日；如果交易日历无法解析，创建失败。
5. 生成 `strategy_portfolio_id` 和 `portfolio_code`。

完成标准：

1. 只有 succeeded backtest 可发布。
2. 同一 `client_request_id` 幂等。
3. source attempt 不匹配时返回 409。

### Phase 3: Dashboard and detail read models

**目标**：后端一次性提供当前 `racingline_new` 看板所需数据。

实现：

1. 新增 `GET /rearview/strategy-portfolios/dashboard`。
2. 新增 detail APIs：`nav`、`signals`、`signal-timeline`、`rebalance-records`、`positions`。
3. 抽出 Step 5 现有辅助逻辑：

```text
benchmark normalized nav
daily_win_rate
rebalance rows aggregation
security display map
```

4. 没有 live daily run 时，dashboard/detail 使用 source backtest 结果作为 `source_backtest` 数据源，并保持 live fields null。
5. `signal-timeline` 第一版用 `portfolio_target` 聚合 target count；完整候选池数量缺失时 UI 隐藏“完整候选池 signal_count”。

完成标准：

1. `dashboard` API 能覆盖首页 `PortfolioCardData` 所有真实字段。
2. detail APIs 能替代 `holdingsByPortfolioId`、`buildStrategySignalPools()` 和 `buildDetailRebalanceRecords()`。
3. API 不返回 mock 值，不用 0 表示缺失指标。
4. 每个返回字段都能追溯到 PostgreSQL control plane、ClickHouse portfolio/calculation data plane、worker 计算结果或 `mart_stock_basic_snapshot` display 查询。

### Phase 4: Racingline publish panel and dashboard API integration

**目标**：前端完成 Step 5 发布组合和看板真实数据接入。

实现：

1. `app/racingline_new/src/api/rearview.ts` 增加 strategy portfolio API client。
2. `app/racingline_new/src/api/queryKeys.ts` 增加 query keys。
3. `strategy-page.tsx` 的 Step 5 成功态下方增加「建立组合」按钮。
4. 点击按钮打开紧凑面板，展示：

```text
策略名称输入
Step 1 条件摘要
Step 2 权重/评分摘要
Step 4 建仓/费率/风控摘要
Step 5 source backtest summary
```

5. 创建成功后 `navigate("/dashboard")`，并 invalidate dashboard query。
6. `PortfolioOverviewBoard` 改为读取 `useStrategyPortfolioDashboardQuery()`。
7. `strategy-detail-page.tsx` 改为读取 detail APIs。
8. 删除或隔离 `portfolioCards` mock，确保生产路径不 fallback。
9. 若 score breakdown 未落库，详情页隐藏“得分项”列或显示 unavailable，不生成假数据。
10. 保持现有看板和详情页排版：卡片网格、卡片 header/content、收益/风险/信号/净值区块顺序、详情页模块顺序、调仓记录表格列结构不变。

完成标准：

1. 成功 Step 5 可创建组合并返回看板。
2. 看板新卡片来自 API。
3. 详情页不再调用 mock builders。
4. `pending_first_run` 状态展示 source backtest 并明确标注。
5. 关闭 Rearview API 或返回错误时，页面显示错误/空态，不出现任何 mock 数据。
6. 视觉快照或浏览器验收确认布局与当前 `racingline_new` 看板设计保持一致，只发生真实数据替换和必要状态展示。
7. 所有前端业务数据请求必须通过 Rearview API client；不得在组件内生成净值、信号、持仓、调仓、收益或风险指标。

### Phase 5: Daily run control plane and worker

**目标**：Rearview 可为 active portfolios 创建 daily run，并由 worker 复用现有 simulation 路径计算。

实现：

1. API 新增 `POST /rearview/strategy-portfolios/daily-runs`。
2. `RearviewPg` 新增：

```text
create_strategy_portfolio_daily_runs_for_trade_date
list_pending_strategy_portfolio_daily_outbox
mark_strategy_portfolio_daily_outbox_published/failed
get_strategy_portfolio_daily_run
claim_strategy_portfolio_daily_run
update_strategy_portfolio_daily_progress
finalize_strategy_portfolio_daily_run_to_clickhouse
fail_strategy_portfolio_daily_run
```

3. NATS typed task 新增 `strategy_portfolio_daily_run`。
4. `rearview-server` outbox dispatcher 发布 daily run task。
5. `rearview-portfolio-worker` 新增 handler：

```text
handle_strategy_portfolio_daily_run_task
process_strategy_portfolio_daily_run
```

6. daily run 复用 strategy backtest transient signal materialization，区别：

```text
run_start_date = strategy_portfolio.live_start_date
end_date = trade_date
rule_snapshot = strategy_portfolio.rule_snapshot
execution_config = strategy_portfolio.execution_config
benchmark = strategy_portfolio.benchmark_security_code
portfolio_run_id = strategy_portfolio_daily_run_id
source_kind = strategy_portfolio_daily_run
```

7. 成功后更新：

```text
strategy_portfolio_daily_run.status = succeeded
strategy_portfolio_daily_run.current_result_attempt_id = result_attempt_id
strategy_portfolio.latest_daily_run_id = daily_run_id
strategy_portfolio.current_result_attempt_id = result_attempt_id
```

完成标准：

1. 同一 `(strategy_portfolio_id, trade_date)` 重复触发不会创建重复有效 run。
2. worker 写入 ClickHouse `fleur_portfolio` / `fleur_calculation`。
3. dashboard 可读取 latest daily run live summary。

### Phase 6: Dagster asset/job/schedule

**目标**：Dagster 每日定时触发 active strategy portfolio daily runs。

实现：

1. 新增 `pipeline/scheduler/src/scheduler/defs/rearview/` 模块。
2. 新增 Rearview HTTP resource，读取 base URL 和超时配置。
3. 新增日分区 asset：

```text
rearview/strategy_portfolio_daily_runs
```

4. asset 调用：

```text
POST /rearview/strategy-portfolios/daily-runs
```

5. 新增 job：

```text
strategy_portfolio__daily_run_job
```

6. 新增 schedule：

```text
portfolio__daily_run_schedule
```

建议 cron：`0 20 * * *`，晚于当前 `stock__daily_build_schedule = 30 18 * * *`。

7. materialization metadata：

```text
trade_date
active_portfolio_count
created_run_count
skipped_run_count
daily_run_ids
rearview_request_id
```

8. 在 `pipeline/scheduler/src/scheduler/defs/definitions.py` 显式聚合 rearview defs。

完成标准：

1. `cd pipeline/scheduler && uv run dg list defs --json` 能看到 asset/job/schedule。
2. `uv run dg check defs` 通过。
3. asset 不因 active portfolio 数量变化而改变 Dagster definitions。

### Phase 7: End-to-end acceptance and report

**目标**：形成可复验证据链。

验收路径：

1. `make racingline-dev` 启动 Rearview server、portfolio worker 和前端。
2. 在 `/strategies` 完成 Step 1/2/3/4/5，等待 Step 5 succeeded。
3. 点击「建立组合」，填写策略名称，确认发布。
4. 返回 `/dashboard`，看到新组合卡片，状态为 `pending_first_run` 或已有 live summary。
5. 调用 daily run API 或 Dagster asset 触发当日运行。
6. 等 worker succeeded 后刷新 dashboard，看到 live fields 更新。
7. 进入 `/dashboard/strategies/{strategy_portfolio_id}`，验证信号、绩效、净值、持仓/调仓均来自 API。
8. 点击删除，组合归档并从默认 dashboard 消失。

输出：

1. 新增 job report：`docs/jobs/reports/2026-06-24-racingline-strategy-portfolio-publish-dashboard-dagster.md`。
2. 记录命令、API 样例、关键 ID、截图路径和验证结果。

## 测试策略

### Rust / Rearview

必须覆盖：

1. `portfolio_code` 格式和唯一冲突重试。
2. publish API 只允许 succeeded source backtest。
3. source result attempt mismatch 返回 conflict。
4. archived portfolio 不出现在 dashboard 默认列表。
5. dashboard pending-first-run 不把 source backtest metric 写入 live summary。
6. daily run `(strategy_portfolio_id, trade_date)` 幂等。
7. daily run worker 使用 `portfolio_run_id = strategy_portfolio_daily_run_id` 写 ClickHouse。
8. rebalance rows aggregation 对 buy/hold/sell 的字段口径与 Step 5 wrapper 一致。

### Frontend / Racingline

必须覆盖：

1. Publish panel button enablement：只有 Step 5 succeeded 且无 pending config change 时启用。
2. 策略名称为空时确认按钮禁用。
3. publish mutation 成功后返回 dashboard 并 invalidate dashboard query。
4. dashboard 渲染 `pending_first_run`。
5. detail page 不使用 mock builders。
6. score breakdown 缺失时不生成 fake score items。
7. dashboard API error 时不显示 `portfolioCards`。
8. detail API error 时不显示 `holdingsByPortfolioId` 或 generated records。
9. UI 布局保持当前设计：测试或验收截图覆盖首页卡片、详情页信号区、绩效区、净值区和持仓记录区。
10. `rg "portfolioCards|holdingsByPortfolioId|buildStrategySignalPools|buildDetailRebalanceRecords" app/racingline_new/src` 不命中生产展示调用路径。

### Dagster

必须覆盖：

1. definitions check。
2. asset materialization metadata 字段完整。
3. Rearview API error 时 asset fail，不静默成功。

## 验证命令

文档变更：

```bash
make docs-check
git diff --check
```

前端实现：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

Rearview / Rust 实现：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

迁移：

```bash
cd pipeline/migrate
uv run alembic upgrade head
```

Dagster：

```bash
cd pipeline/scheduler
uv run dg check defs
uv run dg list defs --json
```

端到端：

```bash
make racingline-dev
```

## 完成标准

1. `app/racingline_new` 从 Step 5 可发布 strategy portfolio。
2. `strategy_portfolio_id` 和 `portfolio_code` 均持久化并出现在 dashboard/detail API。
3. Dashboard 首页不再依赖 `portfolioCards` mock。
4. Strategy detail 页不再依赖 `holdingsByPortfolioId`、`buildStrategySignalPools()` 和 `buildDetailRebalanceRecords()`。
5. 发布后立即能在 dashboard 看到新组合，即使 daily run 尚未完成。
6. Dagster 可定时创建 active portfolios 的 daily runs。
7. Worker 能完成 daily run 并把 live results 写入 ClickHouse。
8. `GET /rearview/strategy-portfolios/dashboard` 能展示 latest live summary。
9. 所有验证命令通过。
10. 验收报告落到 `docs/jobs/reports/`。
11. 所有业务展示字段均有真实数据来源说明；缺失字段显示 unavailable/empty，不使用 mock 兜底。
12. 前端看板和详情页设计、排版、信息层级保持当前 `racingline_new` 形态。

## 暂缓项

1. Strategy portfolio versioning。
2. 增量持仓/现金状态恢复。
3. 多用户权限和审计。
4. 真实交易账户。
5. 完整候选池 signal_count 和 score breakdown 的长期明细表；第一版可隐藏得分项或只展示 target score。
