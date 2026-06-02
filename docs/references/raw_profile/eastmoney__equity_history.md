# Raw 数据画像：eastmoney__equity_history

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__equity_history.yml`
- dbt source：`source('raw', 'eastmoney__equity_history')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：`pipeline/elt/models/staging/eastmoney/stg_eastmoney__equity_history.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`eastmoney__equity_history`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__equity_history --execute --output ../docs/references/raw_profile/eastmoney__equity_history.md`，并补充 ClickHouse 结构化汇总查询
- 行数：146,365
- 数据范围：`END_DATE`: 1990-12-19 至 2026-06-10，NULL 0 行；`NOTICE_DATE`: 1990-12-19 至 2026-06-02，NULL 0 行；`LISTING_DATE`: 1990-12-19 至 2026-06-30，NULL 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`eastmoney__equity_history`
- ClickHouse raw 表：`fleur_raw.eastmoney__equity_history`
- 表说明：EastMoney equity history F10 rows by natural-year raw partition.

## 2. 粒度与键

- 观察到的粒度：候选自然键为 `SECUCODE`, `END_DATE`，本次 profiling 未发现重复。
- 候选自然键：`SECUCODE`, `END_DATE`
- 重复检查：未发现重复
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；如果候选键重复，需要在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | 证券代码（纯数字） |
| ORG_CODE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 机构代码 |
| END_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 股本变动截止日 |
| CHANGE_REASON | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 变动原因 |
| LIMITED_SHARES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 有限售条件股份 |
| UNLIMITED_SHARES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 无限售条件股份（已流通） |
| TOTAL_SHARES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 总股本 |
| LIMITED_SHARES_RATIO | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售股比例（%） |
| LISTED_SHARES_RATIO | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 已流通股比例（%） |
| TOTAL_SHARES_RATIO | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 总股本比例（%） |
| LISTED_A_SHARES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 已上市流通 A 股 |
| LIMITED_A_SHARES | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售 A 股 |
| LISTED_A_SHARES_RATIO | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | A 股流通比例（%） |
| LIMITED_A_SHARES_RATIO | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售A股比例（%） |
| B_FREE_SHARE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 已上市流通 B 股 |
| H_FREE_SHARE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 已上市流通 H 股 |
| B_FREE_SHARE_RATIO | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | B股流通比例（%） |
| H_FREE_SHARE_RATIO | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | H 股流通比例（%） |
| SECURITY_TYPE_CODE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 证券类型代码 |
| NON_FREE_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非自由流通股 |
| NON_FREESHARES_RATIO | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流通股比例（%） |
| LIMITED_B_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售 B 股 |
| LIMITED_BSHARES_RATIO | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售B股比例（%） |
| OTHER_FREE_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他已上市流通股 |
| OTHER_FREESHARES_RATIO | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他流通股比例（%） |
| LIMITED_STATE_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 国家持股（限售） |
| LIMITED_STATE_LEGAL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 国有法人持股（限售） |
| LIMITED_OTHARS | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他限售股份 |
| LIMITED_DOMESTIC_NOSTATE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 境内非国有法人持股（限售） |
| LIMITED_DOMESTIC_NATURAL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 境内自然人持股（限售） |
| LOCK_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 锁定股份 |
| LIMITED_FOREIGN_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 外资持股（限售） |
| LIMITED_OVERSEAS_NOSTATE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 境外非国有法人持股（限售） |
| LIMITED_OVERSEAS_NATURAL | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 境外自然人持股（限售） |
| LIMITED_H_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售 H 股 |
| SPONSOR_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 发起人股份 |
| STATE_SPONSOR_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 国家发起人股份 |
| SPONSOR_SOCIAL_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 社会发起人股份 |
| RAISE_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 募集法人股份 |
| RAISE_STATE_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 国家募集法人股份 |
| RAISE_DOMESTIC_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 境内募集法人股份 |
| RAISE_OVERSEAS_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 境外募集法人股份 |
| NOTICE_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 公告披露日 |
| LISTING_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 上市流通日期 |
| LIMITED_SHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售股变动量 |
| UNLIMITED_SHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流通股变动量 |
| TOTAL_SHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 总股本变动量 |
| LISTED_ASHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 已上市流通A股变动量 |
| LIMITED_ASHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售A股变动量 |
| B_FREESHARE_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | B股流通变动量 |
| H_FREESHARE_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | H股流通变动量 |
| LIMITED_BSHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售B股变动量 |
| NONFREE_SHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 非流通股变动量 |
| OTHERFREE_SHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他流通股变动量 |
| FREE_SHARES | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 流通股（通常 = TOTAL_SHARES） |
| CHANGE_REASON_EXPLAIN | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 变动原因详细说明 |
| LIMITED_H_SHARES_RATIO | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售H股比例（%） |
| LIMITED_H_SHARES_CHANGE | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 限售H股变动量 |
| IS_FREE_WINDOW | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 是否为自由流通窗口 |
| IS_LIMITED_WINDOW | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 是否限售窗口 |
| LISTED_A_RATIOPC | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | A 股占已流通比例（%） |
| LISTED_B_RATIOPC | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | B股占已流通比例（%） |
| LISTED_H_RATIOPC | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | H 股占已流通比例（%） |
| LISTED_OTHER_RATIOPC | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 其他占已流通比例（%） |
| LISTED_SUM_RATIOPC | Float64 | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 合计占已流通比例（%） |
| MARKET_CODE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 市场代码 |
| IS_USE | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否有效 |
| SECURITY_NAME_ABBR | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 证券简称 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 146365/146365，供应商前缀 0/146365，纯数字 0/146365，空值 0/146365；`SECURITY_CODE`: canonical 后缀 0/146365，供应商前缀 0/146365，纯数字 146365/146365，空值 0/146365
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：EastMoney 后缀格式可直接作为 canonical security_code；本地代码必须仅作为 local code 使用。

### 日期与时间字段

- 已画像字段：`END_DATE`, `NOTICE_DATE`, `LISTING_DATE`
- 范围：`END_DATE`: 1990-12-19 至 2026-06-10，NULL 0 行；`NOTICE_DATE`: 1990-12-19 至 2026-06-02，NULL 0 行；`LISTING_DATE`: 1990-12-19 至 2026-06-30，NULL 0 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `ORG_CODE`, `CHANGE_REASON`, `SECURITY_TYPE_CODE`, `CHANGE_REASON_EXPLAIN`, `IS_FREE_WINDOW`, `IS_LIMITED_WINDOW`
- 取值：`SECUCODE`: `000333.SZ`(615), `300253.SZ`(247), `300068.SZ`(240), `300015.SZ`(235), `300070.SZ`(235), `002450.SZ`(223), `300113.SZ`(209), `300271.SZ`(206)；`SECURITY_CODE`: `000333`(615), `300253`(247), `300068`(240), `300015`(235), `300070`(235), `002450`(223), `300113`(209), `300271`(206)；`ORG_CODE`: `10015250`(615), `10129024`(247), `10145268`(240), `10122383`(235), `10109380`(235), `10123790`(223), `10147589`(209), `10099806`(206)；`CHANGE_REASON`: `高管股份变动`(33611), `债转股上市`(15171), `回购`(9840), `首发限售股份上市`(9650), `自主行权`(9266), `网下配售股份上市`(8951), `转增股上市`(8920), `股份性质变更`(7986)；`SECURITY_TYPE_CODE`: `058001001`(146321), `058001008`(44)；`CHANGE_REASON_EXPLAIN`: `年报披露`(15180), `中报披露`(14826), `债转股上市`(14172), `高管股份变动`(13056), `回购`(9675), `首发限售股份上市`(9565), `网下配售股份上市`(8860), `自主行权`(8701)；`IS_FREE_WINDOW`: `False`(139506), `True`(6859)；`IS_LIMITED_WINDOW`: `False`(145934), `True`(431)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`TOTAL_SHARES`, `LIMITED_SHARES`, `UNLIMITED_SHARES`, `LIMITED_SHARES_RATIO`, `LISTED_SHARES_RATIO`, `TOTAL_SHARES_RATIO`, `LISTED_A_SHARES`, `LIMITED_A_SHARES`, `LISTED_A_SHARES_RATIO`, `LIMITED_A_SHARES_RATIO`
- 最小/最大值：`TOTAL_SHARES` min=33000.0, max=356406257089.0, zero=0, negative=0, NULL=0；`LIMITED_SHARES` min=0.0, max=283744968904.0, zero=18271, negative=0, NULL=0；`UNLIMITED_SHARES` min=0.0, max=356406257089.0, zero=44, negative=0, NULL=0；`LIMITED_SHARES_RATIO` min=0.0, max=100.0, zero=18271, negative=0, NULL=0；`LISTED_SHARES_RATIO` min=0.0, max=100.0, zero=44, negative=0, NULL=0；`TOTAL_SHARES_RATIO` min=100.0, max=100.0, zero=0, negative=0, NULL=0；`LISTED_A_SHARES` min=0.0, max=319244210777.0, zero=46, negative=0, NULL=0；`LIMITED_A_SHARES` min=0.0, max=283744968904.0, zero=18355, negative=0, NULL=0；`LISTED_A_SHARES_RATIO` min=0.0, max=100.0, zero=46, negative=0, NULL=0；`LIMITED_A_SHARES_RATIO` min=0.0, max=100.0, zero=18355, negative=0, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECURITY_CODE` 只有 6 位本地代码 | 中 | 146365/146365 行 | 仅作为 `security_local_code`；不可单独推出交易所 | 需要其他字段或主数据补齐交易所 |

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

- `select count() from fleur_raw.eastmoney__equity_history`：146,365
- 日期字段范围：`END_DATE`: 1990-12-19 至 2026-06-10，NULL 0 行；`NOTICE_DATE`: 1990-12-19 至 2026-06-02，NULL 0 行；`LISTING_DATE`: 1990-12-19 至 2026-06-30，NULL 0 行
- 证券代码格式：`SECUCODE`: canonical 后缀 146365/146365，供应商前缀 0/146365，纯数字 0/146365，空值 0/146365；`SECURITY_CODE`: canonical 后缀 0/146365，供应商前缀 0/146365，纯数字 146365/146365，空值 0/146365
- 候选键重复：未发现重复
- 枚举 top values：`SECUCODE`: `000333.SZ`(615), `300253.SZ`(247), `300068.SZ`(240), `300015.SZ`(235), `300070.SZ`(235), `002450.SZ`(223), `300113.SZ`(209), `300271.SZ`(206)；`SECURITY_CODE`: `000333`(615), `300253`(247), `300068`(240), `300015`(235), `300070`(235), `002450`(223), `300113`(209), `300271`(206)；`ORG_CODE`: `10015250`(615), `10129024`(247), `10145268`(240), `10122383`(235), `10109380`(235), `10123790`(223), `10147589`(209), `10099806`(206)；`CHANGE_REASON`: `高管股份变动`(33611), `债转股上市`(15171), `回购`(9840), `首发限售股份上市`(9650), `自主行权`(9266), `网下配售股份上市`(8951), `转增股上市`(8920), `股份性质变更`(7986)；`SECURITY_TYPE_CODE`: `058001001`(146321), `058001008`(44)；`CHANGE_REASON_EXPLAIN`: `年报披露`(15180), `中报披露`(14826), `债转股上市`(14172), `高管股份变动`(13056), `回购`(9675), `首发限售股份上市`(9565), `网下配售股份上市`(8860), `自主行权`(8701)；`IS_FREE_WINDOW`: `False`(139506), `True`(6859)；`IS_LIMITED_WINDOW`: `False`(145934), `True`(431)
- 数值范围摘要：`TOTAL_SHARES` min=33000.0, max=356406257089.0, zero=0, negative=0, NULL=0；`LIMITED_SHARES` min=0.0, max=283744968904.0, zero=18271, negative=0, NULL=0；`UNLIMITED_SHARES` min=0.0, max=356406257089.0, zero=44, negative=0, NULL=0；`LIMITED_SHARES_RATIO` min=0.0, max=100.0, zero=18271, negative=0, NULL=0；`LISTED_SHARES_RATIO` min=0.0, max=100.0, zero=44, negative=0, NULL=0；`TOTAL_SHARES_RATIO` min=100.0, max=100.0, zero=0, negative=0, NULL=0；`LISTED_A_SHARES` min=0.0, max=319244210777.0, zero=46, negative=0, NULL=0；`LIMITED_A_SHARES` min=0.0, max=283744968904.0, zero=18355, negative=0, NULL=0；`LISTED_A_SHARES_RATIO` min=0.0, max=100.0, zero=46, negative=0, NULL=0；`LIMITED_A_SHARES_RATIO` min=0.0, max=100.0, zero=18355, negative=0, NULL=0
