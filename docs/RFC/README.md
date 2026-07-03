# RFC

`docs/RFC/` 顶层保留当前仍在讨论或尚未执行的活跃 RFC；历史设计文档统一放在 [archive/](archive/)。

## Active RFCs

- [0047-production-compose-nginx-entrypoint.md](0047-production-compose-nginx-entrypoint.md) | Implemented | 将现有开发 Docker Compose 改名为 `docker-compose.dev.yaml`，重新设计生产 `docker-compose.yml`，通过 build 构建应用镜像，并引入 nginx 作为 Racingline 默认入口和 Dagster 统一入口；验收见 [2026-07-03 smoke report](../jobs/reports/2026-07-03-production-compose-nginx-smoke.md)

## Archive

归档 RFC 只用于追溯方案背景。引用归档 RFC 时，应同时以对应 `docs/architecture/`、`docs/ADR/`、运行报告或当前代码作为当前事实依据。

- [0046-racingline-strategy-detail-config-display.md](archive/0046-racingline-strategy-detail-config-display.md) | Implemented | Racingline 策略详情页页头策略配置方案按钮、点击详情层、Step 5 配置复用和 canonical 配置派生展示；0075 已完成 `rule_snapshot` + `execution_config` 纠偏和 0051 browser smoke
- [0045-strategy-portfolio-daily-nav-liquidation.md](archive/0045-strategy-portfolio-daily-nav-liquidation.md) | Implemented | `strategy_portfolio_daily_runs` 已更名为 `daily__portfolio_nav_liquidation`，并作为 `daily__fetch_history_sources_to_marts_schedule_job` 的终端阶段执行 strategy portfolio 日度 NAV 清算；实现见 Plan 0073 和 2026-07-02 运行报告
- [0044-racingline-0051-low-reversal-regression-case.md](archive/0044-racingline-0051-low-reversal-regression-case.md) | Implemented | Racingline 0051 低位反转 example portfolio data config、Rearview example ensure API 与 `example__portfolio_live_job`；实现见 Plan 0072，验收见 2026-07-02 运行报告
- [0043-racingline-strategy-detail-delete-action.md](archive/0043-racingline-strategy-detail-delete-action.md) | Implemented | Racingline 策略详情页删除按钮接入 Rearview archive API；实现见 Plan 0071，验收见 2026-07-02 运行报告
- [0042-racingline-trade-win-rate-semantics.md](archive/0042-racingline-trade-win-rate-semantics.md) | Archived | Racingline 交易胜率、卖出胜率、已平仓交易胜率和日胜率的当前口径盘点与命名收敛讨论
- [0041-racingline-strategy-backtest-entry-rule-baseline.md](archive/0041-racingline-strategy-backtest-entry-rule-baseline.md) | Implemented | Racingline 回测 entry rule baseline、T+1 和信号日期约束
- [0040-dagster-stg-to-mart-asset-inventory.md](archive/0040-dagster-stg-to-mart-asset-inventory.md) | Archived | Dagster stg 到 mart 资产盘点，包含 dbt staging/intermediate/marts、Furnace calculation 和 portfolio 相关资产基线
- [0039-source-raw-backfill-complexity-baseline.md](archive/0039-source-raw-backfill-complexity-baseline.md) | Implemented | Source/Raw 回填复杂度现状基线与统一手动回填 controller job；实现见 Plan 0065，验收见 2026-06-30 运行报告
- [0038-dbt-baostock-downstream-performance-optimization.md](archive/0038-dbt-baostock-downstream-performance-optimization.md) | Archived | BaoStock dbt 下游构建性能分层优化，优先拆分重型测试窗口和日常/完整验证路径
- [0037-baostock-daily-kline-unified-range-request.md](archive/0037-baostock-daily-kline-unified-range-request.md) | Completed | BaoStock 日 K 取消 daily/range_backfill mode 分支，统一由 Dagster partition selection 推导区间请求
- [0036-racingline-strategy-portfolio-statement.md](archive/0036-racingline-strategy-portfolio-statement.md) | Archived | Racingline 策略组合详情页账户对账单的数据盘点、ClickHouse 支撑评估和 Rearview read model 缺口
- [0035-racingline-strategy-portfolio-virtual-account-panel.md](archive/0035-racingline-strategy-portfolio-virtual-account-panel.md) | Implemented | Racingline 策略组合详情页虚拟账户面板、资金口径和对账入口
- [0034-racingline-step5-portfolio-publish-dialog-tabs.md](archive/0034-racingline-step5-portfolio-publish-dialog-tabs.md) | Archived | Racingline Step 5「建立策略组合」弹层分 Tab 信息架构、T+1 建仓语义和 backtest/live 数据隔离
- [0033-project-version-management.md](archive/0033-project-version-management.md) | Archived | fleur 多工程组件版本、数据契约版本、迁移 revision 和集成发布 tag 管理方案
- [0032-racingline-step5-backtest-worker-execution-latency.md](archive/0032-racingline-step5-backtest-worker-execution-latency.md) | Archived | Racingline Step 5 回测 worker 执行流程抽象、必须/冗余流程和计算耗时优化讨论
- [0031-racingline-step4-step5-backtest-latency-slimming.md](archive/0031-racingline-step4-step5-backtest-latency-slimming.md) | Archived | Racingline Step 4 到 Step 5 回测跳转链路瘦身与延时治理
- [0030-baostock-daily-kline-compacted-yearly-range-rebuild.md](archive/0030-baostock-daily-kline-compacted-yearly-range-rebuild.md) | Archived | BaoStock 日 K range backfill 与 compacted 完整性设计
- [0030-racingline-step2-step3-preview-latency-slimming.md](archive/0030-racingline-step2-step3-preview-latency-slimming.md) | Implemented | Racingline Step 2 到 Step 3 预览链路瘦身与延时治理
