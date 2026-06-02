# Raw 数据画像：baostock__query_stock_basic

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/baostock__query_stock_basic.yml`
- dbt source：`source('raw', 'baostock__query_stock_basic')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/baostock/stg_baostock__query_stock_basic.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`baostock__query_stock_basic`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table baostock__query_stock_basic --execute --output ../docs/references/raw_profile/baostock__query_stock_basic.md`，并补充 ClickHouse 结构化汇总查询
- 行数：8,769
- 数据范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行；`outDate`: 1970-01-01 至 2026-06-24，NULL 0 行，`1970-01-01` 占位 7644 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`baostock__query_stock_basic`
- ClickHouse raw 表：`fleur_raw.baostock__query_stock_basic`
- 表说明：BaoStock security basic-information snapshot.

## 2. 粒度与键

- 观察到的粒度：候选自然键为 `code`，本次 profiling 未发现重复。
- 候选自然键：`code`
- 重复检查：未发现重复
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；如果候选键重复，需要在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| code | String | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | BaoStock 基础信息接口返回的证券代码。 |
| code_name | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | BaoStock 基础信息接口返回的证券简称。 |
| ipoDate | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 证券上市日期。 |
| outDate | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 证券退市日期；未退市时通常为空。 |
| type | Int8 | 见关键字段画像 | 未逐列统计 | 见关键字段画像 | 证券类型代码。 |
| status | Int8 | 见关键字段画像 | 未逐列统计 | 见关键字段画像 | 证券上市状态。 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`code`
- 观察到的格式：`code`: canonical 后缀 0/8769，供应商前缀 8769/8769，纯数字 0/8769，空值 0/8769
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：BaoStock `sh.`/`sz.` 前缀格式必须用 normalization macro 转成 `000001.SZ` 格式；不要在 staging 做主数据修正。

### 日期与时间字段

- 已画像字段：`ipoDate`, `outDate`
- 范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行；`outDate`: 1970-01-01 至 2026-06-24，NULL 0 行，`1970-01-01` 占位 7644 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`type`, `status`
- 取值：`type`: `1`(5532), `5`(1544), `4`(1097), `2`(596)；`status`: `1`(7644), `0`(1125)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`type`, `status`
- 最小/最大值：`type` min=1, max=5, zero=0, negative=0, NULL=0；`status` min=0, max=1, zero=1125, negative=0, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `outDate` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 7644 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `code` 使用供应商前缀格式 | 中 | 8769/8769 行 | 使用证券代码 normalization macro 转成 `000001.SZ` 格式 | 无 |

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

- `select count() from fleur_raw.baostock__query_stock_basic`：8,769
- 日期字段范围：`ipoDate`: 1990-12-10 至 2026-06-01，NULL 0 行；`outDate`: 1970-01-01 至 2026-06-24，NULL 0 行，`1970-01-01` 占位 7644 行
- 证券代码格式：`code`: canonical 后缀 0/8769，供应商前缀 8769/8769，纯数字 0/8769，空值 0/8769
- 候选键重复：未发现重复
- 枚举 top values：`type`: `1`(5532), `5`(1544), `4`(1097), `2`(596)；`status`: `1`(7644), `0`(1125)
- 数值范围摘要：`type` min=1, max=5, zero=0, negative=0, NULL=0；`status` min=0, max=1, zero=1125, negative=0, NULL=0
