# Raw 数据画像：chinabond__government_bond

日期：2026-06-16

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/chinabond__government_bond.yml`
- dbt source：`source('raw', 'chinabond__government_bond')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：`pipeline/elt/models/staging/chinabond/stg_chinabond__government_bond.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`chinabond__government_bond`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table chinabond__government_bond --key work_date --date-column work_date --enum-column curve_name --numeric-column three_month_yield_pct --numeric-column six_month_yield_pct --numeric-column one_year_yield_pct --numeric-column two_year_yield_pct --numeric-column three_year_yield_pct --numeric-column five_year_yield_pct --numeric-column seven_year_yield_pct --numeric-column ten_year_yield_pct --numeric-column fifteen_year_yield_pct --numeric-column twenty_year_yield_pct --numeric-column thirty_year_yield_pct --execute --status Accepted --output ../docs/references/raw_profile/chinabond__government_bond.md`
- 行数：5,075
- 数据范围：`work_date`: 2006-03-01 至 2026-06-16，NULL 0 行，占位日期 0 行
- 分区范围：2006-2026 年度 raw 分区；ClickHouse raw 表未暴露独立 `year` 输出列给 staging 使用
- 契约数据集：`chinabond__government_bond`
- ClickHouse raw 表：`fleur_raw.chinabond__government_bond`
- 表说明：chinabond__government_bond

## 2. 数据分析发现

基于当前 raw 表的现状分析：

- 数据量与覆盖
  - 总记录数：5,075
  - 覆盖主体数：1 条曲线，`curve_name` 全部为 `中债国债收益率曲线`
  - 日期 / 分区范围：`work_date` 覆盖 2006-03-01 至 2026-06-16，年度 raw 分区覆盖 2006-2026
- 粒度与候选键
  - 观察到的粒度：每个 ChinaBond 国债收益率曲线工作日一行
  - 候选自然键去重结果：`work_date` 未发现重复
  - 旧候选键或备选键对比：`curve_name` 当前为单一枚举，不需要进入 staging natural key
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`work_date`、`curve_name` 和 3M/6M/1Y/2Y/3Y/5Y/7Y/10Y/30Y 收益率均无 NULL；15Y、20Y 收益率 5,075 行全为 NULL
  - 占位值：`work_date` 无 `1970-01-01`；收益率无 0 值和负值
  - 预期缺失：15Y、20Y 为上游当前未提供的期限点，staging 保留 nullable，不填补
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：无证券代码字段；`work_date` 为 ClickHouse `Date`
  - 直接 raw input 参照命中情况：staging 直接引用 `source('raw', 'chinabond__government_bond')`
- 分布与相关性
  - 枚举 top values：`curve_name = 中债国债收益率曲线`，5,075 行
  - 少量值 / 长尾文本：未发现多枚举或长尾文本字段
  - 字段间强相关：各期限收益率同为同一曲线的横截面点；不在 staging 计算期限利差或形态指标
- 时间字段合理性
  - 日期范围：2006-03-01 至 2026-06-16
  - 日期先后关系异常：未发现 NULL、占位日期或重复日期
  - 批次时间范围：初始 raw 回填覆盖 2006-2026
- 数值字段合理性
  - 负数 / 零值 / 极端值：所有非空收益率无负数、无 0 值；最大值为 30Y 的 5.2
  - 单位判断：收益率字段单位为百分比点，字段名后缀 `_yield_pct` 不代表小数比例
- 其他观察
  - 对 staging 设计有影响、但不应在 staging 静默修正的事实：`curve_name` 当前不输出到 staging；如未来需要多曲线建模，应调整 grain 和 natural key

## 3. 粒度与键

- 观察到的粒度：每个 ChinaBond 国债收益率曲线工作日一行
- 候选自然键：`work_date`，在 staging 中改名为 `trade_date`
- 重复检查：未发现重复 `work_date`
- 粒度注意事项：当前 raw 只有 `中债国债收益率曲线` 一条曲线，staging 不输出 `curve_name`；若未来接入多曲线，必须重新评估 grain

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| work_date | Date | 0 | 无 `1970-01-01` | 5,075 行唯一 | staging 改名为 `trade_date`。 |
| curve_name | LowCardinality(String) | 0 | 无 | `中债国债收益率曲线` 5,075 行 | 仅用于 raw 核验，staging 不输出。 |
| three_month_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 0.78, max 5.11 | 3 个月收益率，百分比点。 |
| six_month_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 0.82, max 4.37 | 6 个月收益率，百分比点。 |
| one_year_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 0.89, max 4.25 | 1 年收益率，百分比点。 |
| two_year_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 1.01, max 4.42 | 2 年收益率，百分比点。 |
| three_year_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 1.09, max 4.5 | 3 年收益率，百分比点。 |
| five_year_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 1.34, max 4.53 | 5 年收益率，百分比点。 |
| seven_year_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 1.49, max 4.67 | 7 年收益率，百分比点。 |
| ten_year_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 1.6, max 4.72 | 10 年收益率，百分比点。 |
| fifteen_year_yield_pct | Nullable(Float64) | 5,075 | 全部 NULL | 无非空值 | 15 年收益率，上游当前未提供。 |
| twenty_year_yield_pct | Nullable(Float64) | 5,075 | 全部 NULL | 无非空值 | 20 年收益率，上游当前未提供。 |
| thirty_year_yield_pct | Nullable(Float64) | 0 | 无 0 值/负值 | min 1.8, max 5.2 | 30 年收益率，百分比点。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：无
- 观察到的格式：无证券代码字段。
- 无效样例：不适用。
- 建议 staging 处理：不适用。

### 日期与时间字段

- 已画像字段：`work_date`
- 范围：`work_date`: 2006-03-01 至 2026-06-16，NULL 0 行，占位日期 0 行。
- 无效值或占位值：未发现。
- 建议 staging 处理：改名为 `trade_date`，保留 `Date` 类型。

### 枚举字段

- 已画像字段：`curve_name`
- 取值：`中债国债收益率曲线` 5,075 行。
- 未知或异常取值：未发现。
- 建议 staging 处理：`curve_name` 不输出；如未来出现多曲线，应重新设计 grain，而不是静默过滤。

### 数值字段

- 已画像字段：`three_month_yield_pct`, `six_month_yield_pct`, `one_year_yield_pct`, `two_year_yield_pct`, `three_year_yield_pct`, `five_year_yield_pct`, `seven_year_yield_pct`, `ten_year_yield_pct`, `fifteen_year_yield_pct`, `twenty_year_yield_pct`, `thirty_year_yield_pct`
- 最小/最大值：核心期限收益率最小值 0.78，最大值 5.2；15Y/20Y 无非空值。
- 负数/零值/极端值：所有非空收益率无负数、无 0 值。
- 单位假设：百分比点，保留 `_yield_pct` 后缀，不除以 100。
- 建议 staging 处理：仅透传数值；核心期限加 `not_null`，15Y/20Y 保留 nullable。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 15Y/20Y 收益率全为空 | 低 | 两列 NULL 数均为 5,075/5,075 | 保留 nullable，不填补 | 若业务需要完整期限结构，在 intermediate/mart 做插值或补点 |
| `curve_name` 当前为单一枚举 | 低 | `中债国债收益率曲线` 5,075 行 | staging 不输出 | 多曲线接入时重评 grain 和 natural key |

## 7. Staging 设计决策

- 重命名：`work_date` 改名为 `trade_date`；收益率列保持 raw canonical 名称。
- 类型转换：raw Date/Float64 类型已由 ClickHouse schema 承载，staging 不额外 cast。
- 标准化：不做跨源日期校准，不用 A 股交易日历过滤 ChinaBond 日期。
- NULL 处理：15Y/20Y 保留 NULL，不填补；其他核心收益率若未来出现 NULL，应由 dbt tests 暴露。
- 测试：`trade_date` 加 `not_null`/`unique`；3M/6M/1Y/2Y/3Y/5Y/7Y/10Y/30Y 加 `not_null`；15Y/20Y 不加 `not_null`。
- YAML 元数据：每个输出字段记录 `dictionary_scope: local` 和 `source_columns`；收益率列记录 `unit: percent`、`scale: percent_value_not_fraction`。

## 8. 延后到 Intermediate/Mart

- 跨源 join：与交易日历、行情、宏观或其他利率曲线数据的 join 延后。
- 需要优先级判断的去重：如未来出现重复日期，不在 staging 静默取最新。
- 主数据修正：不使用 A 股交易日历修正 ChinaBond 工作日。
- 粒度变化：宽表转期限长表延后。
- 业务指标逻辑：期限利差、斜率、曲率、插值和百分比点转小数比例延后。

## 待确认问题

- [x] 当前 staging 不输出 `curve_name`；如未来出现多曲线枚举，需重新设计 staging 粒度。

## 关键 SQL 证据摘要

- 行数：5,075
- 日期 / 分区范围：`work_date` 2006-03-01 至 2026-06-16；年度 raw 分区覆盖 2006-2026
- 候选键重复：`work_date` 未发现重复
- 关键 NULL / 占位值：`work_date` NULL 0、占位日期 0；15Y/20Y 全 NULL
- 枚举 / 文本分布：`curve_name = 中债国债收益率曲线` 5,075 行
- 数值范围：非空收益率无负数、无 0 值；min/max 见字段画像

## 9. 验收清单

- [x] 已抽样 raw source。
- [x] 已记录行数和日期/分区范围。
- [x] 已评估粒度和候选键。
- [x] 已完成关键字段画像。
- [x] 已列出 staging 转换建议。
- [x] 已列出延后处理事项。
- [x] 已提出测试或明确豁免。

## Profiling SQL 与结果

### 样例行

```sql
select *
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:49:58  Running with dbt=1.11.11
20:49:58  Registered adapter: clickhouse=1.10.0
20:49:59  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:49:59
20:49:59  Concurrency: 1 threads (target='dev')
20:49:59
Previewing inline node:
|  work_date | curve_name | three_month_yield... | six_month_yield_pct | one_year_yield_pct | two_year_yield_pct | ... |
| ---------- | ---------- | -------------------- | ------------------- | ------------------ | ------------------ | --- |
| 2006-03-01 | 中债国债收益率曲线  |                 1.51 |                1.59 |               1.68 |               1.85 | ... |
| 2006-03-02 | 中债国债收益率曲线  |                 1.47 |                1.52 |               1.69 |               1.86 | ... |
| 2006-03-03 | 中债国债收益率曲线  |                 1.46 |                1.51 |               1.68 |               1.84 | ... |
| 2006-03-06 | 中债国债收益率曲线  |                 1.50 |                1.56 |               1.73 |               1.85 | ... |
| 2006-03-07 | 中债国债收益率曲线  |                 1.47 |                1.53 |               1.71 |               1.83 | ... |
| 2006-03-08 | 中债国债收益率曲线  |                 1.45 |                1.52 |               1.69 |               1.81 | ... |
| 2006-03-09 | 中债国债收益率曲线  |                 1.46 |                1.53 |               1.68 |               1.81 | ... |
| 2006-03-10 | 中债国债收益率曲线  |                 1.46 |                1.53 |               1.67 |               1.82 | ... |
| 2006-03-13 | 中债国债收益率曲线  |                 1.47 |                1.54 |               1.68 |               1.83 | ... |
| 2006-03-14 | 中债国债收益率曲线  |                 1.48 |                1.55 |               1.69 |               1.84 | ... |
| 2006-03-15 | 中债国债收益率曲线  |                 1.50 |                1.57 |               1.71 |               1.85 | ... |
| 2006-03-16 | 中债国债收益率曲线  |                 1.51 |                1.58 |               1.72 |               1.86 | ... |
| 2006-03-17 | 中债国债收益率曲线  |                 1.49 |                1.56 |               1.70 |               1.84 | ... |
| 2006-03-20 | 中债国债收益率曲线  |                 1.51 |                1.58 |               1.71 |               1.85 | ... |
| 2006-03-21 | 中债国债收益率曲线  |                 1.50 |                1.57 |               1.70 |               1.84 | ... |
| 2006-03-22 | 中债国债收益率曲线  |                 1.52 |                1.58 |               1.71 |               1.84 | ... |
| 2006-03-23 | 中债国债收益率曲线  |                 1.51 |                1.60 |               1.69 |               1.90 | ... |
| 2006-03-24 | 中债国债收益率曲线  |                 1.49 |                1.58 |               1.68 |               1.89 | ... |
| 2006-03-27 | 中债国债收益率曲线  |                 1.50 |                1.61 |               1.71 |               1.93 | ... |
| 2006-03-28 | 中债国债收益率曲线  |                 1.49 |                1.60 |               1.70 |               1.93 | ... |
| 2006-03-29 | 中债国债收益率曲线  |                 1.49 |                1.62 |               1.70 |               1.93 | ... |
| 2006-03-30 | 中债国债收益率曲线  |                 1.51 |                1.64 |               1.72 |               1.92 | ... |
| 2006-03-31 | 中债国债收益率曲线  |                 1.51 |                1.64 |               1.71 |               1.91 | ... |
| 2006-04-03 | 中债国债收益率曲线  |                 1.50 |                1.64 |               1.72 |               1.91 | ... |
| 2006-04-04 | 中债国债收益率曲线  |                 1.49 |                1.62 |               1.73 |               1.94 | ... |
| 2006-04-05 | 中债国债收益率曲线  |                 1.49 |                1.60 |               1.70 |               1.94 | ... |
| 2006-04-06 | 中债国债收益率曲线  |                 1.47 |                1.58 |               1.69 |               1.93 | ... |
| 2006-04-07 | 中债国债收益率曲线  |                 1.45 |                1.55 |               1.64 |               1.86 | ... |
| 2006-04-10 | 中债国债收益率曲线  |                 1.45 |                1.55 |               1.66 |               1.88 | ... |
| 2006-04-11 | 中债国债收益率曲线  |                 1.47 |                1.57 |               1.67 |               1.89 | ... |
| 2006-04-12 | 中债国债收益率曲线  |                 1.49 |                1.59 |               1.69 |               1.91 | ... |
| 2006-04-13 | 中债国债收益率曲线  |                 1.48 |                1.57 |               1.70 |               1.90 | ... |
| 2006-04-14 | 中债国债收益率曲线  |                 1.49 |                1.58 |               1.71 |               1.92 | ... |
| 2006-04-17 | 中债国债收益率曲线  |                 1.52 |                1.60 |               1.71 |               1.95 | ... |
| 2006-04-18 | 中债国债收益率曲线  |                 1.55 |                1.64 |               1.71 |               1.99 | ... |
| 2006-04-19 | 中债国债收益率曲线  |                 1.55 |                1.63 |               1.70 |               1.98 | ... |
| 2006-04-20 | 中债国债收益率曲线  |                 1.54 |                1.62 |               1.71 |               2.00 | ... |
| 2006-04-21 | 中债国债收益率曲线  |                 1.55 |                1.62 |               1.72 |               1.98 | ... |
| 2006-04-24 | 中债国债收益率曲线  |                 1.56 |                1.62 |               1.70 |               1.98 | ... |
| 2006-04-25 | 中债国债收益率曲线  |                 1.56 |                1.63 |               1.72 |               2.02 | ... |
| 2006-04-26 | 中债国债收益率曲线  |                 1.57 |                1.65 |               1.70 |               2.02 | ... |
| 2006-04-27 | 中债国债收益率曲线  |                 1.54 |                1.63 |               1.73 |               2.02 | ... |
| 2006-04-28 | 中债国债收益率曲线  |                 1.52 |                1.62 |               1.72 |               2.00 | ... |
| 2006-04-29 | 中债国债收益率曲线  |                 1.54 |                1.65 |               1.72 |               1.97 | ... |
| 2006-04-30 | 中债国债收益率曲线  |                 1.53 |                1.63 |               1.73 |               1.95 | ... |
| 2006-05-08 | 中债国债收益率曲线  |                 1.54 |                1.65 |               1.71 |               1.98 | ... |
| 2006-05-09 | 中债国债收益率曲线  |                 1.52 |                1.63 |               1.71 |               1.96 | ... |
| 2006-05-10 | 中债国债收益率曲线  |                 1.50 |                1.60 |               1.69 |               1.98 | ... |
| 2006-05-11 | 中债国债收益率曲线  |                 1.52 |                1.62 |               1.69 |               1.98 | ... |
| 2006-05-12 | 中债国债收益率曲线  |                 1.54 |                1.64 |               1.74 |               1.98 | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:04  Running with dbt=1.11.11
20:50:04  Registered adapter: clickhouse=1.10.0
20:50:05  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:05
20:50:05  Concurrency: 1 threads (target='dev')
20:50:05
Previewing inline node:
| row_count |
| --------- |
|      5075 |
```

### 日期范围

```sql
select
    min(`work_date`) as min_work_date,
    max(`work_date`) as max_work_date,
    countIf(isNull(`work_date`)) as null_work_date,
    countIf(`work_date` = toDate('1970-01-01')) as placeholder_work_date
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:09  Running with dbt=1.11.11
20:50:10  Registered adapter: clickhouse=1.10.0
20:50:11  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:11
20:50:11  Concurrency: 1 threads (target='dev')
20:50:11
Previewing inline node:
| min_work_date | max_work_date | null_work_date | placeholder_work_... |
| ------------- | ------------- | -------------- | -------------------- |
|    2006-03-01 |    2026-06-16 |              0 |                    0 |
```

### 候选键重复检查

```sql
select
    `work_date`,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
group by `work_date`
having row_count > 1
order by row_count desc
```


结果（成功）：

```text
20:50:15  Running with dbt=1.11.11
20:50:16  Registered adapter: clickhouse=1.10.0
20:50:17  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:17
20:50:17  Concurrency: 1 threads (target='dev')
20:50:17
Previewing inline node:
||
|  |
```

### 高频取值：curve_name

```sql
select
    `curve_name` as value,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
group by `curve_name`
order by row_count desc
```


结果（成功）：

```text
20:50:21  Running with dbt=1.11.11
20:50:21  Registered adapter: clickhouse=1.10.0
20:50:23  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:23
20:50:23  Concurrency: 1 threads (target='dev')
20:50:23
Previewing inline node:
| value     | row_count |
| --------- | --------- |
| 中债国债收益率曲线 |      5075 |
```

### 数值范围：three_month_yield_pct

```sql
select
    min(`three_month_yield_pct`) as min_value,
    max(`three_month_yield_pct`) as max_value,
    countIf(`three_month_yield_pct` = 0) as zero_count,
    countIf(`three_month_yield_pct` < 0) as negative_count,
    countIf(isNull(`three_month_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:27  Running with dbt=1.11.11
20:50:27  Registered adapter: clickhouse=1.10.0
20:50:28  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:28
20:50:28  Concurrency: 1 threads (target='dev')
20:50:28
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      0.78 |      5.11 |          0 |              0 |          0 |      5075 |
```

### 数值范围：six_month_yield_pct

```sql
select
    min(`six_month_yield_pct`) as min_value,
    max(`six_month_yield_pct`) as max_value,
    countIf(`six_month_yield_pct` = 0) as zero_count,
    countIf(`six_month_yield_pct` < 0) as negative_count,
    countIf(isNull(`six_month_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:32  Running with dbt=1.11.11
20:50:32  Registered adapter: clickhouse=1.10.0
20:50:34  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:34
20:50:34  Concurrency: 1 threads (target='dev')
20:50:34
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      0.82 |      4.37 |          0 |              0 |          0 |      5075 |
```

### 数值范围：one_year_yield_pct

```sql
select
    min(`one_year_yield_pct`) as min_value,
    max(`one_year_yield_pct`) as max_value,
    countIf(`one_year_yield_pct` = 0) as zero_count,
    countIf(`one_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`one_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:38  Running with dbt=1.11.11
20:50:38  Registered adapter: clickhouse=1.10.0
20:50:39  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:39
20:50:39  Concurrency: 1 threads (target='dev')
20:50:39
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      0.89 |      4.25 |          0 |              0 |          0 |      5075 |
```

### 数值范围：two_year_yield_pct

```sql
select
    min(`two_year_yield_pct`) as min_value,
    max(`two_year_yield_pct`) as max_value,
    countIf(`two_year_yield_pct` = 0) as zero_count,
    countIf(`two_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`two_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:44  Running with dbt=1.11.11
20:50:44  Registered adapter: clickhouse=1.10.0
20:50:45  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:45
20:50:45  Concurrency: 1 threads (target='dev')
20:50:45
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      1.01 |      4.42 |          0 |              0 |          0 |      5075 |
```

### 数值范围：three_year_yield_pct

```sql
select
    min(`three_year_yield_pct`) as min_value,
    max(`three_year_yield_pct`) as max_value,
    countIf(`three_year_yield_pct` = 0) as zero_count,
    countIf(`three_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`three_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:49  Running with dbt=1.11.11
20:50:50  Registered adapter: clickhouse=1.10.0
20:50:51  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:51
20:50:51  Concurrency: 1 threads (target='dev')
20:50:51
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      1.09 |       4.5 |          0 |              0 |          0 |      5075 |
```

### 数值范围：five_year_yield_pct

```sql
select
    min(`five_year_yield_pct`) as min_value,
    max(`five_year_yield_pct`) as max_value,
    countIf(`five_year_yield_pct` = 0) as zero_count,
    countIf(`five_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`five_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:50:55  Running with dbt=1.11.11
20:50:55  Registered adapter: clickhouse=1.10.0
20:50:57  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:50:57
20:50:57  Concurrency: 1 threads (target='dev')
20:50:57
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      1.34 |      4.53 |          0 |              0 |          0 |      5075 |
```

### 数值范围：seven_year_yield_pct

```sql
select
    min(`seven_year_yield_pct`) as min_value,
    max(`seven_year_yield_pct`) as max_value,
    countIf(`seven_year_yield_pct` = 0) as zero_count,
    countIf(`seven_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`seven_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:51:01  Running with dbt=1.11.11
20:51:01  Registered adapter: clickhouse=1.10.0
20:51:02  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:51:02
20:51:02  Concurrency: 1 threads (target='dev')
20:51:02
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      1.49 |      4.67 |          0 |              0 |          0 |      5075 |
```

### 数值范围：ten_year_yield_pct

```sql
select
    min(`ten_year_yield_pct`) as min_value,
    max(`ten_year_yield_pct`) as max_value,
    countIf(`ten_year_yield_pct` = 0) as zero_count,
    countIf(`ten_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`ten_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:51:06  Running with dbt=1.11.11
20:51:07  Registered adapter: clickhouse=1.10.0
20:51:08  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:51:08
20:51:08  Concurrency: 1 threads (target='dev')
20:51:08
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|       1.6 |      4.72 |          0 |              0 |          0 |      5075 |
```

### 数值范围：fifteen_year_yield_pct

```sql
select
    min(`fifteen_year_yield_pct`) as min_value,
    max(`fifteen_year_yield_pct`) as max_value,
    countIf(`fifteen_year_yield_pct` = 0) as zero_count,
    countIf(`fifteen_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`fifteen_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:51:12  Running with dbt=1.11.11
20:51:12  Registered adapter: clickhouse=1.10.0
20:51:14  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:51:14
20:51:14  Concurrency: 1 threads (target='dev')
20:51:14
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|           |           |          0 |              0 |       5075 |      5075 |
```

### 数值范围：twenty_year_yield_pct

```sql
select
    min(`twenty_year_yield_pct`) as min_value,
    max(`twenty_year_yield_pct`) as max_value,
    countIf(`twenty_year_yield_pct` = 0) as zero_count,
    countIf(`twenty_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`twenty_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:51:18  Running with dbt=1.11.11
20:51:18  Registered adapter: clickhouse=1.10.0
20:51:19  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:51:19
20:51:19  Concurrency: 1 threads (target='dev')
20:51:19
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|           |           |          0 |              0 |       5075 |      5075 |
```

### 数值范围：thirty_year_yield_pct

```sql
select
    min(`thirty_year_yield_pct`) as min_value,
    max(`thirty_year_yield_pct`) as max_value,
    countIf(`thirty_year_yield_pct` = 0) as zero_count,
    countIf(`thirty_year_yield_pct` < 0) as negative_count,
    countIf(isNull(`thirty_year_yield_pct`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'chinabond__government_bond') }}
```


结果（成功）：

```text
20:51:23  Running with dbt=1.11.11
20:51:24  Registered adapter: clickhouse=1.10.0
20:51:25  Found 35 models, 271 data tests, 3 operations, 1 sql operation, 23 sources, 530 macros
20:51:25
20:51:25  Concurrency: 1 threads (target='dev')
20:51:25
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|       1.8 |       5.2 |          0 |              0 |          0 |      5075 |
```
