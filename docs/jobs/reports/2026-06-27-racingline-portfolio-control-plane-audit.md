# Racingline Portfolio Control Plane Audit

日期：2026-06-27

范围：`strategy_backtest_run`、`strategy_backtest_task_outbox`、`strategy_portfolio`、`strategy_portfolio_daily_run`、`strategy_portfolio_daily_task_outbox` 以及 Step 5 publish/dashboard/detail resolver。

## 验证命令

```bash
rg -n "strategy_portfolio|current_live_result_attempt_id|pending_buy_signal_snapshot|initial_signal_date" engines/crates/rearview-core/src pipeline/migrate/versions/rearview
uv run alembic -c alembic.ini heads
```

## 结论

本轮不新增 `strategy_portfolio_publish_context` 或 `strategy_portfolio_live_state` 表。现有 PostgreSQL 控制面已经按 backtest run、portfolio publish record、portfolio daily run 和 outbox 分表；混用风险集中在 `strategy_portfolio` 缺少不可变 T 日信号上下文，以及组合级 `current_result_attempt_id` 与 live daily run result attempt 的语义不够明确。

采用小范围受控字段迁移：

| 决策 | 结果 |
|---|---|
| 发布上下文 | 在 `strategy_portfolio` 增加 `initial_signal_date` 和 `pending_buy_signal_snapshot`。 |
| live current attempt | 在 `strategy_portfolio` 增加 `current_live_result_attempt_id`，daily run 成功后只更新该字段。 |
| 旧 `current_result_attempt_id` | 保留为历史兼容字段，本轮新 live finalization 不再写它。 |
| 单独 publish/live state 表 | 暂不新增；等字段职责继续扩大或 outbox/attempt registry 需要独立生命周期时再拆。 |

对应 migration：`pipeline/migrate/versions/rearview/0009_strategy_portfolio_publish_context.py`。

## 字段归属矩阵

| 表 | 字段/字段组 | 归属 | 读写路径 |
|---|---|---|---|
| `strategy_backtest_run` | rule/execution/required metrics/marts/catalog | 回测配置事实 | Step 5 创建 backtest 写入；publish preview 和 publish portfolio 只读。 |
| `strategy_backtest_run` | period/range/benchmark/price_basis | 回测区间和发布依据 | publish preview 读取 `end_date` 作为 `source_signal_date`，创建 portfolio 复制为 source segment。 |
| `strategy_backtest_run` | status/progress/summary/signal_summary/data_coverage/error | backtest worker 状态 | backtest worker 更新；dashboard/detail 不把它作为 live 结果。 |
| `strategy_backtest_run` | `current_result_attempt_id` | backtest result attempt | backtest worker finalization 写入；backtest API 读 backtest ClickHouse family。 |
| `strategy_backtest_task_outbox` | payload/dispatch/subject/retry | backtest 调度 | backtest create 入队；runner 发布；不服务 portfolio live。 |
| `strategy_portfolio` | source backtest ids/period/range/benchmark | 发布依据 | publish portfolio 从 source run 固化；dashboard 作为 `backtest_segment` 暴露。 |
| `strategy_portfolio` | rule/execution/required metrics/marts/ui snapshot | 组合规则快照和 UI 展示 | publish portfolio 复制；daily worker 只读规则运行 live simulation。 |
| `strategy_portfolio` | `initial_signal_date` | 发布 T 日信号种子 | publish portfolio 写入；daily run 创建用作 `run_start_date`。 |
| `strategy_portfolio` | `live_start_date` | 正式 live 建仓日 | publish preview 后端解析 T+1；daily run eligibility 使用该字段。 |
| `strategy_portfolio` | `pending_buy_signal_snapshot` | pending 首次运行前的待调入信号快照 | publish portfolio 写入；dashboard/signals/signal-timeline pending 读取。 |
| `strategy_portfolio` | `latest_daily_run_id` | live tracking 当前 daily run 指针 | daily run finalization 写入；live resolver 读取。 |
| `strategy_portfolio` | `current_live_result_attempt_id` | live result attempt 指针 | daily run finalization 写入；live resolver 读取。 |
| `strategy_portfolio` | client_request/request_hash | publish 幂等 | create portfolio 写入和冲突校验；request hash 包含 expected T/T+1。 |
| `strategy_portfolio_daily_run` | run_start_date/trade_date/status/progress/error | live daily worker 状态 | daily run batch 创建；worker claim/finalize/fail 更新。 |
| `strategy_portfolio_daily_run` | summary/signal_summary/data_coverage | live run 运行摘要 | worker 写入；dashboard/detail 可展示 live 摘要。 |
| `strategy_portfolio_daily_run` | `current_result_attempt_id` | 单次 daily run result attempt | daily run finalization 写入；组合级 pointer 使用 `current_live_result_attempt_id`。 |
| `strategy_portfolio_daily_task_outbox` | payload/dispatch/subject/retry | live daily 调度 | daily run batch 入队；runner 发布；不服务 source backtest。 |

## 读写调用链

| 流程 | 调用链 | 结论 |
|---|---|---|
| 创建 backtest | API create -> `create_strategy_backtest` -> `strategy_backtest_task_outbox` -> runner -> backtest worker -> backtest ClickHouse family | backtest 控制面和 outbox 独立。 |
| 发布预检 | `GET /strategy-backtests/{id}/portfolio-publish-preview` -> source run current attempt 校验 -> trade calendar T+1 -> 单日 TopN signal compile | 不写 PostgreSQL/ClickHouse，不读 source latest target。 |
| 创建 portfolio | `POST /strategy-portfolios` -> 复用 publish preview -> 校验 expected T/T+1 -> `create_strategy_portfolio` | 固化 source segment、T/T+1 和 pending snapshot；不复制 source 持仓/现金/订单/绩效。 |
| 创建 daily run | API/调度 -> `create_strategy_portfolio_daily_runs_for_trade_date` | eligibility 使用 `live_start_date <= trade_date`，插入 `run_start_date = initial_signal_date`。 |
| daily run finalization | worker -> live ClickHouse writer -> `finalize_strategy_portfolio_daily_run_to_clickhouse` | 更新 daily run `current_result_attempt_id`、portfolio `latest_daily_run_id` 和 `current_live_result_attempt_id`。 |
| dashboard pending | `get_strategy_portfolio_dashboard` | pending 返回空 live metrics/curve/nav 和 `pending_buy_signals`，不回退 source backtest。 |
| detail live endpoints | nav/performance/positions/rebalance -> live resolver | pending 返回 `409 portfolio_pending_first_run`；signals/timeline 返回 publish snapshot。 |

## 数据迁移与风险

`0009_strategy_portfolio_publish_context` 对已有 portfolio 行执行：

- `initial_signal_date = source_end_date`，用于保存历史行可解释的 T 日种子。
- `pending_buy_signal_snapshot = []`，避免为历史 pending 行伪造信号。
- 不静默修正 `live_start_date <= initial_signal_date` 的历史行；这类数据需要上线前按业务事实人工审查。

上线前建议执行的只读审计 SQL：

```sql
select strategy_portfolio_id, source_end_date, live_start_date
from strategy_portfolio
where live_start_date <= source_end_date;

select strategy_portfolio_id, source_end_date, live_start_date
from strategy_portfolio
where latest_daily_run_id is null;
```

如果存在旧 pending 行且 `pending_buy_signal_snapshot = []`，UI 只能展示空待调入信号；禁止用 source backtest target 补假信号。

## 后续拆分触发条件

后续若出现以下任一情况，再新增独立表：

| 触发条件 | 候选拆分 |
|---|---|
| 发布上下文需要版本化、撤销或多次确认 | `strategy_portfolio_publish_context` |
| live tracking 状态需要独立锁、重试或多状态机 | `strategy_portfolio_live_state` |
| result attempt 需要跨 daily run 注册和审计 | `strategy_portfolio_live_attempt` |
| outbox payload 需要多版本合同 | 收窄 backtest/live outbox payload 表或增加 payload schema version |
