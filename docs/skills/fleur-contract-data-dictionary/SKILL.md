---
name: fleur-contract-data-dictionary
description: mono-fleur 的 raw 层数据契约、Parquet schema adapter、字段 glossary、dbt sources.yml 和 data_dict 生成工作流。用于新增或修改 pipeline/contracts/datasets、维护 parquet.fields/source/clickhouse_raw 字段事实、维护 external_description_zh/description_zh、运行 fleur-contracts 与 scheduler contract schema 边界校验、修复生成物漂移。
---

# Fleur Contract Data Dictionary

当任务涉及 `pipeline/contracts`、`pipeline/contract_tools`、scheduler Parquet schema 边界、`docs/references/data_dict`、dbt `sources.yml` 的 raw 字段事实、中文字段描述或生成物同步时，使用这个 skill。

## 核心边界

- 字段事实源是 `pipeline/contracts/datasets/*.yml` 和 `pipeline/contracts/glossary/*.yml`，范围到 ClickHouse raw 层为止。
- `parquet.fields` 是 `source -> S3 Parquet -> ClickHouse raw` 的 Parquet 字段、类型、nullable 和顺序事实源。
- `docs/references/data_dict/*.md`、`pipeline/elt/models/sources.yml` 是生成物，不作为字段事实源。
- dbt `staging.yml`、`stg_*.sql`、stg 字段描述和 tests 由 `pipeline/elt` 项目维护，不写入数据契约。
- 修改字段事实时先改 contract/glossary，再运行生成器；不要手工修 generated data_dict 或 dbt `sources.yml`。
- 所有 Python、dbt、contract 命令在 `pipeline/` 下通过 `uv run` 执行。
- source 业务代码不直接解析 contract；contract 读取集中在 `pipeline/contract_tools` 和相关 adapter。
- scheduler 允许 `scheduler.defs.contract_schemas` 和 `scheduler.defs.clickhouse.specs` 作为 contract boundary 直接 import `fleur_contracts`。
- scheduler source business code 和 `S3IOManager` 不直接 import `fleur_contracts`；运行时只 import `scheduler.defs.contract_schemas` 暴露的 schema maps/constants。

## 常用入口

- 数据集契约：`pipeline/contracts/datasets/*.yml`
- 字段 glossary：`pipeline/contracts/glossary/fields.yml`
- 表 glossary：`pipeline/contracts/glossary/tables.yml`
- 命名规则：`pipeline/contracts/naming_rules.yml`
- contract CLI：`pipeline/contract_tools/src/fleur_contracts/cli.py`
- 描述质量门禁：`pipeline/contract_tools/src/fleur_contracts/description_quality.py`
- Parquet adapter：`pipeline/contract_tools/src/fleur_contracts/adapters/parquet.py`
- Parquet adapter tests：`pipeline/contract_tools/tests/test_parquet_adapter.py`
- scheduler Parquet schema boundary：`pipeline/scheduler/src/scheduler/defs/contract_schemas.py`
- scheduler contract schema tests：`pipeline/scheduler/tests/unit/test_contract_schemas.py`
- data_dict 生成器：`pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py`
- dbt sources YAML 生成器：`pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`

## 工作流

1. 先读相关 dataset contract 和 glossary，不从 generated data_dict 反推字段事实。
2. 确认字段链路：`source.fields` -> `parquet.fields` -> `clickhouse_raw.fields`。
3. 修改字段、类型、nullable、`from` 映射、`glossary_key` 或中文描述。
4. 如果改动 `parquet.fields`、`storage_mode` 或 `partition_key_name`，运行 scheduler contract schema boundary tests。
5. 运行 contract 静态校验。
6. 写回生成物。
7. 再运行生成物一致性检查、Parquet adapter tests 和相关 scheduler schema tests。

## Parquet schema adapter 规则

- `fleur_contracts.adapters.parquet` 是唯一 PyArrow type parser 和 Parquet schema/hash adapter。
- adapter 输出 `ParquetFieldContract`、`ParquetSchemaContract`、`pyarrow_schema_from_contract()`、`pyarrow_schema_by_dataset()` 和 `parquet_schema_hash()`。
- `parquet_schema_hash()` 对字段名、类型、nullable 和字段顺序敏感。
- 支持的 contract Parquet 类型包括：`string`、`date32[day]`、`date32`、`bool`、`int8`、`int32`、`int64`、`float64`、`double`、`timestamp[ns]`、`timestamp[ns, tz=UTC]`、`time32[ms]`。
- unsupported type 报错必须包含 dataset、field 和 type 上下文。
- `scheduler.defs.contract_schemas` 只允许通过 Parquet adapter 取得 schema/type 表达，不新增第二套 type string parser。
- `S3IOManager` 会在写 S3 前用 `scheduler.defs.contract_schemas.PARQUET_SCHEMAS` 做 exact schema equality 校验；source conversion 必须产出正确 typed table，不依赖 IO manager 自动 cast。
- materialization metadata 由 `S3IOManager` 自动补充 `contract_dataset`、`contract_version`、`contract_schema_hash`、`source_schema_hash`、`parquet_schema_hash`。

基础命令：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
```

涉及真实存储或 ClickHouse schema 时追加：

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

## 中文描述规则

`source.fields[].external_description_zh`：

- 表达外源或供应商语境下的字段含义。
- 必须是中文自然语言描述，不能等于字段名，不能只是英文缩写或数据库列名。
- 金额、比例、日期、状态、布尔字段尽量说明单位、口径或取值语境。
- EastMoney 字段优先参考 `docs/references/openapi/eastmoney__*.yaml` 和 `docs/references/remote_endpoint/eastmoney__*.md`，不要凭字段名猜财报口径。
- 无法确认含义时使用规范格式：`待核实：供应商字段 <FIELD_NAME>，当前仅确认来自 <dataset> 原始响应。`，并在最终汇报中统计数量。

`glossary/fields.yml` 的 `description_zh`：

- 表达 mono-fleur 系统内 canonical 字段语义。
- 不写供应商特有字段名；供应商别名留在 dataset contract。
- 复用字段要比单个数据集更抽象。

禁止：

- `external_description_zh: TOTAL_ASSETS`
- `description_zh: open`
- “字段值”“相关信息”“数据字段”这类无意义中文。
- 为通过校验而伪造不确定字段的业务语义。

## 新增或修改 dataset

新增 raw 数据集时：

- 新增或更新 `pipeline/contracts/datasets/<dataset>.yml`。
- dataset 文件名、`dataset`、`clickhouse_raw.table` 保持一致。
- `raw_asset_key` 使用 `["clickhouse", "raw", dataset]`。
- `clickhouse_raw.fields[].from` 必须指向 `parquet.fields`。
- `parquet.fields` 的字段名、类型、nullable 和顺序变更后必须运行 scheduler contract schema boundary tests。
- `LowCardinality(String)` 字段必须写 `reason`。
- raw 字段变更后重新验证 scheduler Parquet schema boundary，并重新生成 dbt `sources.yml` 和 data_dict。

## 验收

contract-only 或 data_dict 变更至少运行：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
git diff --check
```

改了 `contract_tools` Python 代码时追加：

```bash
cd pipeline
uv run ruff check contract_tools/src contract_tools/tests
uv run ruff format --check contract_tools/src contract_tools/tests
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
```

如果改动影响 scheduler contract schema boundary、S3 IO manager、source conversion 或 raw sync adapter，追加：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/test_contract_schemas.py scheduler/tests/unit/storage/test_storage_and_services.py -q
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

如果改动影响 dbt models，再按 `AGENTS.md` 跑对应 dbt 最小门禁。
