# RFC

`docs/RFC/` 顶层保留当前仍在讨论或尚未执行的活跃 RFC；历史设计文档统一放在 [archive/](archive/)。

## Active RFCs

| RFC | 状态 | 用途 |
|---|---|---|
| [0038-dbt-baostock-downstream-performance-optimization.md](0038-dbt-baostock-downstream-performance-optimization.md) | Proposed | BaoStock dbt 下游构建性能分层优化，优先拆分重型测试窗口和日常/完整验证路径 |
| [0036-racingline-strategy-portfolio-statement.md](0036-racingline-strategy-portfolio-statement.md) | Proposed | Racingline 策略组合详情页账户对账单的数据盘点、ClickHouse 支撑评估和 Rearview read model 缺口 |
| [0034-racingline-step5-portfolio-publish-dialog-tabs.md](0034-racingline-step5-portfolio-publish-dialog-tabs.md) | Proposed | Racingline Step 5「建立策略组合」弹层分 Tab 信息架构、T+1 建仓语义和 backtest/live 数据隔离 |
| [0033-project-version-management.md](0033-project-version-management.md) | Proposed | fleur 多工程组件版本、数据契约版本、迁移 revision 和集成发布 tag 管理方案 |
| [0032-racingline-step5-backtest-worker-execution-latency.md](0032-racingline-step5-backtest-worker-execution-latency.md) | Proposed | Racingline Step 5 回测 worker 执行流程抽象、必须/冗余流程和计算耗时优化讨论 |
| [0031-racingline-step4-step5-backtest-latency-slimming.md](0031-racingline-step4-step5-backtest-latency-slimming.md) | Proposed | Racingline Step 4 到 Step 5 回测跳转链路瘦身与延时治理 |
| [0030-baostock-daily-kline-compacted-yearly-range-rebuild.md](0030-baostock-daily-kline-compacted-yearly-range-rebuild.md) | Active | BaoStock 日 K range backfill 与 compacted 完整性设计 |

## Archive

归档 RFC 只用于追溯方案背景。引用归档 RFC 时，应同时以对应 `docs/architecture/`、`docs/ADR/`、运行报告或当前代码作为当前事实依据。

- [0037-baostock-daily-kline-unified-range-request.md](archive/0037-baostock-daily-kline-unified-range-request.md) | Completed | BaoStock 日 K 取消 daily/range_backfill mode 分支，统一由 Dagster partition selection 推导区间请求
