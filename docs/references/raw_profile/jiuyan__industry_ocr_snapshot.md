# Raw 数据画像：jiuyan__industry_ocr_snapshot

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/jiuyan__industry_ocr_snapshot.yml`
- dbt source：`source('raw', 'jiuyan__industry_ocr_snapshot')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/jiuyan/stg_jiuyan__industry_ocr_snapshot.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`jiuyan__industry_ocr_snapshot`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table jiuyan__industry_ocr_snapshot --execute --output ../docs/references/raw_profile/jiuyan__industry_ocr_snapshot.md`，并补充 ClickHouse 结构化汇总查询
- 行数：1,069
- 数据范围：无日期字段；以当前表快照为范围
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`jiuyan__industry_ocr_snapshot`
- ClickHouse raw 表：`fleur_raw.jiuyan__industry_ocr_snapshot`
- 表说明：Successful JiuYan OCR result snapshot.

## 2. 粒度与键

- 观察到的粒度：候选自然键为 `industry_id`, `image_filename`, `image_index`, `ocr_row_index`，本次 profiling 未发现重复。
- 候选自然键：`industry_id`, `image_filename`, `image_index`, `ocr_row_index`
- 重复检查：未发现重复
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；如果候选键重复，需要在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| industry_id | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 行业研究记录标识 |
| image_filename | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | OCR 来源图片文件名。 |
| image_index | Int32 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | OCR 来源图片在批次中的序号。 |
| ocr_row_index | Int32 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | OCR 结果在图片中的行序号。 |
| stock_name | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | OCR 识别出的股票名称。 |
| theme_path | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | OCR 识别出的题材或主题路径。 |
| relation | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | OCR 识别出的股票与题材关系说明。 |
| source | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | OCR 结果对应的来源文件或来源渠道。 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：无
- 观察到的格式：无证券代码字段或代码字段未识别。
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：九眼数据多为业务文本或本地代码；无法只靠本表稳定推出 canonical 证券代码时，应保留 source-local 字段并延后实体匹配。

### 日期与时间字段

- 已画像字段：无
- 范围：无 Date/DateTime 类型字段
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`stock_name`, `source`
- 取值：`stock_name`: `三孚新科`(7), `天准科技`(6), `中恒电气`(5), `特变电工`(5), `伊戈尔`(4), `大族数控`(4), `华工科技`(4), `长电科技`(4)；`source`: ``(535), `互动`(220), `公告`(67), `调研`(38), `年报`(36), `图片`(34), `半年报`(16), `研报`(12)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`image_index`, `ocr_row_index`
- 最小/最大值：`image_index` min=0, max=1, zero=979, negative=0, NULL=0；`ocr_row_index` min=0, max=171, zero=30, negative=0, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 未发现需要 staging 静默修正的数据质量问题 | 低 | 基础 profiling 未发现 key 重复以外的阻断项 | 仅做确定性重命名、类型保留和格式标准化 | 业务口径判断延后 |

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

- `select count() from fleur_raw.jiuyan__industry_ocr_snapshot`：1,069
- 日期字段范围：无 Date/DateTime 类型字段
- 证券代码格式：无证券代码字段或代码字段未识别。
- 候选键重复：未发现重复
- 枚举 top values：`stock_name`: `三孚新科`(7), `天准科技`(6), `中恒电气`(5), `特变电工`(5), `伊戈尔`(4), `大族数控`(4), `华工科技`(4), `长电科技`(4)；`source`: ``(535), `互动`(220), `公告`(67), `调研`(38), `年报`(36), `图片`(34), `半年报`(16), `研报`(12)
- 数值范围摘要：`image_index` min=0, max=1, zero=979, negative=0, NULL=0；`ocr_row_index` min=0, max=171, zero=30, negative=0, NULL=0
