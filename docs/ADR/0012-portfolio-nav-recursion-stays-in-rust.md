# ADR 0012: 组合净值递推与绩效指标权威计算留在 Rust

状态：Accepted

日期：2026-06-17

## 背景

Q&A 0001 决定把组合结果事实从 PostgreSQL 迁移到 ClickHouse portfolio data plane，PostgreSQL 只保留 control plane。Q&A 0001 待决问题 #5 进一步追问：绩效指标由 worker 计算为主，还是 dbt / ClickHouse 模型复算为主。

该问题容易和一个更模糊的设想混在一起：是否把净值（NAV）计算本身的职能边界迁移到 ClickHouse，用 ClickHouse SQL 递推 NAV、`daily_return` 和 `drawdown`，替代 Rust worker 内存计算。

当前实现位于 `engines/crates/rearview-core/src/portfolio/mod.rs`（约 1040 行），是"目标 → 订单 → 成交 → 每日持仓 → 现金 → NAV → `daily_return` → `drawdown` running max"的逐日状态机递推，并配有确定性单测（如 `summary_max_drawdown_uses_lowest_nav_drawdown`）。worker 当前从 ClickHouse 读行情 / 信号 / 交易日历，从 PostgreSQL 读运行配置，把结果写回 PostgreSQL。

如果把递推搬到 ClickHouse SQL，需要在窗口函数里跨多表复刻账本状态机和跨日依赖，失去 Rust 单测覆盖，也与 Furnace 边界（指标递推公式只在 `furnace-core`，不在 dbt / ClickHouse SQL 重写）精神冲突。

## 决策

净值递推计算和绩效指标权威计算都保留在 Rust worker，不迁移到 ClickHouse SQL 或 dbt SQL。组合净值**结果事实的存储**按 Q&A 0001 迁入 ClickHouse `fleur_portfolio`；组合绩效指标、closed trade ledger 和交易级指标作为 Rust 外部计算产物写入 `fleur_calculation`，再由 dbt 通过 source / thin wrapper / mart ranking 消费。存储位置和计算职责分开裁决。

职责分层如下：

| 职责 | 承载方 | 说明 |
|---|---|---|
| 账本、NAV、`daily_return`、`drawdown` 递推 | Rust `rearview-core` worker | 有状态逐日递推，确定性单测覆盖 |
| 净值 / 持仓 / 成交结果事实存储 | ClickHouse `fleur_portfolio.*` | 按 Q&A 0001 迁移，append-only + `result_attempt_id` |
| 绩效指标（Sharpe / Sortino / Alpha / Beta / 信息比率 / 特雷诺 / 波动率） | Rust worker + ClickHouse `fleur_calculation.*` | worker 权威计算，dbt 不复算公式 |
| risk-free 折算、benchmark 日频入口 | dbt intermediate + marts | worker 只读现成日频序列 |
| 跨 run 排名、批量回测对比、窗口分析 | dbt thin wrapper + marts | 消费 `fleur_calculation` 产物，不重写公式 |

具体约束：

- Rust worker 继续在内存中构造目标、订单、成交、每日持仓、净值曲线和绩效指标，批量写入 ClickHouse，再回写 PostgreSQL 终态和 `current_result_attempt_id`。
- 净值、持仓、订单、成交、目标和事件进入 `fleur_portfolio`；绩效指标、closed trade ledger 和交易级指标进入 `fleur_calculation`。
- dbt 不复算 12 个核心绩效指标公式。dbt 负责声明 `fleur_calculation.*` source、提供 `fleur_intermediate.int_*` thin wrapper、做 contract/质量检查和跨 run ranking。
- `daily_return`、`drawdown` 和绩效指标公式不得在 dbt 模型或 ClickHouse SQL 中重写。
- 净值递推和指标计算的输入对齐（benchmark return、risk-free rate、交易日历）由 dbt intermediate/mart 层准备成现成字段，worker 不在内存里做原始期限选择或日频折算口径裁决。

## 依据

- Q&A 0001 Worker 角色第 3 步明确"在 Rust 内存中构造目标、订单、成交、每日持仓、净值曲线和绩效指标"。
- 净值是顺序状态机，不是聚合：`daily_return = total_equity / prev_total_equity - 1`、`drawdown` 依赖 running max，跨日依赖强。
- ClickHouse 擅长 Q&A 0001 §"ClickHouse 规则依据"列出的批量扫描 / 聚合 / 排序，不擅长跨多表有序事务模拟。
- 与 Furnace 边界一致：递推公式只在 core crate，不在 SQL 重写，保留单测覆盖与可重放性。
- ADR 0009 明确外部计算引擎直接写入的计算产物进入 `fleur_calculation`，再由 dbt source + thin wrapper + marts 消费。portfolio worker 的绩效指标和交易级指标属于这一类外部计算产物。
- 绩效指标虽是无状态聚合，但它们是 run attempt 结果的一部分，需与 `portfolio_metric_config`、`result_attempt_id` 和 worker 账本输出一起保持可审计一致；因此由 worker 权威计算，dbt 不做第二套公式实现。

## 后果

- worker 仍是净值和绩效指标权威计算层，迁移 ClickHouse 不改变其递推职责，只改变输出目标库。
- `fleur_calculation` 承接 worker 绩效指标和交易级计算产物；dbt marts 获得明确的消费、排名和分析职责。
- 任何想把 NAV 递推进 ClickHouse SQL 的后续提议都需要先推翻本 ADR。
- 任何想让 dbt 复算 12 个核心绩效指标公式的后续提议都需要先推翻本 ADR。
- 需要新增 `int_risk_free_rate_daily`、`mart_risk_free_rate_daily`、`mart_benchmark_returns_daily` 等模型，作为 worker 输入对齐的稳定入口。
- worker 写入 ClickHouse 后，PostgreSQL 结果事实表逐步标记 deprecated（见 Q&A 0001 迁移含义）。

## 关联

- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](../Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [Q&A 0002: Portfolio Metrics 基础数据缺口](../Q&A/0002-portfolio-metrics.md)
- [ADR 0009: ClickHouse 按 dbt 建模层分库](0009-clickhouse-layered-databases.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md)
- [engines/crates/rearview-core/src/portfolio/mod.rs](../../engines/crates/rearview-core/src/portfolio/mod.rs)
