# RFC 0014: ClickHouse 四层 database 改造与 raw 迁移验收设计

状态：Archived（2026-06-25；归档前状态：草案（2026-06-02））

## 摘要

ADR 0009 已接受 ClickHouse 按 dbt 建模层分为四个 database：

```text
fleur_raw
fleur_staging
fleur_intermediate
fleur_marts
```

本文档把该决策展开为概要设计：在不要求用户手工敲 ClickHouse DDL/DML 的前提下，让现有 fleur 项目从历史 `raw`/`analytics` 命名迁移到四层 database 结构。

核心方案：

1. **raw 层建库由 Dagster 项目实现**：`pipeline/scheduler` 的 ClickHouse raw sync 继续负责 raw 表创建、staging 表创建、分区替换和 snapshot 替换；目标 database 从历史 `raw` 迁移为 `fleur_raw`。
2. **staging/intermediate/marts 建库由 dbt 实现**：`pipeline/elt` 通过 dbt schema routing、model directory config 和必要的 dbt macro/hook，自动确保 `fleur_staging`、`fleur_intermediate`、`fleur_marts` 存在并承载对应模型。
3. **raw 迁移优先从 S3 source 重新物化**：现有 ClickHouse raw 数据迁移到 `fleur_raw` 的首选路径是重跑 Dagster ClickHouse raw sync assets，从 S3 Parquet 事实源重建目标库，而不是依赖用户手工 `CREATE TABLE AS SELECT`。
4. **验收必须覆盖数据一致性**：迁移完成后，对每张 raw 表做 schema hash、row count、partition count/range、抽样 checksum 或关键字段聚合校验，并输出 `docs/jobs/reports/` 运行报告。

## 关键点

- **关键点 1：source name 不等于物理 database。** dbt 中 `source('raw', '<dataset>')` 可以继续保留 source name `raw`，但 generated `sources.yml` 的 physical schema 应改为 `fleur_raw`。
- **关键点 2：raw database 创建在 Dagster raw sync 内完成。** 当前 `RawSyncService._prepare_staging()` 已调用 `CREATE DATABASE IF NOT EXISTS <spec.clickhouse_database>`；实现迁移时应让 contract/spec 传入 `fleur_raw`，不要求用户手动建库。
- **关键点 3：dbt 三层 database 不能靠 profile 默认 schema 混放。** `staging`、`intermediate`、`marts` 需要各自 materialize 到 `fleur_staging`、`fleur_intermediate`、`fleur_marts`。
- **关键点 4：迁移验收是设计的一部分。** 不能只看到 `fleur_raw` 有表就认为完成；必须比较旧 `raw` 与新 `fleur_raw`，或比较 S3/Dagster metadata 与 `fleur_raw`。
- **关键点 5：不把四层分库当性能优化。** database 只表达治理、owner、权限和生命周期边界；表级 `ORDER BY`、partition、engine、TTL 仍按 ClickHouse 表设计规则独立评估。

## 背景

当前项目已经形成以下边界：

```text
Dagster source assets
  -> S3 Parquet source objects
  -> Dagster ClickHouse raw sync assets
  -> ClickHouse raw tables
  -> dbt source()
  -> dbt staging models
  -> dbt intermediate / marts
```

相关长期决策：

- ADR 0005：Dagster 负责 ClickHouse raw 同步，dbt 负责建模。
- ADR 0007：dbt staging 只做 source-local、确定性、低业务口径风险的清洗和标准化。
- ADR 0008：新增或重写 staging 前必须先做 raw source profiling。
- ADR 0009：ClickHouse database 固定为 `fleur_raw`、`fleur_staging`、`fleur_intermediate`、`fleur_marts` 四层。

## 当前结构扫描

### Dagster / scheduler

当前 ClickHouse raw sync 已经存在：

```text
pipeline/scheduler/src/scheduler/defs/clickhouse/
├── assets.py        # build_clickhouse_raw_asset()
├── definitions.py   # CLICKHOUSE_RAW_JOBS
├── raw_sync.py      # RawSyncService
├── specs.py         # contract -> scheduler spec
└── sql.py           # ClickHouse SQL rendering
```

当前事实：

- `pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py` 的 `_prepare_staging()` 会执行：
  - `CREATE DATABASE IF NOT EXISTS <database>`
  - `CREATE TABLE IF NOT EXISTS <raw_table>`
  - `DROP TABLE IF EXISTS <stage_table>`
  - `CREATE TABLE <stage_table> AS <raw_table>`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py` 从 `fleur_contracts.adapters.clickhouse.build_scheduler_specs()` 构造 raw specs。
- `pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py` 已定义 raw sync jobs：
  - `clickhouse__raw_sync_snapshot_job`
  - `clickhouse__raw_sync_baostock_job`
  - `clickhouse__raw_sync_eastmoney_job`
  - `clickhouse__raw_sync_jiuyan_market_event_job`
  - `clickhouse__raw_sync_ths_market_event_job`

### Contract registry

当前 raw database 事实源在：

```text
pipeline/contracts/datasets/*.yml
```

当前多数数据集仍写作：

```yaml
clickhouse_raw:
  database: raw
```

生成和消费链路：

- `pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py` 把 `clickhouse_raw.database` 传给 scheduler raw specs。
- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py` 生成 `pipeline/elt/models/sources.yml`。
- `pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py` 生成 `docs/references/data_dict/*.md`。

### dbt / elt

当前 dbt 结构：

```text
pipeline/elt/
├── dbt_project.yml
├── profiles.yml
├── models/
│   ├── sources.yml
│   └── staging/
│       ├── baostock/
│       ├── eastmoney/
│       └── sina/
├── macros/
└── scripts/
```

当前事实：

- `pipeline/elt/profiles.yml` 默认 schema 为 `analytics`。
- `pipeline/elt/dbt_project.yml` 只配置了 `models.elt.staging.+materialized: view`。
- `pipeline/elt/models/sources.yml` 当前为：

```yaml
sources:
  - name: raw
    schema: raw
```

- 现有 staging SQL 通过 `source('raw', '<dataset>')` 读取 raw 表。
- 当前项目还没有 `models/intermediate/` 和 `models/marts/` 目录。

## 目标

1. 让 `fleur_raw` 成为 ClickHouse raw 层唯一长期目标 database。
2. 让 Dagster raw sync 自动创建 `fleur_raw` 和 raw/staging tables。
3. 让 dbt 自动创建或准备 `fleur_staging`、`fleur_intermediate`、`fleur_marts`。
4. 保留 dbt source name `raw`，但让它指向物理 `fleur_raw`。
5. 让现有 ClickHouse raw 数据迁移到 `fleur_raw`，并有可审计验收报告。
6. 不要求用户手工执行 ClickHouse `CREATE DATABASE`、`CREATE TABLE`、`INSERT SELECT` 或 `EXCHANGE TABLES`。
7. 给后续实施计划提供明确模块、阶段和验收标准。

## 非目标

1. 本 RFC 不实现代码。
2. 不在本 RFC 内新增新的业务 staging/intermediate/mart 模型。
3. 不改变 ADR 0005：raw sync 仍归 Dagster，dbt 不负责装载 raw。
4. 不把 `pipeline/contracts` 扩展到 dbt staging/intermediate/mart 字段事实。
5. 不在迁移中重新设计每张 raw 表的 `ORDER BY`、partition 或类型，除非现有 contract 已经错误。
6. 不默认删除历史 `raw` 或 `analytics` database；删除旧库必须在迁移验收报告接受后单独执行。
7. 不引入 Kafka、流式 ingest、ClickHouse dictionary 或语义层。

## 设计原则

### S3 是 raw 事实源

现有 raw 数据迁移的首选路径是通过 Dagster raw sync 从 S3 Parquet 重建 `fleur_raw`。这样迁移同时验证了：

- contract -> scheduler spec 是否正确；
- S3 object key 推导是否正确；
- ClickHouse raw sync staging/validate/replace 协议是否仍可恢复数据；
- `fleur_raw` 是否能独立承载当前 raw 层。

只有在某些历史 raw 表无法从 S3 完整重建时，才引入受控的 ClickHouse table-to-table copy 作为例外，并在运行报告中说明原因。

### Dagster 只负责 raw 层

Dagster 的职责：

- 创建 `fleur_raw`。
- 创建 raw table 和 staging table。
- 从 S3 读取 Parquet。
- 校验 staging。
- 替换 raw snapshot 或 year partition。
- 输出 materialization metadata 和迁移验收证据。

Dagster 不负责创建 `fleur_staging`、`fleur_intermediate`、`fleur_marts`，也不直接执行 dbt 模型建表。

### dbt 负责模型层 database

dbt 的职责：

- generated source catalog 指向 `fleur_raw`。
- staging models materialize 到 `fleur_staging`。
- intermediate models materialize 到 `fleur_intermediate`。
- marts models materialize 到 `fleur_marts`。
- 通过 dbt 自身的 schema creation 流程和必要 macro/hook 自动准备 database。

dbt source name `raw` 可以保留，因为它表达 source catalog 语义；物理 database 通过 `schema: fleur_raw` 表达。

### database 分层不替代表设计

ClickHouse database 是表的逻辑分组和权限/DDL 作用域。性能仍由表级设计决定：

- Per `schema-pk-plan-before-creation`：MergeTree `ORDER BY` 需要按查询模式规划，建表后修改成本高。
- Per `schema-partition-lifecycle`：partition 主要用于 retention、archiving 和批量替换，不是 database 分层的替代。
- Per `insert-batch-size`：raw sync 仍应避免小批量插入带来的 parts 压力。

## 目标架构

```text
S3 Parquet source objects
    |
    v
Dagster ClickHouse raw sync
    - contract-driven specs
    - CREATE DATABASE IF NOT EXISTS fleur_raw
    - CREATE/validate/replace raw tables
    |
    v
fleur_raw.*
    |
    v
dbt source('raw', ...)
    - source name: raw
    - physical schema/database: fleur_raw
    |
    v
fleur_staging.stg_*
    |
    v
fleur_intermediate.int_*
    |
    v
fleur_marts.*
```

## 改造范围

### Contract 改造

目标：

- 将所有 raw-enabled dataset 的 `clickhouse_raw.database` 从 `raw` 迁移为 `fleur_raw`。
- 保持 `clickhouse_raw.table`、`raw_asset_key` 和字段定义不变，避免同时改变表名和资产 key。

影响：

- `build_scheduler_specs()` 自动把 `fleur_raw` 传给 Dagster raw sync。
- generated `sources.yml` 的 `clickhouse_raw_table` metadata 应展示 `fleur_raw.<table>`。
- generated data dictionary 应展示 `fleur_raw.<table>`。

关键点：

- contract 只管理 raw 层物理事实，不新增 staging/intermediate/marts 字段事实。
- contract schema 不需要为了四层结构新增 dbt-owned 字段。

### Dagster raw sync 改造

目标：

- raw sync 使用 contract 中的 `fleur_raw` 作为目标 database。
- raw sync materialization metadata 输出 `clickhouse_database: fleur_raw`。
- raw sync jobs 能用于重建 `fleur_raw` 的 snapshot 表和 year partition 表。

建议补充：

- 新增一个迁移/验收导向的 Dagster job，选择所有 enabled ClickHouse raw sync assets；或者确认现有 jobs 能完整覆盖所有 raw-enabled specs。
- 对 year-partitioned raw tables，迁移需要覆盖当前已存在或应存在的 year partition range。
- 对 snapshot raw tables，迁移需要覆盖全部 snapshot assets。

关键点：

- `CREATE DATABASE IF NOT EXISTS fleur_raw` 应在 raw sync 执行路径中自然发生。
- 不在 Dagster definitions 加载阶段连接 ClickHouse 做建库或 schema discovery。
- 不让用户手工执行裸 SQL 建库。

### dbt source 改造

目标：

- 保持 source name：

```yaml
sources:
  - name: raw
```

- 物理 schema/database 改为：

```yaml
schema: fleur_raw
```

影响：

- 现有 staging SQL 中 `{{ source('raw', 'sina__trade_calendar') }}` 不需要改成 `source('fleur_raw', ...)`。
- raw profile 文档和脚本中的 `source('raw', ...)` 可以继续保留，但展示的 ClickHouse raw table metadata 需要更新为 `fleur_raw.<table>`。

关键点：

- source name 是 dbt lineage/catalog 名称，`schema` 才是 ClickHouse physical database。

### dbt model database routing

目标目录：

```text
pipeline/elt/models/
├── staging/
├── intermediate/
└── marts/
```

目标 routing：

| dbt 目录 | materialized 默认值 | ClickHouse database |
| --- | --- | --- |
| `models/staging/` | `view` | `fleur_staging` |
| `models/intermediate/` | `view` 或 `table`，按模型决定 | `fleur_intermediate` |
| `models/marts/` | `table` 或 `incremental`，按 SLA 决定 | `fleur_marts` |

设计约束：

- `dbt_project.yml` 应按目录配置 `+schema`。
- 如果 dbt-clickhouse 默认 schema 名拼接行为不能直接得到 `fleur_staging` 等完整 database 名，应通过 `generate_schema_name` macro 固定项目期望的 ClickHouse database 名。
- 如果 adapter 的 create schema 行为不足以覆盖空库初始化，应通过 dbt macro/hook 执行幂等 `CREATE DATABASE IF NOT EXISTS`。

关键点：

- dbt 三层建库通过 `uv run dbt parse/build/run` 触发，不要求用户手工建库。
- profile 默认 schema 不再作为业务层 database，避免继续混放到历史 `analytics`。

## raw 迁移策略

### 首选路径：从 S3 重新物化

迁移步骤概念上分为：

1. 将 contract raw database 目标改为 `fleur_raw`。
2. 运行 contract generate/check，更新 dbt source catalog 和 data dictionary。
3. 通过 Dagster raw sync jobs 重建 `fleur_raw`：
   - snapshot assets 运行一次；
   - year-partitioned assets 按目标 year range backfill；
   - compacted market-event assets 按 compacted year partitions backfill。
4. 对比旧 `raw` 与新 `fleur_raw`。
5. dbt parse/build staging，确认 `source('raw', ...)` 解析到 `fleur_raw`。
6. 生成迁移验收报告。

该路径符合 S3 作为 raw 事实源的原则，也能验证 raw sync 的恢复能力。

### 例外路径：ClickHouse 内部复制

仅当某个 raw table 缺少可重建的 S3 source、历史分区不完整或迁移窗口受限时，允许设计 table-to-table copy：

```sql
INSERT INTO fleur_raw.<table>
SELECT * FROM raw.<table>
```

该路径必须满足：

- 先由 Dagster 或受控脚本创建目标库和目标表；
- copy SQL 由代码路径生成和执行，不要求用户手敲；
- copy 前后做 row count、partition range 和 schema 校验；
- 在迁移报告中标明为何没有从 S3 重建。

## 迁移验收设计

迁移完成后必须生成：

```text
docs/jobs/reports/<YYYY-MM-DD>-clickhouse-layered-database-migration-report.md
```

报告至少包含：

- 执行日期和操作者。
- Git commit 或工作树状态。
- 迁移范围：datasets、snapshot tables、year-partitioned tables、partition range。
- 执行方式：Dagster raw sync rebuild 或 ClickHouse copy 例外。
- 每张表的校验结果。
- 失败、跳过、例外和后续处理。

### 表级验收项

| 检查 | snapshot table | year-partitioned table |
| --- | --- | --- |
| database exists | `fleur_raw` 存在 | `fleur_raw` 存在 |
| table exists | `fleur_raw.<table>` 存在 | `fleur_raw.<table>` 存在 |
| schema | `system.columns` 与 contract hash 一致 | `system.columns` 与 contract hash 一致 |
| row count | `fleur_raw` 与 S3 metadata 或旧 `raw` 一致 | 按 `year` 比较 row count |
| partition | `PARTITION BY tuple()` 或 snapshot 设计符合 spec | `year` 分区集合符合迁移范围 |
| sampling | 关键字段抽样一致 | 每个 year 抽样或聚合一致 |
| metadata | Dagster materialization 记录 `fleur_raw` | Dagster materialization 记录 `fleur_raw` 和 partition key |

### 推荐校验查询

数据库存在：

```sql
SELECT name, engine
FROM system.databases
WHERE name IN ('fleur_raw', 'fleur_staging', 'fleur_intermediate', 'fleur_marts');
```

raw 表清单：

```sql
SELECT database, name
FROM system.tables
WHERE database IN ('raw', 'fleur_raw')
ORDER BY database, name;
```

schema 校验：

```sql
SELECT name, type, position
FROM system.columns
WHERE database = 'fleur_raw'
  AND table = '<table>'
ORDER BY position;
```

行数对比：

```sql
SELECT 'old' AS side, count() AS rows FROM raw.<table>
UNION ALL
SELECT 'new' AS side, count() AS rows FROM fleur_raw.<table>;
```

年度分区对比：

```sql
SELECT 'old' AS side, year, count() AS rows
FROM raw.<table>
GROUP BY year
UNION ALL
SELECT 'new' AS side, year, count() AS rows
FROM fleur_raw.<table>
GROUP BY year
ORDER BY year, side;
```

这些 SQL 是验收逻辑的表达。最终实现时应由 Dagster asset、Python 验证脚本或受控 CLI 封装执行，不要求用户手工敲入。

## dbt 三层建库验收

dbt 改造完成后至少验证：

- `dbt parse` 成功。
- `dbt ls --select source:raw.*` 中 raw source 仍存在。
- `target/manifest.json` 中 raw source relation 指向 `fleur_raw`。
- staging models 的 database/schema 指向 `fleur_staging`。
- intermediate models 的 database/schema 指向 `fleur_intermediate`。
- marts models 的 database/schema 指向 `fleur_marts`。
- 执行定向 `dbt build --select staging` 后，ClickHouse 中存在 `fleur_staging.stg_*`。

如果当时还没有 intermediate/marts 实体模型，也应至少通过占位目录配置、manifest 校验或轻量 smoke model 验证 routing 规则；是否引入 smoke model 需在实施计划中单独决定，避免为了验收污染业务模型。

## 阶段设计

### Phase 0：事实基线

输出：

- 当前 raw-enabled datasets 清单。
- 当前 `raw.*` 表清单和 row count。
- 当前 dbt source/staging 清单。
- 当前 Dagster raw sync jobs 覆盖范围。

完成标准：

- 明确哪些 tables 必须迁移。
- 明确 year-partitioned tables 的 partition range。
- 明确是否存在无法从 S3 重建的例外表。

### Phase 1：contract 和 generated catalog 切换

输出：

- contract 中 raw database 目标改为 `fleur_raw`。
- generated `sources.yml` 指向 `fleur_raw`。
- generated data dictionary 展示 `fleur_raw.<table>`。

完成标准：

- `uv run fleur-contracts validate` 通过。
- `uv run fleur-contracts generate --check` 通过。
- scheduler raw specs 加载到 `fleur_raw`。

### Phase 2：Dagster raw 层自动建库与重建

输出：

- `fleur_raw` 由 raw sync 执行路径自动创建。
- snapshot raw tables 重建到 `fleur_raw`。
- year-partitioned raw tables 按范围重建到 `fleur_raw`。

完成标准：

- 所有 enabled raw sync assets 成功 materialize。
- Dagster metadata 展示 `clickhouse_database=fleur_raw`。
- `fleur_raw` 表清单覆盖 contract raw-enabled datasets。

### Phase 3：dbt 三层 database routing

输出：

- `staging` -> `fleur_staging`
- `intermediate` -> `fleur_intermediate`
- `marts` -> `fleur_marts`
- 必要时新增 dbt macro/hook 保证幂等建库。

完成标准：

- `uv run dbt parse --project-dir elt --profiles-dir elt` 通过。
- `uv run dbt build --project-dir elt --profiles-dir elt --select staging` 通过。
- ClickHouse 中可观察到 `fleur_staging` 及当前 staging views。

### Phase 4：迁移验收报告

输出：

- `docs/jobs/reports/<date>-clickhouse-layered-database-migration-report.md`

完成标准：

- 每个 raw table 的 schema、row count、partition、抽样校验有结果。
- 例外路径有原因和后续处理。
- 明确旧 `raw` / `analytics` database 是否保留、冻结或等待清理。

## 风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| `source('raw', ...)` 被误改为 `source('fleur_raw', ...)` | 破坏既有 staging SQL 和 profiling 文档 | 保留 source name `raw`，只改 physical schema |
| dbt schema 名被 target schema 拼接 | 表落到非预期 database | 使用 `generate_schema_name` macro 或 adapter 验证锁定完整库名 |
| 只迁移表结构未迁移数据 | 下游 staging 空表或数据缺失 | raw 迁移验收必须包含 row count 和 partition 对比 |
| 从 S3 重建发现历史对象缺失 | 部分 raw 表无法恢复 | 标记例外，使用受控 ClickHouse copy 或补 S3 backfill |
| 一次性删除旧 `raw` | 回滚困难 | 旧库只冻结不删除，清理另开计划 |
| materialized view / table engine 设计不匹配查询 | marts 性能不可控 | marts 层按模型 SLA 独立设计 engine/order_by/materialization |

## 开放问题

- 是否需要新增一个专门的 `clickhouse__raw_sync_all_job`，还是现有分组 jobs 足够覆盖迁移？
- year-partitioned raw tables 的迁移范围以旧 `raw` 中已有 `year` 为准，还是以 S3 object listing 为准？
- 是否要保留 `analytics` 作为临时开发 schema，还是完全停止使用？
- dbt 三层 database 自动创建优先依赖 dbt adapter 的 schema creation，还是显式新增 `create_layer_databases` macro/hook？
- 迁移验收脚本应放在 `pipeline/contract_tools`、`pipeline/scheduler`，还是 `pipeline/elt/scripts`？初步倾向：raw 层验收靠 `contract_tools` 或 scheduler-adjacent 工具，dbt routing 验收靠 `pipeline/elt/scripts`。

## 参考

- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`
- `docs/ADR/0009-clickhouse-layered-databases.md`
- `docs/RFC/archive/0009-dagster-clickhouse-raw-sync.md`
- `docs/RFC/archive/0012-dbt-field-glossary-and-raw-source-governance.md`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- `pipeline/elt/dbt_project.yml`
- `pipeline/elt/profiles.yml`

外部文档依据：

- ClickHouse docs：database 是 tables 的 logical grouping，支持 `CREATE DATABASE IF NOT EXISTS`。
- dbt Core docs：model directory config 支持 `+schema`，dbt run 会准备 schemas 并执行 hooks。
- dbt-clickhouse docs：ClickHouse profile 使用 `schema` 表达目标 database，模型支持 ClickHouse engine/order_by/partition_by 等配置。
- Dagster docs：`define_asset_job` 可定义 selected assets 的物化 job，`dg launch --assets/--job` 支持本地运行 selected assets/jobs 和 partitions。

## 分类

- raw database creation by Dagster raw sync：derived。
- dbt database routing for staging/intermediate/marts：derived。
- raw migration by re-materializing from S3 source：field，基于本项目 S3 是 raw 事实源的架构选择。
- database 分层不替代表级 ClickHouse schema/order/partition 设计：official / derived。
