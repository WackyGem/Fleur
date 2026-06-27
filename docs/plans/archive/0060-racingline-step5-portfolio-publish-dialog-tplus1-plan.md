# Plan 0060: Racingline Step 5 建立组合弹层与 T+1 建仓语义实施计划

日期：2026-06-27

状态：Completed

领域：racingline, rearview

关联系统：racingline, rearview, data-platform

代码根：

- `app/racingline/`
- `engines/crates/rearview-core/`
- `engines/crates/rearview-server/`
- `engines/crates/rearview-portfolio-worker/`
- `pipeline/migrate/`
- `pipeline/elt/`

关联文档：

- [RFC 0034: Racingline Step 5 建立策略组合弹层分 Tab 信息架构](../../RFC/0034-racingline-step5-portfolio-publish-dialog-tabs.md)
- [RFC 0029: Racingline 回测结果发布为策略组合与 Dagster 日运行资产](../../RFC/archive/0029-racingline-strategy-portfolio-publish-and-daily-run.md)
- [Plan 0052: Racingline 策略组合发布、看板真实数据与 Dagster 日运行实施计划](0052-racingline-strategy-portfolio-publish-dashboard-dagster-plan.md)
- [Racingline 系统地图](../../architecture/racingline.md)
- [Rearview 系统地图](../../architecture/rearview.md)
- [数据平台系统地图](../../architecture/data-platform.md)

## 背景

RFC 0034 已确认 Step 5「建立组合」弹层的信息架构和发布口径需要同时调整：

1. 弹层从单页长面板改成两个 tab：`策略配置` 和 `回测业绩`。
2. 公共顶部只保留 `策略名称` 和 `建仓日期`，`T+1 交易日`作为建仓日期后的次级文本。
3. Tab 1 用竖直清单展示 Step 1 指标过滤、Step 2 权重得分和 Step 4 建仓摘要；风控规则和风控摘要合并，只展示启用项。
4. Tab 2 用简报展示回测业绩，不展示回测快照；周期、业绩基准先展示，随后展示 `业绩表现`，并把 `业绩日期`放在该段第一条。
5. 发布后的策略组合必须区分两段数据：发布依据的 `backtest_segment` 和建仓后跟踪的 `live_segment`。
6. 用户点击「确定」后，真实建仓日是 T+1 交易日；在首个 live daily run 成功前，看板只能展示 T 日产生、T+1 调入的待买入信号，不能展示回测业绩冒充 live 业绩。

本计划把 RFC 0034 拆成可实施、可验收的后端接口、数据模型、worker、前端弹层和看板语义变更。

## 目标

1. 新增发布预检能力，由 Rearview 权威返回 `source_signal_date`、`planned_live_start_date` 和 `pending_buy_signals`。
2. 创建组合时重新执行发布预检，并校验前端确认的 T/T+1 日期，避免 stale 弹层发布。
3. 在 `strategy_portfolio` 控制面保存首批信号日和发布时待调入信号快照。
4. Dashboard 和详情 live result endpoint 停止在 pending 首次运行时回退到 source backtest 业绩。
5. 首个 strategy portfolio daily run 使用 `initial_signal_date` 作为信号/模拟窗口起点，确保 T 日信号能在 T+1 执行。
6. live daily run 写入 ClickHouse 前，以 `live_start_date` 将策略净值、基准净值和业绩指标重新归 1、重新计算并入库。
7. live daily run 从空持仓和配置初始资金开始，不继承 source backtest 的持仓、现金、成本、订单、成交、事件或绩效状态。
8. ClickHouse 结果事实层强制拆成 backtest result 和 strategy portfolio live result 两套物理表。
9. PostgreSQL 控制平面先完成现状梳理、字段归属和调用链审计，再决定是否进一步拆表或新增 publish/live state 表。
10. 前端「建立组合」弹层实现 RFC 0034 的 tab 信息架构和简报展示。
11. 前端看板对 pending-first-run 组合展示建仓日和待调入信号，不展示 latest nav、收益、风险、净值曲线等 live 业绩字段。

## 非目标

1. 不引入真实券商交易、实盘下单、成交回报或资金账户。
2. 不实现增量持仓恢复；daily run 第一版继续沿用全窗口模拟。
3. 不修改 Step 1、Step 2、Step 3、Step 4 的业务配置语义。
4. 不在浏览器内计算交易日历、权威建仓日、权威信号、净值、绩效或持仓。
5. 不把尚未建仓的 pending 状态写成 ClickHouse 假 nav、假 performance、假 target result attempt。
6. 不重做 Dashboard 或详情页视觉结构；只修正 pending/live/backtest 数据来源和必要空态。
7. 不新增第三个 tab；第一版只实现 `策略配置` 和 `回测业绩`。
8. 不在 PostgreSQL 控制平面审计完成前直接拍脑袋拆表；审计结论必须先写回本计划或后续 migration 设计。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| 前端发布动作 | [strategy-page.tsx](../../../app/racingline/src/routes/strategy-page.tsx) 的 `publishPortfolio()` 只提交 `source_strategy_backtest_run_id`、`source_result_attempt_id`、`name` 和 `client_request_id`，成功后跳转 `/dashboard`。 |
| 前端 API client | [rearview.ts](../../../app/racingline/src/api/rearview.ts) 只有 `POST /rearview/strategy-portfolios`，没有发布预检接口。 |
| 前端类型 | [rearview.ts](../../../app/racingline/src/types/rearview.ts) 的 `StrategyPortfolioCreateRequest` 不包含 expected T/T+1 字段，`StrategyPortfolioDashboardCard.today_signals` 只有 code/name/score。 |
| 后端创建组合 | [api/mod.rs](../../../engines/crates/rearview-core/src/api/mod.rs) 的 `create_strategy_portfolio()` 校验 source backtest succeeded 和 attempt 匹配后，直接从 `source_run.end_date` 解析 `live_start_date` 并写入组合。 |
| 建仓日期解析 | `resolve_strategy_portfolio_live_start_date()` 会查询 source end date 后 45 天交易日，但找不到时 `unwrap_or(source_end_date)`，会把建仓日退回 T。 |
| Dashboard pending | `get_strategy_portfolio_dashboard()` 在没有 `latest_daily_run_id` 时设置 `live_status = pending_first_run`、`curve_source = source_backtest`。 |
| live 结果解析 | `resolve_strategy_portfolio_result()` 在没有 daily run result 时回退到 source backtest result attempt。 |
| Dashboard 读模型 | `strategy_portfolio_dashboard_read_model()` 基于 resolved result 读取 nav、performance、latest targets，并生成 latest nav、returns/risk/curve/today signals。 |
| daily run 创建 | [postgres/mod.rs](../../../engines/crates/rearview-core/src/postgres/mod.rs) 的 `create_strategy_portfolio_daily_runs_for_trade_date()` 把 `run_start_date` 写成 `portfolio.live_start_date`。 |
| daily run 信号 | [rearview-portfolio-worker main.rs](../../../engines/crates/rearview-portfolio-worker/src/main.rs) 的 `materialize_strategy_portfolio_daily_run_signals()` 要求 run range 至少两个交易日，并把 signal date 映射到下一交易日。 |
| backtest 尾日信号 | strategy backtest worker 会丢弃 `execution_date > run.end_date` 的信号，所以 source backtest latest target 不能代表 T 日生成、T+1 调入的发布信号。 |
| ClickHouse 结果事实 | [portfolio_schema.rs](../../../engines/crates/rearview-core/src/clickhouse/portfolio_schema.rs) 当前只有统一的 `fleur_portfolio.portfolio_*` facts；strategy backtest 和 strategy portfolio daily run 都写入这套表，只靠 `portfolio_run_id/result_attempt_id` 和 snapshot JSON 区分来源。 |
| ClickHouse 绩效事实 | [calculation_schema.rs](../../../engines/crates/rearview-core/src/clickhouse/calculation_schema.rs) 当前只有统一的 `fleur_calculation.calc_portfolio_*` facts；backtest 和 live performance metric 共享物理表。 |
| worker 写入路径 | [rearview-portfolio-worker main.rs](../../../engines/crates/rearview-portfolio-worker/src/main.rs) 中 backtest 和 daily run 最终都适配成 `PortfolioRunRecord`，再调用统一 ClickHouse writer。 |
| PostgreSQL 控制面 | 当前已有 `strategy_backtest_run`、`strategy_backtest_task_outbox`、`strategy_portfolio`、`strategy_portfolio_daily_run`、`strategy_portfolio_daily_task_outbox` 等表；表面已按 backtest/live 分开，但仍需梳理字段归属、状态机、current attempt、summary/progress、outbox 和 resolver 调用链后再决定是否进一步拆分。 |
| 当前迁移 | 最新 Rearview migration 是 `pipeline/migrate/versions/rearview/0008_create_strategy_portfolio_control_plane.py`。后续控制面字段应使用新 revision。 |

## 目标数据口径

```text
source_signal_date = T
planned_live_start_date / live_start_date = T 的下一个交易日
initial_signal_date = source_signal_date

strategy_portfolio
  ├─ backtest_segment：发布依据，不冒充 live 业绩
  └─ live_segment：建仓后跟踪，Dashboard 和详情默认读取这一段
```

`initial_signal_date` 只是 T 日信号种子，不是正式组合净值基准日。`live_start_date` 是正式建仓日，也是 live 策略净值、基准净值和绩效指标的归一化基准日。

回测段只提供发布依据。正式组合不是 source backtest 的状态延续，不能继承历史回测持仓、现金、成本价、订单、成交、事件、止损状态、净值或绩效指标。live tracking 是一个新的账本：使用同一套策略规则和执行配置，在 T 日重新生成首批待调入信号，然后从 `live_start_date` 开始跟踪。

以 2026-06-27 周六发布为例：

```text
source_signal_date = 2026-06-26
planned_live_start_date = 2026-06-29

发布后立即返回 Dashboard:
  live_status = pending_first_run
  performance_source = none
  signal_source = publish_preview
  latest_nav = null
  returns/risk/efficiency/relative = []
  curve = []
  pending_buy_signals = T 日生成、T+1 调入的信号

2026-06-29 daily run:
  run_start_date = 2026-06-26
  trade_date = 2026-06-29
  initial portfolio state = cash only, no inherited holdings
  signal 2026-06-26 -> execution 2026-06-29

ClickHouse live result attempt:
  nav rows start at 2026-06-29
  strategy_nav(2026-06-29) = 1.0
  benchmark_nav(2026-06-29) = 1.0
  performance window = 2026-06-29..trade_date
```

入库口径：

| 数据 | 入库要求 |
|---|---|
| `fleur_portfolio.live_nav_daily` | live result attempt 只写入 `live_start_date..trade_date` 的归一化净值序列；第一条 live strategy nav 为 `1.0`。回测段对应序列只能写入 `fleur_backtest.backtest_nav_daily`。 |
| 基准净值 | 与策略净值同基准，以 `live_start_date` 归 1 后写入或随 wrapper 使用同一 live window 计算；不得沿用 source backtest benchmark nav。 |
| `fleur_portfolio.live_performance_metric` | 基于归一化后的 live 策略净值和 live 基准净值重新计算并写入 live performance 表；回测段绩效只能写入 `fleur_backtest.backtest_performance_metric`。 |
| `initial_signal_date` 种子 nav | 只允许存在于 worker 内存计算或诊断 summary；不得作为 live nav/performance 展示事实入库。 |
| holdings / cash / cost basis | 从空持仓和配置初始资金开始计算；不得从 source backtest result attempt 继承。 |
| orders / trades / events | 只写入 live daily run 自己产生的记录；不得复制 source backtest 历史记录。 |

ClickHouse 写入遵守 append-only result attempt：Per `insert-mutation-avoid-update`，归一化口径变化通过新的 daily run result attempt 重新计算写入，不使用 mutation 修正旧事实；Per `insert-batch-size`，nav、target、order、trade、position、event 和 performance facts 继续按 daily run 批量写入；Per `schema-pk-filter-on-orderby`，读取仍通过 run id 和 `result_attempt_id` 前缀过滤已完成 result attempt。

## 表级隔离目标

### ClickHouse 强制拆表

本计划要求 ClickHouse 结果事实层物理拆成两套 family。新 backtest result 写入 `fleur_backtest`，新 live result 写入 `fleur_portfolio.live_*`；二者不再写入旧统一 `fleur_portfolio.portfolio_*` / `fleur_calculation.calc_portfolio_*` 裸表。

目标结构：

```text
fleur_backtest
  backtest_run_snapshot
  backtest_nav_daily
  backtest_target
  backtest_order
  backtest_trade
  backtest_position_day
  backtest_event
  backtest_performance_metric
  backtest_performance_metric_status
  backtest_closed_trade
  backtest_trade_metric

fleur_portfolio
  live_run_snapshot
  live_nav_daily
  live_target
  live_order
  live_trade
  live_position_day
  live_event
  live_performance_metric
  live_performance_metric_status
  live_closed_trade
  live_trade_metric
```

命名可在实施时微调，但必须满足：

1. 回测结果和正式组合 live 结果是不同物理表。
2. backtest worker 只能写 backtest result family。
3. strategy portfolio daily run worker 只能写 live result family。
4. Backtest API 只能读 backtest result family。
5. Portfolio live API 只能读 live result family。
6. dbt sources / wrappers 分开声明，不允许同一 source 再靠 `source_kind` 过滤。
7. 旧 `fleur_portfolio.portfolio_*` 和 `fleur_calculation.calc_portfolio_*` 只作为历史兼容或迁移来源，不作为新写入目标。

表级身份字段和查询合同：

| family | 表前缀 | 主运行 ID 字段 | attempt 字段 | 主要查询前缀 |
|---|---|---|---|---|
| `fleur_backtest` | `backtest_*` | `strategy_backtest_run_id` | `result_attempt_id` | `strategy_backtest_run_id + result_attempt_id` |
| `fleur_portfolio` | `live_*` | `strategy_portfolio_daily_run_id` | `result_attempt_id` | `strategy_portfolio_daily_run_id + result_attempt_id` |

表名合同：

| 语义 | backtest 表 | live 表 |
|---|---|---|
| run snapshot | `fleur_backtest.backtest_run_snapshot` | `fleur_portfolio.live_run_snapshot` |
| nav | `fleur_backtest.backtest_nav_daily` | `fleur_portfolio.live_nav_daily` |
| target | `fleur_backtest.backtest_target` | `fleur_portfolio.live_target` |
| order | `fleur_backtest.backtest_order` | `fleur_portfolio.live_order` |
| trade | `fleur_backtest.backtest_trade` | `fleur_portfolio.live_trade` |
| position day | `fleur_backtest.backtest_position_day` | `fleur_portfolio.live_position_day` |
| event | `fleur_backtest.backtest_event` | `fleur_portfolio.live_event` |
| performance metric | `fleur_backtest.backtest_performance_metric` | `fleur_portfolio.live_performance_metric` |
| metric status | `fleur_backtest.backtest_performance_metric_status` | `fleur_portfolio.live_performance_metric_status` |
| closed trade | `fleur_backtest.backtest_closed_trade` | `fleur_portfolio.live_closed_trade` |
| trade metric | `fleur_backtest.backtest_trade_metric` | `fleur_portfolio.live_trade_metric` |

`portfolio_run_id` 只能出现在旧统一 facts 的兼容路径或迁移脚本中。新表、新 Rust row struct、新 reader/writer API 不再使用泛化 `portfolio_run_id` 表达 backtest/live 两种来源。

ClickHouse 设计约束：

| 规则 | 本计划应用 |
|---|---|
| Per `schema-pk-plan-before-creation` | 拆表时先列出 backtest result 和 live result 各自 top query patterns，再定 ORDER BY；不能只复制旧表结构当作最终设计。 |
| Per `schema-pk-filter-on-orderby` | API 查询必须使用 run id + result attempt 前缀过滤；backtest 使用 `strategy_backtest_run_id`，live 使用 `strategy_portfolio_daily_run_id`。 |
| Per `schema-pk-cardinality-order` | 如果新增低基数字段如 `window_key/status`，不要把高基数 UUID 放在无过滤收益的位置之外；最终排序以实际查询为准。 |
| Per `schema-partition-low-cardinality` | nav/position/trade/event 等时序事实优先沿用月分区；禁止按 run id、portfolio id 或 strategy id 高基数分区。 |
| Per `insert-mutation-avoid-update` | 两套 result family 都保持 append-only result attempt；重算生成新 attempt，不 mutation 覆盖。 |
| Per `insert-batch-size` | 两套 writer 都按 run 批量写入，不引入单行补写。 |

### PostgreSQL 控制平面审计

PostgreSQL 涉及状态机、outbox、幂等、current attempt 和前端 read model，不能在未梳理现状前直接拆表。本计划要求先完成控制平面审计，再决定拆分。

审计范围：

| 表/链路 | 需要确认 |
|---|---|
| `strategy_backtest_run` | 回测配置、range、benchmark、status/progress、summary、signal_summary、现有 `current_result_attempt_id`、ui_display_snapshot 的字段归属。 |
| `strategy_backtest_task_outbox` | task payload、dispatch status、幂等和 retry 是否只服务 backtest。 |
| `strategy_portfolio` | 发布依据、live_start_date、initial_signal_date、pending signal snapshot、latest_daily_run_id、现有 `current_result_attempt_id` 是否应迁移为 `current_live_result_attempt_id`、ui_display_snapshot 是否混合过多职责。 |
| `strategy_portfolio_daily_run` | live run 状态机、run_start_date、trade_date、summary、signal_summary、现有 `current_result_attempt_id` 是否足够独立。 |
| `strategy_portfolio_daily_task_outbox` | 是否只服务 live daily run，是否仍含 source backtest 语义。 |
| API resolver | 是否还有 `resolve_strategy_portfolio_result()` 这类跨 source fallback；是否需要拆成 backtest resolver 和 live resolver。 |

审计后的拆分候选：

1. `strategy_portfolio_publish_context`：保存发布时 source backtest、T/T+1、pending signal snapshot 和不可变回测依据。
2. `strategy_portfolio_live_state`：保存 live tracking 当前状态、latest daily run、current live result attempt 和可展示 live summary。
3. `strategy_portfolio_live_attempt` 或 result registry：把 live attempt 元数据从 daily run 状态中拆出，避免和 source backtest attempt 混用。
4. outbox payload 类型收窄：backtest outbox 和 live daily run outbox 保持不同 payload contract。

Phase 0 必须产出 PostgreSQL 控制平面审计结论，建议记录到 `docs/jobs/reports/YYYY-MM-DD-racingline-portfolio-control-plane-audit.md`；Phase 3 才能据此落控制平面 migration。

## API 和数据合同草案

### 发布预检

新增只读接口：

```http
GET /rearview/strategy-backtests/{strategy_backtest_run_id}/portfolio-publish-preview?source_result_attempt_id=...
```

目标响应：

```json
{
  "can_publish": true,
  "blockers": [],
  "source_strategy_backtest_run_id": "01J...",
  "source_result_attempt_id": "01J...",
  "source_signal_date": "2026-06-26",
  "planned_live_start_date": "2026-06-29",
  "source_period_key": "1y",
  "source_start_date": "2025-06-26",
  "source_end_date": "2026-06-26",
  "benchmark_security_code": "000903.SH",
  "pending_buy_signals": [
    {
      "security_code": "600000.SH",
      "security_name": "浦发银行",
      "source_rank": 1,
      "source_score": 91.4,
      "signal_date": "2026-06-26",
      "execution_date": "2026-06-29"
    }
  ]
}
```

预检 resolver 职责：

1. 校验 source backtest `status = succeeded`。
2. 校验 `source_result_attempt_id` 等于 source run 当前 attempt。
3. 取 `source_signal_date = source_run.end_date`。
4. 通过交易日历解析 `planned_live_start_date = next_trade_date(source_signal_date)`；找不到时返回 blocker，不允许回退到 T。
5. 针对 `source_signal_date` 单日编译 TopN 信号，并把每条 `execution_date` 设置为 `planned_live_start_date`。
6. 补齐证券展示名和 score/rank；不从 source backtest latest target 读取发布信号。

### 创建组合

更新现有接口：

```http
POST /rearview/strategy-portfolios
```

请求新增必填字段：

```json
{
  "source_strategy_backtest_run_id": "01J...",
  "source_result_attempt_id": "01J...",
  "name": "策略组合",
  "expected_source_signal_date": "2026-06-26",
  "expected_live_start_date": "2026-06-29",
  "client_request_id": "strategy-portfolio-..."
}
```

创建职责：

1. 重新执行发布预检；不信任前端日期。
2. 后端解析出的 T/T+1 与 expected 字段不一致时返回 `409 Conflict`。
3. 创建组合时保存 `live_start_date`、`initial_signal_date` 和 `pending_buy_signal_snapshot`。
4. 创建组合只复制 source backtest 的规则、执行配置、benchmark、period 和发布展示快照；不得复制 source backtest 的持仓、现金、成本、订单、成交、事件、净值或绩效状态。
5. `latest_daily_run_id` 和 `current_live_result_attempt_id` 仍为空，`live_status = pending_first_run`。
6. `client_request_id` 的 request hash 必须纳入 expected T/T+1 字段，避免同一幂等 key 复用不同发布上下文。

### Dashboard read model

Dashboard response 需要显式区分 `backtest_segment` 和 `live_segment`。目标结构可以在保持现有 card 字段兼容的同时新增 segment 字段；前端迁移后再清理旧 `curve_source = source_backtest` 语义。

```json
{
  "backtest_segment": {
    "source_strategy_backtest_run_id": "01J...",
    "source_result_attempt_id": "01J...",
    "period_key": "1y",
    "start_date": "2025-06-26",
    "end_date": "2026-06-26",
    "benchmark_security_code": "000903.SH"
  },
  "live_segment": {
    "live_status": "pending_first_run",
    "live_start_date": "2026-06-29",
    "initial_signal_date": "2026-06-26",
    "latest_daily_run_id": null,
    "current_live_result_attempt_id": null,
    "performance_source": "none",
    "signal_source": "publish_preview"
  },
  "latest_nav": null,
  "recent_change": null,
  "returns": [],
  "risk": [],
  "efficiency": [],
  "relative": [],
  "curve": [],
  "pending_buy_signals": [
    {
      "code": "600000.SH",
      "name": "浦发银行",
      "score": 91.4,
      "rank": 1,
      "signal_date": "2026-06-26",
      "execution_date": "2026-06-29"
    }
  ]
}
```

详情 live result endpoint 规则：

| Endpoint | pending-first-run 行为 |
|---|---|
| `/dashboard` | 返回 pending card、空 live 业绩、pending buy signals。 |
| `/nav` | 不回退 source backtest；返回 `409 portfolio_pending_first_run`。 |
| `/performance` | 不回退 source backtest；返回 `409 portfolio_pending_first_run`。 |
| `/positions` | 不回退 source backtest；返回 `409 portfolio_pending_first_run`。 |
| `/rebalance-records` | 不回退 source backtest；返回 `409 portfolio_pending_first_run`。 |
| `/signals` / `/signal-timeline` | pending 时可读取 `pending_buy_signal_snapshot`，并明确 `signal_source = publish_preview`。 |

本计划固定为：Dashboard 返回 200 pending card，详情类 live result endpoint 返回 `409 portfolio_pending_first_run`，signals endpoint 返回 pending snapshot。所有路径都必须停止使用 source backtest 作为 live 结果 fallback。

## 禁止模式

1. 禁止前端用自然日 `+1` 推导建仓日。
2. 禁止找不到 T+1 交易日时退回 T。
3. 禁止把 source backtest nav、performance、curve、target 填进 live dashboard 区域。
4. 禁止用 source backtest latest target 代表发布日 T 的待调入信号。
5. 禁止在 ClickHouse 写入尚未建仓的占位 nav/performance/target。
6. 禁止把 `initial_signal_date` 的种子 nav 作为 live 净值、基准净值或业绩指标入库展示。
7. 禁止在查询/API 层临时把 source backtest 或 seed nav 重标为 live 净值；归一化必须在 live result attempt 写入前完成。
8. 禁止继承 source backtest 的持仓、现金、成本、订单、成交、事件、止损状态或绩效状态作为 live 初始状态。
9. 禁止新 backtest/live 写入继续共用 ClickHouse 统一 `portfolio_*` / `calc_portfolio_*` facts。
10. 禁止用 `source_kind` 过滤统一 facts 作为最终隔离方案；`source_kind` 只能用于迁移期审计或历史兼容。
11. 禁止新增跨 backtest/live 的通用 result resolver 自动 fallback。
12. 禁止在 PostgreSQL 控制平面审计完成前随意拆字段或拆表。
13. 禁止在弹层公共顶部展示条件指标、评分项、候选/持仓、回测区间等信息。
14. 禁止 Tab 1 使用明显表格布局展示指标过滤、权重得分和建仓摘要。
15. 禁止展示未启用的风控项。
16. 禁止 UI API 失败时回退到 mock、fixture 或前端生成业务数据。

## 允许保留的例外

1. `backtest_segment` 可以在 Dashboard 或详情中作为“回测依据”展示，但不能进入 live 业绩区域。
2. 迁移前已存在且仍 pending 的组合，如果没有 `pending_buy_signal_snapshot`，可以展示空待调入信号和“发布信号未留存”的空态；不得用 source backtest target 代替。
3. Tab 2 的回测业绩仍读取 source backtest result，因为它的标签明确是“回测业绩”，不是 live 组合业绩。
4. 第一次实施可以保留现有 `strategy-page.tsx` 单文件结构的局部 render helper；若改动继续扩大，再单独拆组件。

## 实施阶段

### Phase 0：合同冻结、控制平面审计与拆表合同

目标：把接口、迁移、ClickHouse 拆表合同和 PostgreSQL 控制平面审计口径固定下来，避免前后端并行时出现字段漂移。

实施项：

1. 固化发布预检响应类型、create request 新字段和 dashboard segment 字段命名。
2. 固化 pending detail endpoint 的 response shape：`/nav`、`/performance`、`/positions` 和 `/rebalance-records` 在 pending-first-run 时返回 `409 portfolio_pending_first_run`；`/dashboard` 返回 200 pending card；`/signals` 和 `/signal-timeline` 返回 pending snapshot。
3. 固化 ClickHouse 两套 result family 的库名、表名、字段、ORDER BY、分区和 writer/reader 路由。
4. 完成 PostgreSQL 控制平面审计：
   - 列出 backtest control plane 表、portfolio publish 表、portfolio live run 表和 outbox 表。
   - 标注每个字段属于发布依据、live tracking、worker 状态、UI snapshot、幂等还是调度。
   - 追踪 create backtest、publish portfolio、create daily run、worker finalize、dashboard 和 detail endpoints 的读写路径。
   - 识别需要拆出的 publish context、live state 或 attempt registry。
   - 形成审计报告 `docs/jobs/reports/YYYY-MM-DD-racingline-portfolio-control-plane-audit.md`，包含字段归属矩阵、读写调用链、拆分决策和迁移风险。
5. 固定一个周末发布样本，用于验证 `2026-06-27 -> 2026-06-29` 的 T/T+1 行为。
6. 审计现有 `strategy_portfolio` 行：
   - 是否存在 `live_start_date = source_end_date` 的历史脏数据。
   - 是否存在 pending 且没有 daily run 的组合。
   - 是否需要 migration 后临时允许 empty pending snapshot。
7. 更新前后端类型契约测试计划，明确旧 create request 缺少 expected 字段时应返回 validation error。

测试策略：

- 文档-only 阶段运行 `make docs-check` 和 `git diff --check`。
- 实施阶段新增后端 contract 单测和前端类型/请求单测。

完成标准：

- 发布预检、创建组合、Dashboard card 和 pending detail endpoint 的字段名称在文档、Rust 类型和 TypeScript 类型中一致。
- 后续 Phase 不再引入同义字段，例如 `portfolio_start_date`、`planned_start_date`、`next_trade_date` 混用。
- ClickHouse backtest/live 两套 result family 的 DDL 和 writer/reader 路由固定。
- PostgreSQL 控制平面审计报告落在 `docs/jobs/reports/`，并产出明确的“保持 / 拆字段 / 新增表 / 迁移”清单。

### Phase 1：Rearview 发布预检与 T+1 交易日解析

目标：建立后端权威的发布预检路径，前端不再推测建仓日。

实施项：

1. 新增 route：
   - `GET /rearview/strategy-backtests/{strategy_backtest_run_id}/portfolio-publish-preview`
2. 抽取发布预检 resolver：
   - 输入 `strategy_backtest_run_id` 和 `source_result_attempt_id`。
   - 输出 source run、T/T+1 日期、pending signals、blockers。
   - 后续 create API 复用同一个 resolver。
3. 修改或替换 `resolve_strategy_portfolio_live_start_date()`：
   - 找不到下一交易日时返回 validation/conflict 错误。
   - 不允许 `unwrap_or(source_end_date)`。
4. 单日生成 pending buy signals：
   - 使用 source run 的 `rule_snapshot` 和 `execution_config.buy_signal_top_n`。
   - 针对 `source_signal_date` 编译 TopN 信号。
   - 填入 `execution_date = planned_live_start_date`。
   - 补齐 `source_rank`、`source_score` 和证券展示名。
5. 预检失败不写 PostgreSQL，不写 ClickHouse，不发 NATS。

测试策略：

- Rust 单测覆盖：
  - source backtest 非 succeeded 时 blocker。
  - attempt mismatch 时 blocker。
  - 2026-06-27 周六样本解析出 `source_signal_date = 2026-06-26`、`planned_live_start_date = 2026-06-29`。
  - 交易日历找不到下一交易日时不回退 T。
  - pending signals 的 `execution_date` 全部等于 planned live start date。
- 运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

完成标准：

- 弹层打开时可以只通过预检接口展示建仓日期和待调入信号。
- 发布预检不依赖 source backtest latest target。
- T+1 交易日口径只来自后端交易日历。

### Phase 2：ClickHouse backtest/live result family 物理拆表

目标：把回测结果和正式组合 live 结果从底层事实表开始物理隔离，避免后续只能靠 `source_kind` 约定过滤。

实施项：

1. 新增或重构 ClickHouse schema DDL：
   - backtest result family：`fleur_backtest.backtest_*`。
   - live result family：`fleur_portfolio.live_*`。
   - nav、target、order、trade、position、event、snapshot、performance metric、metric status、closed trade、trade metric 全部成对拆分。
   - 字段命名必须遵守上文表级身份合同：backtest 表使用 `strategy_backtest_run_id`，live 表使用 `strategy_portfolio_daily_run_id`，两者都保留 `result_attempt_id`。
   - 如果某些通用 helper 内部仍需要泛型参数，外部 public API 和 DDL 也必须暴露为 backtest/live 专用类型，避免业务代码继续传入 `portfolio_run_id`。
2. 分别设计 ORDER BY 和分区：
   - backtest API 主要按 `strategy_backtest_run_id + result_attempt_id` 查询。
   - live API 主要按 `strategy_portfolio_daily_run_id + result_attempt_id` 查询。
   - 时序事实优先月分区；不按 run id、portfolio id 或 strategy id 分区。
3. 拆分 ClickHouse writer：
   - `write_strategy_backtest_results()` 只写 backtest family。
   - `write_strategy_portfolio_live_results()` 只写 live family。
   - 不再把二者都适配为通用 `PortfolioRunRecord` 后写统一 facts。
4. 拆分 ClickHouse reader：
   - strategy backtest wrapper APIs 只读 backtest family。
   - strategy portfolio live APIs 只读 live family。
   - legacy `/portfolio-runs` 如仍存在，必须保留为旧虚拟账户路径，不参与 strategy backtest/live 混用。
5. 迁移策略：
   - 本阶段新写入先切到新表。
   - 是否回填旧 unified facts 到新表，单独记录迁移范围；如果不回填，旧 run 只作为历史兼容读取。
   - 旧统一 facts 不作为新 strategy backtest 或 strategy portfolio live 的写入目标。
6. dbt source / wrapper：
   - 如果当前 dbt 声明了旧统一 `fleur_portfolio.portfolio_*` / `fleur_calculation.calc_portfolio_*` source，本阶段新增 `fleur_backtest.backtest_*` 和 `fleur_portfolio.live_*` source 声明计划。
   - 后续 wrapper 命名必须区分 `strategy_backtest` 和 `strategy_portfolio_live`。

测试策略：

- Rust DDL 单测覆盖两套 family 的表数量、ORDER BY、分区和禁止高基数分区。
- Rust writer 单测覆盖：
  - backtest task 只调用 backtest writer。
  - daily run task 只调用 live writer。
  - result attempt append-only，不 mutation 旧表。
- API 单测覆盖：
  - backtest `/nav`、`/performance` 等只读 backtest family。
  - portfolio `/nav`、`/performance` 等只读 live family。
- 运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

完成标准：

- 新 backtest result 不再写入统一 `fleur_portfolio.portfolio_*` / `fleur_calculation.calc_portfolio_*`。
- 新 live result 不再写入统一 `fleur_portfolio.portfolio_*` / `fleur_calculation.calc_portfolio_*`。
- Backtest 和 live 的 writer、reader、query id 和 API resolver 在类型和命名上分开。
- 旧统一 facts 的兼容读取策略明确，不影响新结果隔离。

### Phase 3：PostgreSQL 控制平面拆分决策与发布上下文落库

目标：基于 Phase 0 审计结果决定 PostgreSQL 控制平面的拆分形态，让 strategy portfolio 保存不可漂移的发布上下文，并在创建时防 stale。

实施项：

1. 根据 Phase 0 审计结果选择 migration 形态：
   - 如果 `strategy_portfolio` 只缺少少量不可变发布字段，可新增受控字段。
   - 如果发布依据、pending signal snapshot 和 live state 已经混杂，优先新增 `strategy_portfolio_publish_context` 和/或 `strategy_portfolio_live_state`。
   - 如果 current attempt 语义仍混杂，评估新增 live result attempt registry。
2. 推荐的 publish context 字段：
   - `strategy_portfolio_id`
   - `source_strategy_backtest_run_id`
   - `source_result_attempt_id`
   - `source_signal_date`
   - `planned_live_start_date`
   - `pending_buy_signal_snapshot`
   - `source_period_key`
   - `source_start_date`
   - `source_end_date`
   - `benchmark_security_code`
   - `created_at`
3. 推荐的 live state 字段：
   - `strategy_portfolio_id`
   - `live_start_date`
   - `latest_daily_run_id`
   - `current_live_result_attempt_id`
   - `live_status`
   - `updated_at`
4. 迁移现有数据：
   - `initial_signal_date` 默认回填为 `source_end_date`。
   - `pending_buy_signal_snapshot` 默认 `[]`。
   - 对 `live_start_date <= initial_signal_date` 的历史行先记录审计结果，再决定是否手动修正；不得用 migration 静默造假信号。
5. 更新 Rust record、insert、select 和 response 类型；命名必须体现 publish context 和 live state 的职责差异。
6. 更新 `StrategyPortfolioCreateRequest`：
   - 新增必填 `expected_source_signal_date`。
   - 新增必填 `expected_live_start_date`。
7. `create_strategy_portfolio()` 复用 Phase 1 预检 resolver：
   - expected 字段不一致返回 `409 Conflict`。
   - request hash 纳入 expected 字段。
   - 保存 source signal date、planned live start date 和 pending signal snapshot。
8. 创建成功响应返回 `backtest_segment` / `live_segment` 或至少返回新增 publish context / live state 字段。

测试策略：

- Alembic upgrade 在 rearview target 下通过：

```bash
cd pipeline/migrate
uv run alembic -c alembic.ini -x target=rearview upgrade head
```

- Rust 单测覆盖：
  - 缺 expected 字段 validation error。
  - expected 与 resolver 不一致返回 conflict。
  - 相同 `client_request_id` + 相同 expected 字段幂等返回已有组合。
  - 相同 `client_request_id` + 不同 expected 字段返回 conflict。
  - 新记录保存 source signal date、planned live start date 和 pending signal snapshot。
  - publish context 不被 live daily run 更新覆盖。
  - live state/current live attempt 不写回 source backtest 控制字段。

完成标准：

- PostgreSQL 控制平面完成审计并形成明确 migration：保持字段、拆字段或新增表都有依据。
- 发布上下文足以表达发布时 T/T+1 和首批待调入信号。
- 创建组合不再只依赖 source backtest end date。
- 旧 pending 组合不被 source backtest target 自动补假信号。

### Phase 4：Dashboard 和 portfolio live result 语义修正

目标：把 pending-first-run 的展示数据从 source backtest fallback 改成明确的 live 空态 + 发布信号快照。

实施项：

1. 拆分 result resolver：
   - `resolve_strategy_backtest_result()` 只解析 source backtest 的 `strategy_backtest_run_id + result_attempt_id`，只读 `fleur_backtest.backtest_*`。
   - `resolve_strategy_portfolio_live_result()` 只解析 portfolio live 的 `strategy_portfolio_daily_run_id + current_live_result_attempt_id`，只读 `fleur_portfolio.live_*`。
   - 删除或收窄现有 `resolve_strategy_portfolio_result()` 的跨 family fallback；如保留函数名，只能作为 live resolver wrapper，不能再回退 source backtest。
   - pending 首次运行返回 no live result / pending unavailable，不返回 source backtest result。
   - succeeded 时只返回 live result family 的 run id / attempt，不返回 backtest result family。
2. 修改 `get_strategy_portfolio_dashboard()`：
   - pending 时不调用基于 source backtest 的 `strategy_portfolio_dashboard_read_model()`。
   - 返回 `performance_source = none`、空 metrics、空 curve、空 latest nav。
   - 返回 `pending_buy_signals`，每条带 `rank/score/signal_date/execution_date`。
   - 同时返回 `backtest_segment` 和 `live_segment`。
3. 修改 portfolio result endpoints：
   - `/nav`、`/performance`、`/positions`、`/rebalance-records` pending 时不回退 source backtest。
   - succeeded 时只查询 `fleur_portfolio.live_*` facts 和 live performance facts。
   - `/signals` 和 `/signal-timeline` pending 时可读取 pending snapshot，并标明 `signal_source = publish_preview`。
4. TypeScript 类型同步：
   - 增加 `performance_source`、`signal_source`、`backtest_segment`、`live_segment`。
   - `today_signals` 重命名或新增 `pending_buy_signals`；避免 pending 文案仍叫“今日信号”。
5. 保留 source backtest summary 只作为 `backtest_segment` 的回测依据字段，不参与 live metrics。

测试策略：

- Rust API 单测覆盖：
  - pending dashboard 不查询 source backtest nav/performance。
  - pending dashboard latest nav / metrics / curve 为空。
  - pending dashboard signals 来自 `pending_buy_signal_snapshot`。
  - succeeded daily run dashboard 只读取 latest daily run result attempt 和 live result family。
  - pending detail endpoints 不返回 source backtest 数据。
- 前端单测覆盖：
  - pending card 不渲染 latest nav 数值和净值曲线。
  - pending signals 展示 `signal_date -> execution_date`。
  - backtest segment 如果展示，标签必须是“回测依据”。

完成标准：

- 新建组合返回 Dashboard 后，用户只能看到待调入信号和建仓日期。
- 任何 live 业绩区域都不会显示 source backtest 指标。
- source backtest 仍可作为明确标注的发布依据被查看。

### Phase 5：daily run 首次信号窗口与 live 净值归一化

目标：让 T+1 首次 daily run 能执行 T 日生成的首批买入信号，并把正式 live 净值、基准净值和业绩指标按建仓日重新归 1 后入库。

实施项：

1. 修改 daily run 创建：
   - eligible 逻辑仍使用 `portfolio.live_start_date <= trade_date`。
   - 插入 `strategy_portfolio_daily_run.run_start_date = portfolio.initial_signal_date`。
2. 确保首个 run 的 trade date 列表包含 T 和 T+1：
   - `run_start_date = initial_signal_date`
   - `trade_date = live_start_date`
3. worker 继续用 `run.run_start_date..run.trade_date` 生成信号，并把 T 信号映射到 T+1。
4. live result attempt 写入前增加归一化步骤：
   - `initial_signal_date` 的模拟种子 nav 不写入 `fleur_portfolio.live_nav_daily`。
   - `live_start_date` 是第一条 live nav 日期，策略净值写为 `1.0`。
   - 基准净值同样以 `live_start_date` 归 1。
   - daily return、holding return、excess return、drawdown、risk/efficiency/relative metrics 基于 `live_start_date..trade_date` 重新计算。
5. live simulation input 明确从空持仓和配置初始资金开始：
   - 不读取 source backtest latest positions。
   - 不读取 source backtest cash、cost basis、orders、trades、events。
   - 只使用 source backtest 固化的 rule snapshot、execution config、benchmark 和 required metrics/marts。
6. succeeded 后 portfolio live state 的 `current_live_result_attempt_id` 和 `latest_daily_run_id` 仍只指向 live daily run result，不回填 source backtest。
7. worker 写入目标：
   - `live_nav_daily`
   - `live_target`
   - `live_order`
   - `live_trade`
   - `live_position_day`
   - `live_event`
   - `live_performance_metric`
   - `live_performance_metric_status`
   - `live_closed_trade`
   - `live_trade_metric`
   - `live_run_snapshot`

测试策略：

- Rust 单测覆盖：
  - `source_signal_date = 2026-06-26`、`live_start_date = 2026-06-29` 时，2026-06-29 daily run 的 `run_start_date = 2026-06-26`。
  - worker 不再因首个 run 只有一个交易日而失败。
  - 首个 daily run signal summary 中有 `signal_date = 2026-06-26`、`execution_date = 2026-06-29`。
  - live result attempt 的第一条 nav date 是 `live_start_date`，不是 `initial_signal_date`。
  - live result attempt 的第一条 strategy nav 和 benchmark nav 都是 `1.0`。
  - performance metrics 的窗口起点是 `live_start_date`。
  - 即使 source backtest 尾日存在持仓，live 首个 daily run 也从空持仓和初始资金开始。
  - live orders/trades/events 只包含 live daily run 自己产生的记录。
  - live daily run 不向 `fleur_portfolio.portfolio_*` 或 `fleur_calculation.calc_portfolio_*` 写新结果。
  - daily run succeeded 后 Dashboard 切换到 `performance_source = live_daily_run`。

完成标准：

- 首个 live daily run 可以在 T+1 正常完成。
- T 日 pending signal 与 T+1 成交语义一致。
- live curve 从 live daily run 产生，不拼接 source backtest curve。
- live strategy nav、benchmark nav 和 performance metrics 都以 `live_start_date` 归 1 后入库。
- live positions、cash、orders、trades 和 events 都来自 live daily run，不继承 source backtest。
- live facts 只写入 strategy portfolio live result family。

### Phase 6：前端建立组合弹层 Tab 改造

目标：实现 RFC 0034 的弹层信息架构，并接入发布预检。

实施项：

1. 弹层打开时拉取 publish preview：
   - query key 包含 `strategy_backtest_run_id` 和 `source_result_attempt_id`。
   - preview stale、blocker 或错误时禁用「确定」。
2. 公共顶部：
   - 保留策略名称输入。
   - 展示 `建仓日期 2026-06-29  T+1 交易日`。
   - 不展示条件指标、评分项、候选/持仓、回测区间。
3. 使用现有 `Tabs` 组件实现：
   - `策略配置`
   - `回测业绩`
4. Tab 1 `策略配置`：
   - `指标过滤`：竖直清单展示 group、关系和表达式。
   - `权重得分`：竖直清单展示序号、加分值和表达式。
   - `建仓摘要`：竖直清单展示资金、TopN、最大持仓、单票上限、交易成本和风控。
   - 风控只展示启用规则，固定止损和时间止损等启用项合并在一条 `风控` 清单中。
5. Tab 2 `回测业绩`：
   - 简报样式，不做指标矩阵，不展示回测快照。
   - 先展示 `周期：近一年（起始日期 - 结束日期）`。
   - 再展示 `业绩基准`。
   - 再展示 `业绩表现`，第一条为 `业绩日期：YYYY-MM-DD`。
   - 策略净值、基准净值、超额收益等指标竖向展示。
6. 点击「确定」：
   - 提交 `expected_source_signal_date` 和 `expected_live_start_date`。
   - 创建失败留在当前 tab 并展示错误。
   - 创建成功后沿用跳转 `/dashboard`。
7. 删除弹层中回测快照、Run ID、result attempt 等诊断信息的主流程展示。

测试策略：

- 前端单测覆盖：
  - preview 未成功时确定按钮禁用。
  - create request 带 expected T/T+1 字段。
  - Tab 1 风控只展示启用项。
  - Tab 2 没有回测快照文案或调试字段。
- 浏览器/Playwright smoke：
  - desktop 和 mobile 下弹层顶部、tab、底部按钮不重叠。
  - 策略净值、基准净值、超额收益竖向展示。
- 运行：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

完成标准：

- 弹层视觉结构与 RFC 0034 线框一致。
- 前端没有建仓日自然日推导逻辑。
- 用户能在发布前看到后端确认的 T+1 建仓日期。

### Phase 7：前端 Dashboard 和详情 pending 状态改造

目标：让发布后返回看板时正确表达“未建仓，仅有待调入信号”。

实施项：

1. Dashboard card：
   - `performance_source = none` 时不展示 live 最新净值、收益指标、风险指标和曲线数值。
   - 信号标题使用“待调入信号”或“买入信号”，不使用“今日信号”。
   - 每条信号展示 `signal_date -> execution_date`。
2. Dashboard detail：
   - pending detail endpoint 返回 `409 portfolio_pending_first_run` 时渲染待建仓空态。
   - 不用 source backtest 填充 nav/performance/positions/rebalance。
3. 已有 source backtest 信息如果保留展示，必须放在“回测依据”区域。
4. API 错误、字段为空和 pending 状态使用明确 UI 状态，不回退 mock。

测试策略：

- 前端单测覆盖：
  - pending card 不调用或不渲染 live curve。
  - `pending_buy_signals` 渲染日期链路。
  - pending detail 不显示 source backtest 持仓、调仓或业绩。
- 浏览器 smoke：
  - 发布成功跳转 Dashboard 后首屏展示 pending 状态。
- T+1 daily run succeeded 后刷新为 live 业绩。

完成标准：

- 发布当天/周末发布后，看板不再显示回测净值和回测收益冒充 live 业绩。
- 首个 live daily run 成功后，看板只显示 live daily run 结果。

### Phase 8：端到端验收与文档收敛

目标：用真实开发环境验证 T/T+1 发布、pending Dashboard 和首个 daily run 闭环。

实施项：

1. 使用代表策略完成 Step 5 succeeded backtest。
2. 打开「建立组合」弹层，确认：
   - Tab 1 清单布局。
   - Tab 2 简报布局。
   - 建仓日期来自 preview。
3. 点击确定发布组合。
4. 验证 Dashboard pending：
   - live 业绩为空。
   - 待调入信号日期为 T -> T+1。
5. 触发 T+1 daily run。
6. 验证 succeeded 后 Dashboard 和详情切到 live result。
7. 形成 job report，记录命令、样本 run id、portfolio id、日期口径、接口响应和截图。
8. 实施完成后将本计划状态更新为 Completed 并归档，必要时同步 Racingline/Rearview 系统地图。

验证命令：

```bash
make docs-check
git diff --check

cd pipeline/migrate
uv run alembic -c alembic.ini -x target=rearview upgrade head

cd ../../engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../app/racingline
npm run lint
npm run typecheck
npm test
npm run build
```

完成标准：

- RFC 0034 中的弹层线框和内容取舍全部落地。
- 2026-06-27 周六发布样本解析出 2026-06-29 建仓日。
- 发布后 pending Dashboard 不显示任何 live 业绩数据。
- T+1 首个 daily run 能执行 T 日 pending buy signals。
- live 组合净值、基准净值和业绩指标均以建仓日重新归 1 后入库。
- live 组合持仓、现金、订单、成交和事件均从建仓后新账本产生，不延续回测账本。
- ClickHouse backtest result 和 strategy portfolio live result 已写入不同物理表。
- PostgreSQL 控制平面已完成现状审计，拆分或保留决策有字段归属和调用链依据。
- source backtest 和 live tracking 在 API、类型和 UI 标签中明确分离。
- 无 mock、无自然日推导、无 source backtest live fallback。

## 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Dashboard 与详情 endpoint 的 pending 语义不一致 | 前端处理分散，容易重复 fallback | Phase 0 固定合同：Dashboard 200 pending card；详情类 live endpoint 统一 `409 portfolio_pending_first_run`。 |
| 历史 pending portfolio 没有 pending signal snapshot | Dashboard 无法展示发布信号 | 迁移后展示空态，不用 source backtest target 造假；必要时单独补数据修复 runbook。 |
| 单日 pending signal 编译和 backtest worker 信号路径漂移 | 发布信号和首个 daily run 信号不一致 | 复用 `compile_backtest_signals` / `query_backtest_signal_rows` 的同一 TopN 口径，并加 T/T+1 单测。 |
| create API 新字段破坏旧客户端 | 旧发布路径 validation error | Racingline 与 Rearview 同版本发布；旧请求不兼容属于本次语义修正的一部分。 |
| Dashboard 既要显示回测依据又不能混入 live | 用户误解数据来源 | UI 标签强制区分 `回测依据` 与 `建仓后跟踪`。 |
| ClickHouse 强制拆表带来旧数据兼容和迁移成本 | 旧 backtest run 或旧 portfolio daily run 可能仍在统一 facts 表中 | 新写入先切换到两套 result family；旧统一 facts 只作为历史兼容或受控回填来源，兼容读取不能进入新写入路径。 |
| PostgreSQL 控制平面直接拆表过早 | 可能破坏 outbox、幂等、current attempt 或 dashboard resolver | Phase 0 先做字段归属和调用链审计，Phase 3 才落 migration。 |
| 首个 daily run 从 T 开始可能出现 T 日初始 nav | live curve 起点解释需要一致 | 明确 `run_start_date = initial_signal_date` 是信号/模拟窗口起点，`live_start_date` 才是正式建仓日和净值基准日；写入 live result attempt 前剔除或隔离 T 日种子 nav。 |
| 建仓日开盘成交到收盘的首日波动是否计入首日收益 | 若直接把 T+1 收盘 nav 归 1，会把建仓日作为基准日而不是收益日 | 第一版按用户确认的“建仓日重新归 1”执行，建仓日为净值基准日；若后续要展示开盘建仓到收盘收益，需要另设 intraday inception 口径。 |

## 最小验证

本文是文档-only 计划。提交前至少运行：

```bash
make docs-check
git diff --check
```
