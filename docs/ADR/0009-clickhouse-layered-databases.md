# ADR 0009: ClickHouse 按数据层和计算产物分库

状态：Accepted

日期：2026-06-02

## 背景

ADR 0005 已决定 Dagster 负责 ClickHouse raw 同步，dbt 负责 staging/marts 建模。ADR 0007 进一步定义了 dbt staging 清洗边界：staging 只处理单一 raw source/table 内、确定性、低业务口径风险的清洗和标准化；跨源组合、实体归并、改变 grain 的结构进入 intermediate 或 marts。

当前实现仍存在历史命名：

- raw contract 中的 ClickHouse database 多数仍写作 `raw`。
- dbt profile 默认 schema 仍写作 `analytics`。
- data dictionary 文档仍展示 `raw.<table>`。

随着 staging、intermediate 和 marts 逐步成型，如果继续把所有模型放入同一个 ClickHouse database，只靠表名前缀表达层次，会削弱权限、生命周期、owner 和 dbt 路由边界。后续 Rust 计算引擎 `furnace` 还会直接物化技术指标等外部计算产物；这些产物既不是 raw contract，也不是 dbt model。需要把长期目标库名固定下来，作为后续迁移 contract、dbt profile、Dagster raw sync、Furnace 计算写入和文档生成的依据。

## 决策

ClickHouse 按数据建模层次和外部计算产物拆分为五个稳定 database：

| Database | Owner | 内容 | 默认消费者 |
| --- | --- | --- | --- |
| `fleur_raw` | Dagster raw sync + contract registry | S3 Parquet source objects 在 ClickHouse 中的 raw 物化副本 | dbt `source()` |
| `fleur_staging` | dbt | source-local canonical staging models | dbt intermediate/marts |
| `fleur_intermediate` | dbt | 跨 staging 组合、实体归并、业务过程中间结构、可复用但非最终消费模型 | dbt marts |
| `fleur_calculation` | Furnace / Dagster 外部计算引擎 | Rust 等外部计算引擎直接写入的计算产物、运行版本和参数化结果 | dbt `source()` + thin intermediate wrapper |
| `fleur_marts` | dbt | 面向分析、应用和 BI 的宽表、聚合表、指标表和稳定消费接口 | 查询用户、应用、BI、下游服务 |

长期数据流如下：

```text
Dagster source assets
  -> S3 Parquet source objects
  -> Dagster ClickHouse raw sync assets
  -> fleur_raw.*
  -> dbt staging models in fleur_staging.*
  -> dbt intermediate models in fleur_intermediate.*
  -> external compute outputs in fleur_calculation.*
  -> dbt sources over calculation outputs
  -> thin dbt intermediate wrappers
  -> dbt marts models in fleur_marts.*
```

具体约束：

- `fleur_raw` 只承载 contract scope 内的 ClickHouse raw tables。字段事实来自 `pipeline/contracts/datasets/*.yml`，表由 Dagster raw sync 写入或替换。
- dbt 不在 `fleur_raw` 中 materialize staging、intermediate 或 marts models。
- `fleur_staging` 只承载 dbt staging models。staging 仍遵守 ADR 0007：单 source/table、确定性、低业务口径风险，不做跨源 join/union、实体匹配、聚合或复杂口径计算。
- `fleur_intermediate` 承载 dbt intermediate models。这里可以组合多个 staging models、统一实体、改变 grain、处理跨源优先级和可复用业务过程，但不作为对外稳定接口。
- `fleur_calculation` 只承载外部计算引擎直接写入的计算产物，例如 `furnace` 计算的技术指标。该层不作为最终消费接口；dbt 必须先将该层表声明为 `source()`，再通过 thin intermediate wrapper 暴露稳定 `int_*` 语义接口。下游 marts 不直接读取 `fleur_calculation.*`。
- `fleur_marts` 承载 dbt marts models。这里表达稳定消费语义，可以使用表、增量表、聚合表、物化视图或 refreshable materialized view，具体 materialization 由查询 SLA、刷新成本和 ClickHouse 表设计决定。
- 不按数据源或供应商继续拆 database，例如不默认创建 `fleur_raw_jiuyan`、`fleur_staging_eastmoney`。`fleur_calculation` 是按 owner/计算产物边界拆出的例外，不是按数据源拆库。只有当权限、保留周期、成本归属、部署边界或合规要求确实按数据源分离时，才通过新的 ADR/RFC 评估例外。
- database 分层只表达治理和 owner 边界，不替代 ClickHouse 表级设计。每张 MergeTree 表仍必须独立规划 `ORDER BY`、partition、TTL、engine 和 materialization。
- 现有 `raw` 和 `analytics` database 视为迁移前状态，不再作为长期目标命名。

### 外部计算产物消费规则

对 `fleur_calculation.*` 这类非 dbt 写入的计算产物，统一采用三层接入方式：

1. **dbt source**：在 dbt `sources.yml` 中声明物理表，例如 `source('fleur_calculation', 'calc_stock_technical_indicators_daily')`。source 是 dbt DAG 的外部输入边界，承载源表文档、基础 source tests 和与物理 database/table 的映射。
2. **Dagster asset metadata**：由 Furnace/Dagster materialize 物理计算表，并记录运行 metadata，例如输入范围、输出行数、受影响分区、参数、状态来源和分区替换结果。dbt source 可通过 `meta.dagster.asset_key` 映射到对应 Dagster asset，使 Dagster 的 dbt 集成能展示跨工具 lineage。
3. **thin dbt wrapper model**：在 `fleur_intermediate` 中提供薄包装模型，例如 `int_stock_technical_indicators_daily`。wrapper 默认只做字段选择、命名稳定、轻量过滤和文档/tests，不重新实现 Furnace 指标公式。下游 marts 只能通过 `ref()` 消费 wrapper。

这三者职责不能互相替代：`source()` 解决 dbt 外部输入和 lineage；Dagster metadata 解决编排、观测和运行审计；thin wrapper 解决 dbt 分层语义和下游稳定契约。

## 依据

技术依据：

- ClickHouse database 是表的逻辑分组，也是 DDL、权限和对象命名的作用域。按层分库可以让 owner、grant、备份、迁移脚本和 dbt schema routing 更明确。
- `fleur_calculation` 让外部计算引擎产物与 dbt-owned intermediate/marts 分离，避免 `int_*` 命名被非 dbt 写入路径污染，也便于对计算产物设置独立权限、生命周期和重算策略。
- Per `schema-pk-plan-before-creation`，ClickHouse 性能关键仍是表级 `ORDER BY` 和 sparse index 设计，database 名称不会替代表设计。
- Per `schema-partition-lifecycle`，partition 主要服务 retention、archiving、tiered storage 和 `DROP PARTITION` 等生命周期管理；不能用 database 分层代替表级 retention 设计。
- Per `query-mv-incremental` 和 `query-mv-refreshable`，marts 层的实时聚合、复杂 join 缓存或低延迟消费可以通过合适的 materialized view 模式实现，但应按具体 mart 模型选择。
- Per `insert-batch-size` 和 `insert-mutation-avoid-update`，raw 写入策略仍应避免小批量插入和频繁 mutation；分库不改变 Dagster raw sync 的 staging/validate/replace 协议。

项目依据：

- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`
- `pipeline/contracts/README.md`
- `pipeline/elt/dbt_project.yml`
- `pipeline/elt/profiles.yml`

## 后果

- 后续 contract 中 `clickhouse_raw.database` 的目标值应迁移为 `fleur_raw`，并同步更新 generated dbt `sources.yml`、data dictionary 和 raw sync specs。
- dbt project 需要为 `staging`、`intermediate` 和 `marts` 配置不同 schema/database 路由，使模型按目录 materialize 到对应 ClickHouse database。
- 外部计算引擎直接写入的表应进入 `fleur_calculation`，例如 `fleur_calculation.calc_stock_technical_indicators_daily`；dbt 先将其声明为 source，再通过 `fleur_intermediate.int_*` thin wrapper 提供稳定接口。marts 不直接 `source()` 计算产物表。
- Dagster raw sync metadata、asset tags 和运行报告应记录 `fleur_raw.<table>`，避免继续输出历史 `raw.<table>`。
- 查询用户和应用默认只授予 `fleur_marts` 读取权限；开发和维护角色再按需授予 `fleur_staging`、`fleur_intermediate`、`fleur_calculation` 或 `fleur_raw` 权限。
- lineage 更清晰：`fleur_raw` 是 raw contract 边界，`fleur_staging` 是 canonical 字段边界，`fleur_intermediate` 是业务组合边界，`fleur_calculation` 是外部计算产物边界，`fleur_marts` 是消费接口边界。
- 跨库查询会让 SQL 中的 database/table 名更长，但这是可接受的显式性成本。
- 迁移时必须避免一次性破坏现有 raw sync 和 dbt build。推荐先让 dbt/contract 配置支持新库名，再通过定向 backfill/build 迁移表，最后清理旧 `raw`/`analytics` database。

## 迁移验收

完成本 ADR 的实现迁移后，应至少满足：

- `pipeline/contracts/datasets/*.yml` 中 ClickHouse raw database 统一为 `fleur_raw`。
- generated dbt raw `sources.yml` 指向 `fleur_raw`。
- dbt `staging` models materialize 到 `fleur_staging`。
- dbt `intermediate` models materialize 到 `fleur_intermediate`。
- dbt `marts` models materialize 到 `fleur_marts`。
- 外部计算引擎直接写入的计算产物 materialize 到 `fleur_calculation`。
- dbt 为 `fleur_calculation.*` 计算产物声明 source，并通过 `meta.dagster.asset_key` 或等价配置映射到对应 Dagster asset。
- dbt 为被 marts 消费的计算产物提供 `fleur_intermediate.int_*` thin wrapper；marts 通过 `ref()` wrapper 消费，不直接读取 `fleur_calculation.*`。
- generated data dictionary 不再展示历史 `raw.<table>` 作为目标 ClickHouse raw 表名。
- Dagster raw sync asset metadata 展示 `fleur_raw.<table>`。
- 最小验证命令：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
```

如果迁移触及 scheduler raw sync 代码，还应追加：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests
cd scheduler
uv run dg check defs
```

## 分类

- `fleur_raw` / `fleur_staging` / `fleur_intermediate` / `fleur_marts` 作为 dbt 与 raw 同步治理边界：derived。
- `fleur_calculation` 作为外部计算产物治理边界：field。该建议基于 mono-fleur 引入 Rust Furnace 计算引擎后的 owner、权限、重算和 dbt wrapper 需求；如外部计算引擎规模变化，应重新评估。
- `fleur_calculation.*` 采用 dbt source + Dagster asset metadata + thin dbt wrapper 的消费规则：derived / field。`source()` 与 `ref()` lineage 来自 dbt 和 Dagster dbt 集成的常规语义；强制 thin wrapper 是 mono-fleur 为保持 `fleur_intermediate` 分层契约、降低 marts 对物理计算表耦合而采用的项目规则。
- 不按数据源默认继续拆库：field。该建议是项目规模、权限模型和 dbt 运维复杂度下的启发式选择；若未来出现源级隔离需求，应重新评估。
- database 分层不替代表级 `ORDER BY`、partition、TTL 和 materialization 设计：official / derived。
