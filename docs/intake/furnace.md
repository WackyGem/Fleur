# Intake: Furnace

状态：当前需求投递入口（2026-06-13）

当前事实地图：[../systems/furnace.md](../systems/furnace.md)

## 适用需求

- 新增或修改 Rust 技术指标公式、参数、状态和输出字段。
- 修改 Furnace CLI、运行模式、RowBinary I/O、ClickHouse 写入和分区替换逻辑。
- 优化指标计算性能、并行策略、内存和全市场运行表现。
- 调整 Dagster 调用 Furnace 的资源参数、asset 定义或运行摘要消费方式。

## 不适用

- dbt mart 消费模型和字段治理：走 [data-platform.md](data-platform.md) 或 [data-governance.md](data-governance.md)。
- Rearview 规则选股查询和信号结果：走 [rearview.md](rearview.md)。
- 前端展示指标结果：走 [racingline.md](racingline.md)。

## 投递材料

1. 指标名称、公式、参数和 canonical 参数约束。
2. 输入表、输出表、字段命名和历史修正规则。
3. 是否需要 previous state、lookback、replace-cascade 或年度分区替换。
4. 性能目标、样本证券、日期范围和全市场验收范围。
5. Dagster/dbt wrapper 和 mart 消费影响。

## 文档落点

| 情况 | 落点 |
|---|---|
| 新指标族或计算架构变化 | `docs/RFC/` |
| 已确定的指标实施和性能优化 | `docs/plans/` |
| 指标字段命名或 canonical 参数长期约束 | `docs/ADR/` |
| 当前 Furnace CLI、质量门禁或边界变化 | [../systems/furnace.md](../systems/furnace.md) |
| smoke run、性能基线和全市场核验 | `docs/jobs/reports/` |

## 验证要求

Rust 变更至少运行：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

涉及 Dagster 调用时追加定向 scheduler 测试。
