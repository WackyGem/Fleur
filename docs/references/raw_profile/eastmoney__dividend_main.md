# Raw 数据画像：eastmoney__dividend_main

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__dividend_main.yml`
- dbt source：`source('raw', 'eastmoney__dividend_main')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__dividend_main.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`eastmoney__dividend_main`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__dividend_main --execute --output ../docs/references/raw_profile/eastmoney__dividend_main.md`，并补充 ClickHouse 结构化汇总查询
- 行数：151,606
- 数据范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行；`EQUITY_RECORD_DATE`: 1970-01-01 至 2026-07-10，NULL 0 行，`1970-01-01` 占位 95808 行；`EX_DIVIDEND_DATE`: 1970-01-01 至 2026-07-09，NULL 0 行，`1970-01-01` 占位 96900 行；`PAY_CASH_DATE`: 1970-01-01 至 2026-07-13，NULL 0 行，`1970-01-01` 占位 99965 行；`GMDECISION_NOTICE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 70793 行；`DAT_YAGGR`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 100343 行；`LAST_TRADE_DATE`: 1970-01-01 至 1970-01-01，NULL 0 行，`1970-01-01` 占位 151606 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；上游 raw asset/Parquet 可能按自然年或快照组织。
- 契约数据集：`eastmoney__dividend_main`
- ClickHouse raw 表：`fleur_raw.eastmoney__dividend_main`
- 表说明：EastMoney dividend main F10 rows by natural-year raw partition.

## 2. 粒度与键

- 观察到的粒度：候选自然键为 `INFO_CODE`，但存在 55 组重复，最大重复 70499 行。
- 候选自然键：`INFO_CODE`
- 重复检查：发现 55 组重复，最大重复 70499 行
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；如果候选键重复，需要在 intermediate/mart 设计中处理。

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 未逐列统计 | 见关键字段画像 | 见关键字段画像 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 证券简称 |
| NOTICE_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 公告日期 |
| IMPL_PLAN_PROFILE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 分红方案简述 |
| ASSIGN_PROGRESS | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 分配进度 |
| EQUITY_RECORD_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 股权登记日 |
| EX_DIVIDEND_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 除权除息日 |
| PAY_CASH_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 派息日 |
| IS_UNASSIGN | Bool | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 是否不分配："0" 否，"1" 是 |
| REPORT_DATE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 报告期 |
| ASSIGN_OBJECT | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 见关键字段画像 | 分配对象 |
| IMPL_PLAN_NEWPROFILE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 方案简介 + 进度后缀 |
| NEW_PROFILE | LowCardinality(String) | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分红方案（含税） |
| GMDECISION_NOTICE_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 股东大会决议公告日 |
| INFO_CODE | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 公告编号 |
| DAT_YAGGR | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 年度股东大会日期 |
| TOTAL_DIVIDEND | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 分红总额（元） |
| TOTAL_DIVIDEND_A | Float64 | 见关键字段画像 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | A股分红总额（元） |
| REPORT_TIME | String | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 报告期截止日 |
| DAT_YAGGR_TODAY | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否今日年度股东大会 |
| NOTICE_TODAY | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否今日公告 |
| GMDECISION_TODAY | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否今日股东大会决议 |
| DIRECTORSUPERVISOR_TODAY | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否今日监事会决议 |
| EQUITY_TODAY | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否今日股权登记 |
| EX_DIVIDEND_TODAY | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否今日除权除息 |
| PAYCASH_TODAY | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否今日派息 |
| IS_PAYCASH | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否派息 |
| IS_EQUITY_RECENT | Bool | 未逐列统计 | 未逐列统计 | 保留 raw 字段；按需在具体 staging 中补充 | 是否近期股权登记 |
| LAST_TRADE_DATE | Date | 见关键字段画像 | 见关键字段画像 | 保留 raw 字段；按需在具体 staging 中补充 | 最后交易日 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 151606/151606，供应商前缀 0/151606，纯数字 0/151606，空值 0/151606；`SECURITY_CODE`: canonical 后缀 0/151606，供应商前缀 0/151606，纯数字 151606/151606，空值 0/151606
- 无效样例：本轮聚合未输出逐条无效样例；空值和格式不匹配已在上方计数中体现。
- 建议 staging 处理：EastMoney 后缀格式可直接作为 canonical security_code；本地代码必须仅作为 local code 使用。

### 日期与时间字段

- 已画像字段：`NOTICE_DATE`, `EQUITY_RECORD_DATE`, `EX_DIVIDEND_DATE`, `PAY_CASH_DATE`, `GMDECISION_NOTICE_DATE`, `DAT_YAGGR`, `LAST_TRADE_DATE`
- 范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行；`EQUITY_RECORD_DATE`: 1970-01-01 至 2026-07-10，NULL 0 行，`1970-01-01` 占位 95808 行；`EX_DIVIDEND_DATE`: 1970-01-01 至 2026-07-09，NULL 0 行，`1970-01-01` 占位 96900 行；`PAY_CASH_DATE`: 1970-01-01 至 2026-07-13，NULL 0 行，`1970-01-01` 占位 99965 行；`GMDECISION_NOTICE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 70793 行；`DAT_YAGGR`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 100343 行；`LAST_TRADE_DATE`: 1970-01-01 至 1970-01-01，NULL 0 行，`1970-01-01` 占位 151606 行
- 无效值或占位值：`1970-01-01` 在日期字段中视为高风险占位值；是否转 NULL 必须逐字段记录。
- 建议 staging 处理：Date 类型保持 Date；明显占位日期可 source-local 转 NULL，并在 YAML meta 中记录 normalization。

### 枚举字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `SECURITY_NAME_ABBR`, `IMPL_PLAN_PROFILE`, `ASSIGN_PROGRESS`, `IS_UNASSIGN`, `REPORT_DATE`, `ASSIGN_OBJECT`
- 取值：`SECUCODE`: `000002.SZ`(71), `000020.SZ`(70), `600663.SH`(70), `000001.SZ`(70), `600601.SH`(70), `600654.SH`(70), `600602.SH`(70), `600610.SH`(70)；`SECURITY_CODE`: `000002`(71), `600663`(70), `000020`(70), `000001`(70), `600610`(70), `600601`(70), `600654`(70), `600602`(70)；`SECURITY_NAME_ABBR`: `东方明珠`(113), `百联股份`(104), `万科A`(71), `平安银行`(70), `中毅达`(70), `中安科`(70), `方正科技`(70), `深华发A`(70)；`IMPL_PLAN_PROFILE`: `不分配不转增`(93454), `10派1元`(5205), `10派2元`(2955), `10派0.5元`(2871), `10派1.5元`(2150), `10派3元`(1777), `10派0.2元`(1299), `10派0.3元`(1298)；`ASSIGN_PROGRESS`: `董事会预案`(69280), `实施方案`(55806), `股东大会预案`(26108), `预披露`(409), `股东大会否决`(2), `董事会决议未通过`(1)；`IS_UNASSIGN`: `True`(93454), `False`(58152)；`REPORT_DATE`: `2025年报`(5193), `2025半年报`(5146), `2024年报`(5143), `2023年报`(5109), `2024半年报`(5092), `2023半年报`(5034), `2022年报`(4971), `2022半年报`(4761)；`ASSIGN_OBJECT`: ``(77399), `全体股东`(69514), `A股股东`(3154), `流通股股东`(1262), `A股流通股股东`(76), `重整投资人,债权人`(35), `重整管理人,债权人`(25), `非流通股股东`(17)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：`TOTAL_DIVIDEND`, `TOTAL_DIVIDEND_A`
- 最小/最大值：`TOTAL_DIVIDEND` min=0.0, max=110593000000.0, zero=55447, negative=0, NULL=0；`TOTAL_DIVIDEND_A` min=0.0, max=83661000000.0, zero=55448, negative=0, NULL=0
- 负数/零值/极端值：负值和零值按字段语义解释；财务科目、增长率、行情指标不应在 staging 静默过滤。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位需在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后到具体模型设计。

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| 候选自然键存在重复 | 中 | 重复 key group 55 组，最大重复 70499 行 | staging 不静默去重；保留原始粒度或增加局部序号 | 需要优先级判断的去重放到 intermediate/mart |
| `EQUITY_RECORD_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 95808 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `EX_DIVIDEND_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 96900 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `PAY_CASH_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 99965 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `GMDECISION_NOTICE_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 70793 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `DAT_YAGGR` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 100343 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `LAST_TRADE_DATE` 使用 `1970-01-01` 表示缺失/未发生日期 | 中 | 151606 行 | 在 staging 中按字段语义转为 NULL 或保留并显式标注 | 是否作为业务缺失值需在对应 model 中确认 |
| `SECURITY_CODE` 只有 6 位本地代码 | 中 | 151606/151606 行 | 仅作为 `security_local_code`；不可单独推出交易所 | 需要其他字段或主数据补齐交易所 |

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

- `select count() from fleur_raw.eastmoney__dividend_main`：151,606
- 日期字段范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行；`EQUITY_RECORD_DATE`: 1970-01-01 至 2026-07-10，NULL 0 行，`1970-01-01` 占位 95808 行；`EX_DIVIDEND_DATE`: 1970-01-01 至 2026-07-09，NULL 0 行，`1970-01-01` 占位 96900 行；`PAY_CASH_DATE`: 1970-01-01 至 2026-07-13，NULL 0 行，`1970-01-01` 占位 99965 行；`GMDECISION_NOTICE_DATE`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 70793 行；`DAT_YAGGR`: 1970-01-01 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 100343 行；`LAST_TRADE_DATE`: 1970-01-01 至 1970-01-01，NULL 0 行，`1970-01-01` 占位 151606 行
- 证券代码格式：`SECUCODE`: canonical 后缀 151606/151606，供应商前缀 0/151606，纯数字 0/151606，空值 0/151606；`SECURITY_CODE`: canonical 后缀 0/151606，供应商前缀 0/151606，纯数字 151606/151606，空值 0/151606
- 候选键重复：55 组重复，最大重复 70499 行
- 枚举 top values：`SECUCODE`: `000002.SZ`(71), `000020.SZ`(70), `600663.SH`(70), `000001.SZ`(70), `600601.SH`(70), `600654.SH`(70), `600602.SH`(70), `600610.SH`(70)；`SECURITY_CODE`: `000002`(71), `600663`(70), `000020`(70), `000001`(70), `600610`(70), `600601`(70), `600654`(70), `600602`(70)；`SECURITY_NAME_ABBR`: `东方明珠`(113), `百联股份`(104), `万科A`(71), `平安银行`(70), `中毅达`(70), `中安科`(70), `方正科技`(70), `深华发A`(70)；`IMPL_PLAN_PROFILE`: `不分配不转增`(93454), `10派1元`(5205), `10派2元`(2955), `10派0.5元`(2871), `10派1.5元`(2150), `10派3元`(1777), `10派0.2元`(1299), `10派0.3元`(1298)；`ASSIGN_PROGRESS`: `董事会预案`(69280), `实施方案`(55806), `股东大会预案`(26108), `预披露`(409), `股东大会否决`(2), `董事会决议未通过`(1)；`IS_UNASSIGN`: `True`(93454), `False`(58152)；`REPORT_DATE`: `2025年报`(5193), `2025半年报`(5146), `2024年报`(5143), `2023年报`(5109), `2024半年报`(5092), `2023半年报`(5034), `2022年报`(4971), `2022半年报`(4761)；`ASSIGN_OBJECT`: ``(77399), `全体股东`(69514), `A股股东`(3154), `流通股股东`(1262), `A股流通股股东`(76), `重整投资人,债权人`(35), `重整管理人,债权人`(25), `非流通股股东`(17)
- 数值范围摘要：`TOTAL_DIVIDEND` min=0.0, max=110593000000.0, zero=55447, negative=0, NULL=0；`TOTAL_DIVIDEND_A` min=0.0, max=83661000000.0, zero=55448, negative=0, NULL=0
