# Plans

顶层只保留仍需行动的 active plans。完成、废弃或被替代的计划移入 [archive/](archive/)。

## Active Plans

| Plan | 状态 | 说明 |
|---|---|---|
| [0023](0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md) | Proposed | Contract-driven Parquet schema adapter 合入后的 dev 环境重置、小批量回填和全量回填准入计划 |
| [0036](0036-rust-rearview-stock-screening-service-implementation-plan.md) | Proposed | Rust Rearview 规则选股 HTTP 服务、PostgreSQL rearview 库、metric catalog 和 ClickHouse runtime join 的第一版实施计划 |

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
