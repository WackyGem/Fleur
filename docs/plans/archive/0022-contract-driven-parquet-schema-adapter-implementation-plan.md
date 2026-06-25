# Plan 0022: Contract-driven Parquet schema adapter implementation

日期：2026-06-01

状态：Draft

Superseded note（2026-06-01）：

- 本计划 Phase 2 中的 scheduler checked-in generated Parquet schema module 方案已被 `docs/plans/0022.2-direct-contract-parquet-schema-boundary-plan.md` 替代。
- 应保留的成果包括 `fleur_contracts.adapters.parquet`、source schema replacement、`S3IOManager` schema guard、materialization metadata 和 EastMoney 专用 Parquet schema 删除。
- 当前 scheduler schema/source-field boundary 是 `pipeline/scheduler/src/scheduler/defs/contract_schemas.py`，不再运行 `pipeline/scheduler/scripts/generate_parquet_schemas.py` 或 `pipeline/scheduler/scripts/extract_eastmoney_schema_fields.py`。

关联文档：

- `docs/RFC/archive/0011-contract-driven-parquet-schema-adapter.md`
- `docs/RFC/archive/0010-data-contract-registry-and-contract-tools.md`
- `docs/plans/0021-contract-scope-raw-layer-cleanup-plan.md`
- `docs/plans/0020-field-type-normalization-debt-remediation-plan.md`
- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/architecture/scheduler-module-boundaries.md`
- `docs/skills/fleur-contract-data-dictionary/SKILL.md`

## 1. 背景

RFC 0011 已决定一刀切收敛 Dagster source assets 写 S3 Parquet 时使用的 schema：`pipeline/contracts/datasets/*.yml` 的 `parquet.fields` 必须成为唯一字段和类型事实源。

当前仓库已经完成 Plan 0021 的 raw-layer scope 收缩，contract 不再拥有 dbt staging 字段事实。当前 contract registry 已覆盖 raw dataset 和 source-only dataset，ClickHouse raw specs 也已经从 `fleur_contracts.adapters.clickhouse` 派生。但 Dagster source 侧仍存在多处 dataset-level `pa.schema(...)`，EastMoney 仍有专用 schema 生成器，S3 IO manager 写入前也不强制校验表 schema 是否等于 contract schema。

本计划把 RFC 0011 转成开发实施顺序，用于指导一次性破坏性改造。

## 2. 目标

完成后应满足：

1. `pipeline/contracts/datasets/*.yml` 的 `parquet.fields` 是 `source -> S3 Parquet -> ClickHouse raw` 中 Parquet schema 的唯一事实源。
2. `pipeline/contract_tools/src/fleur_contracts/adapters/parquet.py` 提供唯一 PyArrow type parser、schema contract 和 schema hash。
3. `pipeline/scheduler/src/scheduler/defs/generated/parquet_schemas.py` 覆盖全部 18 个 dataset，scheduler runtime 只 import 该 checked-in generated module。
4. 删除 EastMoney 专用 Parquet schema generator、generated schema module 和 OpenAPI 字段顺序 generated fields；EastMoney 字段名由 contract `source.fields` 经 `scheduler.defs.contract_schemas.SOURCE_FIELD_NAMES` 提供。
5. 删除 scheduler source modules 中 dataset-level 手写 `pa.schema(...)`。
6. `S3IOManager` 在写入 S3 前对 latest snapshot 和 partitioned tables 做 schema equality 校验，mismatch fail fast。
7. source asset materialization metadata 自动包含 contract/schema hash 信息。
8. ClickHouse raw sync、dbt `sources.yml` 和 data_dict 继续从 contract 派生，dbt staging 所有权不回退到 contracts。

## 3. 非目标

本计划不做以下事情：

1. 不迁移 dbt staging 字段、YAML 或 SQL 到 contracts。
2. 不改变 S3 object layout、partition 目录规则或 ClickHouse raw sync staging/replace 协议。
3. 不把 nested JSON flatten、数组展开、OCR aliases、EastMoney endpoint config、BaoStock API request fields 放进 contracts。
4. 不实现 IO manager 自动 cast；source conversion 必须产出正确 typed table。
5. 不保留旧手写 schema 常量作为 fallback。
6. 不在 Dagster definitions 加载阶段读取 YAML、连接 S3/ClickHouse 或访问远端 API 做 schema discovery。
7. 不把 generated data_dict 当作 schema 输入。

## 4. 当前事实基线

已核验的当前事实：

- `pipeline/contracts/datasets/*.yml` 已包含 18 个 dataset，且每个 dataset 有 `parquet.fields`。
- `pipeline/contract_tools/src/fleur_contracts/schema.py` 已支持 source-only dataset：`clickhouse_raw` 和 `raw_asset_key` 可为空。
- `pipeline/contract_tools/src/fleur_contracts/loader.py` 已提供 `dataset_schema_hash()`、`source_schema_hash()`、`clickhouse_schema_hash()`，但没有 `parquet_schema_hash()`。
- `pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py` 已经能为 scheduler 构造 ClickHouse raw specs。
- `pipeline/contract_tools/src/fleur_contracts/validate_parquet.py` 只校验已存在 Parquet 文件与 contract 是否一致，不提供 scheduler 可复用 `pa.Schema` adapter。
- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py` 仍定义 `STOCK_BASIC_SCHEMA`、`K_HISTORY_DAILY_SCHEMA`。
- `pipeline/scheduler/src/scheduler/defs/http/schemas.py` 仍定义 `THS_LIMIT_UP_POOL_SCHEMA`、`JIUYAN_ACTION_FIELD_SCHEMA`、`JIUYAN_INDUSTRY_LIST_SCHEMA`。
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py` 仍定义 `JIUYAN_INDUSTRY_OCR_SCHEMA`。
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py` 仍定义 `JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA` 和 `SNAPSHOT_SCHEMA_VERSION`。
- `pipeline/scheduler/scripts/generate_eastmoney_schemas.py` 和 EastMoney generated schema/fields 链路已被删除。
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py` 现在从 `scheduler.defs.contract_schemas` 读取 schema 和 source field names。
- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py` 当前只校验对象类型、空表和 partition key，不校验 table schema。

## 5. 目标架构

改造后 schema 链路为：

```text
pipeline/contracts/datasets/*.yml
  -> fleur_contracts.adapters.parquet
  -> scheduler/scripts/generate_parquet_schemas.py
  -> scheduler.defs.generated.parquet_schemas
  -> source conversion functions
  -> S3IOManager schema equality check
  -> S3 Parquet
  -> ClickHouse raw sync specs
```

边界要求：

| 层 | 允许读取 contracts | 说明 |
|----|--------------------|------|
| `contract_tools` | 是 | contract parser、adapter、hash、validate/generate |
| `scheduler/scripts/generate_parquet_schemas.py` | 是 | 开发期生成 checked-in module |
| `scheduler.defs.generated.parquet_schemas` | 否 | 只 import `pyarrow as pa` |
| scheduler source business code | 否 | 只能 import generated schema constants/maps |
| `S3IOManager` | 否 | 只能 import generated schema maps |

## 6. 实施阶段

### Phase 0: 基线冻结和影响面确认

范围：

- `docs/RFC/archive/0011-contract-driven-parquet-schema-adapter.md`
- `pipeline/contracts/datasets/*.yml`
- `pipeline/scheduler/src/scheduler/defs/**`
- `pipeline/scheduler/tests/**`
- `pipeline/contract_tools/tests/**`

动作：

- 确认 18 个 dataset 名称与 `pipeline/contracts/datasets/*.yml` 文件一一对应。
- 列出所有使用 `s3_io_manager` 的 asset，确认 asset key path 最后一段是否等于 dataset 名。
- 搜索当前 dataset-level `pa.schema(...)`，分成三类：
  - 必须替换为 generated schema 的 dataset schema。
  - 可保留的 helper 或测试内临时 schema。
  - 表达非 Parquet dataset 语义的 schema，需单独说明。
- 确认 nested JSON conversion 字段组是否有 flatten/API 语义，避免误删。

完成标准：

- 后续 phase 的文件列表完整，不遗漏使用 `s3_io_manager` 写 Parquet 的 source asset。
- 若 asset key 到 dataset 名不能用最后一段推导，先记录显式 mapping 需求，不用手写 schema 回避。

Phase 7 测试记录：

- 增加架构测试覆盖 source business code 不直接 import `fleur_contracts`。
- 增加 generated schema 覆盖全部 contract datasets 的测试。

### Phase 1: 新增 contract_tools Parquet adapter

范围：

- `pipeline/contract_tools/src/fleur_contracts/adapters/parquet.py`
- `pipeline/contract_tools/src/fleur_contracts/loader.py`
- `pipeline/contract_tools/tests/test_parquet_adapter.py`

动作：

- 新增 `ParquetFieldContract` 和 `ParquetSchemaContract`。
- 新增 `parquet_schema_hash(contract)`，hash 至少对字段名、类型、nullable 和字段顺序敏感。
- 新增 `parquet_schema_contract_from_dataset(contract)` 和 `parquet_schema_contracts(contracts)`。
- 新增 `pyarrow_type_from_contract(type_text)` 和 `pyarrow_schema_from_contract(contract)`。
- 新增 `pyarrow_schema_by_dataset(contracts)`。
- 支持当前 contract 已使用的类型字符串：
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
- unsupported type 必须抛出包含 dataset、field、type 的错误；如果底层 parser 函数只收到 type 字符串，调用方负责补上下文。

完成标准：

- `contract_tools` 中只有 `adapters/parquet.py` 负责 PyArrow type string 到 `pa.DataType` 的映射。
- `loader.py` 或 parquet adapter 对外提供 `parquet_schema_hash()`，后续 generator 和 metadata 不重复实现 hash。

Phase 7 测试记录：

- adapter 可加载 18 个 dataset。
- adapter 输出 schema 与 contract `parquet.fields` 完全一致。
- unsupported type 报错包含可定位上下文。
- hash 对字段顺序、字段名、类型、nullable 变化敏感。

### Phase 2: 新增全局 scheduler generated schema 模块

范围：

- `pipeline/scheduler/scripts/generate_parquet_schemas.py`
- `pipeline/scheduler/src/scheduler/defs/generated/__init__.py`
- `pipeline/scheduler/src/scheduler/defs/generated/parquet_schemas.py`
- `pipeline/pyproject.toml`（仅在 ruff 需要排除 generated 文件时修改）

动作：

- 新增 generator script，通过 `fleur_contracts.loader.load_registry()` 和 `fleur_contracts.adapters.parquet` 生成 PyArrow schema module。
- generator 支持默认写入和 `--check` 模式。
- generated module 覆盖全部 18 个 dataset。
- generated module 输出：
  - 每个 dataset 的 schema constant，例如 `BAOSTOCK_QUERY_STOCK_BASIC_SCHEMA`。
  - `PARQUET_SCHEMAS: dict[str, pa.Schema]`。
  - `PARQUET_SCHEMA_HASHES: dict[str, str]`。
  - `CONTRACT_SCHEMA_HASHES: dict[str, str]`。
  - `SOURCE_SCHEMA_HASHES: dict[str, str]`。
  - `CONTRACT_VERSIONS: dict[str, int]`。
  - 必要时输出 `SOURCE_ASSET_KEYS: dict[str, tuple[str, ...]]` 和 `STORAGE_MODES` / `PARTITION_KEY_NAMES`。
- generated module 不读取 YAML，不 import `fleur_contracts`，不连接外部系统。

完成标准：

- `cd pipeline && uv run python scheduler/scripts/generate_parquet_schemas.py --check` 可作为 current check。
- `scheduler.defs.generated.parquet_schemas` 可在 Dagster definitions 加载时安全 import。

Phase 7 测试记录：

- checked-in generated module 与 generator 输出一致。
- generated maps 覆盖所有 contract dataset，且无额外 dataset。
- generated module import graph 不包含 `fleur_contracts`。

### Phase 3: 删除 EastMoney 专用 schema 生成链路

范围：

- 删除 `pipeline/scheduler/scripts/generate_eastmoney_schemas.py`
- 删除 `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`
- 修改 `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/__init__.py`
- 修改 `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py`
- 修改 `pipeline/scheduler/tests/unit/sources/eastmoney/test_eastmoney.py`

动作：

- `eastmoney_typed_schema(endpoint)` 改为从 `scheduler.defs.generated.parquet_schemas.PARQUET_SCHEMAS` 读取 schema。
- 保留 `eastmoney_schema(endpoint)` 对外接口。
- 删除 `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/fields.py` 和 `pipeline/scheduler/scripts/extract_eastmoney_schema_fields.py`；contract `source.fields` 已覆盖字段顺序和 unknown field 计数所需的允许字段集合。
- 删除 `test_generated_schemas_match_contract_generation()` 中对 EastMoney 专用 generator 的断言，改为断言 EastMoney endpoint schema 和 field names 来自 `scheduler.defs.contract_schemas`。

完成标准：

- `rg -n "generate_eastmoney_schemas|extract_eastmoney_schema_fields|sources/eastmoney/generated" pipeline/scheduler` 无实现引用。
- EastMoney rows conversion、unknown field count 和 endpoint configs 语义不变。

Phase 7 测试记录：

- EastMoney 8 个 endpoint 的 `eastmoney_schema(endpoint)` 等于 `contract_schemas.PARQUET_SCHEMAS`。
- EastMoney field-name 测试验证 `eastmoney_business_field_names()` 来自 `contract_schemas.SOURCE_FIELD_NAMES`。

### Phase 4: 替换 scheduler source schema 常量

范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/src/scheduler/defs/http/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/sources/sina/trade_calendar.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/schema.py`
- `pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/action_field_compact.py`
- `pipeline/scheduler/src/scheduler/defs/sources/ths/limit_up_pool_compact.py`

动作：

- 删除 dataset-level 手写 `pa.schema(...)`，从 `scheduler.defs.generated.parquet_schemas` 导入对应 constants 或 map。
- BaoStock：
  - `STOCK_BASIC_SCHEMA`、`K_HISTORY_DAILY_SCHEMA` 改为 generated schema alias。
  - `STOCK_BASIC_FIELDS` 可从 schema names 派生。
  - `K_HISTORY_DAILY_FIELDS` 若用于 BaoStock API request order，可保留为独立 API 字段顺序，并增加与 schema names 一致的测试。
- HTTP shared schemas：
  - `THS_LIMIT_UP_POOL_SCHEMA`、`JIUYAN_ACTION_FIELD_SCHEMA`、`JIUYAN_INDUSTRY_LIST_SCHEMA` 改为 generated schema alias。
  - 删除与 schema names 完全重复的 column list；保留 `JIUYAN_ACTION_FIELD_OUTER_COLUMNS`、`JIUYAN_ACTION_FIELD_STOCK_COLUMNS`、`JIUYAN_ACTION_FIELD_ACTION_INFO_COLUMNS`、`THS_LIMIT_UP_POOL_INFO_COLUMNS` 等表达 flatten 语义的字段组。
- Sina：
  - `trade_calendar_dates_to_table()` 使用 `SINA_TRADE_CALENDAR_SCHEMA`。
- OCR：
  - `JIUYAN_INDUSTRY_OCR_SCHEMA`、`JIUYAN_INDUSTRY_OCR_SNAPSHOT_SCHEMA` 改为 generated schema alias。
  - 删除 `SNAPSHOT_SCHEMA_VERSION`，metadata 改用 contract/schema hash。
  - 保留 OCR aliases、normalization、snapshot row count validation。
- Compact：
  - compacted dataset 使用自己的 generated schema，不隐式复用 source-only daily schema。
  - compact 输出返回前校验或选择/构造为 compacted dataset schema；不要在 IO manager 才发现字段不一致。
- Common helper：
  - 保留 `typed_table()`。
  - `typed_schema()` 如无生产使用可删除；如仍用于非 dataset 临时场景，可保留但不得成为 dataset schema 事实源。

完成标准：

- source conversion 函数产出的 table schema 等于 generated schema。
- nested JSON transform、API request fields、unknown field count、OCR aliases 行为不因删除 schema 重复字段列表而变化。

Phase 7 测试记录：

- BaoStock、HTTP、Sina、OCR、EastMoney、compact conversion 输出 schema 全部等于 generated schema。
- 对保留的 API request field order 增加顺序一致性测试。

### Phase 5: S3IOManager 写入前强制 schema equality

范围：

- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py`
- `pipeline/scheduler/tests/unit/io_managers/` 或现有 S3 IO manager 测试位置

动作：

- `handle_output()` 在 `validate_table()` / `validate_partition_tables()` 和 empty check 之后、写入 S3 之前执行 schema 校验。
- 根据 `context.asset_key.path[-1]` 推导 dataset 名。
- 从 `PARQUET_SCHEMAS` 获取 expected schema。
- latest snapshot 校验单个 `pa.Table.schema == expected_schema`。
- partitioned 校验每个 partition table 的 schema；错误信息包含 asset key、partition key、expected schema 和 actual schema。
- 使用 `s3_io_manager` 但没有 generated schema 的 asset 必须 fail fast。唯一例外是该 asset 不写 Parquet 或不使用此 IO manager。
- 不做自动 cast。

完成标准：

- 所有 S3 Parquet 写入在 IO manager 边界有 contract schema equality guard。
- 错误信息足够定位具体 asset 和 partition。

Phase 7 测试记录：

- latest snapshot schema mismatch fail fast。
- partitioned schema mismatch fail fast，错误包含 partition key。
- missing generated schema fail fast。
- correct schema 正常调用 writer。

### Phase 6: metadata 自动写入 contract/schema hash

范围：

- `pipeline/scheduler/src/scheduler/defs/asset_contracts.py`
- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py`
- source asset 或 compact asset 中已有手写 schema metadata 的位置

动作：

- 优先由 `S3IOManager.handle_output()` 根据 asset key 自动添加：
  - `contract_dataset`
  - `contract_version`
  - `contract_schema_hash`
  - `source_schema_hash`
  - `parquet_schema_hash`
- 如果某些 metadata 必须出现在 static asset definition metadata 中，提供一个轻量 helper 从 generated maps 构造，不让 source modules 手写 hash。
- OCR snapshot 删除 `snapshot_schema_version` 后，使用 contract/schema hash 表达 schema 版本。
- ClickHouse raw asset 现有 `clickhouse_schema_hash` 保持不变。

完成标准：

- 每个通过 `s3_io_manager` 写 Parquet 的 materialization metadata 都能追溯到 contract version/hash。
- 不再有 source asset 手写独立 schema version 常量。

Phase 7 测试记录：

- S3IOManager metadata 包含 contract/schema hash。
- OCR snapshot metadata 不再依赖 `SNAPSHOT_SCHEMA_VERSION`。

### Phase 7: 统一补齐测试和质量门禁

RFC 0011 明确要求先完成全部生产代码迁移，再统一补测试。本阶段承接 Phase 0-6 记录的测试项，不把测试范围留到后续计划。

必须新增或修改的测试：

- `pipeline/contract_tools/tests/test_parquet_adapter.py`
- scheduler generated schema current 测试。
- BaoStock conversion schema 测试。
- HTTP conversion schema 测试。
- Sina trade calendar schema 测试。
- OCR result 和 OCR snapshot schema 测试。
- EastMoney schema wrapper 测试。
- Compact output schema 测试。
- S3IOManager schema guard 和 metadata 测试。
- 架构边界测试：
  - source business code 不 import `fleur_contracts`。
  - generated parquet schema module 不 import `fleur_contracts`。
  - generator script 允许 import `fleur_contracts`。

完成标准：

- 测试覆盖 RFC 0011 所有禁止回退的边界。
- 不存在只靠人工 review 维护的 schema 事实源。

## 7. 验证命令

文档-only 阶段：

```bash
git diff --check
```

生产代码迁移完成后：

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

涉及真实对象或已部署 raw 表时追加：

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

## 8. 禁止模式

- 禁止在 `pipeline/scheduler/src/scheduler/defs/sources/**` 或 `pipeline/scheduler/src/scheduler/defs/baostock/**` 直接 import `fleur_contracts`。
- 禁止在 scheduler source modules 中新增 dataset-level `pa.schema(...)`。
- 禁止新增第二套 PyArrow type string parser。
- 禁止保留 EastMoney 专用 schema generator。
- 禁止 IO manager 自动 cast 错误 schema。
- 禁止误删表达 nested JSON path、数组展开、API 参数顺序、aliases 或 unknown field count 的字段组。
- 禁止让 dbt staging YAML/SQL 重新成为 raw ingestion schema 事实源。
- 禁止在 Dagster definitions 加载阶段做外部 schema discovery。

## 9. 完成标准

代码层面：

- `pipeline/contract_tools/src/fleur_contracts/adapters/parquet.py` 是唯一 Parquet adapter。
- `pipeline/scheduler/src/scheduler/defs/contract_schemas.py` 覆盖全部 contract dataset。
- `rg -n "pa\\.schema\\(" pipeline/scheduler/src/scheduler/defs -g '*.py'` 不再命中 dataset-level schema 定义；允许 helper、测试或非 dataset 临时构造场景。
- `rg -n "generate_eastmoney_schemas|extract_eastmoney_schema_fields|sources/eastmoney/generated" pipeline/scheduler` 无实现引用。
- `S3IOManager` 对所有 `s3_io_manager` 写入强制 schema equality。
- source business code 不直接 import `fleur_contracts`。

数据治理层面：

- 修改 raw ingestion 字段或 Parquet 类型时，只需要修改 `pipeline/contracts/datasets/*.yml` 并重新生成。
- scheduler Parquet schema、ClickHouse raw specs、dbt `sources.yml` 和 data_dict 均来自同一份 contract registry。
- dbt staging 字段事实仍由 `pipeline/elt` 维护，不回写到 contracts。

## 10. 建议执行顺序

1. Phase 0：先冻结基线，确认所有 `s3_io_manager` assets 都能映射到 dataset。
2. Phase 1：先落 `contract_tools` adapter，避免 scheduler generator 自己解析 type。
3. Phase 2：生成全局 checked-in schema module。
4. Phase 3：删除 EastMoney 专用 schema 链路。
5. Phase 4：替换 scheduler source schema 常量。
6. Phase 5：最后打开 IO manager schema guard，避免迁移中途被半改状态阻断。
7. Phase 6：补齐 metadata。
8. Phase 7：统一补测试并跑完整质量门禁。
