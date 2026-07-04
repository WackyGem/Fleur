# fleur Rust Engines 文档地图

`engines/` 是 fleur 的 Rust / Cargo workspace，用于承载高性能后端计算引擎和应用服务。
当前主要实现包括：

- `furnace`：由 Dagster 调度的金融技术指标计算 CLI，支持日频 KDJ、MA、RSI、BOLL、MACD 和价格行为结构指标计算。
- `rearview-core`、`rearview-server`、`rearview-portfolio-worker`：规则选股 HTTP 服务、共享核心库和组合净值异步 worker。

## Workspace

```text
engines/
├── Cargo.toml
├── Cargo.lock
└── crates/
    ├── furnace/                    # CLI 入口
    ├── furnace-core/               # 纯指标计算核心
    ├── furnace-io/                 # ClickHouse I/O、批量写入和运行摘要
    ├── rearview-core/              # Rearview 共享 domain、repository、API、ClickHouse 和组合计算
    ├── rearview-server/            # Rearview HTTP server binary
    └── rearview-portfolio-worker/  # 组合净值 NATS worker binary
```

所有 Rust / Cargo 命令都应在 `engines/` 目录下执行；不要把 Rust crate 放入 `pipeline/` 的 uv workspace。

## Crate 边界

| Crate | 类型 | 当前职责 |
|-------|------|----------|
| `furnace` | binary | 解析 `furnace kdj/ma/rsi/boll/macd/price-pattern` CLI 参数，校验运行请求，调用 I/O 层并输出 JSON summary |
| `furnace-core` | library | 提供 KDJ、MA、RSI、BOLL、MACD、价格行为结构的参数、输入/输出模型、状态和单证券纯计算；不依赖 ClickHouse、Dagster、dbt、Rayon 或环境变量 |
| `furnace-io` | library | 负责 ClickHouse 表名、DDL、SQL、官方 HTTP client typed I/O、按证券并行调度、staging/partition replace 和运行摘要 |
| `rearview-core` | library | 提供 Rearview config、domain、API router、PostgreSQL repository、ClickHouse client、metric catalog、规则运行服务和组合净值纯计算 |
| `rearview-server` | binary | 提供 Rearview HTTP server、catalog CLI 和 portfolio outbox dispatcher |
| `rearview-portfolio-worker` | binary | 消费 NATS JetStream portfolio 任务，读取 PostgreSQL 快照和 ClickHouse 后复权行情，写回组合明细账本和净值 |

核心边界：

- 指标公式只放在 `furnace-core`。
- ClickHouse I/O、Rayon 并行、staging 和分区替换只放在 `furnace-io`。
- Dagster 只通过 Python resource 调用 CLI、传参并读取 JSON summary；不要在 Python asset 或 dbt SQL 中重写指标公式。

## 当前 Furnace 指标流程

```text
dbt intermediate
  fleur_intermediate.int_stock_quotes_daily_adj
      ↓ ClickHouse HTTP typed scan
furnace-io
  按 security_code 分组，按证券维度 Rayon 并行
      ↓
furnace-core
  单证券按 trade_date 串行计算 KDJ、MA、RSI、BOLL、MACD 或 price-pattern
      ↓
furnace-io
  typed batches 写入 staging 或生产表
      ↓
ClickHouse calculation
  fleur_calculation.calc_stock_<indicator>_daily
      ↓ dbt wrapper
  fleur_intermediate.int_stock_<indicator>_daily
```

生产写入只允许各指标的 canonical 参数。历史修正使用 `replace-cascade`，会从请求起点按指标 lookback 或 previous state 规则级联到受影响证券的最新输入交易日，并通过年度分区替换实现幂等。需要按新 schema 或新算法全量刷新时使用 `rebuild-table`，Furnace 会先完成本次计算，确认有产出行后删除并重建输出表，再写入本次请求范围内的全量结果。

## CLI

常用 dry-run：

```bash
cd engines
cargo run -p furnace -- kdj \
  --from 2026-05-06 \
  --to 2026-06-01 \
  --symbols 000069.SZ \
  --mode dry-run \
  --output-format json
```

可用指标命令：

- `kdj`：RSV/K/D/J，生产 canonical 参数为 `KDJ(9,3,3)`。
- `ma`：价格均线、成交量均线和 EMA 派生指标。
- `rsi`：多窗口 RSI 和递推状态。
- `boll`：多窗口 Bollinger Bands。
- `macd`：DIF、DEA、MACD histogram 和递推 EMA 状态。
- `price-pattern`：连阳/连阴和最近 20 个交易日内的 L1 -> H1 -> L2 -> 当前重新上攻 N 字结构字段。

生产模式：

- `dry-run`：只计算和输出摘要，不写 ClickHouse。
- `append-latest`：用于最新区间追加，目标表存在同日或更晚结果时拒绝写入。
- `replace-cascade`：用于历史回填和修正，写入 staging 后替换受影响年度分区。
- `rebuild-table`：用于 schema 或算法变化后的全量重建。该模式不使用影子表、staging 或分区替换；先计算并校验本次产出非空，再执行 `DROP TABLE IF EXISTS`、创建 canonical 输出表并批量写入。请求范围就是新表内容，局部范围会得到局部新表。

ClickHouse 配置口径：

Furnace 当前通过官方 `clickhouse` Rust crate 的 HTTP client 访问 ClickHouse。生产路径不依赖宿主机 `clickhouse-client`、Docker exec wrapper 或 native port。

| 变量 | 用途 |
|------|------|
| `FURNACE_BINARY_PATH` | Dagster Furnace asset 调用的 CLI 路径；默认 `engines/target/release/furnace`，需要调试时才显式改为 debug binary |
| `FURNACE_CLICKHOUSE_URL` | 可选覆盖；未设置时从 `CLICKHOUSE_HOST` / `CLICKHOUSE_PORT` 推导 Furnace ClickHouse HTTP URL |
| `FURNACE_CLICKHOUSE_VALIDATE_SCHEMA` | official client schema validation 开关，默认 `false`；使用 RowBinary 性能路径，关闭 validation 后的全部指标写入 smoke 见 2026-07-01 迁移报告 |
| `CLICKHOUSE_HOST` / `CLICKHOUSE_PORT` | 未设置 `FURNACE_CLICKHOUSE_URL` 时用于推导 HTTP URL；本地 Makefile 默认由 `CLICKHOUSE_HTTP_PORT` 派生 |
| `CLICKHOUSE_USER` / `CLICKHOUSE_PASSWORD` | 可选认证信息 |
| `CLICKHOUSE_DB` | optional default database；Furnace SQL 仍使用 fully-qualified table |
| `CLICKHOUSE_SECURE` | 未设置 `FURNACE_CLICKHOUSE_URL` 时用于选择 `https` |
| `CLICKHOUSE_QUERY_TIMEOUT_SECONDS` | 查询和写入 timeout 设置 |
| `RAYON_NUM_THREADS` | 可选 Rayon worker 数；Dagster resource 默认注入 8，外部已设置时尊重外部值 |

`CLICKHOUSE_DATABASE` 服务 scheduler raw sync / contract 工具，不属于 Furnace 配置；Furnace SQL 必须继续使用 fully-qualified table。

Furnace CLI 和 Dagster config 的默认 `insert_batch_size` 为 100,000。该值处于 ClickHouse 推荐的 10K-100K 行/批范围上限，用于减少 HTTP insert 请求数；需要排查单批写入问题时可在 CLI 或 Dagster op config 中临时下调。

## Rearview HTTP 服务和组合 worker

Rearview 当前拆分为三个 crate：`rearview-core`、`rearview-server` 和 `rearview-portfolio-worker`。`rearview-server` 和 `rearview-portfolio-worker` 只依赖 `rearview-core`，二者不互相依赖。旧 `rearview` package 不再作为 workspace 入口。

本地开发复用根目录 `.env` 和 `deploy/docker-compose.yml` 中的 PostgreSQL / ClickHouse：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d postgres clickhouse nats

cd pipeline
uv run alembic -c migrate/alembic.ini -x target=pipeline upgrade head
uv run alembic -c migrate/alembic.ini -x target=rearview upgrade head

cd ../engines
cargo run -p rearview-server -- catalog check
cargo run -p rearview-server -- catalog sync
cargo run -p rearview-server -- serve
cargo run -p rearview-portfolio-worker -- run
```

常用 HTTP API：

| Method | Path | 用途 |
|---|---|---|
| `GET` | `/healthz` | 服务健康检查 |
| `POST` | `/rearview/rule-sets` | 创建规则集 |
| `POST` | `/rearview/rule-sets/{rule_set_id}/versions` | 创建不可变规则版本 |
| `POST` | `/rearview/runs` | 发起区间选股运行 |
| `GET` | `/rearview/runs/{run_id}` | 查询运行状态和 summary |
| `GET` | `/rearview/runs/{run_id}/chunks` | 查询 chunk 状态、ClickHouse query id 和耗时 |
| `GET` | `/rearview/runs/{run_id}/days` | 查询日粒度股票池和信号数量 |
| `GET` | `/rearview/runs/{run_id}/pool?trade_date=YYYY-MM-DD` | 查询某日股票池 |
| `GET` | `/rearview/runs/{run_id}/signals?trade_date=YYYY-MM-DD` | 查询某日 TopN 买入信号 |
| `POST` | `/rearview/explain` | 校验规则并返回所需 mart、列、SQL hash；带日期时返回 chunk plan |
| `POST` | `/rearview/strategy-backtests/validate` | 校验 transient `RuleVersionSpec + BacktestExecutionConfig`，返回 canonical config、rule hash、execution config hash 和执行摘要；不创建 run 或 portfolio run |
| `GET` | `/rearview/strategy-backtests/options` | 查询 Step 5 period/benchmark 选项和动态解析区间 |
| `POST` | `/rearview/strategy-backtests` | 创建 strategy backtest queued run 和 outbox task，返回 `202 Accepted` |
| `GET` | `/rearview/strategy-backtests/{strategy_backtest_run_id}` | 查询 strategy backtest run、frozen range 和 result attempt 状态 |
| `GET` | `/rearview/strategy-backtests/{strategy_backtest_run_id}/status` | 查询 Step 5 compact status/gate view |
| `GET` | `/rearview/strategy-backtests/{strategy_backtest_run_id}/overview?view=ui` | 查询 Step 5 首屏 compact overview：status、nav、performance 和 rebalance read model |
| `GET` | `/rearview/strategy-backtests/{strategy_backtest_run_id}/nav` | 查询 strategy backtest 净值曲线，支持 `view=ui` compact response |
| `GET` | `/rearview/strategy-backtests/{strategy_backtest_run_id}/rebalance-records` | 查询 strategy backtest 调仓记录，支持 `view=ui` compact response |
| `GET` | `/rearview/strategy-backtests/{strategy_backtest_run_id}/performance` | 查询 strategy backtest 绩效指标，支持 `view=ui` compact response |
| `GET` | `/rearview/market-fee-templates/default?market=CN_A_SHARE` | 查询默认市场费率和滑点模板 |
| `GET` | `/rearview/rule-sets/{rule_set_id}/account-templates` | 查询策略虚拟账户模板 |
| `POST` | `/rearview/rule-sets/{rule_set_id}/account-templates` | 创建策略虚拟账户模板 |
| `PATCH` | `/rearview/account-templates/{account_template_id}` | 更新策略虚拟账户模板 |
| `POST` | `/rearview/portfolio-runs` | 从成功选股 run 创建组合运行并写入 outbox |
| `GET` | `/rearview/portfolio-runs` | 查询组合运行列表 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}` | 查询组合运行状态、dispatch 状态和 summary |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/nav` | 查询组合净值曲线 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/targets` | 查询调仓目标 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/orders` | 查询虚拟订单 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/trades` | 查询虚拟成交 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/positions` | 查询每日或最新持仓 |
| `GET` | `/rearview/portfolio-runs/{portfolio_run_id}/events` | 查询组合 warning 和审计事件 |

Rearview 环境变量：

| 变量 | 用途 |
|------|------|
| `REARVIEW_DATABASE_URL` | 运行时派生的 PostgreSQL `rearview` database 连接；本地和 compose 默认由 `POSTGRES_*` 生成 |
| `REARVIEW_HTTP_BIND` | HTTP bind 地址，默认 `127.0.0.1:34057` |
| `REARVIEW_MAX_CONCURRENT_RUNS` | 并发运行上限 |
| `REARVIEW_CHUNK_SMALL_RANGE_TRADING_DAYS` | 小区间单次 range query 阈值，默认 90 |
| `REARVIEW_CLICKHOUSE_MARTS_DATABASE` | ClickHouse mart database，默认 `fleur_marts` |
| `REARVIEW_CLICKHOUSE_MAX_EXECUTION_TIME_SECONDS` | ClickHouse 单查询执行时间上限 |
| `REARVIEW_CLICKHOUSE_MAX_ROWS_TO_READ` | ClickHouse 单查询扫描行数上限 |
| `REARVIEW_CLICKHOUSE_MAX_BYTES_TO_READ` | ClickHouse 单查询扫描字节上限 |
| `REARVIEW_NATS_URL` | NATS 连接地址；本地默认由 `NATS_CLIENT_PORT` 派生 |
| `REARVIEW_PORTFOLIO_STREAM` | portfolio JetStream stream 名称 |
| `REARVIEW_PORTFOLIO_REQUEST_SUBJECT` | portfolio 任务 subject |
| `REARVIEW_PORTFOLIO_WORKER_DURABLE` | portfolio worker durable consumer |
| `REARVIEW_PORTFOLIO_WORKER_QUEUE` | portfolio worker queue group |
| `CLICKHOUSE_HOST` / `CLICKHOUSE_PORT` | ClickHouse HTTP 连接地址 |
| `CLICKHOUSE_USER` / `CLICKHOUSE_PASSWORD` | ClickHouse 认证 |

生成代表性规则样例：

```bash
cd engines
cargo run -p rearview-server -- sample-rule
```

## 质量门禁

Rust 变更至少运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 release 性能、全市场回填或写入路径时，补充：

```bash
cd engines
cargo build --release -p furnace
```

涉及 Dagster 调用 Furnace 时，还需要在 `pipeline/` 下运行相关 scheduler 检查和单元测试，例如：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/unit/resources/test_furnace.py
```

## 相关文档

| 文档 | 用途 |
|------|------|
| `docs/RFC/archive/0016-rust-furnace-compute-engine.md` | Furnace Rust 计算引擎原始需求和长期边界 |
| `docs/plans/archive/0068-furnace-clickhouse-rust-client-migration-plan.md` | Furnace 官方 `clickhouse` Rust client 迁移计划和完成记录 |
| `docs/jobs/reports/2026-07-02-furnace-price-pattern-rebuild-table-rerun.md` | Price Pattern 新 N 字结构字段 rebuild-table 全量重建和 dbt 重跑记录 |
| `docs/jobs/reports/2026-07-01-furnace-clickhouse-rust-client-migration.md` | Furnace 全指标 HTTP client 迁移、dry-run、replace-cascade 写入和性能验证记录 |
| `docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md` | RSV/KDJ 第一版实施方案和 Dagster/dbt/ClickHouse 边界 |
| `docs/plans/archive/0028-furnace-kdj-parallel-performance-implementation-plan.md` | 迁移前全市场 KDJ 并行计算、RowBinary 和性能观测方案 |
| `docs/jobs/reports/2026-06-07-furnace-kdj-smoke-run.md` | 单证券 dry-run、append-latest、replace-cascade 冒烟记录 |
| `docs/jobs/reports/2026-06-07-furnace-kdj-performance-baseline.md` | KDJ 性能基线记录 |
| `docs/jobs/reports/2026-06-07-furnace-kdj-parallel-optimization.md` | 迁移前 RowBinary、Rayon 和 full-range replace-cascade 优化记录 |
| `docs/jobs/reports/2026-06-09-furnace-price-pattern-full-market-validation.md` | 价格行为结构指标全市场写入、性能优化和验收记录 |
| `pipeline/scheduler/src/scheduler/defs/furnace/` | Dagster Furnace asset、job 和 schedule 定义 |
| `pipeline/scheduler/src/scheduler/defs/resources/furnace.py` | Python 侧 Furnace CLI resource |

生成 Rust API 文档：

```bash
make rust-doc
make rust-doc-serve
```
