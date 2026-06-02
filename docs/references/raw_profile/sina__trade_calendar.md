# Raw 数据画像：sina__trade_calendar

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/sina__trade_calendar.yml`
- dbt source：`source('raw', 'sina__trade_calendar')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：`pipeline/elt/models/staging/sina/stg_sina__trade_calendar.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`sina__trade_calendar`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table sina__trade_calendar --execute --output ../docs/references/raw_profile/sina__trade_calendar.md`
- 行数：8,797
- 数据范围：`1990-12-19` 至 `2026-12-31`
- 分区范围：最新快照，无分区字段
- 契约数据集：`sina__trade_calendar`
- ClickHouse raw 表：`raw.sina__trade_calendar`
- 表说明：A 股交易日历快照。

## 2. 粒度与键

- 观察到的粒度：每个 A 股交易日一行
- 候选自然键：`trade_date`
- 重复检查：profiling 查询未返回重复的 `trade_date`
- 粒度注意事项：数据包含截至 `2026-12-31` 的未来日期；下游应将其理解为交易日历，而不是已发生的市场交易事实

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| trade_date | Date | 0 | 不适用 | 8,797 行，范围 1990-12-19 至 2026-12-31 | 来自 `sina` 原始字段 `trade_date`。原始字段说明：新浪交易日历中的 A 股交易日期。 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：不适用
- 观察到的格式：不适用，本表没有证券代码字段。
- 无效样例：不适用。
- 建议 staging 处理：staging 不需要处理证券代码。

### 日期与时间字段

- 已画像字段：`trade_date`
- 范围：`1990-12-19` 至 `2026-12-31`
- 无效值或占位值：NULL 计数 profiling 未发现无效值。
- 建议 staging 处理：直接保留为 canonical `trade_date`，并保留 `not_null` 测试。

### 枚举字段

- 已画像字段：不适用
- 取值：不适用。
- 未知或异常取值：不适用。
- 建议 staging 处理：staging 不需要处理枚举字段。

### 数值字段

- 已画像字段：不适用
- 最小/最大值：不适用。
- 负数/零值/极端值：不适用。
- 单位假设：不适用。
- 建议 staging 处理：staging 不需要处理数值字段。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 交易日历包含截至 2026-12-31 的未来日期 | 低 | 日期范围 profiling 查询 | 保留这些日期；交易日历快照包含未来日期是预期行为 | 下游模型不应将未来日期视为已发生的交易数据 |

## 6. 建议的 Staging 转换

- 重命名：保留 `trade_date` 作为规范字段 `trade_date`
- 类型转换：无需转换；raw 类型已经是 `Date`
- 标准化：无需标准化
- NULL 处理：未观察到 NULL；保留 `not_null`
- 测试：对 `trade_date` 添加 `not_null`
- YAML 元数据：`glossary_key: trade_date`，source column lineage（源字段血缘）指向 `raw.sina__trade_calendar.trade_date`

## 7. 延后到 Intermediate/Mart

- 跨源 join：该 staging model 不需要
- 需要优先级判断的去重：未观察到
- 主数据修正：无
- 粒度变化：无
- 业务指标逻辑：无

## 8. 待确认问题

- [ ] 确认下游模型是否需要单独的 `is_future_trade_date` 字段或交易日历有效性过滤条件。

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
from {{ source('raw', 'sina__trade_calendar') }}
```


结果（成功）：

```text
15:36:32  Running with dbt=1.11.11
15:36:33  Registered adapter: clickhouse=1.10.0
15:36:33  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:33  
15:36:33  Concurrency: 1 threads (target='dev')
15:36:33  
Previewing inline node:
| trade_date |
| ---------- |
| 1990-12-19 |
| 1990-12-20 |
| 1990-12-21 |
| 1990-12-24 |
| 1990-12-25 |
| 1990-12-26 |
| 1990-12-27 |
| 1990-12-28 |
| 1990-12-31 |
| 1991-01-02 |
| 1991-01-03 |
| 1991-01-04 |
| 1991-01-07 |
| 1991-01-08 |
| 1991-01-09 |
| 1991-01-10 |
| 1991-01-11 |
| 1991-01-14 |
| 1991-01-15 |
| 1991-01-16 |
| 1991-01-17 |
| 1991-01-18 |
| 1991-01-21 |
| 1991-01-22 |
| 1991-01-23 |
| 1991-01-24 |
| 1991-01-25 |
| 1991-01-28 |
| 1991-01-29 |
| 1991-01-30 |
| 1991-01-31 |
| 1991-02-01 |
| 1991-02-04 |
| 1991-02-05 |
| 1991-02-06 |
| 1991-02-07 |
| 1991-02-08 |
| 1991-02-11 |
| 1991-02-12 |
| 1991-02-13 |
| 1991-02-14 |
| 1991-02-19 |
| 1991-02-20 |
| 1991-02-21 |
| 1991-02-22 |
| 1991-02-25 |
| 1991-02-26 |
| 1991-02-27 |
| 1991-02-28 |
| 1991-03-01 |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'sina__trade_calendar') }}
```


结果（成功）：

```text
15:36:38  Running with dbt=1.11.11
15:36:38  Registered adapter: clickhouse=1.10.0
15:36:39  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:39  
15:36:39  Concurrency: 1 threads (target='dev')
15:36:39  
Previewing inline node:
| row_count |
| --------- |
|      8797 |
```

### 日期范围

```sql
select
    min(`trade_date`) as min_trade_date,
    max(`trade_date`) as max_trade_date,
    countIf(isNull(`trade_date`)) as null_trade_date
from {{ source('raw', 'sina__trade_calendar') }}
```


结果（成功）：

```text
15:36:42  Running with dbt=1.11.11
15:36:43  Registered adapter: clickhouse=1.10.0
15:36:43  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:43  
15:36:43  Concurrency: 1 threads (target='dev')
15:36:43  
Previewing inline node:
| min_trade_date | max_trade_date | null_trade_date |
| -------------- | -------------- | --------------- |
|     1990-12-19 |     2026-12-31 |               0 |
```

### 候选键重复检查

```sql
select
    `trade_date`,
    count(*) as row_count
from {{ source('raw', 'sina__trade_calendar') }}
group by `trade_date`
having row_count > 1
order by row_count desc
```


结果（成功）：

```text
15:36:48  Running with dbt=1.11.11
15:36:48  Registered adapter: clickhouse=1.10.0
15:36:49  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:49  
15:36:49  Concurrency: 1 threads (target='dev')
15:36:49  
Previewing inline node:
||
|  |
```
