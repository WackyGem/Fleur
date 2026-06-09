# mono-fleur Rust Engines 文档地图

`engines/` 是 mono-fleur 的 Rust / Cargo workspace，用于承载高性能后端计算引擎。
当前主要实现是 `furnace`：由 Dagster 调度的金融技术指标计算 CLI，支持日频 KDJ、MA、RSI、BOLL 和价格行为结构指标计算。

## Workspace

```text
engines/
├── Cargo.toml
├── Cargo.lock
└── crates/
    ├── furnace/       # CLI 入口
    ├── furnace-core/  # 纯指标计算核心
    └── furnace-io/    # ClickHouse I/O、批量写入和运行摘要
```

所有 Rust / Cargo 命令都应在 `engines/` 目录下执行；不要把 Rust crate 放入 `pipeline/` 的 uv workspace。

## Crate 边界

| Crate | 类型 | 当前职责 |
|-------|------|----------|
| `furnace` | binary | 解析 `furnace kdj/ma/rsi/boll/price-pattern` CLI 参数，校验运行请求，调用 I/O 层并输出 JSON summary |
| `furnace-core` | library | 提供 KDJ、MA、RSI、BOLL、价格行为结构的参数、输入/输出模型、状态和单证券纯计算；不依赖 ClickHouse、Dagster、dbt、Rayon 或环境变量 |
| `furnace-io` | library | 负责 ClickHouse 表名、DDL、SQL、`clickhouse-client` 执行、RowBinary 读写、按证券并行调度、staging/partition replace 和运行摘要 |

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
| `docs/plans/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md` | RSV/KDJ 第一版实施方案和 Dagster/dbt/ClickHouse 边界 |
| `docs/plans/0028-furnace-kdj-parallel-performance-implementation-plan.md` | 全市场 KDJ 并行计算、RowBinary 和性能观测方案 |
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
