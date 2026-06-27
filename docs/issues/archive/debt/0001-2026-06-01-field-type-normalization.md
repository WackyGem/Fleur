# Debt: BaoStock 字段类型与命名语义收敛

日期：2026-06-01

## 执行状态

状态更新：2026-06-01。Plan 0020 的 contract、scheduler schema、dbt staging 和生成物修复已落地；已发布的历史 S3 Parquet 对象和 ClickHouse raw 表状态单独记录在 `docs/jobs/reports/2026-06-01-field-type-normalization-migration-report.md`。

| # | 债务 | 状态 | 证据 |
|---|------|------|------|
| 1 | `baostock__query_history_k_data_plus_daily.isST` 从 Parquet 起收敛为布尔 | 代码和 contract 已关闭；历史存储待回填 | `K_HISTORY_DAILY_SCHEMA` 使用 `pa.bool_()`，contract Parquet 为 `bool`、raw 为 `Bool`、stg 为 `is_st Bool`；当前 S3/ClickHouse 仍有旧 `int8`/`Int8`，见迁移报告。 |
| 2 | `baostock__query_history_k_data_plus_daily.tradestatus` 在 stg 表达交易状态 | 已关闭 | stg 输出 `trading_status`，SQL 映射 `1 -> trading`、`0 -> suspended`，Parquet/raw 保留供应商 `int8`/`Int8`。 |
| 3 | `baostock__query_stock_basic.status` 在 stg 表达上市状态 | 已关闭 | stg 输出 `stock_status`，SQL 映射 `1 -> listed`、`0 -> delisted`，Parquet/raw 保留供应商 `int8`/`Int8`。 |
| 4 | BaoStock stg 证券标识字段 canonical 命名 | 已关闭 | stg 和 contract 使用 `security_code`、`security_name`，dbt YAML 由 contract 生成。 |
| 5 | source-only asset 纳入 contract registry | 已关闭 | 新增 `jiuyan__action_field.yml`、`jiuyan__industry_ocr.yml`、`ths__limit_up_pool.yml`；data_dict 显示 ClickHouse raw 不适用，dbt sources 不生成 source-only raw entry。 |
| 6 | `eastmoney__dividend_allotment.EX_DIVIDEND_DATEE` 日期类型收敛 | 代码和 contract 已关闭；历史存储待回填 | contract Parquet 为 `date32[day]`、raw 为 `Date`，EastMoney generated schema 由 contract 生成；当前 sampled S3 object 仍为 `string`，raw table 缺失，见迁移报告。 |
| 7 | `eastmoney__dividend_main` 日期字段外源类型和 nullable 事实 | 已关闭 | 日期字段 source 改为 `string` 且可空字段 `required: false`，Parquet/raw nullable 同步；`REPORT_TIME` 已收敛为 Parquet `date32[day]` 和 ClickHouse `Nullable(Date)`，历史非日期标签在 source-to-Parquet 转换中置为 NULL。 |
| 8 | `jiuyan__action_field_compacted` LowCardinality 和中文口径 | 已关闭 | S3 parquet 统计写入 validation notes；`expound` 改为 raw `String`，其余低基数字段保留；字段中文口径统一为“题材异动”。 |
| 9 | `jiuyan__industry_ocr_snapshot` 标识、路径和关系字段 LowCardinality | 已关闭 | S3 parquet 统计写入 validation notes；`relation` 改为 raw `String`，`industry_id` 和 `theme_path` 依据样本低基数保留 `LowCardinality(String)`。 |
| 10 | `ths__limit_up_pool_compacted.reason_type` LowCardinality | 已关闭 | S3 parquet 统计 `uniq=11573`，超过 10000 阈值，raw 改为 `String`。 |
| 11 | `jiuyan__industry_list.industry_id` LowCardinality | 已关闭 | S3 snapshot 统计唯一率 1.0，raw 改为 `String`。 |
| 12 | `eastmoney__dividend_main.INFO_CODE` LowCardinality | 已关闭 | 2025 公告样本 `nonnull=500 uniq=497`，raw 改为 `String`，Parquet 保持 nullable `string`。 |
| 13 | EastMoney 审计意见字段外源类型 | 已关闭 | `OPINION_TYPE` / `OSOPINION_TYPE` source 类型修正为 `string`，Parquet 保持 nullable `string`，raw 保留 `LowCardinality(String)`。 |

剩余工作只限已发布存储对象和 raw 表迁移窗口，不再作为字段事实源债务继续展开。下一次回填后需要重新运行：

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

## 背景

BaoStock 数据进入 stg 层后，应从外源字段命名和编码值逐步收敛为 mono-fleur 的 canonical 字段语义。当前仍有两类债务：

- 部分字段在外源响应中以 `"0"` / `"1"` 字符串承载，但字段语义不是普通整数。
- 部分 stg 字段仍沿用外源泛化命名，例如用 `code` 表示证券代码。

相关位置：

- `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml`
- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`
- `pipeline/elt/models/staging/stg_baostock__query_history_k_data_plus_daily.sql`
- `docs/references/data_dict/baostock__query_history_k_data_plus_daily.md`

Per ClickHouse `schema-types-native-types`，布尔值应使用原生 `Bool` 或明确语义类型，而不是长期作为普通整数。Per `schema-types-minimize-bitwidth`，`0/1` 标记不需要 `Int8` 的有符号整数语义。若字段是状态而非纯布尔，stg 层应使用更清楚的语义字段或枚举值。

## 债务清单

### 1. `isST` 从 Parquet 层开始收敛为布尔语义

外源字段 `isST` 表示证券是否处于 ST 或风险警示状态。外源返回 `"1"` 表示 ST 或风险警示，`"0"` 表示非 ST。

当前类型链路：

| 层 | 当前类型 |
|----|----------|
| source payload | `string` |
| Parquet | `int8` |
| ClickHouse raw | `Int8` |
| dbt staging `is_st` | `Int8` |

建议目标：

| 层 | 目标类型 |
|----|----------|
| source payload | `string` |
| Parquet | `bool` |
| ClickHouse raw | `Bool` |
| dbt staging `is_st` | `Bool` |

修复要点：

- 保留 source contract 的 `type: string`，因为这是外源真实返回。
- `K_HISTORY_DAILY_SCHEMA` 中 `pa.field("isST", pa.int8())` 改为布尔类型。
- BaoStock 响应转 Arrow 时把 `"0"` / `"1"` 显式转换为 `False` / `True`。
- contract 中 Parquet、ClickHouse raw、dbt staging 类型同步更新。
- 已存在 ClickHouse raw 表如果仍是 `Int8`，需要决定迁移列类型、重建 raw 表，或在下一次 raw 重建/回填窗口处理。

### 2. `tradestatus` 在 stg 层表达交易状态语义

外源字段 `tradestatus` 表示是否正常交易。当前已知外源返回 `"1"` 表示正常交易，`"0"` 表示停牌。

当前类型链路：

| 层 | 当前类型 |
|----|----------|
| source payload | `string` |
| Parquet | `int8` |
| ClickHouse raw | `Int8` |
| dbt staging `tradestatus` | `Int8` |

建议目标：

| 层 | 目标类型 |
|----|----------|
| source payload | `string` |
| Parquet | `int8` |
| ClickHouse raw | `Int8` |
| dbt staging | 语义字段 + 枚举 |

`tradestatus` 是状态码，不是纯布尔标志。Parquet 层继续保留供应商 `0/1` 状态码更利于追溯；语义化应放在 stg 层完成。

stg 层建议：

- 保留或派生一个语义化字段，例如 `trading_status`。
- 输出枚举语义值，例如 `trading` / `suspended`，其中 `1 -> trading`，`0 -> suspended`。
- 如仍需要布尔查询便利性，可额外派生 `is_trading Bool`，但不要只暴露无语义的 `tradestatus Int8`。

修复要点：

- 保留 source contract 的 `type: string`。
- Parquet schema 和 ClickHouse raw 层继续保持 `int8` / `Int8`。
- dbt staging contract 和 SQL 同步改成语义字段和枚举输出。
- 重新生成 dbt YAML 和 data_dict。

### 3. `stock_basic.status` 在 stg 层表达上市状态语义

外源字段 `status` 来自 `baostock__query_stock_basic`，表示证券上市状态。当前已知外源返回 `"1"` 表示上市，`"0"` 表示退市。

当前类型链路：

| 层 | 当前类型 |
|----|----------|
| source payload | `string` |
| Parquet | `int8` |
| ClickHouse raw | `Int8` |
| dbt staging `stock_status` | `Int8` |

建议目标：

| 层 | 目标类型 |
|----|----------|
| source payload | `string` |
| Parquet | `int8` |
| ClickHouse raw | `Int8` |
| dbt staging | 语义字段 + 枚举 |

`status` 与 `tradestatus` 类似，是供应商状态码。Parquet 层保留 `int8` 合理，stg 层应输出语义化枚举值。

stg 层建议：

- 将 `stock_status` 输出为枚举语义值，例如 `listed` / `delisted`，其中 `1 -> listed`，`0 -> delisted`。
- 如需要布尔查询便利性，可额外派生 `is_listed Bool`。
- 字段说明应明确外源编码和值含义，避免下游继续记忆 `0/1`。

修复要点：

- 保留 source contract 的 `type: string`。
- Parquet schema 和 ClickHouse raw 层继续保持 `int8` / `Int8`。
- dbt staging contract 和 SQL 同步改成语义字段和枚举输出。
- 重新生成 dbt YAML 和 data_dict。

### 4. stg 层证券标识字段统一为 canonical 命名

当前部分 stg 模型仍使用外源或泛化字段名表示证券标识，例如：

- `stg_baostock__query_stock_basic`
- `stg_baostock__query_history_k_data_plus_daily`

对应 contract 中也仍使用：

- `dbt_staging.fields[].name: code`
- `dbt_staging.fields[].name: code_name`
- `glossary_key: code`
- `glossary_key: code_name`
- `pipeline/contracts/glossary/fields.yml` 中的 `code` / `code_name`

`code` 在不同数据源中可能表示证券代码、行业代码、供应商内部对象编码或其他业务编码；`code_name` 也不如 `security_name` 明确。stg 层应使用更明确的 canonical 字段名。

建议目标：

| 层 | 当前 | 目标 |
|----|------|------|
| source payload | `code` / `code_name` 或供应商原字段 | 保持外源原名 |
| ClickHouse raw | `code` / `code_name` 或供应商原字段 | 保持 raw 原名 |
| dbt staging | `code` | `security_code` |
| dbt staging | `code_name` | `security_name` |
| glossary | `code` / `code_name` | `security_code` / `security_name` |

修复要点：

- 更新 `pipeline/contracts/datasets/baostock__query_stock_basic.yml`。
- 更新 `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml`。
- 更新 `pipeline/contracts/glossary/fields.yml`，新增或迁移 `security_code` 和 `security_name` glossary。
- 更新 `pipeline/elt/models/staging/stg_baostock__query_stock_basic.sql`。
- 更新 `pipeline/elt/models/staging/stg_baostock__query_history_k_data_plus_daily.sql`。
- contract 中 `dbt_staging.primary_key` 如包含 `code`，同步改为 `security_code`。
- 重新生成 dbt YAML 和 data_dict。
- 下游引用 `stg_baostock__*`.`code` 或 `code_name` 的模型、分析 SQL 或测试需要同步改名。

### 5. source-only 资产也应纳入 contract registry

当前以下 data_dict 文档不是由 `fleur-contracts generate` 生成，而是旧字段校对脚本产物：

- `docs/references/data_dict/jiuyan__action_field.md`
- `docs/references/data_dict/jiuyan__industry_ocr.md`
- `docs/references/data_dict/ths__limit_up_pool.md`

对应资源虽然不直接写入 ClickHouse raw，但它们是正式 Dagster source asset，并且是 compacted 年度资产或 snapshot 资产的输入：

| source-only asset | downstream asset | ClickHouse raw |
|-------------------|------------------|----------------|
| `source/jiuyan__action_field` | `source/jiuyan__action_field_compacted` | `raw.jiuyan__action_field_compacted` |
| `source/jiuyan__industry_ocr` | `source/jiuyan__industry_ocr_snapshot` | `raw.jiuyan__industry_ocr_snapshot` |
| `source/ths__limit_up_pool` | `source/ths__limit_up_pool_compacted` | `raw.ths__limit_up_pool_compacted` |

当前只存在 downstream contract：

- `pipeline/contracts/datasets/jiuyan__action_field_compacted.yml`
- `pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml`
- `pipeline/contracts/datasets/ths__limit_up_pool_compacted.yml`

这会造成字段事实源分裂：原始 source asset 的外源字段、未使用字段、嵌套字段展开、Parquet 类型和 downstream 输入字段仍停留在旧 Markdown 中，不能被 contract 工具统一校验和生成。按照当前项目约定，`docs/references/data_dict/*.md` 是生成物，不应承载独立字段事实。

建议目标：

| 层 | 目标 |
|----|------|
| source contract | 新增 `jiuyan__action_field.yml`、`jiuyan__industry_ocr.yml` 和 `ths__limit_up_pool.yml` |
| ClickHouse raw | 明确标记为不适用，不生成 raw sync spec，不记录 ClickHouse 字段类型 |
| dbt source/staging | 不为 source-only asset 生成 dbt source 或 stg 模型 |
| data_dict | 由 contract 生成 source/parquet-only 数据字典，不展示 ClickHouse 类型列 |
| lineage | downstream contract 显式记录上游 source-only asset |

修复要点：

- 扩展 `pipeline/contract_tools/src/fleur_contracts/schema.py`，支持 source/parquet-only dataset contract。
- 不要用假的 `clickhouse_raw` 字段伪装 source-only 资产；应支持 `clickhouse_raw: null` 或等价的 `raw_sync.status: not_applicable` 语义。
- `raw_asset_key` 只应对实际 ClickHouse raw sync 数据集强制存在；source-only 数据集不应强制 `["clickhouse", "raw", dataset]`。
- 更新 data_dict adapter，使没有 ClickHouse raw 的 contract 也能生成文档；这类 source-only 文档不应记录或展示 ClickHouse 类型，ClickHouse/raw/dbt 列应省略或明确显示不适用。
- 更新 dbt YAML adapter，只为存在 active ClickHouse raw 的 contract 生成 `sources.yml` entry。
- 新增 source-only contracts：
  - `pipeline/contracts/datasets/jiuyan__action_field.yml`
  - `pipeline/contracts/datasets/jiuyan__industry_ocr.yml`
  - `pipeline/contracts/datasets/ths__limit_up_pool.yml`
- 在 downstream contracts 中记录上游 lineage，例如 `source/jiuyan__action_field`、`source/jiuyan__industry_ocr` 和 `source/ths__limit_up_pool`。
- 删除或替换旧字段校对 Markdown，确保 `docs/references/data_dict/jiuyan__action_field.md`、`docs/references/data_dict/jiuyan__industry_ocr.md` 和 `docs/references/data_dict/ths__limit_up_pool.md` 由 contract_tool 生成。
- 重新运行 contract 校验、生成物一致性检查和 contract_tools 测试。

### 6. `eastmoney__dividend_allotment.EX_DIVIDEND_DATEE` 应收敛为日期类型

`eastmoney__dividend_allotment` 的 `EX_DIVIDEND_DATEE` 字段表示除权除息日。已观察到样例值为 `1993-03-29 00:00:00`，语义上是日期或日期时间，而不是分类文本。

当前类型链路：

| 层 | 当前类型 |
|----|----------|
| source payload | `string` |
| Parquet | `string` |
| ClickHouse raw | `LowCardinality(String)` |
| dbt staging | 未开始 |

相关位置：

- `pipeline/contracts/datasets/eastmoney__dividend_allotment.yml`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py`
- `docs/references/data_dict/eastmoney__dividend_allotment.md`

Per ClickHouse `schema-types-native-types`，可解析的日期或时间字段应使用 `Date`、`Date32`、`DateTime` 或 `DateTime64` 等原生类型，而不是长期保存为字符串。`schema-types-lowcardinality` 适合重复分类文本，不适合作为日期字段的主要建模方式。

建议目标：

| 层 | 目标类型 |
|----|----------|
| source payload | `string` |
| Parquet | `date32[day]` 或必要时 `timestamp[ns]` |
| ClickHouse raw | `Date` 或必要时 `DateTime64(3)` |
| dbt staging | 后续启用时使用语义化日期字段 |

优先判断口径：

- 如果时间部分恒为 `00:00:00`，该字段应按除权除息自然日处理，目标为 Parquet `date32[day]` 和 ClickHouse `Date`。
- 如果存在非零时间部分且业务需要保留时刻，目标为 Parquet `timestamp[ns]` 和 ClickHouse `DateTime64(3)`。

修复要点：

- 保留 source contract 的 `type: string`，因为这是外源真实返回形态。
- 更新 EastMoney schema 生成源或生成脚本，避免只手工改 `generated/schemas.py`。
- 将 `EX_DIVIDEND_DATEE` 的 Parquet、ClickHouse raw contract 类型改为日期或时间原生类型。
- 更新响应转 Arrow 的解析逻辑，明确处理 `YYYY-MM-DD HH:MM:SS` 样式。
- 如果已有 ClickHouse raw 表仍是 `LowCardinality(String)`，需要安排 raw 表 schema 迁移、重建或回填窗口。
- 重新生成 data_dict，并运行 contract 校验与 ClickHouse schema 校验。

### 7. `eastmoney__dividend_main` 日期字段外源类型和 nullable 事实不一致

`eastmoney__dividend_main` 的 contract 和 data_dict 中，多处字段的外源类型或 nullable 约束与真实 EastMoney 响应不一致。

核验样本：

- Endpoint：`https://datacenter.eastmoney.com/securities/api/data/v1/get`
- 参数：`reportName=RPT_F10_DIVIDEND_MAIN`、`columns=ALL`、`source=HSF10`、`client=PC`
- 样本 1：`filter=(SECUCODE="601088.SH")`，返回 38 条。
- 样本 2：`filter=(NOTICE_DATE>='2025-01-01')(NOTICE_DATE<='2025-12-31')`，第一页 100 条，总记录数 23579。

明确不符字段：

| 字段 | contract/data_dict 当前外源类型 | curl 实际类型和值形态 | 问题 |
|------|---------------------------------|-----------------------|------|
| `EQUITY_RECORD_DATE` | `number` | `null|string`，如 `2025-11-07 00:00:00` | 外源类型错误，且实际 nullable |
| `EX_DIVIDEND_DATE` | `number` | `null|string`，如 `2025-11-10 00:00:00` | 外源类型错误，且实际 nullable |
| `PAY_CASH_DATE` | `number` | `null|string`，如 `2025-11-10 00:00:00` | 外源类型错误，且实际 nullable |
| `GMDECISION_NOTICE_DATE` | `number` | `null|string`，如 `2025-10-25 00:00:00` | 外源类型错误，且实际 nullable |
| `LAST_TRADE_DATE` | `number` | `null|string`，如 `2026-01-06 00:00:00` | 外源类型错误，且实际 nullable |

其他 nullable 或语义类型债务：

| 字段 | contract/data_dict 当前状态 | curl 实际类型和值形态 | 问题 |
|------|-----------------------------|-----------------------|------|
| `DAT_YAGGR` | 外源 `string`，Parquet/ClickHouse `date32[day]` / `Date`，非空 | `null|string`，如 `2026-03-31 00:00:00` | 实际 nullable |
| `REPORT_TIME` | Parquet/ClickHouse `string` / `String`，非空 | `null|string`，如 `2025-12-31 00:00:00` | 语义为报告期截止日，应评估日期类型，且实际 nullable |
| `ASSIGN_OBJECT` | 外源 `string`，非空 | `null|string` | 实际 nullable |
| `INFO_CODE` | 外源 `string`，非空 | `null|string` | 实际 nullable |

建议目标：

| 字段组 | source contract | Parquet | ClickHouse raw |
|--------|-----------------|---------|----------------|
| 日期字符串字段 | `type: string`，允许 nullable 或记录实际可空 | `date32[day]`，nullable 或默认值策略明确 | `Date`，nullable 或默认值策略明确 |
| `REPORT_TIME` | `type: string`，允许 nullable 或记录实际可空 | 优先评估 `date32[day]` | 优先评估 `Date` |
| 普通 nullable 文本 | `type: string`，允许 nullable 或记录实际可空 | `string`，nullable 或默认值策略明确 | `String` / `LowCardinality(String)`，nullable 或默认值策略明确 |

Per ClickHouse `schema-types-native-types`，可解析的日期字段不应长期保存为字符串或伪 `number`。如果选择保留非空 ClickHouse 字段，则必须在 source 转换层定义明确默认值策略；不能让 contract 声称外源必填但真实响应为 `null`。

修复要点：

- 更新 `pipeline/contracts/datasets/eastmoney__dividend_main.yml` 的 source 外源类型和 nullable 事实表达。
- 更新 EastMoney schema 生成源或生成脚本，避免只手工改 `generated/schemas.py`。
- 更新 `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/generated/schemas.py` 对应字段类型和 nullable 策略。
- 更新响应转 Arrow 逻辑，确保 `YYYY-MM-DD HH:MM:SS` 日期字符串和 `null` 能稳定转换。
- 如果 ClickHouse raw 表已存在，需要安排 schema 迁移、重建或回填窗口。
- 重新生成 `docs/references/data_dict/eastmoney__dividend_main.md`，并运行 contract 校验、生成物一致性检查和 ClickHouse schema 校验。

### 8. `jiuyan__action_field_compacted` 标识、文本字段类型和中文口径需修正

`jiuyan__action_field_compacted` 当前有来源唯一标识、名称和两个长文本语义字段被建模为 `LowCardinality(String)`：

| 字段 | 当前 ClickHouse 类型 | 当前中文描述 |
|------|----------------------|--------------|
| `action_field_id` | `LowCardinality(String)` | 韭研行动领域记录唯一标识。 |
| `name` | `LowCardinality(String)` | 行动领域名称。 |
| `reason` | `LowCardinality(String)` | 行动领域形成或归类原因。 |
| `expound` | `LowCardinality(String)` | 行动领域补充说明。 |

`action_field_id` 是来源系统唯一标识，`name` 是题材异动名称，`reason` 和 `expound` 是题材异动原因和补充说明，文本内容通常接近逐事件描述，预计重复度不高。Per ClickHouse `schema-types-lowcardinality`，`LowCardinality(String)` 适合低基数重复字符串，建议以真实数据 `uniq(field)` 校验；如果 unique values 超过 10,000 或接近行数，应改为普通 `String`。

另外，`jiuyan__action_field_compacted` 的中文业务口径不应翻译成“行动领域表”。该数据集在项目中更贴近“韭研题材异动表”或“题材异动每日数据”，字段描述中的“行动领域”也容易误导下游理解。

建议目标：

| 项 | 当前 | 目标 |
|----|------|------|
| `action_field_id` ClickHouse raw 类型 | `LowCardinality(String)` | 若真实基数较高，改为 `String` |
| `name` ClickHouse raw 类型 | `LowCardinality(String)` | 若真实基数较高，改为 `String` |
| `reason` ClickHouse raw 类型 | `LowCardinality(String)` | 若真实基数较高，改为 `String` |
| `expound` ClickHouse raw 类型 | `LowCardinality(String)` | 若真实基数较高，改为 `String` |
| 数据集中文口径 | 行动领域 | 题材异动 |
| 字段中文描述 | 行动领域唯一标识、行动领域名称、行动领域原因、行动领域补充说明 | 题材异动唯一标识、题材异动名称、题材异动原因、题材异动补充说明 |

修复要点：

- 用 ClickHouse 或 Parquet 样本统计 `action_field_id`、`name`、`reason`、`expound` 的 `count()`、`uniq()` 和重复率，记录判断依据。
- 更新 `pipeline/contracts/datasets/jiuyan__action_field_compacted.yml` 中 `action_field_id`、`name`、`reason`、`expound` 的 `clickhouse_raw.fields[].type` 和 `reason` 字段。
- 将 contract 中相关中文描述从“行动领域”修正为“题材异动”，尤其是 `reason`、`expound`、`action_field_id`、`name` 等字段。
- 检查 `pipeline/contracts/glossary/tables.yml` 和 generated data_dict，确保数据集中文说明统一为“韭研题材异动”。
- 如果 ClickHouse raw 表已存在，需要安排列类型迁移、重建或回填窗口。
- 重新生成 `docs/references/data_dict/jiuyan__action_field_compacted.md`，并运行 contract 校验和 ClickHouse schema 校验。

### 9. `jiuyan__industry_ocr_snapshot` 标识、路径和关系说明字段不应使用 LowCardinality

`jiuyan__industry_ocr_snapshot` 当前有三个文本字段被建模为 `LowCardinality(String)`：

| 字段 | 外源类型 | 当前 Parquet 类型 | 当前 ClickHouse 类型 | stg 字段 | 当前中文描述 |
|------|----------|-------------------|----------------------|----------|--------------|
| `industry_id` | `N/A` | `string` | `LowCardinality(String)` | `industry_id` | 行业研究记录在来源系统中的唯一标识。 |
| `theme_path` | `array` | `string` | `LowCardinality(String)` | `theme_path` | 题材或主题在来源系统中的层级路径。 |
| `relation` | `N/A` | `string` | `LowCardinality(String)` | `relation` | 记录之间或主题之间的关联说明。 |

`industry_id` 是来源系统唯一标识，`theme_path` 是来源系统层级路径压平后的文本，`relation` 是记录或主题之间的关系说明。这三类字段都可能随主题层级、OCR 内容和人工整理结果产生较多唯一值，不应默认视为低基数字典字段。Per ClickHouse `schema-types-lowcardinality`，`LowCardinality(String)` 应用于低重复枚举或分类文本；若真实 unique values 超过 10,000 或接近行数，应使用普通 `String`。

建议目标：

| 字段 | 当前 ClickHouse 类型 | 目标 ClickHouse 类型 |
|------|----------------------|----------------------|
| `industry_id` | `LowCardinality(String)` | `String` |
| `theme_path` | `LowCardinality(String)` | `String` |
| `relation` | `LowCardinality(String)` | `String` |

修复要点：

- 用 ClickHouse 或 Parquet 样本统计 `industry_id`、`theme_path`、`relation` 的 `count()`、`uniq()` 和重复率，记录判断依据。
- 更新 `pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml` 中 `industry_id`、`theme_path`、`relation` 的 `clickhouse_raw.fields[].type`，移除不适用的 LowCardinality `reason`。
- 检查 `dbt_staging` 字段说明和 glossary，确保 `industry_id`、`theme_path`、`relation` 的 canonical 描述仍准确。
- 如果 ClickHouse raw 表已存在，需要安排列类型迁移、重建或回填窗口。
- 重新生成 `docs/references/data_dict/jiuyan__industry_ocr_snapshot.md`、dbt YAML，并运行 contract 校验和 ClickHouse schema 校验。

### 10. `ths__limit_up_pool_compacted.reason_type` 不应默认使用 LowCardinality

`ths__limit_up_pool_compacted` 当前将 `reason_type` 建模为 `LowCardinality(String)`：

| 字段 | 外源类型 | 当前 Parquet 类型 | 当前 ClickHouse 类型 | 当前中文描述 |
|------|----------|-------------------|----------------------|--------------|
| `reason_type` | `string` | `string` | `LowCardinality(String)` | 涨停原因类型。 |

`reason_type` 表示涨停原因类型或原因文本分类，实际值可能随题材、市场热点和供应商口径变化而产生较多唯一值，不应只因为字段名包含 `type` 就默认使用低基数字典类型。Per ClickHouse `schema-types-lowcardinality`，`LowCardinality(String)` 应用于低重复枚举或分类文本；若真实 unique values 超过 10,000 或接近行数，应使用普通 `String`。

建议目标：

| 字段 | 当前 ClickHouse 类型 | 目标 ClickHouse 类型 |
|------|----------------------|----------------------|
| `reason_type` | `LowCardinality(String)` | `String` |

修复要点：

- 用 ClickHouse 或 Parquet 样本统计 `reason_type` 的 `count()`、`uniq()` 和重复率，记录判断依据。
- 更新 `pipeline/contracts/datasets/ths__limit_up_pool_compacted.yml` 中 `reason_type` 的 `clickhouse_raw.fields[].type`，移除不适用的 LowCardinality `reason`。
- 如果 ClickHouse raw 表已存在，需要安排列类型迁移、重建或回填窗口。
- 重新生成 `docs/references/data_dict/ths__limit_up_pool_compacted.md`，并运行 contract 校验和 ClickHouse schema 校验。

### 11. `jiuyan__industry_list.industry_id` 唯一标识不应默认使用 LowCardinality

`jiuyan__industry_list` 当前将 `industry_id` 建模为 `LowCardinality(String)`：

| 字段 | 外源类型 | 当前 Parquet 类型 | 当前 ClickHouse 类型 | stg 字段 | 当前中文描述 |
|------|----------|-------------------|----------------------|----------|--------------|
| `industry_id` | `string` | `string` | `LowCardinality(String)` | `industry_id` | 行业研究记录在来源系统中的唯一标识。 |

`industry_id` 是来源系统唯一标识，不是低基数分类字段。Per ClickHouse `schema-types-lowcardinality`，`LowCardinality(String)` 应用于低重复枚举或分类文本；唯一标识字段通常应使用普通 `String`，除非真实数据证明其基数长期很低。

修复要点：

- 用 ClickHouse 或 Parquet 样本统计 `industry_id` 的 `count()`、`uniq()` 和重复率，记录判断依据。
- 更新 `pipeline/contracts/datasets/jiuyan__industry_list.yml` 中 `industry_id` 的 `clickhouse_raw.fields[].type`，移除不适用的 LowCardinality `reason`。
- 如果 ClickHouse raw 表已存在，需要安排列类型迁移、重建或回填窗口。
- 重新生成 `docs/references/data_dict/jiuyan__industry_list.md`、dbt YAML，并运行 contract 校验和 ClickHouse schema 校验。

### 12. EastMoney 公告编号字段不应默认使用 LowCardinality

curl 核验 `eastmoney__dividend_main` 2025 年公告日期第一页 500 条样本后，`INFO_CODE` 基本确认是高基数公告编号字段：

```text
INFO_CODE nonnull=500 uniq=497
```

该字段当前被建模为 `LowCardinality(String)`：

| 数据集 | 字段 | 当前 ClickHouse 类型 | 当前中文描述 |
|--------|------|----------------------|--------------|
| `eastmoney__dividend_main` | `INFO_CODE` | `LowCardinality(String)` | 公告编号 |

`INFO_CODE` 是公告编号，样本中唯一值几乎接近行数。Per ClickHouse `schema-types-lowcardinality`，`LowCardinality(String)` 应用于低重复枚举或分类文本；这类高基数字段应使用普通 `String`。

另外，以下说明文本字段经第一页样本核验未直接证明高基数，不应在本债务中直接判定为错误；后续如有全量统计证据，再单独记录：

| 数据集 | 字段 | 样本结果 |
|--------|------|----------|
| `eastmoney__dividend_main` | `IMPL_PLAN_PROFILE` | 500 条样本 178 个唯一值 |
| `eastmoney__dividend_main` | `IMPL_PLAN_NEWPROFILE` | 500 条样本 180 个唯一值 |
| `eastmoney__dividend_main` | `NEW_PROFILE` | 500 条样本 178 个唯一值 |
| `eastmoney__dividend_allotment` | `EVENT_EXPLAIN` | 500 条样本 47 个唯一值 |
| `eastmoney__equity_history` | `CHANGE_REASON_EXPLAIN` | 500 条样本 42 个唯一值 |

修复要点：

- 更新 `pipeline/contracts/datasets/eastmoney__dividend_main.yml` 中 `INFO_CODE` 的 `clickhouse_raw.fields[].type` 为 `String`，移除不适用的 LowCardinality `reason`。
- 保留真正低基数枚举字段，例如 `ASSIGN_PROGRESS`、`REPORT_TYPE`、`CURRENCY`、市场代码等，避免过度修正。
- 如果 ClickHouse raw 表已存在，需要安排列类型迁移、重建或回填窗口。
- 重新生成 `docs/references/data_dict/eastmoney__dividend_main.md`，并运行 contract 校验和 ClickHouse schema 校验。

### 13. EastMoney 审计意见字段外源类型应修正为 string

多个 EastMoney 财务报表 data_dict 显示，审计意见类字段的外源类型为 `number`。curl 核验 `balance`、`income_sq`、`income_ytd`、`cashflow_sq`、`cashflow_ytd` 多股票 2020-2026 样本后，远端实际返回为 `null|string`，非 `number`：

```text
OPINION_TYPE types=null|string examples=["标准无保留意见"]
OSOPINION_TYPE types=null
```

因此这里的主要债务不是 raw string 链路错误，而是 source contract / data_dict 的外源类型事实错误。

| 数据集 | 字段 | 外源类型 | Parquet 类型 | ClickHouse 类型 |
|--------|------|----------|--------------|-----------------|
| `eastmoney__balance` | `OPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__balance` | `OSOPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__cashflow_sq` | `OPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__cashflow_sq` | `OSOPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__cashflow_ytd` | `OPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__cashflow_ytd` | `OSOPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__income_sq` | `OPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__income_sq` | `OSOPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |
| `eastmoney__income_ytd` | `OPINION_TYPE` | `number` | `string` | `LowCardinality(String)` |

审计意见字段是低基数文本枚举，Parquet `string` 和 ClickHouse `LowCardinality(String)` 可以保留；需要修正的是 `source.fields[].type` 和生成的数据字典外源类型。Per ClickHouse `schema-types-lowcardinality`，这类低基数字符串枚举适合继续使用 `LowCardinality(String)`。

修复要点：

- 将相关 dataset contract 中 `OPINION_TYPE`、`OSOPINION_TYPE` 的 source 外源类型从 `number` 修正为 `string`，并明确实际可能为 `null`。
- 同步更新 OpenAPI/remote endpoint 参考文档中不一致的字段类型。
- 重新生成对应 data_dict，并运行 contract 校验和生成物一致性检查。

## 通用迁移影响

这类修复不只是 contract 文档改动，还需要同步处理：

- BaoStock source schema 与响应转换逻辑。
- dataset contract 中 Parquet、ClickHouse raw、dbt staging 字段类型和说明。
- dbt staging SQL 的字段命名和类型转换。
- glossary 字段语义。
- generated dbt YAML 和 data_dict。
- 若改动 ClickHouse raw 类型，必须处理已有 raw 表 schema 与历史数据回填策略。

## 验收命令

```bash
cd pipeline

uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run dbt compile --project-dir elt --profiles-dir elt --select staging.*
uv run pytest contract_tools/tests scheduler/tests/unit/baostock -q
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
git diff --check
```

如果涉及已存在 ClickHouse raw 表，还需要运行：

```bash
cd pipeline
uv run fleur-contracts validate-clickhouse --all-available
```

## 后续追加规则

后续发现 BaoStock 其他字段存在类似“外源字符串编码，但下游应表达布尔、枚举或语义类型”或“stg 字段命名不符合 canonical 语义”的问题时，继续追加到本文件的“债务清单”，不要为每个字段新建零散债务文件。
