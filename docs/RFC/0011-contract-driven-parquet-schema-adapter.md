# RFC 0011: 一刀切收敛 Dagster Parquet schema 到 contracts

状态：草案（2026-06-01）

## 摘要

本文档记录当前仓库中 `source -> S3 Parquet -> ClickHouse raw` schema 管理链路的代码事实，并定义一次破坏性、无历史兼容包袱的改造方案。

目标不是最小改动，也不是分阶段保留双轨；目标是一次性把 Dagster source assets 写 S3 Parquet 所用的 schema 事实源收敛到 `pipeline/contracts/datasets/*.yml`，删除 scheduler 内手写 PyArrow schema 和 EastMoney 专用 schema 生成路径。

改造原则：

1. 先完成全部生产代码迁移，再补测试；不采用边改边测的迭代方式。
2. 不保留旧 schema 常量作为兼容层。
3. 不保留 EastMoney 专用 schema generator。
4. 不让 source business code 直接解析 contracts。
5. 不改变 raw ingestion 边界：contracts 只管 `source.fields -> parquet.fields -> clickhouse_raw.fields`，dbt staging 继续由 dbt 项目维护。
6. 不把多层 JSON flatten、数组行生成、aliases、unknown field 计数或 API 字段顺序迁移进 Parquet schema adapter。

当前已核验事实：

1. `pipeline/contracts/datasets/*.yml` 已经包含 18 个 dataset，其中 15 个 raw dataset、3 个 source-only dataset。
2. 每个 dataset 都有 `parquet.fields`，字段包含 `name`、`type`、`nullable`。
3. ClickHouse raw specs 已经通过 `fleur_contracts.adapters.clickhouse.build_scheduler_specs()` 从 contract registry 构造。
4. `contract_tools` 当前有 ClickHouse adapter 和 Parquet schema 校验 CLI，但没有统一的 Parquet adapter。
5. scheduler 里仍有多处手写或专用生成的 `pa.schema(...)`。
6. 现有架构测试禁止 `pipeline/scheduler/src/scheduler/defs/sources/**` 和 `defs/baostock/**` 直接 import `fleur_contracts` 或引用 `pipeline/contracts`。
7. 部分 source asset 的远端响应不是平铺表，而是多层嵌套 JSON，当前由 source-specific conversion code 展开、广播、过滤或归一化后生成 Parquet rows。

## 关联文档

- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/RFC/0009-dagster-clickhouse-raw-sync.md`
- `docs/RFC/0010-data-contract-registry-and-contract-tools.md`
- `docs/plans/0021-contract-scope-raw-layer-cleanup-plan.md`
- `docs/architecture/scheduler-architecture.md`
- `docs/architecture/scheduler-module-boundaries.md`
- `docs/skills/fleur-contract-data-dictionary/SKILL.md`

## 当前项目结构事实

当前仓库是 `pipeline/` uv workspace：

| 路径 | 当前职责 |
|------|----------|
| `pipeline/contracts/` | dataset contracts、glossary、naming rules；当前 raw ingestion 字段事实源 |
| `pipeline/contract_tools/` | `contract-tools` workspace package，Python import 包名为 `fleur_contracts` |
| `pipeline/scheduler/` | Dagster scheduler 项目，定义 source assets、S3 IO manager、ClickHouse raw sync assets |
| `pipeline/elt/` | dbt 项目；当前由 contracts 生成 `models/sources.yml`，staging 层由 dbt 自己维护 |
| `docs/references/data_dict/` | contract 生成的数据字典文档，不是字段事实源 |

workspace 依赖关系已经存在：

- `pipeline/pyproject.toml` 的 `[tool.uv.workspace].members` 包含 `contract_tools` 和 `scheduler`。
- `pipeline/scheduler/pyproject.toml` 的 dependencies 包含 `contract-tools`。
- `pipeline/scheduler/pyproject.toml` 的 `[tool.uv.sources]` 声明 `contract-tools = { workspace = true }`。
- `pipeline/contract_tools/pyproject.toml` 声明 wheel package 为 `src/fleur_contracts`，并提供 CLI `fleur-contracts = "fleur_contracts.cli:main"`。

因此 scheduler 当前通过普通 Python workspace package import `fleur_contracts`，不是通过 Dagster resource 跨项目调用。

## 当前 contract_tools 事实

### Contract schema

`pipeline/contract_tools/src/fleur_contracts/schema.py` 当前定义：

- `SourceField`：`name`、`type`、`required`、`external_description_zh`。
- `ParquetField`：`name`、`type`、`nullable`。
- `ClickHouseRawField`：`name`、`type`、`from`、`nullable`、`default`、`glossary_key`、`reason`。
- `DatasetContract`：包含 `source_asset_key`、可选 `raw_asset_key`、`external`、`source`、`parquet`、可选 `clickhouse_raw`、`dataset_note_zh`、`validation_notes`。

当前 schema validator 已经校验：

- `parquet.storage_mode == "partitioned"` 时必须有 `parquet.partition_key_name`。
- `parquet.storage_mode == "latest_snapshot"` 时不能有 `parquet.partition_key_name`。
- source-only dataset 不能定义 `raw_asset_key`。
- raw dataset 必须定义 `raw_asset_key`，且 `raw_asset_key == ["clickhouse", "raw", dataset]`。
- `clickhouse_raw.table` 必须等于 dataset 名。
- year partition raw strategy 要求 Parquet 是 `partitioned` 且 `partition_key_name == "year"`。
- snapshot raw strategy 要求 Parquet 是 `latest_snapshot`。
- `clickhouse_raw.fields[].from` 必须引用存在的 `parquet.fields[].name`。
- `LowCardinality(...)` ClickHouse raw 字段必须写 `reason`。
- `clickhouse_raw.order_by` 必须引用 ClickHouse raw 字段。

这些验证说明：contracts 已经能表达 Parquet schema 和 ClickHouse raw schema 的字段链路。

### Loader and hashes

`pipeline/contract_tools/src/fleur_contracts/loader.py` 当前提供：

- `load_registry(contract_root=DEFAULT_CONTRACT_ROOT)`：加载所有 dataset、glossary 和 naming rules。
- `load_dataset_contract(path)`：加载单个 dataset contract，并验证文件名 stem 等于 `contract.dataset`。
- `dataset_schema_hash(contract)`：对整个 contract 做 JSON canonical dump 后 SHA256。
- `clickhouse_schema_hash(contract)`：由 `clickhouse_raw.fields` 的 `name:type` 和 year 分区列计算。
- `source_schema_hash(contract)`：由 `source.fields` 的 `name:type` 计算。

当前没有 `parquet_schema_hash(contract)`。

### ClickHouse adapter

`pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py` 当前提供：

- `ClickHouseColumnContract`
- `ClickHouseRawTableContract`
- `raw_table_contract_from_dataset(contract)`
- `raw_table_contracts(contracts)`
- `build_scheduler_specs(...)`

`raw_table_contract_from_dataset()` 已经把 `clickhouse_raw.fields[].from` 与 `parquet.fields` join，构造每个 ClickHouse column 的 `pyarrow_type` 和 `clickhouse_type`。

`pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py` 当前通过以下调用消费该 adapter：

```python
CLICKHOUSE_RAW_TABLE_SPECS = build_scheduler_specs(
    load_registry().datasets,
    asset_key_factory=dg.AssetKey,
    table_spec_factory=ClickHouseRawTableSpec,
    column_spec_factory=ClickHouseColumnSpec,
)
```

这说明 ClickHouse raw sync spec 已经从 contract registry 派生。

### Parquet validator

`pipeline/contract_tools/src/fleur_contracts/validate_parquet.py` 当前提供 `validate_available_parquet()`：

- 读取 `.env`。
- 加载 registry。
- 构造 PyArrow S3 filesystem。
- 对每个 dataset 推导一个 S3 object key。
- 用 `pq.ParquetFile(...).schema_arrow` 读取真实 Parquet schema。
- 将实际 `[(field.name, str(field.type))]` 与 `dataset.parquet.fields` 的 `[(field.name, field.type)]` 对比。

当前限制和事实：

- validator 只做已写入对象和 contract 的对比。
- validator 没有提供 scheduler 可复用的 `pa.Schema` adapter。
- `_object_key_for_dataset()` 对 partitioned dataset 固定检查 `year=2026`。

## 当前 scheduler schema 使用事实

### ClickHouse raw sync

ClickHouse raw 已经独立在 `pipeline/scheduler/src/scheduler/defs/clickhouse/`：

| 文件 | 当前事实 |
|------|----------|
| `clickhouse/specs.py` | 定义 `ClickHouseColumnSpec`、`ClickHouseRawTableSpec`，并从 `fleur_contracts.adapters.clickhouse` 构造 specs |
| `clickhouse/assets.py` | `build_clickhouse_raw_asset(spec)` 根据 spec 生成 Dagster raw asset |
| `clickhouse/raw_sync.py` | 使用 spec 编排 staging load、schema 校验、row count 校验和替换 |
| `clickhouse/sql.py` | 使用 spec 渲染 ClickHouse SQL |
| `clickhouse/definitions.py` | 使用 enabled specs 构造 raw sync jobs |

`clickhouse/assets.py` 会把 `contract_dataset`、`contract_version`、`contract_schema_hash`、`source_schema_hash`、`clickhouse_schema_hash`、`storage_mode`、`partition_key_name` 等写入 raw asset metadata。

### S3 IO manager and storage

`pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py` 当前：

- 从 Dagster asset definition metadata 读取 `storage_mode`、`partition_key_name`、`allow_empty`。
- 对 latest snapshot 要求 asset 返回 `pa.Table`。
- 对 partitioned asset 要求返回 `Mapping[str, pa.Table]`，且 keys 等于 Dagster asset partition keys。
- 校验非空表规则，但不校验表 schema 是否等于 contract。
- 调用 `S3DatasetService.write_latest_snapshot()` 或 `write_partitioned()`。

`pipeline/scheduler/src/scheduler/defs/storage/parquet.py` 当前：

- `write_parquet_dataset(table, base_dir, filesystem, ...)` 直接写入传入的 `pa.Table`。
- 不接收 expected schema。
- 不做 contract schema cast 或 schema equality 校验。

`pipeline/scheduler/src/scheduler/defs/storage/dataset_writer.py` 当前：

- `S3DatasetWriter.write_latest_snapshot()` 和 `write_partitioned()` 直接转发 `pa.Table`。
- `partition_column_count()` 只比较多个 partition table 的 column names 是否一致，不比较类型或 nullable。

这些事实说明：当前 S3 写入层信任 source asset 已经构造出正确 schema。

### Nested JSON conversion facts and risks

当前有多处 source conversion 不是简单字段选择，而是把多层 JSON 或半结构化内容转换成最终 Parquet rows：

| 文件 | 当前事实 |
|------|----------|
| `pipeline/scheduler/src/scheduler/defs/http/schemas.py` | `jiuyan_action_field_to_table()` 从 `content_row -> list -> stock -> article -> action_info` 提取字段，并维护 outer、stock、action_info 字段组 |
| `pipeline/scheduler/src/scheduler/defs/http/schemas.py` | `ths_limit_up_pool_to_table()` 从 page-level `date` 和 `data.info[]` 组合输出 rows |
| `pipeline/scheduler/src/scheduler/defs/http/schemas.py` | `jiuyan_industry_list_to_table()` 从 page `data.result[]` 输出 rows |
| `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py` | OCR JSON 结果通过 aliases 归一化为 `stock_name`、`theme_path`、`relation`、`source` |
| `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py` | EastMoney row conversion 使用 OpenAPI/generated field names 做 unknown field 计数，并把 rows 转为 typed table |

这些转换逻辑包含 schema 以外的执行语义：

- JSON path 到输出列的映射。
- 数组展开为多行的规则。
- page-level 字段向 item-level row 的广播规则。
- aliases、去重、空行过滤和 unknown field 计数。
- API 请求字段顺序和远端响应字段顺序。
- 源字段缺失时生成 null、空字符串或跳过 row 的策略。

风险判断：

- 有风险的做法：把 contracts 的 `parquet.fields` 当成 flatten spec，删除 source-specific nested conversion code。
- 可控的做法：contracts 只接管最终输出 `pa.Schema`，source-specific conversion code 继续持有 nested JSON 解析、展开、过滤和归一化语义。

因此本 RFC 的一刀切范围只包含 dataset-level output schema，不包含 JSON transform ownership 迁移。

### Source asset schema definitions

当前 scheduler 中已经核验的 `pa.schema(...)` 事实来源如下。

#### BaoStock

`pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`：

- `STOCK_BASIC_FIELDS`
- `K_HISTORY_DAILY_FIELDS`
- `K_HISTORY_DAILY_FIELD_PARAM`
- `STOCK_BASIC_SCHEMA = pa.schema([...])`
- `K_HISTORY_DAILY_SCHEMA = pa.schema([...])`
- `response_to_table(response, schema)`
- `stock_basic_response_to_table(response)`
- `k_history_daily_response_to_table(response)`

使用点：

- `BaostockStockBasicRefreshService` 和 `BaostockDailyKlineRefreshService` 通过 schema conversion 生成 `pa.Table`。
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py` 的 `empty_k_history_table()` 直接 import `K_HISTORY_DAILY_SCHEMA`。

一刀切调整：

- 删除 `STOCK_BASIC_SCHEMA` 的手写 `pa.schema(...)`。
- 删除 `K_HISTORY_DAILY_SCHEMA` 的手写 `pa.schema(...)`。
- 从统一生成模块导入 `BAOSTOCK_QUERY_STOCK_BASIC_SCHEMA` 和 `BAOSTOCK_QUERY_HISTORY_K_DATA_PLUS_DAILY_SCHEMA`。
- `STOCK_BASIC_FIELDS` 和 `K_HISTORY_DAILY_FIELDS` 改为从 schema names 派生，除非 BaoStock API request 顺序必须独立声明。若独立声明，则必须保留顺序断言测试。
- `K_HISTORY_DAILY_FIELD_PARAM` 继续由 BaoStock API fields join 得出，它属于 API 请求参数，不属于 Parquet adapter。

#### HTTP shared schemas: JiuYan action_field, JiuYan industry_list, THS limit_up_pool

`pipeline/scheduler/src/scheduler/defs/http/schemas.py` 当前定义：

- `JIUYAN_ACTION_FIELD_COLUMNS`
- `JIUYAN_ACTION_FIELD_OUTER_COLUMNS`
- `JIUYAN_ACTION_FIELD_STOCK_COLUMNS`
- `JIUYAN_ACTION_FIELD_ACTION_INFO_COLUMNS`
- `THS_LIMIT_UP_POOL_COLUMNS`
- `THS_LIMIT_UP_POOL_INFO_COLUMNS`
- `JIUYAN_INDUSTRY_LIST_COLUMNS`
- `THS_LIMIT_UP_POOL_SCHEMA = pa.schema([...])`
- `JIUYAN_ACTION_FIELD_SCHEMA = pa.schema([...])`
- `JIUYAN_INDUSTRY_LIST_SCHEMA = pa.schema([...])`
- conversion functions：`jiuyan_action_field_to_table()`、`ths_limit_up_pool_to_table()`、`jiuyan_industry_list_to_table()`。
- helper：`empty_string_table()`、`rows_to_string_table()`、`string_schema()`、`rows_to_typed_table()`。

使用点：

- `sources/jiuyan/action_field.py` 使用 `empty_jiuyan_action_field_table()` 和 `jiuyan_action_field_to_table()`。
- `sources/jiuyan/industry_list.py` 使用 `jiuyan_industry_list_to_table()`。
- `sources/ths/limit_up_pool.py` 使用 `ths_limit_up_pool_to_table()`。
- source-only daily assets `jiuyan__action_field`、`ths__limit_up_pool` 不是 ClickHouse raw sync 输入，但仍有 contract 的 `parquet.fields`，也在 data_dict 中展示。

一刀切调整：

- 删除 `THS_LIMIT_UP_POOL_SCHEMA` 的手写 `pa.schema(...)`。
- 删除 `JIUYAN_ACTION_FIELD_SCHEMA` 的手写 `pa.schema(...)`。
- 删除 `JIUYAN_INDUSTRY_LIST_SCHEMA` 的手写 `pa.schema(...)`。
- 从统一生成模块导入对应 dataset schema。
- `*_COLUMNS` 改为从 schema names 派生。只保留真正表达嵌套 flatten 逻辑的子集常量，例如 outer、stock、action_info、info columns。
- 删除任何与 schema 字段重复的列表，避免第二事实源。
- `string_schema(columns)` 可以保留，因为它是 helper，不是 dataset schema 事实源。

#### Sina trade calendar

`pipeline/scheduler/src/scheduler/defs/sources/sina/trade_calendar.py` 当前：

- `trade_calendar_dates_to_table(trade_dates)` 内部直接创建 `pa.schema([pa.field("trade_date", pa.date32())])`。
- `sina__trade_calendar` asset 返回 latest snapshot `pa.Table` 给 `s3_io_manager`。

一刀切调整：

- 删除函数内手写 schema。
- 从统一生成模块导入 `SINA_TRADE_CALENDAR_SCHEMA`。
- `trade_calendar_dates_to_table()` 使用导入 schema 构造 table。

#### JiuYan OCR result and OCR snapshot

`pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py` 当前：

- `JIUYAN_INDUSTRY_OCR_SCHEMA = pa.schema([...])`。
- `ocr_rows_to_table(industry_id, rows)` 使用该 schema。
- 该 dataset 是 source-only contract，当前不直接同步 ClickHouse raw。

`pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py` 当前：

- import `JIUYAN_INDUSTRY_OCR_SCHEMA`。
- 定义 `SNAPSHOT_SCHEMA_VERSION = 1`。
- 定义 `JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA = pa.schema([...])`。
- `build_industry_ocr_snapshot()` 合并 OCR result table，并构造 snapshot table。
- `_validated_ocr_result_table()` 用 `JIUYAN_INDUSTRY_OCR_SCHEMA` 验证 OCR result table。
- `_add_snapshot_columns()` 用 `JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA` 构造 snapshot table。

一刀切调整：

- 删除 `JIUYAN_INDUSTRY_OCR_SCHEMA` 的手写 `pa.schema(...)`。
- 删除 `JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA` 的手写 `pa.schema(...)`。
- 从统一生成模块导入对应 dataset schema。
- 删除 `SNAPSHOT_SCHEMA_VERSION`，改为在 metadata 写入 `contract_dataset`、`contract_version`、`contract_schema_hash`、`parquet_schema_hash`。
- 保留 OCR aliases、normalize logic、snapshot row count validation 和 `_validated_ocr_result_table()`。

#### EastMoney

`pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py` 当前：

- 文件头明确写着“由 scripts/generate_eastmoney_schemas.py 从 dataset contract 自动生成”。
- 包含 8 个 `EASTMONEY_*_SCHEMA = pa.schema([...])`。
- `EASTMONEY_SCHEMAS: dict[str, pa.Schema]` 按 asset_name 查表。

`pipeline/scheduler/scripts/generate_eastmoney_schemas.py` 当前：

- 直接用 `yaml.safe_load()` 读取 `pipeline/contracts/datasets/<asset>.yml`。
- 自带 `PA_TYPE_MAP`。
- 只处理 8 个 EastMoney dataset。
- 写入 `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`。
- 支持 `--check`。

`pipeline/scheduler/tests/unit/sources/eastmoney/test_eastmoney.py` 当前有测试：

- `test_generated_schemas_match_contract_generation()` 比较 checked-in generated module 与 `generate_eastmoney_schemas.render_schemas_module()`。

一刀切调整：

- 删除 `pipeline/scheduler/scripts/generate_eastmoney_schemas.py`。
- 删除 `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`。
- 删除 EastMoney 专用 schema generation 测试。
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py` 改为从统一生成模块的 schema map 读取 schema。
- 保留 `eastmoney_typed_schema(endpoint)`、`eastmoney_schema(endpoint)` 和 `EASTMONEY_SCHEMAS` 等对外接口形态时，只作为统一生成模块的 thin wrapper，不再有 EastMoney 独立 schema 源。

#### Common schema conversion

`pipeline/scheduler/src/scheduler/defs/common/schema.py` 当前：

- `typed_schema(fields)` 从 `(name, pa.DataType)` 构造 schema。
- `typed_table(rows, schema, converters=None)` 根据 schema 做通用类型转换。
- `_default_converter(dtype)` 根据 PyArrow type 选择 `to_date32`、`to_float64`、`to_int64`、`to_bool`、`to_timestamp`、`to_time32_ms`、`to_string`。

一刀切调整：

- 保留 `typed_table()`，它是 payload 到 typed `pa.Table` 的转换执行逻辑，不是字段事实源。
- 删除 `typed_schema()`，除非仍有非 dataset 场景使用。当前检索到的生产代码中它只在 `common/schema.py` 定义，未作为 dataset schema 事实源使用。
- Parquet adapter 只提供 expected schema，不替代 source-specific flatten、API response validation 和 value conversion。

## 设计决策

### 决策 1：只保留一个 Parquet schema 生成入口

新增：

- `pipeline/contract_tools/src/fleur_contracts/adapters/parquet.py`
- `pipeline/scheduler/scripts/generate_parquet_schemas.py`
- `pipeline/scheduler/src/scheduler/defs/generated/parquet_schemas.py`

删除：

- `pipeline/scheduler/scripts/generate_eastmoney_schemas.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`

不新增 runtime bridge。scheduler source modules 不在 definitions 加载时解析 YAML，只 import checked-in generated Python module。

理由：

- 当前 source business code 已有测试禁止 import `fleur_contracts`。
- generated module 可以 review diff，避免 Dagster definitions 加载时每次重新解析所有 YAML。
- EastMoney 现有生成路径证明 checked-in generated schema 在本项目可接受；本次改造把它推广为全局唯一生成路径。

### 决策 2：统一生成模块覆盖所有 18 个 dataset

`pipeline/scheduler/src/scheduler/defs/generated/parquet_schemas.py` 必须覆盖全部 dataset：

- `baostock__query_history_k_data_plus_daily`
- `baostock__query_stock_basic`
- `eastmoney__balance`
- `eastmoney__cashflow_sq`
- `eastmoney__cashflow_ytd`
- `eastmoney__dividend_allotment`
- `eastmoney__dividend_main`
- `eastmoney__equity_history`
- `eastmoney__income_sq`
- `eastmoney__income_ytd`
- `jiuyan__action_field`
- `jiuyan__action_field_compacted`
- `jiuyan__industry_list`
- `jiuyan__industry_ocr`
- `jiuyan__industry_ocr_snapshot`
- `sina__trade_calendar`
- `ths__limit_up_pool`
- `ths__limit_up_pool_compacted`

统一生成模块必须提供：

- 每个 dataset 的 schema constant。
- `PARQUET_SCHEMAS: dict[str, pa.Schema]`。
- `PARQUET_SCHEMA_HASHES: dict[str, str]`。
- `CONTRACT_SCHEMA_HASHES: dict[str, str]`。
- `CONTRACT_VERSIONS: dict[str, int]`。

命名规则：

- schema constant 使用 dataset upper snake，例如 `BAOSTOCK_QUERY_STOCK_BASIC_SCHEMA`。
- map key 使用 dataset 名，例如 `"baostock__query_stock_basic"`。

### 决策 3：compact schema 也来自自己的 contract

`jiuyan__action_field_compacted` 和 `ths__limit_up_pool_compacted` 不是简单复用 source-only daily dataset schema 的隐式事实源。它们必须从自己的 dataset contract 生成 schema。

当前 compact code 通过读取 daily partitions 后 `pa.concat_tables()` 输出 yearly partition table。迁移后必须在 compact asset 返回前 cast 或校验为 compacted dataset schema。

涉及文件：

- `pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/action_field_compact.py`
- `pipeline/scheduler/src/scheduler/defs/sources/ths/limit_up_pool_compact.py`

### 决策 4：S3IOManager 强制 schema 校验

本次不是只替换 schema 常量。S3 写入边界也必须成为 schema 闭环的一部分。

改造 `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py`：

- 根据 Dagster asset key 推导 dataset 名。
- 从 generated module 获取 expected schema。
- latest snapshot 写入前校验 `table.schema == expected_schema`。
- partitioned 写入前逐 partition 校验 `table.schema == expected_schema`。
- mismatch 直接失败，不自动 cast。

不自动 cast 的原因：

- source conversion 函数才负责把 payload 转成 typed value。
- IO manager 自动 cast 会掩盖 source 转换错误。
- schema mismatch 应该尽早暴露。

需要注意：

- `jiuyan__industry_images` 不是 Parquet table source，不应要求 generated schema。
- state/work-queue 类资产如果不走 `s3_io_manager`，不进入该校验。
- 对所有使用 `s3_io_manager` 的 source/compacted assets，必须能从 generated module 找到 dataset schema。

### 决策 5：测试最后补，但必须一次性补齐

实施顺序是先完成全部生产代码迁移，然后统一补测试。测试不是边改边跑的辅助迁移工具，而是迁移完成后的验收网。

最终必须补齐的测试包括：

- contract_tools Parquet adapter tests。
- generated schema current tests。
- scheduler exposed schema 与 generated schema map 的使用测试。
- S3IOManager schema mismatch fail-fast tests。
- source conversion output schema tests。
- architecture boundary tests 保持 source business code 不直接 import `fleur_contracts`。

### 决策 6：保留 nested JSON conversion ownership

本次一刀切只收敛最终 Parquet table schema，不把 JSON transform DSL 引入 contracts。

明确保留在 source code 中的内容：

- `JIUYAN_ACTION_FIELD_OUTER_COLUMNS`
- `JIUYAN_ACTION_FIELD_STOCK_COLUMNS`
- `JIUYAN_ACTION_FIELD_ACTION_INFO_COLUMNS`
- `THS_LIMIT_UP_POOL_INFO_COLUMNS`
- OCR aliases 和 normalization 逻辑
- EastMoney endpoint configs、OpenAPI/generated field names、unknown field 计数逻辑
- BaoStock API request field order

必须删除或改造的是与 output schema 完全重复、没有 transform 语义的字段列表；不能删除表达 nested JSON path、数组展开、API 参数顺序或 unknown field 计数语义的字段组。

风险控制：

- `S3IOManager` schema equality 只校验最终 `pa.Table` schema。
- source conversion code 继续负责 nested JSON 到 rows 的语义转换。
- 迁移完成后统一补 conversion-level tests，验证典型嵌套 payload 的 row count、字段值、unknown field count 和 final schema。

不采用 contracts transform DSL 的原因：

- 当前 contracts 的边界是 raw ingestion 字段链路，不是 source payload parser 规范。
- 多层 JSON transform 里包含程序逻辑、异常处理、分页语义和 aliases，强行 YAML 化会引入新的复杂度和第二套执行引擎。
- 现有 source code 已经是这些远端响应结构的 owning boundary；本次只删除 schema 重复，不迁移 parser ownership。

## 目标

1. `pipeline/contracts/datasets/*.yml` 的 `parquet.fields` 成为 Dagster source assets 写 S3 Parquet 的唯一 schema 事实源。
2. 删除 scheduler 内所有 dataset-level 手写 `pa.schema(...)`。
3. 删除 EastMoney 专用 schema generator 和 generated schema module。
4. `contract_tools.adapters.parquet` 成为唯一 PyArrow type string parser。
5. 所有使用 `s3_io_manager` 写入 Parquet 的 assets 在写入前由 IO manager 强制校验 schema。
6. ClickHouse raw sync 继续从 contract adapter 构造 specs。
7. dbt staging 所有权不回退到 contracts。

## 非目标

1. 不迁移 dbt staging 字段事实。
2. 不改变 S3 object layout。
3. 不改变 ClickHouse raw sync staging/replace 协议。
4. 不把 source-specific API request fields、flatten 规则、OCR aliases、EastMoney endpoint configs 放进 contracts。
5. 不自动 cast IO manager 收到的错误 schema。
6. 不保留旧手写 schema 常量作为兼容 fallback。
7. 不把 nested JSON flatten、row explosion、aliases 或 unknown field 计数逻辑生成化或 YAML 化。

## 一刀切实施方案

### Step 1：新增 contract_tools Parquet adapter

新增文件：

- `pipeline/contract_tools/src/fleur_contracts/adapters/parquet.py`

必须公开：

```python
@dataclass(frozen=True)
class ParquetFieldContract:
    name: str
    type: str
    nullable: bool


@dataclass(frozen=True)
class ParquetSchemaContract:
    dataset: str
    version: int
    contract_schema_hash: str
    source_schema_hash: str
    parquet_schema_hash: str
    source_asset_key: tuple[str, ...]
    storage_mode: str
    partition_key_name: str | None
    fields: tuple[ParquetFieldContract, ...]
```

必须提供：

- `parquet_schema_hash(contract: DatasetContract) -> str`
- `parquet_schema_contract_from_dataset(contract: DatasetContract) -> ParquetSchemaContract`
- `parquet_schema_contracts(contracts: Sequence[DatasetContract]) -> tuple[ParquetSchemaContract, ...]`
- `pyarrow_type_from_contract(type_text: str) -> pa.DataType`
- `pyarrow_schema_from_contract(contract: DatasetContract) -> pa.Schema`
- `pyarrow_schema_by_dataset(contracts: Sequence[DatasetContract]) -> dict[str, pa.Schema]`

必须支持当前 contract/scheduler 已出现的 type 字符串：

- `string`
- `date32[day]`
- `date32`
- `bool`
- `int8`
- `int32`
- `int64`
- `float64`
- `double`
- `timestamp[ns]`
- `timestamp[ns, tz=UTC]`
- `time32[ms]`

要求：

- adapter 不依赖 scheduler。
- adapter 不访问 S3、ClickHouse 或远端 API。
- unsupported type 必须抛出带 dataset/field/type 上下文的错误。

### Step 2：新增全局 scheduler generated schema

新增：

- `pipeline/scheduler/scripts/generate_parquet_schemas.py`
- `pipeline/scheduler/src/scheduler/defs/generated/__init__.py`
- `pipeline/scheduler/src/scheduler/defs/generated/parquet_schemas.py`

生成器必须：

- 使用 `fleur_contracts.loader.load_registry()` 加载 registry。
- 使用 `fleur_contracts.adapters.parquet` 解析 type 和 schema。
- 输出全部 18 个 dataset schema。
- 输出 schema/hash/version maps。
- 支持 `--check`。

生成文件必须：

- 不读取 YAML。
- 不 import `fleur_contracts`。
- 只 import `pyarrow as pa`。
- 提供常量和 maps 给 scheduler runtime 使用。

如新增 generated 目录导致 lint 噪声，更新：

- `pipeline/pyproject.toml` 的 `tool.ruff.extend-exclude`。

### Step 3：删除 EastMoney 专用 schema 生成链路

删除：

- `pipeline/scheduler/scripts/generate_eastmoney_schemas.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`

调整：

- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/__init__.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py`
- `pipeline/scheduler/tests/unit/sources/eastmoney/test_eastmoney.py`

保留：

- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/fields.py`
- `pipeline/scheduler/scripts/extract_eastmoney_schema_fields.py`

原因：

- `fields.py` 当前服务于 OpenAPI 字段顺序/unknown field 计数，不等同于 Parquet schema。
- `schemas.py` 和 `generate_eastmoney_schemas.py` 是重复 schema 生成路径，必须删除。

### Step 4：替换所有 source schema 常量

调整文件：

- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/src/scheduler/defs/http/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/sources/sina/trade_calendar.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py`

要求：

- 删除 dataset-level 手写 `pa.schema(...)`。
- 从 `scheduler.defs.generated.parquet_schemas` 导入 schema constants 或 map。
- 所有 source conversion 函数返回的 table schema 必须等于 generated schema。
- Compact 输出必须校验为 compacted dataset schema。

允许保留：

- source-specific nested payload column group constants。
- API request field constants。
- `string_schema(columns)` helper。
- `typed_table()`。

必须删除或改造：

- 与 schema names 完全重复、没有 API/flatten 语义的 column list。
- EastMoney schema-specific wrapper 对旧 generated module 的依赖。
- `SNAPSHOT_SCHEMA_VERSION`。

### Step 5：S3IOManager 加入强制 schema 校验

调整文件：

- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py`

新增行为：

- `handle_output()` 在空表检查之后、写入之前校验 schema。
- latest snapshot：校验单个 `pa.Table`。
- partitioned：逐 partition 校验每个 `pa.Table`。
- schema mismatch 抛出 `ValueError`，错误信息包含 asset key、partition key、expected schema、actual schema。

实现细节：

- 从 `context.asset_key` 推导 dataset name：asset key path 最后一段即 dataset。
- 使用 generated module 的 `PARQUET_SCHEMAS` 查 expected schema。
- 如果 asset 使用 `s3_io_manager` 但没有 generated schema，直接失败。

### Step 6：metadata 写入 contract/schema hash

调整：

- `pipeline/scheduler/src/scheduler/defs/asset_contracts.py`
- source asset metadata 构造处
- compacted asset metadata 构造处
- `S3IOManager.handle_output()` 输出 metadata

要求：

- 每个 S3 Parquet asset materialization metadata 至少包含：
  - `contract_dataset`
  - `contract_version`
  - `contract_schema_hash`
  - `parquet_schema_hash`

实现方式：

- 优先由 `S3IOManager` 根据 asset key 和 generated maps 自动添加，避免每个 source asset 手写。
- 不要求 source asset decorator 的 static metadata 全部手工写入 hash。

### Step 7：生产代码全部完成后统一补测试

先完成 Step 1 到 Step 6 的生产代码迁移，再补测试。

必须新增或修改的测试：

#### contract_tools

- `pipeline/contract_tools/tests/test_parquet_adapter.py`
  - adapter 能加载 18 个 dataset。
  - adapter 输出的 `pa.Schema` 与 contract `parquet.fields` 一致。
  - 所有当前 type 字符串被支持。
  - unsupported type 报错清晰。
  - `parquet_schema_hash()` 对字段名、类型、nullable 变化敏感。

#### scheduler generated schema

- 新增或修改 scheduler 测试：
  - `generate_parquet_schemas.py --check` current。
  - generated module 覆盖全部 18 个 dataset。
  - generated maps 包含 hash/version。

#### source conversion

- BaoStock tests：`stock_basic_response_to_table()`、`k_history_daily_response_to_table()` 输出 schema 等于 generated schema。
- HTTP schema tests：JiuYan action_field、industry_list、THS limit_up_pool 输出 schema 等于 generated schema。
- Sina tests：`trade_calendar_dates_to_table()` 输出 schema 等于 generated schema。
- OCR tests：OCR result 和 OCR snapshot 输出 schema 等于 generated schema。
- EastMoney tests：`eastmoney_schema(endpoint)` 输出 schema 等于 generated schema map。
- Compact tests：compacted output schema 等于 compacted dataset generated schema。

#### S3IOManager

- latest snapshot schema mismatch fail-fast。
- partitioned schema mismatch fail-fast，错误信息包含 partition key。
- missing generated schema fail-fast。
- correct schema writes normally。

#### architecture boundary

- 保留 `test_source_business_code_does_not_parse_contract_registry`。
- 新增检查：`scheduler.defs.generated.parquet_schemas` 不 import `fleur_contracts`。
- 允许 generator script import `fleur_contracts`。

## 禁止模式

- 禁止在 `pipeline/scheduler/src/scheduler/defs/sources/**` 或 `defs/baostock/**` 直接 import `fleur_contracts`。
- 禁止在 scheduler source modules 中手写 dataset-level `pa.schema(...)`。
- 禁止新增第二套 PyArrow type string parser。
- 禁止保留 EastMoney 专用 schema generator。
- 禁止 IO manager 自动 cast 错误 schema。
- 禁止把 nested JSON transform 字段组当作重复 schema 常量误删。
- 禁止用 generated data_dict 反推 schema。
- 禁止让 dbt staging YAML 或 SQL 成为 raw ingestion schema 事实源。
- 禁止在 Dagster definitions 加载阶段连接 S3、ClickHouse 或远端 API 做 schema discovery。

## 验收命令

文档-only 阶段：

```bash
git diff --check
```

生产代码迁移完成且测试补齐后，统一运行：

```bash
cd pipeline
uv run python scheduler/scripts/generate_parquet_schemas.py --check
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run ruff format --check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests -q
cd scheduler
uv run dg check defs
```

涉及真实 S3 或 ClickHouse schema 时追加：

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

## 完成标准

代码层面：

- `rg -n "pa\\.schema\\(" pipeline/scheduler/src/scheduler/defs -g '*.py'` 不再命中 dataset-level schema 定义。允许测试、helper 或非 dataset 临时构造场景继续使用。
- `rg -n "generate_eastmoney_schemas|EASTMONEY_.*_SCHEMA|sources/eastmoney/generated/schemas" pipeline/scheduler` 无实现引用。
- `pipeline/scheduler/src/scheduler/defs/generated/parquet_schemas.py` 覆盖全部 18 个 dataset。
- `S3IOManager` 对所有 `s3_io_manager` 写入强制 schema equality。
- source business code 仍不直接 import `fleur_contracts`。

数据治理层面：

- 修改任意 raw ingestion 字段或类型时，只需要改 `pipeline/contracts/datasets/*.yml`。
- 重新生成后，scheduler Parquet schema、ClickHouse raw specs、dbt `sources.yml` 和 data_dict 都来自同一份 contract。
- dbt staging 不被重新纳入 contracts。

## 当前不确定项

以下不是事实结论，实施前需要用代码或运行命令确认：

- 所有使用 `s3_io_manager` 的 asset 是否都能通过 asset key path 最后一段匹配 dataset 名。若有例外，应在 generated schema lookup 层显式维护映射，不能回退到 source 模块手写。
- `validate_parquet.py` 对 partitioned dataset 固定检查 `year=2026` 是否足够。这个行为是当前事实，不在本文中直接更改。
- `typed_schema()` 是否还有非测试生产使用。当前检索没有发现 dataset schema 事实源使用，实施时可删除或保留为通用 helper。

## 结论

本 RFC 采用破坏性一刀切方案：新增唯一 Parquet adapter 和全局 generated schema，删除所有 scheduler 手写 dataset schema 和 EastMoney 专用 schema 生成链路，S3IOManager 在写入边界强制 schema equality。改造完成后，`pipeline/contracts/datasets/*.yml` 将成为 `source -> S3 Parquet -> ClickHouse raw` 的唯一字段和类型事实源。
