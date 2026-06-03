# Raw 数据画像：jiuyan__action_field_compacted

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/jiuyan__action_field_compacted.yml`
- dbt source：`source('raw', 'jiuyan__action_field_compacted')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`jiuyan__action_field_compacted`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table jiuyan__action_field_compacted --execute --output ../docs/references/raw_profile/jiuyan__action_field_compacted.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`jiuyan__action_field_compacted`
- ClickHouse raw 表：`fleur_raw.jiuyan__action_field_compacted`
- 表说明：JiuYan action-field daily rows compacted into yearly raw partitions.

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
| action_field_id | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `action_field_id`。 原始字段说明：韭研题材异动记录唯一标识。 |
| name | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `name`。 原始字段说明：韭研题材异动名称。 |
| date | Date | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `date`。 原始字段说明：韭研题材异动对应的交易日期。 |
| reason | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `reason`。 原始字段说明：韭研题材异动形成或归类原因。 |
| sort_no | Int64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `sort_no`。 原始字段说明：韭研题材异动展示排序号。 |
| is_delete | Bool | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `is_delete`。 原始字段说明：韭研题材异动记录是否被标记为删除。 |
| delete_time | Nullable(DateTime64(3)) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `delete_time`。 原始字段说明：韭研题材异动记录删除时间。 |
| create_time | DateTime64(3) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `create_time`。 原始字段说明：韭研题材异动记录创建时间。 |
| update_time | Nullable(DateTime64(3)) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `update_time`。 原始字段说明：韭研题材异动记录更新时间。 |
| count | Int64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `count`。 原始字段说明：韭研题材异动关联对象数量。 |
| code | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `code`。 原始字段说明：题材异动关联的证券代码。 |
| time | Nullable(String) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `time`。 原始字段说明：题材异动关联证券的事件时间。 |
| num | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `num`。 原始字段说明：题材异动关联证券的连板数量描述。 |
| price | Int64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `price`。 原始字段说明：题材异动关联证券的价格数值。 |
| day | Nullable(Int64) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `day`。 原始字段说明：题材异动关联证券的连板天数。 |
| edition | Nullable(Int64) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `edition`。 原始字段说明：题材异动关联证券的连板板数。 |
| shares_range | Float64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `shares_range`。 原始字段说明：题材异动关联证券的股份区间数值。 |
| expound | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `expound`。 原始字段说明：题材异动关联证券的补充说明。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`code`
- 观察到的格式：待补充
- 无效样例：待补充
- 建议 staging 处理：待补充

### 日期与时间字段

- 已画像字段：`date`, `delete_time`, `create_time`, `update_time`
- 范围：`date`: 2026-03-04 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行
- 无效值或占位值：本次已画像日期/时间字段未发现 `1970-01-01` 占位值
- 建议 staging 处理：待补充

### 枚举字段

- 已画像字段：`is_delete`, `code`, `num`
- 取值：待补充
- 未知或异常取值：待补充
- 建议 staging 处理：待补充

### 数值字段

- 已画像字段：`sort_no`, `count`, `price`, `day`, `edition`, `shares_range`
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
- 日期 / 分区范围：`date`: 2026-03-04 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行
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
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:00  Running with dbt=1.11.11
21:33:00  Registered adapter: clickhouse=1.10.0
21:33:01  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:33:01  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:01
21:33:01  Concurrency: 1 threads (target='dev')
21:33:01
Previewing inline node:
| action_field_id      | name  |       date | reason               | sort_no | is_delete | ... |
| -------------------- | ----- | ---------- | -------------------- | ------- | --------- | --- |
| 828462bc92a9489cb... | 海菲曼   | 2026-03-04 |                      |      10 |     False | ... |
| e3b9e4aa5fc944eaa... | 亚盛集团  | 2026-03-04 | 近日农业农村部主持召开部党组会议，... |       5 |     False | ... |
| d05b8b188b8e490fb... | 廊坊发展  | 2026-03-04 |                      |       9 |     False | ... |
| 6c5a39eb47c547b68... | 桂冠电力  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| e3b9e4aa5fc944eaa... | 农发种业  | 2026-03-04 | 近日农业农村部主持召开部党组会议，... |       5 |     False | ... |
| 661ddcb40b0a438ea... | 卓郎智能  | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| 6c5a39eb47c547b68... | 保变电气  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 080e2de67ba3437db... | ST洲际  | 2026-03-04 | 1、美以袭击伊朗后，伊朗宣布关闭霍... |       3 |     False | ... |
| 6c5a39eb47c547b68... | 金开新能  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| d05b8b188b8e490fb... | 京城股份  | 2026-03-04 |                      |       9 |     False | ... |
| 080e2de67ba3437db... | 石化油服  | 2026-03-04 | 1、美以袭击伊朗后，伊朗宣布关闭霍... |       3 |     False | ... |
| 6c5a39eb47c547b68... | 宏盛华源  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 661ddcb40b0a438ea... | 福达合金  | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| 661ddcb40b0a438ea... | 海星股份  | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| 661ddcb40b0a438ea... | 超颖电子  | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| 3664652a00bd40f09... | 诚邦股份  | 2026-03-04 | 佰维存储、华邦电子等龙头2026年... |       7 |     False | ... |
| 080e2de67ba3437db... | 水发燃气  | 2026-03-04 | 1、美以袭击伊朗后，伊朗宣布关闭霍... |       3 |     False | ... |
| 6c5a39eb47c547b68... | 神马电力  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 6c5a39eb47c547b68... | 汇金通   | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 3332ed8f11f04af29... | 天创时尚  | 2026-03-04 | 中国人大会议发言人：随着人形机器人... |       4 |     False | ... |
| 6c5a39eb47c547b68... | 杭电股份  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 6c5a39eb47c547b68... | 华通线缆  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 6c5a39eb47c547b68... | 起帆电缆  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 3332ed8f11f04af29... | 王力安防  | 2026-03-04 | 中国人大会议发言人：随着人形机器人... |       4 |     False | ... |
| 4b5237d699f94fbba... | 亚虹医药  | 2026-03-04 |                      |       0 |     False | ... |
| 4b5237d699f94fbba... | 佰维存储  | 2026-03-04 |                      |       0 |     False | ... |
| 661ddcb40b0a438ea... | *ST美丽 | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| e3b9e4aa5fc944eaa... | 红太阳   | 2026-03-04 | 近日农业农村部主持召开部党组会议，... |       5 |     False | ... |
| 6c5a39eb47c547b68... | 顺钠股份  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| b8346442a4e24fec9... | 炼石航空  | 2026-03-04 |                      |      11 |     False | ... |
| b8346442a4e24fec9... | ST京蓝  | 2026-03-04 |                      |      11 |     False | ... |
| 6c5a39eb47c547b68... | 银星能源  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 661ddcb40b0a438ea... | 法尔胜   | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| 3332ed8f11f04af29... | 三联锻造  | 2026-03-04 | 中国人大会议发言人：随着人形机器人... |       4 |     False | ... |
| 3664652a00bd40f09... | 德明利   | 2026-03-04 | 佰维存储、华邦电子等龙头2026年... |       7 |     False | ... |
| d05b8b188b8e490fb... | 誉帆科技  | 2026-03-04 |                      |       9 |     False | ... |
| 6c5a39eb47c547b68... | 豫能控股  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 6c5a39eb47c547b68... | 三变科技  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| 3332ed8f11f04af29... | 悦心健康  | 2026-03-04 | 中国人大会议发言人：随着人形机器人... |       4 |     False | ... |
| d05b8b188b8e490fb... | 东方锆业  | 2026-03-04 |                      |       9 |     False | ... |
| 5333c67a935d4831b... | 延华智能  | 2026-03-04 | 智能经济是继农业经济、工业经济、数... |       6 |     False | ... |
| 080e2de67ba3437db... | *ST准油 | 2026-03-04 | 1、美以袭击伊朗后，伊朗宣布关闭霍... |       3 |     False | ... |
| d30ce59c5af54d669... | 北化股份  | 2026-03-04 | 3月4日消息，美方称已打击约200... |       8 |     False | ... |
| 661ddcb40b0a438ea... | 川润股份  | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| 6c5a39eb47c547b68... | 积成电子  | 2026-03-04 | 1、据The Informatio... |       1 |     False | ... |
| d05b8b188b8e490fb... | 富临运业  | 2026-03-04 |                      |       9 |     False | ... |
| 661ddcb40b0a438ea... | 森源电气  | 2026-03-04 | 中国2月AI调用量首超美国，四款大... |       2 |     False | ... |
| d30ce59c5af54d669... | 航天彩虹  | 2026-03-04 | 3月4日消息，美方称已打击约200... |       8 |     False | ... |
| 4b5237d699f94fbba... | 闰土股份  | 2026-03-04 |                      |       0 |     False | ... |
| 080e2de67ba3437db... | 山东墨龙  | 2026-03-04 | 1、美以袭击伊朗后，伊朗宣布关闭霍... |       3 |     False | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:05  Running with dbt=1.11.11
21:33:05  Registered adapter: clickhouse=1.10.0
21:33:05  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:33:06  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:06
21:33:06  Concurrency: 1 threads (target='dev')
21:33:06
Previewing inline node:
| row_count |
| --------- |
|      5853 |
```

### 日期范围

```sql
select
    min(`date`) as min_date,
    max(`date`) as max_date,
    countIf(isNull(`date`)) as null_date,
    countIf(`date` = toDate('1970-01-01')) as placeholder_date,
    min(`delete_time`) as min_delete_time,
    max(`delete_time`) as max_delete_time,
    countIf(isNull(`delete_time`)) as null_delete_time,
    countIf(`delete_time` = toDateTime64('1970-01-01 00:00:00', 3)) as placeholder_delete_time,
    min(`create_time`) as min_create_time,
    max(`create_time`) as max_create_time,
    countIf(isNull(`create_time`)) as null_create_time,
    countIf(`create_time` = toDateTime64('1970-01-01 00:00:00', 3)) as placeholder_create_time,
    min(`update_time`) as min_update_time,
    max(`update_time`) as max_update_time,
    countIf(isNull(`update_time`)) as null_update_time,
    countIf(`update_time` = toDateTime64('1970-01-01 00:00:00', 3)) as placeholder_update_time
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:09  Running with dbt=1.11.11
21:33:09  Registered adapter: clickhouse=1.10.0
21:33:10  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:33:10  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:10
21:33:10  Concurrency: 1 threads (target='dev')
21:33:10
Previewing inline node:
|   min_date |   max_date | null_date | placeholder_date | min_delete_time | max_delete_time | ... |
| ---------- | ---------- | --------- | ---------------- | --------------- | --------------- | --- |
| 2026-03-04 | 2026-06-01 |         0 |                0 |                 |                 | ... |
```

### 格式分布：code

```sql
select
    countIf(match(toString(`code`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`code`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`code`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`code`) or toString(`code`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:14  Running with dbt=1.11.11
21:33:14  Registered adapter: clickhouse=1.10.0
21:33:14  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:33:15  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:15
21:33:15  Concurrency: 1 threads (target='dev')
21:33:15
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |             0 |      5853 |
```

### 高频取值：is_delete

```sql
select
    `is_delete` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
group by `is_delete`
order by row_count desc
```


结果（成功）：

```text
21:33:18  Running with dbt=1.11.11
21:33:18  Registered adapter: clickhouse=1.10.0
21:33:19  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:33:19  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:19
21:33:19  Concurrency: 1 threads (target='dev')
21:33:19
Previewing inline node:
| value | row_count |
| ----- | --------- |
| False |      5853 |
```

### 高频取值：code

```sql
select
    `code` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
group by `code`
order by row_count desc
```


结果（成功）：

```text
21:33:22  Running with dbt=1.11.11
21:33:23  Registered adapter: clickhouse=1.10.0
21:33:23  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:33:24  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:24
21:33:24  Concurrency: 1 threads (target='dev')
21:33:24
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| sh600381 |        29 |
| sz000711 |        24 |
| sh600396 |        23 |
| sz002528 |        22 |
| sh603272 |        20 |
| sh600726 |        19 |
| sh603580 |        19 |
| sh603843 |        18 |
| sz000609 |        18 |
| sz002289 |        18 |
| sz001270 |        18 |
| sz002620 |        17 |
| sz002199 |        17 |
| sh603813 |        17 |
| sh603773 |        17 |
| sz000908 |        16 |
| sz000669 |        15 |
| sz002713 |        15 |
| sh600545 |        15 |
| sz002081 |        14 |
```

### 高频取值：num

```sql
select
    `num` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
group by `num`
order by row_count desc
```


结果（成功）：

```text
21:33:27  Running with dbt=1.11.11
21:33:27  Registered adapter: clickhouse=1.10.0
21:33:27  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:33:28  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:28
21:33:28  Concurrency: 1 threads (target='dev')
21:33:28
Previewing inline node:
| value | row_count |
| ----- | --------- |
|       |      4328 |
| 2天2板  |       444 |
| 4天2板  |       137 |
| 3天3板  |       134 |
| 3天2板  |       123 |
| 4天4板  |        65 |
| 5天3板  |        63 |
| 4天3板  |        56 |
| 6天3板  |        52 |
| 7天4板  |        33 |
| 8天4板  |        27 |
| 5天5板  |        27 |
| 10天5板 |        24 |
| 6天4板  |        21 |
| 5天4板  |        17 |
| 9天5板  |        16 |
| 8天5板  |        16 |
| 11天6板 |        14 |
| 6天5板  |        14 |
| 12天6板 |        14 |
```

### 数值范围：sort_no

```sql
select
    min(`sort_no`) as min_value,
    max(`sort_no`) as max_value,
    countIf(`sort_no` = 0) as zero_count,
    countIf(`sort_no` < 0) as negative_count,
    countIf(isNull(`sort_no`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:31  Running with dbt=1.11.11
21:33:32  Registered adapter: clickhouse=1.10.0
21:33:32  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:33:32  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:32
21:33:32  Concurrency: 1 threads (target='dev')
21:33:32
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         0 |        17 |        554 |              0 |          0 |      5853 |
```

### 数值范围：count

```sql
select
    min(`count`) as min_value,
    max(`count`) as max_value,
    countIf(`count` = 0) as zero_count,
    countIf(`count` < 0) as negative_count,
    countIf(isNull(`count`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:36  Running with dbt=1.11.11
21:33:36  Registered adapter: clickhouse=1.10.0
21:33:36  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:33:37  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:37
21:33:37  Concurrency: 1 threads (target='dev')
21:33:37
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         1 |        66 |          0 |              0 |          0 |      5853 |
```

### 数值范围：price

```sql
select
    min(`price`) as min_value,
    max(`price`) as max_value,
    countIf(`price` = 0) as zero_count,
    countIf(`price` < 0) as negative_count,
    countIf(isNull(`price`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:40  Running with dbt=1.11.11
21:33:41  Registered adapter: clickhouse=1.10.0
21:33:41  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:33:41  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:41
21:33:41  Concurrency: 1 threads (target='dev')
21:33:41
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|        78 |    169996 |          0 |              0 |          0 |      5853 |
```

### 数值范围：day

```sql
select
    min(`day`) as min_value,
    max(`day`) as max_value,
    countIf(`day` = 0) as zero_count,
    countIf(`day` < 0) as negative_count,
    countIf(isNull(`day`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:45  Running with dbt=1.11.11
21:33:45  Registered adapter: clickhouse=1.10.0
21:33:45  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:33:46  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:46
21:33:46  Concurrency: 1 threads (target='dev')
21:33:46
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         2 |        41 |          0 |              0 |       4328 |      5853 |
```

### 数值范围：edition

```sql
select
    min(`edition`) as min_value,
    max(`edition`) as max_value,
    countIf(`edition` = 0) as zero_count,
    countIf(`edition` < 0) as negative_count,
    countIf(isNull(`edition`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:49  Running with dbt=1.11.11
21:33:50  Registered adapter: clickhouse=1.10.0
21:33:50  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:33:50  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:50
21:33:50  Concurrency: 1 threads (target='dev')
21:33:50
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         2 |        24 |          0 |              0 |       4328 |      5853 |
```

### 数值范围：shares_range

```sql
select
    min(`shares_range`) as min_value,
    max(`shares_range`) as max_value,
    countIf(`shares_range` = 0) as zero_count,
    countIf(`shares_range` < 0) as negative_count,
    countIf(isNull(`shares_range`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__action_field_compacted') }}
```


结果（成功）：

```text
21:33:54  Running with dbt=1.11.11
21:33:54  Registered adapter: clickhouse=1.10.0
21:33:54  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:33:55  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:33:55
21:33:55  Concurrency: 1 threads (target='dev')
21:33:55
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|      -999 |   151,052 |          3 |             45 |          0 |      5853 |
```
