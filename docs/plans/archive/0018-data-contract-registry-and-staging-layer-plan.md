# Plan 0018: 数据契约注册表与 stg 层实施计划

日期：2026-06-01

关联文档：

- `docs/RFC/archive/0010-data-contract-registry-and-contract-tools.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/architecture/scheduler-architecture.md`
- `docs/architecture/scheduler-module-boundaries.md`
- `docs/plans/0017-dagster-clickhouse-raw-sync-implementation-plan.md`
- `docs/references/data_dict/README.md`

## 1. 目标

本计划把当前手写 `docs/references/data_dict/*.md` 升级为可校验、可生成、可复用的数据契约注册表，用来统一记录每个数据集从远端响应到 S3 Parquet、ClickHouse raw table、dbt staging model 的字段和类型演化。

目标产物：

```text
http/tcp source payload
  -> S3 Parquet source asset
  -> ClickHouse raw table
  -> dbt source()
  -> dbt staging model
  -> generated human-readable data_dict
```

完成后应满足：

- 每个进入 ClickHouse raw 和 dbt staging 的数据集都有一份机器可读 contract。
- contract 记录字段粒度、主键候选、分区策略、类型转换、nullable/default 策略和字段重命名规则。
- contract 区分外源字段命名和 mono-fleur canonical 字段命名。
- 中文描述集中维护在 dataset contract 和 glossary 中，不散落在 SQL、dbt YAML 和 Markdown 里。
- raw sync specs、dbt source YAML、staging YAML 和 data_dict 文档不再各自手写一套字段事实。
- CI 可以检查 contract 与 ClickHouse `system.columns`、Parquet schema、dbt model YAML 的一致性。
- Dagster materialization metadata 可以记录 contract version、schema hash 和主要字段统计，便于排查类型漂移。

## 2. 非目标

本计划不做以下事情：

- 不引入 DataHub、OpenMetadata、Amundsen 等独立数据目录平台。
- 不改变 ADR 0005：Dagster 继续负责 ClickHouse raw sync，dbt 继续负责 staging/marts 建模。
- 不让 dbt 直接读取 S3 Parquet 或远端 HTTP/TCP 数据源。
- 不把 contract 设计成业务指标语义层；指标口径留给后续 mart/semantic layer。
- 不保留“先试点、后迁移”的长期双轨；当前表数量有限且结构稳定，本计划按一次性收敛设计。
- 不把所有原始字段都强制变成 ClickHouse `String`；类型选择仍要按实际数据和查询需求收敛。
- 不把 contract tools 拆到独立仓库；工具代码作为 `pipeline` uv workspace member 维护。

## 3. 当前事实基线

当前已存在：

- ClickHouse raw sync 代码：
  - `pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py`
  - `pipeline/scheduler/src/scheduler/defs/clickhouse/sql.py`
  - `pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py`
  - `pipeline/scheduler/src/scheduler/defs/clickhouse/assets.py`
- ClickHouse resource：
  - `pipeline/scheduler/src/scheduler/defs/resources/clickhouse.py`
- dbt 初始 staging：
  - `pipeline/elt/models/sources.yml`
  - `pipeline/elt/models/staging/staging.yml`
  - `pipeline/elt/models/staging/stg_*.sql`
- 人类可读字段文档：
  - `docs/references/data_dict/*.md`

当前问题：

- data_dict 是 Markdown，适合阅读，不适合作为代码生成和 CI 校验的唯一事实来源。
- `ClickHouseRawTableSpec`、dbt `sources.yml`、staging YAML 和 data_dict 之间存在重复字段定义。
- JSON/HTTP/TCP 原始字段、Parquet 类型、ClickHouse 类型和 dbt staging 类型之间没有一份结构化转换记录。
- 字段类型变更时，缺少 schema hash、contract version 和机械校验来阻止文档与真实表漂移。

## 4. 设计原则

### 4.1 分层职责

| 层 | 职责 |
|----|------|
| Contract registry | 字段事实、类型演化、stg 输出契约、schema version |
| Dagster source assets | 远端采集、S3 Parquet 写入、source asset metadata |
| Dagster ClickHouse raw sync | S3 Parquet 到 ClickHouse raw 的装载、校验、替换 |
| dbt source/staging | 从 ClickHouse raw 读取，做字段命名、轻清洗、类型收敛和基础测试 |
| data_dict Markdown | 从 contract 生成的人类可读参考文档 |
| contract_tools | 读取 contract/glossary，生成和校验 scheduler/dbt/docs 相关产物 |

### 4.2 命名与中文描述边界

raw 层保留外源命名，stg 层开始使用 mono-fleur canonical naming：

| 层 | 字段命名 |
|----|----------|
| source payload | 外源原始字段名 |
| S3 Parquet | 外源字段名或当前 source asset 发布字段名 |
| ClickHouse raw | 外源字段名，不做系统级重命名 |
| dbt staging | mono-fleur canonical `snake_case` 字段名 |

中文描述分三类维护：

| 字段 | 位置 | 说明 |
|------|------|------|
| `external_description_zh` | `pipeline/contracts/datasets/*.yml` | 外源文档或供应商语境下的字段描述 |
| `description_zh` | `pipeline/contracts/glossary/fields.yml` | mono-fleur 系统统一字段语义 |
| `dataset_note_zh` | `pipeline/contracts/datasets/*.yml` | 当前数据集特有异常值、清洗规则或口径说明 |

### 4.3 ClickHouse 类型约束

本计划的 ClickHouse 字段设计遵守以下规则：

- Per `schema-types-native-types`：已知结构字段优先使用 ClickHouse 原生类型，不默认使用 `String`。
- Per `schema-types-lowcardinality`：低基数字符串才使用 `LowCardinality(String)`，阈值和实际 `uniq()` 统计应记录到 contract 或验证报告。
- Per `schema-json-when-to-use`：只有字段结构动态且需要保留动态属性时才使用 ClickHouse JSON 类型；固定 schema 应拆成 typed columns。
- Per `schema-types-avoid-nullable`：只有原始语义确实允许缺失、且默认值会误导下游时才使用 `Nullable`。
- Per `schema-pk-plan-before-creation` 和 `schema-pk-prioritize-filters`：raw 表 `ORDER BY` 应在 contract 中记录高频过滤字段和排序键原因。

官方参考：

- ClickHouse Select Data Types: `https://clickhouse.com/docs/best-practices/select-data-types`
- ClickHouse Use JSON Where Appropriate: `https://clickhouse.com/docs/best-practices/use-json-where-appropriate`

## 5. Contract 格式

第一版使用 YAML 维护契约数据，用独立 `contract_tools` workspace package 维护 schema、CLI、生成器和校验器。

```text
pipeline/
  contract_tools/
    pyproject.toml
    src/fleur_contracts/
      cli.py
      schema.py
      loader.py
      validate.py
      generate.py
      validate_parquet.py
      validate_clickhouse.py
      adapters/
        clickhouse.py
        dbt.py
        data_dict.py
        parquet.py
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

单个 contract 示例：

```yaml
dataset: baostock__query_history_k_data_plus_daily
version: 1
owner: data
grain: one row per stock code per trade date
source_asset_key: ["source", "baostock__query_history_k_data_plus_daily"]
raw_asset_key: ["clickhouse", "raw", "baostock__query_history_k_data_plus_daily"]

external:
  provider: baostock
  source_table_name: baostock__query_history_k_data_plus_daily
  source_description_zh: BaoStock 日频行情数据

source:
  protocol: tcp
  payload_format: tabular
  fields:
    - name: code
      type: string
      required: true
      external_description_zh: 证券代码
    - name: date
      type: string
      required: true
      external_description_zh: 交易日期
    - name: close
      type: string
      required: false
      external_description_zh: 收盘价

parquet:
  storage_mode: partitioned
  partition_key_name: year
  fields:
    - name: code
      type: string
      nullable: false
    - name: date
      type: date32
      nullable: false
    - name: close
      type: decimal128(18, 4)
      nullable: true

clickhouse_raw:
  database: raw
  table: baostock__query_history_k_data_plus_daily
  partition_strategy: year
  engine: MergeTree
  partition_by: toYear(date)
  order_by: [code, date]
  fields:
    - name: code
      type: LowCardinality(String)
      from: code
      nullable: false
      glossary_key: stock_code
      reason: low_cardinality_security_code
    - name: date
      type: Date
      from: date
      nullable: false
      glossary_key: trade_date
    - name: close
      type: Nullable(Decimal(18, 4))
      from: close
      nullable: true

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
    - name: close_price
      from: close
      glossary_key: close_price
      type: Decimal(18, 4)
```

## 6. 一次性实施方案

当前表数量不多，且字段类型和结构已经通过 data_dict、ClickHouse raw sync smoke test、dbt staging 初稿形成稳定基线。本计划不按“试点一批、观察一批、再迁移一批”的方式推进，而是一次性完成 contract、生成、校验、接入和文档收敛。

### 6.1 本次一次性覆盖范围

必须一次性纳入 contract 的 ClickHouse raw 数据集：

| 类别 | datasets |
|------|----------|
| snapshot | `sina__trade_calendar`、`baostock__query_stock_basic`、`jiuyan__industry_list`、`jiuyan__industry_ocr_snapshot` |
| BaoStock 年度分区 | `baostock__query_history_k_data_plus_daily` |
| compacted 年度分区 | `jiuyan__action_field_compacted`、`ths__limit_up_pool_compacted` |
| EastMoney 年度分区 | `eastmoney__balance`、`eastmoney__cashflow_sq`、`eastmoney__cashflow_ytd`、`eastmoney__dividend_allotment`、`eastmoney__dividend_main`、`eastmoney__equity_history`、`eastmoney__income_sq`、`eastmoney__income_ytd` |

必须一次性纳入 dbt staging contract 的当前 models：

- `stg_sina__trade_calendar`
- `stg_baostock__query_stock_basic`
- `stg_baostock__query_history_k_data_plus_daily`
- `stg_jiuyan__industry_list`
- `stg_jiuyan__industry_ocr_snapshot`

compact/EastMoney raw 数据集即使暂时没有 stg SQL，也必须在 contract 中明确 `dbt_staging: null` 或 `dbt_staging.status: not_started`，避免后续误以为 contract 缺失。

### 6.2 单次实施步骤

#### Step 1：建立 contract 包和 schema

实施内容：

- 新增 `pipeline/contract_tools/` uv workspace member：
  - `pyproject.toml`：package 名称建议为 `contract-tools`，导出 CLI `fleur-contracts`。
  - `schema.py`：Pydantic contract schema。
  - `loader.py`：本地 YAML 加载、排序和 schema hash 计算。
  - `validate.py`：contract 静态校验 CLI。
  - `generate.py`：生成 dbt YAML、data_dict Markdown 和 ClickHouse raw columns。
  - `validate_parquet.py`：Parquet 真实 schema 校验 CLI。
  - `validate_clickhouse.py`：ClickHouse metadata schema 校验 CLI。
- 在 `pipeline/pyproject.toml` 的 workspace members 中加入 `contract_tools`。
- 在 `pipeline/scheduler/pyproject.toml` 中加入 `contract-tools` workspace dependency，使 scheduler definitions 可导入 `fleur_contracts.adapters.clickhouse`。
- 更新 `pipeline/pyproject.toml` 中 Ruff、Pyright、Pytest 配置，让 `contract_tools/src` 和 `contract_tools/tests` 纳入质量门禁。
- 新增 `pipeline/contracts/glossary/fields.yml`、`pipeline/contracts/glossary/tables.yml` 和 `pipeline/contracts/naming_rules.yml`。
- 新增 `pipeline/contracts/datasets/*.yml`，一次性覆盖 15 个 ClickHouse raw 数据集。
- contract schema 必须支持 `dbt_staging` 可选，但 `clickhouse_raw` 对本批数据集必填。
- contract schema 必须校验以下引用关系：
  - ClickHouse field `from` 指向 Parquet field。
  - dbt staging field `from` 指向 ClickHouse raw field。
  - stg field `glossary_key` 存在且 canonical name 与 stg field name 一致。
  - 没有 `glossary_key` 的 stg field 必须显式声明 `canonical_exempt: true` 和 `exempt_reason`。
  - `partition_key_name` 存在于 Parquet 或 asset partition metadata。
  - `order_by` 字段存在于 ClickHouse raw fields。
  - stg 字段符合 `naming_rules.yml` 的 canonical naming 规则。

完成标准：

- 15 个 YAML 全部可加载。
- dataset 文件名、`dataset` 字段、raw table 名称一致。
- 所有字段转换链路可静态解析。
- 当前 5 个 stg models 的字段都能解析到 glossary，例外字段必须显式标记。

#### Step 2：一次性迁移字段事实

实施内容：

- 以当前 `docs/references/data_dict/*.md` 和 `pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py` 为迁移输入。
- 对每个 dataset 迁移：
  - source 协议和 payload 形态。
  - S3 storage mode 和 partition key。
  - Parquet 字段名、类型、nullable。
  - ClickHouse raw 字段名、类型、nullable/default、`LowCardinality` 原因。
  - raw table engine、partition strategy、`ORDER BY`。
  - 当前已有 stg model 的字段重命名和基础 tests。
- 对每个外源字段补齐 `external_description_zh`。
- 对每个 stg canonical 字段补齐或引用 `glossary_key`。
- 对数据集特有含义补齐 `dataset_note_zh`，不把 dataset 特例写进全局 glossary。

完成标准：

- 15 个 ClickHouse raw contract 字段数量与当前 raw specs/data_dict 一致。
- 5 个现有 stg models 的输出字段全部有 contract 记录。
- 所有 `LowCardinality(String)` 字段都有决策原因；已知 `uniq()` 统计可写入 `validation_notes`。
- 中文描述只存在于 contract/glossary 中，dbt YAML 和 data_dict 的中文说明由生成器写入。

#### Step 3：生成物一次性替换

实施内容：

- 生成并写回：
  - `pipeline/elt/models/sources.yml`
  - `pipeline/elt/models/staging/staging.yml`
  - `docs/references/data_dict/*.md`
- 每个生成的 data_dict 顶部加来源说明：

```text
本文件由 pipeline/contracts/datasets/<dataset>.yml 生成。字段事实以 contract 为准。
```

- `stg_*.sql` 不模板化，继续手写；但 SQL 输出列必须被 contract/staging YAML 覆盖。
- `sources.yml` 和 `staging.yml` 的 `meta` 写入：

```yaml
meta:
  contract_dataset: baostock__query_history_k_data_plus_daily
  contract_version: 1
  upstream_raw_asset: clickhouse/raw/baostock__query_history_k_data_plus_daily
```

完成标准：

- 生成器写回后再次运行生成器无 diff。
- data_dict 与 contract 不存在字段事实冲突。
- dbt YAML 与 contract 不存在字段事实冲突。
- 生成的 data_dict 能同时展示外源字段名、raw 字段名、stg 字段名、glossary 中文名称和中文描述。

#### Step 4：一次性接入 Dagster raw sync specs

实施内容：

- 在 `pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py` 中通过 `fleur_contracts.adapters.clickhouse` 从 contract 构造 `ClickHouseRawTableSpec`。
- 保留现有 `ClickHouseRawTableSpec` dataclass 和 raw sync service，不重写 raw sync 执行协议。
- definitions 加载阶段只读取本地 YAML，不访问 S3、ClickHouse 或远端服务。
- raw sync materialization metadata 增加：
  - `contract_dataset`
  - `contract_version`
  - `contract_schema_hash`
  - `source_schema_hash`
  - `clickhouse_schema_hash`
- 对 contract 缺失、字段引用错误、重复 table 名称、重复 asset key 直接在 definitions load 或单元测试中失败。

完成标准：

- 当前 15 个 `clickhouse/raw/*` assets 仍然完整注册。
- `uv run dg list defs --json` 中 raw asset key 不变化。
- raw sync 相关单元测试通过。

#### Step 5：一次性接入真实 schema 校验

实施内容：

- Parquet 校验：
  - snapshot 数据集读取 latest snapshot object。
  - 年度分区数据集至少校验当前年份分区；有 smoke test 数据的优先校验 2026。
  - 校验字段名、Arrow/Parquet 类型、nullable 和 partition key。
- ClickHouse 校验：
  - 查询 `system.columns` 校验字段名、类型、默认值和顺序。
  - 查询 `system.tables` 或 `SHOW CREATE TABLE` 校验 engine、partition key 和 sorting key。
  - 校验 `LowCardinality` 字段的实际类型。
- dbt 校验：
  - `dbt compile --select staging.*`
  - `dbt build --select staging.*`
- 校验结果不得打印 secret；只记录 dataset、bucket/object key、database/table、schema hash 和 mismatch。

完成标准：

- 15 个 raw 数据集都能做 contract 静态校验。
- 已存在 ClickHouse raw table 的数据集都能做 ClickHouse schema 校验。
- 已存在 S3 object 的数据集都能做 Parquet schema 校验。
- 当前 5 个 stg models build 通过。

#### Step 6：一次性补测试和架构约束

实施内容：

- 新增 contract 单元测试：
  - YAML schema load。
  - dataset name 与文件名一致。
  - asset key 格式合法。
  - field `from` 引用合法。
  - stg field `glossary_key` 引用合法。
  - stg field name 符合 canonical naming rules。
  - canonical 例外字段必须有 `canonical_exempt: true` 和 `exempt_reason`。
  - generated dbt YAML/data_dict 无 diff。
  - `ClickHouseRawTableSpec` adapter 输出与 contract 一致。
- 新增或更新架构边界测试：
  - data source 业务代码不能直接解析 contract。
  - contract 读取集中在 `pipeline/contract_tools`、ClickHouse adapter 和 dbt adapter。
  - generated 文件不应被手工改出 contract 字段事实。
- 更新 `docs/references/data_dict/README.md`，说明 contract registry 是新的字段事实入口。
- 更新 `docs/architecture/scheduler-architecture.md` 或 `AGENTS.md`，加入 contract registry 路由。

完成标准：

- 新增数据集时，缺 contract 或字段链路不完整会在测试中失败。
- contract 变更后，生成物未同步会在测试中失败。

### 6.3 单次验收命令

一次性实施完成后，必须运行：

```bash
cd pipeline

uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available

uv run dbt compile --project-dir elt --profiles-dir elt --select staging.*
uv run dbt build --project-dir elt --profiles-dir elt --select staging.*

uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run ruff format --check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests --cov=scheduler/src/scheduler --cov=contract_tools/src/fleur_contracts --cov-report=term-missing
uv run dg check defs --target-path scheduler

git diff --check
```

## 7. 禁止模式

- 不在多个 YAML、Python specs、Markdown 中手动复制同一套字段类型。
- 不让 generated docs 成为事实源；字段事实以 contract 为准。
- 不在 Dagster definitions 加载阶段连接 ClickHouse、S3 或远端服务做 schema discovery。
- 不把 stg 层做成 raw 的简单 `select *` 镜像；stg 必须表达字段命名、类型收敛、粒度和基础测试。
- 不把 contract 作为运行时热配置；生产运行依赖随代码发布的版本化 contract。
- 不在文档中记录 secret、cookie、token 或完整连接串。

## 8. 验收标准

计划完成时应满足：

- 当前 15 个 ClickHouse raw 数据集都有 contract YAML。
- 当前 5 个 stg models 都有 dbt staging contract。
- 当前 5 个 stg models 的 canonical 字段都有 glossary 记录，例外字段有显式说明。
- `scheduler` 显式依赖 `contract-tools` workspace package，且 `contract_tools` 不反向依赖 `scheduler`。
- `ClickHouseRawTableSpec` 字段定义来自 contract adapter。
- dbt `sources.yml` 和 `staging.yml` 由 contract 生成或严格校验。
- data_dict Markdown 可从 contract 生成。
- 中文描述只维护在 `pipeline/contracts/datasets` 和 `pipeline/contracts/glossary`，生成物不作为中文描述事实源。
- 所有可访问 S3 object 的数据集完成 Parquet schema 校验。
- 所有已存在 ClickHouse raw table 的数据集完成 ClickHouse schema 校验。
- 当前 5 个 stg models 完成 dbt compile/build。
- Dagster raw sync metadata 记录 contract version 和 schema hash。
- `make webui` / `uv run dg check defs --target-path scheduler` 不因 contract 加载访问外部服务。

## 9. 后续扩展

- 后续新增 raw 或 stg 数据集时，必须先新增 contract，再接入 raw sync 或 dbt model。
- 当 contract 字段说明变多后，可评估把字段描述拆成 `description` 和 `business_description`。
- 当 mart 层成型后，再设计 mart contract 或 metric/semantic layer；不要提前把指标口径塞进 raw/stg contract。
- 如果 contract 生成物和人工 SQL 之间的漂移变多，再考虑 dbt model SQL 的受控模板化；第一版不模板化 SQL。
