# RFC 0009: Dagster 调度 ClickHouse raw 层同步设计

状态：草案（2026-05-31）

## 摘要

本文档定义 `pipeline/scheduler` 将 S3 Parquet source assets 同步到 ClickHouse raw 层的具体设计。RFC 0007 已确定 raw 层采用 ClickHouse `MergeTree` table 物化、由 Dagster 刷新；本 RFC 补足当前 Dagster 资产结构下的执行协议。

核心决策：

1. **S3 仍是 raw 数据事实源**：Dagster 采集资产先写入 S3 Parquet，ClickHouse raw 表是面向查询和 dbt 的物化副本。
2. **年度分区资产使用分区替换**：Dagster 每次只把被调度的 `year` 分区加载到 staging 表，校验后执行 ClickHouse 分区替换。
3. **日分区稀疏资产不直接进入 ClickHouse**：`jiuyan__action_field`、`ths__limit_up_pool` 只作为 compact 输入；ClickHouse raw 层消费其 compacted 年度资产。
4. **最新快照资产使用完整快照替换协议**：交易日历、证券基础信息、行业列表、OCR 结果快照等无分区资产按整表快照加载，不暴露半成品。
5. **ClickHouse 同步是独立 Dagster asset 层**：新增 raw sync assets 依赖已有 S3 assets，dbt staging/marts 只依赖 ClickHouse raw，不直接读取 S3。
6. **第一阶段使用显式 spec + asset factory**：在当前 `SourceBundle` 代码架构下，用静态 raw table specs 生成 Dagster assets；不在 definitions 加载阶段查询 ClickHouse 外部状态。

## 背景

当前 scheduler 通过 `SourceBundle` 聚合各数据源 definitions。以 `cd pipeline/scheduler && uv run dg list defs --json` 核验，当前基线为：

| 类型 | 当前内容 |
|------|----------|
| Assets | 19 个，全部在 group `s3_sources` |
| Jobs | 10 个 source jobs |
| Schedules | 7 个 source schedules |
| Resources | 8 个，无 ClickHouse resource |
| Sensors | 2 个，包括 `default_automation_condition_sensor` 和 `slack_asset_failure_sensor` |

现有 S3 存储契约来自 ADR 0002：

| storage mode | 路径形态 | 语义 |
|--------------|----------|------|
| `latest_snapshot` | `source/{asset_key}/000000_0.parquet` | 每次 materialize 覆盖最新快照 |
| `partitioned` | `source/{asset_key}/{partition_key_name}={partition_key}/000000_0.parquet` | 每次 materialize 写入 Dagster 选中的分区目录 |

当前对 ClickHouse raw 同步最重要的资产形态：

| 类别 | 资产 | 当前特点 | ClickHouse 同步策略 |
|------|------|----------|---------------------|
| 最新快照 | `source/sina__trade_calendar` | 无 Dagster 分区，S3 最新快照 | 整表快照替换 |
| 最新快照 | `source/baostock__query_stock_basic` | 无 Dagster 分区，S3 最新快照 | 整表快照替换 |
| 最新快照 | `source/jiuyan__industry_list` | 无 Dagster 分区，S3 最新快照 | 整表快照替换 |
| 最新快照 | `source/jiuyan__industry_ocr_snapshot` | 无 Dagster 分区，S3 最新快照，依赖 OCR work-queue processor | 整表快照替换 |
| 年度分区 | `source/baostock__query_history_k_data_plus_daily` | `year` 分区，每日调度当前年并传入 `refresh_until_trade_date` | 替换对应 `year` 分区 |
| 年度分区 | `source/eastmoney__*` | 多个 F10 资产，`year` 分区，每日调度当前自然年 | 替换对应 `year` 分区 |
| 日分区稀疏 | `source/jiuyan__action_field` | `trade_date` 日分区，允许空分区 | 不直接同步 |
| 日分区稀疏 | `source/ths__limit_up_pool` | `trade_date` 日分区，允许空分区 | 不直接同步 |
| 年度 compact | `source/jiuyan__action_field_compacted` | 由日分区 compact 为 `year` 分区，`eager()` 自动化 | 替换对应 `year` 分区 |
| 年度 compact | `source/ths__limit_up_pool_compacted` | 由日分区 compact 为 `year` 分区，`eager()` 自动化 | 替换对应 `year` 分区 |

以下资产不直接作为 ClickHouse raw 同步输入：

| 资产 | 原因 |
|------|------|
| `source/jiuyan__industry_images` | 图片对象与 PostgreSQL 状态资产，不是 Parquet 表 |
| `source/jiuyan__industry_ocr` | OCR work-queue processor，不是稳定发布表；ClickHouse raw 同步消费其下游 `source/jiuyan__industry_ocr_snapshot` |

## 目标

1. 让 ClickHouse raw 表稳定物化当前 S3 Parquet source 数据。
2. 支持年度分区资产按 `year` 幂等刷新，尤其是当前年每日重刷和历史年 backfill。
3. 支持最新快照资产整表刷新，避免下游读取半成品。
4. 保持 Dagster asset lineage 可读：S3 source asset -> ClickHouse raw sync asset -> dbt staging/marts。
5. 在失败时保持 ClickHouse raw 旧版本可用。
6. 为后续 dbt `source()`、staging view 和 marts table 提供稳定 raw 表边界。

## 非目标

1. 不在本 RFC 中实现代码。
2. 不在第一阶段引入流式秒级 ingest、Kafka engine 或 ClickHouse materialized view ingest。
3. 不让 dbt 直接读取 S3 Parquet。
4. 不把日分区稀疏资产直接装载到 ClickHouse raw 后再在 ClickHouse 内 compact。
5. 不在 Dagster definitions 加载阶段连接 ClickHouse 或 S3 做远端发现。
6. 不在 raw 同步里做业务清洗、字段重命名或 mart 聚合。

## 设计原则

### S3 是事实源，ClickHouse raw 是物化副本

Dagster source assets 负责从 API/TCP 拉取数据并写入 S3 Parquet。ClickHouse raw sync assets 只负责把已经物化的 S3 数据装载到 ClickHouse。任何 ClickHouse raw 表损坏，都应能通过重跑 Dagster 分区或快照资产从 S3 恢复。

### 分区用于替换和生命周期，不替代排序键

年度资产的 ClickHouse raw 表应使用低基数分区：

```sql
PARTITION BY year
```

这是为了和 Dagster/S3 的 `year=YYYY` 分区、年度回填、当前年每日替换对齐。查询性能仍主要依赖 `ORDER BY`，例如：

```sql
ORDER BY (code, trade_date)
```

或按实际查询模式调整为：

```sql
ORDER BY (trade_date, code)
```

该原则对应 ClickHouse 规则：

- Per `schema-partition-low-cardinality`：避免高基数分区，年度分区数量可控。
- Per `schema-partition-lifecycle`：分区主要用于生命周期和批量替换。
- Per `schema-pk-plan-before-creation`：`ORDER BY` 建表后不可轻易修改，必须在首批 raw 表 DDL 前确定。
- Per `schema-pk-prioritize-filters`：排序键应来自下游常用过滤条件。

### 用 staging 表隔离失败

ClickHouse raw sync 必须先写 staging，再替换生产 raw 表。失败发生在 staging 装载、schema 校验、row count 校验或分区校验阶段时，生产 raw 表保持旧版本。

禁止默认使用：

```sql
ALTER TABLE raw_x DROP PARTITION ...
INSERT INTO raw_x ...
```

该模式在失败时可能让 raw 表出现空洞。默认使用 staging + 分区替换或 staging + 整表替换。

## 目标架构

```text
Dagster source assets
  - API/TCP -> PyArrow Table
  - S3 Parquet latest_snapshot/year/trade_date
       |
       v
Dagster compact assets
  - daily trade_date -> yearly parquet
  - only for sparse daily market-event sources
       |
       v
Dagster ClickHouse raw sync assets
  - read upstream S3 path and metadata
  - load ClickHouse staging table
  - validate schema/count/partition
  - replace raw table partition or snapshot
       |
       v
ClickHouse raw database
  - MergeTree/ReplicatedMergeTree tables
  - native ClickHouse types
  - bounded partitions aligned with S3/Dagster
       |
       v
dbt staging/marts
  - source() references ClickHouse raw
  - staging views for rename/cast/light cleaning
  - marts tables/incremental models for query workloads
```

## Dagster 模块设计

推荐新增模块：

```text
pipeline/scheduler/src/scheduler/defs/
├── clickhouse/
│   ├── __init__.py
│   ├── assets.py              # raw sync asset factory
│   ├── definitions.py         # CLICKHOUSE_RAW_ASSETS / jobs
│   ├── protocols.py           # ClickHouse client protocol
│   ├── raw_sync.py            # sync service and validation flow
│   ├── specs.py               # static raw table specs
│   └── sql.py                 # SQL rendering helpers
├── resources/
│   └── clickhouse.py          # ClickHouseResource
└── config/
    └── env.py                 # CLICKHOUSE_* env vars
```

模块职责：

| 文件 | 职责 | 禁止事项 |
|------|------|----------|
| `clickhouse/specs.py` | 静态声明 S3 asset 到 ClickHouse raw 表的映射、分区策略、排序键、schema 版本 | 不连接 ClickHouse，不读取环境变量 |
| `clickhouse/assets.py` | 根据 specs 生成 Dagster raw sync assets | 不写 SQL 字符串细节 |
| `clickhouse/raw_sync.py` | 编排 staging 装载、校验、替换、metadata | 不导入具体 source business modules |
| `clickhouse/sql.py` | 渲染 ClickHouse DDL/DML 片段 | 不执行 SQL |
| `resources/clickhouse.py` | 构造 ClickHouse client，集中管理连接配置 | 不包含业务资产映射 |
| `definitions.py` | 注册 ClickHouse resource、raw sync assets、raw sync jobs | 不包含装载流程细节 |

当前项目 `pyproject.toml` 已配置 `scheduler.components.*` registry，但现有 scheduler definitions 仍以 Python assets 和 `SourceBundle` 为主。第一阶段采用 asset factory 以降低迁移面；如果后续项目开始使用 Dagster Components，可将 `specs.py + assets.py` 封装为 `ClickHouseRawSyncComponent`，保持同步协议不变。

## Definitions 装配

ClickHouse raw sync assets 是跨数据源的下游物化层，不应塞入任一 source bundle。推荐在 `scheduler.defs.definitions.defs()` 中显式追加：

```python
return dg.Definitions(
    assets=[
        *bundle_assets(SOURCE_BUNDLES),
        *CLICKHOUSE_RAW_ASSETS,
    ],
    jobs=[
        *bundle_jobs(SOURCE_BUNDLES),
        *CLICKHOUSE_RAW_JOBS,
    ],
    schedules=bundle_schedules(SOURCE_BUNDLES),
    resources={
        "s3_io_manager": S3IOManager(),
        "s3_settings": S3SettingsResource(),
        "clickhouse": ClickHouseResource(),
        ...
    },
)
```

第一阶段不为 ClickHouse raw sync 新增 schedule。raw sync assets 通过依赖和 automation condition 跟随上游 S3 assets：

| 上游类型 | raw sync 触发 |
|----------|---------------|
| 最新快照 source asset | 上游快照 materialize 后触发 |
| 年度分区 source asset | 对同一个 `year` partition 触发 |
| 年度 compact asset | compacted `year` partition 完成后触发 |

为便于手动回填和验证，应同时提供定向 asset job，例如：

| Job | Selection |
|-----|-----------|
| `clickhouse__raw_sync_all_job` | 所有 ClickHouse raw sync assets |
| `clickhouse__raw_sync_baostock_job` | BaoStock raw sync assets |
| `clickhouse__raw_sync_market_event_job` | compacted Jiuyan/THS raw sync assets |

## Raw Table Spec

每个同步资产由一个静态 spec 描述。建议字段：

| 字段 | 说明 |
|------|------|
| `source_asset_key` | 上游 S3 asset key |
| `raw_asset_key` | Dagster 中表示 ClickHouse raw 表的 asset key，例如 `clickhouse/raw/baostock__query_history_k_data_plus_daily` |
| `storage_mode` | `latest_snapshot` 或 `partitioned` |
| `source_partition_key_name` | S3 hive 分区字段，年度资产为 `year` |
| `clickhouse_database` | 默认 `raw` |
| `clickhouse_table` | raw 表名 |
| `staging_table` | staging 表名或命名模板 |
| `partition_strategy` | `snapshot` 或 `year` |
| `order_by` | ClickHouse `ORDER BY` 列表达式 |
| `columns` | ClickHouse 列定义，与 Parquet schema 对齐 |
| `allow_empty` | 是否允许 0 行分区进入 raw |
| `sync_enabled` | 控制灰度接入 |

示例：

```python
ClickHouseRawTableSpec(
    source_asset_key=dg.AssetKey(["source", "baostock__query_history_k_data_plus_daily"]),
    raw_asset_key=dg.AssetKey(["clickhouse", "raw", "baostock__query_history_k_data_plus_daily"]),
    storage_mode="partitioned",
    source_partition_key_name="year",
    clickhouse_database="raw",
    clickhouse_table="baostock__query_history_k_data_plus_daily",
    partition_strategy="year",
    order_by=("code", "trade_date"),
    allow_empty=False,
)
```

`columns` 的最终类型应直接以 `docs/references/data_dict/` 中对应资产的 PyArrow 类型为准。若某个资产尚未补齐 data_dict，先补齐 data_dict，再进入 ClickHouse raw spec 和实现；不要把 `String` 作为默认回退方案。

## 同步协议：年度分区

适用资产：

- `source/baostock__query_history_k_data_plus_daily`
- `source/eastmoney__*`
- `source/jiuyan__action_field_compacted`
- `source/ths__limit_up_pool_compacted`

协议：

1. Dagster raw sync asset 获取当前 `context.partition_key`，必须是四位年份。
2. 根据 `source_asset_key`、`source_partition_key_name="year"`、`partition_key` 推导 S3 Parquet 路径。
3. 创建或清空 staging 表。staging 表结构必须与 raw 表兼容。
4. 从 S3 Parquet 装载该年度文件到 staging 表。
5. 如果 Parquet 文件不包含 `year` 列，则在 `INSERT SELECT` 中使用 Dagster partition key 注入常量 `year`。
6. 校验 staging 表：
   - `count()` 与上游 materialization metadata 的分区行数一致；
   - `min(year) = max(year) = partition_key`；
   - raw 表目标列均存在，类型可兼容；
   - 不允许空表，除非 spec `allow_empty=True`；
   - 可选：关键业务列非空比例、日期范围、证券代码格式。
7. 校验通过后执行分区替换：

```sql
ALTER TABLE raw.<table>
REPLACE PARTITION <year>
FROM raw.<staging_table>;
```

8. 清理 staging 表或保留最近一次 staging 供排查。
9. Dagster materialization metadata 记录：
   - `clickhouse_database`
   - `clickhouse_table`
   - `partition_key`
   - `s3_object_key`
   - `loaded_row_count`
   - `raw_partition_row_count`
   - `schema_hash`
   - `replace_partition_seconds`

失败语义：

| 失败阶段 | raw 表状态 |
|----------|------------|
| S3 文件不存在 | 保持旧版本 |
| staging INSERT 失败 | 保持旧版本 |
| schema 校验失败 | 保持旧版本 |
| row count 校验失败 | 保持旧版本 |
| `REPLACE PARTITION` 失败 | ClickHouse 保证不应暴露部分替换；Dagster run 失败并告警 |

## 同步协议：最新快照

适用资产：

- `source/sina__trade_calendar`
- `source/baostock__query_stock_basic`
- `source/jiuyan__industry_list`
- `source/jiuyan__industry_ocr_snapshot`

协议：

1. 根据 `source_asset_key` 推导固定 S3 Parquet 路径。
2. 装载到 snapshot staging 表。
3. 校验 staging 行数、schema、主键候选唯一性。
4. 使用完整快照替换生产 raw 表。

实现有两个可选方案：

| 方案 | 说明 | 选择 |
|------|------|------|
| staging + table swap | staging 校验后与生产表交换或重命名 | 推荐用于小型快照表，需确认当前 ClickHouse database engine 支持的原子能力 |
| 单分区 raw 表 + partition replace | 将快照表设计为单分区表，沿用 staging + replace 语义 | 可作为统一实现，但实现前需验证 ClickHouse 具体语法 |

第一阶段建议先在实现计划中验证 ClickHouse 版本和 database engine，再最终选择 snapshot 替换 SQL。无论采用哪种 SQL，外部协议保持一致：只有 staging 校验通过后才替换生产 raw 表。

`source/jiuyan__industry_ocr_snapshot` 按最新快照处理，不按 OCR 图片、文章日期或运行日期做增量同步。该资产已把可重跑、失败、pending、stale running 等 OCR 状态收敛为稳定 Parquet 快照；ClickHouse raw 层只接收发布后的结果行。对这类小规模、可变状态输出使用全表快照替换，是从 ClickHouse 避免频繁 `ALTER TABLE UPDATE/DELETE` mutation 的规则推导出的 workload 决策；当前规模约 3 万行，整表替换成本低于维护增量去重或 ReplacingMergeTree 语义。

## S3 路径和分区列

现有 S3 writer 通过路径表达 hive 分区：

```text
source/{asset_key}/year=2026/000000_0.parquet
```

但 Parquet 文件内容不一定包含 `year` 列。ClickHouse raw 表如果需要 `year` 作为 `PARTITION BY year`，raw sync 负责在装载时补充：

```sql
INSERT INTO raw.<staging_table> (<columns>, year)
SELECT
  <columns>,
  toUInt16(2026) AS year
FROM s3(...);
```

对于 compacted 日分区资产，Parquet 内容通常应保留业务日期列，如 `trade_date`；`year` 只作为 ClickHouse 分区管理列，不替代业务日期。

## ClickHouse DDL 规范

年度 raw 表默认形态：

```sql
CREATE TABLE raw.<table>
(
    ...,
    year UInt16
)
ENGINE = MergeTree
PARTITION BY year
ORDER BY (<query_driven_order_by>);
```

staging 表默认形态：

```sql
CREATE TABLE raw.<table>__stage
AS raw.<table>;
```

约束：

1. raw 表必须使用原生类型；不要把所有列建成 `String`，除非上游 Parquet 暂未完成类型优化。
2. `Nullable` 只在语义必要时使用；默认值能表达未知值时优先用非 Nullable + DEFAULT。
3. `ORDER BY` 必须在建表前为每张 raw 表单独确定。
4. 不在常规同步流程中执行 `OPTIMIZE TABLE ... FINAL`。
5. 不使用频繁 `ALTER TABLE UPDATE/DELETE` 修补 raw 数据；需要修正时重刷对应 S3 分区再替换 ClickHouse 分区。

这些约束对应 ClickHouse 规则：

- `schema-types-native-types`
- `schema-types-avoid-nullable`
- `schema-pk-plan-before-creation`
- `insert-mutation-avoid-update`
- `insert-mutation-avoid-delete`
- `insert-optimize-avoid-final`

## dbt 边界

dbt 不负责装载 S3，不直接调用 ClickHouse `REPLACE PARTITION`。dbt 项目只看到稳定的 ClickHouse raw tables：

```text
source('raw', 'baostock__query_history_k_data_plus_daily')
source('raw', 'eastmoney__income_ytd')
source('raw', 'jiuyan__action_field_compacted')
source('raw', 'jiuyan__industry_ocr_snapshot')
```

推荐分层：

| 层 | 物化 | 职责 |
|----|------|------|
| ClickHouse raw | table | S3 Parquet 的物化副本，保留源字段和装载分区列 |
| dbt staging | view | 类型收敛后的字段命名、轻清洗、基础过滤 |
| dbt marts | table/incremental | 面向查询的宽表、聚合、排序键优化 |

dbt staging 不应承担大规模重复计算。如果某个 staging view 被多个 marts 反复扫描且成本高，应提升为 intermediate table 或 mart 级 table。

## 自动化与回填

### 当前年每日刷新

以 BaoStock 为例：

```text
baostock__daily_schedule
  -> partition_key = "2026"
  -> source/baostock__query_history_k_data_plus_daily year=2026
  -> clickhouse/raw/baostock__query_history_k_data_plus_daily year=2026
```

ClickHouse 每天只替换 `year=2026` 分区。历史年份不受影响。

### 历史年份 backfill

对年度资产执行 Dagster partition backfill 时，ClickHouse raw sync asset 应与上游使用相同 partition key。每个年份独立 staging、独立校验、独立替换。

### 日分区 compact 后同步

以 `ths__limit_up_pool` 为例：

```text
source/ths__limit_up_pool trade_date=2026-05-29
  -> source/ths__limit_up_pool_compacted year=2026
  -> clickhouse/raw/ths__limit_up_pool_compacted year=2026
```

ClickHouse raw 层只消费 compacted 年度资产，避免在 ClickHouse 内处理稀疏日分区缺失、空分区和交易日过滤。

### OCR processor 后同步

`jiuyan__industry_ocr` 仍是状态型 processor asset，负责处理新发现、pending、failed 和 stale running 的图片。`jiuyan__industry_ocr_snapshot` 是其下游稳定发布资产：

```text
source/jiuyan__industry_ocr
  -> source/jiuyan__industry_ocr_snapshot
  -> clickhouse/raw/jiuyan__industry_ocr_snapshot
```

ClickHouse raw sync 不读取 OCR processor 的中间状态，也不直接读取每张图片的 OCR 结果文件；它只读取 snapshot asset 发布的 `source/jiuyan__industry_ocr_snapshot/000000_0.parquet`。

## 初始接入顺序

推荐按风险从低到高接入：

| 阶段 | 范围 | 完成标准 |
|------|------|----------|
| 1 | `baostock__query_history_k_data_plus_daily` 年度分区 | 能将单个 `year` 从 S3 加载到 ClickHouse staging 并替换 raw 分区 |
| 2 | `sina__trade_calendar`、`baostock__query_stock_basic`、`jiuyan__industry_list` 最新快照 | 明确 snapshot 替换 SQL，并完成小表同步 |
| 3 | `jiuyan__industry_ocr_snapshot` 最新快照 | OCR processor 完成后发布稳定快照，并按 snapshot 替换协议同步 ClickHouse |
| 4 | `jiuyan__action_field_compacted`、`ths__limit_up_pool_compacted` | compacted 年度资产完成后自动同步 ClickHouse |
| 5 | `eastmoney__*` 年度资产 | 批量 spec 生成，按 `docs/references/data_dict/` 的 PyArrow 类型生成列定义 |
| 6 | dbt raw source/staging/marts | dbt 从 ClickHouse raw 读取，不再直接依赖 S3 |

## 测试策略

### 单元测试

| 测试 | 目的 |
|------|------|
| raw table spec validation | 确认 source asset、raw asset、partition strategy、table name 合法 |
| S3 object key rendering | 确认 latest snapshot 和 `year=YYYY` 路径推导与 ADR 0002 一致 |
| SQL rendering | 确认 staging insert、validation query、replace SQL 不拼错表名和分区 |
| sync service success path | 使用 fake ClickHouse client 验证执行顺序 |
| sync service failure path | staging/validation 失败时不执行 replace |
| daily sparse exclusion | 确认 `jiuyan__action_field`、`ths__limit_up_pool` 不直接生成 raw sync spec |
| OCR processor exclusion | 确认 `jiuyan__industry_ocr` 不直接生成 raw sync spec，`jiuyan__industry_ocr_snapshot` 生成 latest snapshot spec |

### Definitions 测试

更新现有 definitions 集成测试，确认：

- 新增 `clickhouse` resource；
- 新增 ClickHouse raw sync assets；
- 现有 source assets、jobs、schedules 不被破坏；
- `default_automation_condition_sensor` 保持存在；
- 如果 raw sync 使用 `eager()`，automation sensor 能识别新增自动化资产。

### 最小验证命令

文档或 spec 变更：

```bash
git diff --check
```

实现 raw sync 代码后：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

## 运行观测

每个 ClickHouse raw sync materialization 应记录：

| metadata | 说明 |
|----------|------|
| `clickhouse_database` | 目标 database |
| `clickhouse_table` | 目标 raw table |
| `staging_table` | 本次 staging 表 |
| `storage_mode` | latest snapshot / partitioned |
| `partition_key_name` | 年度资产为 `year` |
| `partition_key` | 分区运行时为年份；快照为空 |
| `s3_object_key` | 输入 parquet 对象 |
| `loaded_row_count` | staging 装载行数 |
| `raw_row_count_after_replace` | 替换后目标范围行数 |
| `schema_hash` | spec schema 或 ClickHouse columns hash |
| `clickhouse_insert_seconds` | staging 装载耗时 |
| `clickhouse_validation_seconds` | 校验耗时 |
| `clickhouse_replace_seconds` | 替换耗时 |

失败日志必须包含 asset key、partition key、S3 object key、ClickHouse table 和失败 SQL 的安全摘要。不要在日志中打印 ClickHouse 密码、S3 secret 或完整连接串。

## 安全和配置

新增环境变量建议：

| 变量 | 用途 |
|------|------|
| `CLICKHOUSE_HOST` | ClickHouse host |
| `CLICKHOUSE_PORT` | HTTP 或 native 端口，取决于客户端选择 |
| `CLICKHOUSE_DATABASE` | 默认 raw database，可由 spec 覆盖 |
| `CLICKHOUSE_USER` | 用户名 |
| `CLICKHOUSE_PASSWORD` | 密码 |
| `CLICKHOUSE_SECURE` | 是否使用 TLS |
| `CLICKHOUSE_CONNECT_TIMEOUT_SECONDS` | 连接超时 |
| `CLICKHOUSE_QUERY_TIMEOUT_SECONDS` | 查询超时 |

配置读取遵循 scheduler 边界：`config/env.py` 声明变量，`resources/clickhouse.py` 构造 resource，业务 asset 不直接读取环境变量。

## 风险和缓解

| 风险 | 缓解 |
|------|------|
| data_dict 未补齐或 S3 Parquet 与 data_dict 不一致 | 先修正 data_dict 或上游 PyArrow schema，再接入 ClickHouse raw；不要默认回退到 String |
| `ORDER BY` 选错导致后续迁移成本高 | 每张 raw 表在 DDL 前列出主要 dbt/mart 查询模式 |
| 当前年每日替换整年分区成本过高 | 先监控 staging insert 和 replace 耗时；如超过窗口，再考虑月分区或增量 append + 去重模型 |
| snapshot 替换 SQL 在目标 ClickHouse 版本上语义不确定 | 第一阶段实现前用最小表验证 table swap 或单分区 replace |
| compacted 资产和 raw sync 自动化形成过度触发 | raw sync asset 使用同分区依赖和 `eager()`，必要时加 job tags 或显式 jobs 控制 |
| staging 表残留导致排查混乱 | 使用固定 staging 表并在每次 run 开始清空，或使用 run-scoped staging 表并定期清理 |

## 待决问题

1. ClickHouse 客户端选型已由 ADR 0006 决定：使用官方 Python client `clickhouse-connect`，通过 HTTP interface 连接 ClickHouse。
2. snapshot 表替换 SQL：采用 table swap 还是单分区 replace，需要以目标 ClickHouse 版本验证。
3. raw database/table 命名：是否固定为 `raw.<asset_name>`，还是加入 source 前缀和 schema 前缀。
4. 每张 raw 表的 `ORDER BY`：需要结合首批 dbt staging/marts 查询模式确定。
5. `eastmoney__*` schema 类型：按 `docs/references/data_dict/` 的 PyArrow 类型接入；若 data_dict 缺失，先补齐再接入 ClickHouse。
6. 是否需要在 Dagster 中新增 asset checks 表达 ClickHouse row count/schema 校验，而不只是 materialization metadata。

## 验收标准

本 RFC 对应实现完成时，应满足：

1. Dagster definitions 中存在 ClickHouse raw sync assets，且 lineage 指向对应 S3 source/compacted assets。
2. `baostock__query_history_k_data_plus_daily` 至少一个年度分区可从 S3 同步到 ClickHouse raw。
3. raw sync 失败时不会清空或部分覆盖生产 raw 表。
4. ClickHouse raw sync materialization metadata 能定位 S3 输入、目标表、分区、行数和耗时。
5. 日分区稀疏资产不直接同步；只同步 compacted 年度资产。
6. dbt raw source 可以引用 ClickHouse raw 表作为 staging 上游。
7. 通过最小质量门禁：ruff、pyright、pytest、`dg check defs`。
