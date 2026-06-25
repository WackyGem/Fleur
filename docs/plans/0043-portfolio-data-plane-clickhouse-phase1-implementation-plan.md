# Plan 0043: 组合数据面迁移 ClickHouse 第一阶段实施计划（结果事实迁移与 worker 切换）

日期：2026-06-17

状态：Proposed

关联文档：

- [RFC 0022: 组合数据面迁移 ClickHouse 与绩效指标分层](../RFC/0022-portfolio-data-plane-clickhouse-and-metrics.md)
- [Q&A 0001: PostgreSQL Control Plane 与 ClickHouse Portfolio Data Plane](../Q&A/0001-postgresql-control-plane-clickhouse-portfolio-data-plane.md)
- [ADR 0012: 组合净值递推留在 Rust，指标复算上 ClickHouse mart](../ADR/0012-portfolio-nav-recursion-stays-in-rust.md)
- [ADR 0009: ClickHouse 按 dbt 建模层分库](../ADR/0009-clickhouse-layered-databases.md)
- [Plan 0041: Racingline 虚拟账户与组合调仓净值实施计划](archive/0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md)

## 目标

1. 在 ClickHouse 新增 `fleur_portfolio` database 和七张组合结果事实表（不含绩效指标表）。
2. Rust worker 从"写 PostgreSQL 结果事实"切换为"写 ClickHouse 结果事实 + 回写 PostgreSQL 控制状态和 `current_result_attempt_id`"。
3. 引入 `result_attempt_id` 实现幂等重算，append-only，不做 `ALTER UPDATE` 覆盖。
4. PostgreSQL `portfolio_run` 新增 `current_result_attempt_id` 字段。
5. Rearview API 组合结果查询切换到从 ClickHouse 读取。

## 非目标

1. 不把净值递推搬到 ClickHouse SQL 或 dbt 模型（ADR 0012）；`daily_return` / `drawdown` 仍由 Rust 内存递推。
2. 不实现绩效指标计算（Sharpe / Sortino / Alpha / Beta 等）、`portfolio_performance_metric` 表、`portfolio_metric_config` 表——留到下一计划（Plan 0044）。
3. 不新增 dbt mart 层 `mart_risk_free_rate_daily` / `mart_benchmark_returns_daily`——留到 Plan 0044。
4. 不实现 closed trade ledger、胜率、盈亏比等交易级指标。
5. 不删除旧 PostgreSQL 结果事实表；仅停止写入，后续标记 deprecated。
6. 不做 dbt mart 复算校验和跨 run 批量排名。

## 当前事实基线

1. worker 唯一写入点是 `rearview-core/src/postgres/mod.rs:795` 的 `write_portfolio_results`，在一个 PG 事务内 delete + insert 六张表（`portfolio_target`、`portfolio_order`、`portfolio_trade`、`portfolio_position_day`、`portfolio_nav`、`portfolio_event`），事务末尾 `update portfolio_run set status = 'succeeded'`。
2. `process_run`（`rearview-portfolio-worker/src/main.rs:117`）流程：`build_simulation_input` → `simulate_portfolio` → `write_portfolio_results`。失败时走 `set_portfolio_run_status`。
3. `claim_portfolio_run_for_calculation` 把 `portfolio_run.status` 置为 `calculating_nav` 并返回 claimed run。
4. `ClickHouseClient`（`rearview-core/src/clickhouse/mod.rs`）当前只读，仅提供 `query_*` 方法，无写入能力。
5. PostgreSQL `portfolio_run` 无 `current_result_attempt_id` 字段；无 `result_attempt_id` 概念。
6. Rust `PortfolioSimulationOutput`（`portfolio/mod.rs:73`）含 `targets` / `orders` / `trades` / `positions` / `nav` / `events` / `summary` 七个字段，是 worker 写入的数据源。
7. Furnace 用 Rust 代码 `CREATE TABLE IF NOT EXISTS` 管理 ClickHouse DDL（`furnace-io/src/schema/tables.rs`），portfolio 结果事实表属 worker 写入，DDL 也应由 Rust 拥有，不进 dbt on-run-start。
8. dbt `on-run-start` 只创建 `fleur_staging` / `fleur_intermediate` / `fleur_marts`，不涉及 `fleur_portfolio`。

## 命名约定

- ClickHouse database：`fleur_portfolio`（与 ADR 0009 `fleur_*` 前缀一致，但 owner 是 Rust worker 而非 dbt）。
- ClickHouse 结果事实表：`portfolio_run_snapshot`、`portfolio_nav_daily`、`portfolio_position_day`、`portfolio_trade`、`portfolio_order`、`portfolio_target`、`portfolio_event`（表名沿用 `portfolio_*` 前缀，与 PostgreSQL 既有表同名语义对齐）。
- 所有结果事实行带 `portfolio_run_id` + `result_attempt_id`，append-only。
- `result_attempt_id` 使用 ULID（时间有序，避免 UUID 全随机导致 part 排序压力）。

## 实施阶段

### 阶段 1：ClickHouse DDL（Rust schema 模块）

**目标**：worker 启动时幂等创建 `fleur_portfolio` database 和七张表。

**实现**：

1. 新增 `rearview-core/src/clickhouse/portfolio_schema.rs`（参照 `furnace-io/src/schema/tables.rs` 模式），提供 `CREATE TABLE IF NOT EXISTS` SQL 生成函数。
2. `ClickHouseClient` 新增 `ensure_portfolio_schema(&self) -> RearviewResult<()>` 方法，启动时调用，幂等建库建表。
3. `rearview-portfolio-worker` 的 `main` 在连接 ClickHouse 后、消费消息前调用 `ensure_portfolio_schema`。

**DDL 规格**：

```sql
CREATE DATABASE IF NOT EXISTS fleur_portfolio
```

七张表统一规则：`MergeTree`，append-only，无 `ReplacingMergeTree`；时间序列表按 `toYYYYMM(trade_date)` 月分区；`ORDER BY` 以 `(portfolio_run_id, result_attempt_id, ...)` 开头。字段集对齐 `PortfolioSimulationOutput` 各 Row 结构 + `portfolio_run_id` + `result_attempt_id`。

| 表 | ORDER BY | 分区 | 字段来源 |
|---|---|---|---|
| `portfolio_run_snapshot` | `(portfolio_run_id, result_attempt_id)` | 无 | `PortfolioSummary` + run 元数据 |
| `portfolio_nav_daily` | `(portfolio_run_id, result_attempt_id, trade_date)` | `toYYYYMM(trade_date)` | `PortfolioNavRow` |
| `portfolio_position_day` | `(portfolio_run_id, result_attempt_id, trade_date, security_code)` | `toYYYYMM(trade_date)` | `PortfolioPositionDayRow` |
| `portfolio_trade` | `(portfolio_run_id, result_attempt_id, trade_date, security_code)` | `toYYYYMM(trade_date)` | `PortfolioTradeRow` |
| `portfolio_order` | `(portfolio_run_id, result_attempt_id, signal_date, security_code)` | `toYYYYMM(signal_date)` | `PortfolioOrderRow` |
| `portfolio_target` | `(portfolio_run_id, result_attempt_id, signal_date, security_code)` | `toYYYYMM(signal_date)` | `PortfolioTargetRow` |
| `portfolio_event` | `(portfolio_run_id, result_attempt_id, trade_date)` | `toYYYYMM(trade_date)` | `PortfolioEventRow` |

**字段映射要点**：

- `PortfolioNavRow`：`trade_date`、`cash_balance`、`position_market_value`、`total_equity`、`nav`、`daily_return`（`Nullable`）、`drawdown`、`gross_exposure`、`position_count`（`UInt32`）、`turnover`、`fee_amount`、`warning_count`（`UInt32`）。
- `PortfolioPositionDayRow`：`trade_date`、`security_code`、`quantity`、`cost_basis`、`average_entry_price`、`close_price`、`market_value`、`unrealized_pnl`、`unrealized_return`、`holding_days`（`UInt32`）、`is_stale_price`（`Bool`）。
- `PortfolioTradeRow`：`trade_seq`（`UInt32`）、`order_seq`（`UInt32`）、`trade_date`、`signal_date`（`Nullable(Date)`）、`security_code`、`side`（`String`）、`quantity`、`reference_price`、`execution_price`、`gross_amount`、`commission`、`stamp_duty`、`transfer_fee`、`total_fee`、`slippage_cost`、`reason`（`String`）。
- `PortfolioOrderRow`：新增 `portfolio_order_id`（`String`，worker 生成 UUID）；`order_seq`（`UInt32`）、`signal_date`（`Nullable(Date)`）、`execution_date`、`security_code`、`side`、`order_quantity`、`order_amount`、`reference_price`、`reason`、`status`（`String`）。
- `PortfolioTargetRow`：`signal_date`、`execution_date`、`security_code`、`source_rank`（`UInt32`）、`source_score`、`target_weight`、`target_amount`、`target_quantity`、`target_reason`（`String`）。
- `PortfolioEventRow`：新增 `portfolio_event_id`（`String`）；`event_seq`（`UInt32`）、`trade_date`、`security_code`（`Nullable(String)`）、`event_type`（`String`）、`message`（`String`）。
- `portfolio_run_snapshot`：`portfolio_run_id`、`result_attempt_id`、`source_run_id`、`rule_version_id`、`rule_hash`、`account_snapshot`（`String` JSON）、`execution_snapshot`（`String` JSON）、`start_date`、`end_date`、`summary`（`String` JSON，来自 `PortfolioSummary`）、`created_at`（`DateTime`）。
- 枚举字段（`side` / `reason` / `status` / `target_reason` / `event_type`）在 ClickHouse 用 `String`，由 worker 调用现有 `order_side_str` / `order_reason_str` / `order_status_str` / `target_reason_str` / `portfolio_event_type_str` 转字符串后写入，与 PostgreSQL 当前序列化口径一致。

### 阶段 2：PostgreSQL control plane 扩展

**目标**：`portfolio_run` 支持指向当前有效 attempt。

**实现**：

1. 新增 Alembic migration `pipeline/migrate/versions/rearview/0004_add_current_result_attempt.py`。
2. `portfolio_run` 新增列：

```python
sa.Column("current_result_attempt_id", sa.Text(), nullable=True)
```

3. 新增索引 `idx_portfolio_run_current_attempt`（`portfolio_run_id`, `current_result_attempt_id`），支持 API 按 attempt 点查。
4. 不新增 `portfolio_metric_config` 表（留到 Plan 0044）。
5. 不删除、不修改旧六张结果事实表的任何列。
6. migration 必须可回滚（`downgrade` drop 列和索引）。

### 阶段 3：ClickHouseClient 写入能力

**目标**：`ClickHouseClient` 支持批量写入七张结果事实表。

**实现**：

1. 在 `rearview-core/src/clickhouse/` 新增写入方法，复用现有 HTTP client（`reqwest`），使用 ClickHouse `INSERT ... FORMAT JSONEachRow` 批量写入。
2. 新增方法签名：

```rust
impl ClickHouseClient {
    pub async fn ensure_portfolio_schema(&self) -> RearviewResult<()>;
    pub async fn write_portfolio_results(
        &self,
        portfolio_run_id: &str,
        result_attempt_id: &str,
        output: &PortfolioSimulationOutput,
    ) -> RearviewResult<()>;
}
```

3. `write_portfolio_results` 内部按表分批 POST：每张表一次性写入该表全量行（单 run 行数通常 <100K，满足 Q&A 0001 批量写入要求）。若未来单表超 100K 行再分片。
4. 写入顺序：`portfolio_target` → `portfolio_order` → `portfolio_trade` → `portfolio_position_day` → `portfolio_nav_daily` → `portfolio_event` → `portfolio_run_snapshot`，最后写 snapshot 表示该 attempt 完整落库。
5. 每行注入 `portfolio_run_id` 和 `result_attempt_id`。
6. order_id / event_id 在写入前由 worker 生成（UUID），与现有 PG 写入逻辑一致。
7. 写入失败返回 `RearviewError::ClickHouse`，供 `portfolio_failure_status` 映射到 `failed_write`。
8. ClickHouse 无事务，幂等性由 `result_attempt_id` 保证：重算生成新 attempt，旧数据保留，不 delete。

### 阶段 4：worker 切换写入目标

**目标**：`process_run` 改为写 ClickHouse + 回写 PG 控制状态。

**实现**：

1. `process_run`（`rearview-portfolio-worker/src/main.rs:117`）改造为：

```rust
async fn process_run(
    postgres: &RearviewPg,
    clickhouse: &ClickHouseClient,
    run: &PortfolioRunRecord,
) -> RearviewResult<()> {
    let input = build_simulation_input(postgres, clickhouse, run).await?;
    let output = simulate_portfolio(&input)?;
    let result_attempt_id = ulid::Ulid::new().to_string();
    clickhouse
        .write_portfolio_results(&run.portfolio_run_id, &result_attempt_id, &output)
        .await?;
    postgres
        .finalize_portfolio_run_to_clickhouse(
            &run.portfolio_run_id,
            &result_attempt_id,
            &output.summary,
        )
        .await?;
    info!(
        portfolio_run_id = run.portfolio_run_id,
        result_attempt_id = result_attempt_id,
        nav_points = output.nav.len(),
        trades = output.trades.len(),
        "portfolio run succeeded"
    );
    Ok(())
}
```

2. 新增 `RearviewPg::finalize_portfolio_run_to_clickhouse`（替代 `write_portfolio_results` 的事务末尾部分），只做控制面回写：

```sql
update portfolio_run
set status = 'succeeded',
    current_result_attempt_id = $2,
    summary = $3::jsonb,
    error_type = null,
    error_message = null,
    completed_at = now(),
    updated_at = now()
where portfolio_run_id = $1
```

3. 旧的 `write_portfolio_results`（delete + insert 六表）废弃，不再被 `process_run` 调用。第一阶段保留函数代码不删（避免破坏测试），后续计划清理。
4. 失败路径不变：`portfolio_failure_status` 把 `RearviewError::ClickHouse` 映射为 `failed_market_data`；需确认写入失败应映射为 `failed_write` 而非 `failed_market_data`——调整 `portfolio_failure_status`，把 ClickHouse 写入错误与读取错误区分，或统一写失败为 `failed_write`。
5. ack 时机不变：只有 `finalize_portfolio_run_to_clickhouse` 成功后 ack NATS message（已在 main loop 中）。
6. 重算时 `claim_portfolio_run_for_calculation` 把 status 置回 `calculating_nav`，生成新 `result_attempt_id`，旧 attempt 在 ClickHouse 保留，PG `current_result_attempt_id` 更新为新值。

**`portfolio_failure_status` 调整**：

当前 `RearviewError::ClickHouse(_)` 统一映射为 `failed_market_data`。写入失败应区分。方案：在 `RearviewError` 中新增 `ClickHouseWrite` 变体，或保留单变体但在 `write_portfolio_results` 失败时显式返回 `failed_write`。第一阶段采用后者：`process_run` 捕获 ClickHouse 写入错误后调用 `set_portfolio_run_status(..., "failed_write", ...)`，不依赖 `portfolio_failure_status` 的自动映射。

### 阶段 5：Rearview API 切换读取源

**目标**：组合结果查询从 ClickHouse 读取。

**实现**：

1. `RearviewPg` 现有读取方法（`list_portfolio_nav`、`list_portfolio_trades`、`list_portfolio_positions` 等，`postgres/mod.rs:570+`）改为委托 ClickHouse 查询，带 `current_result_attempt_id` 过滤。
2. 新增 `ClickHouseClient` 读取方法：

```rust
pub async fn query_portfolio_nav(
    &self,
    portfolio_run_id: &str,
    result_attempt_id: &str,
) -> RearviewResult<Vec<PortfolioNavRecord>>;
// 同理 query_portfolio_trades / positions / orders / targets / events
```

3. `RearviewPg::get_portfolio_run` 返回的 `PortfolioRunRecord` 新增 `current_result_attempt_id` 字段，供读取层使用。
4. API 响应结构不变，Racingline 前端无感知。
5. 保留查询历史 attempt 的能力（可选 `result_attempt_id` 查询参数）。

## 验证命令

```bash
cd pipeline

# PostgreSQL migration 幂等
cd migrate
uv run alembic -c alembic.ini -x target=rearview upgrade head
uv run alembic -c alembic.ini -x target=rearview downgrade -1
uv run alembic -c alembic.ini -x target=rearview upgrade head

cd ..
# dbt 解析（确认未破坏现有模型）
uv run dbt parse --project-dir elt --profiles-dir elt
```

```bash
cd engines

# Rust 质量门禁
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

```bash
# 文档门禁
make docs-check
git diff --check
```

## 验收 Checklist

### ClickHouse DDL

- [ ] `fleur_portfolio` database 存在。
- [ ] 七张结果事实表已创建，`SHOW CREATE TABLE` 与设计一致。
- [ ] 所有时间序列表按 `toYYYYMM(trade_date)` / `toYYYYMM(signal_date)` 月分区。
- [ ] `ORDER BY` 以 `(portfolio_run_id, result_attempt_id, ...)` 开头。
- [ ] 无 `ReplacingMergeTree`，全部 `MergeTree` append-only。
- [ ] `ensure_portfolio_schema` 幂等（重复执行不报错）。
- [ ] worker 启动时自动调用 `ensure_portfolio_schema`。

### PostgreSQL control plane

- [ ] `portfolio_run.current_result_attempt_id` 字段已新增，`nullable`。
- [ ] `idx_portfolio_run_current_attempt` 索引已创建。
- [ ] `alembic upgrade head`（target=rearview）成功且幂等。
- [ ] `alembic downgrade -1` + `upgrade head` 往返成功。
- [ ] 旧六张结果事实表未被删除，未新增列。
- [ ] 未新增 `portfolio_metric_config` / `portfolio_performance_metric` 表（留 Plan 0044）。

### ClickHouseClient 写入

- [ ] `write_portfolio_results` 按 ORDER BY 顺序写入七张表。
- [ ] 每行带 `portfolio_run_id` 和 `result_attempt_id`。
- [ ] 单表全量一次写入（单 run <100K 行不分片）。
- [ ] `portfolio_run_snapshot` 最后写入，标记 attempt 完整。
- [ ] 写入失败返回 `RearviewError::ClickHouse`。
- [ ] Rust 单测覆盖写入 payload 序列化（JSONEachRow 字段与 DDL 一致）。

### worker 切换

- [ ] `process_run` 生成 ULID `result_attempt_id` 并写入 ClickHouse 七表。
- [ ] `process_run` 不再调用旧 `write_portfolio_results`（delete + insert PG 六表）。
- [ ] `finalize_portfolio_run_to_clickhouse` 回写 `status='succeeded'` + `current_result_attempt_id` + `summary`。
- [ ] ClickHouse 写入失败时回写 `failed_write`（不误判为 `failed_market_data`）。
- [ ] NATS message 仅在 PG 终态写入成功后 ack。
- [ ] 重算生成新 `result_attempt_id`，旧 attempt 保留，PG 指针更新。
- [ ] `cargo test --workspace` 通过。
- [ ] 端到端：一个完整 run 的结果全部落在 `fleur_portfolio.*`，PostgreSQL 只剩控制状态。

### Rearview API 切换

- [ ] `list_portfolio_nav` 等读取方法从 ClickHouse 读取，带 `current_result_attempt_id` 过滤。
- [ ] `PortfolioRunRecord` 含 `current_result_attempt_id` 字段。
- [ ] API 响应结构与切换前一致，Racingline 前端无感知。
- [ ] 可选查询历史 `result_attempt_id` 的能力可用。

### 边界合规

- [ ] ClickHouse SQL / dbt 模型中无 NAV 递推公式（`daily_return` / `drawdown` 重算）—— ADR 0012。
- [ ] `fleur_portfolio` DDL 由 Rust 拥有，不进 dbt on-run-start。
- [ ] 旧 PG 结果事实表仅停止写入，未删除。
- [ ] 本计划不涉及绩效指标、metric_config、mart risk-free/benchmark。

### 门禁

- [ ] `make docs-check` 通过。
- [ ] `git diff --check` 通过。
- [ ] `cargo fmt --check` / `cargo clippy` / `cargo test --workspace` 通过。
- [ ] `dbt parse` 通过。
- [ ] `alembic upgrade/downgrade` 往返通过。

## 完成标准

1. 一个完整组合 run 的结果写入 ClickHouse `fleur_portfolio.*` 七张表，PostgreSQL 只保留控制状态和 `current_result_attempt_id`。
2. 同一 `portfolio_run_id` 重算生成新 `result_attempt_id`，旧 attempt 保留，PG 指针更新。
3. Rearview API 返回的组合结果来自 ClickHouse，Racingline 前端无感知。
4. 全部验收 Checklist 勾选通过。

## 后续计划

Plan 0044（待立项）将承接本计划未覆盖的内容：

1. dbt mart 层 `mart_risk_free_rate_daily`、`mart_benchmark_returns_daily`。
2. PostgreSQL `portfolio_metric_config` 表。
3. ClickHouse `portfolio_performance_metric` 表与 worker 绩效指标初算（12 个核心指标）。
4. mart 复算校验和跨 run 批量排名。
5. closed trade ledger 与交易级指标。
