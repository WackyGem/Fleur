# System Maps

状态：当前事实入口（2026-06-13）

本目录按系统和产品线组织 mono-fleur 的当前事实地图。新需求先从本目录确认当前事实，再进入 `ADR/`、`RFC/`、`plans/`、`jobs/`、`design/` 或 `references/` 等对应生命周期目录。

使用方式：

1. 先从下方系统索引选择最接近的领域。
2. 再从系统地图确认代码根、职责边界、运行入口和质量门禁。
3. 跳转到相关 ADR、RFC、plan、job report、design 或 reference 文档。
4. 最后回到当前代码和测试验证事实，避免把历史设计当作当前实现。

## 系统索引

| 系统 | 当前代码根 | 事实地图 | 角色 |
|---|---|---|---|
| 数据平台 | `pipeline/scheduler/`、`pipeline/elt/` | [data-platform.md](data-platform.md) | Dagster 编排、S3 Parquet、ClickHouse raw sync、dbt 建模 |
| 数据治理 | `pipeline/contracts/`、`pipeline/contract_tools/` | [data-governance.md](data-governance.md) | raw 数据契约、字段字典、contract 生成与校验 |
| Furnace | `engines/crates/furnace*` | [furnace.md](furnace.md) | Rust 技术指标计算 CLI 和高性能写入路径 |
| Rearview | `engines/crates/rearview/` | [rearview.md](rearview.md) | Rust 规则选股 HTTP 服务 |
| Racingline | `app/racingline/` | [racingline.md](racingline.md) | Rearview 策略研究前端工作台 |
| 部署与运行 | `deploy/`、`pipeline/migrate/` | [deploy-ops.md](deploy-ops.md) | 本地基础设施、环境变量、迁移和运行记录入口 |

## 地图边界

系统地图只保留当前可导航事实：

- 代码根和主要入口。
- 职责与非职责。
- 上下游依赖。
- 常用运行命令和质量门禁。
- 相关当前文档和历史材料指针。
- 待决问题。

系统地图不应复制长篇设计、完整 API 规格、字段字典、执行计划或运行报告；这些内容应留在对应生命周期目录中，并从地图做简短链接。

## 维护规则

- 新增系统、应用、服务或长期子域时，先新增或更新本目录地图，再在 `docs/README.md` 挂入口。
- 修改系统职责、代码根、运行命令或质量门禁时，同步更新对应系统地图。
- 系统地图中的历史文档必须标明其角色，不能把 archive 中的计划当作当前事实。
- 如果同一约束在多个系统地图都需要出现，优先沉淀到 ADR、architecture、skill 或测试，再在系统地图中链接。
