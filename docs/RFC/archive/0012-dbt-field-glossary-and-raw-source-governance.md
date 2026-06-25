# RFC 0012: dbt 字段字典与 ClickHouse raw source 治理

状态：草案（2026-06-02）

## 摘要

本文档定义 mono-fleur 在 `pipeline/contracts` 收敛到 ClickHouse raw 层之后，dbt `stg/int/mart` 层字段命名、字段值格式标准化、字段文档和 raw source 列级治理的新方案。

核心决策：

1. **一刀切删除 contract 字段 glossary 旧方案**：删除 `pipeline/contracts/glossary/fields.yml` 及其 `contract_tools` 校验、schema、loader 和测试依赖，不保留兼容层。
2. **dbt 项目拥有 stg 以后字段字典**：公共字段名、语义、描述、值格式标准、派生字段例外和 stg 映射由 `pipeline/elt` 内的 metadata、model YAML、docs blocks 和 manifest 校验维护。
3. **contract 继续生成 dbt raw sources**：`pipeline/elt/models/sources.yml` 仍由 `contract_tools` 生成，但升级为 ClickHouse raw source catalog，包含 raw 表和 raw 字段描述、类型和 contract metadata。
4. **raw source catalog 不参与 canonical 命名**：raw 字段保持外源/Parquet/ClickHouse raw 命名，canonical 字段名只从 dbt staging 开始。
5. **字段命名和格式治理用 dbt 官方机制落地**：dbt YAML 负责 column `description`、`data_type`、`data_tests`、`config.meta`，集中 docs blocks 复用描述，dbt macro 执行可复用标准化转换，独立脚本读取 `target/manifest.json` 做结构校验。

## 背景

当前数据链路已经稳定为：

```text
source payload
  -> Dagster source asset
  -> S3 Parquet
  -> ClickHouse raw table
  -> dbt source()
  -> dbt staging
  -> dbt intermediate / mart
```

前序治理已完成 contract scope 收敛：

- `pipeline/contracts/datasets/*.yml` 负责 `source.fields -> parquet.fields -> clickhouse_raw.fields`。
- `pipeline/contract_tools` 负责 raw contract 校验、Parquet/ClickHouse raw schema adapter、`docs/references/data_dict/*.md` 和 dbt `models/sources.yml` 生成。
- dbt `staging.yml`、`stg_*.sql`、stg 字段描述和 tests 由 `pipeline/elt` 维护。

遗留问题是 `pipeline/contracts/glossary/fields.yml` 仍存在：

- `loader.py` 加载 `glossary/fields.yml`。
- `schema.py` 仍定义 `GlossaryField`、`ContractRegistry.glossary_fields` 和 `ClickHouseRawField.glossary_key`。
- `description_quality.py` 仍校验 glossary 字段描述。
- tests 仍构造或断言 `glossary_fields`。
- `docs/skills/fleur-contract-data-dictionary/SKILL.md` 仍把字段 glossary 作为 contract 工作流入口。

这些残留已经没有实际 dbt 应用。继续保留会让后续 stg 建模误以为 contract 仍管理 canonical 字段名，和当前边界冲突。

## 目标

1. 删除 `contracts/glossary/fields.yml` 相关旧方案，避免 stg 字段治理回流到 raw contract。
2. 在 dbt 项目内建立公共字段字典，用于统一 stg/int/mart 中常见字段命名、字段描述和值格式标准。
3. 让 stg model YAML 显式记录 canonical 字段与 raw source 字段的映射。
4. 使用 dbt 官方 YAML/docs/meta 机制承载字段文档和字段元数据。
5. 使用 manifest lint 作为机械约束，减少仅靠 review 维持命名一致性的风险。
6. 升级 generated `sources.yml`，补齐 ClickHouse raw 表的列级描述、类型和来源 metadata。
7. 明确 raw source catalog 与 dbt canonical field dictionary 的职责边界。
8. 明确 stg 层的 source-local 标准化职责，例如证券代码格式、日期格式、枚举值和单位归一。

## 非目标

1. 不把 dbt staging SQL 模板化。
2. 不把 `pipeline/elt` 的字段字典重新做成 `pipeline/contracts` 的一部分。
3. 不让 dbt 直接读取 S3 Parquet。
4. 不引入 DataHub、OpenMetadata、Amundsen 等外部数据目录平台。
5. 不在 raw 层改名、清洗、合并或派生业务字段。
6. 不要求本 RFC 同时实现所有 stg/int/mart 模型。
7. 不把 mart 指标口径中心和字段命名字典混为一谈；指标口径后续应由 mart/semantic layer 单独治理。
8. 不把复杂跨源实体匹配放入 staging；证券主数据合并、退市状态修正、代码历史映射应进入 intermediate 或 mart 维表。

## 关联文档

- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/RFC/archive/0009-dagster-clickhouse-raw-sync.md`
- `docs/RFC/archive/0010-data-contract-registry-and-contract-tools.md`
- `docs/RFC/archive/0011-contract-driven-parquet-schema-adapter.md`
- `docs/plans/archive/0021-contract-scope-raw-layer-cleanup-plan.md`
- `docs/skills/fleur-contract-data-dictionary/SKILL.md`
- dbt column properties: https://docs.getdbt.com/reference/resource-properties/columns
- dbt descriptions and docs blocks: https://docs.getdbt.com/reference/resource-properties/description
- dbt `doc()` function: https://docs.getdbt.com/reference/dbt-jinja-functions/doc

## 设计原则

### raw 字段事实和 dbt 字段语义分离

raw source catalog 只回答：

- ClickHouse raw 有哪些表？
- raw 表有哪些字段？
- raw 字段类型是什么？
- raw 字段来自哪个 Parquet 字段？
- raw 字段在外源语境中是什么意思？
- raw 表关联哪个 contract dataset/version/schema hash？

dbt 字段字典只回答：

- dbt 输出字段应该叫什么 canonical name？
- 这个 canonical field 在 mono-fleur 内表达什么语义？
- 这个 canonical field 的标准值格式和值域是什么？
- 哪个 dbt macro 或测试用于把 source-local 格式转换为标准格式？
- 哪些 stg model 的字段复用这个 canonical field？
- 这个 stg 字段来自哪些 raw source column？
- 哪些字段是局部字段、派生字段或允许的命名例外？

两者通过 stg model YAML 的 `meta.source_columns` 和 `meta.glossary_key` 连接，不通过 contract registry 连接。

### 外源命名保留到 raw

raw 表字段可以是外源命名、供应商命名或已由 Parquet adapter 固定的 raw 命名。raw catalog 不要求 lower snake case，也不要求 canonical 统一。

示例：

| 层级 | 字段名 | 职责 |
|------|--------|------|
| source payload | `SECUCODE` | 供应商字段 |
| S3 Parquet | `SECUCODE` | source asset 写出的字段 |
| ClickHouse raw | `SECUCODE` | dbt source 可读的 raw 字段 |
| dbt staging | `security_code` | mono-fleur canonical 字段 |

### dbt staging 是第一次 canonical 命名边界

stg SQL 承担外源字段到 canonical 字段的显式映射，也承担 source-local、确定性、低业务口径风险的格式标准化。

stg 层允许：

- 证券代码格式标准化，例如 `sh.601088` -> `601088.SH`。
- 日期、时间戳和布尔字段的类型收敛。
- 明确枚举值的轻量标准化，例如供应商交易状态编码 -> mono-fleur 交易状态枚举。
- 单位已明确且不涉及业务口径选择的数值归一。

stg 层不应承担：

- 跨源实体合并和主数据修正。
- 财务指标口径重算。
- 依赖多表上下文的业务推断。
- 为了下游查询便利而提前做宽表聚合。

示例：

```sql
select
    {{ normalize_cn_security_code("SECUCODE", input_format="eastmoney_suffix") }} as security_code,
    TRADE_DATE as trade_date,
    CLOSE_PRICE as close_price
from {{ source('raw', 'eastmoney__equity_history') }}
```

stg YAML 承担字段语义、来源和校验声明：

```yaml
version: 2

models:
  - name: stg_eastmoney__equity_history
    description: EastMoney equity history normalized to mono-fleur canonical fields.
    columns:
      - name: security_code
        description: "{{ doc('field_security_code') }}"
        data_type: String
        data_tests:
          - cn_security_code_format
        config:
          meta:
            glossary_key: security_code
            normalization:
              macro: normalize_cn_security_code
              input_format: eastmoney_suffix
            source_columns:
              - source: raw
                table: eastmoney__equity_history
                column: SECUCODE

      - name: trade_date
        description: "{{ doc('field_trade_date') }}"
        data_type: Date
        config:
          meta:
            glossary_key: trade_date
            source_columns:
              - source: raw
                table: eastmoney__equity_history
                column: TRADE_DATE
```

### 证券代码标准化决策

A 股证券代码第一版采用后缀大写格式作为 canonical 值：

```text
security_code = "<6位证券代码>.<交易所代码>"
```

示例：

| 来源 | raw 值 | canonical `security_code` |
|------|--------|---------------------------|
| EastMoney | `601088.SH` | `601088.SH` |
| BaoStock | `sh.601088` | `601088.SH` |
| BaoStock | `sz.000001` | `000001.SZ` |

不采用纯数字作为 `security_code`。纯数字会丢失交易所信息，不能作为可靠 join key；例如 `000001.SH` 和 `000001.SZ` 表达不同证券或指数语境。纯数字如需保留，应命名为 companion field：

```text
security_local_code = "601088"
exchange_code = "SH"
```

`security_code`、`security_local_code` 和 `exchange_code` 的推荐关系：

| 字段 | 值示例 | 用途 |
|------|--------|------|
| `security_code` | `601088.SH` | stg/int/mart 主 join key |
| `security_local_code` | `601088` | 展示、分组或需要本地代码的接口参数 |
| `exchange_code` | `SH` | 交易所维度、过滤和代码解析 |

允许的 A 股交易所代码第一版为 `SH`、`SZ`、`BJ`。如果后续接入港股、美股、基金、债券或指数，应扩展字段字典的 `value_domain`，不要让 pure numeric code 回到 `security_code`。

### 转换实现位置

证券代码、交易状态、日期等标准化转换不使用外部 dbt 插件作为第一选择。第一版应在本 dbt 项目内实现 reusable macros 和 generic tests：

```text
pipeline/elt/
  macros/
    standardize/
      security_code.sql
  tests/
    generic/
      cn_security_code_format.sql
```

原因：

- 这些转换是 mono-fleur 的领域语义，不是通用 dbt 能力。
- 转换需要和 ClickHouse SQL、raw source catalog、字段字典和 stg YAML meta 一起演进。
- 项目内 macro 更容易 review、测试和调整，不引入 package version 管理成本。

只有当相同标准化逻辑需要被多个 dbt 项目复用时，才考虑抽成 dbt package。即使抽成 package，也应先在 `pipeline/elt/macros` 中稳定规则和测试样例。

建议 macro 形态：

```jinja
{{ normalize_cn_security_code("code", input_format="baostock_prefix") }}
{{ normalize_cn_security_code("SECUCODE", input_format="eastmoney_suffix") }}
{{ cn_security_local_code("code", input_format="baostock_prefix") }}
{{ cn_exchange_code("code", input_format="baostock_prefix") }}
```

stg SQL 使用 macro，stg YAML 记录 source format：

```yaml
columns:
  - name: security_code
    description: "{{ doc('field_security_code') }}"
    data_type: String
    data_tests:
      - cn_security_code_format
    config:
      meta:
        glossary_key: security_code
        normalization:
          macro: normalize_cn_security_code
          input_format: baostock_prefix
        source_columns:
          - source: raw
            table: baostock__query_history_k_data_plus_daily
            column: code
```

## 一刀切删除旧 contract 字段 glossary

### 删除范围

一次性删除以下旧方案，不做 deprecated alias、不做兼容读取：

| 路径或符号 | 处理 |
|------------|------|
| `pipeline/contracts/glossary/fields.yml` | 删除 |
| `GlossaryField` | 删除 |
| `ContractRegistry.glossary_fields` | 删除 |
| `ClickHouseRawField.glossary_key` | 删除 |
| `loader.py` 对 `glossary/fields.yml` 的加载 | 删除 |
| `description_quality.py` 对 `registry.glossary_fields` 的校验 | 删除 |
| `test_glossary_descriptions_are_quality_checked` 中字段 glossary 断言 | 删除或改为只覆盖 source external descriptions |
| test fixture 中的 `glossary_fields={...}` | 删除 |
| `docs/skills/fleur-contract-data-dictionary/SKILL.md` 字段 glossary 工作流 | 更新为 raw-only |

`pipeline/contracts/glossary/tables.yml` 可以保留，因为它服务于 raw table 描述和 generated `sources.yml` 表级 description，不承担 stg 字段命名职责。

### 命名规则处理

`pipeline/contracts/naming_rules.yml` 当前只包含：

```yaml
canonical_field_pattern: "^[a-z][a-z0-9_]*$"
```

该规则属于 dbt canonical field 命名，不应继续由 raw contract 持有。实施时有两个可选处理：

1. 删除 `pipeline/contracts/naming_rules.yml` 和 `NamingRules`。
2. 若 contract 仍需要 dataset/table 命名规则，则改名为 raw contract 专用配置，例如 `dataset_name_pattern`，不要包含 canonical field pattern。

推荐选择 1，后续在 `pipeline/elt/metadata/field_glossary.yml` 或 dbt lint 配置中定义 dbt 字段命名规则。

### 禁止回流

后续禁止：

- 在 `pipeline/contracts/datasets/*.yml` 重新加入 stg 字段映射。
- 在 `clickhouse_raw.fields` 中加入 canonical 字段名、stg 字段名、`glossary_key` 或 dbt tests。
- 在 `contract_tools` 中校验 stg output column name。
- 从 stg SQL 反向解析字段并写回 contract。
- 用 generated raw data_dict 代替 dbt model YAML 文档。

## dbt 字段字典方案

### 目标目录

```text
pipeline/elt/
  metadata/
    field_glossary.yml
  macros/
    standardize/
      security_code.sql
  models/
    _docs/
      fields.md
    staging/
      <source>/
        stg_<source>__<entity>.sql
        stg_<source>__<entity>.yml
    intermediate/
    marts/
  scripts/
    validate_field_glossary.py
  tests/
    generic/
      cn_security_code_format.sql
```

### `field_glossary.yml`

字段字典只维护 dbt canonical 字段，不维护 raw 字段。

建议结构：

```yaml
version: 1

rules:
  canonical_field_pattern: "^[a-z][a-z0-9_]*$"
  required_column_meta:
    - glossary_key
    - source_columns
  required_normalization_meta:
    - macro
    - input_format

fields:
  security_code:
    name: security_code
    description_zh: 带交易所后缀的证券标准代码。
    description: Market-qualified security code with exchange suffix.
    semantic_type: security_identifier
    domains: [security]
    preferred_data_type: String
    value_format: "<6位证券代码>.<交易所代码>"
    value_domain:
      exchange_codes: [SH, SZ, BJ]
    regex: "^[0-9]{6}\\.(SH|SZ|BJ)$"
    examples:
      - "601088.SH"
      - "000001.SZ"
    normalization_macro: normalize_cn_security_code
    companion_fields:
      - security_local_code
      - exchange_code
    allowed_suffixes: []
    deprecated_names:
      - stock_code
      - code
    deprecated_formats:
      - "sh.601088"
      - "601088"

  security_local_code:
    name: security_local_code
    description_zh: 不带交易所前后缀的 6 位证券本地代码。
    description: Six-digit exchange-local security code without exchange prefix or suffix.
    semantic_type: security_identifier_component
    domains: [security]
    preferred_data_type: String
    value_format: "<6位证券代码>"
    regex: "^[0-9]{6}$"

  exchange_code:
    name: exchange_code
    description_zh: 证券所属交易所代码，例如 SH、SZ 或 BJ。
    description: Security exchange code, such as SH, SZ, or BJ.
    semantic_type: exchange_identifier
    domains: [security]
    preferred_data_type: String
    value_domain:
      values: [SH, SZ, BJ]

  trade_date:
    name: trade_date
    description_zh: A 股市场交易日日期。
    description: A-share trading date.
    semantic_type: date
    domains: [market]
    preferred_data_type: Date
    allowed_suffixes: []
    deprecated_names:
      - date

  report_date:
    name: report_date
    description_zh: 财务报表或公告对应的报告期日期。
    description: Reporting-period date for financial statements or disclosures.
    semantic_type: date
    domains: [financial_statement]
    preferred_data_type: Date
```

字段字典可以记录 `deprecated_names` 和 `deprecated_formats`，但不应自动接受这些旧名或旧格式。出现旧列名时 manifest lint 应报错，并提示目标 canonical name；出现旧值格式时 dbt data test 应失败。

### dbt docs blocks

公共字段描述集中在 `pipeline/elt/models/_docs/fields.md`：

```jinja
{% docs field_security_code %}

带交易所后缀的证券标准代码，例如 601088.SH。

{% enddocs %}

{% docs field_trade_date %}

A 股市场交易日日期。

{% enddocs %}
```

stg/int/mart YAML 使用 dbt 官方 `doc()` 函数复用：

```yaml
columns:
  - name: trade_date
    description: "{{ doc('field_trade_date') }}"
```

这样 dbt docs 展示、manifest metadata 和代码 review 都能看到统一字段语义。

### stg model YAML

stg column 必须声明：

- `name`
- `description`
- `data_type`
- `config.meta.glossary_key`
- `config.meta.source_columns`
- 对需要值格式标准化的字段，声明 `config.meta.normalization`

示例：

```yaml
version: 2

models:
  - name: stg_baostock__daily_prices
    description: BaoStock daily price rows normalized to mono-fleur canonical fields.
    columns:
      - name: security_code
        description: "{{ doc('field_security_code') }}"
        data_type: String
        data_tests:
          - not_null
          - cn_security_code_format
        config:
          meta:
            glossary_key: security_code
            normalization:
              macro: normalize_cn_security_code
              input_format: baostock_prefix
            source_columns:
              - source: raw
                table: baostock__query_history_k_data_plus_daily
                column: code

      - name: close_price
        description: "{{ doc('field_close_price') }}"
        data_type: Float64
        config:
          meta:
            glossary_key: close_price
            source_columns:
              - source: raw
                table: baostock__query_history_k_data_plus_daily
                column: close
```

### 局部字段和派生字段

不是所有 stg 输出都一定适合进入公共字段字典。允许两类例外，但必须显式声明。

局部字段：

```yaml
config:
  meta:
    dictionary_scope: local
    source_columns:
      - source: raw
        table: jiuyan__industry_ocr_snapshot
        column: image_filename
```

派生字段：

```yaml
config:
  meta:
    glossary_key: trading_status
    derived_from:
      - source: raw
        table: baostock__query_history_k_data_plus_daily
        column: tradestatus
    derivation_note_zh: 将供应商交易状态编码标准化为 mono-fleur 交易状态枚举。
```

manifest lint 应要求例外字段有 `dictionary_scope: local` 或 `derived_from`，避免字段缺少治理却悄悄通过。

### int/mart 使用规则

int/mart 可以继续复用 `glossary_key`，但约束强度不同：

| 层级 | 规则 |
|------|------|
| staging | 强制每个输出 column 有 `glossary_key` 或显式 local/derived 例外 |
| intermediate | 强制公共字段沿用相同 `glossary_key`；允许技术字段和聚合中间字段 local |
| mart | 关键维度、主键、时间字段和指标字段必须有 `glossary_key` 或 mart metric metadata |

第一版优先强约束 staging，避免在 int/mart 治理还没成熟时阻塞建模。

## manifest lint

### 为什么不用 dbt data test

dbt data tests 适合校验数据值，例如 not null、unique、accepted values、relationships 和证券代码格式。字段命名、YAML meta、docs block 引用、normalization 声明和 source column lineage 属于项目元数据约束，应该读取 dbt manifest 校验。

因此第一版采用双层校验：

| 校验类型 | 输入 | 负责内容 |
|----------|------|----------|
| manifest lint | `target/manifest.json`、`field_glossary.yml`、raw source catalog | 字段是否声明、是否引用 glossary、是否记录 source columns 和 normalization meta |
| dbt generic data tests | stg model query result | 标准化后的值是否满足 regex、accepted values、not null、unique 等数据约束 |

### 命令

建议新增：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
```

### 校验输入

- `pipeline/elt/metadata/field_glossary.yml`
- `pipeline/elt/target/manifest.json`
- 可选：`pipeline/elt/models/sources.yml`

### 校验规则

第一版规则：

1. `models/staging/**` 下的 model column 必须在 YAML 中声明。
2. stg column 必须有 `description`。
3. stg column 必须有 `data_type`。
4. stg column 必须有 `config.meta.glossary_key`，或显式声明 `config.meta.dictionary_scope: local`。
5. `glossary_key` 必须存在于 `field_glossary.yml`。
6. 默认 `column.name == glossary_key`。
7. 如果 `column.name != glossary_key`，必须命中字段字典中明确允许的 suffix/prefix 规则。
8. `source_columns` 中的 raw table 和 raw column 必须存在于 generated raw source catalog。
9. `deprecated_names` 出现在 stg 输出列名时直接失败。
10. 字段描述若应复用公共 docs block，必须引用 `{{ doc('field_<glossary_key>') }}`。
11. 如果 glossary field 定义了 `normalization_macro`，stg column 必须声明 `config.meta.normalization.macro` 和 `input_format`。
12. `config.meta.normalization.macro` 必须等于 glossary field 的 `normalization_macro`，除非该 column 显式声明 `normalization_exempt_reason_zh`。
13. 如果 glossary field 定义了 `regex` 或 `value_domain`，stg column 必须配置对应 generic data test，或记录 `data_test_exempt_reason_zh`。

后续规则：

1. 对关键字段强制 `not_null` 或记录不强制原因。
2. 对枚举字段强制 accepted values 或记录字典来源。
3. 对主键字段强制 unique/not_null 或记录 grain 例外。
4. 对 mart 指标字段要求 metric metadata。
5. 对 `security_code` 等高价值字段，要求 companion fields 的组合一致性，例如 `security_code == concat(security_local_code, '.', exchange_code)`。

## ClickHouse raw source catalog 治理

### 当前问题

`pipeline/elt/models/sources.yml` 当前由 `contract_tools` 生成，但只包含 raw 表级信息：

```yaml
version: 2
sources:
  - name: raw
    schema: raw
    tables:
      - name: eastmoney__balance
        description: EastMoney balance sheet F10 rows by natural-year raw partition.
        meta:
          contract_dataset: eastmoney__balance
          contract_version: 1
          upstream_raw_asset: clickhouse/raw/eastmoney__balance
```

缺口：

- dbt docs 中看不到 raw table columns。
- stg author 不容易从 dbt 项目内查看 raw 字段类型和外源字段描述。
- stg YAML 的 `source_columns` 无法被 manifest lint 对照 generated source columns 校验。
- raw table schema 变化时，dbt parse 层缺少字段级 metadata 漂移信号。

### 生成方案

继续由 `contract_tools` 从 `pipeline/contracts/datasets/*.yml` 生成 `pipeline/elt/models/sources.yml`，但增加 columns：

```yaml
version: 2

sources:
  - name: raw
    schema: raw
    description: ClickHouse raw tables synchronized from Dagster-published S3 Parquet assets.
    tables:
      - name: baostock__query_history_k_data_plus_daily
        description: BaoStock daily K-line rows by natural-year raw partition.
        meta:
          contract_dataset: baostock__query_history_k_data_plus_daily
          contract_version: 1
          upstream_raw_asset: clickhouse/raw/baostock__query_history_k_data_plus_daily
          clickhouse_raw_table: raw.baostock__query_history_k_data_plus_daily
          source_schema_hash: "<hash>"
          parquet_schema_hash: "<hash>"
          clickhouse_schema_hash: "<hash>"
        columns:
          - name: code
            description: 证券代码，来源于 BaoStock 原始响应字段 code。
            data_type: String
            config:
              meta:
                source_field: code
                parquet_field: code
                clickhouse_raw_field: code
                external_description_zh: 证券在 BaoStock 来源系统中的代码。

          - name: close
            description: 交易日收盘价，来源于 BaoStock 原始响应字段 close。
            data_type: Float64
            config:
              meta:
                source_field: close
                parquet_field: close
                clickhouse_raw_field: close
                external_description_zh: 交易标的在交易日收盘时的价格。
```

字段描述来源规则：

1. 优先使用 `source.fields[].external_description_zh`。
2. 如果 `clickhouse_raw.fields[].from` 与 `source.fields` 不同名，通过 `parquet.fields` 链路追溯。
3. 如果字段为 raw sync 追加的技术字段，应由 contract 明确标注，不允许生成器猜测。
4. 描述表达 raw/source 语境，禁止写成 dbt canonical 语义。

### raw columns 和 data_dict 的关系

`docs/references/data_dict/*.md` 仍是人类阅读的 raw contract 文档，不是 dbt 字段事实源。

`pipeline/elt/models/sources.yml` 是 dbt raw source catalog，让 dbt docs 和 manifest 能看到 raw columns。

两者都从 `pipeline/contracts/datasets/*.yml` 生成，但服务对象不同：

| 生成物 | 主要读者 | 用途 |
|--------|----------|------|
| `docs/references/data_dict/*.md` | 人和 agent | 查看 source/parquet/clickhouse raw 链路 |
| `pipeline/elt/models/sources.yml` | dbt parser、dbt docs、stg author、manifest lint | dbt 内 raw source 元数据和 stg source column 校验 |

### raw source column tests

默认不在 generated raw source columns 上生成大量 dbt tests。

原因：

- raw 层保留供应商事实，很多字段 nullable、缺失或异常是来源事实，不应在 dbt source catalog 中过度约束。
- raw schema correctness 已由 contract、Parquet validator 和 ClickHouse validator 负责。
- stg/int/mart 才是业务质量测试的主要位置。

允许少量例外：

- raw 表天然主键或分区字段可以生成 `not_null`，但必须由 contract 显式声明。
- source freshness 如后续启用，应按表级配置，不从字段 glossary 推导。

## 实施计划

### Phase 1: 删除 contract field glossary

改动：

- 删除 `pipeline/contracts/glossary/fields.yml`。
- 删除 `GlossaryField`、`ContractRegistry.glossary_fields`、`ClickHouseRawField.glossary_key`。
- 删除 loader、description quality、tests 中字段 glossary 依赖。
- 更新 `docs/skills/fleur-contract-data-dictionary/SKILL.md`，把 contract skill 改为 raw-only。

验收：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
```

### Phase 2: 升级 generated raw sources

改动：

- 修改 `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`，为 raw source tables 生成 `columns`。
- raw source column `description` 取自 source external description 和 raw 字段链路。
- raw source column `data_type` 取 ClickHouse raw type。
- raw source column `config.meta` 写入 source/parquet/clickhouse raw 字段链路。
- 重新生成 `pipeline/elt/models/sources.yml`。

验收：

```bash
cd pipeline
uv run fleur-contracts generate --check
uv run dbt parse --project-dir elt --profiles-dir elt
```

### Phase 3: 建立 dbt 字段字典

改动：

- 新增 `pipeline/elt/metadata/field_glossary.yml`。
- 新增 `pipeline/elt/models/_docs/fields.md`。
- 为首批 staging model 制定 YAML 模板。
- 从旧 `contracts/glossary/fields.yml` 一次性迁移可确认的 canonical 字段描述；迁移后旧文件删除，不保留同步关系。
- 为 `security_code`、`security_local_code` 和 `exchange_code` 写入第一版值格式标准。

验收：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

### Phase 4: dbt 标准化 macro 和 generic tests

改动：

- 新增 `pipeline/elt/macros/standardize/security_code.sql`。
- 实现 `normalize_cn_security_code()`、`cn_security_local_code()` 和 `cn_exchange_code()`。
- 新增 `pipeline/elt/tests/generic/cn_security_code_format.sql`。
- 为 macro 增加文档 YAML，记录参数、支持的 `input_format` 和输出格式。
- 第一版支持 `eastmoney_suffix` 和 `baostock_prefix`。

验收：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

### Phase 5: manifest lint

改动：

- 新增 `pipeline/elt/scripts/validate_field_glossary.py`。
- 读取 `target/manifest.json`、`metadata/field_glossary.yml` 和 generated raw source catalog。
- 对 `models/staging/**` 应用第一版规则。
- 校验 `normalization.macro`、`normalization.input_format` 和必要 generic data tests。

验收：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
```

### Phase 6: 首批 stg 模型治理

改动：

- 首批 stg model 按 source 分目录维护。
- 每个 stg SQL 显式 alias raw 字段到 canonical 字段。
- 对证券代码等字段调用 dbt 标准化 macro，不在 SQL 中散落手写字符串表达式。
- 每个 stg YAML 写入 column docs、data_type、data_tests 和 `config.meta`。
- 对 local/derived/normalization-exempt 字段显式记录例外原因。

验收：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select staging
uv run python elt/scripts/validate_field_glossary.py
```

## 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 删除 `contracts/glossary/fields.yml` 后短期缺少公共字段描述 | stg 建模初期需要重新整理字段语义 | 一次性迁移可信条目到 `pipeline/elt/metadata/field_glossary.yml` 和 dbt docs blocks |
| raw `sources.yml` 变大 | generated YAML 更长，review diff 增加 | 文件仍是生成物，review 重点放在 contract dataset diff |
| manifest lint 过严阻塞早期 stg 探索 | 新模型迭代变慢 | 第一版只强约束 staging，允许 `dictionary_scope: local` 和 derived 例外 |
| raw field description 被误写成 canonical 语义 | raw/source 和 stg 语义混淆 | source catalog 描述只使用 `external_description_zh`，canonical 描述只在 dbt field glossary |
| `source_columns` 维护成本高 | stg YAML 变长 | 这是字段 lineage 的显式成本；后续可加 codegen 辅助生成初始 YAML，但不反向生成 SQL |
| `security_code` 被实现成纯数字 | 跨交易所 join key 冲突，后续 mart 难以修复 | manifest lint 检查 glossary field；dbt test 检查 `^[0-9]{6}\\.(SH|SZ|BJ)$` |
| 标准化 macro 承担过多业务推断 | stg 层变成隐式主数据修正层 | stg macro 只做 source-local 确定性转换；跨源主数据逻辑进入 int/mart |
| 不同 stg SQL 手写不同转换表达式 | 相同字段值格式漂移 | 高价值字段必须调用 dbt macro；manifest lint 检查 `normalization.macro` |
| companion fields 与主 code 不一致 | 下游过滤和 join 产生细微错误 | 后续增加组合一致性 test：`security_code == concat(security_local_code, '.', exchange_code)` |

## 开放问题

1. `field_glossary.yml` 是否需要进入单独 Python package，还是先作为 `pipeline/elt` metadata 文件由脚本读取即可？
2. 是否需要为 `field_glossary.yml` 生成 Markdown 字段字典，放到 `docs/references/dbt_field_glossary.md`？
3. mart 指标字段是否在同一 glossary 中维护，还是后续引入独立 metric glossary？
4. raw source column `data_type` 应保留 ClickHouse 原始类型字符串，还是映射成 dbt adapter 更通用的类型名？
5. 是否需要对 generated `sources.yml` 分文件，例如 `models/sources/raw/*.yml`，以降低单文件 diff 压力？
6. `security_code` 后续是否需要支持非 A 股市场。如果支持，应扩展为 market-qualified 标准，还是新增 `market_code` companion field？
7. 是否需要为日期、金额单位、比例、交易状态等字段建立和 `security_code` 同级的 value standard？

## 最终状态

完成后，字段治理边界应为：

```text
pipeline/contracts/datasets/*.yml
  -> source/parquet/clickhouse raw field facts
  -> generated docs/references/data_dict/*.md
  -> generated pipeline/elt/models/sources.yml with raw columns

pipeline/elt/metadata/field_glossary.yml
  -> dbt canonical field names, descriptions, value formats and normalization metadata
  -> pipeline/elt/macros/standardize/*.sql
  -> pipeline/elt/tests/generic/*.sql
  -> pipeline/elt/models/_docs/fields.md
  -> stg/int/mart YAML column metadata
  -> manifest lint
```

`pipeline/contracts` 不再知道 stg 字段名。`pipeline/elt` 不再从 contract field glossary 继承 canonical 命名。两者只在 dbt `source('raw', ...)` 和 stg YAML `source_columns` 中显式相遇。
