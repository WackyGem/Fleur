# Docs Governance Inventory

日期：2026-06-10

状态：Completed

关联计划：

- `docs/plans/archive/0033-docs-governance-implementation-plan.md`

## 文件基线

执行时的 `docs/` 基线：

| 目录 | 文件数 |
|---|---:|
| `ADR/` | 10 |
| `Q&A/` | 1 |
| `RFC/` | 17 |
| `architecture/` | 2 |
| `debt/` | 1 |
| `design/` | 28 |
| `jobs/` | 10 |
| `optimize/` | 5 |
| `plans/` | 36 |
| `references/` | 70 |
| `skills/` | 8 |

总计：

- Markdown 文件：167
- 全部文件：188

## 发现

1. `docs/` 缺少总入口，只有 `AGENTS.md` 作为外层地图。
2. `docs/design/` 已有 dbt layer 文档，但不在 `AGENTS.md` 的文档入口中。
3. `docs/plans/` 顶层存在已完成计划和历史 Furnace 计划，active 与 archive 边界不清。
4. `docs/plans/` 顶层存在 `0030` 重号，但两个重号计划均已归档为历史材料。
5. `docs/jobs/` 同时包含 runbook、snapshot 和 reports，需要 README 明确边界。

## 已执行治理

1. 新增 `docs/README.md` 和关键目录 README。
2. 将已完成计划移入 `docs/plans/archive/`。
3. 顶层 `docs/plans/` 只保留 active plan。
4. 新增 `scripts/validate_docs_governance.py` 和 `make docs-check`。
5. 更新 `AGENTS.md` 与 `docs/skills/fleur-harness/SKILL.md` 的文档治理入口。

## 验证

```bash
make docs-check
git diff --check
```
