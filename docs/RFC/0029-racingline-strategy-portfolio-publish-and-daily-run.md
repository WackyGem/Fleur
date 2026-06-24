# RFC 0029: Racingline 回测结果发布为策略组合与 Dagster 日运行资产

状态：Proposed
领域：racingline, rearview, data-platform
关联系统：racingline, rearview, data-platform
代码根：app/racingline_new/, engines/crates/rearview-core/, engines/crates/rearview-server/, engines/crates/rearview-portfolio-worker/, pipeline/scheduler/, pipeline/migrate/
需求入口：docs/intake/racingline.md

## 摘要

Step 5 策略回测已经落地为 Rearview 异步 backtest run。下一步不是把 Step 5 结果继续留在研究页面，而是让用户把一次成功回测发布为可持续观察的策略组合：

```text
Step 5 策略回测 succeeded
  -> 用户点击「建立组合」
  -> 确认 Step 1 / Step 2 / Step 4 配置快照并填写策略名称
  -> Rearview 创建 strategy portfolio 并固化发布配置
  -> Racingline 返回 /dashboard 并展示新组合卡片
  -> Dagster 每日定时触发 active strategy portfolios 的日运行
```

本文档只定义方案和当前资产盘点，不实现代码。

## 背景

[Q&A 0004](../Q&A/0004-racingline-prototype-dashboard-to-strategy-loop.md) 把 `/dashboard -> /strategies -> Step 5 -> 运行策略 -> /dashboard` 定义为核心用户故事，并明确 `运行策略` 当时只是交互闭环占位。

[RFC 0028](0028-racingline-strategy-backtest-step5.md) 已把 Step 5 收敛为 durable research backtest run：`strategy_backtest_run` 保存不可变 `rule_snapshot`、`execution_config`、period、benchmark、hash、progress 和 `current_result_attempt_id`，worker 把结果写入现有 portfolio/calculation data plane。

因此，本 RFC 的边界是把“成功回测结果”发布为“正式策略组合”，并让该组合进入每日盘后运行和看板展示。它不替代 Step 5，也不把历史 backtest run 直接改名为正式组合。

## 当前资产盘点

### Racingline 前端

| 资源 | 当前事实 | 对本设计的支持 | 缺口 |
|---|---|---|---|
| `/strategies` Step 1/2/4/5 | `app/racingline_new/src/routes/strategy-page.tsx` 已接入 Step 5 create/poll/result wrapper，Step 1/2/4 配置可生成 `RuleVersionSpec` 和 `BacktestExecutionConfig` | 能为「建立组合」面板提供配置快照和源 backtest id | 尚无「建立组合」按钮、发布面板、策略名称输入和 publish mutation |
| Step 5 回测类型 | `app/racingline_new/src/types/rearview.ts` 已有 `StrategyBacktestRunRecord`、nav、performance、targets、orders、trades、positions、events 类型 | 可判断 `status = succeeded`、读取 `current_result_attempt_id` 和展示 source summary | 尚无持久化 strategy portfolio 类型 |
| 看板 | `app/racingline_new/src/routes/dashboard-page.tsx` 和 `strategy-detail-page.tsx` 读取 `portfolioCards` mock 数据 | 可保留看板信息层级 | 不能展示真实新建组合；需要 Rearview 列表/详情 API 替换 mock |

### Rearview 后端与数据面

| 资源 | 当前事实 | 对本设计的支持 | 缺口 |
|---|---|---|---|
| Strategy backtest control plane | `pipeline/migrate/versions/rearview/0007_create_strategy_backtest_control_plane.py` 已创建 `strategy_backtest_run`、`strategy_backtest_task_outbox`、`strategy_backtest_metric_config` | 可作为发布组合的 source run，提供不可变规则、执行配置、benchmark、period 和结果 attempt | 该表是 research backtest，不应承载正式组合生命周期 |
| Step 5 API | `engines/crates/rearview-core/src/api/mod.rs` 已提供 `/rearview/strategy-backtests`、`/{id}`、`/nav`、`/performance` 和明细 wrapper | 可校验 source backtest 是否 succeeded，并 pin `current_result_attempt_id` | 尚无 `/rearview/strategy-portfolios` publish/list/detail/daily-run API |
| Portfolio data plane | `engines/crates/rearview-core/src/clickhouse/portfolio_schema.rs` 和 calculation schema 已支持 portfolio run snapshot、nav、持仓、成交、订单、目标、事件和绩效计算结果 | 新组合每日运行可以复用同一 ClickHouse 结果事实和 `result_attempt_id` 结果重算模型 | 需要在 snapshot/source metadata 中区分 `strategy_portfolio_daily_run` 与 `strategy_backtest` |
| Portfolio worker | `engines/crates/rearview-portfolio-worker/src/main.rs` 已处理 `StrategyBacktest` typed task，并把 `strategy_backtest_run_id` 映射为 portfolio data plane 的 `portfolio_run_id` | 组合日运行可复用 transient signal materialization、NAV 递推和绩效计算 | 需要新增从 strategy portfolio 发布配置读取规则、按单个交易日或滚动窗口生成日运行的任务语义 |

### Dagster 当前资产

使用 `cd pipeline/scheduler && uv run dg list defs --json` 盘点当前定义，结果为 100 个 assets、20 个 jobs、9 个 schedules。

与 portfolio 相关的当前 Dagster assets：

| Asset | 可执行 | 当前角色 |
|---|---:|---|
| `fleur_portfolio/portfolio_run_snapshot` | 否 | dbt source/external asset，表示 ClickHouse portfolio data plane 已有结果快照 |
| `fleur_calculation/calc_portfolio_performance_metric` | 否 | dbt source/external asset，表示 worker 已写入的绩效结果 |
| `fleur_calculation/calc_portfolio_performance_metric_status` | 否 | dbt source/external asset，表示绩效计算状态 |
| `fleur_calculation/calc_portfolio_closed_trade` | 否 | dbt source/external asset，表示 closed trade 计算结果 |
| `fleur_calculation/calc_portfolio_trade_metric` | 否 | dbt source/external asset，表示交易级指标结果 |
| `int_portfolio_*`、`mart_portfolio_*` | 是 | dbt intermediate/mart wrapper 与排名模型 |

当前 jobs 和 schedules 中没有包含 `strategy`、`portfolio` 或 `rearview` 的主动运行任务。已有 schedules 是数据源、ClickHouse raw sync 和 `stock__daily_build_schedule` 等数据准备任务。

结论：

| 问题 | 当前能否实现 | 说明 |
|---|---|---|
| Step 5 成功回测作为发布来源 | 部分能 | `strategy_backtest_run` 和结果 attempt 已存在 |
| 用户填写策略名称并保存正式组合 | 不能 | 没有 strategy portfolio control plane/API |
| 返回看板展示新组合 | 不能 | `app/racingline_new` 看板仍是 mock `portfolioCards` |
| Dagster 识别组合结果数据面 | 部分能 | dbt source 和 mart wrapper 已能识别部分 portfolio/calculation 结果表 |
| Dagster 每日运行用户创建的组合 | 不能 | 没有 active portfolio registry asset、daily run asset、job 或 schedule |

## `racingline_new` 看板字段级盘点

本节以当前 `app/racingline_new` 看板实际展示的数据为准盘点资源，而不是只按后端已有 portfolio 表泛化设计。

当前看板入口：

| 页面 | 路径 | 当前数据来源 |
|---|---|---|
| 策略看板 | `app/racingline_new/src/routes/dashboard-page.tsx`、`app/racingline_new/src/components/racingline/dashboard/portfolio-overview-board.tsx` | `portfolioCards` mock |
| 策略详情 | `app/racingline_new/src/routes/strategy-detail-page.tsx` | `portfolioCards`、`holdingsByPortfolioId`、`buildStrategySignalPools()`、`buildDetailRebalanceRecords()` mock/generated data |
| mock 数据定义 | `app/racingline_new/src/components/racingline/dashboard/portfolio-data.ts` | 本地常量 |

### 看板首页卡片字段

当前 `PortfolioCardData` 字段：

```text
id
name
startDate
backtestDays
simulationDays
latestNav
recentChange
returns[]
risk[]
efficiency[]
relative[]
todaySignals[]
curve[]
```

首页实际展示字段和缺口：

| UI 字段 | 当前 mock 字段 | 可复用现有资源 | 缺口 | 补齐建议 |
|---|---|---|---|---|
| 策略 id | `id` | 无正式 strategy portfolio id | `strategy_backtest_run_id` 不能作为正式组合 id | 新增内部 `strategy_portfolio_id` 和用户可读 `portfolio_code`，dashboard 路由使用 `/dashboard/strategies/{strategy_portfolio_id}` |
| 策略名称 | `name` | 发布面板用户填写 | 缺持久化字段 | `strategy_portfolio.name` |
| 策略编号 | 当前无字段 | 无 | 看板截图、用户沟通和搜索需要比 UUID 更友好的编号 | 新增 `portfolio_code`，格式建议 `SP-YYYYMMDD-XXXXX`，唯一但不承载业务语义 |
| 建仓日期 | `startDate` | `portfolio_nav_daily` 首个有效交易日、或 source backtest start date | 未定义正式组合“建仓”语义 | 第一版定义为组合首个 forward daily run trade date；未运行时用 source backtest start date 并标注为回测区间 |
| 运行天数 | `simulationDays` | `portfolio_nav_daily` 有效 nav 行数 | 缺按 strategy portfolio 聚合 latest active run 的查询 | Dashboard summary API 聚合 active portfolio 的 live nav observation count |
| 最新净值 | `latestNav` | `portfolio_nav_daily.nav` | 现有 API 只按 `portfolio_run_id` 查，不按 portfolio 查 latest | 新增 portfolio summary 查询，解析组合 latest effective run/attempt |
| 最近变化 | `recentChange` | `portfolio_nav_daily.daily_return` 或最近两点 nav 比值 | 缺 latest daily return summary | Dashboard summary 返回 `latest_daily_return` |
| 收益指标 | `returns[]`: 持仓收益、年化收益 | `calc_portfolio_performance_metric.holding_period_return`、`annualized_return` | 现有 performance API 按 run id，不按 portfolio active summary | Summary API 返回 canonical metric slots，不让前端按中文 label 拼装 |
| 风险指标 | `risk[]`: 最大回撤、年化波动率、下行波动率 | `calc_portfolio_performance_metric.max_drawdown`、`annualized_volatility`、`downside_deviation` | 同上 | Summary API 返回 risk metric slots |
| 今日信号 | `todaySignals[]`: code、name、score | `portfolio_target.source_score/source_rank/security_code`，Step 5 preview/backtest 曾有 selected metrics | `portfolio_target` 没有 `security_name`，没有 score breakdown；按“今日”缺 portfolio active run trade_date 解析 | 新增 `GET /strategy-portfolios/{id}/signals?trade_date=`，join `mart_stock_basic_snapshot` 补名称；第一版今日信号可来自 `portfolio_target` |
| 今日信号滚动占位 | UI 固定展示 5 行，不足补 `--` | 前端可保留 | 无后端缺口 | 前端 presentation 行为 |
| 净值与基准曲线 | `curve[]`: time、nav、benchmark | `portfolio_nav_daily.nav` + `mart_benchmark_returns_daily`，Step 5 nav wrapper 已做 benchmark normalization | 现有 generic portfolio `/nav` 不返回 benchmark nav；strategy backtest `/nav` 返回 benchmark，但不是正式组合接口 | 新增 portfolio dashboard nav endpoint 返回 normalized `strategy_nav`、`benchmark_nav` |
| 创建新组合卡片 | 静态 link `/strategies` | 当前可保留 | 无数据缺口 | 保留为入口 |

首页最小 API 不应只返回裸 `strategy_portfolio` 列表，而应返回 dashboard card view model。否则前端要串行调用 nav、performance、signals 和 benchmark 多个接口，首屏会变慢且错误态难以统一。

### 策略详情页字段

详情页当前展示模块：

1. 顶部：策略名称、建仓日期、运行天数、删除按钮。
2. 策略信号：历史信号数曲线、选中日期 Top 股票、得分项、总得分。
3. 策略业绩：业绩日期、业绩基准、策略净值、基准净值、收益/风险/性价比/相对市场指标。
4. 净值走势：策略净值和基准净值曲线。
5. 持仓记录：调仓日期列表、当日调入/持有/调出计数、股票、调仓理由、持仓天数、涨跌幅、成本价、现价、收益贡献。
6. 删除策略：当前只有前端确认弹窗，无后端操作。

详情页字段和缺口：

| UI 模块 | 当前 mock/生成字段 | 可复用现有资源 | 缺口 | 补齐建议 |
|---|---|---|---|---|
| 顶部策略元信息 | `portfolioCards` | `strategy_portfolio`、latest daily run summary | 无 strategy portfolio control plane | `GET /strategy-portfolios/{id}` 返回 id/code/name/status/start/live days/source backtest |
| 删除策略 | Dialog only | 无 | 无删除/停用 API；直接硬删会破坏审计 | 第一版提供 `PATCH /strategy-portfolios/{id}` 设置 `status=archived`，看板默认过滤 archived |
| 历史信号数曲线 | `buildStrategySignalPools()` 生成 `signalCount` | `portfolio_target` 可按 signal_date count；Step 5 signal materialization 有 summary | 缺按 portfolio/date 聚合目标或买入信号数量的 API；`portfolio_target` 只保存最终 target，不一定等于完整候选池数量 | 新增 `GET /strategy-portfolios/{id}/signal-timeline`，字段 `trade_date/signal_count/target_count`；若要展示完整候选池数量，需新增 daily signal summary 表或扩展 worker summary |
| 信号股票列表 | generated `stocks[]` | `portfolio_target` 的 `security_code/source_rank/source_score/target_reason`；`mart_stock_basic_snapshot` 可补名称 | `portfolio_target` 没有 score breakdown；现有 portfolio target API 不补证券名称 | 新增 signals endpoint join display name；score breakdown 可第一版缺省或扩展 data plane |
| 信号得分项 | generated `scoreItems[]` | Step 3/Step 5 计算时存在 selected metrics / score breakdown 概念 | `portfolio_target` schema 无 `score_breakdown`、`selected_metrics`；daily run 若不保存则无法复盘 | 若 UI 保留得分项，新增 `portfolio_signal_detail` 或扩展 `portfolio_target` payload 字段保存 `score_breakdown`、`selected_metrics` |
| 业绩日期 | latest curve point | `portfolio_nav_daily.trade_date` | 缺 portfolio-level latest effective run/attempt | Summary/detail API 返回 `latest_trade_date` |
| 业绩基准 | 当前硬编码“沪深300 / 000300.SH” | `strategy_backtest_run.benchmark_security_code`；正式组合发布配置保存 benchmark | 硬编码不符合 Step 5 可选 benchmark | `strategy_portfolio.benchmark_security_code` + display label |
| 策略净值/基准净值 | `curve` latest point | `portfolio_nav_daily.nav` + benchmark returns normalized | generic portfolio nav API 无 benchmark nav | 新增 portfolio nav endpoint 或复用 Step 5 nav normalization 逻辑 |
| 超额收益 | `latest.nav - latest.benchmark` | 可由 normalized nav 计算 | 需要明确是点位差还是收益率差 | 后端 view model 返回 `excess_return`，定义为 `strategy_nav - benchmark_nav` 或改为累计收益差 |
| 日胜率 | mock 固定 `0.584` | Step 5 `daily_win_rate()` 已基于 nav.daily_return 计算 | generic portfolio performance API 未返回 daily win rate | 将 `daily_win_rate` 从 strategy backtest wrapper 下沉为 portfolio view capability |
| 性价比指标 | `efficiency[]`: Sharpe、Sortino、Calmar、Treynor | `calc_portfolio_performance_metric` | 缺 portfolio summary slot 映射 | Summary/detail API 返回 typed metrics |
| 相对市场指标 | `relative[]`: Alpha、Beta、Information Ratio | `calc_portfolio_performance_metric` | benchmark 必须来自组合发布配置，不可默认 000300.SH | performance query 使用 `strategy_portfolio.benchmark_security_code` |
| 当前持仓/持仓记录 | `holdingsByPortfolioId`、generated records | `portfolio_position_day` | 现有 API 按 run id 和 date 查；不按 strategy portfolio 查 latest/selected date | `GET /strategy-portfolios/{id}/positions?trade_date=` |
| 调仓日期列表 | generated records | `portfolio_nav_daily.position_count` + `query_portfolio_rebalance_trade_counts()` | 现有 rebalance records wrapper 只对 strategy backtest 实现，generic portfolio 没有同等 view API | 抽出为通用 portfolio rebalance view，再挂到 strategy portfolio detail |
| 调入/持有/调出计数 | generated trades | `portfolio_trade.side`、`portfolio_position_day`、closed trades | 可复用 Step 5 `build_strategy_backtest_rebalance_rows()` | 泛化为 `GET /strategy-portfolios/{id}/rebalance-records?trade_date=` |
| 调仓理由 | generated reason | `portfolio_trade.reason`、`portfolio_order.reason`、`portfolio_target.target_reason`、closed trade `exit_reason` | hold 行通常没有 reason；buy/sell 可复用 | API 明确 hold reason 为空；UI 显示 `-` |
| 持仓天数 | generated string | `portfolio_position_day.holding_days`、closed trade `holding_days` | 可复用 | 后端返回 number，前端格式化 |
| 涨跌幅 | generated percent | position `unrealized_return`、closed trade `realized_return` | 可复用 | 后端 view model 返回 `change_pct` |
| 成本价/现价 | generated currency | position `average_entry_price/close_price`、closed trade entry/exit gross amount | 可复用 | 后端 view model 返回 numeric values |
| 收益贡献 | generated percent | Step 5 wrapper 用 unrealized/realized pnl 除 total equity | 可复用 | 泛化 contribution 逻辑 |

### 当前资源能支撑的看板字段

| 能力 | 可支撑字段 |
|---|---|
| `strategy_backtest_run` control plane | 发布来源、source period、benchmark、rule/config snapshot、source result attempt |
| `portfolio_nav_daily` | 最新净值、最近日收益、运行天数、净值曲线、持仓数、调仓日期列表基础 |
| `calc_portfolio_performance_metric` | 持仓收益、年化收益、最大回撤、年化波动率、下行波动率、Calmar、Sortino、Sharpe、Information Ratio、Beta、Alpha、Treynor |
| `portfolio_target` | 今日信号 code、rank、score、target reason、目标权重/金额 |
| `portfolio_position_day` | 持仓、持仓天数、成本价、现价、涨跌幅、浮盈亏 |
| `portfolio_trade` / `portfolio_order` | 调入/调出、成交价、费用、调仓理由 |
| `calc_portfolio_closed_trade` | 调出行的 realized return、持仓天数、退出原因 |
| `mart_stock_basic_snapshot` | 证券名称、交易所、交易板块显示信息 |
| Step 5 wrapper 逻辑 | benchmark normalized nav、rebalance rows、daily win rate 的可复用实现样板 |

### 当前缺口清单

| 缺口 ID | 缺口 | 影响 | 建议补齐 |
|---|---|---|---|
| D-G1 | 没有 `strategy_portfolio` control plane | 看板没有真实策略组合 id/code/name/status/发布配置 | 新增 `strategy_portfolio`，不引入 version 表 |
| D-G2 | 看板首页没有聚合 API | 前端无法高效渲染卡片，需要多接口拼装 | 新增 `GET /rearview/strategy-portfolios/dashboard` 或让 list endpoint 返回 card summary |
| D-G3 | 详情页没有按 portfolio id 查询的 view API | 现有 portfolio/strategy-backtest API 都以 run id 为核心 | 新增 `GET /strategy-portfolios/{id}/summary|nav|signals|signal-timeline|rebalance-records|positions` |
| D-G4 | generic portfolio nav 不返回 benchmark normalized series | 首页和详情页曲线需要基准 | 下沉 Step 5 benchmark normalization 到通用 portfolio/strategy portfolio query |
| D-G5 | generic portfolio performance 不返回 daily win rate | 详情页“日胜率”无法真实展示 | 下沉 `daily_win_rate()` 到 portfolio performance view |
| D-G6 | `portfolio_target` 没有证券名称 | 今日信号和信号列表需要中文名 | 查询时 join `mart_stock_basic_snapshot`，不冗余写入结果表 |
| D-G7 | `portfolio_target` 没有 score breakdown / selected metrics | 详情页“得分项”无法复盘 | 如果保留得分项 UI，新增 signal detail payload；否则第一版隐藏得分项列 |
| D-G8 | 没有 signal timeline 聚合 | 历史信号数曲线无法真实展示 | 从 `portfolio_target` 聚合 target_count；完整候选池 signal_count 需 worker 写 daily signal summary |
| D-G9 | 删除策略没有后端状态 | 删除按钮只是 mock | 增加 archive/disable API，不做物理删除 |
| D-G10 | active portfolio latest run/attempt 解析不存在 | 无法从 portfolio id 找到 latest nav/performance/signals | `strategy_portfolio` control plane 保存 latest daily run/current attempt 指针，或提供解析服务 |
| D-G11 | dashboard 字段未区分回测摘要和正式运行摘要 | 用户可能把 source backtest 当成 live 结果 | API 字段分为 `backtest_summary` 和 `live_summary`，UI 标明口径 |
| D-G12 | Dagster 当前只识别 portfolio result source/mart，不触发 active portfolio daily run | 新建组合不会每日更新 | 新增 `rearview/strategy_portfolio_daily_runs` asset/job/schedule |
| D-G13 | 发布后尚未有 daily run 时 dashboard 无 live 数据 | 创建成功返回看板会出现新组合但没有正式运行曲线/指标 | dashboard view model 必须支持 `live_status = pending_first_run`，此时用 source backtest summary/curve 展示回测依据，并明确标注不是 live 结果 |
| D-G14 | 现有 portfolio simulation 不支持从昨日持仓状态增量恢复 | “基于前一日持仓状态”的日运行会要求新增账本恢复能力 | 第一版 daily run 改为从组合 start date 到 trade_date 的全窗口重算，生成新的 daily run attempt；增量运行另起后续优化 |

## 目标

1. Step 5 下方新增「建立组合」按钮，只在当前 backtest run 成功且配置未 stale 时启用。
2. 点击「建立组合」打开紧凑确认面板，展示 Step 1、Step 2、Step 4、Step 5 source backtest 的关键快照，并要求用户填写策略名称。
3. 点击确定后，Rearview 从成功的 `strategy_backtest_run` 创建正式 `strategy_portfolio`，并在组合记录上固化发布配置。
4. 创建成功后，Racingline 返回 `/dashboard`，用真实 API 展示新组合卡片，而不是前端 mock。
5. Dagster 能看到策略组合注册表和每日运行资产，并按日定时触发 active strategy portfolios 的运行。
6. 每日运行结果继续落到 existing portfolio data plane 和 calculation data plane，保留 `result_attempt_id` 幂等重算和审计能力。

## 非目标

1. 不实现实盘交易、券商下单、真实资金账户或交易所撮合。
2. 不引入登录、鉴权、用户隔离或权限审计；如需要另起 ADR/RFC。
3. 不把每个用户创建的策略组合注册成一个新的 Dagster asset definition。
4. 不把 Step 5 research backtest run 原地变成正式组合。
5. 不允许前端计算权威信号、成交、持仓、费用、净值或绩效指标。
6. 不要求第一版支持任意历史批量重跑所有组合；历史回填另走 runbook。

## 核心设计决策

### D1: 正式策略组合是独立 control-plane entity

新增 Rearview control-plane 概念：

```text
strategy_portfolio
  strategy_portfolio_id
  portfolio_code
  name
  status
  rule_snapshot
  rule_hash
  execution_config
  execution_config_hash
  benchmark_security_code
  price_basis
  catalog_hash
  required_metrics
  required_marts
  source_strategy_backtest_run_id
  source_result_attempt_id
  source_period_key
  source_start_date
  source_end_date
  ui_display_snapshot
  latest_daily_run_id
  current_result_attempt_id
  created_at
  updated_at
  archived_at
```

`strategy_backtest_run` 仍代表一次 research backtest。`strategy_portfolio` 代表用户确认要持续观察的策略组合，并直接保存发布时的不可变配置。

第一版不引入 `strategy_portfolio_version`。如果用户需要修改策略条件、权重、建仓参数或 benchmark，应从当前组合复制配置进入 `/strategies` 重新回测，并在成功后创建一个新的 `strategy_portfolio`。这样可以避免在看板、Dagster 日运行和结果归因中引入无实际价值的版本层。

ID 设计：

| 字段 | 用途 | 规则 |
|---|---|---|
| `strategy_portfolio_id` | 内部主键、API 路由、外键、Dagster metadata | UUIDv7 或 ULID，稳定不可变 |
| `portfolio_code` | 用户可读编号、看板搜索、截图沟通 | `SP-YYYYMMDD-XXXXX`，唯一索引，不承载业务语义 |
| `name` | 用户填写的策略名称 | 可改名；不作为主键 |

示例：

```text
strategy_portfolio_id = 01J1X7W4F6T2C8A9MZQ1P6N3BY
portfolio_code = SP-20260624-K7Q9M
name = 红利低波增强
```

### D2: 发布时必须 pin 成功 backtest attempt

「建立组合」请求必须引用：

```json
{
  "source_strategy_backtest_run_id": "uuid",
  "source_result_attempt_id": "uuid",
  "name": "红利低波增强"
}
```

后端必须校验：

| 校验 | 规则 |
|---|---|
| backtest 状态 | `strategy_backtest_run.status = succeeded` |
| 结果 attempt | 请求的 `source_result_attempt_id` 等于 backtest 当前有效 attempt，或显式允许 pin 历史 attempt |
| 快照完整性 | `rule_snapshot`、`execution_config`、`rule_hash`、`execution_config_hash`、benchmark、range 和 `ui_display_snapshot` 均存在 |
| 名称 | 非空、长度受限、同名策略的处理策略明确 |
| 幂等性 | 支持 `client_request_id`，重复提交返回同一 portfolio |

发布成功后，组合持有的是配置快照和 source result pointer，不依赖前端当前表单状态。

### D3: 「建立组合」面板只做确认，不重新配置策略

面板信息密度应高于 Step 页面，目标是在一个 drawer/sheet 中让用户快速确认“这就是我要持续运行的策略”。

建议结构：

| 区域 | 展示内容 |
|---|---|
| 顶部 | 策略名称输入、source backtest 状态、回测周期、benchmark、结果 attempt |
| Step 1 策略选股 | universe、核心条件组、输出指标数量、required metrics 数量 |
| Step 2 权重配置 | scoring rule 数、主要权重项、score clamp |
| Step 4 模拟建仓 | 初始资金、买入 TopN、最大持仓、单票上限、费用、滑点、止盈止损规则数 |
| Step 5 回测摘要 | 年化收益、最大回撤、Sharpe、Alpha/Beta、交易日数、最后净值 |

UI 约束：

1. 使用两列或三列紧凑 key-value 网格，避免大段解释文案。
2. 只展示 canonical 后端快照，不展示本地 draft 或 stale config。
3. 确定按钮只在名称合法、source backtest succeeded、result attempt 存在时启用。
4. 创建 API 成功后再跳转 `/dashboard`；失败留在面板显示后端错误。

### D4: 看板必须从 Rearview 读取正式组合

新增 API 草案：

```http
POST /rearview/strategy-portfolios
GET /rearview/strategy-portfolios
GET /rearview/strategy-portfolios/dashboard
GET /rearview/strategy-portfolios/{strategy_portfolio_id}
PATCH /rearview/strategy-portfolios/{strategy_portfolio_id}
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/daily-runs
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/nav
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/signals
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/signal-timeline
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/rebalance-records
GET /rearview/strategy-portfolios/{strategy_portfolio_id}/positions
```

`GET /rearview/strategy-portfolios/dashboard` 返回当前 `racingline_new` 看板首页卡片所需的 view model：

```json
{
  "items": [
    {
      "strategy_portfolio_id": "uuid",
      "portfolio_code": "SP-20260624-K7Q9M",
      "name": "红利低波增强",
      "status": "active",
      "created_at": "2026-06-24T00:00:00Z",
      "source_strategy_backtest_run_id": "uuid",
      "source_result_attempt_id": "uuid",
      "start_date": "2026-06-01",
      "live_day_count": 16,
      "live_status": "succeeded",
      "latest_trade_date": "2026-06-24",
      "latest_nav": 1.0342,
      "latest_daily_return": 0.0041,
      "benchmark": {
        "security_code": "000300.SH",
        "label": "沪深300"
      },
      "backtest_summary": {
        "period_key": "1y",
        "start_date": "2025-06-24",
        "end_date": "2026-06-24",
        "holding_period_return": 0.1842,
        "annualized_return": 0.2196,
        "max_drawdown": -0.0824,
        "sharpe_ratio": 1.42
      },
      "live_summary": {
        "holding_period_return": 0.0342,
        "annualized_return": 0.192,
        "max_drawdown": -0.031,
        "annualized_volatility": 0.1375,
        "downside_deviation": 0.0941,
        "calmar_ratio": 2.66,
        "sortino_ratio": 1.91,
        "sharpe_ratio": 1.42,
        "information_ratio": 0.88,
        "beta": 0.78,
        "alpha": 0.041,
        "treynor_ratio": 0.23,
        "daily_win_rate": 0.584
      },
      "today_signals": [
        {
          "security_code": "600036.SH",
          "security_name": "招商银行",
          "score": 91.4,
          "rank": 1
        }
      ],
      "curve": [
        {
          "trade_date": "2026-06-24",
          "strategy_nav": 1.0342,
          "benchmark_nav": 1.0121,
          "excess_return": 0.0221
        }
      ]
    }
  ]
}
```

第一版看板可以同时展示 backtest summary 和 live forward-run summary，但字段名必须区分，避免把历史回测收益当成真实运行收益。

当组合刚从 Step 5 发布、尚未完成第一次 daily run 时，`live_status` 返回 `pending_first_run`，`live_summary`、`latest_trade_date`、`latest_nav` 和 `latest_daily_return` 可以为 `null`。此时 dashboard 仍应展示新组合卡片，卡片数据使用 `backtest_summary` 与 source backtest curve，并在 UI 上标注“待首次日运行”。不得把 source backtest 指标写入 `live_summary`。

详情页 API 应避免让前端继续用 mock 生成数据：

| Endpoint | 支撑 UI |
|---|---|
| `GET /rearview/strategy-portfolios/{id}` | 顶部策略元信息、portfolio code、source backtest、benchmark |
| `GET /rearview/strategy-portfolios/{id}/nav` | 净值走势、策略净值、基准净值、超额收益 |
| `GET /rearview/strategy-portfolios/{id}/signals?trade_date=` | 选中日期信号股票、rank、score、可选 score items |
| `GET /rearview/strategy-portfolios/{id}/signal-timeline` | 历史信号数曲线 |
| `GET /rearview/strategy-portfolios/{id}/rebalance-records?trade_date=` | 调仓日期列表、调入/持有/调出、成本价、现价、收益贡献、调仓理由 |
| `GET /rearview/strategy-portfolios/{id}/positions?trade_date=` | 当前或历史持仓表 |
| `PATCH /rearview/strategy-portfolios/{id}` | 删除按钮第一版实现为 `status=archived` |

### D5: Dagster 使用稳定日分区资产，不为每个组合动态建 asset

用户创建组合是高频业务状态变化，不能要求每次创建组合都修改 Dagster Python definitions。第一版采用一个稳定资产处理所有 active portfolios：

```text
Asset: rearview/strategy_portfolio_registry
  代表 Rearview PostgreSQL 中的 active strategy portfolios 和发布配置
  第一版可作为 external asset 或 metadata-only observable asset

Asset: rearview/strategy_portfolio_daily_runs
  Daily partitioned executable asset
  每个 partition_key = trade_date
  读取 active strategy portfolios
  为每个 active strategy portfolio 创建或触发该 trade_date 的 daily run
  materialization metadata 记录 total/created/skipped/succeeded/failed 和 portfolio ids
```

理由：

1. Dagster asset definition 应稳定，用户新增组合不应触发代码部署。
2. Dagster materialization metadata 足够表达某日处理了哪些 portfolio。
3. 单个资产能统一表达每日盘后工作流、失败告警、重跑和数据依赖。
4. 后续如果确实需要逐组合可见性，再评估 dynamic partitions 或 state-backed component，而不是在第一版引入。

### D6: 每日运行是 forward run，不是重新跑 Step 5 全历史 backtest

策略组合发布后的日运行语义：

```text
给定 trade_date 和 active strategy portfolio
  -> 使用该 strategy_portfolio 的 rule_snapshot 和 execution_config
  -> 从 strategy_portfolio.live_start_date 到 trade_date 重新生成候选池、score、TopN 信号
  -> 用现有 PortfolioSimulationInput 从 initial_cash 全窗口重算组合账本
  -> 写入 strategy_portfolio_run control plane 和 ClickHouse data plane
```

第一版采用全窗口重算，而不是从前一日持仓状态增量恢复。原因是当前 `PortfolioSimulationInput` 只有 `start_date`、`end_date`、`initial_cash`、signals 和 prices，没有“从上一日持仓/现金恢复”的输入结构。全窗口重算与现有 Step 5 backtest worker 能力一致，能更快形成可开发闭环；增量恢复可在 active portfolio 数量和窗口长度成为性能瓶颈后另起优化。

第一版可以采用“按组合、按日创建 run”的控制面：

```text
strategy_portfolio_daily_run
  strategy_portfolio_daily_run_id
  strategy_portfolio_id
  run_start_date
  trade_date
  status
  dispatch_status
  current_result_attempt_id
  error_type
  error_message
  created_at
  completed_at
```

幂等约束：

```text
unique(strategy_portfolio_id, trade_date)
```

如果同一天重复触发，后端应返回已有 daily run 或创建新的 result attempt，不应产生重复有效结果。

## Dagster 设计

### 资产图

建议新增：

| 名称 | 类型 | 分区 | 依赖 | 输出 |
|---|---|---|---|---|
| `rearview/strategy_portfolio_registry` | external 或 observable asset | 无 | Rearview PostgreSQL control plane | active portfolio count、config hashes |
| `rearview/strategy_portfolio_daily_runs` | executable asset | Daily | `rearview/strategy_portfolio_registry`、worker-ready marts、portfolio source assets | 每日 active portfolios 运行结果 metadata |

`strategy_portfolio_daily_runs` 应依赖的数据准备资产包括：

| 依赖 | 原因 |
|---|---|
| 股票行情/指标 mart | 生成当日选股、评分和止损信号 |
| `mart_benchmark_returns_daily` | 计算 benchmark 对比指标 |
| `mart_risk_free_rate_daily` | 计算风险调整指标 |
| portfolio/calculation source assets | 保持下游 dbt ranking 与结果消费链路可见 |

### Job 与 schedule

新增 job：

```text
strategy_portfolio__daily_run_job
```

新增 schedule：

```text
strategy_portfolio__daily_run_schedule
```

调度策略：

1. 每日固定时间运行，时间必须晚于行情、指标、risk-free、benchmark 和 `stock__daily_build_schedule` 的常规完成时间。
2. Asset 内部根据 Rearview 返回的 latest available trade date 判断是否为交易日。
3. 如果上游数据未准备好，asset 应失败并携带明确 metadata，而不是静默跳过。
4. 如果当日无 active portfolio，asset materialize 成功，metadata 中 `active_portfolio_count = 0`。

Dagster materialization metadata 至少包含：

| Metadata | 含义 |
|---|---|
| `trade_date` | 本次 partition 对应交易日 |
| `active_portfolio_count` | 查询到的 active portfolios 数量 |
| `created_run_count` | 新创建 daily run 数量 |
| `skipped_run_count` | 因幂等已存在而跳过的数量 |
| `succeeded_run_count` | 本次确认成功的数量 |
| `failed_run_count` | 本次失败数量 |
| `portfolio_ids` | 可截断的 portfolio id 列表 |
| `rearview_request_id` | 调用 Rearview 的请求 id 或 trace id |

### Dagster 与 Rearview 的职责边界

| 职责 | 承载方 |
|---|---|
| 组合定义、发布配置、状态机、幂等、错误 | Rearview PostgreSQL control plane |
| 选股、评分、成交、持仓、NAV、绩效计算 | Rearview worker |
| 日运行定时触发、上游数据依赖可见、materialization metadata、告警 | Dagster |
| portfolio/calculation 结果事实 | ClickHouse `fleur_portfolio` / `fleur_calculation` |
| 下游 ranking 和 mart wrapper | dbt assets |

Dagster 不直接写 strategy portfolio 表，也不在 Python asset 中重写策略计算逻辑。

## API 与数据流

### 发布组合

```text
Racingline Step 5
  -> POST /rearview/strategy-portfolios
  -> Rearview 读取并校验 strategy_backtest_run
  -> Rearview 创建 strategy_portfolio
  -> Racingline navigate('/dashboard')
  -> Dashboard GET /rearview/strategy-portfolios
```

### 每日运行

```text
Dagster schedule
  -> materialize rearview/strategy_portfolio_daily_runs partition=YYYY-MM-DD
  -> Rearview list active portfolios
  -> Rearview create/enqueue daily runs
  -> rearview-portfolio-worker consumes tasks
  -> worker writes ClickHouse portfolio/calculation data plane
  -> Rearview updates daily run status
  -> Dagster materialization metadata records counts and failures
  -> dbt portfolio marts consume result source assets in subsequent build
```

## UI 验收标准

1. Step 5 `succeeded` 且没有 pending config change 时展示「建立组合」按钮。
2. 点击按钮打开紧凑面板，展示 Step 1/2/4 配置摘要和 Step 5 source backtest 摘要。
3. 用户必须填写策略名称；名称非法时确定按钮禁用。
4. 点击确定只调用 Rearview publish API，不在前端拼装权威组合结果。
5. 创建成功后返回 `/dashboard`，新组合来自 Rearview API 响应。
6. 创建失败时停留在面板并显示后端错误。

## 后端验收标准

1. `POST /rearview/strategy-portfolios` 只能从 succeeded strategy backtest 创建组合。
2. 创建的 `strategy_portfolio` 固化 `rule_snapshot`、`execution_config`、hash、benchmark、source range 和 source result attempt。
3. 创建的 `strategy_portfolio` 同时生成唯一 `portfolio_code`，格式为 `SP-YYYYMMDD-XXXXX`。
4. 重复 `client_request_id` 保持幂等。
5. `GET /rearview/strategy-portfolios/dashboard` 能支撑 dashboard card 的真实数据。
6. Daily run 对 `(strategy_portfolio_id, trade_date)` 幂等。
7. 失败状态能在 control plane 和 dashboard 中查询。

## Dagster 验收标准

1. `cd pipeline/scheduler && uv run dg list defs --json` 能看到 `rearview/strategy_portfolio_daily_runs` asset、`strategy_portfolio__daily_run_job` job 和 `strategy_portfolio__daily_run_schedule` schedule。
2. Daily asset materialization metadata 包含 active portfolio 数量、created/skipped/succeeded/failed 计数和 trade date。
3. 调度时间晚于数据准备链路；当上游缺失时失败可见。
4. 不因为用户创建新组合而修改 Dagster definition。
5. 下游 dbt portfolio marts 能继续消费 ClickHouse portfolio/calculation 结果。

## 风险与待决问题

| 风险 | 处理 |
|---|---|
| 前端本地表单和后端 source snapshot 不一致 | 发布 API 只读取 `strategy_backtest_run` 的 canonical snapshot |
| 用户误解回测收益等同实盘收益 | dashboard 字段区分 backtest summary 和 live forward-run summary |
| 每日运行与数据准备竞态 | Dagster schedule 晚于数据准备，并在 asset 内检查 latest available trade date |
| active portfolio 数量增长导致单次 asset 超时 | 后续可批量分页、拆 batch task 或引入 dynamic partitions |
| 同日重复触发产生重复结果 | Rearview control plane 使用唯一约束和 `result_attempt_id` |
| Dagster asset 粒度过粗导致单组合可观测性不足 | 第一版通过 metadata 和 Rearview dashboard 展示；后续再评估 dynamic partitions/state-backed component |

## 实施顺序建议

1. Rearview migration 和 API：`strategy_portfolio`、publish/list/detail。
2. Racingline Step 5 UI：建立组合按钮、紧凑确认面板、创建成功返回 dashboard。
3. Dashboard API 化：替换 `portfolioCards` mock，展示真实组合卡片和详情。
4. Rearview daily run control plane 与 worker task：active portfolio 每日运行、幂等和状态回写。
5. Dagster asset/job/schedule：新增 `rearview/strategy_portfolio_daily_runs`，记录 materialization metadata。
6. 端到端验收：从 Step 5 成功回测发布组合，返回看板展示，并由 Dagster 日运行触发。

## 最小验证命令

文档阶段：

```bash
make docs-check
git diff --check
```

后续实现阶段：

```bash
cd app/racingline_new
npm run lint
npm run typecheck
npm test
npm run build
```

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

```bash
cd pipeline/scheduler
uv run dg check defs
uv run dg list defs --json
```
