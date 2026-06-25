# Plan 0017: Dagster ClickHouse raw 同步实施计划

日期：2026-06-01

关联设计文档：

- `docs/RFC/archive/0009-dagster-clickhouse-raw-sync.md`
- `docs/RFC/archive/0007-dbt-raw-layer-and-dagster-dbt-integration.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/ADR/0006-clickhouse-python-client-selection.md`
- `docs/architecture/scheduler-architecture.md`
- `docs/architecture/scheduler-module-boundaries.md`

## 1. 目标

本计划把 RFC 0009 落到可执行实现：在 `pipeline/scheduler` 中新增 ClickHouse raw sync asset 层，将已经发布到 S3 Parquet 的 source/compacted assets 同步为 ClickHouse raw tables。

目标产物：

```text
Dagster source/compacted assets
  -> S3 Parquet latest_snapshot/year partition
  -> Dagster clickhouse/raw/* sync assets
  -> ClickHouse raw tables
  -> dbt source()/staging/marts
```

完成后应满足：

- `source/baostock__query_history_k_data_plus_daily` 至少一个 `year` 分区可同步到 `raw.baostock__query_history_k_data_plus_daily`。
- snapshot assets 可通过 staging 校验后整表替换，不暴露半成品。
- 日分区稀疏 assets 不直接进入 ClickHouse；只同步 compacted 年度资产。
- raw sync 失败时生产 raw 表保持旧版本可用。
- Dagster asset graph 中能看到 `source/* -> clickhouse/raw/*` lineage。
- dbt 可以把 ClickHouse raw tables 声明为 `source()`，但不负责 raw 装载。

## 2. 非目标

本计划不做以下事情：

- 不改变 S3 作为 raw 数据事实源的定位。
- 不让 dbt 读取 S3 Parquet 或执行 raw 层分区替换。
- 不引入 Kafka、ClickHouse materialized view ingest、秒级流式写入或多生产者队列。
- 不把 `source/jiuyan__action_field`、`source/ths__limit_up_pool` 两个日分区稀疏资产直接同步到 ClickHouse。
- 不把 `source/jiuyan__industry_ocr` processor asset 直接同步到 ClickHouse；只同步 `source/jiuyan__industry_ocr_snapshot`。
- 不在 Dagster definitions 加载阶段连接 ClickHouse、S3 或外部服务做发现。
- 不在 raw sync 中做业务清洗、字段重命名、mart 聚合或复杂去重。
- 不在常规同步流程中使用频繁 `ALTER TABLE UPDATE/DELETE` 或 `OPTIMIZE TABLE ... FINAL` 修补数据。

## 3. 当前事实基线

以 `cd pipeline/scheduler && uv run dg list defs --json` 核验，当前 definitions 基线为：

| 类型 | 当前状态 |
|------|----------|
| Assets | 19 个，全部在 group `s3_sources` |
| Jobs | 10 个 source jobs |
| Schedules | 7 个 source schedules |
| Resources | 8 个，无 ClickHouse resource |
| Sensors | 2 个：`default_automation_condition_sensor`、`slack_asset_failure_sensor` |

当前 `pipeline/scheduler/src/scheduler/defs/definitions.py` 只通过 `SOURCE_BUNDLES` 聚合 source bundles。ClickHouse raw sync 是跨数据源下游物化层，不属于任一 source bundle，应在 definitions 聚合层显式追加。

当前 `pipeline/scheduler` 尚未引入 `clickhouse-connect` 依赖。ADR 0006 已决定使用官方 Python client `clickhouse-connect`，并通过项目内窄协议隔离业务代码。

当前 `pipeline/elt` 仍是 dbt starter project，尚无 ClickHouse raw sources、staging models 或 marts。dbt 接入应等首批 raw table 可稳定同步后再做最小 source/staging。

当前 S3 Parquet 布局来自 ADR 0002：

| storage mode | 对象路径 |
|--------------|----------|
| `latest_snapshot` | `source/{asset_key}/000000_0.parquet` |
| `partitioned` | `source/{asset_key}/{partition_key_name}={partition_key}/000000_0.parquet` |

当前 S3 Parquet 的 PyArrow 类型基线以 `docs/references/data_dict/` 为准。raw sync 实现应直接复用这些已校正类型；如果实现、data_dict 和实际输出出现不一致，应先修正源数据和参考文档，而不是默认回退到 `String`。

ClickHouse raw 字段设计以 data_dict 的 `ClickHouse 类型` 列为准，映射规则见 `docs/references/data_dict/README.md`。首批 raw sync specs 生成 columns 前必须先读取对应资产的 data_dict：

| 资产 | data_dict |
|------|-----------|
| `source/sina__trade_calendar` | `docs/references/data_dict/sina__trade_calendar.md` |
| `source/jiuyan__industry_list` | `docs/references/data_dict/jiuyan__industry_list.md` |
| `source/jiuyan__industry_ocr_snapshot` | `docs/references/data_dict/jiuyan__industry_ocr_snapshot.md` |
| `source/jiuyan__action_field_compacted` | `docs/references/data_dict/jiuyan__action_field.md` |
| `source/ths__limit_up_pool_compacted` | `docs/references/data_dict/ths__limit_up_pool.md` |
| `source/eastmoney__*` | `docs/references/data_dict/eastmoney__*.md` |
| `source/baostock__query_stock_basic` | `docs/references/data_dict/baostock__query_stock_basic.md`，需先补齐 |
| `source/baostock__query_history_k_data_plus_daily` | `docs/references/data_dict/baostock__query_history_k_data_plus_daily.md`，需先补齐 |

`source/baostock__query_stock_basic` 和 `source/baostock__query_history_k_data_plus_daily` 若尚未有对应 data_dict，应先补齐 data_dict，再启用 ClickHouse raw spec。

当前 ClickHouse raw sync 输入分组：

| 分组 | 资产 | 同步策略 |
|------|------|----------|
| 首批年度分区 | `source/baostock__query_history_k_data_plus_daily` | `year` 分区替换 |
| snapshot | `source/sina__trade_calendar`、`source/baostock__query_stock_basic`、`source/jiuyan__industry_list`、`source/jiuyan__industry_ocr_snapshot` | 整表快照替换 |
| compacted 年度分区 | `source/jiuyan__action_field_compacted`、`source/ths__limit_up_pool_compacted` | `year` 分区替换 |
| 后续批量年度分区 | `source/eastmoney__*` | `year` 分区替换，按 `data_dict` 生成列定义 |

## 4. ClickHouse 约束基线

### 4.1 Workload Summary

| 项 | 内容 |
|----|------|
| workload | A 股市场数据 / 财务数据 raw layer，偏批量刷新 |
| latency target | 日级调度和历史 backfill，不是秒级 ingest |
| data shape | S3 Parquet latest snapshot、年度分区、compacted 年度分区 |
| primary query patterns | dbt staging/marts 读取 raw；按证券代码、交易日期、报告期或业务日期过滤 |
| operational constraints | 失败不能清空 raw；S3 可重放；definitions 加载不能访问外部服务 |

### 4.2 规则来源

计划实现必须遵守下列 ClickHouse 规则：

- Per `schema-pk-plan-before-creation`：`ORDER BY` 建表后不可轻易修改，首批 DDL 前必须列出每张表的主要查询模式。
- Per `schema-pk-prioritize-filters`：排序键应优先覆盖高频过滤列。
- Per `schema-types-native-types`：raw 表优先使用原生类型，不默认全 String。
- Per `schema-types-avoid-nullable`：只在语义必要时使用 `Nullable`，否则优先用默认值。
- Per `schema-types-lowcardinality`：低基数字符串可使用 `LowCardinality(String)`，具体列以 `docs/references/data_dict/` 的 `ClickHouse 类型` 为准。
- Per `schema-partition-low-cardinality`：年度分区 cardinality 可控；禁止按日或证券代码做高基数分区。
- Per `schema-partition-lifecycle`：分区主要用于生命周期、回填和批量替换，不把分区当作主要查询优化手段。
- Per `insert-batch-size`：生产装载应使用 ClickHouse server-side `INSERT ... SELECT ... FROM s3(...)` 批量读取 Parquet，不做 Python 单行写入。
- Per `insert-mutation-avoid-update` 和 `insert-mutation-avoid-delete`：raw 修正通过重刷 S3 分区并替换 ClickHouse 分区完成，不用频繁 mutation。
- Per `insert-optimize-avoid-final`：同步完成后不常规执行 `OPTIMIZE TABLE ... FINAL`。

Context7 官方文档核验点：

- ClickHouse `s3()` table function 支持从 S3 Parquet 执行 `INSERT INTO ... SELECT ... FROM s3(...)`。
- ClickHouse 官方示例包含 `EXCHANGE TABLES` 原子交换语义，但 snapshot 替换仍需在目标 ClickHouse 版本和 database engine 上做最小验证。
- `REPLACE PARTITION FROM` 属于 MergeTree 分区替换路径，实施前需用目标环境验证 staging/raw 表 engine、storage policy 和 schema 兼容。

## 5. 实施阶段

### 阶段 0：环境和 SQL 语义探针

目标：在写生产代码前确认目标 ClickHouse 环境支持 RFC 0009 需要的替换语义。

实施内容：

- 确认目标 ClickHouse 版本、database engine、raw database 名称和用户权限。
- 用最小测试表验证年度分区替换：

```sql
CREATE DATABASE IF NOT EXISTS raw;
CREATE TABLE raw.__raw_sync_probe (id UInt64, year UInt16)
ENGINE = MergeTree
PARTITION BY year
ORDER BY id;
CREATE TABLE raw.__raw_sync_probe__stage AS raw.__raw_sync_probe;
INSERT INTO raw.__raw_sync_probe__stage VALUES (1, 2026);
ALTER TABLE raw.__raw_sync_probe
REPLACE PARTITION 2026
FROM raw.__raw_sync_probe__stage;
```

- 验证 snapshot 替换 SQL，二选一并记录结论：
  - `EXCHANGE TABLES raw.table AND raw.table__stage`。
  - 单分区 snapshot table + `REPLACE PARTITION`。
- 验证 ClickHouse 能通过 `s3()` 读取 RustFS/MinIO S3 Parquet，包含 endpoint、bucket、access key、secret key、TLS/非 TLS 和 path-style URL。
- 记录验证 SQL、ClickHouse 版本和结果到后续 job report；不要把密码、secret 或完整连接串写入文档。

完成标准：

- 明确年度分区替换 SQL。
- 明确 snapshot 替换 SQL。
- 明确 `s3()` URL 和 credential 渲染方式。
- 明确 raw database/table 命名规则：第一阶段固定为 `raw.<asset_name>`。

测试策略：

- 该阶段是环境探针，不改生产代码。
- 记录 probe 命令和结果；失败时先停在本阶段，不进入 raw sync 实现。

### 阶段 1：依赖、配置和 ClickHouse resource

目标：把 ClickHouse 连接能力接入 scheduler，但不在 definitions 加载阶段发起连接。

前置条件：

- `docs/references/data_dict/baostock__query_stock_basic.md` 已补齐。
- `docs/references/data_dict/baostock__query_history_k_data_plus_daily.md` 已补齐。
- 两份 BaoStock data_dict 都包含 `ClickHouse 类型` 列，并通过 `docs/references/data_dict/README.md` 的映射规则审查。

实施内容：

- 在 `pipeline/scheduler/pyproject.toml` 增加 `clickhouse-connect` 依赖，并同步 `uv.lock`。
- 在 `pipeline/scheduler/src/scheduler/defs/config/env.py` 增加 `CLICKHOUSE_*` 环境变量默认读取：
  - `CLICKHOUSE_HOST`
  - `CLICKHOUSE_PORT`
  - `CLICKHOUSE_DATABASE`
  - `CLICKHOUSE_USER`
  - `CLICKHOUSE_PASSWORD`
  - `CLICKHOUSE_SECURE`
  - `CLICKHOUSE_CONNECT_TIMEOUT_SECONDS`
  - `CLICKHOUSE_QUERY_TIMEOUT_SECONDS`
- 新增 `pipeline/scheduler/src/scheduler/defs/clickhouse/protocols.py`，定义项目内窄协议：

```python
class ClickHouseClientProtocol(Protocol):
    @property
    def server_version(self) -> str: ...
    def ping(self) -> bool: ...
    def command(self, sql: str, *, settings: Mapping[str, object] | None = None) -> object: ...
    def query(self, sql: str, *, settings: Mapping[str, object] | None = None) -> ClickHouseQueryResult: ...
    def close(self) -> None: ...
```

- 新增 `pipeline/scheduler/src/scheduler/defs/resources/clickhouse.py`，实现 `ClickHouseResource`。
- `ClickHouseResource` 只负责配置和 client 构造；业务 asset 和 sync service 只依赖 protocol。
- raw sync assets 同时依赖现有 `S3SettingsResource`。`S3SettingsResource` 负责提供 RustFS/MinIO endpoint、bucket、access key、secret key、region 和 secure/path-style 相关信息，供 ClickHouse `s3()` SQL 渲染使用。
- 新增一个窄的 S3 SQL 输入配置对象，例如 `ClickHouseS3InputConfig`，只暴露 `s3()` 装载需要的字段；日志和 metadata 只能记录脱敏后的 endpoint、bucket 和 object key，不记录 secret。
- 更新 `pipeline/scheduler/src/scheduler/defs/definitions.py`，注册 `"clickhouse": ClickHouseResource()`。
- 更新 `.env.example` 中 ClickHouse 配置占位，不提交真实 `.env`。

完成标准：

- definitions load 不连接 ClickHouse。
- 单元测试可用 fake protocol 覆盖 raw sync 业务逻辑。
- integration definitions 测试能看到 `clickhouse` resource。
- raw sync asset factory 明确声明 `clickhouse` 和 `s3_settings` 两个 resource 依赖。

测试策略：

- 新增 resource/config 单元测试，覆盖默认值、timeout 转换、secure 布尔解析和 secret 不进入 repr/log。
- 新增 S3 SQL 输入配置单测，覆盖 object key 到 `s3()` URL 的渲染、path-style URL 和 secret 脱敏。
- 更新 `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py` 的资源基线。

### 阶段 2：spec、S3 key 渲染和 SQL helper

目标：建立静态 raw table specs 和纯 SQL 渲染层，为 asset factory 和 sync service 提供稳定输入。

实施内容：

- 新增模块：

```text
pipeline/scheduler/src/scheduler/defs/clickhouse/
  __init__.py
  specs.py
  sql.py
  protocols.py
```

- 在 `specs.py` 定义 `ClickHouseRawTableSpec`，至少包含：
  - `source_asset_key`
  - `raw_asset_key`
  - `storage_mode`
  - `source_partition_key_name`
  - `clickhouse_database`
  - `clickhouse_table`
  - `staging_table`
  - `partition_strategy`
  - `order_by`
  - `columns`
  - `allow_empty`
  - `sync_enabled`
- `columns` 应直接来自对应 data_dict 的字段表，包括字段名、PyArrow 类型和 `ClickHouse 类型`。
- `docs/references/data_dict/README.md` 是 PyArrow 到 ClickHouse 类型转换规则的权威说明；spec 生成器不得在代码里维护另一套不一致的映射规则。
- 首批只启用 `baostock__query_history_k_data_plus_daily` 年度分区 spec。
- 为 snapshot、compacted 和 EastMoney 预留 spec，但可以先 `sync_enabled=False` 或延后添加，避免首批 DDL 面过大。
- 明确排除：
  - `source/jiuyan__action_field`
  - `source/ths__limit_up_pool`
  - `source/jiuyan__industry_images`
  - `source/jiuyan__industry_ocr`
- 任何启用 spec 的 `columns` 必须能直接追溯到 `docs/references/data_dict/` 的 `ClickHouse 类型` 列，不要在 spec 中保留“暂时全 String”的回退开关。
- 首次创建 raw table 前必须为所有 `LowCardinality(String)` 字段记录基数验证结果：优先用 S3 Parquet 样本或 staging 表 `uniq()` 探针确认 unique values 低于 10,000；超过阈值时先更新对应 data_dict 为 `String`，再生成 DDL。
- 在 `sql.py` 提供纯函数：
  - identifier quote/validation。
  - raw table DDL。
  - staging table create/truncate/drop。
  - `INSERT INTO stage SELECT ... FROM s3(...)`。
  - staging count/schema/partition validation query。
  - `ALTER TABLE ... REPLACE PARTITION ... FROM ...`。
  - snapshot 替换 SQL，基于阶段 0 结论实现。
- S3 object key 渲染必须与 ADR 0002 一致，禁止在 clickhouse 模块里复制一套不受测的路径规则；优先复用或提取现有 storage helper。

完成标准：

- spec validation 能在单测中拒绝非法 asset key、table name、partition strategy、缺失 `year` partition 和禁用资产误入。
- SQL helper 不执行 SQL，便于纯单测覆盖。
- 首批 DDL 前为 `baostock__query_history_k_data_plus_daily` 写明 `ORDER BY` 依据。
- 首批 DDL 前完成 BaoStock data_dict 的 ClickHouse 类型审查和 `LowCardinality(String)` 基数验证。

测试策略：

- `test_clickhouse_specs.py`：
  - 首批 enabled specs 只包含预期资产。
  - 日分区稀疏资产和 OCR processor 不生成 spec。
  - 年度资产必须有四位 `year` 分区策略。
  - enabled spec 的每个 column 都能追溯到对应 data_dict 的 `ClickHouse 类型`。
  - `LowCardinality(String)` 字段必须带有基数验证记录或显式例外说明。
- `test_clickhouse_sql.py`：
  - DDL 包含 `MergeTree`、`PARTITION BY year`、`ORDER BY (...)`。
  - staging insert 能在 Parquet 无 `year` 列时注入 `toUInt16(<year>) AS year`。
  - replace SQL 不使用 `DROP PARTITION` + `INSERT`。
  - snapshot SQL 与阶段 0 选择一致。
- `test_s3_key_rendering.py`：
  - latest snapshot 和 `year=YYYY` 路径与 ADR 0002 一致。

### 阶段 3：raw sync service

目标：实现 staging 装载、校验、替换和 metadata 记录的业务编排。

实施内容：

- 新增 `pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py`。
- 定义 `RawSyncRequest`、`RawSyncResult`、`RawSyncValidationResult` 等数据类。
- 年度分区同步流程：
  1. 校验 `partition_key` 是四位年份。
  2. 根据 spec 和 partition key 渲染 S3 Parquet object key。
  3. 从 `S3SettingsResource` 构造脱敏可观测、含凭据仅用于 SQL 执行的 S3 输入配置。
  4. 创建 raw 表和 staging 表，或按阶段 0 约定清空 staging。
  5. 通过 ClickHouse `s3()` table function 装载 staging。
  6. 校验 staging 行数、`min(year)`、`max(year)`、schema 兼容、`LowCardinality(String)` 字段基数和 `allow_empty`。
  7. 校验通过后执行 `REPLACE PARTITION`。
  8. 查询替换后 raw 分区行数。
  9. 返回 metadata 和耗时。
- snapshot 同步流程：
  1. 根据 spec 渲染 fixed S3 Parquet object key。
  2. 从 `S3SettingsResource` 构造 S3 输入配置。
  3. 装载 snapshot staging。
  4. 校验 row count、schema、`LowCardinality(String)` 字段基数和主键候选唯一性。
  5. 执行阶段 0 选定的整表替换 SQL。
  6. 返回 metadata 和耗时。
- 失败语义：
  - S3 文件不存在、staging insert 失败、schema 校验失败、row count 校验失败时，不执行 replace。
  - replace 失败时 Dagster run 失败并由现有 slack sensor 告警。
  - 日志只包含安全 SQL 摘要，不打印 secret。

完成标准：

- fake ClickHouse client 能证明成功路径 SQL 顺序正确。
- fake ClickHouse client 能证明任一校验失败时不执行 replace。
- `RawSyncResult` 包含 RFC 0009 要求的 materialization metadata 字段。

测试策略：

- `test_raw_sync_success_path.py`：覆盖年度分区成功和 snapshot 成功。
- `test_raw_sync_failure_path.py`：覆盖 staging insert、schema mismatch、empty disallowed、partition mismatch、row count mismatch。
- `test_raw_sync_metadata.py`：覆盖 metadata key、耗时字段和 secret 脱敏。
- `test_raw_sync_s3_config.py`：覆盖 raw sync service 从 `S3SettingsResource` 获取 S3 配置，不从 ClickHouse resource 或环境变量直接拼接。

### 阶段 4：Dagster asset factory、jobs 和 definitions 装配

目标：把 raw sync service 暴露为 Dagster assets，并接入现有 definitions。

实施内容：

- 新增：

```text
pipeline/scheduler/src/scheduler/defs/clickhouse/
  assets.py
  definitions.py
```

- `assets.py` 根据 enabled specs 生成 Dagster assets：
  - asset key：`clickhouse/raw/<table_name>`。
  - group：`clickhouse_raw`。
  - deps：对应 `source/*` 或 compacted asset。
  - tags：保留 owner、kind、source/layer/storage/state/modality；新增 `layer=raw`、`storage=clickhouse`。
  - 年度资产使用与上游相同的 `year` partitions_def。
  - snapshot 资产无 partition。
- asset 函数保持薄封装，只把 context、partition_key、resource 和 spec 交给 raw sync service。
- `definitions.py` 暴露：
  - `CLICKHOUSE_RAW_ASSETS`
  - `CLICKHOUSE_RAW_JOBS`
- 顶层 `scheduler.defs.definitions.defs()` 显式追加：
  - `assets=[*bundle_assets(SOURCE_BUNDLES), *CLICKHOUSE_RAW_ASSETS]`
  - `jobs=[*bundle_jobs(SOURCE_BUNDLES), *CLICKHOUSE_RAW_JOBS]`
  - `resources["clickhouse"]`
- 第一阶段不新增 schedule。提供定向 jobs：
  - `clickhouse__raw_sync_all_job`
  - `clickhouse__raw_sync_baostock_job`
  - `clickhouse__raw_sync_market_event_job`，可以在 compacted specs 启用后加入。
- 自动化策略：
  - 首批 BaoStock 分区 sync 可以先通过 job 手动验证。
  - 验证通过后为 raw sync assets 添加 `dg.AutomationCondition.eager()`，依赖 `default_automation_condition_sensor` 跟随上游 materialization。
  - 不通过 schedule 重复表达同一触发关系。

完成标准：

- `dg list defs --json` 能看到 `clickhouse/raw/baostock__query_history_k_data_plus_daily`。
- asset lineage 为 `source/baostock__query_history_k_data_plus_daily -> clickhouse/raw/baostock__query_history_k_data_plus_daily`。
- 现有 source bundles 的 assets/jobs/schedules 不被破坏。

测试策略：

- 更新 `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py`：
  - source bundle contract 仍只描述 source bundles。
  - registered definitions 断言额外包含 ClickHouse raw assets/jobs/resource。
  - sensors 保持 `default_automation_condition_sensor` 和 `slack_asset_failure_sensor`。
- 新增 `test_clickhouse_asset_factory.py`：
  - asset key、group、deps、tags、partition def 与 spec 一致。
  - disabled spec 不生成 asset。

### 阶段 5：首批 BaoStock 年度分区端到端验证

目标：用一个年度分区证明协议可运行、可观测、可恢复。

实施内容：

- 选择已有 S3 分区，例如 `year=2026`。
- 确认上游 S3 object 存在：

```text
source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet
```

- 启动或连接目标 ClickHouse。
- 运行定向 raw sync job：

```bash
set -a
. ./.env
set +a
make dagster-home
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily" \
  --partition 2026
```

- 验证 ClickHouse：
  - staging 装载行数。
  - raw 分区行数。
  - `min(year) = max(year) = 2026`。
  - `system.parts` 中目标表 part 数量处于合理范围。
- 人工制造一个校验失败样本，确认 raw 旧分区不被清空。
- 将命令、时间、分区、行数、耗时和失败样本写入 `docs/jobs/reports/`。

完成标准：

- BaoStock 单年分区可以从 S3 同步到 ClickHouse raw。
- 失败样本证明不会出现先 drop 后失败导致空洞。
- materialization metadata 能定位 S3 object、ClickHouse table、partition、row count 和耗时。

测试策略：

- 自动化单测仍使用 fake client。
- 该阶段增加一次环境 smoke test，不能替代单元测试和 definitions check。

### 阶段 6：snapshot assets 接入

目标：将小型 latest snapshot assets 接入 ClickHouse raw，并最终确认 snapshot 替换协议。

实施顺序：

1. `source/sina__trade_calendar`
2. `source/baostock__query_stock_basic`
3. `source/jiuyan__industry_list`
4. `source/jiuyan__industry_ocr_snapshot`

实施内容：

- 为每张 snapshot 表补充 spec、DDL、主键候选和唯一性校验。
- 按阶段 0 结论实现 snapshot 替换 SQL。
- 先用小表验证 `sina__trade_calendar`，再接入其他 snapshot。
- `jiuyan__industry_ocr_snapshot` 只消费 latest snapshot Parquet，不读取 OCR processor 中间状态或单图对象。

完成标准：

- 四个 snapshot assets 均可通过 staging 校验后替换 raw table。
- snapshot 替换期间下游不会读到半成品。
- `jiuyan__industry_ocr` 仍不生成 raw sync asset。

测试策略：

- 扩展 spec、SQL 和 raw sync service snapshot tests。
- 增加 definitions 测试，确认四个 snapshot raw assets lineage 正确。

### 阶段 7：compacted 年度资产和 EastMoney 批量接入

目标：扩大年度分区同步范围，但保持接入顺序和 schema 风险可控。

实施顺序：

1. `source/jiuyan__action_field_compacted`
2. `source/ths__limit_up_pool_compacted`
3. `source/eastmoney__balance`
4. `source/eastmoney__cashflow_sq`
5. `source/eastmoney__cashflow_ytd`
6. `source/eastmoney__dividend_allotment`
7. `source/eastmoney__dividend_main`
8. `source/eastmoney__equity_history`
9. `source/eastmoney__income_sq`
10. `source/eastmoney__income_ytd`

实施内容：

- compacted assets 使用 `year` 分区替换，业务日期列继续保留为字段，不用 `year` 替代业务日期。
- EastMoney specs 批量生成前，先审查每张表的 schema 类型，并以 `docs/references/data_dict/eastmoney__*.md` 的 `ClickHouse 类型` 列为准。
- 每张表在 DDL 前写明初始 `ORDER BY` 依据，避免随意选择排序键。
- 对当前年每日替换整年分区的耗时做 metadata 观测；如果超过运行窗口，再新增 RFC/ADR 评估月分区或 append + dedupe 模型。

完成标准：

- compacted 年度资产可跟随上游 compact 后同步。
- EastMoney 批量 assets 可逐表启用，失败时不影响已启用表。
- `source/jiuyan__action_field` 和 `source/ths__limit_up_pool` 日分区资产仍不直接同步。

测试策略：

- 扩展 daily sparse exclusion 单测。
- 为 EastMoney generated specs 增加唯一性和 disabled/enabled 开关测试。
- 至少对一个 EastMoney 表做环境 smoke test 后再批量启用。

### 阶段 8：dbt raw source 和首批 staging

目标：让 dbt 从 ClickHouse raw tables 读取，完成 RFC 0009 的下游边界。

实施内容：

- 在 `pipeline/elt/models/sources.yml` 声明 ClickHouse raw sources。
- 首批只为已验证的 raw tables 添加 source 和 staging view：
  - `raw.baostock__query_history_k_data_plus_daily`
  - 通过阶段 6 验证后再加入 snapshot tables。
- dbt staging 只做字段命名、类型收敛、轻清洗和基础过滤。
- 不在 dbt 中调用 S3、`REPLACE PARTITION` 或 raw sync SQL。
- 后续如接入 dagster-dbt，应把 dbt staging/marts 纳入 Dagster asset graph，但不改变 raw 写入职责。

完成标准：

- dbt `source()` 可以引用 ClickHouse raw table。
- 首批 staging view 可编译。
- raw sync asset 和 dbt asset 的职责边界清晰。

测试策略：

- 使用 dbt 定向编译/构建命令验证首批 source/staging。
- 如尚未配置 dbt ClickHouse adapter，本阶段先记录 adapter 依赖和 profiles 配置需求，不把配置问题隐藏在 scheduler 实现里。

## 6. 禁止模式和允许例外

禁止模式：

- 在 definitions 加载阶段 ping ClickHouse 或查询外部 schema。
- 在业务 source asset 中直接读取 `CLICKHOUSE_*` 环境变量。
- 在 raw sync 中直接构造 `clickhouse_connect.Client`，绕过 `ClickHouseResource` 和 protocol。
- 用 `DROP PARTITION` + `INSERT` 替代 staging 校验后的分区替换。
- 把日分区稀疏资产直接装入 ClickHouse raw。
- 把 OCR processor 中间状态当作 raw table。
- 在 dbt 中实现 S3 到 ClickHouse raw 的装载协议。
- 为所有列默认使用 `String`，绕过 `docs/references/data_dict/` 中已校正的 PyArrow 类型。
- 在同步完成后常规执行 `OPTIMIZE TABLE ... FINAL`。

允许例外：

- 如果目标 ClickHouse 环境不支持预期 snapshot table swap，可以采用单分区 snapshot table + `REPLACE PARTITION`，但必须在阶段 0 probe 中验证并记录。
- 如果 `data_dict`、S3 Parquet 或 ClickHouse raw schema 出现不一致，先修正源数据类型或参考文档，再接入 raw sync；不要把“全 String”作为默认回退方案。
- 如果首批启用 `eager()` 自动化带来过度触发，可以先保留手动 job，待 smoke test 和运行窗口稳定后再启用自动化。

## 7. 验证命令

文档-only 变更：

```bash
git diff --check
```

scheduler 代码变更后：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

dbt 代码变更后：

```bash
cd pipeline
uv run dbt compile --project-dir elt
uv run dbt build --project-dir elt --select <changed_models>
```

环境 smoke test：

```bash
set -a
. ./.env
set +a
make dagster-home
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily" \
  --partition 2026
```

如果 `dg launch --assets ... --partition ...` 的实际 CLI 参数与当前 `dagster-dg-cli` 版本不匹配，以 `uv run dg launch --help` 和 `docs/skills/dg-backfill-runbook/SKILL.md` 为准更新命令，并把最终命令写入 job report。

## 8. 运行观测和报告

每个 raw sync materialization 必须记录：

| metadata | 说明 |
|----------|------|
| `clickhouse_database` | 目标 database |
| `clickhouse_table` | 目标 raw table |
| `staging_table` | 本次 staging table |
| `storage_mode` | `latest_snapshot` 或 `partitioned` |
| `partition_key_name` | 年度资产为 `year` |
| `partition_key` | 分区运行时为年份；snapshot 为空 |
| `s3_object_key` | 输入 Parquet object |
| `loaded_row_count` | staging 装载行数 |
| `raw_row_count_after_replace` | 替换后目标范围行数 |
| `schema_hash` | spec schema 或 ClickHouse columns hash |
| `clickhouse_insert_seconds` | staging 装载耗时 |
| `clickhouse_validation_seconds` | 校验耗时 |
| `clickhouse_replace_seconds` | 替换耗时 |

首批 BaoStock smoke test、snapshot 替换 probe、首次 compacted/EastMoney 接入都应写入 `docs/jobs/reports/`。报告必须包含命令、UTC 时间、分区或 snapshot 范围、行数、结果和失败处理；不得包含密码、secret 或完整连接串。

## 9. 完成后的文档维护

实现完成后：

- 将 RFC 0009 状态从草案更新为已实施或部分实施，并列出仍未接入的资产。
- 更新 `docs/architecture/scheduler-architecture.md` 的 registered resources、asset groups 和 ClickHouse raw sync 层。
- 更新 `docs/architecture/scheduler-module-boundaries.md`，加入 `clickhouse/` 模块职责和禁止模式。
- 如 snapshot 替换 SQL、raw table 命名或 `ORDER BY` 决策形成长期约束，新增或更新 ADR。
- 将本计划移动到 `docs/plans/archive/`，或在文件顶部标记完成状态和完成日期。
