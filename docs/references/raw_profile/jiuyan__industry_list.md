# Raw 数据画像：jiuyan__industry_list

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/jiuyan__industry_list.yml`
- dbt source：`source('raw', 'jiuyan__industry_list')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/jiuyan/stg_jiuyan__industry_list.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`jiuyan__industry_list`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table jiuyan__industry_list --execute --output ../docs/references/raw_profile/jiuyan__industry_list.md`，并补充 ClickHouse 结构化汇总查询
- 行数：956
- 数据范围：`delete_time`: 1970-01-01 00:00:00 至 1970-01-01 00:00:00，NULL 0 行，`1970-01-01` 占位 956 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`jiuyan__industry_list`
- ClickHouse raw 表：`fleur_raw.jiuyan__industry_list`
- 表说明：JiuYan industry research list snapshot.

## 2. 粒度与键

- 观察到的粒度：候选自然键为 `industry_id`，本次 profiling 未发现重复。
- 候选自然键：`industry_id`
- 重复检查：未发现重复
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；如果候选键重复，需要在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| industry_id | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 韭研行业研究记录唯一标识。 |
| title_red | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 行业研究标题是否红色高亮展示。 |
| title_bold | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 行业研究标题是否加粗展示。 |
| title | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究标题。 |
| author | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 行业研究内容作者。 |
| imgs | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容关联图片列表。 |
| keyword | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容关键词。 |
| content | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究正文内容。 |
| is_top | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 行业研究内容是否置顶。 |
| status | Int64 | 见关键字段画像 | 未逐列统计 | 见关键字段画像 | 行业研究内容发布状态。 |
| sort_no | Int64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容展示排序号。 |
| forward_count | Int64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容转发次数。 |
| browsers_count | Int64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容浏览次数。 |
| is_delete | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 行业研究内容是否被标记为删除。 |
| delete_time | DateTime64(3) | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容删除时间。 |
| create_time | DateTime64(3) | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容创建时间。 |
| update_time | DateTime64(3) | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究内容更新时间。 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：无
- 观察到的格式：无证券代码字段或代码字段未识别。
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：九眼数据多为业务文本或本地代码；无法只靠本表稳定推出 canonical 证券代码时，应保留 source-local 字段并延后实体匹配。

### 日期与时间字段

- 已画像字段：`delete_time`, `create_time`, `update_time`
- 范围：`delete_time`: 1970-01-01 00:00:00 至 1970-01-01 00:00:00，NULL 0 行，`1970-01-01` 占位 956 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`title_red`, `title_bold`, `author`, `is_top`, `status`, `is_delete`
- 取值：`title_red`: `False`(946), `True`(10)；`title_bold`: `False`(956)；`author`: ``(756), `网络用户`(57), `用户贡献`(18), `用户`(6), `题材图谱小集`(6), `糖葫芦趁热吃`(5), `概念百科`(4), `公社用户`(4)；`is_top`: `False`(955), `True`(1)；`status`: `0`(956)；`is_delete`: `False`(956)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`sort_no`, `status`, `forward_count`, `browsers_count`
- 最小/最大值：`sort_no` min=-1, max=20, zero=935, negative=2, NULL=0；`status` min=0, max=0, zero=956, negative=0, NULL=0；`forward_count` min=0, max=568, zero=148, negative=0, NULL=0；`browsers_count` min=35, max=163974, zero=0, negative=0, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `delete_time` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 956 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `sort_no` 存在负值 | 低 | 2 行 | 不在 staging 中过滤；保留并按财务/行情语义解释 | 指标口径解释放到具体业务模型 |

## 6. 建议的 Staging 转换

- 重命名：按 `pipeline/elt/metadata/field_glossary.yml` 选择 canonical 字段；不要仅凭 raw 字段名自动扩展全部宽表字段。
- 类型转换：raw Date/Bool/Float/Int 类型已由 ClickHouse schema 承载；字符串日期或占位日期需要显式处理。
- 标准化：证券代码、交易所、本地代码使用项目 macro；文本清洗限于 trim/nullif 等 source-local 规则。
- NULL 处理：空字符串、`1970-01-01` 和明显缺失值可转 NULL，但必须在 YAML `config.meta.normalization` 记录。
- 测试：候选键字段、日期字段和 canonical security_code 应优先加 `not_null`/format tests；非封闭业务文本不加 accepted-values。
- YAML 元数据：每个 staging 输出字段必须记录 `config.meta.source_columns`；派生字段记录 normalization 来源和输入格式。

## 7. 延后到 Intermediate/Mart

- 跨源 join：证券主数据、行业/题材实体匹配、财务 statement 合并均延后。
- 需要优先级判断的去重：候选键重复或多公告版本选择不在 staging 静默处理。
- 主数据修正：证券代码历史、上市/退市状态、交易所归属修正延后。
- 粒度变化：财报宽表拆长表、事件合并、题材归并和行情事实组装延后。
- 业务指标逻辑：财务科目重算、同比/环比口径、限售/分红状态解释延后。

## 8. 待确认问题

- [ ] 具体 staging model 落地时，针对实际暴露字段补充更细的字段级 tests 和单位 metadata。
- [ ] 对候选键存在重复的表，确认是否需要保留 source-local 行版本字段或推迟到 intermediate 去重。

## 9. 验收清单

- [x] 已抽样 raw source。
- [x] 已记录行数和日期/分区范围。
- [x] 已评估粒度和候选键。
- [x] 已完成关键字段画像。
- [x] 已列出 staging 转换建议。
- [x] 已列出延后处理事项。
- [x] 已提出测试或明确豁免。

## Profiling SQL 与结果摘要

- `select count() from fleur_raw.jiuyan__industry_list`：956
- 日期字段范围：`delete_time`: 1970-01-01 00:00:00 至 1970-01-01 00:00:00，NULL 0 行，`1970-01-01` 占位 956 行；`create_time`: 2024-03-16 21:08:41 至 2026-05-29 11:57:31，NULL 0 行；`update_time`: 2026-05-07 18:34:17 至 2026-06-02 03:23:55，NULL 0 行
- 证券代码格式：无证券代码字段或代码字段未识别。
- 候选键重复：未发现重复
- 枚举 top values：`title_red`: `False`(946), `True`(10)；`title_bold`: `False`(956)；`author`: ``(756), `网络用户`(57), `用户贡献`(18), `用户`(6), `题材图谱小集`(6), `糖葫芦趁热吃`(5), `概念百科`(4), `公社用户`(4)；`is_top`: `False`(955), `True`(1)；`status`: `0`(956)；`is_delete`: `False`(956)
- 数值范围摘要：`sort_no` min=-1, max=20, zero=935, negative=2, NULL=0；`status` min=0, max=0, zero=956, negative=0, NULL=0；`forward_count` min=0, max=568, zero=148, negative=0, NULL=0；`browsers_count` min=35, max=163974, zero=0, negative=0, NULL=0
