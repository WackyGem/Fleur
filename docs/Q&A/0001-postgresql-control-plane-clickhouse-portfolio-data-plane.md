# Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane

状态：Proposed

日期：2026-06-16

## 问题

当前组合运行已经把 `portfolio_run`、`portfolio_nav`、`portfolio_position_day`、订单、成交和事件写入 PostgreSQL。随着后续需要分析组合有效性，例如夏普比率、索提诺比率、alpha、beta、跨组合排名和批量回测对比，组合结果事实是否仍应由 PostgreSQL 承载？

## 结论

长期方向应调整为：

- PostgreSQL 不再承载组合结果事实，只保留 control plane。
- ClickHouse 承载 backtest / portfolio data plane。
- Rust `rearview-portfolio-worker` 从 PostgreSQL 读取不可变运行配置，从 ClickHouse 读取行情、信号、基准和无风险利率等分析输入，计算组合账本、净值和绩效指标，再把结果批量写入 ClickHouse。
- PostgreSQL 只回写任务状态、错误、当前有效结果指针和必要 summary，Racingline 仍通过 Rearview API 访问结果，不直接访问 PostgreSQL 或 ClickHouse。

这个方向会替代 RFC 0021 第一版中“PostgreSQL 保存组合结果，ClickHouse 只作为行情和指标输入”的临时边界。RFC 0021 的第一版实现仍可作为过渡阶段，但不能作为长期组合分析架构。

## 背景

RFC 0021 第一版为了降低实现复杂度，把组合运行状态和组合结果都落在 PostgreSQL：

- `portfolio_run`
- `portfolio_target`
- `portfolio_order`
- `portfolio_trade`
- `portfolio_position_day`
- `portfolio_nav`
- `portfolio_event`

该方案便于 UI 查询、审计和幂等重算，但它把两类不同职责混在同一个 OLTP 数据库中：

| 职责 | 更合适的系统 |
|---|---|
| 运行状态、任务分发、参数快照、错误和当前有效结果指针 | PostgreSQL |
| 大量时间序列净值、每日持仓、成交事实、跨组合分析和批量绩效指标 | ClickHouse |

当组合结果用于策略研究和回测分析时，主要访问模式是 OLAP：按 run、日期、策略、基准和指标窗口批量扫描、聚合、排序和对比。ClickHouse 更适合作为这部分事实的存储和计算层。

## 推荐边界

### PostgreSQL Control Plane

PostgreSQL 保留：

- `portfolio_run`：运行 ID、source run、状态、起止日期、账户快照、执行快照。
- `portfolio_task_outbox`：HTTP 创建运行和 NATS 发布之间的可恢复 outbox。
- 任务状态：`created`、`queued`、`calculating`、`succeeded`、`failed_*`、`cancelled`。
- `current_result_attempt_id` 或等价字段：指向 ClickHouse 中当前有效结果。
- 错误类型、错误消息、完成时间和轻量 summary。

PostgreSQL 不应长期保存每日净值、每日持仓、订单、成交、目标和事件等组合结果事实。

### ClickHouse Portfolio Data Plane

ClickHouse 新增或预留 portfolio / backtest 分析库，建议以 `fleur_portfolio`、`fleur_backtest` 或 `rearview_analytics` 命名。核心事实表包括：

| 表 | 粒度 | 用途 |
|---|---|---|
| `portfolio_run_snapshot` | 每次组合运行或结果 attempt 一行 | 分析维表，保存 run 元数据和不可变快照摘要 |
| `portfolio_nav_daily` | 每 `portfolio_run_id`、`trade_date` 一行 | 净值、日收益、回撤、现金、持仓市值 |
| `portfolio_position_day` | 每 `portfolio_run_id`、`trade_date`、`security_code` 一行 | 每日持仓、成本、市值、浮盈亏 |
| `portfolio_trade` | 每笔虚拟成交一行 | 成交、费用、滑点和成交原因 |
| `portfolio_order` | 每个虚拟订单一行 | 目标到成交之间的审计链路 |
| `portfolio_target` | 每个目标持仓一行 | 信号、rank、score、目标权重和目标金额 |
| `portfolio_event` | 每个 warning / event 一行 | 价格缺失、现金不足、止损触发等事件 |
| `portfolio_performance_metric` | 每 run、benchmark、window 一行 | 夏普、索提诺、alpha、beta、年化收益、最大回撤等 |

## Worker 角色

Rust worker 的长期角色应从“组合净值 worker”升级为“回测/组合计算引擎”：

1. 从 PostgreSQL 读取 `portfolio_run` 的不可变账户和执行参数快照。
2. 从 ClickHouse 读取 source run 买入信号、后复权行情、交易日历、benchmark return 和 risk-free rate。
3. 在 Rust 内存中构造目标、订单、成交、每日持仓、净值曲线和绩效指标。
4. 批量写入 ClickHouse portfolio fact tables。
5. 回写 PostgreSQL run 终态和当前有效结果指针。
6. 只有 PostgreSQL 终态写入成功后 ack NATS message。

worker 不应在浏览器请求路径中同步执行长时间回测，也不应让 Racingline 直接查询 ClickHouse。

## 幂等和重算

推荐引入 `result_attempt_id` 或 `run_attempt`：

- 每次重算生成新的 attempt。
- ClickHouse 结果事实都带 `portfolio_run_id` 和 `result_attempt_id`。
- PostgreSQL `portfolio_run.current_result_attempt_id` 指向当前有效 attempt。
- 旧 attempt 保留为审计和对比材料，必要时再按生命周期策略清理。

这种 append-only attempt 模型优先于频繁覆盖同一主键。若需要同一主键替换语义，可使用 ClickHouse `ReplacingMergeTree(version, is_deleted)`，但查询层要明确是否需要 latest-state view 或 `argMax` 聚合，避免大量依赖 `FINAL`。

## ClickHouse 建模原则

### 净值事实表

推荐查询模式是按 `portfolio_run_id` 拉取净值曲线，或按日期窗口批量扫描多个 run：

```sql
ENGINE = ReplacingMergeTree(result_version, is_deleted)
PARTITION BY toYYYYMM(trade_date)
ORDER BY (portfolio_run_id, trade_date)
```

### 持仓事实表

推荐主查询模式是按 run 和日期查看持仓：

```sql
ENGINE = ReplacingMergeTree(result_version, is_deleted)
PARTITION BY toYYYYMM(trade_date)
ORDER BY (portfolio_run_id, trade_date, security_code)
```

如果后续经常按证券反查“哪些组合持有过该证券”，应新增投影表或第二事实表，按 `(security_code, trade_date, portfolio_run_id)` 排序，而不是让单表同时优化所有访问模式。

### 绩效指标表

夏普、索提诺、alpha 和 beta 可以由 worker 计算后写入，也可以由 ClickHouse / dbt 基于 `portfolio_nav_daily` 批量复算。长期建议两层并存：

- worker 写 run 级权威 summary，便于 API 快速返回。
- ClickHouse / dbt 保留可复算模型，便于研究、口径变更和跨策略分析。

指标口径必须显式固定：

| 指标口径 | 待固定内容 |
|---|---|
| 年化天数 | A 股默认可用 `252`，但应写入配置或计算 metadata |
| 收益率 | 简单收益或 log return |
| 无风险利率 | 日频序列或固定年化值折算 |
| benchmark | 沪深 300、中证 500、全 A 或策略指定 benchmark |
| alpha | CAPM alpha 或相对 benchmark 的年化超额收益 |
| 日期对齐 | 组合交易日、benchmark 交易日交集，或明确补齐规则 |

## ClickHouse 规则依据

Workload shape：market data / financial services，混合时间序列 OLAP 和按 run 点查。

| 规则 | 应用 |
|---|---|
| `insert-batch-size` | worker 写入 ClickHouse 时应批量写入，目标 10K-100K 行，避免单行或小批量写入导致 part 压力 |
| `insert-mutation-avoid-update` | 重算结果不使用频繁 `ALTER UPDATE`；优先 attempt append-only，其次 `ReplacingMergeTree` |
| `schema-partition-low-cardinality` | portfolio 时间序列表按月分区，不按 `portfolio_run_id` 等高基数字段分区 |
| `schema-pk-prioritize-filters` | `ORDER BY` 必须服务主要查询过滤列；净值曲线优先 `(portfolio_run_id, trade_date)` |
| `decision-late-arriving-upserts` | 如需要 replacement semantics，使用带版本列的 `ReplacingMergeTree` |
| `decision-partitioning-timeseries` | 时间序列组合结果优先月分区，除非数据量和保留策略证明无需分区或需要其他粒度 |

官方 ClickHouse 文档支持 `ReplacingMergeTree` 版本列、时间序列分区、批量写入和物化视图 rollup；本 Q&A 中的具体表边界属于基于 mono-fleur workload 的 derived 设计。

## 迁移含义

短期可保留当前 PostgreSQL 结果表作为兼容层，但下一阶段应避免继续扩大 PostgreSQL 中的组合结果事实：

1. 新增 ClickHouse portfolio / backtest database 和结果事实表。
2. worker 改为写 ClickHouse 结果事实，再回写 PostgreSQL 状态和 `current_result_attempt_id`。
3. Rearview API 查询组合结果时从 ClickHouse 读取，PostgreSQL 只用于 run 状态和权限/控制信息。
4. Racingline API contract 尽量保持稳定，避免前端感知底层存储迁移。
5. 旧 PostgreSQL `portfolio_nav` / `portfolio_position_day` 可作为过渡回填来源，迁移完成后标记 deprecated。

## 待决问题

1. ClickHouse 结果库命名：`fleur_portfolio`、`fleur_backtest` 还是 `rearview_analytics`。
2. 是否采用 `result_attempt_id` 作为所有结果事实主维度，还是用 `ReplacingMergeTree` 覆盖同一 `portfolio_run_id`。
3. benchmark 和 risk-free rate 的来源表、更新频率和缺失值处理。
4. 绩效指标由 worker 计算为主，还是 dbt / ClickHouse 模型复算为主。
5. 是否需要新增独立 `backtester` crate，或继续扩展 `rearview-portfolio-worker`。
6. 当前 RFC 0021 和 Plan 0041 中 PostgreSQL 结果事实边界何时正式标记为 superseded。

## 相关文档

- [RFC 0021: Racingline 虚拟账户与组合调仓净值](../RFC/0021-racingline-virtual-account-portfolio-rebalancing.md)
- [Plan 0041: Racingline 虚拟账户与组合调仓净值实施计划](../plans/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md)
- [System: Rearview](../systems/rearview.md)
- [System: Data Platform](../systems/data-platform.md)
- ClickHouse docs: ReplacingMergeTree, partitions, insert strategy and materialized views.
