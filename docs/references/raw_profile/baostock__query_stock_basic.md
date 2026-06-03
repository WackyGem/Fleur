# Raw 数据画像：baostock__query_stock_basic

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/baostock__query_stock_basic.yml`
- dbt source：`source('raw', 'baostock__query_stock_basic')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`baostock__query_stock_basic`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table baostock__query_stock_basic --execute --output ../docs/references/raw_profile/baostock__query_stock_basic.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`baostock__query_stock_basic`
- ClickHouse raw 表：`fleur_raw.baostock__query_stock_basic`
- 表说明：BaoStock security basic-information snapshot.

## 2. 数据分析发现

基于当前 raw 表的现状分析：

- 数据量与覆盖
  - 总记录数：待补充
  - 覆盖主体数：待补充
  - 日期 / 分区范围：待补充
- 粒度与候选键
  - 观察到的粒度：待补充
  - 候选自然键去重结果：待补充
  - 旧候选键或备选键对比：待补充
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：待补充
  - 占位值：本次已画像日期/时间字段未发现 `1970-01-01` 占位值
  - 预期缺失：待补充
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：待补充
  - 直接 raw input 参照命中情况：待补充
- 分布与相关性
  - 枚举 top values：待补充
  - 少量值 / 长尾文本：待补充
  - 字段间强相关：待补充
- 时间字段合理性
  - 日期范围：待补充
  - 日期先后关系异常：待补充
  - 批次时间范围：待补充
- 数值字段合理性
  - 负数 / 零值 / 极端值：待补充
  - 单位判断：待补充
- 其他观察
  - 对 staging 设计有影响、但不应在 staging 静默修正的事实：待补充

## 3. 粒度与键

- 观察到的粒度：待补充
- 候选自然键：待补充
- 重复检查：待补充
- 粒度注意事项：待补充

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| code | String | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `code`。 原始字段说明：BaoStock 基础信息接口返回的证券代码。 |
| code_name | String | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `code_name`。 原始字段说明：BaoStock 基础信息接口返回的证券简称。 |
| ipoDate | Date | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `ipoDate`。 原始字段说明：证券上市日期。 |
| outDate | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `outDate`。 原始字段说明：证券退市日期；未退市时通常为空。 |
| type | Int8 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `type`。 原始字段说明：证券类型代码。 |
| status | Int8 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `status`。 原始字段说明：证券上市状态。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`code`, `code_name`
- 观察到的格式：待补充
- 无效样例：待补充
- 建议 staging 处理：待补充

### 日期与时间字段

- 已画像字段：`ipoDate`, `outDate`
- 范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行
- 无效值或占位值：本次已画像日期/时间字段未发现 `1970-01-01` 占位值
- 建议 staging 处理：待补充

### 枚举字段

- 已画像字段：`type`, `status`
- 取值：待补充
- 未知或异常取值：待补充
- 建议 staging 处理：待补充

### 数值字段

- 已画像字段：`type`, `status`
- 最小/最大值：待补充
- 负数/零值/极端值：待补充
- 单位假设：待补充
- 建议 staging 处理：待补充

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 未发现需要 staging 静默修正的数据质量问题 | 低 | 基础 profiling 未发现日期占位值问题 | 仅做确定性重命名、类型保留和格式标准化 | 业务口径判断延后 |

## 7. Staging 设计决策

- 重命名：待补充
- 类型转换：待补充
- 标准化：待补充
- NULL 处理：待补充
- 测试：待补充
- YAML 元数据：待补充

## 8. 延后到 Intermediate/Mart

- 跨源 join：待补充
- 需要优先级判断的去重：待补充
- 主数据修正：待补充
- 粒度变化：待补充
- 业务指标逻辑：待补充

## 待确认问题

- [ ] 确认画像发现，并在依赖该报告开展新 staging 工作前更新报告状态。

## 关键 SQL 证据摘要

- 行数：待补充
- 日期 / 分区范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行
- 候选键重复：待补充
- 关键 NULL / 占位值：本次已画像日期/时间字段未发现 `1970-01-01` 占位值
- 枚举 / 文本分布：待补充
- 数值范围：待补充

## 9. 验收清单

- [ ] 已抽样 raw source。
- [ ] 已记录行数和日期/分区范围。
- [ ] 已评估粒度和候选键。
- [ ] 已完成关键字段画像。
- [ ] 已列出 staging 转换建议。
- [ ] 已列出延后处理事项。
- [ ] 已提出测试或明确豁免。

## Profiling SQL 与结果

### 样例行

```sql
select *
from {{ source('raw', 'baostock__query_stock_basic') }}
```


结果（成功）：

```text
21:23:13  Running with dbt=1.11.11
21:23:14  Registered adapter: clickhouse=1.10.0
21:23:14  Unable to do partial parsing because profile has changed
21:23:17  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:23:17  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:17
21:23:17  Concurrency: 1 threads (target='dev')
21:23:17
Previewing inline node:
| code      | code_name   |    ipoDate | outDate | type | status |
| --------- | ----------- | ---------- | ------- | ---- | ------ |
| sh.000001 | 上证综合指数      | 1991-07-15 |         |    2 |      1 |
| sh.000002 | 上证A股指数      | 1992-02-21 |         |    2 |      1 |
| sh.000003 | 上证B股指数      | 1992-08-17 |         |    2 |      1 |
| sh.000004 | 上证工业类指数     | 1993-05-03 |         |    2 |      1 |
| sh.000005 | 上证商业类指数     | 1993-05-03 |         |    2 |      1 |
| sh.000006 | 上证房地产指数     | 1993-05-03 |         |    2 |      1 |
| sh.000007 | 上证公用事业指数    | 1993-05-03 |         |    2 |      1 |
| sh.000008 | 上证综合业类指数    | 1993-05-03 |         |    2 |      1 |
| sh.000009 | 上证380       | 2010-11-29 |         |    2 |      1 |
| sh.000010 | 上证180指数     | 2002-07-01 |         |    2 |      1 |
| sh.000011 | 上证基金指数      | 2000-06-09 |         |    2 |      1 |
| sh.000012 | 上证国债指数      | 2003-01-02 |         |    2 |      1 |
| sh.000013 | 上证企业债指数     | 2003-06-09 |         |    2 |      1 |
| sh.000015 | 上证红利指数      | 2005-01-04 |         |    2 |      1 |
| sh.000016 | 上证50指数      | 2004-01-02 |         |    2 |      1 |
| sh.000017 | 新上证综指       | 2006-01-04 |         |    2 |      1 |
| sh.000018 | 上证180金融股指数  | 2007-12-10 |         |    2 |      1 |
| sh.000019 | 上证公司治理指数    | 2008-01-02 |         |    2 |      1 |
| sh.000020 | 上证中型企业综合指数  | 2008-05-12 |         |    2 |      1 |
| sh.000021 | 上证180公司治理指数 | 2008-09-10 |         |    2 |      1 |
| sh.000022 | 沪公司债        | 2008-11-19 |         |    2 |      1 |
| sh.000025 | 上证180基建指数   | 2008-12-15 |         |    2 |      1 |
| sh.000026 | 上证180资源指数   | 2008-12-15 |         |    2 |      1 |
| sh.000027 | 上证180交通运输指数 | 2008-12-15 |         |    2 |      1 |
| sh.000028 | 上证180成长指数   | 2009-01-09 |         |    2 |      1 |
| sh.000029 | 上证180价值指数   | 2009-01-09 |         |    2 |      1 |
| sh.000030 | 上证180相对成长指数 | 2009-01-09 |         |    2 |      1 |
| sh.000031 | 上证180相对价值指数 | 2009-01-09 |         |    2 |      1 |
| sh.000032 | 上证能源行业指数    | 2009-01-09 |         |    2 |      1 |
| sh.000033 | 上证原材料行业指数   | 2009-01-09 |         |    2 |      1 |
| sh.000034 | 上证工业行业指数    | 2009-01-09 |         |    2 |      1 |
| sh.000035 | 上证可选消费行业指数  | 2009-01-09 |         |    2 |      1 |
| sh.000036 | 上证主要消费行业指数  | 2009-01-09 |         |    2 |      1 |
| sh.000037 | 上证医药卫生行业指数  | 2009-01-09 |         |    2 |      1 |
| sh.000038 | 上证金融地产行业指数  | 2009-01-09 |         |    2 |      1 |
| sh.000039 | 上证信息技术行业指数  | 2009-01-09 |         |    2 |      1 |
| sh.000040 | 上证电信业务行业指数  | 2009-01-09 |         |    2 |      1 |
| sh.000041 | 上证公用事业行业指数  | 2009-01-09 |         |    2 |      1 |
| sh.000042 | 上证中央企业50指数  | 2009-03-30 |         |    2 |      1 |
| sh.000043 | 上证超级大盘指数    | 2009-04-23 |         |    2 |      1 |
| sh.000044 | 上证中盘        | 2009-07-03 |         |    2 |      1 |
| sh.000045 | 上证小盘        | 2009-07-03 |         |    2 |      1 |
| sh.000046 | 上证中小        | 2009-07-03 |         |    2 |      1 |
| sh.000047 | 上证全指        | 2009-07-03 |         |    2 |      1 |
| sh.000048 | 责任指数        | 2009-08-05 |         |    2 |      1 |
| sh.000049 | 上证民企        | 2009-08-25 |         |    2 |      1 |
| sh.000050 | 50等权        | 2011-01-04 |         |    2 |      1 |
| sh.000051 | 180等权       | 2011-05-24 |         |    2 |      1 |
| sh.000052 | 50基本        | 2012-01-09 |         |    2 |      1 |
| sh.000053 | 180基本       | 2012-01-09 |         |    2 |      1 |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'baostock__query_stock_basic') }}
```


结果（成功）：

```text
21:23:21  Running with dbt=1.11.11
21:23:21  Registered adapter: clickhouse=1.10.0
21:23:21  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:23:22  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:22
21:23:22  Concurrency: 1 threads (target='dev')
21:23:22
Previewing inline node:
| row_count |
| --------- |
|      8769 |
```

### 日期范围

```sql
select
    min(`ipoDate`) as min_ipodate,
    max(`ipoDate`) as max_ipodate,
    countIf(isNull(`ipoDate`)) as null_ipodate,
    countIf(`ipoDate` = toDate('1970-01-01')) as placeholder_ipodate,
    min(`outDate`) as min_outdate,
    max(`outDate`) as max_outdate,
    countIf(isNull(`outDate`)) as null_outdate,
    countIf(`outDate` = toDate('1970-01-01')) as placeholder_outdate
from {{ source('raw', 'baostock__query_stock_basic') }}
```


结果（成功）：

```text
21:23:25  Running with dbt=1.11.11
21:23:25  Registered adapter: clickhouse=1.10.0
21:23:26  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:23:26  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:26
21:23:26  Concurrency: 1 threads (target='dev')
21:23:26
Previewing inline node:
| min_ipodate | max_ipodate | null_ipodate | placeholder_ipodate | min_outdate | max_outdate | ... |
| ----------- | ----------- | ------------ | ------------------- | ----------- | ----------- | --- |
|  1990-12-10 |  2026-06-01 |            0 |                   0 |  1995-12-31 |  2026-06-24 | ... |
```

### 格式分布：code

```sql
select
    countIf(match(toString(`code`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`code`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`code`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`code`) or toString(`code`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'baostock__query_stock_basic') }}
```


结果（成功）：

```text
21:23:30  Running with dbt=1.11.11
21:23:30  Registered adapter: clickhouse=1.10.0
21:23:30  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:23:31  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:31
21:23:31  Concurrency: 1 threads (target='dev')
21:23:31
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |          8769 |            0 |             0 |      8769 |
```

### 格式分布：code_name

```sql
select
    countIf(match(toString(`code_name`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`code_name`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`code_name`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`code_name`) or toString(`code_name`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'baostock__query_stock_basic') }}
```


结果（成功）：

```text
21:23:34  Running with dbt=1.11.11
21:23:34  Registered adapter: clickhouse=1.10.0
21:23:35  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:23:35  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:35
21:23:35  Concurrency: 1 threads (target='dev')
21:23:35
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |      8769 |
```

### 高频取值：type

```sql
select
    `type` as value,
    count(*) as row_count
from {{ source('raw', 'baostock__query_stock_basic') }}
group by `type`
order by row_count desc
```


结果（成功）：

```text
21:23:39  Running with dbt=1.11.11
21:23:39  Registered adapter: clickhouse=1.10.0
21:23:39  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:23:40  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:40
21:23:40  Concurrency: 1 threads (target='dev')
21:23:40
Previewing inline node:
| value | row_count |
| ----- | --------- |
|     1 |      5532 |
|     5 |      1544 |
|     4 |      1097 |
|     2 |       596 |
```

### 高频取值：status

```sql
select
    `status` as value,
    count(*) as row_count
from {{ source('raw', 'baostock__query_stock_basic') }}
group by `status`
order by row_count desc
```


结果（成功）：

```text
21:23:43  Running with dbt=1.11.11
21:23:43  Registered adapter: clickhouse=1.10.0
21:23:44  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:23:44  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:44
21:23:44  Concurrency: 1 threads (target='dev')
21:23:44
Previewing inline node:
| value | row_count |
| ----- | --------- |
|     1 |      7644 |
|     0 |      1125 |
```

### 数值范围：type

```sql
select
    min(`type`) as min_value,
    max(`type`) as max_value,
    countIf(`type` = 0) as zero_count,
    countIf(`type` < 0) as negative_count,
    countIf(isNull(`type`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'baostock__query_stock_basic') }}
```


结果（成功）：

```text
21:23:48  Running with dbt=1.11.11
21:23:48  Registered adapter: clickhouse=1.10.0
21:23:48  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:23:49  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:49
21:23:49  Concurrency: 1 threads (target='dev')
21:23:49
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         1 |         5 |          0 |              0 |          0 |      8769 |
```

### 数值范围：status

```sql
select
    min(`status`) as min_value,
    max(`status`) as max_value,
    countIf(`status` = 0) as zero_count,
    countIf(`status` < 0) as negative_count,
    countIf(isNull(`status`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'baostock__query_stock_basic') }}
```


结果（成功）：

```text
21:23:52  Running with dbt=1.11.11
21:23:52  Registered adapter: clickhouse=1.10.0
21:23:53  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:23:53  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:23:53
21:23:53  Concurrency: 1 threads (target='dev')
21:23:53
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         0 |         1 |       1125 |              0 |          0 |      8769 |
```
