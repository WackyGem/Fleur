# Plan 0020: 字段类型规范化债务清偿实施计划

日期：2026-06-01

关联文档：

- `docs/debt/archive/0001-2026-06-01-field-type-normalization.md`
- `docs/plans/0018-data-contract-registry-and-staging-layer-plan.md`
- `docs/plans/0019-contract-zh-description-quality-remediation-plan.md`
- `docs/RFC/0010-data-contract-registry-and-contract-tools.md`
- `pipeline/contracts/README.md`
- `docs/references/data_dict/README.md`

## 1. 背景

`docs/debt/archive/0001-2026-06-01-field-type-normalization.md` 已经记录当前 contract 项目的字段类型和命名债务。债务范围不再只限于 BaoStock，已经扩展到四类问题：

- 外源字符串编码在 Parquet、ClickHouse raw 或 dbt staging 中缺少布尔、日期、枚举等语义类型收敛。
- stg 层仍暴露外源或泛化字段名，例如 `code`、`code_name`、`tradestatus`。
- source-only Dagster asset 尚未进入 contract registry，导致旧 data_dict Markdown 仍承载字段事实。
- 部分 ClickHouse `LowCardinality(String)` 使用缺少真实基数依据，或者已由样本证明不适合低基数字典编码。

Plan 0018 已经确立 contract registry 是字段事实源，Plan 0019 已经把中文描述质量纳入 `fleur-contracts validate`。本计划在这两个基础上清偿字段类型债务，并把类型判断规则沉淀为可执行校验和可复查记录。

## 2. 目标

完成后应满足：

- `docs/debt/archive/0001-2026-06-01-field-type-normalization.md` 中 13 项债务全部关闭，或在债务文件中明确标记为因外部依赖暂缓且有下一步动作。
- source payload、Parquet、ClickHouse raw、dbt staging 的字段类型链路在 contract 中保持一致、可生成、可校验。
- stg 层字段名优先使用 mono-fleur canonical 命名；外源字段名只保留在 source、Parquet 和 raw 层。
- source-only asset 也由 contract registry 维护字段事实，旧 data_dict Markdown 不再独立承载字段定义。
- `LowCardinality(String)` 字段必须有 `reason` 和真实数据基数依据；高基数字段收敛为普通 `String`。
- 已存在 ClickHouse raw 表的类型变更有迁移、重建或回填窗口说明，不把 contract 改动伪装成存储层已完成。

## 3. 非目标

- 不在本计划内一次性重构全部 EastMoney schema 生成机制；只改动债务清单涉及字段和必要的生成源。
- 不把 contract registry 扩展成完整业务指标语义层。
- 不为了避免 `Nullable` 直接把真实 `null` 静默改成误导性默认值；默认值策略必须逐字段说明。
- 不在 generated data_dict、dbt YAML 或手写 Markdown 中直接修字段事实。
- 不在没有真实统计或远端核验依据时批量移除所有 `LowCardinality(String)`。

## 4. 当前事实基线

当前 contract 事实源和生成器位置：

- dataset contract：`pipeline/contracts/datasets/*.yml`
- glossary：`pipeline/contracts/glossary/*.yml`
- contract schema：`pipeline/contract_tools/src/fleur_contracts/schema.py`
- contract 校验：`pipeline/contract_tools/src/fleur_contracts/validate.py`
- dbt YAML 生成：`pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- data_dict 生成：`pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py`
- ClickHouse raw spec 生成：`pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py`

当前 schema 约束仍假设每个 dataset 都有 `raw_asset_key` 和 `clickhouse_raw`，因此不能表达 source-only contract。`LowCardinality(String)` 已要求填写 `reason`，但尚未要求记录真实 `count()`、`uniq()` 或样本范围。`source.fields[].required` 可以表达外源必填性，但 EastMoney nullable 债务还需要逐字段核实“远端可能为 null”和“本地转换如何处理 null”。

当前 `pipeline/contract_tools` 不生成、修改或回写 Dagster source asset 的 Parquet schema。`fleur-contracts generate` 只写 dbt YAML 和 `docs/references/data_dict/*.md`；scheduler 侧的 ClickHouse raw spec 会通过 `fleur_contracts.adapters.clickhouse` 在运行时读取 contract，但 BaoStock、THS、Jiuyan、Sina 等 source asset 写 Parquet 使用的 `pa.Schema` 仍在 Dagster 项目代码中维护。例如 BaoStock 日频行情 schema 定义在 `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py` 的 `K_HISTORY_DAILY_SCHEMA`。因此，只要 contract 的 `parquet.fields[].type` 改变，就必须把对应 Dagster Parquet schema 和响应转换测试纳入同一批次；不能假设 contract 生成器会自动修改 scheduler 代码。

### 4.1 Dagster `pa.Schema` 影响面

本计划中的债务分为三类，执行时必须先判断是否影响 Dagster 发布到 S3 的 Parquet schema。

第一类是会改变 Parquet 类型的债务，必须同步修改 scheduler `pa.Schema`：

| 债务 | scheduler schema 位置 | 当前 scheduler 类型 | 目标 scheduler 类型 | 说明 |
|------|------------------------|---------------------|---------------------|------|
| BaoStock `isST` | `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py` 的 `K_HISTORY_DAILY_SCHEMA` | `pa.int8()` | `pa.bool_()` | source 仍是外源字符串，进入 Arrow/Parquet 时转布尔 |
| EastMoney `EX_DIVIDEND_DATEE` | `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`，由 EastMoney schema 生成流程维护 | `pa.string()` | `pa.date32()` 或必要时 `pa.timestamp("ns")` | 与 contract 的 Parquet 类型同步，不能只手工改 generated 文件 |
| EastMoney `REPORT_TIME` | 同上 | `pa.string()` | 优先评估 `pa.date32()` | 只有确认报告期截止日语义后才改；若保留字符串，债务不得标记关闭 |

第二类是不改变 Parquet 类型，但需要同步确认 scheduler schema 已经匹配 contract：

| 债务 | scheduler schema 位置 | 目标处理 |
|------|------------------------|----------|
| BaoStock `tradestatus` | `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py` 的 `K_HISTORY_DAILY_SCHEMA` | 保持 `pa.int8()`，语义化只发生在 stg |
| BaoStock `stock_basic.status` | `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py` 的 `STOCK_BASIC_SCHEMA` | 保持 `pa.int8()`，语义化只发生在 stg |
| EastMoney `DAT_YAGGR`、`EQUITY_RECORD_DATE`、`EX_DIVIDEND_DATE`、`PAY_CASH_DATE`、`GMDECISION_NOTICE_DATE`、`LAST_TRADE_DATE` | `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py` | 当前已经是 `pa.date32()`，重点核实 nullable/default 策略和 source 外源类型事实 |
| EastMoney `OPINION_TYPE`、`OSOPINION_TYPE` | `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py` | 当前 `pa.string()` 合理，主要修 source contract 外源类型和 nullable 事实 |
| EastMoney `INFO_CODE` | `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py` | 当前 `pa.string()` 合理；债务是 ClickHouse raw 不应使用 `LowCardinality(String)` |

第三类是只改 ClickHouse raw 类型或 contract 表达，不改变 source Parquet schema：

| 债务 | scheduler schema 位置 | 目标处理 |
|------|------------------------|----------|
| `jiuyan__action_field_compacted.action_field_id`、`name`、`reason`、`expound` | `pipeline/scheduler/src/scheduler/defs/http/schemas.py` 的 `JIUYAN_ACTION_FIELD_SCHEMA` | 保持 `pa.string()`；是否移除 `LowCardinality(String)` 只影响 ClickHouse raw contract |
| `jiuyan__industry_ocr_snapshot.industry_id`、`theme_path`、`relation` | `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py` 与 `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py` | 保持 `pa.string()`；是否移除 `LowCardinality(String)` 只影响 ClickHouse raw contract |
| `jiuyan__industry_list.industry_id` | `pipeline/scheduler/src/scheduler/defs/http/schemas.py` 的 `JIUYAN_INDUSTRY_LIST_SCHEMA` | 保持 `pa.string()`；债务是 ClickHouse raw 唯一标识不应默认低基数 |
| `ths__limit_up_pool_compacted.reason_type` | `pipeline/scheduler/src/scheduler/defs/http/schemas.py` 的 `THS_LIMIT_UP_POOL_SCHEMA` | 保持 `pa.string()`；先统计再决定 raw 是否保留 `LowCardinality(String)` |

EastMoney schema 的维护路径需要单独约束：`pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py` 是生成文件，当前生成脚本是 `pipeline/scheduler/scripts/generate_eastmoney_schemas.py`。执行 EastMoney Parquet 类型变更时，必须同步修正 contract、生成输入或生成脚本，再重新生成 scheduler schema；禁止只手工改 `generated/schemas.py`。

ClickHouse 类型清偿遵守以下规则：

- Per `schema-types-native-types`：日期、时间、布尔、金额、数值等已知结构字段优先使用原生类型，不长期保留为字符串。
- Per `schema-types-minimize-bitwidth`：`0/1` 标记不得长期用有符号整数表达布尔语义。
- Per `schema-types-lowcardinality`：低基数字符串才使用 `LowCardinality(String)`；超过 10,000 个唯一值或唯一值接近行数时应使用普通 `String`。
- Per `schema-types-enum`：固定且稳定的状态集合可考虑 `Enum8`/`Enum16`，供应商值可能变动时优先在 stg 层输出语义字符串。
- Per `schema-types-avoid-nullable`：ClickHouse raw 避免滥用 `Nullable`，但真实缺失且默认值会误导下游时必须显式表达 nullable 或默认值策略。

## 5. 实施阶段

### Phase 0：债务基线冻结与核验矩阵

范围：

- `docs/debt/archive/0001-2026-06-01-field-type-normalization.md`
- `pipeline/contracts/datasets/*.yml`
- `docs/references/remote_endpoint/*.md`
- `docs/references/openapi/*.yaml`

动作：

- 为 13 项债务建立执行矩阵，记录数据集、字段、当前类型链路、目标类型链路、是否需要远端核验、是否需要 ClickHouse 表迁移。
- 执行矩阵必须包含 `parquet_schema_owner` 字段，用于标记 Dagster `pa.Schema` 的维护位置：`baostock/schemas.py`、`http/schemas.py`、`eastmoney/generated/schemas.py`、`jiuyan/ocr_schema.py`、`industry_ocr_snapshot.py` 或“不影响 Parquet schema”。
- 对 EastMoney 字段继续保留已 curl 核验结论：`INFO_CODE` 是高基数公告编号；`OPINION_TYPE` / `OSOPINION_TYPE` 债务是 source 外源类型错误，不是 raw `LowCardinality(String)` 错误。
- 对所有待调整 `LowCardinality(String)` 字段补充统计命令或样本来源，避免只凭字段名判断。

完成标准：

- 债务文件中的每个条目都能映射到后续 Phase。
- 每个债务都明确是否需要同步修改 Dagster `pa.Schema`；需要修改时必须列出具体文件和测试。
- 无法立即核验的数据字段保留为“需要统计后决策”，不能直接改 contract 类型。

### Phase 1：contract_tools 表达能力补齐

范围：

- `pipeline/contract_tools/src/fleur_contracts/schema.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/data_dict.py`
- `pipeline/contract_tools/tests/test_contract_registry.py`

动作：

- 支持 source/parquet-only dataset contract，允许 source-only asset 不定义 `raw_asset_key` 和 `clickhouse_raw`。
- dbt source YAML 和 ClickHouse raw specs 只为存在 active `clickhouse_raw` 的 dataset 生成。
- data_dict 生成器能渲染 source/parquet-only contract；这类文档不展示伪 ClickHouse 类型。
- 扩展 contract schema 或 validation notes，要求 `LowCardinality(String)` 的 `reason` 能关联样本统计、远端枚举事实或稳定业务枚举。
- 保留现有 active raw dataset 的严格约束：真实 raw dataset 的 `raw_asset_key` 仍必须是 `["clickhouse", "raw", dataset]`，`clickhouse_raw.table` 仍必须等于 dataset。

完成标准：

- 新增测试覆盖 source-only contract、active raw contract、dbt YAML 过滤、data_dict 渲染。
- `uv run fleur-contracts validate` 能区分 source-only dataset 和 raw-sync dataset。

### Phase 2：BaoStock 类型和 stg canonical 命名收敛

范围：

- `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml`
- `pipeline/contracts/datasets/baostock__query_stock_basic.yml`
- `pipeline/contracts/glossary/fields.yml`
- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`
- `pipeline/elt/models/staging/stg_baostock__query_history_k_data_plus_daily.sql`
- `pipeline/elt/models/staging/stg_baostock__query_stock_basic.sql`

动作：

- `isST` 保留 source `string`，从 Parquet 起收敛为 `bool`，ClickHouse raw 使用 `Bool`，stg 输出 `is_st Bool`。
- 同步更新 Dagster BaoStock source asset 的 Parquet schema：`K_HISTORY_DAILY_SCHEMA` 中 `isST` 必须从 `pa.int8()` 改为 `pa.bool_()`，确保 source 写出的 Parquet 文件已经是布尔列。
- 保持外源响应字段列表 `K_HISTORY_DAILY_FIELDS` 和 BaoStock 请求参数不变；改变的是 source 响应进入 Arrow/Parquet 时的类型转换，不改变远端请求字段名。
- 确认 `response_to_table()` 经过 `typed_table()` 后会把 BaoStock 返回的 `"0"` / `"1"` 转成 `False` / `True`；如现有通用 `to_bool()` 不满足 BaoStock 约束，则补字段级 converter，而不是把 Parquet 继续写成 `Int8`。
- `tradestatus` 保留 Parquet/raw `int8`/`Int8` 外源状态码，stg 输出语义字段，例如 `trading_status`，必要时增加 `is_trading Bool`。
- `stock_basic.status` 保留 Parquet/raw `int8`/`Int8` 外源状态码，stg 输出 `stock_status` 语义枚举值，必要时增加 `is_listed Bool`。
- stg 层把 `code` / `code_name` 收敛为 `security_code` / `security_name`，同步更新 glossary 和 primary key。
- 搜索并同步修复下游对 `stg_baostock__*`.`code`、`code_name`、`tradestatus` 的引用。

完成标准：

- BaoStock source schema、contract、stg SQL、generated dbt YAML 和 data_dict 一致。
- BaoStock 单元测试覆盖 `k_history_daily_response_to_table()` 输出的 `isST` Arrow 类型为 `bool`，并覆盖 `"1" -> True`、`"0" -> False`、空值或异常值处理。
- contract 中 `source.fields[].type` 仍记录外源真实 `string`；`parquet.fields[].type`、`clickhouse_raw.fields[].type` 和 Dagster `pa.Schema` 同步记录布尔语义。

### Phase 3：source-only asset 纳入 contract registry

范围：

- `pipeline/contracts/datasets/jiuyan__action_field.yml`
- `pipeline/contracts/datasets/jiuyan__industry_ocr.yml`
- `pipeline/contracts/datasets/ths__limit_up_pool.yml`
- `pipeline/contracts/datasets/jiuyan__action_field_compacted.yml`
- `pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml`
- `pipeline/contracts/datasets/ths__limit_up_pool_compacted.yml`
- `docs/references/data_dict/jiuyan__action_field.md`
- `docs/references/data_dict/jiuyan__industry_ocr.md`
- `docs/references/data_dict/ths__limit_up_pool.md`

动作：

- 为三个 source-only asset 新增 source/parquet-only contract，不使用假的 `clickhouse_raw`。
- 对照现有 Dagster source-only Parquet schema 建 contract：`JIUYAN_ACTION_FIELD_SCHEMA`、`JIUYAN_INDUSTRY_OCR_SCHEMA`、`THS_LIMIT_UP_POOL_SCHEMA` 是这三个 source-only contract 的初始 Parquet 类型基线。
- 如果 source-only contract 的 `parquet.fields[].type` 与现有 scheduler `pa.Schema` 不一致，先判断是 contract 草案错误还是 scheduler schema 需要迁移；不能让 source-only contract 和实际 Parquet 发布 schema 分叉。
- downstream compacted/snapshot contracts 记录上游 source-only asset lineage 或 validation note。
- 替换旧字段校对 Markdown，让三个 source-only data_dict 由 contract_tools 生成。
- 不为 source-only contract 生成 dbt source、stg model 或 ClickHouse raw sync spec。

完成标准：

- `fleur-contracts generate --check` 能稳定生成 source-only data_dict。
- `pipeline/elt/models/sources.yml` 不出现 source-only asset 的 raw table entry。
- source-only contract 的 Parquet 类型与对应 scheduler `pa.Schema` 一致，或在债务记录中明确保留待迁移项。

### Phase 4：EastMoney 日期、nullable 和外源类型事实修正

范围：

- `pipeline/contracts/datasets/eastmoney__dividend_allotment.yml`
- `pipeline/contracts/datasets/eastmoney__dividend_main.yml`
- `pipeline/contracts/datasets/eastmoney__balance.yml`
- `pipeline/contracts/datasets/eastmoney__income_sq.yml`
- `pipeline/contracts/datasets/eastmoney__income_ytd.yml`
- `pipeline/contracts/datasets/eastmoney__cashflow_sq.yml`
- `pipeline/contracts/datasets/eastmoney__cashflow_ytd.yml`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`
- EastMoney schema 生成源或生成脚本

动作：

- `eastmoney__dividend_allotment.EX_DIVIDEND_DATEE` 从 Parquet 起收敛为 `date32[day]` / ClickHouse `Date`，除非远端证明存在非零时间且业务需要保留时刻。
- 同步更新 EastMoney Dagster Parquet schema 生成链路，使 `EASTMONEY_DIVIDEND_ALLOTMENT_SCHEMA` 中 `EX_DIVIDEND_DATEE` 不再保持 `pa.string()`。
- `eastmoney__dividend_main` 修正 `EQUITY_RECORD_DATE`、`EX_DIVIDEND_DATE`、`PAY_CASH_DATE`、`GMDECISION_NOTICE_DATE`、`LAST_TRADE_DATE` 的 source 类型和 nullable 事实。
- 评估并修正 `DAT_YAGGR`、`REPORT_TIME`、`ASSIGN_OBJECT`、`INFO_CODE` 的 nullable 或日期语义，不让 contract 声称外源必填但真实响应可能为 `null`。
- 如果 `REPORT_TIME` 收敛为日期类型，同步更新 `EASTMONEY_DIVIDEND_MAIN_SCHEMA` 中 `REPORT_TIME` 的 `pa.Schema`；如果继续保留 `pa.string()`，必须在 debt/validation note 中说明原因。
- `DAT_YAGGR`、`EQUITY_RECORD_DATE`、`EX_DIVIDEND_DATE`、`PAY_CASH_DATE`、`GMDECISION_NOTICE_DATE`、`LAST_TRADE_DATE` 当前 scheduler schema 已是 `pa.date32()`；本阶段重点是 source contract 外源类型、nullable/default 策略和测试，不应误改回字符串。
- 将财务报表中的 `OPINION_TYPE`、`OSOPINION_TYPE` source 外源类型从 `number` 改为 `string`，保留 raw `LowCardinality(String)`。
- `OPINION_TYPE`、`OSOPINION_TYPE` 当前 scheduler schema 已是 `pa.string()`，本阶段不需要改 Parquet 类型，只需要防止 contract/data_dict 继续显示外源 `number`。
- `INFO_CODE` 当前 scheduler schema 已是 `pa.string()`，高基数修复只改 ClickHouse raw contract 为普通 `String`，不改 Parquet schema。
- 同步修正 OpenAPI 和 remote endpoint 参考文档中与核验事实冲突的字段类型。

完成标准：

- EastMoney 债务涉及字段的 source、Parquet、ClickHouse raw contract 与 scheduler schema 一致。
- EastMoney generated schema 通过生成流程更新；变更说明中列出被改动的 `pa.field(...)`，并说明哪些债务未改变 Parquet schema。
- 日期字符串解析覆盖 `YYYY-MM-DD HH:MM:SS`、`null` 和空值策略。
- 不把 `OPINION_TYPE` / `OSOPINION_TYPE` 错误地改成普通 `String`。

### Phase 5：LowCardinality 债务按证据清偿

范围：

- `pipeline/contracts/datasets/jiuyan__action_field_compacted.yml`
- `pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml`
- `pipeline/contracts/datasets/jiuyan__industry_list.yml`
- `pipeline/contracts/datasets/ths__limit_up_pool_compacted.yml`
- `pipeline/contracts/datasets/eastmoney__dividend_main.yml`

动作：

- 对 `jiuyan__action_field_compacted.action_field_id`、`name`、`reason`、`expound` 运行 `count()`、`uniq()` 和重复率统计；高基数字段改为 `String`。
- 对 `jiuyan__industry_ocr_snapshot.industry_id`、`theme_path`、`relation` 运行同样统计；高基数字段改为 `String`。
- `jiuyan__industry_list.industry_id` 作为来源唯一标识，默认改为普通 `String`，除非统计证明长期低基数且理由充分。
- `ths__limit_up_pool_compacted.reason_type` 先统计再决策；没有高基数证据时保留债务记录而不强行修改。
- `eastmoney__dividend_main.INFO_CODE` 改为普通 `String`，因为样本已显示 500 条中 497 个唯一值。
- 本阶段字段的 Dagster Parquet schema 已经是普通 `pa.string()`；除非统计发现需要改变 Parquet 语义类型，否则不要修改 scheduler `pa.Schema`。
- 保留已核验仍适合 `LowCardinality(String)` 的低基数字段，例如市场代码、报告类型、币种和审计意见枚举。

完成标准：

- 每个被移除或保留 `LowCardinality(String)` 的字段都有统计依据或明确业务枚举依据。
- data_dict 的 validation notes 或债务关闭记录包含样本范围、统计结果和结论。
- 对每个 LowCardinality 债务明确记录“Parquet schema 不变，raw ClickHouse 类型变更”或对应例外。

### Phase 6：生成物、迁移窗口和最终关闭

范围：

- `pipeline/elt/models/sources.yml`
- `pipeline/elt/models/staging/staging.yml`
- `docs/references/data_dict/*.md`
- ClickHouse raw 表状态核验和迁移报告

动作：

- 每个 Phase 完成后运行 `fleur-contracts generate`，不手工编辑生成物。
- 对 raw 类型发生变化的表运行 ClickHouse schema 校验，记录现有表是否需要 `ALTER`、重建或回填。
- 如果现有 ClickHouse 表无法在线变更，创建对应 job report，标明等待下一次 raw 重建或回填窗口处理。
- 债务全部清偿后，把 `docs/debt/archive/0001-2026-06-01-field-type-normalization.md` 更新为关闭状态或保留仅含外部阻塞项。

完成标准：

- 生成物无漂移。
- raw 表类型变更不只停留在 contract；要么已迁移验证，要么有明确运行报告记录未完成原因和回填窗口。

## 6. 分批验证命令

文档-only 或计划更新：

```bash
git diff --check
```

contract-only 批次：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
git diff --check
```

改动 `contract_tools` Python 代码：

```bash
cd pipeline
uv run ruff check contract_tools/src contract_tools/tests
uv run ruff format --check contract_tools/src contract_tools/tests
uv run pyright contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest contract_tools/tests -q
```

改动 scheduler schema 或 source 转换：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests -q
```

改动 dbt staging：

```bash
cd pipeline
uv run dbt compile --project-dir elt --profiles-dir elt --select staging.*
```

涉及已存在 ClickHouse raw 表：

```bash
cd pipeline
uv run fleur-contracts validate-clickhouse --all-available
```

如果改动了本地可用 Parquet schema：

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
```

## 7. 禁止模式

- 禁止跳过 contract 直接改 generated data_dict、dbt YAML 或 ClickHouse spec。
- 禁止把 source-only asset 伪装成空 ClickHouse raw table。
- 禁止以为 `fleur-contracts generate` 会自动修改 Dagster source Parquet schema；Parquet 类型变更必须同步修改 scheduler 中对应 `pa.Schema` 或先实现明确的 schema 生成器。
- 禁止只因为字段名包含 `type`、`status`、`id` 就判断是否低基数；必须有统计或稳定枚举依据。
- 禁止把外源真实 `null` 静默转成 `0`、空字符串或 `1970-01-01`，除非字段 contract 明确写出默认值策略和原因。
- 禁止只手工改 `generated/schemas.py`；EastMoney 生成文件必须同步更新生成源或生成流程。
- 禁止一次性扩大到债务文件未列出的全仓类型重构。

## 8. 总体验收标准

计划完成时应满足：

- `docs/debt/archive/0001-2026-06-01-field-type-normalization.md` 的 13 项债务都有关闭记录、迁移报告或明确暂缓原因。
- `uv run fleur-contracts validate`、`uv run fleur-contracts generate --check` 和 `uv run pytest contract_tools/tests -q` 通过。
- 涉及 scheduler/dbt 的批次通过对应最小测试或 compile。
- 涉及 ClickHouse raw 类型变更的 dataset 已通过 `validate-clickhouse --all-available`，或在 job report 中记录无法验证的连接、表不存在或等待回填窗口原因。
- generated `docs/references/data_dict/*.md`、`pipeline/elt/models/sources.yml`、`pipeline/elt/models/staging/staging.yml` 与 contract 一致。
- `git diff --check` 通过。

## 9. 建议执行顺序

优先顺序：

1. Phase 1：先补 source-only 和类型证据表达能力，否则 Phase 3 和 LowCardinality 记录会继续绕过 contract schema。
2. Phase 2：BaoStock 范围小但会触碰 source、stg、glossary，适合作为类型链路收敛样板。
3. Phase 3：把 source-only asset 收进 registry，关闭旧 Markdown 字段事实分裂。
4. Phase 5 中已证据充分的字段：`jiuyan__industry_list.industry_id` 和 `eastmoney__dividend_main.INFO_CODE` 可先做小批次清偿。
5. Phase 4：EastMoney 日期和 nullable 字段涉及生成 schema 与远端核验，单独批次处理。
6. Phase 5 剩余字段：依赖 ClickHouse 或 Parquet 统计结果后再决策。
7. Phase 6：统一补迁移报告、生成物一致性和债务关闭记录。
