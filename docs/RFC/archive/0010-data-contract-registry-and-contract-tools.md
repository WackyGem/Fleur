# RFC 0010: 数据契约注册表、命名词汇表与 contract tools 设计

状态：草案（2026-06-01）

## 摘要

本文档定义 mono-fleur 的数据契约治理方案，用于统一管理外源字段、系统规范字段、中文描述、类型转换、dbt staging 命名和生成物校验。

核心决策：

1. **raw 层保留外源命名**：HTTP/TCP source、S3 Parquet、ClickHouse raw 在 stg 之前不做系统级字段重命名。
2. **stg 层是规范命名边界**：dbt staging models 第一次把外源字段映射为 mono-fleur canonical field names。
3. **contract registry 是字段事实源**：字段类型、转换链路、raw/stg 映射、schema version 和中文描述不再以 Markdown data_dict 为唯一事实源。
4. **glossary 管理系统统一语义**：`stock_code`、`trade_date`、`report_date` 等系统规范字段的中文名称、中文描述、命名规则和外源 aliases 集中维护。
5. **contract_tools 是独立 workspace 工程包**：代码生成、静态校验、Parquet schema 校验、ClickHouse schema 校验和 dbt YAML 生成放在 `pipeline/contract_tools`，不放进 scheduler 或 elt。
6. **data_dict 和 dbt YAML 是生成物或强校验对象**：人类可读文档继续保留，但由 contract 生成或被 contract 严格校验。

## 背景

当前数据链路已经形成稳定分层：

```text
http/tcp source payload
  -> Dagster source assets
  -> S3 Parquet
  -> Dagster ClickHouse raw sync assets
  -> ClickHouse raw tables
  -> dbt source()/staging models
```

当前问题：

- 外源字段名保持了供应商风格，例如 `SECUCODE`、`REPORT_DATE`、`code`、`date`，但系统内尚无统一 canonical naming registry。
- 中文字段描述散落在 `docs/references/data_dict/*.md`、dbt YAML、接口文档和人工说明中。
- `ClickHouseRawTableSpec`、dbt `sources.yml`、staging YAML 和 data_dict 都含有字段事实，存在重复维护和漂移风险。
- stg 层需要承担字段重命名和轻清洗，但缺少一份明确的外源字段到系统字段映射。
- 当前表数量有限，字段类型和结构已相对稳定，适合一次性建立治理机制，而不是长期保留双轨。

## 目标

1. 为当前 ClickHouse raw 和 dbt stg 数据集建立机器可读 contract。
2. 区分外源表/字段命名与 mono-fleur 系统规范命名。
3. 集中维护中文描述，避免描述散落在 SQL、dbt YAML 和 Markdown 中。
4. 通过生成器和校验器维护 dbt YAML、data_dict Markdown、ClickHouse raw specs 的一致性。
5. 提供可在 CI 和本地运行的 contract CLI。
6. 保持 Dagster definitions 加载阶段只读取本地文件，不访问 S3、ClickHouse 或远端服务。

## 非目标

1. 不引入 DataHub、OpenMetadata、Amundsen 等外部数据目录平台。
2. 不把 contract registry 设计成指标语义层或 mart 层口径中心。
3. 不让 dbt 直接读取 S3 Parquet。
4. 不把 stg SQL 模板化作为第一版目标；stg SQL 仍手写。
5. 不在 raw 层提前进行系统级字段重命名。
6. 不把 contract tools 拆到独立仓库。

## 设计原则

### 外源命名和系统命名分离

外源命名必须保留到 raw 层，便于追溯供应商字段和快速对照源文档：

```text
source payload field
  -> parquet field
  -> clickhouse raw field
```

系统命名从 stg 层开始：

```text
clickhouse raw field
  -> dbt staging canonical field
```

示例：

| 外源字段 | ClickHouse raw | dbt stg | canonical glossary |
|----------|----------------|---------|--------------------|
| `SECUCODE` | `SECUCODE` | `stock_code` | `stock_code` |
| `code` | `code` | `stock_code` | `stock_code` |
| `TRADE_DATE` | `TRADE_DATE` | `trade_date` | `trade_date` |
| `REPORT_DATE` | `REPORT_DATE` | `report_date` | `report_date` |

### 中文描述分层

中文描述分为三类：

| 字段 | 归属 | 说明 |
|------|------|------|
| `external_description_zh` | dataset contract | 外源文档或供应商语境下的字段说明 |
| `description_zh` | glossary | mono-fleur 系统内统一语义 |
| `dataset_note_zh` | dataset contract | 当前数据集特有的异常值、清洗规则、口径差异 |

生成 data_dict 或 dbt docs 时，可以组合这些描述，但事实来源仍是 contract 和 glossary。

### Contract registry 和 contract_tools 分离

契约数据和工具代码分离：

```text
pipeline/
  contracts/
    datasets/
    glossary/
    naming_rules.yml

  contract_tools/
    pyproject.toml
    src/fleur_contracts/
    tests/
```

这样 contract YAML 可以被 review 为数据变更，contract_tools 可以作为普通 Python 工程做测试、类型检查和 CLI 发布。

## 目标目录结构

```text
pipeline/
  contract_tools/
    pyproject.toml
    src/fleur_contracts/
      __init__.py
      cli.py
      schema.py
      loader.py
      hash.py
      validate.py
      generate.py
      validate_parquet.py
      validate_clickhouse.py
      adapters/
        clickhouse.py
        dbt.py
        data_dict.py
        parquet.py
      templates/
        data_dict.md.j2
    tests/

  contracts/
    datasets/
      baostock__query_history_k_data_plus_daily.yml
      baostock__query_stock_basic.yml
      sina__trade_calendar.yml
      jiuyan__industry_list.yml
      jiuyan__industry_ocr_snapshot.yml
      jiuyan__action_field_compacted.yml
      ths__limit_up_pool_compacted.yml
      eastmoney__balance.yml
      eastmoney__cashflow_sq.yml
      eastmoney__cashflow_ytd.yml
      eastmoney__dividend_allotment.yml
      eastmoney__dividend_main.yml
      eastmoney__equity_history.yml
      eastmoney__income_sq.yml
      eastmoney__income_ytd.yml
    glossary/
      fields.yml
      tables.yml
    naming_rules.yml
    README.md
```

`pipeline/pyproject.toml` 的 workspace members 应加入：

```toml
[tool.uv.workspace]
members = [
    "scheduler",
    "contract_tools",
]
```

`pipeline/scheduler/pyproject.toml` 应声明对 `contract-tools` workspace package 的依赖，因为 scheduler definitions 需要通过 `fleur_contracts.adapters.clickhouse` 构造 `ClickHouseRawTableSpec`。该依赖只能读取本地 contract 文件，不能在 definitions 加载阶段连接外部服务。

`pipeline/contract_tools/pyproject.toml` 只承载 contract 治理工具依赖，例如 YAML/Pydantic/Jinja2、Parquet schema 校验和 ClickHouse metadata 校验所需依赖。它不应依赖 `scheduler`，避免形成反向耦合。

## Contract 格式

dataset contract 示例：

```yaml
dataset: eastmoney__balance
version: 1
owner: data
grain: one row per stock code per report date

external:
  provider: eastmoney
  source_table_name: eastmoney__balance
  source_description_zh: 东方财富资产负债表数据

source_asset_key: ["source", "eastmoney__balance"]
raw_asset_key: ["clickhouse", "raw", "eastmoney__balance"]

parquet:
  storage_mode: partitioned
  partition_key_name: year
  fields:
    - name: SECUCODE
      type: string
      nullable: false
      external_description_zh: 证券代码
    - name: REPORT_DATE
      type: date32
      nullable: false
      external_description_zh: 报告期

clickhouse_raw:
  database: raw
  table: eastmoney__balance
  partition_strategy: year
  engine: MergeTree
  partition_by: year
  order_by: [SECUCODE, REPORT_DATE]
  preserve_external_names: true
  fields:
    - name: SECUCODE
      from: SECUCODE
      type: LowCardinality(String)
      glossary_key: stock_code
      external_description_zh: 证券代码
    - name: REPORT_DATE
      from: REPORT_DATE
      type: Date
      glossary_key: report_date
      external_description_zh: 报告期

dbt_staging:
  status: not_started
```

stg 已存在的数据集示例：

```yaml
dbt_staging:
  model: stg_baostock__query_history_k_data_plus_daily
  materialized: view
  primary_key: [stock_code, trade_date]
  fields:
    - name: stock_code
      from: code
      glossary_key: stock_code
      type: String
      tests: [not_null]
    - name: trade_date
      from: date
      glossary_key: trade_date
      type: Date
      tests: [not_null]
```

## Glossary 格式

`pipeline/contracts/glossary/fields.yml` 示例：

```yaml
fields:
  stock_code:
    canonical_name: stock_code
    zh_name: 股票代码
    description_zh: A 股证券代码。用于标识上市公司或证券品种，通常来自交易所或数据供应商的证券编码字段。
    type_family: identifier
    naming:
      suffix: _code
    aliases:
      baostock: [code]
      eastmoney: [SECUCODE, SECURITY_CODE]
      ths: [code]

  trade_date:
    canonical_name: trade_date
    zh_name: 交易日期
    description_zh: A 股市场交易日日期。用于日频行情、交易日历和按交易日分区的数据。
    type_family: date
    naming:
      suffix: _date
    aliases:
      baostock: [date]
      sina: [trade_date]

  report_date:
    canonical_name: report_date
    zh_name: 报告期
    description_zh: 财务报表对应的报告截止日期，通常为季度末、半年末或年末。
    type_family: date
    naming:
      suffix: _date
    aliases:
      eastmoney: [REPORT_DATE]
```

## 命名规则

`pipeline/contracts/naming_rules.yml` 维护系统级规则：

```yaml
canonical_column_naming:
  case: snake_case
  allowed_suffixes:
    - _id
    - _code
    - _name
    - _date
    - _datetime
    - _at
    - _amount
    - _ratio
    - _price
    - _count
    - _volume
    - _flag
  boolean_prefixes:
    - is_
    - has_
  disallow_raw_vendor_uppercase_in_stg: true
```

第一版只校验 stg 输出字段，不强制 raw 字段满足 canonical naming。

## contract_tools CLI

推荐 CLI：

```bash
cd pipeline

uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run fleur-contracts generate --write
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
uv run fleur-contracts diff
```

CLI 职责：

| 命令 | 职责 |
|------|------|
| `validate` | 校验 YAML schema、字段引用、命名规则、glossary 引用 |
| `generate --check` | 生成 dbt YAML/data_dict 并检查工作区是否同步 |
| `generate --write` | 写回 dbt YAML/data_dict |
| `validate-parquet` | 读取 S3/RustFS Parquet schema 并与 contract 对比 |
| `validate-clickhouse` | 查询 ClickHouse metadata 并与 contract 对比 |
| `diff` | 输出 contract 与当前生成物/真实 schema 的差异摘要 |

## 与 Dagster 的集成

`scheduler` 不直接解析散落的 YAML 细节，只通过 adapter 获取 raw sync specs：

```text
fleur_contracts.adapters.clickhouse
  -> load contracts
  -> build ClickHouse raw table descriptors
  -> scheduler.defs.clickhouse.specs maps descriptors to ClickHouseRawTableSpec
```

约束：

- definitions 加载阶段只能读取本地 contract 文件。
- adapter 不连接 S3、ClickHouse 或远端 API。
- raw sync materialization metadata 应记录 `contract_dataset`、`contract_version`、`contract_schema_hash`。

## 与 dbt 的集成

dbt YAML 由 contract 生成或严格校验：

```text
contracts + glossary
  -> pipeline/elt/models/sources.yml
  -> pipeline/elt/models/staging/staging.yml
```

stg SQL 仍手写，负责字段选择、重命名、轻清洗和类型收敛。

dbt staging 字段必须：

- 使用 canonical `snake_case` 命名。
- 优先引用 `glossary_key`。
- 禁止直接保留供应商大写缩写，除非明确标记例外。

例外字段必须显式声明原因：

```yaml
    - name: raw_payload_hash
      from: raw_payload_hash
      canonical_exempt: true
      exempt_reason: 技术校验字段，不进入业务 glossary。
```

## 与 data_dict 的集成

`docs/references/data_dict/*.md` 变为 generated human-readable reference。

每个生成文档顶部应包含：

```text
本文件由 pipeline/contracts/datasets/<dataset>.yml 生成。字段事实以 contract 为准。
```

data_dict 中展示：

- 外源字段名。
- Parquet 类型。
- ClickHouse raw 字段名和类型。
- stg 字段名。
- glossary 中文名称和中文描述。
- dataset 特有备注。

## 测试和 CI

必须覆盖：

- contract YAML schema load。
- dataset 文件名与 `dataset` 字段一致。
- raw/stg 字段 `from` 引用合法。
- stg 字段符合 canonical naming rules。
- `glossary_key` 存在且 canonical name 与 stg field name 一致。
- canonical 例外字段必须同时提供 `canonical_exempt: true` 和 `exempt_reason`。
- generated dbt YAML 和 data_dict 无 diff。
- ClickHouse raw spec adapter 输出稳定。
- 可访问环境下的 Parquet/ClickHouse schema 校验。

## 取舍

### 为什么不放进 scheduler

contract registry 同时服务 Dagster、dbt、ClickHouse schema 校验和文档生成。放进 scheduler 会把非 Dagster 的治理工具耦合到 Dagster definitions。

### 为什么不放进 elt

contract 不只是 dbt source/staging 文档，还包含 S3 Parquet、ClickHouse raw、Dagster asset key 和 raw sync metadata。

### 为什么不放在根目录散装 scripts

该工具会包含 schema、adapter、生成器、CLI 和测试。散装脚本不利于类型检查、边界测试和长期维护。

### 为什么不拆独立仓库

contract 必须和 raw sync、dbt models、data_dict 在同一个提交中保持一致。拆仓库会引入版本同步和发布顺序问题。

## 后续扩展

- mart 层稳定后，可以新增 mart contract 或 semantic layer contract。
- 如果字段描述变多，可将 glossary 拆成业务域文件。
- 如果 contract 生成 SQL 的收益大于复杂度，再评估 stg SQL 模板化。
- 如果未来接入外部数据目录平台，contract registry 应作为上游事实源导出 metadata，而不是被平台反向管理。
