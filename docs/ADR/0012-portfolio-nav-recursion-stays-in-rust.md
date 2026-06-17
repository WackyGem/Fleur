# ADR 0012: 组合净值递推留在 Rust，指标复算上 ClickHouse mart

状态：Accepted

日期：2026-06-17

## 背景

Q&A 0001 决定把组合结果事实从 PostgreSQL 迁移到 ClickHouse portfolio data plane，PostgreSQL 只保留 control plane。Q&A 0001 待决问题 #5 进一步追问：绩效指标由 worker 计算为主，还是 dbt / ClickHouse 模型复算为主。

该问题容易和一个更模糊的设想混在一起：是否把净值（NAV）计算本身的职能边界迁移到 ClickHouse，用 ClickHouse SQL 递推 NAV、`daily_return` 和 `drawdown`，替代 Rust worker 内存计算。

当前实现位于 `engines/crates/rearview-core/src/portfolio/mod.rs`（约 1040 行），是"目标 → 订单 → 成交 → 每日持仓 → 现金 → NAV → `daily_return` → `drawdown` running max"的逐日状态机递推，并配有确定性单测（如 `summary_max_drawdown_uses_lowest_nav_drawdown`）。worker 当前从 ClickHouse 读行情 / 信号 / 交易日历，从 PostgreSQL 读运行配置，把结果写回 PostgreSQL。

如果把递推搬到 ClickHouse SQL，需要在窗口函数里跨多表复刻账本状态机和跨日依赖，失去 Rust 单测覆盖，也与 Furnace 边界（指标递推公式只在 `furnace-core`，不在 dbt / ClickHouse SQL 重写）精神冲突。

## 决策

净值递推计算保留在 Rust worker，不迁移到 ClickHouse SQL。组合净值**结果事实的存储**按 Q&A 0001 迁入 ClickHouse；组合净值**递推计算**留在 Rust 内存。两者分开裁决。

职责分层如下：

| 职责 | 承载方 | 说明 |
|---|---|---|
| 账本、NAV、`daily_return`、`drawdown` 递推 | Rust `rearview-core` worker | 有状态逐日递推，确定性单测覆盖 |
| 净值 / 持仓 / 成交结果事实存储 | ClickHouse `portfolio_nav_daily` 等 | 按 Q&A 0001 迁移，append-only + `result_attempt_id` |
| 绩效指标（Sharpe / Sortino / Alpha / Beta / 信息比率 / 特雷诺 / 波动率） | ClickHouse / dbt mart 复算 + worker 初算 | 无状态批量聚合，适合 OLAP |
| risk-free 折算、benchmark 累计、日期对齐 | dbt mart 层 | `mart_risk_free_rate_daily` 等，worker 只读现成结果 |
| 跨 run 排名、批量回测对比、窗口分析 | ClickHouse / dbt mart | Q&A 0001 的 OLAP 价值兑现点 |

具体约束：

- Rust worker 继续在内存中构造目标、订单、成交、每日持仓、净值曲线和绩效指标，批量写入 ClickHouse portfolio fact tables，再回写 PostgreSQL 终态和 `current_result_attempt_id`。
- 绩效指标允许由 worker 先写一版进 `portfolio_performance_metric`，再由 dbt mart 做复算、校验和跨 run 批量排名；mart 复算不替代 worker 的权威 NAV 曲线。
- `daily_return` 和 `drawdown` 的递推公式不得在 dbt 模型或 ClickHouse SQL 中重写；mart 只能在已有 `portfolio_nav_daily` 事实之上做聚合、对齐和指标派生。
- 净值递推的输入对齐（benchmark return、risk-free rate、交易日历）由 mart 层准备成现成字段，worker 不在内存里做期限选择或日频折算口径裁决。

## 依据

- Q&A 0001 Worker 角色第 3 步明确"在 Rust 内存中构造目标、订单、成交、每日持仓、净值曲线和绩效指标"。
- 净值是顺序状态机，不是聚合：`daily_return = total_equity / prev_total_equity - 1`、`drawdown` 依赖 running max，跨日依赖强。
- ClickHouse 擅长 Q&A 0001 §"ClickHouse 规则依据"列出的批量扫描 / 聚合 / 排序，不擅长跨多表有序事务模拟。
- 与 Furnace 边界一致：递推公式只在 core crate，不在 SQL 重写，保留单测覆盖与可重放性。
- 绩效指标本质是无状态批量聚合（`stddevSamp`、`covarSamp`、`product`），适合 mart 复算。

## 后果

- worker 仍是净值权威计算层，迁移 ClickHouse 不改变其递推职责，只改变输出目标库。
- mart 层获得明确的复算 / 分析职责，Q&A 0001 待决问题 #5 由此收敛为分层方案。
- 任何想把 NAV 递推进 ClickHouse SQL 的后续提议都需要先推翻本 ADR。
- 需要新增 `mart_risk_free_rate_daily`、benchmark 累计等 mart 模型，作为 worker 输入对齐的稳定入口。
- worker 写入 ClickHouse 后，PostgreSQL 结果事实表逐步标记 deprecated（见 Q&A 0001 迁移含义）。

## 关联

- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](../Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [Q&A 0002: Portfolio Metrics 基础数据缺口](../Q&A/0002-portfolio-metrics.md)
- [ADR 0009: ClickHouse 按 dbt 建模层分库](0009-clickhouse-layered-databases.md)
- [RFC 0021: Racingline 虚拟账户与组合调仓净值](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md)
- [engines/crates/rearview-core/src/portfolio/mod.rs](../../engines/crates/rearview-core/src/portfolio/mod.rs)
