# mono-fleur Rust Engines 文档地图

`engines/` 是 mono-fleur 的 Rust / Cargo workspace，用于承载高性能后端计算引擎和应用服务。
当前主要实现包括：

- `furnace`：由 Dagster 调度的金融技术指标计算 CLI，支持日频 KDJ、MA、RSI、BOLL 和价格行为结构指标计算。
- `rearview`：规则选股 HTTP 服务，消费 ClickHouse `fleur_marts` 指标 mart，并把规则、运行、股票池和买入信号写入 PostgreSQL `rearview` database。

## Workspace

```text
engines/
├── Cargo.toml
├── Cargo.lock
└── crates/
    ├── furnace/       # CLI 入口
    ├── furnace-core/  # 纯指标计算核心
    ├── furnace-io/    # ClickHouse I/O、批量写入和运行摘要
    └── rearview/      # 规则选股 HTTP 服务
```

所有 Rust / Cargo 命令都应在 `engines/` 目录下执行；不要把 Rust crate 放入 `pipeline/` 的 uv workspace。

## Crate 边界

| Crate | 类型 | 当前职责 |
|-------|------|----------|
| `furnace` | binary | 解析 `furnace kdj/ma/rsi/boll/price-pattern` CLI 参数，校验运行请求，调用 I/O 层并输出 JSON summary |
| `furnace-core` | library | 提供 KDJ、MA、RSI、BOLL、价格行为结构的参数、输入/输出模型、状态和单证券纯计算；不依赖 ClickHouse、Dagster、dbt、Rayon 或环境变量 |
| `furnace-io` | library | 负责 ClickHouse 表名、DDL、SQL、`clickhouse-client` 执行、RowBinary 读写、按证券并行调度、staging/partition replace 和运行摘要 |
| `rearview` | binary + library | 提供规则选股 HTTP API、metric catalog 校验、规则 AST 校验、ClickHouse runtime join 查询规划、PostgreSQL 运行状态和结果写入 |

核心边界：

- 指标公式只放在 `furnace-core`。
- ClickHouse、RowBinary、Rayon 并行、staging 和分区替换只放在 `furnace-io`。
- Dagster 只通过 Python resource 调用 CLI、传参并读取 JSON summary；不要在 Python asset 或 dbt SQL 中重写指标公式。

## 当前 Furnace 指标流程

```text
dbt intermediate
  fleur_intermediate.int_stock_quotes_daily_adj
      ↓ ClickHouse RowBinary scan
furnace-io
  按 security_code 分组，按证券维度 Rayon 并行
      ↓
furnace-core
  单证券按 trade_date 串行计算 KDJ、MA、RSI、BOLL 或 price-pattern
      ↓
furnace-io
  RowBinary 批量写入 staging 或生产表
      ↓
ClickHouse calculation
  fleur_calculation.calc_stock_<indicator>_daily
      ↓ dbt wrapper
  fleur_intermediate.int_stock_<indicator>_daily
```

生产写入只允许各指标的 canonical 参数。历史修正使用 `replace-cascade`，会从请求起点按指标 lookback 或 previous state 规则级联到受影响证券的最新输入交易日，并通过年度分区替换实现幂等。

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
- `price-pattern`：连阳/连阴和最近 20 根有效 high/low 内的前低-次低结构字段。

生产模式：

- `dry-run`：只计算和输出摘要，不写 ClickHouse。
- `append-latest`：用于最新区间追加，目标表存在同日或更晚结果时拒绝写入。
- `replace-cascade`：用于历史回填和修正，写入 staging 后替换受影响年度分区。

ClickHouse CLI 环境变量：

| 变量 | 用途 |
|------|------|
| `FURNACE_CLICKHOUSE_CLIENT` / `CLICKHOUSE_CLIENT` | `clickhouse-client` 命令，或用于本地 Docker 包装的命令 |
| `FURNACE_CLICKHOUSE_CLIENT_ARGS` | 追加给 client 命令的参数，例如 `exec -i mono-fleur-clickhouse clickhouse-client` |
| `CLICKHOUSE_HOST` | ClickHouse host，默认 `127.0.0.1` |
| `CLICKHOUSE_NATIVE_PORT` | Native port，默认 `9000` |
| `CLICKHOUSE_USER` / `CLICKHOUSE_PASSWORD` | 可选认证信息 |
| `CLICKHOUSE_SECURE` | 是否启用 secure 连接 |
| `CLICKHOUSE_CONNECT_TIMEOUT_SECONDS` | 连接超时 |
| `CLICKHOUSE_QUERY_TIMEOUT_SECONDS` | 查询收发超时 |
| `RAYON_NUM_THREADS` | 可选 Rayon worker 数；Dagster resource 默认注入 8，外部已设置时尊重外部值 |

## Rearview HTTP 服务

Rearview 第一版是单 crate 服务：`engines/crates/rearview/`。包内按 `domain`、`api`、`planner`、`clickhouse`、`postgres` 和 `service` 分模块；当前没有跨项目复用需求，不拆成多个 crate。

本地开发复用根目录 `.env` 和 `deploy/docker-compose.yml` 中的 PostgreSQL / ClickHouse：

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

Rearview 环境变量：

| 变量 | 用途 |
|------|------|
| `REARVIEW_DATABASE_URL` | PostgreSQL `rearview` database 连接 |
| `REARVIEW_HTTP_BIND` | HTTP bind 地址，默认 `127.0.0.1:34057` |
| `REARVIEW_MAX_CONCURRENT_RUNS` | 并发运行上限 |
| `REARVIEW_CHUNK_SMALL_RANGE_TRADING_DAYS` | 小区间单次 range query 阈值，默认 90 |
| `REARVIEW_CLICKHOUSE_MARTS_DATABASE` | ClickHouse mart database，默认 `fleur_marts` |
| `REARVIEW_CLICKHOUSE_MAX_EXECUTION_TIME_SECONDS` | ClickHouse 单查询执行时间上限 |
| `REARVIEW_CLICKHOUSE_MAX_ROWS_TO_READ` | ClickHouse 单查询扫描行数上限 |
| `REARVIEW_CLICKHOUSE_MAX_BYTES_TO_READ` | ClickHouse 单查询扫描字节上限 |
| `CLICKHOUSE_HOST` / `CLICKHOUSE_PORT` | ClickHouse HTTP 连接地址 |
| `CLICKHOUSE_USER` / `CLICKHOUSE_PASSWORD` | ClickHouse 认证 |

生成代表性规则样例：

```bash
cd engines
cargo run -p rearview -- sample-rule
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
| `docs/RFC/0016-rust-furnace-compute-engine.md` | Furnace Rust 计算引擎原始需求和长期边界 |
| `docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md` | RSV/KDJ 第一版实施方案和 Dagster/dbt/ClickHouse 边界 |
| `docs/plans/archive/0028-furnace-kdj-parallel-performance-implementation-plan.md` | 全市场 KDJ 并行计算、RowBinary 和性能观测方案 |
| `docs/jobs/reports/2026-06-07-furnace-kdj-smoke-run.md` | 单证券 dry-run、append-latest、replace-cascade 冒烟记录 |
| `docs/jobs/reports/2026-06-07-furnace-kdj-performance-baseline.md` | KDJ 性能基线记录 |
| `docs/jobs/reports/2026-06-07-furnace-kdj-parallel-optimization.md` | RowBinary、Rayon 和 full-range replace-cascade 优化记录 |
| `docs/jobs/reports/2026-06-09-furnace-price-pattern-full-market-validation.md` | 价格行为结构指标全市场写入、性能优化和验收记录 |
| `pipeline/scheduler/src/scheduler/defs/furnace/` | Dagster Furnace asset、job 和 schedule 定义 |
| `pipeline/scheduler/src/scheduler/defs/resources/furnace.py` | Python 侧 Furnace CLI resource |

生成 Rust API 文档：

```bash
make rust-doc
make rust-doc-serve
```
