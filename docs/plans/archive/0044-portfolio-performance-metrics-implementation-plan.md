# Plan 0044: 组合绩效指标、dbt 输入与交易级指标实施计划

日期：2026-06-17

状态：Archived

关联文档：

- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](../../Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [Q&A 0002: Portfolio Metrics 基础数据缺口](../../Q&A/0002-portfolio-metrics.md)
- [RFC 0022: 组合数据面迁移 ClickHouse 与绩效指标分层](../../RFC/archive/0022-portfolio-data-plane-clickhouse-and-metrics.md)
- [ADR 0012: 组合净值递推与绩效指标权威计算留在 Rust](../../ADR/0012-portfolio-nav-recursion-stays-in-rust.md)
- [ADR 0009: ClickHouse 按数据层和计算产物分库](../../ADR/0009-clickhouse-layered-databases.md)
- [Plan 0043: 组合数据面迁移 ClickHouse 第一阶段实施计划](0043-portfolio-data-plane-clickhouse-phase1-implementation-plan.md)

## 目标

1. 新增 dbt intermediate 层 `int_risk_free_rate_daily`，并新增 mart 层 `mart_risk_free_rate_daily` 和 `mart_benchmark_returns_daily` 作为 worker 读取 risk-free / benchmark 日频收益的稳定入口。
2. 新增 PostgreSQL `portfolio_metric_config` 表，把绩效指标口径保存为 per run attempt 的不可变 control-plane 配置。
3. 新增 ClickHouse `fleur_calculation.calc_portfolio_performance_metric` 和 `calc_portfolio_performance_metric_status` 表，由 worker 权威计算 12 个核心绩效指标、row-level 输入状态和 metric-level 可空原因并写入。
4. 新增 dbt source / thin wrapper / mart ranking 模型。dbt 不复算绩效指标公式，只做 contract 校验、消费层建模和跨 run 批量排名。
5. 新增 closed trade ledger，补齐交易级 realized PnL / realized return，支撑胜率、盈亏比等交易质量指标。

## 非目标

1. 不把 NAV、`daily_return`、`drawdown` 的递推搬到 ClickHouse SQL 或 dbt 模型。mart 只能消费 `portfolio_nav_daily` 的既有结果事实，不能重写账本状态机。
2. 不引入 total-return benchmark；本计划仍使用价格指数简单收益，口径通过配置和文档显式标记。
3. 不做持仓归因、行业归因、benchmark 成分权重或风格暴露。
4. 不让 Racingline 直接查询 ClickHouse 或 PostgreSQL。若需要前端展示，仍通过 Rearview API。
5. 不在 dbt seed 中维护 run-time 绩效配置。`portfolio_metric_config` 属于 PostgreSQL control plane。
6. 不清理旧 ClickHouse result attempts；attempt TTL / 清理策略另立计划。
7. 不让 Rust worker 直接写入 `fleur_marts`。`fleur_marts` 只由 dbt materialize，worker 指标计算产物先进入 `fleur_calculation`。

## 当前事实基线

Plan 0043 已交付的基线：

1. PostgreSQL `portfolio_run.current_result_attempt_id` 已存在，迁移为 `0004_add_current_result_attempt.py`。
2. PostgreSQL 旧结果事实表已通过 `0005_drop_portfolio_pg_result_facts.py` 清理，当前保留 `portfolio_run` 和 `portfolio_task_outbox` 作为 control plane。
3. Rust worker 已拥有 `fleur_portfolio` 七张结果事实表的 DDL 和写入路径：`portfolio_run_snapshot`、`portfolio_nav_daily`、`portfolio_position_day`、`portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_event`。
4. ClickHouse 结果事实采用 `MergeTree`、append-only、`portfolio_run_id + result_attempt_id` 过滤，API 默认使用当前有效 attempt。
5. `int_benchmark_returns_daily` 已存在，benchmark 标识采用 canonical `security_code`，字段包含 `trade_date`、`close_price`、`prev_close_price`、`return_daily`。
6. `int_government_bond_yields_daily` 已存在，收益率字段保留百分比点口径，`one_year_yield_pct` 是第一版 risk-free 默认来源。

本计划开始前仍缺：

1. `pipeline/elt/models/intermediate/int_risk_free_rate_daily.sql` / `.yml`。
2. `pipeline/elt/models/marts/mart_risk_free_rate_daily.sql` / `.yml`。
3. `pipeline/elt/models/marts/mart_benchmark_returns_daily.sql` / `.yml`。
4. PostgreSQL `portfolio_metric_config`。
5. ClickHouse `calc_portfolio_performance_metric` 和 `calc_portfolio_performance_metric_status`。
6. Rust worker 的 benchmark / risk-free 读取、日期对齐、12 个核心指标计算和写入。
7. dbt 对 worker 指标产物的 source/thin wrapper、contract 校验和跨 run 排名模型。
8. closed trade ledger、open lot 状态扩展和交易级指标。

## 设计原则

### 职责分层

| 职责 | 承载方 | 说明 |
|---|---|---|
| NAV / `daily_return` / `drawdown` 递推 | Rust `rearview-core` | 权威账本状态机，单测覆盖 |
| 结果事实存储 | ClickHouse `fleur_portfolio` | append-only + `result_attempt_id` |
| risk-free 折算和交易日对齐 | dbt `fleur_intermediate` | 可复用业务中间过程，不面向 worker 直接消费 |
| risk-free / benchmark 日频稳定入口 | dbt `fleur_marts` | worker 只读 mart 薄封装，不直接读 intermediate |
| 指标口径配置 | PostgreSQL `portfolio_metric_config` | control plane，可复现配置 hash |
| 12 个核心指标权威计算 | Rust worker + ClickHouse `fleur_calculation` | 与 run attempt 同步落库，不写 `fleur_marts` |
| 指标消费、校验、跨 run 排名 | dbt source / thin wrapper / mart | 不重算公式，只做 contract、质量检查和排名 |
| closed trade ledger | Rust worker + ClickHouse `fleur_calculation` | 交易 lot 配对依赖账本执行语义 |

### 指标口径

第一版 12 个核心指标：

| 指标 | 字段名 | 依赖 |
|---|---|---|
| 区间收益率 | `holding_period_return` | `portfolio_nav_daily.daily_return` |
| 年化收益率 | `annualized_return` | `daily_return`, `annualization_days` |
| 年化波动率 | `annualized_volatility` | `daily_return` |
| 最大回撤 | `max_drawdown` | `portfolio_nav_daily.drawdown` |
| 卡尔玛比率 | `calmar_ratio` | `annualized_return`, `max_drawdown` |
| 下行波动率 | `downside_deviation` | `daily_return`, `mar_daily` |
| 索提诺比率 | `sortino_ratio` | `annualized_return`, risk-free / MAR |
| 夏普比率 | `sharpe_ratio` | `annualized_return`, risk-free, volatility |
| 信息比率 | `information_ratio` | active return vs benchmark |
| Beta | `beta` | covariance vs benchmark |
| Alpha | `alpha` | CAPM alpha |
| 特雷诺比率 | `treynor_ratio` | excess return / beta |

默认配置：

| 配置 | 默认值 |
|---|---|
| `annualization_days` | `252` |
| `return_type` | `simple` |
| `risk_free_tenor` | `1y` |
| `risk_free_daily_method` | `compound` |
| `risk_free_fill_strategy` | `forward_fill` |
| `security_code` | `000300.SH` |
| `benchmark_fill_strategy` | `skip` |
| `benchmark_return_basis` | `price_index` |
| `portfolio_return_basis` | `price_return` |
| `first_day_return_handling` | `exclude` |
| `alignment_strategy` | `inner_join_trade_dates` |
| `min_observations` | `20` |
| `zero_division_policy` | `null` |

### 计算规则

符号：

| 符号 | 含义 |
|---|---|
| `R_p` | 组合日收益率序列，来自 `portfolio_nav_daily.daily_return` |
| `R_b` | benchmark 日收益率序列，来自 `mart_benchmark_returns_daily.return_daily` |
| `R_f_daily` | 无风险日收益率，来自 `mart_risk_free_rate_daily.daily_rate` |
| `n` | 对齐后的有效交易日样本数，排除首日 `daily_return = NULL` |
| `MAR_daily` | 下行波动率最低可接受日收益，默认 `0`；如配置为 risk-free，则使用 `R_f_daily` |

日期对齐采用 `inner_join_trade_dates`：组合、benchmark、risk-free 三者同日均有有效值才进入样本。所有需要标准差、协方差、方差的指标使用样本口径，对应 ClickHouse `stddevSamp`、`covarSamp`、`varSamp`，分母为 `n - 1`。

无风险利率：

```text
R_f_daily = (1 + R_f_annual) ^ (1 / 252) - 1
annual_risk_free_rate = product(1 + R_f_daily) ^ (252 / n) - 1
```

区间收益率：

```text
holding_period_return = product(1 + R_p) - 1
```

年化收益率：

```text
annualized_return = (1 + holding_period_return) ^ (252 / n) - 1
```

年化波动率：

```text
annualized_volatility = stddevSamp(R_p) * sqrt(252)
```

夏普比率：

```text
sharpe_ratio = (annualized_return - annual_risk_free_rate) / annualized_volatility
```

下行波动率：

```text
downside_deviation =
  sqrt(sum(pow(least(R_p - MAR_daily, 0), 2)) / (n - 1)) * sqrt(252)
```

索提诺比率：

```text
sortino_ratio = (annualized_return - annual_risk_free_rate) / downside_deviation
```

最大回撤：

```text
drawdown = nav / running_max(nav) - 1
max_drawdown = abs(min(drawdown))
```

`portfolio_nav_daily.drawdown` 可继续保存为非正数；`calc_portfolio_performance_metric.max_drawdown` 按本计划保存正数最大回撤幅度。

卡尔玛比率：

```text
calmar_ratio = annualized_return / max_drawdown
```

benchmark 年化收益：

```text
benchmark_holding_period_return = product(1 + R_b) - 1
benchmark_annualized_return = (1 + benchmark_holding_period_return) ^ (252 / n) - 1
```

信息比率：

```text
active_return_daily = R_p - R_b
tracking_error = stddevSamp(active_return_daily) * sqrt(252)
information_ratio = (annualized_return - benchmark_annualized_return) / tracking_error
```

Beta：

```text
beta = covarSamp(R_p, R_b) / varSamp(R_b)
```

Alpha：

```text
alpha =
  annualized_return
  - (annual_risk_free_rate + beta * (benchmark_annualized_return - annual_risk_free_rate))
```

特雷诺比率：

```text
treynor_ratio = (annualized_return - annual_risk_free_rate) / beta
```

零除处理统一遵守 `zero_division_policy = null`：当年化波动率、下行波动率、最大回撤、tracking error、benchmark 方差或 beta 为 0 时，对应比率写 NULL。

状态拆分：

- `calc_portfolio_performance_metric.metric_status` 是 row-level 输入状态，只描述该 run / attempt / benchmark / window 是否有足够输入样本进入指标计算。
- `calc_portfolio_performance_metric_status.metric_status` 是 metric-level 状态，一行解释一个指标是否成功、是否因零除 / 样本不足 / 输入缺失而为 NULL。
- 宽表指标字段为 NULL 时，必须能在 metric-level 状态表中找到同一 `metric_name` 的非 `succeeded` 原因。

### 字段裁剪规则

- 日频输入表只保留粒度键、观测日期、源日期和值字段；计算口径放在 YAML `meta`、设计文档和 `portfolio_metric_config`。
- mart 薄封装只暴露 worker / dbt wrapper 需要的稳定 contract；审计用源字段保留在 upstream intermediate，不向 mart 重复透出。
- PostgreSQL `portfolio_metric_config` 使用 typed columns 作为权威配置事实，不再同时保存同内容的 JSON payload。
- ClickHouse 指标表只保存最终指标、状态和样本数；risk-free 年化值、benchmark 年化值等中间量不持久化，必要时由 worker debug 日志或临时查询复现。
- closed trade ledger 保存成交配对和现金金额事实；`total_fee`、`realized_return` 等可由基础金额直接推导的字段不重复存储。

## ClickHouse 规则依据

Workload shape：market data / financial services，混合时序 OLAP 和按 run / attempt 点查。

| 规则 | 本计划应用 |
|---|---|
| `schema-pk-plan-before-creation` | `calc_portfolio_performance_metric` 和 `calc_portfolio_closed_trade` 在建表前固定主要查询模式，避免后续迁移 ORDER BY |
| `schema-pk-prioritize-filters` | performance metric 优先服务 API 点查：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key` |
| `schema-pk-cardinality-order` | ledger 时间序列表按 run / attempt / 日期排序，避免高基数随机字段作为唯一前缀 |
| `schema-pk-filter-on-orderby` | API 和 dbt source 查询必须带 run / attempt，跨 run 排名交给 mart 排名模型，不强压在同一物理表 |
| `schema-types-native-types` | 日期、计数、数值指标使用 `Date`、`UInt32`、`Float64`、`Bool`，不全部用 `String` |
| `schema-types-lowcardinality` | `window_key`、`metric_status`、`security_code`、`exit_reason` 等低基数字段使用 `LowCardinality(String)` |
| `schema-types-avoid-nullable` | 只有因样本不足、零除或首日收益缺失而语义上可空的指标使用 `Nullable(Float64)` |
| `schema-partition-low-cardinality` | closed trade ledger 按 `toYYYYMM(exit_date)` 月分区，不按 run 或证券分区 |
| `schema-partition-start-without` | `calc_portfolio_performance_metric` 预计行数小、生命周期暂不按日期删除，第一版不分区 |
| `insert-batch-size` | worker 继续按表批量写入，不做单行写入；单 run 超过 100K 行时再分片 |
| `insert-mutation-avoid-update` / `insert-mutation-avoid-delete` | 指标重算生成新 `result_attempt_id`，不更新覆盖旧行 |
| `query-join-filter-before` | dbt ranking / API 分析先过滤 run、attempt、benchmark、window，再关联维度或配置 |

架构分类：

- append-only attempt 模型：derived，来自 ClickHouse mutation avoidance 与 fleur 审计需求。
- `calc_portfolio_performance_metric` 不分区：derived，基于行数小、点查为主、无明确生命周期删除需求。
- closed trade ledger 月分区：derived，基于时间序列保留和按退出日期分析。
- 交易级指标单独建表而不塞入 benchmark-scoped performance row：field，避免 benchmark-independent 指标被每个 benchmark 重复。

## 实施阶段

### 阶段 1：dbt risk-free / benchmark 输入补齐

**目标**：把可复用转换逻辑放在 intermediate 层，把 worker / API / dbt wrapper 读取的稳定 contract 放在 mart 层。

**新增模型**：

1. `pipeline/elt/models/intermediate/int_risk_free_rate_daily.sql`
2. `pipeline/elt/models/intermediate/int_risk_free_rate_daily.yml`
3. `pipeline/elt/models/marts/mart_risk_free_rate_daily.sql`
4. `pipeline/elt/models/marts/mart_risk_free_rate_daily.yml`
5. `pipeline/elt/models/marts/mart_benchmark_returns_daily.sql`
6. `pipeline/elt/models/marts/mart_benchmark_returns_daily.yml`
7. 对应设计文档：
   - `docs/architecture/dbt_layer/fleur_intermediate/int_risk_free_rate_daily.md`
   - `docs/architecture/dbt_layer/fleur_marts/mart_risk_free_rate_daily.md`
   - `docs/architecture/dbt_layer/fleur_marts/mart_benchmark_returns_daily.md`

**分层决策**：

- `int_risk_free_rate_daily` 承担期限选择、百分比点转小数、交易日对齐、forward-fill 和日频折算，是可复用业务中间过程。
- `mart_risk_free_rate_daily` 从 `int_risk_free_rate_daily` 薄封装，作为 worker 和后续 dbt wrapper 的稳定读取入口。
- `mart_benchmark_returns_daily` 从现有 `int_benchmark_returns_daily` 薄封装，避免 worker 直接读取 intermediate 层；不重复维护 benchmark universe。

**`int_risk_free_rate_daily` 粒度**：

一行代表一个 `trade_date + source_tenor` 的日频无风险收益。

| 字段 | 说明 |
|---|---|
| `trade_date` | 交易日，来自 `int_trade_calendar` |
| `source_date` | 实际使用的 ChinaBond 曲线日期 |
| `source_tenor` | 第一版必须支持 `1y` |
| `annual_rate` | 年化利率，小数比例，`one_year_yield_pct / 100` |
| `daily_rate` | 日频收益，默认 `(1 + annual_rate) ^ (1 / 252) - 1` |

实现要点：

- 从 `ref('int_government_bond_yields_daily')` 读取百分比点口径字段，不在 worker 中做单位转换。
- 只 forward-fill `source_date <= trade_date` 的历史值，不允许用未来值补齐。
- 用 `ref('int_trade_calendar')` 生成交易日栅格。
- 第一版只输出 `1y`，但 SQL/YAML 按 `source_tenor` 保留扩展位。
- `annualization_days = 252`、`day_count_basis = 252_trading_days`、`daily_method = compound` 和 `fill_strategy = forward_fill` 是模型口径，不作为日频表行字段；写入 YAML `meta`、设计文档和 `portfolio_metric_config`。
- 如果 `trade_date` 早于第一条 risk-free source，`annual_rate` / `daily_rate` 保持 NULL，worker 按 `min_observations` 和 `metric_status` 处理。

**`mart_risk_free_rate_daily` 粒度**：

与 `int_risk_free_rate_daily` 相同，一行代表一个 `trade_date + source_tenor` 的 worker-ready 日频无风险收益。

实现要点：

- 仅从 `ref('int_risk_free_rate_daily')` 选择稳定字段，不重新实现期限选择、forward-fill 或日频折算。
- 字段命名和 YAML 文档面向 worker / dbt wrapper 消费，作为对外 contract。
- 后续如 risk-free 口径扩展到多期限或多来源，先扩展 `int_risk_free_rate_daily`，mart 仍保持稳定字段集或受控新增字段。

**`mart_benchmark_returns_daily` 粒度**：

一行代表一个 benchmark `security_code + trade_date` 的价格指数日收益。

| 字段 | 说明 |
|---|---|
| `security_code` | canonical benchmark 代码，如 `000300.SH` |
| `trade_date` | benchmark 交易日 |
| `return_daily` | 价格指数简单日收益 |

实现要点：

- 从 `ref('int_benchmark_returns_daily')` 透传，保持 `security_code` 作为唯一 benchmark 标识。
- 只暴露 worker / dbt wrapper 需要的 `return_daily`，不透出 `close_price` / `prev_close_price`；价格审计留在 `int_benchmark_returns_daily`。
- 不重新引入 `benchmark_key` 或 `benchmark_name`。
- benchmark 收益口径 `price_index` 写入 YAML `meta` 和设计文档，不在日频表重复字段。
- 不在此模型派生窗口级累计收益；窗口聚合由 worker 按 run 窗口计算。

**测试策略**：

- `int_risk_free_rate_daily`: unique `trade_date, source_tenor`；`trade_date` / `source_tenor` not null；`daily_rate` 在 `annual_rate` 非空时 not null；用 `dbt show` 校验 risk-free 单位从百分比点转为小数比例。
- `mart_risk_free_rate_daily`: unique `trade_date, source_tenor`；字段与 `int_risk_free_rate_daily` 关键 contract 一致，不重算转换逻辑。
- `mart_benchmark_returns_daily`: unique `security_code, trade_date`；`security_code` not null + `cn_security_code_format`；`trade_date` not null；relationships 到 `int_benchmark_basic_snapshot.security_code`。

### 阶段 2：PostgreSQL `portfolio_metric_config`

**目标**：把绩效指标口径固化为 attempt 级控制面事实。

**实现**：

1. 新增 Alembic migration `pipeline/migrate/versions/rearview/0006_create_portfolio_metric_config.py`。
2. 新增 Rust domain/config struct，例如 `PortfolioMetricConfig` 和 `ResolvedPortfolioMetricConfig`。
3. worker 生成 `result_attempt_id` 后，从 `portfolio_run.execution_snapshot` 和系统默认值解析最终 metric config，按 typed columns 写入 PostgreSQL，并用同一 canonical 字段序列计算 `sha256` `config_hash`。
4. `calc_portfolio_performance_metric.config_hash` 只引用 hash，不冗余配置明细。

**表字段**：

| 字段 | 说明 |
|---|---|
| `portfolio_run_id` | run ID |
| `result_attempt_id` | attempt ID |
| `security_code` | benchmark 指数代码 |
| `window_key` | `full_period` / `ytd` / 后续扩展 |
| `window_start` | 自定义窗口起点，`full_period` 可为 NULL |
| `window_end` | 自定义窗口终点，`full_period` 可为 NULL |
| `annualization_days` | 年化基数 |
| `min_observations` | 最小有效样本数 |
| `portfolio_return_basis` | `price_return` |
| `benchmark_return_basis` | `price_index` |
| `risk_free_tenor` | `1y` |
| `risk_free_daily_method` | `compound` |
| `risk_free_fill_strategy` | `forward_fill` |
| `benchmark_fill_strategy` | `skip` |
| `mar` | MAR 数值 |
| `mar_basis` | `fixed` / 后续 `risk_free_daily` |
| `alignment_strategy` | `inner_join_trade_dates` |
| `first_day_return_handling` | `exclude` |
| `zero_division_policy` | `null` |
| `config_version` | 第一版 `1` |
| `config_hash` | canonical 字段序列的 sha256 |
| `created_at` | 写入时间 |

主键：`portfolio_run_id`, `result_attempt_id`, `security_code`, `window_key`。

约束：

- `annualization_days > 0`。
- `min_observations > 1`。
- `security_code` 非空。
- `config_hash` 非空。
- `window_start <= window_end`，允许两者同时为 NULL。
- `portfolio_run_id` 外键到 `portfolio_run`。

**写入时机**：

- run 创建阶段可把用户选择或默认 benchmark 写入 `execution_snapshot`。
- worker 在 attempt ID 生成后 materialize 最终配置行。这样 `portfolio_metric_config` 与 `result_attempt_id` 一一对应，避免 run 创建时 attempt 不存在的问题。
- 如果指标配置非法，run 标记 `failed_validation`；如果 mart 输入缺失，run 可成功但 row-level `metric_status` 写 `missing_benchmark` / `missing_risk_free_rate`，metric-level 状态表写对应原因。

### 阶段 3：ClickHouse performance metric 计算产物表

**目标**：新增 worker 权威指标计算产物表和 metric-level 状态解释表。

**DDL 归属**：

- 表属于 Rust worker owned `fleur_calculation`，DDL 加到 `engines/crates/rearview-core/src/clickhouse/portfolio_schema.rs` 或独立 `calculation_schema.rs`。
- dbt 只通过 `source()` 读取，不在 `fleur_calculation` materialize。
- worker 不直接写 `fleur_marts`。`fleur_marts` 是 dbt 消费层，后续由 mart ranking / consumption 模型从 thin wrapper 构建。
- 这是 ADR 0009 的外部计算产物路径：Rust worker 直接写 `fleur_calculation.*`，dbt 声明 source，再通过 `fleur_intermediate.int_*` thin wrapper 暴露稳定语义，下游 marts 通过 `ref()` 消费 wrapper。

**DDL 规格**：

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_portfolio_performance_metric
(
    portfolio_run_id          String,
    result_attempt_id         String,
    security_code             LowCardinality(String),
    window_key                LowCardinality(String),
    window_start              Nullable(Date),
    window_end                Nullable(Date),
    config_hash               String,
    metric_status             LowCardinality(String),
    observation_count         UInt32,
    holding_period_return     Nullable(Float64),
    annualized_return         Nullable(Float64),
    annualized_volatility     Nullable(Float64),
    max_drawdown              Nullable(Float64),
    calmar_ratio              Nullable(Float64),
    downside_deviation        Nullable(Float64),
    sortino_ratio             Nullable(Float64),
    sharpe_ratio              Nullable(Float64),
    information_ratio         Nullable(Float64),
    beta                      Nullable(Float64),
    alpha                     Nullable(Float64),
    treynor_ratio             Nullable(Float64),
    computed_at               DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (portfolio_run_id, result_attempt_id, security_code, window_key)
```

`calc_portfolio_performance_metric.metric_status` 是 row-level 输入状态，第一版取值：

- `succeeded`
- `insufficient_observations`
- `missing_benchmark`
- `missing_risk_free_rate`
- `invalid_input`

当 row-level 输入状态为 `succeeded` 时，宽表中仍允许部分指标因零除而为 NULL；具体原因写入 metric-level 状态表。

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_portfolio_performance_metric_status
(
    portfolio_run_id          String,
    result_attempt_id         String,
    security_code             LowCardinality(String),
    window_key                LowCardinality(String),
    metric_name               LowCardinality(String),
    metric_status             LowCardinality(String),
    reason_code               LowCardinality(String),
    computed_at               DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (portfolio_run_id, result_attempt_id, security_code, window_key, metric_name)
```

`calc_portfolio_performance_metric_status.metric_status` 第一版取值：

- `succeeded`
- `insufficient_observations`
- `missing_benchmark`
- `missing_risk_free_rate`
- `zero_division`
- `invalid_input`

`reason_code` 第一版取值：

- `none`
- `n_below_min_observations`
- `benchmark_series_missing`
- `risk_free_series_missing`
- `annualized_volatility_zero`
- `downside_deviation_zero`
- `max_drawdown_zero`
- `tracking_error_zero`
- `benchmark_variance_zero`
- `beta_zero`
- `invalid_return_value`

写入规则：

- `calc_portfolio_performance_metric` 每个 `portfolio_run_id + result_attempt_id + security_code + window_key` 一行。
- `calc_portfolio_performance_metric_status` 每个 `portfolio_run_id + result_attempt_id + security_code + window_key + metric_name` 一行，覆盖 12 个核心指标。
- 若某些指标因零除不可计算，宽表按 `zero_division_policy = null` 写 NULL，row-level `metric_status` 可仍为 `succeeded`，metric-level 状态写具体 `reason_code`。
- 若样本不足或输入缺失导致整行不可计算，宽表 row-level `metric_status` 写 `insufficient_observations` / `missing_benchmark` / `missing_risk_free_rate`，metric-level 状态表为 12 个指标分别写相同原因。
- 不保存 `annual_risk_free_rate`、`benchmark_annualized_return` 等中间量；dbt 不复算公式，必要审计通过 worker 单测、输入快照和临时 query 完成。
- 不使用 `ALTER UPDATE` 修正旧 attempt；计算口径变化生成新 attempt 或新 `config_hash`。
- `portfolio_run_snapshot` 仍是 attempt 完整落库标记。performance metric 纳入 attempt 完整性后，写入顺序必须调整为：结果事实 → metric config → performance metric 宽表 → metric-level 状态表 → snapshot。

### 阶段 4：worker 绩效指标权威计算

**目标**：worker 在一个组合 run 完成时权威计算 12 个核心指标。

**实现**：

1. 新增 `rearview-core/src/portfolio/performance.rs` 或等价模块，放置无状态指标计算函数和配置类型。
2. `ClickHouseClient` 新增读取方法：

```rust
query_mart_benchmark_returns(security_code, start_date, end_date)
query_mart_risk_free_rates(source_tenor, start_date, end_date)
```

3. `process_run` 流程扩展：

```text
build_simulation_input
simulate_portfolio
generate result_attempt_id
resolve + persist portfolio_metric_config
query benchmark/risk-free mart inputs
compute performance metrics
write ClickHouse result facts + calc performance metrics + metric status
finalize PostgreSQL portfolio_run
```

4. 日期对齐遵守 `alignment_strategy = inner_join_trade_dates`：
   - 排除首日 `daily_return = NULL`。
   - 只保留 portfolio、benchmark、risk-free 三者均有值的日期。
   - benchmark 缺失按 `benchmark_fill_strategy = skip` 跳过该日。
   - risk-free 缺失由 mart forward-fill；仍为空则跳过并记录状态。
5. 样本数低于 `min_observations` 时写一行 row-level `metric_status = 'insufficient_observations'`，指标字段为 NULL 或仅写可安全解释的字段，并为 12 个指标写 metric-level 状态行。
6. 公式单元测试覆盖：
   - 区间收益率和年化收益率。
   - 年化波动率。
   - 下行波动率使用 `n - 1` 样本分母。
   - max drawdown 使用 `abs(min(drawdown))`，指标表保存正数 MDD。
   - 信息比率使用 `(annualized_return - benchmark_annualized_return) / tracking_error`。
   - beta 使用日频 `covarSamp(R_p, R_b) / varSamp(R_b)`，不年化。
   - alpha 使用 Jensen alpha：`annualized_return - (annual_risk_free_rate + beta * (benchmark_annualized_return - annual_risk_free_rate))`。
   - zero volatility / zero beta / zero max drawdown / zero tracking error 返回 NULL。
   - 宽表 NULL 指标都能在 `calc_portfolio_performance_metric_status` 找到对应 `reason_code`。
   - benchmark 与 risk-free 日期对齐。
   - `config_hash` 对 canonical 字段序列稳定。

### 阶段 5：dbt source / thin wrapper、质量检查和跨 run 排名

**目标**：让 dbt 消费 worker 计算产物，但不重算绩效指标公式。

**新增 dbt sources**：

- `source('fleur_portfolio', 'portfolio_run_snapshot')`
- `source('fleur_portfolio', 'portfolio_nav_daily')`
- `source('fleur_calculation', 'calc_portfolio_performance_metric')`
- `source('fleur_calculation', 'calc_portfolio_performance_metric_status')`
- 阶段 6 后追加 `source('fleur_calculation', 'calc_portfolio_closed_trade')` 和 `source('fleur_calculation', 'calc_portfolio_trade_metric')`

**新增 dbt wrapper / mart 模型**：

| 模型 | 粒度 | 用途 |
|---|---|---|
| `int_portfolio_performance_metric` | run + attempt + benchmark + window | `calc_portfolio_performance_metric` thin wrapper，inner join `portfolio_run_snapshot` 只保留完整 attempt，字段命名稳定、轻量过滤、文档/tests |
| `int_portfolio_performance_metric_status` | run + attempt + benchmark + window + metric_name | metric-level 状态 thin wrapper，用于解释宽表 NULL 指标 |
| `mart_portfolio_performance_metric_rank` | run + attempt + benchmark + window + metric_name | 同 config / benchmark / window 下跨 run 排名 |

实现约束：

- dbt 不重算 12 个核心指标公式，也不重算 NAV、`daily_return` 或 `drawdown`。
- `int_portfolio_performance_metric` 必须 inner join `portfolio_run_snapshot`，只暴露已写入完整 attempt 标记的指标产物，避免 worker 中途失败产生的半成品进入消费层。
- `int_portfolio_performance_metric` 只做字段选择、类型/命名稳定、完整 attempt 过滤和文档/tests。
- API current attempt 过滤继续由 Rearview 读取 PostgreSQL `current_result_attempt_id` 承担；dbt ranking 默认比较所有完整 attempts，并保留 `result_attempt_id` 维度，不把历史 attempt 折叠为当前 run。
- ranking 模型使用 long format，避免为每个指标新增一套排名列。
- ranking 只比较相同 `config_hash`、`security_code`、`window_key`、`metric_name` 下的指标，避免口径漂移混排。
- ranking 只纳入 metric-level `metric_status = 'succeeded'` 且指标值非 NULL 的行。

**排名方向 catalog**：

第一版在 dbt YAML `meta` 或独立 seed/model 中声明排名方向，不允许在 SQL 中散落硬编码。

| metric_name | rank_direction | NULL policy |
|---|---|---|
| `holding_period_return` | `desc` | exclude |
| `annualized_return` | `desc` | exclude |
| `annualized_volatility` | `asc` | exclude |
| `max_drawdown` | `asc` | exclude |
| `calmar_ratio` | `desc` | exclude |
| `downside_deviation` | `asc` | exclude |
| `sortino_ratio` | `desc` | exclude |
| `sharpe_ratio` | `desc` | exclude |
| `information_ratio` | `desc` | exclude |
| `beta` | `none` | exclude |
| `alpha` | `desc` | exclude |
| `treynor_ratio` | `desc` | exclude |

`beta` 第一版不默认参与“优劣”排名，只可用于筛选或展示；如后续要排序，必须先定义目标 beta（例如 closest-to-1 或 closest-to-0）并更新 catalog。排名并列使用 `dense_rank()`，同时保留原始 `metric_value` 方便二次排序。

**测试策略**：

- `int_portfolio_performance_metric`: unique run + attempt + benchmark + window；row-level `metric_status` accepted values；完整 attempt join 后行数不超过 raw metric 表。
- `int_portfolio_performance_metric_status`: unique run + attempt + benchmark + window + metric；metric-level `metric_status` / `reason_code` accepted values；每个宽表 row 有 12 个状态行。
- `mart_portfolio_performance_metric_rank`: `metric_rank` not null；同一分组 rank 连续性可用聚合 query smoke test 验证；`rank_direction = none` 的指标不得产出 rank。

### 阶段 6：closed trade ledger 与交易级指标

**目标**：把 `portfolio_trade` 成交流水升级为可计算交易质量的闭仓 ledger。

**设计决策**：

closed trade ledger 由 Rust worker 输出，而不是 dbt 从 `portfolio_trade` 事后猜测。原因：

- lot 配对依赖执行语义、持仓状态和卖出原因。
- worker 已经维护逐日仓位和订单/成交因果链。
- dbt 从成交流水还原 lot 容易与 worker 账本分叉。

**新增 ClickHouse 表**：

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_portfolio_closed_trade
(
    portfolio_run_id       String,
    result_attempt_id      String,
    closed_trade_id        String,
    closed_trade_seq       UInt32,
    position_lot_id        String,
    entry_trade_seq        UInt32,
    exit_trade_seq         UInt32,
    security_code          LowCardinality(String),
    entry_date             Date,
    exit_date              Date,
    quantity               Float64,
    entry_gross_amount     Float64,
    exit_gross_amount      Float64,
    entry_fee              Float64,
    exit_fee               Float64,
    realized_pnl           Float64,
    holding_days           UInt32,
    exit_reason            LowCardinality(String),
    created_at             DateTime DEFAULT now()
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(exit_date)
ORDER BY (portfolio_run_id, result_attempt_id, exit_date, security_code, closed_trade_seq)
```

**lot 配对规则**：

- worker 必须从当前 aggregate `PositionState` 扩展出 open-lot 状态；closed ledger 不允许从聚合仓位倒推。
- 第一版使用 FIFO。
- 买入成交创建 open lot，生成稳定 `position_lot_id`，保留 open quantity、entry gross amount、entry fee 和 entry trade metadata。
- 卖出成交按 FIFO 消耗 open lots，可产生多个 closed trade row；同一 open lot 可被多次部分平仓，因此 closed row 身份不能只依赖 `position_lot_id`。
- `closed_trade_id` 是 closed row 稳定 ID；`closed_trade_seq` 是 run attempt 内单调序号，用于排序和测试。
- 部分平仓时按平仓数量比例分摊 `entry_gross_amount` / `entry_fee`。
- `entry_gross_amount` 是买入成交金额，不含费用；`exit_gross_amount` 是卖出成交金额，不含费用。不得把已含费用的 cost basis 写入 `entry_gross_amount`。
- `realized_pnl = exit_gross_amount - entry_gross_amount - entry_fee - exit_fee`。
- `total_fee` 和 `realized_return` 不在 ledger 中重复存储；mart / API 按 `entry_fee + exit_fee` 和 `realized_pnl / (entry_gross_amount + entry_fee)` 计算，分母为 0 时返回 NULL。
- 未闭仓 lot 不写入 closed ledger；仍通过 `portfolio_position_day` 表达未实现收益。

**交易级指标**：

为避免 benchmark-independent 指标重复写入 benchmark-scoped `calc_portfolio_performance_metric`，新增 `calc_portfolio_trade_metric`：

```sql
CREATE TABLE IF NOT EXISTS fleur_calculation.calc_portfolio_trade_metric
(
    portfolio_run_id          String,
    result_attempt_id         String,
    window_key                LowCardinality(String),
    window_start              Nullable(Date),
    window_end                Nullable(Date),
    closed_trade_count        UInt32,
    winning_trade_count       UInt32,
    losing_trade_count        UInt32,
    breakeven_trade_count     UInt32,
    win_rate_closed_trades    Nullable(Float64),
    average_win_return        Nullable(Float64),
    average_loss_return       Nullable(Float64),
    profit_loss_ratio         Nullable(Float64),
    average_holding_days      Nullable(Float64),
    largest_win_return        Nullable(Float64),
    largest_loss_return       Nullable(Float64),
    computed_at               DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (portfolio_run_id, result_attempt_id, window_key)
```

新增 dbt wrapper / mart 模型：

- `int_portfolio_closed_trade`
- `int_portfolio_trade_metric`
- `mart_portfolio_trade_metric_rank`

测试策略：

- FIFO 配对单测覆盖全平、部分平仓、多 lot 平仓、费用分摊。
- open-lot 状态单测覆盖同一 lot 多次部分平仓，确保多条 closed row 的 `closed_trade_id` 唯一且 `closed_trade_seq` 单调。
- `closed_trade_count = winning_trade_count + losing_trade_count + breakeven_trade_count`。
- `calc_portfolio_closed_trade.realized_pnl` 汇总应能与卖出成交造成的已实现现金变化对账。

### 阶段 7：Rearview API 与 Racingline 最小接入

**目标**：让指标和 closed ledger 可被应用查询，但不把前端展示作为本计划的核心复杂项。

**API**：

- `GET /portfolios/{portfolio_run_id}/performance?result_attempt_id=...`
- `GET /portfolios/{portfolio_run_id}/closed-trades?result_attempt_id=...`
- `GET /portfolios/{portfolio_run_id}/trade-metrics?result_attempt_id=...`

约束：

- 默认使用 PostgreSQL `current_result_attempt_id`。
- API 查询 ClickHouse 时必须带 `portfolio_run_id` 和 `result_attempt_id`。
- 若 row-level 或 metric-level `metric_status != succeeded`，API 返回状态和 NULL 指标原因，不伪造 0。
- Racingline type 定义随 API contract 更新；页面展示可后续单独优化。

## 验证命令

### dbt

```bash
cd pipeline
uv run dbt list --project-dir elt --profiles-dir elt --select int_risk_free_rate_daily mart_risk_free_rate_daily mart_benchmark_returns_daily --resource-type model --output name
uv run dbt build --project-dir elt --profiles-dir elt --select int_risk_free_rate_daily mart_risk_free_rate_daily mart_benchmark_returns_daily --quiet --warn-error-options '{"error":["NoNodesForSelectionCriteria"]}'
uv run dbt show --project-dir elt --profiles-dir elt --select int_risk_free_rate_daily --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_risk_free_rate_daily --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_benchmark_returns_daily --limit 10
```

指标消费和排名阶段追加：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select int_portfolio_performance_metric mart_portfolio_performance_metric_rank --quiet --warn-error-options '{"error":["NoNodesForSelectionCriteria"]}'
uv run dbt build --project-dir elt --profiles-dir elt --select int_portfolio_performance_metric_status --quiet --warn-error-options '{"error":["NoNodesForSelectionCriteria"]}'
```

交易级阶段追加：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select int_portfolio_closed_trade int_portfolio_trade_metric mart_portfolio_trade_metric_rank --quiet --warn-error-options '{"error":["NoNodesForSelectionCriteria"]}'
```

### PostgreSQL migration

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
uv run alembic -c migrate/alembic.ini -x target=rearview downgrade -1
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head
```

### Rust

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### Python / docs

```bash
cd pipeline
uv run ruff check migrate
uv run ruff format --check migrate
uv run dbt parse --project-dir elt --profiles-dir elt
```

```bash
make docs-check
git diff --check
```

### ClickHouse smoke

```sql
SHOW CREATE TABLE fleur_calculation.calc_portfolio_performance_metric;
SHOW CREATE TABLE fleur_calculation.calc_portfolio_performance_metric_status;
SHOW CREATE TABLE fleur_calculation.calc_portfolio_closed_trade;

EXPLAIN indexes = 1
SELECT *
FROM fleur_calculation.calc_portfolio_performance_metric
WHERE portfolio_run_id = '<run_id>'
  AND result_attempt_id = '<attempt_id>'
  AND security_code = '000300.SH'
  AND window_key = 'full_period';

SELECT metric_name, metric_status, reason_code
FROM fleur_calculation.calc_portfolio_performance_metric_status
WHERE portfolio_run_id = '<run_id>'
  AND result_attempt_id = '<attempt_id>'
  AND security_code = '000300.SH'
  AND window_key = 'full_period'
ORDER BY metric_name;
```

### 端到端

1. 构造或复用一个 source screening run。
2. 创建 portfolio run，默认 benchmark `000300.SH`，risk-free tenor `1y`。
3. worker 完成后确认：
   - PostgreSQL `portfolio_metric_config` 有当前 attempt 配置行。
   - ClickHouse 七张 0043 结果事实表仍有完整 attempt 数据。
   - ClickHouse `fleur_calculation.calc_portfolio_performance_metric` 有 `full_period` 指标行。
   - ClickHouse `fleur_calculation.calc_portfolio_performance_metric_status` 有 12 个 metric-level 状态行。
   - dbt `int_portfolio_performance_metric` / `int_portfolio_performance_metric_status` 可读取 worker 指标和状态，mart ranking 可用。
   - closed trade ledger 行数与卖出成交配对一致。

## 验收 Checklist

### dbt 输入

- [ ] `int_risk_free_rate_daily` 已创建，粒度为 `trade_date + source_tenor`。
- [ ] `int_risk_free_rate_daily` 承担期限选择、单位转换、交易日对齐、forward-fill 和日频折算。
- [ ] `mart_risk_free_rate_daily` 已创建，粒度为 `trade_date + source_tenor`。
- [ ] `mart_risk_free_rate_daily` 是 `int_risk_free_rate_daily` 的稳定薄封装，不重算转换逻辑。
- [ ] `annual_rate` 从百分比点正确转换为小数比例。
- [ ] `daily_rate` 使用配置中的 `compound` / `252` 口径。
- [ ] forward-fill 不使用未来值。
- [ ] `annualization_days`、`day_count_basis`、`daily_method`、`fill_strategy` 不作为 risk-free 日频表字段重复存储，只写入模型 meta / 设计文档 / metric config。
- [ ] `mart_benchmark_returns_daily` 沿用 `security_code`，不引入 `benchmark_key`。
- [ ] `mart_benchmark_returns_daily` 是 `int_benchmark_returns_daily` 的稳定薄封装，不重复维护 benchmark universe。
- [ ] intermediate / mart 三张表的 YAML 文档和 tests 完整。

### metric config

- [ ] Alembic `0006_create_portfolio_metric_config.py` 可 upgrade / downgrade。
- [ ] `portfolio_metric_config` 主键包含 run、attempt、benchmark、window。
- [ ] `config_hash` 对 canonical 字段序列稳定。
- [ ] worker 在 attempt 生成后写入 config。
- [ ] 不在 PostgreSQL 或 ClickHouse 另存与 typed columns 重复的 config payload。

### calculation metric

- [ ] `calc_portfolio_performance_metric` DDL 由 Rust worker 拥有并写入 `fleur_calculation`。
- [ ] `calc_portfolio_performance_metric_status` DDL 由 Rust worker 拥有并写入 `fleur_calculation`。
- [ ] `ORDER BY` 为 `(portfolio_run_id, result_attempt_id, security_code, window_key)`。
- [ ] worker 写入 12 个核心指标。
- [ ] worker 为每个 performance metric 宽表 row 写入 12 个 metric-level 状态行。
- [ ] worker 公式符合本计划“计算规则”，包括正数 MDD、样本标准差、样本下行波动率、Jensen alpha 和日频 beta。
- [ ] 样本不足、缺 benchmark / risk-free 时返回明确 row-level `metric_status`；零除等单指标失败写入 metric-level `metric_status` 和 `reason_code`。
- [ ] ClickHouse 写入仍是 append-only，不 mutation。

### dbt wrapper 和排名

- [ ] dbt source 声明 `fleur_calculation.calc_portfolio_performance_metric` 和 `calc_portfolio_performance_metric_status`。
- [ ] `int_portfolio_performance_metric` 是 thin wrapper，inner join `portfolio_run_snapshot` 只保留完整 attempt，不重算 12 个核心指标公式。
- [ ] `int_portfolio_performance_metric_status` 暴露 metric-level 状态和原因。
- [ ] ranking 只比较相同 `config_hash`、benchmark、window、metric 的完整 attempts。
- [ ] ranking 输出 long format，并按 ranking catalog 的 `rank_direction` 排序；`rank_direction = none` 的指标不产出 rank。

### closed trade ledger

- [ ] worker 输出 `fleur_calculation.calc_portfolio_closed_trade`。
- [ ] closed ledger 每行有稳定 `closed_trade_id` 和 attempt 内单调 `closed_trade_seq`。
- [ ] FIFO lot 配对单测覆盖部分平仓和多 lot 平仓。
- [ ] open-lot 状态单测覆盖同一 lot 多次部分平仓。
- [ ] `entry_gross_amount` / `exit_gross_amount` 不含费用，`realized_pnl`、费用分摊和 derived `realized_return` 计算有单测。
- [ ] 未闭仓 lot 不进入 closed ledger。
- [ ] 交易级指标不重复写入 benchmark-scoped performance row。

### API

- [ ] performance / closed trades / trade metrics API 默认查询 current attempt。
- [ ] API 查询 ClickHouse 时带 `portfolio_run_id` 和 `result_attempt_id`。
- [ ] row-level 或 metric-level `metric_status != succeeded` 时前端可区分 NULL 指标原因。
- [ ] Racingline 类型定义与 API contract 同步。

## 完成标准

1. `int_risk_free_rate_daily`、`mart_risk_free_rate_daily` 和 `mart_benchmark_returns_daily` 可由 dbt 定向 build，并通过 tests。
2. 每个成功 portfolio attempt 都有一组 `portfolio_metric_config`，并在 `fleur_calculation.calc_portfolio_performance_metric` 指标行中引用同一 `config_hash`。
3. worker 为 `full_period` 默认 benchmark 权威写入 12 个核心绩效指标和对应 metric-level 状态。
4. dbt 不复算指标公式；`int_portfolio_performance_metric` / `int_portfolio_performance_metric_status` thin wrapper 和跨 run ranking 可用。
5. closed trade ledger 能从 worker 账本输出到 `fleur_calculation.calc_portfolio_closed_trade`，交易级指标由 worker 写入 `calc_portfolio_trade_metric`。
6. NAV 递推仍只在 Rust，dbt / ClickHouse SQL 不重写账本状态机。
