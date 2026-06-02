# Plan 0024: dbt field glossary and raw source governance implementation

日期：2026-06-02

状态：Draft

关联文档：

- `docs/RFC/0012-dbt-field-glossary-and-raw-source-governance.md`
- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/RFC/0010-data-contract-registry-and-contract-tools.md`
- `docs/RFC/0011-contract-driven-parquet-schema-adapter.md`
- `docs/plans/archive/0021-contract-scope-raw-layer-cleanup-plan.md`
- `docs/plans/archive/0022-contract-driven-parquet-schema-adapter-implementation-plan.md`
- `docs/skills/fleur-contract-data-dictionary/SKILL.md`

## 1. 背景

RFC 0012 已经决定把字段治理边界彻底拆开：

```text
pipeline/contracts/datasets/*.yml
  -> source / parquet / clickhouse raw 字段事实
  -> generated data_dict
  -> generated dbt raw sources.yml

pipeline/elt
  -> dbt canonical 字段名
  -> stg/int/mart 字段描述、值格式、tests、meta 和 manifest lint
```

Plan 0021 已把 contract scope 收缩到 ClickHouse raw 层，但当前仓库仍保留旧字段 glossary 方案的残留：

- `pipeline/contracts/glossary/fields.yml` 仍存在。
- `pipeline/contracts/naming_rules.yml` 仍包含 `canonical_field_pattern`。
- `pipeline/contract_tools/src/fleur_contracts/schema.py` 仍定义 `GlossaryField`、`NamingRules`、`ContractRegistry.glossary_fields` 和 `ClickHouseRawField.glossary_key`。
- `pipeline/contract_tools/src/fleur_contracts/loader.py` 仍加载字段 glossary 和 naming rules。
- `pipeline/contract_tools/src/fleur_contracts/description_quality.py` 仍校验 glossary 字段描述。
- `pipeline/contract_tools/tests/test_contract_registry.py` 仍构造或断言字段 glossary。
- `docs/skills/fleur-contract-data-dictionary/SKILL.md` 仍把 `glossary/fields.yml` 作为 contract 工作流入口。
- `pipeline/elt/models/sources.yml` 当前只生成 raw 表级 metadata，尚未生成 raw source columns。
- `pipeline/elt` 还没有 `metadata/field_glossary.yml`、字段 docs blocks、标准化 macros、generic tests 或 manifest lint 脚本。

本计划把 RFC 0012 收敛为实施步骤和验收清单，目标是一次性删除旧 contract field glossary，同时在 dbt 项目内建立可机械验证的 canonical 字段治理。

## 2. 目标

完成后应满足：

1. `pipeline/contracts` 不再持有 dbt canonical 字段名、字段 glossary、canonical 命名规则或 stg 字段映射。
2. `pipeline/contracts/glossary/tables.yml` 继续保留，用于 raw table 描述和 generated `sources.yml` 表级 description。
3. `pipeline/elt/models/sources.yml` 由 `contract_tools` 生成 ClickHouse raw source catalog，包含 raw table columns、raw 字段描述、ClickHouse 类型和字段链路 metadata。
4. `pipeline/elt/metadata/field_glossary.yml` 成为 dbt canonical 字段字典事实源。
5. dbt docs blocks 复用公共字段描述，stg YAML 显式记录 `glossary_key`、`source_columns`、`data_type`、`data_tests` 和 normalization metadata。
6. `security_code` 第一版 canonical 值格式固定为 `<6位证券代码>.<交易所代码>`，允许交易所代码为 `SH`、`SZ`、`BJ`。
7. dbt 标准化 macro 和 generic tests 管理证券代码等高价值字段的格式，不在 stg SQL 中散落手写转换表达式。
8. `pipeline/elt/scripts/validate_field_glossary.py` 读取 dbt manifest、dbt field glossary 和 raw source catalog，强制 staging 字段治理规则。
9. 首批 staging model 完成字段治理，后续 int/mart 治理可以在不回流 contract 的前提下逐步加强。

## 3. 非目标

本计划不做以下事情：

1. 不把 dbt staging SQL 模板化或从 metadata 自动生成 SQL。
2. 不让 `pipeline/contracts` 重新管理 stg/int/mart 字段。
3. 不改变 Dagster source assets、S3 Parquet schema、ClickHouse raw sync 或历史 raw 数据。
4. 不把 DataHub、OpenMetadata、Amundsen 等外部数据目录平台引入第一版。
5. 不把 mart 指标口径治理塞进字段命名字典；指标口径后续由 mart 或 semantic layer 单独治理。
6. 不把跨源实体匹配、证券主数据修正、退市状态修正或代码历史映射放入 staging。
7. 不在 generated raw source columns 上默认生成大量 dbt data tests；raw schema correctness 仍由 contract、Parquet validator 和 ClickHouse validator 负责。
8. 不保留旧 contract field glossary 的兼容读取、deprecated alias 或同步关系。

## 4. 目标边界

### 4.1 raw contract 边界

`pipeline/contracts/datasets/*.yml` 只回答：

- 外源字段名、外源类型和供应商语境中文描述。
- S3 Parquet 字段名、类型、nullable 和顺序。
- ClickHouse raw 表名、字段名、类型、分区、排序键和 raw asset key。
- raw 表关联的 contract dataset、version 和 schema hash。

禁止在 contract 中写入：

- canonical 字段名。
- stg 字段名。
- `glossary_key`。
- dbt tests。
- stg source column mapping。
- dbt normalization macro metadata。

### 4.2 dbt 字段治理边界

`pipeline/elt` 负责：

- canonical 字段名和公共字段描述。
- 字段标准值格式和值域。
- stg SQL 中从 raw 字段到 canonical 字段的显式 alias。
- source-local、确定性、低业务口径风险的格式标准化。
- YAML column `description`、`data_type`、`data_tests`、`config.meta`。
- manifest lint。

staging 允许：

- 证券代码格式标准化，例如 `sh.601088` -> `601088.SH`。
- 日期、时间戳、布尔字段的类型收敛。
- 明确枚举值的轻量标准化。
- 单位已明确且不涉及业务口径选择的数值归一。

staging 禁止：

- 跨源实体合并和主数据修正。
- 财务指标口径重算。
- 依赖多表上下文的业务推断。
- 为下游便利提前做宽表聚合。

## 5. 目标目录

第一版完成后，新增或更新目录结构如下：

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
  scripts/
    validate_field_glossary.py
  tests/
    generic/
      cn_security_code_format.sql
```

`pipeline/contracts/glossary/tables.yml` 保留。`pipeline/contracts/glossary/fields.yml` 和 `pipeline/contracts/naming_rules.yml` 删除。

## 6. 实施阶段

### Phase 0: 基线冻结和影响面确认

范围：

- `pipeline/contracts`
- `pipeline/contract_tools/src/fleur_contracts`
- `pipeline/contract_tools/tests`
- `pipeline/elt`
- `docs/skills/fleur-contract-data-dictionary/SKILL.md`

动作：

- 列出所有旧 field glossary 引用：

```bash
rg -n "glossary_fields|GlossaryField|glossary_key|naming_rules|canonical_field_pattern|fields.yml" \
  pipeline/contract_tools pipeline/contracts docs/skills/fleur-contract-data-dictionary/SKILL.md AGENTS.md
```

- 确认 `pipeline/elt/models/sources.yml` 当前只包含 raw table metadata，没有 raw source columns。
- 确认 `pipeline/elt` 是否已有 staging models；若没有，首批治理阶段先新增最小 staging baseline。
- 记录旧 `contracts/glossary/fields.yml` 中可迁移到 dbt field glossary 的可信字段，不迁移无法确认的语义。

完成标准：

- 后续 phase 的删除清单完整。
- 旧字段 glossary 中的可迁移条目有明确去向：迁移到 `pipeline/elt/metadata/field_glossary.yml`、放弃，或标记为待核实。

### Phase 1: 删除 contract field glossary 和 canonical naming 规则

范围：

- `pipeline/contracts/glossary/fields.yml`
- `pipeline/contracts/naming_rules.yml`
- `pipeline/contract_tools/src/fleur_contracts/schema.py`
- `pipeline/contract_tools/src/fleur_contracts/loader.py`
- `pipeline/contract_tools/src/fleur_contracts/description_quality.py`
- `pipeline/contract_tools/tests/test_contract_registry.py`
- 其他引用 `GlossaryField`、`glossary_fields`、`NamingRules`、`glossary_key` 的测试和实现文件

动作：

- 删除 `pipeline/contracts/glossary/fields.yml`。
- 删除 `pipeline/contracts/naming_rules.yml`，不保留 `canonical_field_pattern`。
- 删除 `GlossaryField`、`NamingRules` 和 `ContractRegistry.glossary_fields`。
- 删除 `ClickHouseRawField.glossary_key`。
- 删除 loader 对 `glossary/fields.yml` 和 `naming_rules.yml` 的加载。
- 删除 description quality 对 `registry.glossary_fields` 的校验。
- 更新 tests，不再构造 `glossary_fields={...}` 或断言 field glossary 描述质量。
- 保留 `source.fields[].external_description_zh` 的描述质量校验。

完成标准：

```bash
rg -n "GlossaryField|glossary_fields|NamingRules|canonical_field_pattern|glossary_key" \
  pipeline/contract_tools pipeline/contracts
```

无实现引用。若测试数据中必须保留字符串样例，必须说明其不是 contract 字段治理逻辑。

验证：

```bash
cd pipeline
uv run fleur-contracts validate
uv run pytest contract_tools/tests -q
```

### Phase 2: 更新 contract skill 和 raw-only 文档

范围：

- `docs/skills/fleur-contract-data-dictionary/SKILL.md`
- `pipeline/contracts/README.md`
- 必要时更新 `AGENTS.md` 中 contract/dbt 边界指针

动作：

- 把 contract skill 改成 raw-only：
  - 字段事实源为 `pipeline/contracts/datasets/*.yml`。
  - 字段链路为 `source.fields -> parquet.fields -> clickhouse_raw.fields`。
  - `docs/references/data_dict/*.md` 和 `pipeline/elt/models/sources.yml` 是生成物。
  - dbt `staging.yml`、`stg_*.sql`、stg 字段描述、tests 和 meta 由 `pipeline/elt` 维护。
- 删除 skill 中 `pipeline/contracts/glossary/fields.yml`、`naming_rules.yml`、`glossary_key`、canonical field 描述规则。
- 增加指针：dbt canonical 字段治理见 `pipeline/elt/metadata/field_glossary.yml` 和本计划。
- 保留 `tables.yml` 的 raw table 描述职责。

完成标准：

```bash
rg -n "glossary/fields.yml|naming_rules.yml|glossary_key|canonical_field_pattern" \
  AGENTS.md docs/skills/fleur-contract-data-dictionary/SKILL.md pipeline/contracts/README.md
```

无当前规则冲突。

验证：

```bash
git diff --check
```

### Phase 3: 升级 generated raw `sources.yml` 为 source catalog

范围：

- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- `pipeline/contract_tools/src/fleur_contracts/generate.py`
- `pipeline/contract_tools/tests`
- `pipeline/elt/models/sources.yml`

动作：

- 为每个有 `clickhouse_raw` 的 dataset 生成 dbt source table `columns`。
- raw source column `name` 使用 ClickHouse raw 字段名。
- raw source column `data_type` 使用 ClickHouse raw type 字符串。
- raw source column `description` 使用 source external description，并明确 raw/source 语境。
- 通过 `clickhouse_raw.fields[].from -> parquet.fields -> source.fields` 追溯字段链路。
- raw source column `config.meta` 至少写入：
  - `source_field`
  - `parquet_field`
  - `clickhouse_raw_field`
  - `external_description_zh`
- raw table `meta` 补齐：
  - `contract_dataset`
  - `contract_version`
  - `upstream_raw_asset`
  - `clickhouse_raw_table`
  - `source_schema_hash`
  - `parquet_schema_hash`
  - `clickhouse_schema_hash`
- 不默认生成 raw source column data tests。
- 若某字段是 raw sync 技术字段，必须由 contract 明确标注；生成器不猜测字段语义。

完成标准：

- `pipeline/elt/models/sources.yml` 包含 raw tables 和 raw columns。
- dbt docs/manifest 能看到 raw source column metadata。
- generated `sources.yml` 仍只由 contract 生成，不手工维护。

验证：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run dbt parse --project-dir elt --profiles-dir elt
```

### Phase 4: 建立 dbt canonical field glossary 和 docs blocks

范围：

- `pipeline/elt/metadata/field_glossary.yml`
- `pipeline/elt/models/_docs/fields.md`
- `pipeline/elt/README.md`

动作：

- 新增 `metadata/field_glossary.yml`，结构包含：
  - `version`
  - `rules.canonical_field_pattern`
  - `rules.required_column_meta`
  - `rules.required_normalization_meta`
  - `fields`
- 从旧 `contracts/glossary/fields.yml` 迁移可信 canonical 字段描述；无法确认的字段不伪造语义。
- 第一批至少定义：
  - `security_code`
  - `security_local_code`
  - `exchange_code`
  - `trade_date`
  - `report_date`
- `security_code` 固定：

```text
value_format = "<6位证券代码>.<交易所代码>"
regex = "^[0-9]{6}\\.(SH|SZ|BJ)$"
exchange_codes = [SH, SZ, BJ]
deprecated_formats = ["sh.601088", "601088"]
```

- 新增 `models/_docs/fields.md`，为每个公共字段提供 `field_<glossary_key>` docs block。
- dbt README 记录：canonical 字段事实源在 `pipeline/elt/metadata/field_glossary.yml`，不是 contract。

完成标准：

- 公共字段描述可以通过 `{{ doc('field_<glossary_key>') }}` 被 dbt YAML 复用。
- `security_code` 不接受纯数字作为 canonical join key。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
git diff --check
```

### Phase 5: 实现 dbt 标准化 macros 和 generic tests

范围：

- `pipeline/elt/macros/standardize/security_code.sql`
- `pipeline/elt/macros/standardize/schema.yml`
- `pipeline/elt/tests/generic/cn_security_code_format.sql`

动作：

- 实现：
  - `normalize_cn_security_code(column_name, input_format=...)`
  - `cn_security_local_code(column_name, input_format=...)`
  - `cn_exchange_code(column_name, input_format=...)`
- 第一版支持：
  - `eastmoney_suffix`：`601088.SH` -> `601088.SH`
  - `baostock_prefix`：`sh.601088` -> `601088.SH`
- macro 输出 ClickHouse SQL 表达式。
- generic test `cn_security_code_format` 校验 `^[0-9]{6}\\.(SH|SZ|BJ)$`。
- macro 文档 YAML 记录参数、支持的 `input_format`、输出格式和错误/未知格式处理。

完成标准：

- stg SQL 可以复用 macro，不需要手写证券代码转换表达式。
- 字段字典中声明了 `normalization_macro: normalize_cn_security_code` 的字段，有对应 macro 和 data test。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

如已接入可运行 staging 数据，追加：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select staging
```

### Phase 6: 实现 manifest lint

范围：

- `pipeline/elt/scripts/validate_field_glossary.py`
- `pipeline/elt/scripts` 相关测试或 fixtures
- 必要时更新 `pipeline/elt/README.md`

动作：

- 脚本读取：
  - `pipeline/elt/target/manifest.json`
  - `pipeline/elt/metadata/field_glossary.yml`
  - `pipeline/elt/models/sources.yml`
- 第一版只强约束 `models/staging/**` 下的 model columns。
- 校验规则：
  1. staging model column 必须在 YAML 中声明。
  2. staging column 必须有 `description`。
  3. staging column 必须有 `data_type`。
  4. staging column 必须有 `config.meta.glossary_key`，或显式声明 `config.meta.dictionary_scope: local`。
  5. `glossary_key` 必须存在于 `field_glossary.yml`。
  6. 默认 `column.name == glossary_key`。
  7. `column.name != glossary_key` 时必须命中字典允许的 suffix/prefix 规则。
  8. `source_columns` 中的 raw table 和 raw column 必须存在于 generated raw source catalog。
  9. `deprecated_names` 出现在 stg 输出列名时失败。
  10. 公共字段描述必须引用 `{{ doc('field_<glossary_key>') }}`，除非记录 `description_exempt_reason_zh`。
  11. glossary field 定义了 `normalization_macro` 时，stg column 必须声明 `config.meta.normalization.macro` 和 `input_format`。
  12. `normalization.macro` 必须等于 glossary field 的 `normalization_macro`，除非记录 `normalization_exempt_reason_zh`。
  13. glossary field 定义了 `regex` 或 `value_domain` 时，stg column 必须配置对应 generic data test，或记录 `data_test_exempt_reason_zh`。
- lint 输出应包含 model、column、规则编号和修复提示。

完成标准：

- 对缺少 column meta、错误 glossary key、source column 不存在、deprecated column name、缺少 normalization/test 的 staging YAML 能给出明确失败。
- 对 `dictionary_scope: local` 和 `derived_from` 例外能通过，但要求保留 source lineage 或派生说明。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
```

如新增 Python 测试，追加：

```bash
cd pipeline
uv run pytest elt/tests -q
```

### Phase 7: 首批 staging model 治理

范围：

- `pipeline/elt/models/staging/**`
- `pipeline/elt/models/_docs/fields.md`
- `pipeline/elt/metadata/field_glossary.yml`

动作：

- 首批优先治理含证券代码和交易日期的模型，例如：
  - BaoStock 日 K 线。
  - EastMoney equity history。
  - Sina trade calendar。
- 每个 stg SQL 显式 alias raw 字段到 canonical 字段。
- 对 `security_code` 调用 `normalize_cn_security_code()`。
- companion fields 如存在，使用 `cn_security_local_code()` 和 `cn_exchange_code()`。
- 每个 stg YAML column 写入：
  - `description`
  - `data_type`
  - `data_tests`
  - `config.meta.glossary_key`
  - `config.meta.source_columns`
  - `config.meta.normalization`，如字段需要格式标准化
- 对 local 字段写 `dictionary_scope: local` 和 `source_columns`。
- 对派生字段写 `derived_from` 和 `derivation_note_zh`。
- 对暂不能测试或暂不能标准化的字段写中文例外原因，不静默跳过。

完成标准：

- 首批 staging models 通过 manifest lint。
- `security_code` 不以纯数字或 `sh.601088` 作为 stg 输出格式。
- 每个 stg 输出字段都能追溯到 raw source column，或显式说明 local/derived 例外。

验证：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
uv run dbt build --project-dir elt --profiles-dir elt --select staging
```

### Phase 8: 质量门禁和文档收口

范围：

- `AGENTS.md`
- `docs/skills/fleur-contract-data-dictionary/SKILL.md`
- `pipeline/contracts/README.md`
- `pipeline/elt/README.md`
- 本计划

动作：

- 在 repo 入口和 skill 中只保留一个权威边界：
  - contract owns raw facts and generated raw catalog。
  - dbt owns canonical field glossary and staging governance。
- 如 manifest lint 成为固定门禁，在 `AGENTS.md` 或 dbt skill 路由中加入最小命令指针。
- 在本计划状态中记录完成日期和关键验证命令。
- 完成后将本计划移动到 `docs/plans/archive/`，或保留顶层直到首批 staging 治理完成。

完成标准：

- 后续 agent 不会从当前文档读到“contract 管理 stg canonical 字段”的规则。
- RFC 0012 的开放问题若已决策，在本计划或后续 ADR/RFC 中记录，不留在聊天上下文。

最终验证：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run ruff check contract_tools/src contract_tools/tests
uv run ruff format --check contract_tools/src contract_tools/tests
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest contract_tools/tests -q
uv run dbt parse --project-dir elt --profiles-dir elt
uv run python elt/scripts/validate_field_glossary.py
git diff --check
```

如 Phase 7 已创建可运行 staging models，追加：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select staging
```

## 7. 允许例外和禁止模式

允许例外：

- `pipeline/contracts/glossary/tables.yml` 保留为 raw table glossary。
- staging 局部字段允许 `config.meta.dictionary_scope: local`，但必须保留 `source_columns`。
- staging 派生字段允许不直接对应单一 raw column，但必须写 `derived_from` 和 `derivation_note_zh`。
- 早期探索模型可暂时记录 `data_test_exempt_reason_zh` 或 `normalization_exempt_reason_zh`，但不能省略字段 lineage。

禁止模式：

- 不在 `pipeline/contracts/datasets/*.yml` 加回 `glossary_key`、stg field mapping 或 dbt tests。
- 不从 stg SQL 反向解析字段并写回 contract。
- 不手工修改 generated `pipeline/elt/models/sources.yml`。
- 不用 generated `docs/references/data_dict/*.md` 替代 dbt model YAML 文档。
- 不把 raw source column description 写成 canonical 字段语义。
- 不把 `security_code` 实现成纯数字。
- 不在不同 stg SQL 中手写不同证券代码转换表达式。
- 不为了让 lint 通过而伪造不确定的字段业务含义。

## 8. 验收清单

### Contract cleanup

- [ ] `pipeline/contracts/glossary/fields.yml` 已删除。
- [ ] `pipeline/contracts/naming_rules.yml` 已删除。
- [ ] `GlossaryField`、`NamingRules`、`ContractRegistry.glossary_fields` 已删除。
- [ ] `ClickHouseRawField.glossary_key` 已删除。
- [ ] loader 不再读取 field glossary 或 naming rules。
- [ ] description quality 仍校验 source external descriptions，但不校验 contract field glossary。
- [ ] `uv run fleur-contracts validate` 通过。
- [ ] `uv run pytest contract_tools/tests -q` 通过。

### Raw source catalog

- [ ] `pipeline/elt/models/sources.yml` 生成 raw source columns。
- [ ] raw source column `description` 来自 raw/source 语境。
- [ ] raw source column `data_type` 来自 ClickHouse raw type。
- [ ] raw source column `config.meta` 包含 source/parquet/clickhouse raw 字段链路。
- [ ] raw table `meta` 包含 contract version、raw asset、ClickHouse table 和 schema hash。
- [ ] `uv run fleur-contracts generate --check` 通过。
- [ ] `uv run dbt parse --project-dir elt --profiles-dir elt` 通过。

### dbt field glossary

- [ ] `pipeline/elt/metadata/field_glossary.yml` 已新增。
- [ ] `pipeline/elt/models/_docs/fields.md` 已新增。
- [ ] `security_code`、`security_local_code`、`exchange_code`、`trade_date`、`report_date` 已有第一版定义。
- [ ] `security_code` regex 为 `^[0-9]{6}\\.(SH|SZ|BJ)$`。
- [ ] 公共字段 descriptions 可通过 `{{ doc('field_<glossary_key>') }}` 复用。
- [ ] 旧 contract field glossary 的可信条目已迁移或明确放弃。

### dbt standardization

- [ ] `normalize_cn_security_code()` 已实现。
- [ ] `cn_security_local_code()` 已实现。
- [ ] `cn_exchange_code()` 已实现。
- [ ] `eastmoney_suffix` 和 `baostock_prefix` 已支持。
- [ ] `cn_security_code_format` generic test 已实现。
- [ ] macro 文档说明参数、输入格式和输出格式。

### Manifest lint

- [ ] `pipeline/elt/scripts/validate_field_glossary.py` 已新增。
- [ ] lint 能读取 manifest、field glossary 和 raw source catalog。
- [ ] lint 强约束 staging columns 的 description、data_type、glossary_key/local 例外。
- [ ] lint 校验 `source_columns` 指向 generated raw source catalog 中存在的 raw columns。
- [ ] lint 校验 normalization macro 和 data test 要求。
- [ ] lint 失败输出包含 model、column、规则编号和修复提示。
- [ ] `uv run python elt/scripts/validate_field_glossary.py` 通过。

### First staging governance

- [ ] 首批 staging models 已按 source 分目录维护。
- [ ] stg SQL 显式 alias raw 字段到 canonical 字段。
- [ ] 含证券代码的 stg SQL 使用标准化 macro。
- [ ] stg YAML columns 包含 description、data_type、data_tests 和 config.meta。
- [ ] local/derived 字段有显式例外说明。
- [ ] `uv run dbt build --project-dir elt --profiles-dir elt --select staging` 通过，或记录无法运行原因。

### Documentation

- [ ] `docs/skills/fleur-contract-data-dictionary/SKILL.md` 已改为 raw-only。
- [ ] `pipeline/contracts/README.md` 不再提 contract field glossary。
- [ ] `pipeline/elt/README.md` 说明 dbt canonical 字段治理入口。
- [ ] `AGENTS.md` 如有必要已加入 manifest lint 门禁指针。
- [ ] `git diff --check` 通过。

## 9. 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 删除旧 field glossary 后短期字段描述缺口变大 | stg 建模需要重新整理语义 | 只迁移可信条目，无法确认的字段用待核实清单驱动后续补充 |
| generated `sources.yml` 变大 | review diff 更长 | 把 `sources.yml` 视为生成物，review contract dataset diff 和 generator tests |
| manifest lint 第一版过严 | 早期 stg 探索被阻塞 | 第一版仅强约束 staging，允许 local/derived/exempt 例外但要求写明原因 |
| raw 字段描述混入 canonical 语义 | raw/stg 边界再次混淆 | raw source catalog 只使用 `external_description_zh`，canonical 描述只放 dbt field glossary |
| `source_columns` 维护成本高 | stg YAML 变长 | 接受显式 lineage 成本；后续可做 YAML 初始 codegen，但不反向生成 SQL |
| 证券代码格式实现分裂 | join key 漂移 | 强制 stg 使用 macro，manifest lint 和 generic test 双重校验 |
| 标准化 macro 承担业务推断 | staging 变成隐式主数据层 | macro 只做 source-local 确定性转换，跨源修正进入 int/mart |

## 10. 后续决策

以下问题不阻塞第一版，但应在实施中记录事实并择机转成 ADR/RFC 或后续计划：

1. 是否为 `field_glossary.yml` 生成 `docs/references/dbt_field_glossary.md`。
2. 是否将 raw source catalog 拆分为 `models/sources/raw/*.yml`，降低 generated diff 压力。
3. mart 指标字段是否使用独立 metric glossary。
4. raw source column `data_type` 是否长期保留 ClickHouse 类型字符串，还是映射为 dbt adapter 通用类型。
5. 非 A 股证券、基金、债券、港股、美股接入后，`security_code` 是否扩展为 market-qualified 标准，还是新增 `market_code` companion field。
6. 日期、金额单位、比例、交易状态是否需要和 `security_code` 同级的 value standard。

## 11. 完成状态

计划完成后，最终边界应稳定为：

```text
contracts:
  source/parquet/clickhouse raw facts only
  generated raw data_dict
  generated dbt raw source catalog

dbt:
  canonical field glossary
  docs blocks
  standardization macros
  generic data tests
  staging YAML meta and lineage
  manifest lint
```

`pipeline/contracts` 不再知道 stg 字段名。`pipeline/elt` 不再从 contract field glossary 继承 canonical 命名。两者只在 dbt `source('raw', ...)` 和 stg YAML `source_columns` 中显式相遇。
