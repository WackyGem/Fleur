# Plans

顶层只保留仍需行动的 active plans。完成、废弃或被替代的计划移入 [archive/](archive/)。

## Active Plans

| Plan | 状态 | 说明 |
|---|---|---|
| _none_ |  |  |

## Recently Completed

| Plan | 状态 | 说明 |
|---|---|---|
| [0073](archive/0073-strategy-portfolio-daily-nav-liquidation-plan.md) | Completed | 基于 RFC 0045 将 `strategy_portfolio_daily_runs` 收敛为无分区 `daily__portfolio_nav_liquidation`，并作为 `daily__fetch_history_sources_to_marts_schedule_job` 的 portfolio live terminal step；验收见 [2026-07-02 report](../jobs/reports/2026-07-02-strategy-portfolio-daily-nav-liquidation.md) |
| [0072](archive/0072-racingline-0051-low-reversal-example-live-job-plan.md) | Completed | Racingline 0051 低位反转固化为 data config + Rearview 共享 canonical snapshot/persistence service + `example__portfolio_live_job` 手动清算回归用例；验收见 [2026-07-02 report](../jobs/reports/2026-07-02-racingline-0051-low-reversal-example-live-job.md) |
| [0071](archive/0071-racingline-strategy-detail-delete-action-plan.md) | Completed | Racingline 策略详情页删除按钮接入 Rearview archive API，补齐 archived detail `410 Gone`、Dashboard 跳转和手动 archived daily run 拒绝；验收见 [2026-07-02 report](../jobs/reports/2026-07-02-racingline-strategy-detail-delete-action.md) |
| [0070](archive/0070-racingline-strategy-publish-market-phase-entry-rule-plan.md) | Completed | Racingline Step 5 建立组合发布预检改为交易阶段感知：15:00 前允许上一交易日信号，15:00 后要求当天信号，数据多日落后继续阻断；验收见 [2026-07-02 report](../jobs/reports/2026-07-02-racingline-strategy-publish-market-phase-entry-rule.md) |
| [0069](archive/0069-racingline-strategy-entry-rule-implementation-plan.md) | Completed | Racingline 最近信号建仓日期 gate 与空位补仓规则实施：Rearview publish preview/create stale 校验、Racingline 发布弹层和 Step 4/Step 5 命名解释；验收见 [2026-07-02 report](../jobs/reports/2026-07-02-racingline-strategy-entry-rule-implementation.md) |
| [0068](archive/0068-furnace-clickhouse-rust-client-migration-plan.md) | Completed | Furnace 一刀切迁移到官方 `clickhouse` Rust client，移除外部 `clickhouse-client` / Docker exec 运行时依赖，覆盖 KDJ、MA、RSI、BOLL、MACD 和 Price Pattern；验收见 [2026-07-01 migration report](../jobs/reports/2026-07-01-furnace-clickhouse-rust-client-migration.md) |
| [0067](archive/0067-daily-source-to-marts-clean-slate-orchestration-plan.md) | Completed | Daily Source to Marts clean-slate 编排：新增 `daily__fetch_history_sources_to_marts_schedule_job` 和唯一 `daily__fetch_history_sources_to_marts_schedule`，清理旧 daily/transformation/source-specific production jobs；验收见 [2026-07-01 daily dry-run report](../jobs/reports/2026-07-01-daily-fetch-history-sources-to-marts-schedule-job-dry-run.md) |
| [0066](archive/0066-backfill-source-to-marts-controller-plan.md) | Completed | Backfill Sources to Marts controller：新增 `backfill__fetch_history_sources_to_marts_job`，保留 `backfill__fetch_history_sources_to_raw_job` raw-only 语义，移除旧 snapshot 公开入口，排除 Jiuyan 和 portfolio analytics；验收见 [2026-07-01 report](../jobs/reports/2026-07-01-backfill-source-to-marts-controller-dry-run.md) |
| [0065](archive/0065-source-raw-unified-backfill-controller-implementation-plan.md) | Completed | Source/Raw 统一手动回填 controller：按 `target_scope` 生成 source、compacted source 与 ClickHouse raw sync 子 runs，替换 BaoStock 专用 shell-out controller；验收见 [2026-06-30 report](../jobs/reports/2026-06-30-source-raw-unified-backfill-controller.md) |
| [0064](archive/0064-dbt-baostock-downstream-performance-optimization-plan.md) | Completed | BaoStock dbt 下游存量作业性能优化：删除低价值 mart 字段匹配测试、收敛日常 stock build selection、raw latest year 后触发固定 int/mart/calc 链路，并用 query log 基准完成 KDJ join 优化；验收见 [2026-06-29 report](../jobs/reports/2026-06-29-dbt-baostock-downstream-performance-optimization.md) |
| [0063](archive/0063-baostock-daily-kline-unified-range-timeout-plan.md) | Completed | BaoStock 日 K 取消 daily/range_backfill mode 分支，统一由 Dagster partition selection 推导区间请求，并补强 TCP timeout 与网络 fail-fast |
| [0062](archive/0062-racingline-strategy-portfolio-statement-plan.md) | Completed | Racingline 策略组合详情页对账单、Rearview statement read model、Dagster 清算作业终态校验和 2025 first-signal T+1 建仓验收；验收见 [2026-06-29 report](../jobs/reports/2026-06-29-racingline-strategy-portfolio-statement.md) |
| [0061](archive/0061-racingline-strategy-portfolio-virtual-account-panel-plan.md) | Completed | Racingline 组合详情页「虚拟资金账户」区块、Rearview account read model、pending-first-run 空态和前后端闭环；验收见 [2026-06-28 report](../jobs/reports/2026-06-28-racingline-portfolio-virtual-account-panel.md) |
| [0060](archive/0060-racingline-step5-portfolio-publish-dialog-tplus1-plan.md) | Completed | Racingline Step 5「建立组合」弹层 Tab 改造、T+1 发布预检、pending Dashboard 语义、backtest/live 两段数据分离和首个 daily run 信号窗口修正；控制面审计见 [audit report](../jobs/reports/2026-06-27-racingline-portfolio-control-plane-audit.md)，端到端验收见 [smoke report](../jobs/reports/2026-06-27-racingline-portfolio-publish-tplus1-smoke.md) |
| [0059](archive/0059-version-information-governance-implementation-plan.md) | Completed | 版本信息治理实施：Rust crate 独立版本、release manifest/release note、版本校验脚本、运行时版本暴露、Alembic head 表达修正和 tag 前检查入口 |
| [0058](archive/0058-racingline-step5-backtest-worker-latency-optimization-plan.md) | Completed | Racingline Step 5 回测 worker 执行耗时优化：simulation 低 clone 索引、backtest 专用 TopN signal SQL、MarketDataDemand 实验结论、overview 首屏读取和 worker bounded concurrency；验收见 [2026-06-26 report](../jobs/reports/2026-06-26-racingline-step5-backtest-worker-latency-optimization.md)，设计依据见 [RFC 0032](../RFC/0032-racingline-step5-backtest-worker-execution-latency.md) |
| [0057](archive/0057-baostock-daily-kline-range-backfill-implementation-plan.md) | Completed | BaoStock 日 K daily source 增加 `range_backfill` 模式并完成 2026 年首个有效交易日至 2026-06-25 的 compacted/raw 验收；验收见 [2026-06-25 report](../jobs/reports/2026-06-25-baostock-daily-kline-range-backfill.md) |
| [0056](archive/0056-racingline-step4-step5-backtest-latency-optimization-plan.md) | Completed | Racingline Step 4 到 Step 5 回测延时优化：create accepted 即进入 Step 5、前端状态 owner 收敛、status/result compact view、worker timing、price bars 动态投影、outbox 唤醒和 stale active 诊断；baseline 见 [baseline report](../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-baseline.md)，验收见 [optimization report](../jobs/reports/2026-06-25-racingline-step4-step5-backtest-latency-optimization.md) |
| [0055](archive/0055-racingline-step2-step3-preview-latency-slimming-plan.md) | Completed | Racingline Step 2 到 Step 3 预览链路瘦身：route/query 观测、preview-open、chart-context、Step 3 脏字段和默认 output metrics 清理；baseline 见 [baseline report](../jobs/reports/2026-06-25-racingline-step2-step3-preview-latency-baseline.md)，验收见 [slimming report](../jobs/reports/2026-06-25-racingline-step2-step3-preview-latency-slimming.md) |
| [0054](archive/0054-baostock-daily-kline-daily-source-compaction-plan.md) | Completed | BaoStock 日 K 从年分区远端抓取改为日分区 source + 年度 compacted raw sync，并完成 dev S3 历史年分区迁移；验收见 [2026-06-25 report](../jobs/reports/2026-06-25-baostock-daily-kline-compaction.md) |
| [0044](archive/0044-portfolio-performance-metrics-implementation-plan.md) | Archived | 组合绩效指标、int/mart risk-free 与 benchmark 输入、metric config、worker 写入 fleur_calculation、dbt thin wrapper/ranking 和 closed trade ledger 实施计划 |
| [0043](archive/0043-portfolio-data-plane-clickhouse-phase1-implementation-plan.md) | Archived | 组合数据面迁移 ClickHouse 第一阶段：结果事实存储迁移、result_attempt_id 幂等重算、worker 切换写入目标、API 切换读取源 |
| [0042](archive/0042-chinabond-government-bond-s3-raw-implementation-plan.md) | Archived | ChinaBond 国债收益率曲线 Dagster 年分区 S3 Parquet raw 接入与 2006-2026 回填计划 |
| [0023](archive/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md) | Archived | Contract-driven Parquet schema adapter 合入后的 dev 环境重置、小批量回填和全量回填准入计划 |
| [0053](archive/0053-racingline-legacy-cleanup-and-rename-plan.md) | Completed | Racingline 旧工程清理、`app/racingline_new` 重命名为 `app/racingline`、Makefile 和当前事实文档收敛；验收见 [2026-06-25 report](../jobs/reports/2026-06-25-racingline-legacy-cleanup-rename.md) |
| [0052](archive/0052-racingline-strategy-portfolio-publish-dashboard-dagster-plan.md) | Completed | Racingline 策略组合发布、看板真实数据、Rearview daily run control plane、worker 全窗口重算和 Dagster 日运行资产；验收见 [2026-06-24 report](../jobs/reports/2026-06-24-racingline-strategy-portfolio-publish-dashboard-dagster.md) |
| [0050](archive/0050-racingline-strategy-simulation-position-step4-implementation-plan.md) | Completed | Racingline 策略创建 Step 4 模拟建仓：BacktestExecutionDraft、默认市场费率模板、stale gate、UI 语义收敛和 Step 5 contract handoff；验收见 [2026-06-23 report](../jobs/reports/2026-06-23-racingline-strategy-step4-draft-handoff.md) |
| [0051](archive/0051-racingline-strategy-backtest-step5-implementation-plan.md) | Completed | Racingline 策略回测 Step 5：异步 backtest run、transient signal materialization、NATS worker、ClickHouse 结果复用、真实 Step 5 UI、动态周期和 rerun 闭环；验收见 [2026-06-23 report](../jobs/reports/2026-06-23-racingline-strategy-step5-backtest.md) |
| [0049](archive/0049-racingline-strategy-step3-drift2-remediation-plan.md) | Completed | Racingline Step 3 股池预览二次漂移修正：板块展示、全复权 MA、量柱、动态近一年窗口、Step 3 权重微调和下钻性能优化；验收见 [2026-06-22 report](../jobs/reports/2026-06-22-racingline-strategy-step3-drift2-remediation.md) |
| [0048](archive/0048-racingline-strategy-step3-drift-remediation-plan.md) | Completed | Racingline Step 3 股池预览漂移修正：职责收缩、near-year timeline、10 条分页、K 线复权/MA 和 Step1/Step2 展示语义拆分；验收见 [2026-06-22 report](../jobs/reports/2026-06-22-racingline-strategy-step3-drift-remediation.md) |
| [0047](archive/0047-racingline-strategy-pool-preview-step3-implementation-plan.md) | Completed | Racingline 股池预览 Step 3：真实接口基线、PreviewSnapshot、结果解释、完整候选池分页、证券显示和 preview-only 个股上下文；验收见 [2026-06-22 report](../jobs/reports/2026-06-22-racingline-strategy-step3-preview.md) |
| [0046](archive/0046-racingline-strategy-weight-configuration-step2-implementation-plan.md) | Completed | Racingline 策略权重配置 Step 2：评分规则 adapter、Rearview preview-only API、真实股池预览和 `[0, 100]` score clamp；验收见 [2026-06-22 report](../jobs/reports/2026-06-22-racingline-strategy-step2-preview.md) |
| [0045](archive/0045-racingline-strategy-selection-step1-gap-closure-plan.md) | Completed | Racingline 策略选股 Step 1 缺口填补；验收见 [2026-06-21 report](../jobs/reports/2026-06-21-racingline-strategy-step1-gap-closure.md) |

## 规则

- 新计划文件命名：`NNNN-short-title.md`。
- 顶层计划必须包含 `日期：`、`状态：`、目标、非目标、实施阶段、验证命令和完成标准。
- 顶层允许状态：`Proposed`、`In Progress`、`Blocked`。
- `Completed`、`Superseded` 和历史参考计划应移入 [archive/](archive/)。
- 新增、归档或改名计划后，同步更新本索引。

## 历史说明

- `docs/plans/archive/0030-*` 保留了两个历史 Furnace 指标计划的原编号。它们已归档，不再参与 active plan 编号唯一性约束。

## 校验

```bash
make docs-check
git diff --check
```
