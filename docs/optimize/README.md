# Optimize

本目录保存质量扫描、维护性审计和治理建议的归档入口。

## 使用规则

- 新的当前问题、风险和下一阶段治理方向优先进入对应系统地图、RFC、plan 或 job report。
- 已关闭或已转为计划的质量扫描、维护性审计和治理建议放入 [archive/](archive/)。
- 不把 optimize 文档当作长期架构约束；长期规则进入 `docs/ADR/` 或 `docs/architecture/`。

## 归档入口

| 文档 | 用途 |
|---|---|
| [archive/0001-clickhouse-date-first-order-by-optimization.md](archive/0001-clickhouse-date-first-order-by-optimization.md) | ClickHouse 证券日频表 date-first `ORDER BY` 优化清单、范围和验收标准 |
| [archive/docs-governance-inventory-2026-06-10.md](archive/docs-governance-inventory-2026-06-10.md) | Plan 0033 执行时的 docs inventory 和治理基线 |
