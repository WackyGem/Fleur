# Raw 数据画像：jiuyan__action_field_compacted

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/jiuyan__action_field_compacted.yml`
- dbt source：`source('raw', 'jiuyan__action_field_compacted')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/jiuyan/stg_jiuyan__action_field_compacted.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`jiuyan__action_field_compacted`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table jiuyan__action_field_compacted --execute --status Accepted --output ../docs/references/raw_profile/jiuyan__action_field_compacted.md`
- 行数：5,853
- 数据范围：`date`: 2026-03-04 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`delete_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`create_time`: 2026-03-04 16:39:08 至 2026-06-01 15:31:59，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`time`: 1970-01-01 09:25:00.000 至 1970-01-01 15:00:01.000，NULL 938 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`jiuyan__action_field_compacted`
- ClickHouse raw 表：`fleur_raw.jiuyan__action_field_compacted`
- 表说明：JiuYan action-field daily rows compacted into yearly raw partitions.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：5,853。
  - 覆盖主体数：`code` 2,091 个；`action_field_id` 733 个
  - 日期 / 分区范围：`date`: 2026-03-04 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`delete_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`create_time`: 2026-03-04 16:39:08 至 2026-06-01 15:31:59，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`time`: 1970-01-01 09:25:00.000 至 1970-01-01 15:00:01.000，NULL 938 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `action_field_id`, `code`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`action_field_id` NULL 0 行；`code` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`code`: canonical 后缀 0/5,853，供应商前缀 0/5,853，纯数字 0/5,853，空值 0/5,853
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`is_delete`: `0`(5,853)；`num`: `NULL`(4,328), `2天2板`(444), `4天2板`(137), `3天3板`(134), `3天2板`(123), `4天4板`(65), `5天3板`(63), `4天3板`(56)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`date`: 2026-03-04 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`delete_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`create_time`: 2026-03-04 16:39:08 至 2026-06-01 15:31:59，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`time`: 1970-01-01 09:25:00.000 至 1970-01-01 15:00:01.000，NULL 938 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 6 个数值字段执行 min/max、NULL、零值和负值检查；其中 1 个字段出现负值，2 个字段出现零值，0 个字段 NULL 数不低于 80%。 负值字段样例：`shares_range` 45 行(min=-999)。
  - 单位判断：本报告保留 raw 字段单位；金额、股数、比例和价格单位必须在具体 staging YAML metadata 中记录。
- 其他观察
  - 对 staging 有影响的事实只限确定性格式、类型、NULL/占位和候选键；跨源主数据修正、业务口径和去重优先级不进入 staging。

## 3. 粒度与键

- 观察到的粒度：`action_field_id`, `code`。
- 候选自然键：`action_field_id`, `code`。
- 重复检查：未发现重复。
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；候选键重复时保留 source-local 行并把版本选择延后。

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| action_field_id | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 733 | 韭研题材异动记录唯一标识。 |
| name | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 2,091 | 韭研题材异动名称。 |
| date | Date | 0 | `1970-01-01` 0 | 2026-03-04 至 2026-06-01; distinct 60 | 韭研题材异动对应的交易日期。 |
| reason | String | 0 | 空字符串 2,276；`1970-01-01` 0 | distinct 330 | 韭研题材异动形成或归类原因。 |
| sort_no | Int64 | 0 | 零值 554；负值 0 | min=0, max=17, distinct 18 | 韭研题材异动展示排序号。 |
| is_delete | Bool | 0 | 零值 5,853 | min=0, max=0, distinct 1 | 韭研题材异动记录是否被标记为删除。 |
| delete_time | Nullable(DateTime) | 5,853 | `1970-01-01` 0 | NULL 至 NULL; distinct 0 | 韭研题材异动记录删除时间。 |
| create_time | DateTime | 0 | `1970-01-01` 0 | 2026-03-04 16:39:08 至 2026-06-01 15:31:59; distinct 194 | 韭研题材异动记录创建时间。 |
| update_time | Nullable(DateTime) | 5,853 | `1970-01-01` 0 | NULL 至 NULL; distinct 0 | 韭研题材异动记录更新时间。 |
| count | Int64 | 0 | 零值 0；负值 0 | min=1, max=66, distinct 37 | 韭研题材异动关联对象数量。 |
| code | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 2,091 | 题材异动关联的证券代码。 |
| time | Nullable(Time) | 938 | 空字符串 0；`1970-01-01` 0 | 1970-01-01 09:25:00.000 至 1970-01-01 15:00:01.000; distinct 3,441 | 题材异动关联证券的事件时间。 |
| num | LowCardinality(Nullable(String)) | 4,328 | 空字符串 0；`1970-01-01` 0 | distinct 98 | 题材异动关联证券的连板数量描述。 |
| price | Int64 | 0 | 零值 0；负值 0 | min=78, max=169,996, distinct 3,452 | 题材异动关联证券的价格数值。 |
| day | Nullable(Int64) | 4,328 | 零值 0；负值 0 | min=2, max=41, distinct 34 | 题材异动关联证券的连板天数。 |
| edition | Nullable(Int64) | 4,328 | 零值 0；负值 0 | min=2, max=24, distinct 23 | 题材异动关联证券的连板板数。 |
| shares_range | Float64 | 0 | 零值 3；负值 45 | min=-999, max=151,052, distinct 731 | 题材异动关联证券的股份区间数值。 |
| expound | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 4,241 | 题材异动关联证券的补充说明。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`code`
- 观察到的格式：`code`: canonical 后缀 0/5,853，供应商前缀 0/5,853，纯数字 0/5,853，空值 0/5,853
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`date`, `delete_time`, `create_time`, `update_time`, `time`
- 范围：`date`: 2026-03-04 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`delete_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`create_time`: 2026-03-04 16:39:08 至 2026-06-01 15:31:59，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`time`: 1970-01-01 09:25:00.000 至 1970-01-01 15:00:01.000，NULL 938 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`is_delete`, `num`
- 取值：`is_delete`: `0`(5,853)；`num`: `NULL`(4,328), `2天2板`(444), `4天2板`(137), `3天3板`(134), `3天2板`(123), `4天4板`(65), `5天3板`(63), `4天3板`(56)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 6 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 6 个数值字段执行 min/max、NULL、零值和负值检查；其中 1 个字段出现负值，2 个字段出现零值，0 个字段 NULL 数不低于 80%。 负值字段样例：`shares_range` 45 行(min=-999)。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 未发现需要 staging 静默修正的数据质量问题 | 低 | 已执行 row count、候选键、格式、日期、枚举和全字段基础画像 | staging 只做确定性重命名、类型保留和轻量标准化 | 业务解释延后 |

## 7. Staging 设计决策

- 重命名：按 `pipeline/elt/metadata/field_glossary.yml` 选择 canonical 字段；不要仅凭 raw 字段名自动扩展全部宽表字段。
- 类型转换：Date/DateTime/Bool/Float/Int 保持或显式 cast；字符串日期、报告期和供应商布尔/状态字段需在 staging SQL 中记录转换。
- 标准化：证券代码、交易所、本地代码使用项目 macro；文本清洗限于 trim/nullif 等 source-local 规则。
- NULL 处理：空字符串、`1970-01-01` 和明确缺失值可转 NULL，但必须在 YAML `config.meta.normalization` 记录来源字段和规则。
- 测试：候选键字段、日期字段和 canonical security code 优先加 `not_null`/格式 tests；宽表指标不加低价值全字段 `not_null`。
- YAML 元数据：每个 staging 输出字段必须记录 `config.meta.source_columns`；派生字段记录 `derived_from` 和 normalization metadata。

## 8. 延后到 Intermediate/Mart

- 跨源 join：证券主数据、行业/题材实体匹配、财务 statement 合并均延后。
- 需要优先级判断的去重：候选键重复或多公告版本选择不在 staging 静默处理。
- 主数据修正：证券代码历史、上市/退市状态、交易所归属修正延后。
- 粒度变化：财报宽表拆长表、事件合并、题材归并和行情事实组装延后。
- 业务指标逻辑：财务科目重算、同比/环比口径、分红状态解释和复杂文本归一化延后。

## 待确认问题

- [ ] 具体 staging model 落地时，针对实际暴露字段补充更细的字段级 tests 和单位 metadata。
- [ ] 如候选键重复或事件日期顺序需要业务解释，在 intermediate/mart 设计中确认去重优先级和时间线规则。

## 关键 SQL 证据摘要

- 行数：5,853。
- 日期 / 分区范围：`date`: 2026-03-04 至 2026-06-01，NULL 0 行，`1970-01-01` 占位 0 行；`delete_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`create_time`: 2026-03-04 16:39:08 至 2026-06-01 15:31:59，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: NULL 至 NULL，NULL 5,853 行，`1970-01-01` 占位 0 行；`time`: 1970-01-01 09:25:00.000 至 1970-01-01 15:00:01.000，NULL 938 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`action_field_id` NULL 0 行；`code` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`is_delete`: `0`(5,853)；`num`: `NULL`(4,328), `2天2板`(444), `4天2板`(137), `3天3板`(134), `3天2板`(123), `4天4板`(65), `5天3板`(63), `4天3板`(56)
- 数值范围：已对 6 个数值字段执行 min/max、NULL、零值和负值检查；其中 1 个字段出现负值，2 个字段出现零值，0 个字段 NULL 数不低于 80%。 负值字段样例：`shares_range` 45 行(min=-999)。

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
select `action_field_id`, `code`, `date`, `delete_time`, `create_time`, `update_time`, `time`, `is_delete`, `num` from fleur_raw.jiuyan__action_field_compacted limit 5
```

结果：

```text
[{'action_field_id': '828462bc92a9489cb211fcd060bbee54', 'code': 'bj920183', 'date': datetime.date(2026, 3, 4), 'delete_time': None, 'create_time': datetime.datetime(2026, 3, 4, 16, 39, 8), 'update_time': None, 'time': None, 'is_delete': False, 'num': None}, {'action_field_id': 'e3b9e4aa5fc944eaa2ae3813530fad93', 'code': 'sh600108', 'date': datetime.date(2026, 3, 4), 'delete_time': None, 'create_time': datetime.datetime(2026, 3, 4, 16, 39, 8), 'update_time': None, 'time': '1970-01-01 09:59:32.000', 'is_delete': False, 'num': '4天4板'}, {'action_field_id': 'd05b8b188b8e490fbd1d5c2a2a815c65', 'code': 'sh600149', 'date': datetime.date(2026, 3, 4), 'delete_time': None, 'create_time': datetime.datetime(2026, 3, 4, 16, 39, 8), 'update_time': None, 'time': '1970-01-01 14:49:06.000', 'is_delete': False, 'num': None}, {'action_field_id': '6c5a39eb47c547b68ec23873622ca955', 'code': 'sh600236', 'date': datetime.date(2026, 3, 4), 'delete_time': None, 'create_time': datetime.datetime(2026, 3, 4, 16, 39, 8), 'update_time': None, 'time': '1970-01-01 14:47:39.000', 'is_delete': False, 'num': '3天2板'}, {'action_field_id': 'e3b9e4aa5fc944eaa2ae3813530fad93', 'code': 'sh600313', 'date': datetime.date(2026, 3, 4), 'delete_time': None, 'create_time': datetime.datetime(2026, 3, 4, 16, 39, 8), 'update_time': None, 'time': '1970-01-01 10:47:56.000', 'is_delete': False, 'num': None}]
```

### 行数统计

```sql
select count() from fleur_raw.jiuyan__action_field_compacted
```

结果：

```text
[[5853]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `action_field_id`, `code`, count() as row_count from fleur_raw.jiuyan__action_field_compacted group by `action_field_id`, `code` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```

### 证券代码格式：code

```sql
select countIf(match(toString(`code`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`code`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`code`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`code`) or toString(`code`) = '') as empty_or_null, count() as row_count from fleur_raw.jiuyan__action_field_compacted
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 0, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 5853}
```
