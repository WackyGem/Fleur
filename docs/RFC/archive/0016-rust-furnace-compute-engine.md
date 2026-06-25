# RFC 0016: Furnace Rust 金融指标计算引擎原始需求

状态：Archived（2026-06-25；归档前状态：草案 / 原始需求（2026-06-07））

## 摘要

本文档记录 mono-fleur 新增 Rust 计算引擎 `furnace` 的原始需求。`furnace` 的第一阶段目标是作为高性能批处理计算引擎，计算 RSV/KDJ 等金融技术指标，由 Dagster 调度，从 dbt intermediate 层表读取行情数据，并将计算结果写回可被 marts 层消费的 ClickHouse 表。

本文档不是最终实现方案，也不创建代码。它用于冻结当前需求基线，后续可拆分为 ADR、实施 plan 和具体 crate 设计。

## 背景

当前 mono-fleur 已形成以下数据链路：

```text
Dagster source assets
  -> S3 Parquet source objects
  -> Dagster ClickHouse raw sync assets
  -> ClickHouse fleur_raw tables
  -> dbt source()
  -> dbt staging models
  -> dbt intermediate models
  -> dbt marts models
```

现有 Python/dbt/Dagster 工作区位于 `pipeline/`，并由 `uv` 管理。Rust 计算引擎不应混入该 Python 工作区。为支持 `furnace` 以及未来更多 Rust 后端工程，Rust 代码建议放在新的顶层 Cargo workspace：

```text
mono-fleur/
├── pipeline/              # Python / dbt / Dagster 工作区
├── deploy/
├── docs/
├── app/
└── engines/               # Rust 后端和计算引擎工作区
    ├── Cargo.toml
    ├── Cargo.lock
    └── crates/
        ├── furnace/
        ├── furnace-core/
        └── furnace-io/
```

## 原始需求

### 功能需求

1. `furnace` 应能计算 A 股日频行情上的技术指标，第一阶段指标为 RSV/KDJ；MACD 等其他指标作为后续扩展。
2. 输入数据来自 dbt intermediate 层表，第一候选输入为复权后的日频行情模型，例如 `int_stock_quotes_daily_adj`。
3. 输出数据应进入可被 dbt intermediate/marts 层使用的 ClickHouse 表。第一阶段 Furnace 目标表为 `fleur_calculation.calc_stock_technical_indicators_daily`；dbt 再包装为 `fleur_intermediate.int_stock_technical_indicators_daily`，供 marts 层后续消费。
4. `furnace` 必须支持按日期区间、股票代码集合和指标参数运行。
5. `furnace` 必须支持 Dagster 回填场景，即同一日期区间可重复执行；第一版通过受控年度分区重建/替换实现幂等，不在业务结果表中保留版本字段。
6. 指标计算必须显式处理 lookback / warm-up。计算目标区间时，读取输入数据的范围应向前扩展，以保证 RSV 滚动窗口和 KDJ 递推结果稳定。
7. KDJ 日常增量计算必须读取目标区间之前最近一条历史 K、D 状态；初始 K/D 固定 50 仅用于没有历史状态的空状态启动。
8. RSV 分母为 0 时按行业习惯填充 50，不视为无效行。
9. 输出结果必须可追踪。业务结果表只保留业务和计算字段；`run_id`、计算时间、写入版本等运行审计信息由 Dagster materialization metadata 和运行报告记录。

### 非功能需求

1. Rust 计算核心应与 ClickHouse、Dagster、dbt 解耦，便于单元测试和后续复用。
2. 指标计算结果必须可测试、可回归，第一版应提供 golden fixture 或等价的固定样本测试。
3. CLI 日志应适合被 Dagster 捕获，优先使用结构化日志。
4. ClickHouse 写入必须按批量执行，避免单行 insert 或过小 batch。
5. 重算和晚到数据处理不得依赖高频 `ALTER TABLE UPDATE`。
6. 表结构设计必须先明确主要查询模式，再确定 ClickHouse `ORDER BY`、partition 和 engine。

## 非目标

1. 第一版不实现实时交易、下单、策略执行或风控系统。
2. 第一版不替代 dbt 的 staging、intermediate、marts 建模职责。
3. 第一版不要求实现通用 SQL 引擎。
4. 第一版不要求把 MACD、RSI、布林线等其他技术指标一次性做完。
5. 第一版不要求做独立常驻服务；CLI 形态优先。

## 初步架构边界

### Rust workspace

建议使用 `engines/` 作为 Rust workspace 根目录，避免和 `pipeline/` 的 Python/uv 工作区混合。

初始 crate 边界：

| Crate | 类型 | 职责 |
|-------|------|------|
| `furnace` | binary | CLI 入口、配置加载、日志初始化、命令分发、进程退出码 |
| `furnace-core` | library | 指标公式、时间序列模型、参数、warm-up 规则、纯计算逻辑 |
| `furnace-io` | library | ClickHouse 读写、Parquet/Arrow/Polars 适配、批量写入 |

后续如果出现其他 Rust 后端工程，应作为同一 `engines/` workspace 下的独立 crate 或 crate group 管理，而不是放到 `pipeline/`。

### Dagster 边界

Dagster 负责：

1. 将 `furnace` 计算结果建模为 asset。
2. 表达 `furnace` 输出对 dbt intermediate 模型的真实数据依赖。
3. 提供 schedule、asset sensor 或 declarative automation。
4. 传递分区、日期区间、股票范围和运行参数。
5. 捕获日志、退出码和产出 metadata。

Dagster 不负责：

1. 在 Python asset 中重写指标公式。
2. 直接拼接复杂指标 SQL 替代 Rust 计算核心。

### dbt 边界

dbt 负责：

1. 维护输入 intermediate 模型的字段语义、tests 和文档。
2. 将 `fleur_calculation` 计算产物声明为 source，并通过 thin wrapper 暴露 `fleur_intermediate.int_*` 消费接口。
3. 在 marts 层统一业务可消费字段、补充 tests 和 docs。
4. 保持 `ref()` / `source()` lineage 可读。

dbt 不负责：

1. 在第一版中实现高性能递推指标核心。
2. 管理 Rust 进程执行、重试或回填。

## CLI 需求草案

第一版 CLI 形态示例：

```bash
furnace kdj \
  --from 2026-01-01 \
  --to 2026-01-31 \
  --symbols all \
  --input-table fleur_intermediate.int_stock_quotes_daily_adj \
  --output-table fleur_calculation.calc_stock_technical_indicators_daily \
  --rsv-window 9 \
  --k-smoothing 3 \
  --d-smoothing 3 \
  --run-id <dagster-run-id>
```

CLI 必须：

1. 对非法参数返回非零退出码。
2. 对可恢复 I/O 错误输出明确错误上下文。
3. 输出足够的运行摘要，至少包括输入行数、输出行数、symbol 数、目标日期范围、lookback 范围和写入批次数。

## 数据模型草案

第一阶段输出宽表结构：

```text
security_code
trade_date
rsv_window
k_smoothing
d_smoothing
rsv
k_value
d_value
j_value
```

实际读取输入范围属于单次运行上下文，不写入每行结果；由 Dagster materialization metadata、运行摘要和 job report 记录。

如果后续指标和参数组合显著增加，可评估长表结构：

```text
symbol
trade_date
indicator_name
parameter_set
value_name
value
```

宽表优先用于第一阶段，因为 RSV/KDJ 结果固定、dbt mart wrapper 更简单、下游查询更直接。

## 技术栈候选

| 层 | 候选 | 用途 |
|----|------|------|
| CLI | `clap` | 命令行参数 |
| 配置 | `serde` + `toml` 或 `config` | 环境和运行参数 |
| 日志 | `tracing` + `tracing-subscriber` | Dagster 可捕获日志 |
| 错误处理 | `thiserror` + `anyhow` | library typed errors 和 binary 边界错误 |
| 异步运行时 | `tokio` | ClickHouse I/O |
| ClickHouse | `clickhouse` / `clickhouse-rs` | typed select / insert |
| 列式处理 | `polars` | 批处理、排序、分组、Parquet |
| 并行计算 | `rayon` | 按 symbol 分组并行 |
| 测试 | `insta` / `proptest` / golden fixtures | 回归测试和性质测试 |

DataFusion 暂不作为第一版必需依赖。只有当 `furnace` 需要内置 SQL、注册 UDF 或直接成为可扩展查询引擎时再评估。

## 质量门禁草案

Rust workspace 建立后，建议在 `engines/` 下使用以下最小检查：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

与 mono-fleur 现有质量门禁的关系：

1. Python/dbt/Dagster 仍按 `pipeline/` 下既有 `uv run ...` 命令执行。
2. Rust 检查在 `engines/` 下执行，不改变 `pipeline/` 的 uv workspace。
3. 当 Dagster 集成 `furnace` 后，相关 scheduler 改动仍需运行 `uv run dg check defs`。

## 验收标准

第一阶段完成标准：

1. 仓库中存在 `engines/` Cargo workspace，且不会在项目子目录创建嵌套 Git 仓库。
2. `furnace` CLI 可以对固定样本计算 RSV/KDJ，并通过 golden test 验证。
3. `furnace-core` 不依赖 ClickHouse、Dagster 或 dbt。
4. `furnace` 可以按日期区间和 symbol 集合运行。
5. ClickHouse 写入 `fleur_calculation.calc_stock_technical_indicators_daily` 具备受控年度分区重建/替换幂等策略。
6. Dagster 可以调度一次 `furnace` 运行并记录输入、输出和运行 metadata。
7. dbt 可以将 `fleur_calculation.calc_stock_technical_indicators_daily` 包装为 `fleur_intermediate.int_stock_technical_indicators_daily`，并提供基础 tests 和字段文档。

## 已决策项

1. `fleur_calculation.calc_stock_technical_indicators_daily` 的 dbt/Dagster 接入采用 dbt source + Dagster asset metadata + dbt thin wrapper。source 负责 dbt 外部输入，Dagster metadata 负责运行观测和跨工具 lineage，thin wrapper 负责 `fleur_intermediate` 稳定消费契约。

## 待决问题

1. RSV/KDJ 等指标表长期采用宽表还是长表？
2. Dagster 调度使用固定 schedule、asset sensor，还是 declarative automation？
3. 第一版是否需要 Polars，还是直接使用 typed rows + 按 symbol 流式计算？
4. 指标计算的权威校验基准来自第三方库、Python pandas 实现，还是人工固定 fixture？

## 相关文档

- `AGENTS.md`
- `docs/architecture/scheduler-architecture.md`
- `docs/architecture/scheduler-module-boundaries.md`
- `docs/RFC/archive/0014-clickhouse-layered-database-migration.md`
- `docs/RFC/archive/0015-dagster-dbt-asset-graph-integration.md`
- `docs/skills/fleur-harness/SKILL.md`
