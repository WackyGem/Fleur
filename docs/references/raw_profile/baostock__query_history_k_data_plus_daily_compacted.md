# Raw 数据画像：baostock__query_history_k_data_plus_daily_compacted

日期：2026-06-25

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily_compacted.yml`
- dbt source：`source('raw', 'baostock__query_history_k_data_plus_daily_compacted')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- staging model：`pipeline/elt/models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.sql`
- 迁移报告：`docs/jobs/reports/2026-06-25-baostock-daily-kline-compaction.md`

## 1. 范围

- source 名称：`raw`
- raw 表：`baostock__query_history_k_data_plus_daily_compacted`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table baostock__query_history_k_data_plus_daily_compacted --key code --key date --date-column date --enum-column code --enum-column adjustflag --enum-column tradestatus --enum-column isST --format-column code --numeric-column open --numeric-column high --numeric-column low --numeric-column close --numeric-column volume --numeric-column amount --numeric-column preclose --numeric-column adjustflag --numeric-column turn --numeric-column tradestatus --execute --status Accepted --output ../docs/references/raw_profile/baostock__query_history_k_data_plus_daily_compacted.md`
- 行数：19,648,244
- 数据范围：`date`: 1990-12-19 至 2026-06-25，NULL 0 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw `year`: 1990 至 2026，共 37 个 year 分区
- 契约数据集：`baostock__query_history_k_data_plus_daily_compacted`
- ClickHouse raw 表：`fleur_raw.baostock__query_history_k_data_plus_daily_compacted`
- 表说明：BaoStock 日频行情年度压缩数据，作为 dbt staging 的 raw source。

2026 分区是当前 dev 快照，只有 113 行、1 个代码 `sh.000001`；本画像记录当前 dev 状态，不把 2026 视为完整历史覆盖验收。

## 2. 粒度与键

- 观察到的粒度：一行代表一个 BaoStock `code` 在一个 `date` 的日频行情记录。
- 候选自然键：`code`, `date`
- 重复检查：`group by code, date having count(*) > 1` 未返回重复行。
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；若后续 source 出现候选键重复，应在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL / 空值 | 去重/样例 | 备注 |
|------|------|-------------|-----------|------|
| date | Date | NULL 0；`1970-01-01` 0 | 1990-12-19 至 2026-06-25 | BaoStock 行情接口返回的交易日期。 |
| code | LowCardinality(String) | 空值 0；全部为供应商前缀格式 | 高频：`sh.600653`, `sh.600602`, `sh.600651`, `sh.600601`, `sh.600654` | 需转成 canonical `000001.SZ` 格式。 |
| open | Nullable(Float64) | NULL 70,545；负值 0；0 值 0 | min 0.09；max 44,610.4552 | 交易日开盘价。 |
| high | Nullable(Float64) | NULL 70,545；负值 0；0 值 0 | min 0.11；max 44,903.5195 | 交易日最高价。 |
| low | Nullable(Float64) | NULL 70,545；负值 0；0 值 0 | min 0.09；max 42,420.6527 | 交易日最低价。 |
| close | Nullable(Float64) | NULL 70,545；负值 0；0 值 0 | min 0.09；max 43,972.016 | 交易日收盘价。 |
| preclose | Nullable(Float64) | NULL 70,597；负值 0；0 值 428 | min 0；max 43,972.016 | 上一交易日收盘价。 |
| volume | Nullable(Int64) | NULL 85,102；负值 0；0 值 578,882 | min 0；max 1,055,245,090,000 | 交易日成交量。 |
| amount | Nullable(Float64) | NULL 85,102；负值 0；0 值 578,884 | min 0；max 3,438,502,721,073.89 | 交易日成交金额。 |
| adjustflag | Int8 | NULL 0 | `3`: 19,648,244 | 行情复权标记；当前 raw 全部是不复权口径。 |
| turn | Nullable(Float64) | NULL 686,077；负值 0；0 值 90,205 | min 0；max 9999.999999 | 交易日换手率。 |
| tradestatus | Int8 | NULL 0 | `1`: 19,063,208；`0`: 585,036 | 交易日交易状态。 |
| pctChg | Nullable(Float64) | 本轮未逐列统计 | 保留 raw 字段 | 交易日涨跌幅；当前 staging 不输出。 |
| isST | Nullable(Bool) | NULL 17,702 | `False`: 18,860,041；`True`: 770,501 | 证券是否为 ST 或风险警示状态。 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`code`
- 观察到的格式：canonical 后缀 0/19,648,244；供应商前缀 19,648,244/19,648,244；纯数字 0/19,648,244；空值 0/19,648,244。
- 建议 staging 处理：BaoStock `sh.`/`sz.` 前缀格式必须用 normalization macro 转成 `000001.SZ` 格式；不要在 staging 做主数据修正。

### 日期与时间字段

- 已画像字段：`date`
- 范围：1990-12-19 至 2026-06-25，NULL 0 行，`1970-01-01` 占位 0 行。
- 建议 staging 处理：Date 类型保持 Date；不需要占位日期转 NULL。

### 枚举字段

- 已画像字段：`code`, `adjustflag`, `tradestatus`, `isST`
- 取值：`adjustflag` 全部为 `3`；`tradestatus` 为 `1` 或 `0`；`isST` 为 `False`、`True` 或 NULL。
- 建议 staging 处理：布尔/状态字段保留 source-local 语义；`tradestatus = 0` 可派生 `is_suspend`。

### 数值字段

- 已画像字段：`open`, `high`, `low`, `close`, `volume`, `amount`, `preclose`, `adjustflag`, `turn`, `tradestatus`
- 负数/零值/极端值：未发现负值；成交量、成交金额、preclose 和 turn 存在业务合理 0 值；价格字段 NULL 集中在缺失行情记录，不在 staging 静默补值。
- 单位假设：保留 BaoStock raw 单位；金额、比例和价格单位在下游模型按业务口径解释。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `code` 使用供应商前缀格式 | 中 | 19,648,244/19,648,244 行 | 使用证券代码 normalization macro 转成 `000001.SZ` 格式 | 无 |
| 2026 分区不完整 | 高 | 113 行、1 个代码 `sh.000001` | 不在 staging 修正；迁移报告标注 caveat | 后续补齐策略单独验收 |

## 6. 建议的 Staging 转换

- 重命名：按 `pipeline/elt/metadata/field_glossary.yml` 选择 canonical 字段。
- 类型转换：raw Date/Bool/Float/Int 类型已由 ClickHouse schema 承载；staging 不做额外宽表 cast。
- 标准化：证券代码使用 `normalize_cn_security_code(input_format='baostock_prefix')`。
- NULL 处理：保留 raw NULL；不把缺失价格或 `isST` NULL 静默补为 0/false。
- 测试：`security_code`、`trade_date`、组合键和 `is_suspend` 保持现有 tests。
- YAML 元数据：每个 staging 输出字段记录 `config.meta.source_columns`；派生字段记录 `derived_from` 和派生说明。

## 7. 延后到 Intermediate/Mart

- 跨源 join：证券主数据、指数/股票归属、复权因子和财务估值匹配延后。
- 需要优先级判断的去重：候选键重复或多版本选择不在 staging 静默处理。
- 主数据修正：证券代码历史、上市/退市状态、交易所归属修正延后。
- 粒度变化：行情事实组装和复权口径合并延后。
- 业务指标逻辑：涨跌幅重算、停牌业务裁决、异常阈值判断延后。

## 8. 待确认问题

- [ ] 2026 完整性需要后续补齐策略单独验收。

## 9. 验收清单

- [x] 已抽样 raw source。
- [x] 已记录行数和日期/分区范围。
- [x] 已评估粒度和候选键。
- [x] 已完成关键字段画像。
- [x] 已列出 staging 转换建议。
- [x] 已列出延后处理事项。
- [x] 已提出测试或明确豁免。

## Profiling SQL 与结果摘要

- `select count(*) from source('raw', 'baostock__query_history_k_data_plus_daily_compacted')`：19,648,244。
- 日期范围：1990-12-19 至 2026-06-25，NULL 0 行，`1970-01-01` 占位 0 行。
- 候选键重复：未发现重复。
- 证券代码格式：供应商前缀 19,648,244/19,648,244，空值 0。
- 枚举 top values：`adjustflag=3` 19,648,244；`tradestatus=1` 19,063,208，`tradestatus=0` 585,036；`isST=False` 18,860,041，`isST=True` 770,501，`isST=NULL` 17,702。
- 数值范围摘要：`open` min=0.09, max=44,610.4552, NULL=70,545；`high` min=0.11, max=44,903.5195, NULL=70,545；`low` min=0.09, max=42,420.6527, NULL=70,545；`close` min=0.09, max=43,972.016, NULL=70,545；`volume` min=0, max=1,055,245,090,000, NULL=85,102；`amount` min=0, max=3,438,502,721,073.89, NULL=85,102；`preclose` min=0, max=43,972.016, NULL=70,597；`turn` min=0, max=9999.999999, NULL=686,077；`tradestatus` min=0, max=1, NULL=0。
