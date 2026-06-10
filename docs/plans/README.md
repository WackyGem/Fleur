# Plans

顶层只保留仍需行动的 active plans。完成、废弃或被替代的计划移入 [archive/](archive/)。

## Active Plans

| Plan | 状态 | 说明 |
|---|---|---|
| [0023](0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md) | Proposed | Contract-driven Parquet schema adapter 合入后的 dev 环境重置、小批量回填和全量回填准入计划 |
| [0034](0034-furnace-macd-technical-indicator-implementation-plan.md) | Proposed | 0035 前置任务：在 Furnace calculation 层新增 SMA 启动的 MACD(12,26,9) 计算、Dagster asset 和 dbt wrapper |
| [0035](0035-stock-technical-indicator-marts-implementation-plan.md) | Proposed | 股票技术指标 marts 实施计划：新增趋势指标、动量指标和成交量形指标三个 mart，并先补齐 MACD 上游缺口 |

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
