# Raw 数据画像：jiuyan__industry_list

日期：2026-06-03

状态：Accepted

实施复核（2026-06-03）：落地 `stg_jiuyan__industry_list` 时，当前 raw snapshot 已更新为 957 行、`industry_id` 957 个且仍唯一；`create_time` 最大值为 2026-06-03 18:28:23，`update_time` 最大值为 2026-06-03 21:36:59；`delete_time` 仍全表 NULL，`is_delete` 仍全部为 false；`author` NULL 749 行、空字符串 8 行，`content` 空字符串 17 行。

关联：

- 数据契约：`pipeline/contracts/datasets/jiuyan__industry_list.yml`
- dbt source：`source('raw', 'jiuyan__industry_list')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/jiuyan/stg_jiuyan__industry_list.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`jiuyan__industry_list`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table jiuyan__industry_list --execute --status Accepted --output ../docs/references/raw_profile/jiuyan__industry_list.md`
- 行数：956
- 数据范围：`delete_time`: NULL 至 NULL，NULL 956 行，`1970-01-01` 占位 0 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`jiuyan__industry_list`
- ClickHouse raw 表：`fleur_raw.jiuyan__industry_list`
- 表说明：JiuYan industry research list snapshot.

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：956。
  - 覆盖主体数：`industry_id` 956 个
  - 日期 / 分区范围：`delete_time`: NULL 至 NULL，NULL 956 行，`1970-01-01` 占位 0 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `industry_id`。
  - 候选自然键去重结果：未发现重复。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`industry_id` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：本表无证券代码格式字段；未执行证券代码格式检查。
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`title_red`: `0`(946), `1`(10)；`title_bold`: `0`(956)；`author`: `NULL`(748), `网络用户`(57), `用户贡献`(18), ``(8), `题材图谱小集`(6), `用户`(6), `糖葫芦趁热吃`(5), `公社用户`(4)；`is_top`: `0`(955), `1`(1)；`is_delete`: `0`(956)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`delete_time`: NULL 至 NULL，NULL 956 行，`1970-01-01` 占位 0 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 4 个数值字段执行 min/max、NULL、零值和负值检查；其中 1 个字段出现负值，3 个字段出现零值，0 个字段 NULL 数不低于 80%。 负值字段样例：`sort_no` 2 行(min=-1)。
  - 单位判断：本报告保留 raw 字段单位；金额、股数、比例和价格单位必须在具体 staging YAML metadata 中记录。
- 其他观察
  - 对 staging 有影响的事实只限确定性格式、类型、NULL/占位和候选键；跨源主数据修正、业务口径和去重优先级不进入 staging。

## 3. 粒度与键

- 观察到的粒度：`industry_id`。
- 候选自然键：`industry_id`。
- 重复检查：未发现重复。
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；候选键重复时保留 source-local 行并把版本选择延后。

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| industry_id | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 956 | 韭研行业研究记录唯一标识。 |
| title_red | Bool | 0 | 零值 946 | min=0, max=1, distinct 2 | 行业研究标题是否红色高亮展示。 |
| title_bold | Bool | 0 | 零值 956 | min=0, max=0, distinct 1 | 行业研究标题是否加粗展示。 |
| title | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 952 | 行业研究标题。 |
| author | LowCardinality(Nullable(String)) | 748 | 空字符串 8；`1970-01-01` 0 | distinct 86 | 行业研究内容作者。 |
| imgs | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 956 | 行业研究内容关联图片列表。 |
| keyword | String | 0 | 空字符串 0；`1970-01-01` 0 | distinct 956 | 行业研究内容关键词。 |
| content | String | 0 | 空字符串 17；`1970-01-01` 0 | distinct 929 | 行业研究正文内容。 |
| is_top | Bool | 0 | 零值 955 | min=0, max=1, distinct 2 | 行业研究内容是否置顶。 |
| status | Int64 | 0 | 零值 956；负值 0 | min=0, max=0, distinct 1 | 行业研究内容发布状态。 |
| sort_no | Int64 | 0 | 零值 935；负值 2 | min=-1, max=20, distinct 20 | 行业研究内容展示排序号。 |
| forward_count | Int64 | 0 | 零值 148；负值 0 | min=0, max=568, distinct 101 | 行业研究内容转发次数。 |
| browsers_count | Int64 | 0 | 零值 0；负值 0 | min=35, max=163,974, distinct 928 | 行业研究内容浏览次数。 |
| is_delete | Bool | 0 | 零值 956 | min=0, max=0, distinct 1 | 行业研究内容是否被标记为删除。 |
| delete_time | Nullable(DateTime) | 956 | `1970-01-01` 0 | NULL 至 NULL; distinct 0 | 行业研究内容删除时间。 |
| create_time | DateTime | 0 | `1970-01-01` 0 | 2024-03-16 21:08:41 至 2026-05-29 11:57:31; distinct 956 | 行业研究内容创建时间。 |
| update_time | Nullable(DateTime) | 0 | `1970-01-01` 0 | 2026-05-07 18:34:17 至 2026-06-02 03:23:55; distinct 908 | 行业研究内容更新时间。 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：无
- 观察到的格式：本表无证券代码格式字段；未执行证券代码格式检查。
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`delete_time`, `create_time`, `update_time`
- 范围：`delete_time`: NULL 至 NULL，NULL 956 行，`1970-01-01` 占位 0 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`title_red`, `title_bold`, `author`, `is_top`, `is_delete`
- 取值：`title_red`: `0`(946), `1`(10)；`title_bold`: `0`(956)；`author`: `NULL`(748), `网络用户`(57), `用户贡献`(18), ``(8), `题材图谱小集`(6), `用户`(6), `糖葫芦趁热吃`(5), `公社用户`(4)；`is_top`: `0`(955), `1`(1)；`is_delete`: `0`(956)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 4 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 4 个数值字段执行 min/max、NULL、零值和负值检查；其中 1 个字段出现负值，3 个字段出现零值，0 个字段 NULL 数不低于 80%。 负值字段样例：`sort_no` 2 行(min=-1)。
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

- 行数：956。
- 日期 / 分区范围：`delete_time`: NULL 至 NULL，NULL 956 行，`1970-01-01` 占位 0 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行，`1970-01-01` 占位 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行，`1970-01-01` 占位 0 行
- 候选键重复：未发现重复。
- 关键 NULL / 占位值：`industry_id` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`title_red`: `0`(946), `1`(10)；`title_bold`: `0`(956)；`author`: `NULL`(748), `网络用户`(57), `用户贡献`(18), ``(8), `题材图谱小集`(6), `用户`(6), `糖葫芦趁热吃`(5), `公社用户`(4)；`is_top`: `0`(955), `1`(1)；`is_delete`: `0`(956)
- 数值范围：已对 4 个数值字段执行 min/max、NULL、零值和负值检查；其中 1 个字段出现负值，3 个字段出现零值，0 个字段 NULL 数不低于 80%。 负值字段样例：`sort_no` 2 行(min=-1)。

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
select `industry_id`, `delete_time`, `create_time`, `update_time`, `title_red`, `title_bold`, `author`, `is_top`, `is_delete` from fleur_raw.jiuyan__industry_list limit 5
```

结果：

```text
[{'industry_id': '00902f027bac4915bca4a8528859ad7f', 'delete_time': None, 'create_time': datetime.datetime(2024, 3, 19, 0, 20, 42), 'update_time': datetime.datetime(2026, 5, 28, 21, 25, 13), 'title_red': False, 'title_bold': False, 'author': None, 'is_top': False, 'is_delete': False}, {'industry_id': '0099bee786df4bb7be478d583728158f', 'delete_time': None, 'create_time': datetime.datetime(2025, 8, 14, 23, 10, 59), 'update_time': datetime.datetime(2026, 5, 31, 15, 36, 27), 'title_red': False, 'title_bold': False, 'author': None, 'is_top': False, 'is_delete': False}, {'industry_id': '00c017f5acac407cac3e80ac0e9a8b54', 'delete_time': None, 'create_time': datetime.datetime(2025, 10, 13, 19, 50, 22), 'update_time': datetime.datetime(2026, 6, 1, 23, 16, 52), 'title_red': False, 'title_bold': False, 'author': None, 'is_top': False, 'is_delete': False}, {'industry_id': '012af97c8c8c4c5ab593f17b2af2b563', 'delete_time': None, 'create_time': datetime.datetime(2026, 5, 5, 19, 23, 6), 'update_time': datetime.datetime(2026, 6, 2, 0, 44, 52), 'title_red': False, 'title_bold': False, 'author': None, 'is_top': False, 'is_delete': False}, {'industry_id': '01599b8968be427286bd74d4d24cb1b8', 'delete_time': None, 'create_time': datetime.datetime(2026, 2, 24, 11, 20, 17), 'update_time': datetime.datetime(2026, 6, 1, 19, 21, 52), 'title_red': False, 'title_bold': False, 'author': None, 'is_top': False, 'is_delete': False}]
```

### 行数统计

```sql
select count() from fleur_raw.jiuyan__industry_list
```

结果：

```text
[[956]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `industry_id`, count() as row_count from fleur_raw.jiuyan__industry_list group by `industry_id` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 0, 'max_rows_per_key': 0}
```
