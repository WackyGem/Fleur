# System: Rearview

状态：当前事实入口（2026-06-13）

## 代码根

| 路径 | 角色 |
|---|---|
| [engines/crates/rearview/](../../engines/crates/rearview/) | Rust 规则选股 HTTP 服务 |
| [engines/crates/rearview/config/metric_policy.yml](../../engines/crates/rearview/config/metric_policy.yml) | metric policy overlay |
| [pipeline/migrate/](../../pipeline/migrate/) | PostgreSQL `rearview` database migration 入口 |

## 职责

1. 提供规则集、不可变规则版本、区间运行、股票池和买入信号 HTTP API。
2. 校验规则 AST 和 metric catalog，编译受控 ClickHouse 查询。
3. 消费 ClickHouse `fleur_marts` 指标 mart，并把运行状态、股票池和买入信号写入 PostgreSQL `rearview` database。
4. 保存 rule hash、compiled SQL hash、ClickHouse query id、chunk 状态和结果解释快照。

## 非职责

1. 不重算 KDJ、MA、RSI、BOLL、MACD 或价格行为结构指标；这些由 Furnace/dbt 维护。
2. 不绕过 mart 层读取 raw、staging、intermediate 或 calculation 表。
3. 不提供前端交互；Racingline 承担 UI 工作台。
4. 不自动执行 PostgreSQL DDL migration；迁移由 `pipeline/migrate` 管理。

## 主要依赖

| 依赖 | 用途 |
|---|---|
| PostgreSQL `rearview` database | 规则、版本、运行、chunk、day、pool、signal 和 metric catalog 状态 |
| ClickHouse `fleur_marts` | 日频行情、趋势、动量、成交量和价格行为结构指标 |
| dbt mart YAML | metric catalog 基础字段事实校验来源 |
| Furnace/dbt | 指标计算和 mart 物化 |

## 运行入口

本地开发复用根目录 `.env` 和 `deploy/docker-compose.yml`：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d postgres clickhouse

cd pipeline
uv run alembic -c migrate/alembic.ini -x target=pipeline upgrade head
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head

cd ../engines
cargo run -p rearview -- catalog check
cargo run -p rearview -- catalog sync
cargo run -p rearview -- serve
```

## 质量门禁

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 migration 时追加：

```bash
cd pipeline/migrate
uv run alembic upgrade head
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../../engines/README.md](../../engines/README.md) | Rust engines 工作区地图和 Rearview HTTP API 入口 |
| [../RFC/0018-rust-stock-screening-service.md](../RFC/0018-rust-stock-screening-service.md) | Rearview 后端服务设计 |
| [../RFC/0019-racingline-rearview-frontend-workbench.md](../RFC/0019-racingline-rearview-frontend-workbench.md) | Racingline 前端工作台设计 |
| [../plans/archive/0036-rust-rearview-stock-screening-service-implementation-plan.md](../plans/archive/0036-rust-rearview-stock-screening-service-implementation-plan.md) | Rearview 后端历史实施计划 |
| [../jobs/reports/2026-06-12-rearview-n-structure-low-reversal-smoke-run.md](../jobs/reports/2026-06-12-rearview-n-structure-low-reversal-smoke-run.md) | 代表性规则 smoke run 记录 |

## 待决问题

1. UI 友好接口何时进入后端实施：`GET /rearview/runs`、`GET /rearview/rule-sets`、`GET /rearview/metrics` 等。
2. Rearview 鉴权、用户隔离和 API 错误响应结构是否应上升为 ADR。
3. 是否新增 `mart_stock_rearview_metric_daily` 作为选股专用宽表。
