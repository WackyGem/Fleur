# RFC 0022: 组合数据面迁移 ClickHouse 与绩效指标分层

状态：Archived（2026-06-25；归档前状态：Proposed（2026-06-17））
领域：rearview
关联系统：rearview, data-platform, racingline
代码根：engines/crates/rearview-core/, engines/crates/rearview-portfolio-worker/, pipeline/elt/, pipeline/migrate/
输入材料：docs/Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md, docs/Q&A/0002-portfolio-metrics.md

## 摘要

本 RFC 把 Q&A 0001 和 Q&A 0002 的结论落为可执行的长期架构方案。核心有两层迁移：

1. 组合结果事实从 PostgreSQL 迁移到 ClickHouse portfolio data plane，PostgreSQL 只保留 control plane（运行状态、任务分发、参数快照、当前有效结果指针）。
2. 绩效指标计算分层：净值递推和 12 个核心绩效指标权威计算保留在 Rust worker（ADR 0012），计算产物写入 ClickHouse `fleur_calculation`；基础输入（benchmark 收益、risk-free rate）由 dbt intermediate / mart 层准备好供 worker 读取，dbt 不复算指标公式。

该方案替代 RFC 0021 第一版"PostgreSQL 保存组合结果，ClickHouse 只作为行情和指标输入"的临时边界。RFC 0021 第一版实现仍作为过渡阶段保留，但在本 RFC 落地后标记为 superseded。

关联文档：

- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](../../Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [Q&A 0002: Portfolio Metrics 基础数据缺口](../../Q&A/0002-portfolio-metrics.md)
- [ADR 0012: 组合净值递推与绩效指标权威计算留在 Rust](../../ADR/0012-portfolio-nav-recursion-stays-in-rust.md)
- [ADR 0009: ClickHouse 按 dbt 建模层分库](../../ADR/0009-clickhouse-layered-databases.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](0021-racingline-virtual-account-portfolio-rebalancing.md)
- [System: Rearview](../../architecture/rearview.md)
- [System: Data Platform](../../architecture/data-platform.md)

## 背景

RFC 0021 第一版为了降低实现复杂度，把组合运行状态和组合结果事实都落在 PostgreSQL：

- `portfolio_run`、`portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav`、`portfolio_event`。

该方案便于 UI 查询、审计和幂等重算，但它把两类不同职责混在同一个 OLTP 数据库中。当组合结果用于策略研究和回测分析时，主要访问模式是 OLAP：按 run、日期、策略、基准和指标窗口批量扫描、聚合、排序和对比。PostgreSQL 不擅长这类工作负载。

与此同时，组合绩效指标（Sharpe、Sortino、Alpha、Beta、信息比率、特雷诺、最大回撤、年化波动率等）需要 benchmark 日频收益率序列和 risk-free rate 日频序列作为输入。Q&A 0002 的依赖矩阵显示，当前 int 层已经补齐了 benchmark 收益（`int_benchmark_returns_daily`）和国债收益率上游（`int_government_bond_yields_daily`），但还缺 risk-free 日频折算与交易日对齐、worker-ready benchmark mart、ClickHouse 组合结果事实表、`portfolio_metric_config` 和 closed trade ledger。

Q&A 0001 已经回答了"结果事实是否应迁出 PostgreSQL"——应迁移。Q&A 0002 已经列出了"绩效指标还缺哪些基础数据"。本 RFC 负责把两者合并为一份带表设计、迁移路径和验收标准的可执行方案，并以 ADR 0012 收敛后的职责边界为准：worker 权威计算绩效指标，dbt 不复算公式。

## 目标

1. 在 ClickHouse 新增 portfolio data plane database，承载组合结果事实表，替代 PostgreSQL 结果事实表。
2. Rust worker 从"写 PostgreSQL 结果事实"改为"写 ClickHouse 结果事实 + 回写 PostgreSQL 控制状态"。
3. 引入 `result_attempt_id` 实现幂等重算，PostgreSQL `portfolio_run` 指向当前有效 attempt。
4. 新增 dbt intermediate / mart 层 risk-free rate 和 benchmark 日频模型，作为 worker 输入对齐的稳定入口。
5. 明确绩效指标分层：净值递推和绩效指标权威计算留 Rust（ADR 0012），worker 计算产物进入 `fleur_calculation`，dbt 只做 source / thin wrapper / mart ranking。
6. 第一阶段最小闭环可权威计算 12 个核心绩效指标（区间/年化收益率、年化波动率、最大回撤、卡尔玛、下行波动率、索提诺、夏普、信息比率、Alpha、Beta、特雷诺）。
7. Racingline 通过 Rearview API 访问结果，API contract 尽量不变，前端不感知底层存储迁移。
8. 明确标注 RFC 0021 PostgreSQL 结果事实边界何时 superseded。

## 非目标

1. 不把净值递推（账本 / NAV / `daily_return` / `drawdown`）搬到 ClickHouse SQL 或 dbt 模型——见 ADR 0012。
2. 第一版不实现 closed trade ledger、胜率、盈亏比等交易级指标，列为第二阶段。
3. 第一版不做持仓归因（benchmark 成分权重、行业分类），列为后续阶段。
4. 不构造全收益（total return）benchmark；当前 benchmark 与组合 NAV 同为价格收益口径。
5. 不让 Racingline 直接查询 ClickHouse 或 PostgreSQL。
6. 不在浏览器请求路径中同步执行长时间回测。

## 术语

- **Control plane**：运行状态、任务分发、参数快照、错误和当前有效结果指针，落在 PostgreSQL。
- **Data plane**：组合结果事实（净值、持仓、成交、订单、目标、事件）落在 ClickHouse `fleur_portfolio`；worker 绩效指标和交易级计算产物落在 ClickHouse `fleur_calculation`。
- **result_attempt_id**：每次重算生成的不可变结果版本；同一 `portfolio_run_id` 可有多个 attempt，PostgreSQL 指向当前有效 attempt。
- **NAV 递推**：目标 → 订单 → 成交 → 每日持仓 → 现金 → NAV → `daily_return` → `drawdown` 的逐日状态机，留在 Rust。
- **无状态聚合指标**：Sharpe / Sortino / Alpha / Beta / 信息比率 / 特雷诺 / 波动率等，基于已有 NAV 和对齐后的 benchmark / risk-free 序列做聚合，由 worker 权威计算，dbt 不复算公式。

## 当前事实

### 已具备

- Rust `rearview-core/src/portfolio/mod.rs`（约 1040 行）实现完整 NAV 递推，带确定性单测。
- worker 从 ClickHouse 读行情 / 信号 / 交易日历，从 PostgreSQL 读运行配置，结果写回 PostgreSQL。
- PostgreSQL `rearview` database 已有 `portfolio_run`、`portfolio_task_outbox`、`portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav`、`portfolio_event`。
- dbt int 层已有 `int_benchmark_basic_snapshot`、`int_benchmark_returns_daily`、`int_government_bond_yields_daily`、`int_index_*`、`int_trade_calendar`。
- ADR 0012 已确定净值递推和绩效指标权威计算留 Rust，dbt 不复算指标公式。

### 欠缺

- ClickHouse 组合结果事实表（`portfolio_nav_daily` 等）。
- intermediate 层 risk-free rate 日频折算和交易日对齐模型。
- mart 层 risk-free / benchmark 日频稳定入口。
- `portfolio_metric_config` 配置面（年化天数、benchmark 选择、risk-free 期限、对齐规则、MAR、`zero_division_policy`、`min_observations`）。
- `fleur_calculation.calc_portfolio_performance_metric` 计算产物表。
- closed trade ledger（第二阶段）。
- benchmark 成分权重和行业分类（后续阶段）。

## 设计决策

### D1: 结果库命名

采用 `fleur_portfolio` 作为 ClickHouse portfolio data plane database。理由：

- 与 ADR 0009 分层命名 `fleur_*` 前缀一致。
- `fleur_portfolio` 语义清晰，覆盖虚拟账户组合和未来 backtest；不与 `fleur_marts`（dbt 消费宽表）混淆。
- `rearview_analytics` 偏服务名而非数据边界，不采用。

### D2: result_attempt_id 替代 ReplacingMergeTree 覆盖

所有 ClickHouse 结果事实表都带 `portfolio_run_id` 和 `result_attempt_id`，采用 append-only + `result_attempt_id` 作为版本维度，而非 `ReplacingMergeTree` 覆盖同一 `portfolio_run_id`。理由：

- Q&A 0001 §"ClickHouse 规则依据"`insert-mutation-avoid-update`：避免频繁 `ALTER UPDATE`。
- append-only 保留历史 attempt，支持跨版本对比和审计。
- PostgreSQL `portfolio_run.current_result_attempt_id` 决定当前有效 attempt，查询时带该过滤。
- 旧 attempt 不删除，靠分区 TTL 或显式清理策略管理，避免 ReplacingMergeTree 的 final 合并不确定性。

### D3: 净值递推留 Rust（引用 ADR 0012）

NAV、`daily_return`、`drawdown` 的逐日递推保留在 Rust worker，不在 ClickHouse SQL 或 dbt 模型重写。ClickHouse 只承载递推结果事实。dbt 层只能消费已有 `portfolio_nav_daily` 和 `fleur_calculation.calc_*` 产物，做字段稳定、质量检查和排名，不派生绩效公式。详见 ADR 0012。

### D4: 绩效指标分层

| 职责 | 承载方 | 说明 |
|---|---|---|
| 账本、NAV、`daily_return`、`drawdown` 递推 | Rust worker | 权威值，写入 ClickHouse |
| 绩效指标（Sharpe / Sortino / Alpha / Beta / 信息比率 / 特雷诺 / 波动率） | Rust worker + `fleur_calculation` | worker 权威计算，dbt 不复算公式 |
| risk-free 折算、benchmark 日频入口 | dbt intermediate + marts | worker 只读现成结果 |
| 跨 run 排名、批量回测对比 | dbt thin wrapper + marts | 消费 `fleur_calculation` 产物 |

worker 写入 `fleur_calculation.calc_portfolio_performance_metric` 作为权威绩效指标产物。dbt 通过 source + thin wrapper 消费该产物，并做 contract 校验和跨 run 批量排名；dbt 不实现第二套绩效指标公式。

### D5: dbt 层只做输入准备、thin wrapper 和消费建模

`daily_return`、`drawdown` 和 12 个核心绩效指标不得在 dbt 模型或 ClickHouse SQL 中重写。dbt 层新增模型只做：risk-free 期限选择、日频折算、forward-fill、benchmark 日频稳定入口、`fleur_calculation` source thin wrapper、质量检查和 ranking。

## ClickHouse Portfolio Data Plane

### D6: 库与表设计

`fleur_portfolio` database，核心事实表：

| 表 | 粒度 | Engine | ORDER BY | 分区 | 用途 |
|---|---|---|---|---|---|
| `portfolio_run_snapshot` | 每 `portfolio_run_id` + `result_attempt_id` 一行 | MergeTree | `(portfolio_run_id, result_attempt_id)` | 无（维表） | 分析维表，run 元数据和不可变快照摘要 |
| `portfolio_nav_daily` | 每 `portfolio_run_id` + `result_attempt_id` + `trade_date` 一行 | MergeTree | `(portfolio_run_id, result_attempt_id, trade_date)` | `toYYYYMM(trade_date)` | NAV、日收益、回撤、现金、持仓市值 |
| `portfolio_position_day` | 每 run + attempt + `trade_date` + `security_code` | MergeTree | `(portfolio_run_id, result_attempt_id, trade_date, security_code)` | `toYYYYMM(trade_date)` | 每日持仓、成本、市值、浮盈亏 |
| `portfolio_trade` | 每笔虚拟成交 + run + attempt | MergeTree | `(portfolio_run_id, result_attempt_id, trade_date, security_code)` | `toYYYYMM(trade_date)` | 成交、费用、滑点、原因 |
| `portfolio_order` | 每个虚拟订单 + run + attempt | MergeTree | `(portfolio_run_id, result_attempt_id, signal_date, security_code)` | `toYYYYMM(signal_date)` | 目标到成交审计链路 |
| `portfolio_target` | 每个目标持仓 + run + attempt | MergeTree | `(portfolio_run_id, result_attempt_id, signal_date, security_code)` | `toYYYYMM(signal_date)` | 信号、rank、score、目标权重/金额 |
| `portfolio_event` | 每个 warning/event + run + attempt | MergeTree | `(portfolio_run_id, result_attempt_id, trade_date)` | `toYYYYMM(trade_date)` | 价格缺失、现金不足、止损触发 |

设计约束（Q&A 0001 §"ClickHouse 规则依据"）：

- 时间序列表按月分区 `toYYYYMM(trade_date)`，不按 `portfolio_run_id` 等高基数字段分区。
- `ORDER BY` 服务主要查询过滤列，净值曲线优先 `(portfolio_run_id, result_attempt_id, trade_date)`。
- worker 批量写入，目标 10K-100K 行，避免小批量写入导致 part 压力。
- append-only + `result_attempt_id`，不做 `ALTER UPDATE` 覆盖。

### D7: portfolio_nav_daily 字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `portfolio_run_id` | `String` | 运行 ID |
| `result_attempt_id` | `String` | 结果版本 |
| `trade_date` | `Date` | 交易日 |
| `nav` | `Float64` | 单位净值 |
| `daily_return` | `Nullable(Float64)` | 日收益，首日为 NULL |
| `drawdown` | `Float64` | 回撤 |
| `total_equity` | `Float64` | 总资产 |
| `cash_balance` | `Float64` | 现金 |
| `position_market_value` | `Float64` | 持仓市值 |
| `gross_exposure` | `Float64` | 总暴露 |
| `position_count` | `UInt32` | 持仓数 |
| `turnover` | `Float64` | 换手率 |
| `fee_amount` | `Float64` | 费用 |
| `warning_count` | `UInt32` | 警告数 |

## Worker 角色变更

### D8: worker 输出目标改为 ClickHouse

worker 长期角色从"组合净值 worker"升级为"组合计算引擎"：

1. 从 PostgreSQL 读取 `portfolio_run` 不可变账户和执行参数快照。
2. 从 ClickHouse 读取 source run 买入信号、后复权行情、交易日历、benchmark return（来自 mart）和 risk-free rate（来自 mart）。
3. 在 Rust 内存中构造目标、订单、成交、每日持仓、净值曲线和绩效指标（ADR 0012）。
4. 批量写入 ClickHouse：组合结果事实进入 `fleur_portfolio.*`，绩效指标和交易级计算产物进入 `fleur_calculation.*`，每次写入带新 `result_attempt_id`。
5. 回写 PostgreSQL run 终态和 `current_result_attempt_id`。
6. 只有 PostgreSQL 终态写入成功后 ack NATS message。

### D9: PostgreSQL 保留的控制面

PostgreSQL `rearview` database 保留：

- `portfolio_run`：运行 ID、source run、状态、起止日期、账户快照、执行快照、`current_result_attempt_id`。
- `portfolio_task_outbox`：HTTP 创建运行和 NATS 发布之间的可恢复 outbox。
- 任务状态机：`created` / `queued` / `calculating` / `succeeded` / `failed_*` / `cancelled`。
- 错误类型、错误消息、完成时间和轻量 summary。

PostgreSQL 不再长期保存每日净值、每日持仓、订单、成交、目标和事件等组合结果事实。旧表迁移完成后标记 deprecated。

## mart 层基础数据补齐

### D10: int_risk_free_rate_daily 与 mart_risk_free_rate_daily

`int_risk_free_rate_daily` 负责期限选择、百分比点转小数、交易日对齐、forward-fill 和日频折算；`mart_risk_free_rate_daily` 只从 intermediate 模型选择 worker-ready 稳定字段，不重新实现计算逻辑。

| 字段 | 类型 | 说明 |
|---|---|---|
| `trade_date` | `Date` | 交易日 |
| `source_date` | `Date` | 实际使用的 ChinaBond 曲线日期 |
| `source_tenor` | `LowCardinality(String)` | 来源期限，第一版支持 `1y` |
| `annual_rate` | `Float64` | 年化无风险利率，小数比例（如 0.0225） |
| `daily_rate` | `Float64` | 日频无风险收益，折算口径写入配置 |

职责：

- 从 `int_government_bond_yields_daily` 选取期限（默认 `one_year_yield_pct`）。
- 百分比点转小数比例（`/ 100`）。
- 年化折算为日频（配置决定，如 `(1 + annual) ^ (1/252) - 1` 或 `annual / 252`）。
- 非交易日 forward-fill 到组合交易日，对齐 `int_trade_calendar`。
- `annualization_days`、`day_count_basis`、`daily_method` 和 `fill_strategy` 是模型口径 / 配置，不作为日频表行字段重复存储。

### D11: mart_benchmark_returns_daily

| 字段 | 类型 | 说明 |
|---|---|---|
| `security_code` | `String` | benchmark 指数 canonical 代码，沿用与持仓/行情同一套标识 |
| `trade_date` | `Date` | 交易日 |
| `daily_return` | `Nullable(Float64)` | 价格指数日收益 |

职责：

- 从 `int_benchmark_returns_daily` 透传 `daily_return`，作为 worker 绩效计算（Beta、Alpha、信息比率、特雷诺）的稳定输入。
- 不派生窗口级累计净值或年化收益；这些是 per-run / 窗口级标量，由 worker 读取 `daily_return` 后按 run 窗口现算，并落到 `fleur_calculation.calc_portfolio_performance_metric`。
- benchmark 收益口径（`price_index`）由 int 层和设计文档承载，不在 mart 日频表冗余标记。

### D12: portfolio_metric_config

`portfolio_metric_config` 是组合绩效指标的计算参数集，属于 PostgreSQL control plane，不是 ClickHouse 结果事实。表名沿用现有 `portfolio_*` 前缀，与 `portfolio_run` / `portfolio_nav` 同级，由 `rearview-server` 在创建 `portfolio_run` 时随不可变执行快照一起写入已解析的最终值。

`portfolio_metric_config` 只活在 PostgreSQL，理由：

- 本质是控制面数据：体量小（每行一个 attempt × benchmark × window），访问模式是 worker 启动时点读一次，属于 OLTP 强项，与 `portfolio_run` 的 account/exec snapshot 同类。
- Q&A 0001 的边界是控制面留 PostgreSQL、结果事实才进 ClickHouse；把计算配置塞进 ClickHouse data plane 违背这个划分。
- worker 本来就从 PostgreSQL 读取 `portfolio_run`，加一张表零额外依赖。
- 不采用 dbt seed：seed 是 build-time 产物，而 `portfolio_metric_config` 是 run-time 按 run 生成的不可变快照，时序对不上。

`portfolio_metric_config` 不进 ClickHouse（v1）。ClickHouse `fleur_calculation` 侧只通过 `config_hash` 指向"用了哪套配置"，不知道配置 payload 全量；这保持 control plane / calculation output 边界干净：PostgreSQL 知道"配置是什么"，ClickHouse 只知道"结果是什么 + 用了哪个 hash"。

#### 字段

| 字段 | 说明 | 第一版默认 |
|---|---|---|
| `portfolio_run_id` | 运行 ID | — |
| `result_attempt_id` | 结果版本 | — |
| `security_code` | benchmark 指数代码 | `000300.SH` |
| `window_key` | 窗口 | `full_period` |
| `annualization_days` | 年化基数 | `252` |
| `min_observations` | 最小样本数，低于此值不产出指标 | `20` |
| `return_basis` | 组合收益口径 | `price_return` |
| `first_day_return_handling` | 首日 `daily_return=NULL` 处理 | `exclude` |
| `risk_free_tenor` | 期限 | `1y` |
| `risk_free_daily_method` | 日频折算方法 | `compound` |
| `risk_free_fill_strategy` | 缺口策略 | `forward_fill` |
| `benchmark_return_basis` | benchmark 收益口径 | `price_index` |
| `benchmark_fill_strategy` | benchmark 缺失日处理 | `skip` |
| `mar` | 最低可接受收益率 | `0` |
| `mar_basis` | MAR 来源 | `fixed` |
| `alignment_strategy` | 日期对齐 | `inner_join_trade_dates` |
| `zero_division_policy` | 零除处理 | `null` |
| `window_start` | 窗口起（`full_period`/`ytd` 时为 NULL） | NULL |
| `window_end` | 窗口止（`full_period`/`ytd` 时为 NULL） | NULL |
| `config_hash` | canonical payload 的 sha256 | — |
| `config_version` | schema 版本 | `1` |

主键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`。

#### 不存放

- NAV / `daily_return` / `drawdown` 实际数值 → ClickHouse `portfolio_nav_daily`。
- 指标结果值 → ClickHouse `fleur_calculation.calc_portfolio_performance_metric`。
- 账户与执行参数（初始资金、费率、滑点、止盈止损）→ `portfolio_run` 的 `account_snapshot` / `execution_snapshot`。
- 证券级参数。

#### 与 calc_portfolio_performance_metric 的关系

`calc_portfolio_performance_metric` 每行冗余 `config_hash`，用于跨 run 对比和配置漂移检测（按 hash 分组识别"哪些结果用同一套配置算的"），但不冗余 config payload 全量。计算口径变更必须同步更新 `portfolio_metric_config` 并生成新 hash。

#### 命名

现有 rearview-core 已有 `metric_policy.yml`（选股 metric catalog，`close_price` / `kdj_j_value` 这类可过滤/打分列目录），与本表的"组合绩效计算参数"是两个领域。为避免长期混淆，本表使用 `portfolio_` 前缀独立命名，不与选股 `metric_policy` 复用 schema；Rust 侧可复用加载/overlay 机制，但领域结构独立。

## 幂等和重算

### D13: result_attempt_id 生命周期

- 每次重算生成新 `result_attempt_id`。
- ClickHouse 结果事实都带 `portfolio_run_id` 和 `result_attempt_id`，append-only。
- PostgreSQL `portfolio_run.current_result_attempt_id` 指向当前有效 attempt。
- 旧 attempt 不删除，保留历史对比和审计；按分区 TTL 或显式清理策略管理。
- Rearview API 查询时默认带 `current_result_attempt_id` 过滤，可选查询历史 attempt。

## 绩效指标口径

口径定义基于日频收益率，默认 `annualization_days = 252`。符号：`R_p` 组合日收益、`R_b` benchmark 日收益、`R_f` 无风险利率、`n` 有效样本数、`MAR` 最低可接受收益率。

```text
holding_period_return = product(1 + R_p) - 1
annualized_return = (1 + holding_period_return) ^ (252 / n) - 1
annualized_volatility = stddevSamp(R_p) * sqrt(252)
sharpe_ratio = (annualized_return - annual_risk_free_rate) / annualized_volatility
downside_deviation = sqrt(sum(pow(least(R_p - mar_daily, 0), 2)) / (n - 1)) * sqrt(252)
sortino_ratio = (annualized_return - annual_risk_free_rate) / downside_deviation
drawdown = nav / running_max(nav) - 1
max_drawdown = abs(min(drawdown))
calmar_ratio = annualized_return / max_drawdown
active_return_daily = R_p - R_b
benchmark_holding_period_return = product(1 + R_b) - 1
benchmark_annualized_return = (1 + benchmark_holding_period_return) ^ (252 / n) - 1
information_ratio = (annualized_return - benchmark_annualized_return) / (stddevSamp(active_return_daily) * sqrt(252))
beta = covarSamp(R_p, R_b) / varSamp(R_b)
alpha = annualized_return - (annual_risk_free_rate + beta * (benchmark_annualized_return - annual_risk_free_rate))
treynor_ratio = (annualized_return - annual_risk_free_rate) / beta
```

worker 权威计算使用此口径；口径变更必须同步更新 `portfolio_metric_config` 并生成新 `config_hash`。dbt 不复算该公式，只消费 `fleur_calculation` 产物。

## 阶段划分

### 第一阶段：结果事实迁移与 worker 切换（Plan 0043）

1. 新增 ClickHouse `fleur_portfolio` database 和七张组合结果事实表（DDL 由 Rust 拥有）。
2. `result_attempt_id` 幂等重算，append-only，`portfolio_run.current_result_attempt_id` 指向当前有效 attempt。
3. worker 改为写 ClickHouse 结果事实 + 回写 PostgreSQL 控制状态。
4. Rearview API 从 ClickHouse 读取结果。

本阶段只做存储迁移和幂等，不碰绩效指标计算；明细见 [Plan 0043](../../plans/archive/0043-portfolio-data-plane-clickhouse-phase1-implementation-plan.md)。

### 第二阶段：绩效指标与 mart 输入（Plan 0044，待立项）

5. 新增 dbt intermediate / mart `int_risk_free_rate_daily`、`mart_risk_free_rate_daily`、`mart_benchmark_returns_daily`。
6. 新增 PostgreSQL `portfolio_metric_config` 表。
7. 新增 ClickHouse `fleur_calculation.calc_portfolio_performance_metric` 表与 worker 绩效指标权威计算。
8. dbt source / thin wrapper、contract 校验和跨 run 批量排名。

完成后可权威计算 12 个核心指标。

### 第三阶段：交易级指标

9. 新增 closed trade ledger（lot 配对、`realized_pnl`、`realized_return`）。
10. 胜率、盈亏比、平均盈利、平均亏损、单笔最大亏损。

### 后续阶段

11. benchmark 成分权重 + 行业分类（持仓归因）。
12. 全收益 benchmark（当组合 NAV 改造为 total-return 时同步）。
13. 跨 run 批量回测对比和排名。

## 迁移策略

短期保留 PostgreSQL 结果表作为兼容层，但不再扩大其结果事实：

1. 新增 ClickHouse `fleur_portfolio` database 和结果事实表。
2. worker 改为写 ClickHouse，再回写 PostgreSQL 状态和 `current_result_attempt_id`。
3. Rearview API 查询从 ClickHouse 读取，PostgreSQL 只用于 run 状态和控制信息。
4. Racingline API contract 保持稳定，前端不感知迁移。
5. 旧 PostgreSQL `portfolio_nav` / `portfolio_position_day` 等作为过渡回填来源，迁移完成后标记 deprecated。

## 验收标准

1. worker 成功把一个完整组合运行的结果写入 ClickHouse `fleur_portfolio.*`，PostgreSQL 只保留控制状态和 `current_result_attempt_id`。
2. 同一 `portfolio_run_id` 重算生成新 `result_attempt_id`，旧 attempt 保留，PostgreSQL 指针更新。
3. `int_risk_free_rate_daily`、`mart_risk_free_rate_daily` 和 `mart_benchmark_returns_daily` 产出与 `int_government_bond_yields_daily` / `int_benchmark_returns_daily` 口径一致的对齐序列。
4. 12 个核心绩效指标在 `fleur_calculation.calc_portfolio_performance_metric` 中产出，dbt thin wrapper 和 ranking 可消费。
5. Rearview API 返回的组合结果来自 ClickHouse，Racingline 前端无感知。
6. 净值递推仍由 Rust 单测覆盖（ADR 0012），ClickHouse SQL 中无 NAV 递推公式。

## 待决问题

1. closed trade ledger 的 lot 配对细节和部分成交拆分规则。
2. `portfolio_metric_config` 的 `config_hash` 校验强度，以及 dbt thin wrapper 是否需要物化为 ClickHouse 读模型。
3. 旧 attempt 的 TTL 和清理策略（按 attempt 数量还是按时间）。
4. 跨 run 批量回测对比是否需要独立 `fleur_backtest` database 还是复用 `fleur_portfolio`。
5. 是否需要新增独立 `backtester` crate，或继续扩展 `rearview-portfolio-worker`。
6. benchmark 成分权重和行业分类的 raw source 采集方案。

## 相关文档

- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](../../Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [Q&A 0002: Portfolio Metrics 基础数据缺口](../../Q&A/0002-portfolio-metrics.md)
- [ADR 0009: ClickHouse 按 dbt 建模层分库](../../ADR/0009-clickhouse-layered-databases.md)
- [ADR 0012: 组合净值递推与绩效指标权威计算留在 Rust](../../ADR/0012-portfolio-nav-recursion-stays-in-rust.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](0021-racingline-virtual-account-portfolio-rebalancing.md)
- [System: Rearview](../../architecture/rearview.md)
- [System: Data Platform](../../architecture/data-platform.md)
- [engines/crates/rearview-core/src/portfolio/mod.rs](../../../engines/crates/rearview-core/src/portfolio/mod.rs)
- [pipeline/migrate/versions/rearview/0003_create_rearview_portfolio_schema.py](../../../pipeline/migrate/versions/rearview/0003_create_rearview_portfolio_schema.py)
