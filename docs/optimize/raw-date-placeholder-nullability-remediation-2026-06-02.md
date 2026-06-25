# Raw 日期占位值与 ClickHouse Nullable 修复计划

日期：2026-06-02

状态：Completed

完成报告：`docs/jobs/reports/2026-06-02-raw-date-placeholder-nullability-remediation.md`

## 1. 背景

`docs/references/raw_profile/` 的 raw source profiling 发现多张 ClickHouse raw 表存在
`1970-01-01` 或 `1970-01-01 00:00:00`，用于表达缺失日期、未发生日期或空更新时间。

已单独核验 `baostock__query_stock_basic.outDate`：

- S3 Parquet 中 `outDate` 是 nullable `date32[day]`，当前有 7,644 个 `NULL`。
- ClickHouse 以 `outDate Nullable(Date)` 读取同一个 Parquet 文件时，仍是 7,644 个 `NULL`。
- ClickHouse 以 `outDate Date` 加 `input_format_null_as_default = 1` 读取时，7,644 个 `NULL` 变成 `1970-01-01`。
- 当前 raw 表 `fleur_raw.baostock__query_stock_basic.outDate` 实际类型是 `Date`，不是 `Nullable(Date)`。

因此至少对 BaoStock `outDate`，问题不是 S3 Parquet 落盘错误，而是 ClickHouse raw 层 schema / sync 对 nullable 日期处理不正确。其他同类字段需要按本计划逐项核验，但当前 contract 已显示多个字段 `nullable: true` 却仍使用非 nullable ClickHouse 类型，例如 `Date` 或 `DateTime64(3)`。

## 2. 目标

1. 清除 raw 层中用 `1970-01-01` 表示缺失/未发生日期的错误表达。
2. 让 `pipeline/contracts/datasets/*.yml` 的 `nullable: true` 与 ClickHouse raw 实际类型一致。
3. 避免 `input_format_null_as_default = 1` 静默把 nullable Parquet 值写成 ClickHouse 默认日期。
4. 在 raw profile、contract 校验和 ClickHouse 校验中留下可重复验收方式。
5. 为后续 staging model 提供真实的 `NULL` 语义，避免未来开发时被迫反复修正 raw 层占位值。

## 3. 非目标

1. 不在 staging 层用 `nullIf(date_col, '1970-01-01')` 作为长期主要修复手段。
2. 不改变业务日期字段的口径，例如报告期、公告日、交易日等真实日期。
3. 不把跨源实体匹配、主数据修正或业务优先级去重放入本计划。
4. 不把 raw profile 写回 `pipeline/contracts`；contract 仍是 raw 字段事实源，profile 只记录观察结果。

## 4. 当前问题清单

以下字段来自 `docs/references/raw_profile/*` 的“数据质量问题”表，均明确记录了 `1970-01-01` 占位计数。

| 数据集 | 字段 | 当前 raw profile 证据 | 预期语义判断 | 初步整改方向 |
|--------|------|----------------------|--------------|--------------|
| `baostock__query_stock_basic` | `outDate` | 7,644 行 | 未退市证券的退市日期缺失 | ClickHouse raw 改为 `Nullable(Date)`，重刷 snapshot |
| `eastmoney__balance` | `UPDATE_DATE` | 976 行 | 更新日期缺失或供应商未提供 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__cashflow_sq` | `UPDATE_DATE` | 449 行 | 更新日期缺失或供应商未提供 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__cashflow_ytd` | `UPDATE_DATE` | 686 行 | 更新日期缺失或供应商未提供 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__income_sq` | `UPDATE_DATE` | 193 行 | 更新日期缺失或供应商未提供 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__income_ytd` | `UPDATE_DATE` | 523 行 | 更新日期缺失或供应商未提供 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__dividend_main` | `EQUITY_RECORD_DATE` | 95,808 行 | 分红流程节点尚未发生或未披露 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__dividend_main` | `EX_DIVIDEND_DATE` | 96,900 行 | 分红流程节点尚未发生或未披露 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__dividend_main` | `PAY_CASH_DATE` | 99,965 行 | 分红流程节点尚未发生或未披露 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__dividend_main` | `GMDECISION_NOTICE_DATE` | 70,793 行 | 股东大会决议公告日缺失或未发生 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__dividend_main` | `DAT_YAGGR` | 100,343 行 | 年度股东大会日期缺失或未发生 | 核验 S3 Parquet；如为 NULL，ClickHouse raw 改为 `Nullable(Date)` |
| `eastmoney__dividend_main` | `LAST_TRADE_DATE` | 151,606 行 | 最后交易日未提供；当前全表占位 | 核验字段是否应继续保留；如保留则 `Nullable(Date)` |
| `jiuyan__action_field_compacted` | `delete_time` | 5,853 行 | 未删除记录的删除时间缺失 | ClickHouse raw 改为 `Nullable(DateTime64(3))`，重刷 compacted raw |
| `jiuyan__action_field_compacted` | `update_time` | 5,853 行 | 更新时间缺失或源端未维护 | 核验源端与 S3；如为 NULL，ClickHouse raw 改为 `Nullable(DateTime64(3))` |
| `jiuyan__industry_list` | `delete_time` | 956 行 | 未删除记录的删除时间缺失 | ClickHouse raw 改为 `Nullable(DateTime64(3))`，重刷 snapshot |

## 5. 根因假设

### 5.1 已确认根因：ClickHouse raw nullable 未表达

`pipeline/contracts/datasets/baostock__query_stock_basic.yml` 中：

- `parquet.fields.outDate.nullable: true`
- `clickhouse_raw.fields.outDate.nullable: true`
- 但 `clickhouse_raw.fields.outDate.type: Date`

当前 ClickHouse adapter / scheduler raw sync 使用 `field.type` 生成表结构，没有把 `nullable: true` 自动包成 `Nullable(...)`。raw sync 插入时又使用：

```sql
SETTINGS input_format_null_as_default = 1
```

当目标列是非 nullable `Date` / `DateTime64` 时，Parquet NULL 会被 ClickHouse 默认值写成 `1970-01-01` 或 `1970-01-01 00:00:00`。

### 5.2 待核验根因：源端本身提供占位值

EastMoney 与 JiuYan 的同类字段需要逐项核验 S3 Parquet：

- 如果 Parquet 中是 `NULL`，则按 ClickHouse raw schema 问题处理。
- 如果 Parquet 中已经是 `1970-01-01`，则问题在 source conversion，应在 source typed-table 转换前把供应商占位日期转为 `None`。
- 如果源端确实把 `1970-01-01` 定义为业务值，必须在 raw profile 和 data dict 中记录例外；目前没有证据支持把它当作真实业务日期。

## 6. 整改范围

### 6.1 Contract 与 ClickHouse 类型生成

范围：

- `pipeline/contracts/datasets/*.yml`
- `pipeline/contract_tools/src/fleur_contracts/adapters/clickhouse.py`
- `pipeline/contract_tools/src/fleur_contracts/adapters/dbt.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/sql.py`
- 相关 contract / scheduler tests

整改项：

1. 明确 `clickhouse_raw.fields[].nullable` 的权威语义。
2. 选择一种一致表达方式：
   - 推荐：contract 中继续保留 `type: Date` 和 `nullable: true`，ClickHouse adapter 生成 `Nullable(Date)`。
   - 备选：contract 中显式写 `type: Nullable(Date)`，并让 validator 校验 `nullable: true` 与 `Nullable(...)` 一致。
3. 对所有 `nullable: true` 的 ClickHouse raw 字段生成 `Nullable(<type>)`，包括 `Date`、`DateTime64`、字符串和数值字段。
4. 增加 validator，禁止 `nullable: true` 却生成非 Nullable ClickHouse 类型。
5. 重新生成 `pipeline/elt/models/sources.yml` 和 `docs/references/data_dict/*.md`。

验收清单：

- [ ] `baostock__query_stock_basic.outDate` 生成的 scheduler spec 为 `Nullable(Date)`。
- [ ] EastMoney `UPDATE_DATE` 和 dividend nullable 日期字段生成 `Nullable(Date)`。
- [ ] JiuYan nullable 时间字段生成 `Nullable(DateTime64(3))` 或约定的 nullable DateTime 类型。
- [ ] `uv run fleur-contracts validate` 通过。
- [ ] `uv run fleur-contracts generate --check` 通过。
- [ ] `uv run pytest contract_tools/tests -q` 通过。
- [ ] `uv run pytest scheduler/tests/unit/test_contract_schemas.py -q` 通过。

### 6.2 Raw sync 插入策略

范围：

- `pipeline/scheduler/src/scheduler/defs/clickhouse/sql.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py`
- `pipeline/scheduler/tests/unit/clickhouse/*`

整改项：

1. 复核 `input_format_null_as_default = 1` 是否仍应全局启用。
2. 对 nullable 字段，确保 ClickHouse 读取 S3 Parquet 的 `structure` 是 `Nullable(...)`。
3. 增加单测覆盖：同一份含 NULL 的 Parquet，经生成 SQL 插入后仍为 NULL，不变成默认日期。
4. 如果保留 `input_format_null_as_default = 1`，必须有测试证明 nullable 日期字段不受影响。

验收清单：

- [ ] raw sync SQL 的 S3 structure 对 nullable 日期字段输出 `Nullable(Date)` / `Nullable(DateTime64(...))`。
- [ ] 单测覆盖 `Nullable(Date)` NULL 不被写成 `1970-01-01`。
- [ ] 单测覆盖非 nullable 字段仍按预期拒绝或默认处理，不误吞 source schema 错误。
- [ ] `uv run pytest scheduler/tests/unit/clickhouse -q` 通过。

### 6.3 存量 ClickHouse raw 表修复与重刷

范围：

- `fleur_raw.baostock__query_stock_basic`
- `fleur_raw.eastmoney__balance`
- `fleur_raw.eastmoney__cashflow_sq`
- `fleur_raw.eastmoney__cashflow_ytd`
- `fleur_raw.eastmoney__income_sq`
- `fleur_raw.eastmoney__income_ytd`
- `fleur_raw.eastmoney__dividend_main`
- `fleur_raw.jiuyan__action_field_compacted`
- `fleur_raw.jiuyan__industry_list`

整改项：

1. 对每个字段先做 S3 Parquet vs ClickHouse raw 对比。
2. 如果 S3 是 NULL、ClickHouse 是 `1970-01-01`，在修复 schema 后重新 raw sync。
3. 如果 S3 已经是 `1970-01-01`，回到 source conversion 修复后重跑 source asset，再 raw sync。
4. 对 snapshot 表执行 snapshot raw sync；对 compacted 表按对应 compacted raw 资产流程重刷。
5. 重刷后更新 raw profile 报告中的日期范围、NULL 数和数据质量问题。

验收清单：

- [ ] `baostock__query_stock_basic.outDate`: `countIf(isNull(outDate)) = 7644`，`countIf(outDate = toDate('1970-01-01')) = 0`。
- [ ] EastMoney affected fields: raw 层 `1970-01-01` 计数为 0，缺失值保留为 NULL；如源端确实提供占位，必须在 report 中标明源端占位。
- [ ] JiuYan affected fields: raw 层 `1970-01-01 00:00:00` 计数为 0，缺失值保留为 NULL。
- [ ] `uv run fleur-contracts validate-clickhouse --all-available` 通过。
- [ ] 相关 raw profile 全部更新为 `Accepted`，并删除“使用 `1970-01-01` 表示缺失/未发生日期”的未修复问题行。

### 6.4 Raw profile 与 profiling 脚本改进

范围：

- `pipeline/elt/scripts/profile_raw_source.py`
- `docs/references/raw_profile/*.md`

整改项：

1. profile script 应区分“通用注意事项”和“实际发现的问题”。
2. 只有 `countIf(date_col = '1970-01-01') > 0` 时，才在“数据质量问题”表中写入占位日期问题。
3. 对 nullable ClickHouse 字段同时记录 `countIf(isNull(col))` 和 `countIf(col = '1970-01-01')`。
4. 对 raw sync 修复后的字段，profile report 应显示 NULL 计数而不是占位日期计数。

验收清单：

- [ ] 重新执行 affected datasets 的 raw profile 后，没有误报未出现的 `1970-01-01` 问题。
- [ ] affected fields 修复后，profile report 明确记录 NULL 数。
- [ ] `git diff --check -- docs/references/raw_profile pipeline/elt/scripts/profile_raw_source.py` 通过。

## 7. 推荐执行阶段

本计划保留三阶段修复处理。三阶段只表示代码、contract、raw sync 和 profiling
能力已经修好；不能单独作为完成标准。真正完成必须再执行第 8 节的生产数据闭环：
受影响数据从 S3 Parquet 全部分区重刷到 ClickHouse raw，验证真实 raw 数据已经恢复
`NULL` 语义，再按 `fleur-dbt-model-readiness` 重新完成数据特征分析。

### Phase 0: 事实核验

目标：逐字段确认问题发生在 S3 Parquet、source conversion 还是 ClickHouse raw sync。

完成标准：

- [ ] 为第 4 节每个字段记录 S3 Parquet NULL 数、S3 Parquet `1970-01-01` 数、ClickHouse raw NULL 数、ClickHouse raw `1970-01-01` 数。
- [ ] 将核验结果写入 `docs/jobs/reports/` 或本优化文档的后续执行报告。

### Phase 1: Contract / adapter 修复

目标：让 contract nullable 与 ClickHouse 实际 schema 一致。

完成标准：

- [ ] affected contracts 或 adapter 修复完成。
- [ ] 生成物同步完成。
- [ ] contract 和 scheduler schema boundary tests 通过。

### Phase 2: Raw sync 插入与 schema 修复

目标：让 ClickHouse raw sync 对 S3 Parquet nullable 字段使用 `Nullable(...)`
structure，并停止把 nullable 日期 / 时间写成默认占位值。

完成标准：

- [ ] affected raw tables 重新创建后的实际 schema 为 `Nullable(...)`。
- [ ] raw sync SQL 的 S3 table function structure 对 nullable 日期字段输出 `Nullable(Date)` / `Nullable(DateTime64(...))`。
- [ ] raw sync 单测证明 Parquet NULL 插入后仍为 ClickHouse NULL。
- [ ] 如仍保留 `input_format_null_as_default = 1`，测试必须证明 nullable 日期字段不受影响。

### Phase 3: Raw profile 与 profiling 脚本修复

目标：让 raw profile 能区分通用提醒和实际发现的问题，并正确展示修复后的
NULL / 占位值计数。

完成标准：

- [ ] `profile_raw_source.py` 只在实际出现 `1970-01-01` 时写入占位日期问题。
- [ ] affected fields 的 profile report 同时展示 NULL 数和占位日期数。
- [ ] 修复后的 affected raw profile 不再把已归零的占位日期写成未修复问题。

## 8. 完成闭环：全分区重刷、真实数据验证与重新 profiling

### 8.1 重刷范围

三阶段修复通过后，必须重跑以下受影响数据的 S3 Parquet -> ClickHouse raw 更新。
snapshot 表重跑一次 raw sync；year 分区表必须重跑全部已有年份分区，不能只抽样验证。

| 数据集 | ClickHouse raw asset | 分区策略 | 重刷要求 |
|--------|----------------------|----------|----------|
| `baostock__query_stock_basic` | `key:clickhouse/raw/baostock__query_stock_basic` | snapshot | 重跑 snapshot raw sync |
| `eastmoney__balance` | `key:clickhouse/raw/eastmoney__balance` | year | 重跑全部已有 year 分区 |
| `eastmoney__cashflow_sq` | `key:clickhouse/raw/eastmoney__cashflow_sq` | year | 重跑全部已有 year 分区 |
| `eastmoney__cashflow_ytd` | `key:clickhouse/raw/eastmoney__cashflow_ytd` | year | 重跑全部已有 year 分区 |
| `eastmoney__income_sq` | `key:clickhouse/raw/eastmoney__income_sq` | year | 重跑全部已有 year 分区 |
| `eastmoney__income_ytd` | `key:clickhouse/raw/eastmoney__income_ytd` | year | 重跑全部已有 year 分区 |
| `eastmoney__dividend_main` | `key:clickhouse/raw/eastmoney__dividend_main` | year | 重跑全部已有 year 分区 |
| `jiuyan__action_field_compacted` | `key:clickhouse/raw/jiuyan__action_field_compacted` | year | 重跑全部已有 year 分区 |
| `jiuyan__industry_list` | `key:clickhouse/raw/jiuyan__industry_list` | snapshot | 重跑 snapshot raw sync |

执行前按 `docs/skills/fleur-dagster-backfill-runbook/SKILL.md` 初始化 Dagster 环境：

```bash
set -a
. ./.env
set +a
make dagster-home
cd pipeline
```

snapshot raw sync：

```bash
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_stock_basic"

uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/jiuyan__industry_list"
```

year 分区 raw sync 必须逐年提交。年份范围以 S3 Parquet 现有分区或
`docs/jobs/reports/2026-06-02-clickhouse-layered-database-partitions.json` 中记录的
manifest 为准；如果 manifest 与 S3 实际对象不一致，以 S3 实际对象为准，并在执行报告中说明差异。

```bash
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/<dataset>" \
  --partition YYYY
```

建议执行顺序：

1. 每个 year 分区表先选一个包含占位问题的年份做 smoke run。
2. smoke run 的 schema 和计数通过后，再展开到该数据集全部已有 year 分区。
3. EastMoney 多表回刷遵守 `eastmoney_run_pool` 并发限制，逐数据集记录每个 partition 的 run id。
4. 如 source Parquet 本身已经是占位日期，先修复 source conversion 并重跑 source asset，再执行本节 raw sync。

### 8.2 真实数据验证

每个 affected field 都要在重刷后验证 ClickHouse raw 的真实数据，而不是只验证生成 SQL。
验证结果写入 `docs/jobs/reports/<date>-raw-date-placeholder-nullability-remediation.md`。

通用 ClickHouse 校验口径：

```sql
SELECT
    count() AS rows,
    countIf(isNull(<field>)) AS null_count,
    countIf(<field> = toDate('1970-01-01')) AS placeholder_count
FROM fleur_raw.<dataset>;
```

DateTime 字段使用：

```sql
SELECT
    count() AS rows,
    countIf(isNull(<field>)) AS null_count,
    countIf(<field> = toDateTime64('1970-01-01 00:00:00', 3)) AS placeholder_count
FROM fleur_raw.<dataset>;
```

year 分区表还要按年验证，确认没有遗漏未重刷分区：

```sql
SELECT
    year,
    count() AS rows,
    countIf(isNull(<field>)) AS null_count,
    countIf(<field> = toDate('1970-01-01')) AS placeholder_count
FROM fleur_raw.<dataset>
GROUP BY year
ORDER BY year;
```

必须记录的验收结果：

- [ ] `baostock__query_stock_basic.outDate`: `countIf(isNull(outDate)) = 7644`，`countIf(outDate = toDate('1970-01-01')) = 0`。
- [ ] EastMoney `UPDATE_DATE` 和 `eastmoney__dividend_main` affected 日期字段：raw 层 `1970-01-01` 计数为 0，缺失值保留为 NULL；如源端仍提供占位，执行报告必须标明源端占位证据和后续 source conversion 修复任务。
- [ ] JiuYan affected 时间字段：raw 层 `1970-01-01 00:00:00` 计数为 0，缺失值保留为 NULL。
- [ ] `uv run fleur-contracts validate-clickhouse --all-available` 通过。
- [ ] 每个重刷命令、partition、Dagster run id、成功 / 失败状态和最终计数写入 `docs/jobs/reports/`。

### 8.3 重新执行 fleur-dbt-model-readiness 数据特征分析

真实 raw 数据验证通过后，必须按 `docs/skills/fleur-dbt-model-readiness/SKILL.md` 对 affected
datasets 重新走一遍 raw source profiling。目标不是写 staging model，而是用修复后的 raw
事实重新生成 staging 前置分析输入。

每个 dataset 重新执行：

```bash
cd pipeline
uv run python elt/scripts/profile_raw_source.py \
  --source raw \
  --table <dataset> \
  --execute \
  --output ../docs/references/raw_profile/<dataset>.md
```

重新 profiling 必须覆盖以下检查项：

- row count、日期范围和 year 分区范围与重刷后的 ClickHouse raw 一致。
- affected 日期 / 时间字段显示真实 NULL 计数，`1970-01-01` / `1970-01-01 00:00:00` 计数为 0，除非执行报告记录了源端占位例外。
- grain、候选自然键、null、空字符串、占位值、枚举 top values、证券代码 / 日期格式、数值范围和极端值重新基于修复后的 raw 数据生成。
- 从 report 中重新提取 staging 设计建议：rename、cast、normalize、null handling、data tests、`config.meta.source_columns` 和 deferred-to-intermediate/mart 判断。

完成标准：

- [ ] 第 8.1 节全部 affected raw assets 已从 S3 Parquet 全量重刷到 ClickHouse raw。
- [ ] 第 8.2 节真实 ClickHouse raw 数据验证全部通过。
- [ ] 第 8.3 节 affected `docs/references/raw_profile/*.md` 已按 `fleur-dbt-model-readiness` 重新生成。
- [ ] 修复前后 NULL / `1970-01-01` 计数变化、重刷范围、run id 和 profiling 命令写入 `docs/jobs/reports/`。
- [ ] 本文档状态更新为 `Completed`，或创建链接到本文档的完成报告并在本文档中标注完成报告路径。

## 9. 最小验证命令

```bash
cd pipeline

uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
uv run pytest scheduler/tests/unit/clickhouse -q

uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

文档-only 更新至少运行：

```bash
git diff --check -- docs/optimize docs/references/raw_profile
```

## 10. 禁止模式

- 不允许只在 staging 层长期清洗 raw 层错误的 `1970-01-01`。
- 不允许 `clickhouse_raw.fields[].nullable: true` 继续生成非 Nullable ClickHouse 类型。
- 不允许用 `input_format_null_as_default = 1` 静默吞掉 nullable 日期字段。
- 不允许 raw profile 把通用占位日期提醒写成实际数据问题。
- 不允许在没有 S3 Parquet 核验的情况下断言问题一定来自源端或 ClickHouse。
- 不允许只重刷 smoke 分区后宣告完成；year 分区表必须重刷全部已有分区。
- 不允许跳过修复后的 `fleur-dbt-model-readiness` 数据特征分析。
