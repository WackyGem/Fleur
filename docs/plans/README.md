# Plans

顶层只保留仍需行动的 active plans。完成、废弃或被替代的计划移入 [archive/](archive/)。

## Active Plans

| Plan | 状态 | 说明 |
|---|---|---|
| [0023](0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md) | Proposed | Contract-driven Parquet schema adapter 合入后的 dev 环境重置、小批量回填和全量回填准入计划 |
| [0041](0041-racingline-virtual-account-portfolio-rebalancing-implementation-plan.md) | Proposed | Racingline 虚拟账户、组合运行、NATS worker、明细账本和净值曲线第一版实施计划 |
| [0042](0042-chinabond-government-bond-s3-raw-implementation-plan.md) | Proposed | ChinaBond 国债收益率曲线 Dagster 年分区 S3 Parquet raw 接入与 2006-2026 回填计划 |
| [0043](0043-portfolio-data-plane-clickhouse-phase1-implementation-plan.md) | Proposed | 组合数据面迁移 ClickHouse 第一阶段：结果事实存储迁移、result_attempt_id 幂等重算、worker 切换写入目标、API 切换读取源 |
| [0044](0044-portfolio-performance-metrics-implementation-plan.md) | Proposed | 组合绩效指标、int/mart risk-free 与 benchmark 输入、metric config、worker 写入 fleur_calculation、dbt thin wrapper/ranking 和 closed trade ledger 实施计划 |

## Recently Completed

| Plan | 状态 | 说明 |
|---|---|---|
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
