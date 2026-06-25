# System: Furnace

状态：当前事实入口（2026-06-13）

## 代码根

| 路径 | 角色 |
|---|---|
| [engines/crates/furnace/](../../engines/crates/furnace/) | CLI 入口、参数解析、请求校验和 JSON summary 输出 |
| [engines/crates/furnace-core/](../../engines/crates/furnace-core/) | 技术指标参数、输入输出模型和单证券纯计算 |
| [engines/crates/furnace-io/](../../engines/crates/furnace-io/) | ClickHouse DDL/SQL、RowBinary、并行调度、staging 和分区替换 |
| [pipeline/scheduler/src/scheduler/defs/furnace/](../../pipeline/scheduler/src/scheduler/defs/furnace/) | Dagster Furnace assets、jobs 和 schedules |

## 职责

1. 计算 KDJ、MA、RSI、BOLL、MACD 和价格行为结构等技术指标。
2. 通过 Rust CLI 暴露 dry-run、append-latest 和 replace-cascade 写入模式。
3. 将指标结果写入 ClickHouse `fleur_calculation`，再由 dbt wrapper 暴露到 intermediate/marts。
4. 保持指标公式集中在 `furnace-core`，让 Python 和 dbt 只负责编排与消费。

## 非职责

1. 不负责外部数据采集、raw sync 或 dbt 建模。
2. 不承担 Rearview 规则选股、运行状态和结果解释。
3. 不把 ClickHouse、RowBinary、Rayon 或环境变量依赖放入 `furnace-core`。

## 运行入口

所有 Cargo 命令在 `engines/` 目录下执行：

```bash
cd engines
cargo run -p furnace -- kdj \
  --from 2026-05-06 \
  --to 2026-06-01 \
  --symbols 000069.SZ \
  --mode dry-run \
  --output-format json
```

生成 Rust API 文档：

```bash
make rust-doc
make rust-doc-serve
```

## 质量门禁

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 Dagster 调用 Furnace 时，追加 scheduler 定向测试：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/furnace/test_furnace_definitions.py scheduler/tests/unit/resources/test_furnace.py
```

## 相关文档

| 文档 | 用途 |
|---|---|
| [../../engines/README.md](../../engines/README.md) | Rust engines 工作区地图和 Furnace CLI 入口 |
| [../RFC/archive/0016-rust-furnace-compute-engine.md](../RFC/archive/0016-rust-furnace-compute-engine.md) | Furnace 原始设计和长期边界 |
| [../RFC/archive/0017-furnace-moving-average-technical-indicators.md](../RFC/archive/0017-furnace-moving-average-technical-indicators.md) | MA 指标设计 |
| [../ADR/0010-technical-indicator-field-naming.md](../ADR/0010-technical-indicator-field-naming.md) | 技术指标字段命名决策 |
| [../jobs/reports/2026-06-07-furnace-kdj-smoke-run.md](../jobs/reports/2026-06-07-furnace-kdj-smoke-run.md) | KDJ 冒烟运行记录 |
| [../jobs/reports/2026-06-09-furnace-price-pattern-full-market-validation.md](../jobs/reports/2026-06-09-furnace-price-pattern-full-market-validation.md) | 价格行为结构全市场验证 |
| [../optimize/archive/engines-rust-quality-structure-optimization-2026-06-08.md](../optimize/archive/engines-rust-quality-structure-optimization-2026-06-08.md) | engines Rust 质量结构审计 |

## 待决问题

1. 是否需要为每个指标族拆出当前事实文档，减少历史 plan 和 job report 的检索成本。
2. 是否需要统一 Furnace 与 Rearview 的 Rust service/CLI 观测和错误模型约定。
