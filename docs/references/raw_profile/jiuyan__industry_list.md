# Raw 数据画像：jiuyan__industry_list

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/jiuyan__industry_list.yml`
- dbt source：`source('raw', 'jiuyan__industry_list')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`jiuyan__industry_list`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table jiuyan__industry_list --execute --output ../docs/references/raw_profile/jiuyan__industry_list.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`jiuyan__industry_list`
- ClickHouse raw 表：`fleur_raw.jiuyan__industry_list`
- 表说明：JiuYan industry research list snapshot.

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
  - 占位值：待补充
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
| industry_id | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `industry_id`。 原始字段说明：韭研行业研究记录唯一标识。 |
| title_red | Bool | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `title_red`。 原始字段说明：行业研究标题是否红色高亮展示。 |
| title_bold | Bool | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `title_bold`。 原始字段说明：行业研究标题是否加粗展示。 |
| title | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `title`。 原始字段说明：行业研究标题。 |
| author | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `author`。 原始字段说明：行业研究内容作者。 |
| imgs | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `imgs`。 原始字段说明：行业研究内容关联图片列表。 |
| keyword | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `keyword`。 原始字段说明：行业研究内容关键词。 |
| content | String | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `content`。 原始字段说明：行业研究正文内容。 |
| is_top | Bool | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `is_top`。 原始字段说明：行业研究内容是否置顶。 |
| status | Int64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `status`。 原始字段说明：行业研究内容发布状态。 |
| sort_no | Int64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `sort_no`。 原始字段说明：行业研究内容展示排序号。 |
| forward_count | Int64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `forward_count`。 原始字段说明：行业研究内容转发次数。 |
| browsers_count | Int64 | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `browsers_count`。 原始字段说明：行业研究内容浏览次数。 |
| is_delete | Bool | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `is_delete`。 原始字段说明：行业研究内容是否被标记为删除。 |
| delete_time | Nullable(DateTime64(3)) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `delete_time`。 原始字段说明：行业研究内容删除时间。 |
| create_time | DateTime64(3) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `create_time`。 原始字段说明：行业研究内容创建时间。 |
| update_time | DateTime64(3) | 待补充 | 待补充 | 待补充 | 来自 `jiuyan` 原始字段 `update_time`。 原始字段说明：行业研究内容更新时间。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：待补充
- 观察到的格式：待补充
- 无效样例：待补充
- 建议 staging 处理：待补充

### 日期与时间字段

- 已画像字段：`delete_time`, `create_time`, `update_time`
- 范围：待补充
- 无效值或占位值：待补充
- 建议 staging 处理：待补充

### 枚举字段

- 已画像字段：`title_red`, `title_bold`, `author`, `is_top`, `is_delete`
- 取值：待补充
- 未知或异常取值：待补充
- 建议 staging 处理：待补充

### 数值字段

- 已画像字段：`status`, `sort_no`, `forward_count`, `browsers_count`
- 最小/最大值：待补充
- 负数/零值/极端值：待补充
- 单位假设：待补充
- 建议 staging 处理：待补充

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 待补充 | 待补充 | 待补充 | 待补充 | 待补充 |

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
- 日期 / 分区范围：待补充
- 候选键重复：待补充
- 关键 NULL / 占位值：待补充
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
from {{ source('raw', 'jiuyan__industry_list') }}
```


结果（成功）：

```text
21:33:59  Running with dbt=1.11.11
21:34:00  Registered adapter: clickhouse=1.10.0
21:34:00  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:34:00  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:00
21:34:00  Concurrency: 1 threads (target='dev')
21:34:00
Previewing inline node:
| industry_id          | title_red | title_bold | title                | author    | imgs                 | ... |
| -------------------- | --------- | ---------- | -------------------- | --------- | -------------------- | --- |
| 00902f027bac4915b... |     False |      False | 珠海国资（240130）         |           | ["https://cdn.jiu... | ... |
| 0099bee786df4bb7b... |     False |      False | 电子束光刻机“羲之”（250814）   |           | ["https://cdn.jiu... | ... |
| 00c017f5acac407ca... |     False |      False | 功率半导体（251013)        |           | ["https://cdn.jiu... | ... |
| 012af97c8c8c4c5ab... |     False |      False | 灵心巧手(260505)         |           | ["https://cdn.jiu... | ... |
| 01599b8968be42728... |     False |      False | IC载板(260224)         |           | ["https://cdn.jiu... | ... |
| 0179be73728041359... |     False |      False | 数字货币（250119）         |           | ["https://cdn.jiu... | ... |
| 01a9455da8684099a... |     False |      False | 英伟达代理(251030)        |           | ["https://cdn.jiu... | ... |
| 01de69a7f4d64ebbb... |     False |      False | 铜(240401)            |           | ["https://cdn.jiu... | ... |
| 022afbe8e81a47568... |     False |      False | 深圳AI/机器人（250303）     | 用户贡献      | ["https://cdn.jiu... | ... |
| 0352a4cc6c4242fd8... |     False |      False | 新饮品(250527)          |           | ["https://cdn.jiu... | ... |
| 036565d0b7bc4801b... |     False |      False | 国产航母(250730)         |           | ["https://cdn.jiu... | ... |
| 03e4d00376db4cb1a... |     False |      False | 超威半导体AMD(251007)     |           | ["https://cdn.jiu... | ... |
| 0465896c63024b5db... |     False |      False | 医保DRG/DIP(240723)    |           | ["https://cdn.jiu... | ... |
| 0484e87b75af4e47b... |     False |      False | 农药证件厂家(250108)       |           | ["https://cdn.jiu... | ... |
| 04ac0e0215ab4d2e9... |     False |      False | 小米眼镜(250625)         |           | ["https://cdn.jiu... | ... |
| 052d4174efdb45fa9... |     False |      False | 白银(250606)           |           | ["https://cdn.jiu... | ... |
| 05410f454d1f42cd9... |     False |      False | 财税改革                 | 超前一步      | ["https://cdn.jiu... | ... |
| 0683688c7c5b4d219... |     False |      False | 亿航智能订单量(250330)      |           | ["https://cdn.jiu... | ... |
| 06b7d785c38d47a58... |     False |      False | MR(240118)           |           | ["https://cdn.jiu... | ... |
| 06cf90cd83ef49cd9... |     False |      False | 网络安全/内容标注(260212)    |           | ["https://cdn.jiu... | ... |
| 0702f4cd4e364251a... |     False |      False | AI 医疗（250215更新）      | 网络用户 龙行龘龘 | ["https://cdn.jiu... | ... |
| 071401556f7b45c6a... |     False |      False | L3级别自动驾驶(251215)     |           | ["https://cdn.jiu... | ... |
| 079187564e854ce6a... |     False |      False | 面板(240401)           | 东呈金润      | ["https://cdn.jiu... | ... |
| 07d34db3a8264bfc8... |     False |      False | 智谱(251211)           |           | ["https://cdn.jiu... | ... |
| 07f91b5cde6e4a45a... |     False |      False | 钠离子电池(251118)        |           | ["https://cdn.jiu... | ... |
| 081f6a17d2b648519... |     False |      False | 合成生物（240427）         | 用户        | ["https://cdn.jiu... | ... |
| 0825bf9d457741b7b... |     False |      False | AIPC(240107)         |           | ["https://cdn.jiu... | ... |
| 084d33fdcecc4723b... |     False |      False | 无人物流（240722）         |           | ["https://cdn.jiu... | ... |
| 08bb3273081444b1b... |     False |      False | 第十五届全运会(251028)      |           | ["https://cdn.jiu... | ... |
| 08fee60b0b1141399... |     False |      False | 薄膜铌酸锂(240305)        |           | ["https://cdn.jiu... | ... |
| 09bca14e82354a85a... |     False |      False | 卫星资源/无线电频谱(260111)   |           | ["https://cdn.jiu... | ... |
| 09e00a8d219d476a9... |     False |      False | 博通交换机(250605)        |           | ["https://cdn.jiu... | ... |
| 09f16f17aa044b149... |     False |      False | 2025年政府工作报告利好行业及个... | 用户贡献      | ["https://cdn.jiu... | ... |
| 0a402feef560449f9... |     False |      False | AGV(240109)          |           | ["https://cdn.jiu... | ... |
| 0bf8098c6de64f16b... |     False |      False | 宁德时代钠离子电池供应链(260421) |           | ["https://cdn.jiu... | ... |
| 0bfa56b2229d49acb... |     False |      False | 爱思达航天(260105)        |           | ["https://cdn.jiu... | ... |
| 0c20c92030b64ba98... |     False |      False | 机器人皮肤/仿生皮肤(251107)   |           | ["https://cdn.jiu... | ... |
| 0c228808a67a4f909... |     False |      False | 钴金属(250623)          |           | ["https://cdn.jiu... | ... |
| 0c23df69d0db4e1e9... |     False |      False | 足球-苏超联赛、体彩(250610)   |           | ["https://cdn.jiu... | ... |
| 0c3f9169ac2e418ab... |     False |      False | AI陪伴(250111)         | 网络用户      | ["https://cdn.jiu... | ... |
| 0c8d5c53cc72425d8... |     False |      False | 隧洞设备/盾构机(250721)     |           | ["https://cdn.jiu... | ... |
| 0c9166e5319845298... |     False |      False | 光路交换机OCS(260428)更新   |           | ["https://cdn.jiu... | ... |
| 0c9fb90280284a2f8... |     False |      False | 国产芯片参股公司(250928)     |           | ["https://cdn.jiu... | ... |
| 0cb962a408464ddea... |     False |      False | 积木玩具10大(250109)      | 网络用户      | ["https://cdn.jiu... | ... |
| 0d644e78ce334fa99... |     False |      False | 创新药(250609)          |           | ["https://cdn.jiu... | ... |
| 0d7bcaeb87c942c38... |     False |      False | 造纸(250728)           |           | ["https://cdn.jiu... | ... |
| 0da665c45f0542489... |     False |      False | 甲流(250106)           |           | ["https://cdn.jiu... | ... |
| 0e38cc2976554bf39... |     False |      False | 华为AI存储(251117)更新     |           | ["https://cdn.jiu... | ... |
| 0e981981569f4cbe8... |     False |      False | 磷化工/磷酸铁锂(260325)     |           | ["https://cdn.jiu... | ... |
| 0ec85547423144449... |     False |      False | 农业种业(241216)         |           | ["https://cdn.jiu... | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
```


结果（成功）：

```text
21:34:04  Running with dbt=1.11.11
21:34:04  Registered adapter: clickhouse=1.10.0
21:34:04  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:34:05  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:05
21:34:05  Concurrency: 1 threads (target='dev')
21:34:05
Previewing inline node:
| row_count |
| --------- |
|       956 |
```

### 日期范围

```sql
select
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
from {{ source('raw', 'jiuyan__industry_list') }}
```


结果（成功）：

```text
21:34:08  Running with dbt=1.11.11
21:34:09  Registered adapter: clickhouse=1.10.0
21:34:09  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:34:09  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:09
21:34:09  Concurrency: 1 threads (target='dev')
21:34:09
Previewing inline node:
| min_delete_time | max_delete_time | null_delete_time | placeholder_delet... |     min_create_time |     max_create_time | ... |
| --------------- | --------------- | ---------------- | -------------------- | ------------------- | ------------------- | --- |
|                 |                 |              956 |                    0 | 2024-03-16 21:08:41 | 2026-05-29 11:57:31 | ... |
```

### 高频取值：title_red

```sql
select
    `title_red` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
group by `title_red`
order by row_count desc
```


结果（成功）：

```text
21:34:13  Running with dbt=1.11.11
21:34:13  Registered adapter: clickhouse=1.10.0
21:34:13  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:34:14  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:14
21:34:14  Concurrency: 1 threads (target='dev')
21:34:14
Previewing inline node:
| value | row_count |
| ----- | --------- |
| False |       946 |
|  True |        10 |
```

### 高频取值：title_bold

```sql
select
    `title_bold` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
group by `title_bold`
order by row_count desc
```


结果（成功）：

```text
21:34:17  Running with dbt=1.11.11
21:34:18  Registered adapter: clickhouse=1.10.0
21:34:18  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:34:18  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:18
21:34:18  Concurrency: 1 threads (target='dev')
21:34:18
Previewing inline node:
| value | row_count |
| ----- | --------- |
| False |       956 |
```

### 高频取值：author

```sql
select
    `author` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
group by `author`
order by row_count desc
```


结果（成功）：

```text
21:34:22  Running with dbt=1.11.11
21:34:22  Registered adapter: clickhouse=1.10.0
21:34:22  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:34:23  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:23
21:34:23  Concurrency: 1 threads (target='dev')
21:34:23
Previewing inline node:
| value  | row_count |
| ------ | --------- |
|        |       748 |
| 网络用户   |        57 |
| 用户贡献   |        18 |
|        |         8 |
| 用户     |         6 |
| 题材图谱小集 |         6 |
| 糖葫芦趁热吃 |         5 |
| 公社用户   |         4 |
| 韭之阿蒋   |         4 |
| 概念百科   |         4 |
| 超前挖掘   |         3 |
| 776    |         3 |
| 超前一步   |         3 |
| 逻辑挖掘社  |         3 |
| 场外期权研究 |         3 |
| 大侠风清扬  |         2 |
| 加油奥利给  |         2 |
| 行研屌丝   |         2 |
| 韭盈     |         2 |
| 盘前消息   |         2 |
```

### 高频取值：is_top

```sql
select
    `is_top` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
group by `is_top`
order by row_count desc
```


结果（成功）：

```text
21:34:27  Running with dbt=1.11.11
21:34:27  Registered adapter: clickhouse=1.10.0
21:34:27  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:34:28  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:28
21:34:28  Concurrency: 1 threads (target='dev')
21:34:28
Previewing inline node:
| value | row_count |
| ----- | --------- |
| False |       955 |
|  True |         1 |
```

### 高频取值：is_delete

```sql
select
    `is_delete` as value,
    count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
group by `is_delete`
order by row_count desc
```


结果（成功）：

```text
21:34:31  Running with dbt=1.11.11
21:34:31  Registered adapter: clickhouse=1.10.0
21:34:31  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:34:32  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:32
21:34:32  Concurrency: 1 threads (target='dev')
21:34:32
Previewing inline node:
| value | row_count |
| ----- | --------- |
| False |       956 |
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
from {{ source('raw', 'jiuyan__industry_list') }}
```


结果（成功）：

```text
21:34:35  Running with dbt=1.11.11
21:34:36  Registered adapter: clickhouse=1.10.0
21:34:36  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:34:36  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:36
21:34:36  Concurrency: 1 threads (target='dev')
21:34:36
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         0 |         0 |        956 |              0 |          0 |       956 |
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
from {{ source('raw', 'jiuyan__industry_list') }}
```


结果（成功）：

```text
21:34:40  Running with dbt=1.11.11
21:34:40  Registered adapter: clickhouse=1.10.0
21:34:40  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:34:41  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:41
21:34:41  Concurrency: 1 threads (target='dev')
21:34:41
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|        -1 |        20 |        935 |              2 |          0 |       956 |
```

### 数值范围：forward_count

```sql
select
    min(`forward_count`) as min_value,
    max(`forward_count`) as max_value,
    countIf(`forward_count` = 0) as zero_count,
    countIf(`forward_count` < 0) as negative_count,
    countIf(isNull(`forward_count`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
```


结果（成功）：

```text
21:34:44  Running with dbt=1.11.11
21:34:45  Registered adapter: clickhouse=1.10.0
21:34:45  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:34:45  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:45
21:34:45  Concurrency: 1 threads (target='dev')
21:34:45
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|         0 |       568 |        148 |              0 |          0 |       956 |
```

### 数值范围：browsers_count

```sql
select
    min(`browsers_count`) as min_value,
    max(`browsers_count`) as max_value,
    countIf(`browsers_count` = 0) as zero_count,
    countIf(`browsers_count` < 0) as negative_count,
    countIf(isNull(`browsers_count`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'jiuyan__industry_list') }}
```


结果（成功）：

```text
21:34:49  Running with dbt=1.11.11
21:34:49  Registered adapter: clickhouse=1.10.0
21:34:49  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:34:50  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:34:50
21:34:50  Concurrency: 1 threads (target='dev')
21:34:50
Previewing inline node:
| min_value | max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------- | ---------- | -------------- | ---------- | --------- |
|        35 |    163974 |          0 |              0 |          0 |       956 |
```
