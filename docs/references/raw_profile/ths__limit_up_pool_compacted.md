# Raw 数据画像：ths__limit_up_pool_compacted

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/ths__limit_up_pool_compacted.yml`
- dbt source：`source('raw', 'ths__limit_up_pool_compacted')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/ths/stg_ths__limit_up_pool_compacted.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`ths__limit_up_pool_compacted`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table ths__limit_up_pool_compacted --execute --output ../docs/references/raw_profile/ths__limit_up_pool_compacted.md`，并补充 ClickHouse 结构化汇总查询
- 行数：15,664
- 数据范围：`date`: 2025-05-19 至 2026-06-01，NULL 0 行；`first_limit_up_time`: 2025-05-19 01:25:00 至 2026-06-01 06:56:15，NULL 0 行；`last_limit_up_time`: 2025-05-19 01:25:00 至 2026-06-01 06:56:15，NULL 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`ths__limit_up_pool_compacted`
- ClickHouse raw 表：`fleur_raw.ths__limit_up_pool_compacted`
- 表说明：TongHuaShun limit-up pool daily rows compacted into yearly raw partitions.

## 2. 粒度与键

- 观察到的粒度：候选自然键为 `date`, `code`，本次 profiling 未发现重复。
- 候选自然键：`date`, `code`
- 重复检查：未发现重复
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；如果候选键重复，需要在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| date | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 同花顺涨停池对应的交易日期。 |
| open_num | Int64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 股票当日涨停后开板次数。 |
| first_limit_up_time | DateTime64(3, 'UTC') | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 股票当日首次涨停时间。 |
| last_limit_up_time | DateTime64(3, 'UTC') | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 股票当日最后一次涨停时间。 |
| code | LowCardinality(String) | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | 同花顺涨停池中的证券代码。 |
| limit_up_type | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 涨停类型分类。 |
| order_volume | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 涨停封单量。 |
| is_new | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 是否为当日新进入涨停池的股票。 |
| limit_up_suc_rate | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 涨停成功率。 |
| currency_value | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 股票流通市值。 |
| market_id | Int64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 同花顺市场标识。 |
| is_again_limit | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 是否再次涨停。 |
| change_rate | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 当日涨跌幅。 |
| turnover_rate | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 当日换手率。 |
| reason_type | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 涨停原因类型。 |
| order_amount | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 涨停封单金额。 |
| high_days | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 连板或高度天数文本。 |
| name | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 股票名称。 |
| high_days_value | Int64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 连板或高度天数数值。 |
| change_tag | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 涨跌幅标签。 |
| market_type | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 市场类型。 |
| latest | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 最新成交价格。 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`code`
- 观察到的格式：`code`: canonical 后缀 0/15664，供应商前缀 0/15664，纯数字 15664/15664，空值 0/15664
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：同花顺 `code` 为 6 位本地代码；staging 可保留 `security_local_code`，交易所推断需要额外依据。

### 日期与时间字段

- 已画像字段：`date`, `first_limit_up_time`, `last_limit_up_time`
- 范围：`date`: 2025-05-19 至 2026-06-01，NULL 0 行；`first_limit_up_time`: 2025-05-19 01:25:00 至 2026-06-01 06:56:15，NULL 0 行；`last_limit_up_time`: 2025-05-19 01:25:00 至 2026-06-01 06:56:15，NULL 0 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`code`, `limit_up_type`, `is_new`, `is_again_limit`, `high_days`, `name`, `change_tag`, `market_type`
- 取值：`code`: `603778`(35), `603256`(35), `605255`(31), `002951`(28), `603163`(27), `603601`(27), `601869`(26), `000070`(26)；`limit_up_type`: `换手板`(14006), `一字板`(1058), `T字板`(600)；`is_new`: `False`(15664)；`is_again_limit`: `True`(7857), `False`(7807)；`high_days`: `首板`(11015), `2天2板`(1814), `3天3板`(601), `3天2板`(376), `4天2板`(353), `4天4板`(287), `4天3板`(195), `5天3板`(176)；`name`: `宏和科技`(35), `国晟科技`(35), `天普股份`(31), `金时科技`(28), `圣晖集成`(27), `再升科技`(27), `天际股份`(26), `特发信息`(26)；`change_tag`: `LIMIT_BACK`(7857), `FIRST_LIMIT`(7807)；`market_type`: `HS`(13857), `GEM`(1333), `STAR`(474)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`open_num`, `order_volume`, `currency_value`, `change_rate`, `limit_up_suc_rate`, `market_id`, `turnover_rate`, `order_amount`, `high_days_value`, `latest`
- 最小/最大值：`open_num` min=0, max=183, zero=8604, negative=0, NULL=0；`order_volume` min=289.0, max=862627850.0, zero=0, negative=0, NULL=0；`currency_value` min=637398040.0, max=2129275300000.0, zero=0, negative=0, NULL=0；`change_rate` min=5.0157, max=20.2312, zero=0, negative=0, NULL=0；`limit_up_suc_rate` min=0.0, max=1.0, zero=1346, negative=0, NULL=0；`market_id` min=17, max=33, zero=0, negative=0, NULL=0；`turnover_rate` min=0.0354, max=77.9943, zero=0, negative=0, NULL=0；`order_amount` min=20053.71, max=5305161300.0, zero=0, negative=0, NULL=0；`high_days_value` min=65537, max=1114137, zero=0, negative=0, NULL=0；`latest` min=1.41, max=1699.96, zero=0, negative=0, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `code` 只有 6 位本地代码 | 中 | 15664/15664 行 | 仅作为 `security_local_code`；不可单独推出交易所 | 需要其他字段或主数据补齐交易所 |

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

- `select count() from fleur_raw.ths__limit_up_pool_compacted`：15,664
- 日期字段范围：`date`: 2025-05-19 至 2026-06-01，NULL 0 行；`first_limit_up_time`: 2025-05-19 01:25:00 至 2026-06-01 06:56:15，NULL 0 行；`last_limit_up_time`: 2025-05-19 01:25:00 至 2026-06-01 06:56:15，NULL 0 行
- 证券代码格式：`code`: canonical 后缀 0/15664，供应商前缀 0/15664，纯数字 15664/15664，空值 0/15664
- 候选键重复：未发现重复
- 枚举 top values：`code`: `603778`(35), `603256`(35), `605255`(31), `002951`(28), `603163`(27), `603601`(27), `601869`(26), `000070`(26)；`limit_up_type`: `换手板`(14006), `一字板`(1058), `T字板`(600)；`is_new`: `False`(15664)；`is_again_limit`: `True`(7857), `False`(7807)；`high_days`: `首板`(11015), `2天2板`(1814), `3天3板`(601), `3天2板`(376), `4天2板`(353), `4天4板`(287), `4天3板`(195), `5天3板`(176)；`name`: `宏和科技`(35), `国晟科技`(35), `天普股份`(31), `金时科技`(28), `圣晖集成`(27), `再升科技`(27), `天际股份`(26), `特发信息`(26)；`change_tag`: `LIMIT_BACK`(7857), `FIRST_LIMIT`(7807)；`market_type`: `HS`(13857), `GEM`(1333), `STAR`(474)
- 数值范围摘要：`open_num` min=0, max=183, zero=8604, negative=0, NULL=0；`order_volume` min=289.0, max=862627850.0, zero=0, negative=0, NULL=0；`currency_value` min=637398040.0, max=2129275300000.0, zero=0, negative=0, NULL=0；`change_rate` min=5.0157, max=20.2312, zero=0, negative=0, NULL=0；`limit_up_suc_rate` min=0.0, max=1.0, zero=1346, negative=0, NULL=0；`market_id` min=17, max=33, zero=0, negative=0, NULL=0；`turnover_rate` min=0.0354, max=77.9943, zero=0, negative=0, NULL=0；`order_amount` min=20053.71, max=5305161300.0, zero=0, negative=0, NULL=0；`high_days_value` min=65537, max=1114137, zero=0, negative=0, NULL=0；`latest` min=1.41, max=1699.96, zero=0, negative=0, NULL=0
