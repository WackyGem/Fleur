# Raw 数据画像：eastmoney__dividend_main

日期：2026-06-03

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__dividend_main.yml`
- dbt source：`source('raw', 'eastmoney__dividend_main')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待定；建议为 `pipeline/elt/models/staging/eastmoney/stg_eastmoney__dividend_main.sql`

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__dividend_main`
- profiling 命令：结构化 ClickHouse 汇总查询；同等 dbt 入口为 `cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__dividend_main --execute --status Accepted --output ../docs/references/raw_profile/eastmoney__dividend_main.md`
- 行数：151,606
- 数据范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 0 行；`EQUITY_RECORD_DATE`: 1991-05-27 至 2026-07-10，NULL 95,808 行，`1970-01-01` 占位 0 行；`EX_DIVIDEND_DATE`: 1991-02-26 至 2026-07-09，NULL 96,900 行，`1970-01-01` 占位 0 行；`PAY_CASH_DATE`: 1992-03-23 至 2026-07-13，NULL 99,965 行，`1970-01-01` 占位 0 行；`REPORT_DATE`: 1990年报 至 2026重整计划，NULL 0 行，`1970-01-01` 占位 0 行；`GMDECISION_NOTICE_DATE`: 1991-04-17 至 2026-06-02，NULL 70,793 行，`1970-01-01` 占位 0 行；`DAT_YAGGR`: 2003-03-01 至 2026-06-02，NULL 100,343 行，`1970-01-01` 占位 0 行；`REPORT_TIME`: 1990-12-31 00:00:00 至 2026-09-30 00:00:00，NULL 1,621 行，`1970-01-01` 占位 0 行；`LAST_TRADE_DATE`: NULL 至 NULL，NULL 151,606 行，`1970-01-01` 占位 0 行
- 分区范围：ClickHouse raw 表内未暴露独立分区字段；本报告使用 raw 表内日期/时间字段描述覆盖范围。
- 契约数据集：`eastmoney__dividend_main`
- ClickHouse raw 表：`fleur_raw.eastmoney__dividend_main`
- 表说明：EastMoney dividend main F10 rows by natural-year raw partition.

### 2026-06-03 类型勘误

- 本报告原始 profiling 基于当时的 ClickHouse raw 表，`REPORT_TIME` 观察类型为 `Nullable(String)`，值形如 `1990-12-31 00:00:00`。
- 追加核验确认：`REPORT_TIME` 非 NULL 值不可解析数量为 0，原始时间部分非 `00:00:00` 数量为 0。
- 当前 contract 已将 `REPORT_TIME` 的 S3 Parquet 类型收敛为 `date32[day]`、ClickHouse raw 类型收敛为 `Nullable(Date)`；历史非日期标签在 source-to-Parquet 转换中置为 NULL。

## 2. 数据分析发现

- 数据量与覆盖
  - 总记录数：151,606。
  - 覆盖主体数：`secucode` 5,520 个；`security_code` 5,520 个
  - 日期 / 分区范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 0 行；`EQUITY_RECORD_DATE`: 1991-05-27 至 2026-07-10，NULL 95,808 行，`1970-01-01` 占位 0 行；`EX_DIVIDEND_DATE`: 1991-02-26 至 2026-07-09，NULL 96,900 行，`1970-01-01` 占位 0 行；`PAY_CASH_DATE`: 1992-03-23 至 2026-07-13，NULL 99,965 行，`1970-01-01` 占位 0 行；`REPORT_DATE`: 1990年报 至 2026重整计划，NULL 0 行，`1970-01-01` 占位 0 行；`GMDECISION_NOTICE_DATE`: 1991-04-17 至 2026-06-02，NULL 70,793 行，`1970-01-01` 占位 0 行；`DAT_YAGGR`: 2003-03-01 至 2026-06-02，NULL 100,343 行，`1970-01-01` 占位 0 行；`REPORT_TIME`: 1990-12-31 00:00:00 至 2026-09-30 00:00:00，NULL 1,621 行，`1970-01-01` 占位 0 行；`LAST_TRADE_DATE`: NULL 至 NULL，NULL 151,606 行，`1970-01-01` 占位 0 行
- 粒度与候选键
  - 观察到的粒度：候选自然键为 `SECUCODE`, `REPORT_DATE`。
  - 候选自然键去重结果：发现 19 组重复键，单键最大 2 行。
  - 旧候选键或备选键对比：本轮未发现需要替换的旧候选键；如后续 staging 引入公告号、批次或版本字段，需要重新执行重复检查。
- 缺失与占位
  - 关键字段 NULL / 空字符串分布：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行。
  - 占位值：日期/时间字段合计 `1970-01-01` 0 行。
  - 预期缺失：宽表财务科目、可选事件日期、删除时间、公告编号等字段存在 NULL/空值时，需按字段语义解释；staging 不用全字段 `not_null` 覆盖。
- 格式与参照完整性
  - 证券代码 / 报告期 / 高价值字符串格式：`SECUCODE`: canonical 后缀 151,606/151,606，供应商前缀 0/151,606，纯数字 0/151,606，空值 0/151,606；`SECURITY_CODE`: canonical 后缀 0/151,606，供应商前缀 0/151,606，纯数字 151,606/151,606，空值 0/151,606
  - 直接 raw input 参照命中情况：本表 profiling 只检查直接 raw 字段，不做跨源主数据裁决。
- 分布与相关性
  - 枚举 top values：`SECURITY_CODE`: `000002`(71), `600663`(70), `000020`(70), `000001`(70), `600610`(70), `600601`(70), `600654`(70), `600602`(70)；`SECURITY_NAME_ABBR`: `东方明珠`(113), `百联股份`(104), `万科A`(71), `平安银行`(70), `中毅达`(70), `中安科`(70), `方正科技`(70), `深华发A`(70)；`IMPL_PLAN_PROFILE`: `不分配不转增`(93,454), `10派1元`(5,205), `10派2元`(2,955), `10派0.5元`(2,871), `10派1.5元`(2,150), `10派3元`(1,777), `10派0.2元`(1,299), `10派0.3元`(1,298)；`ASSIGN_PROGRESS`: `董事会预案`(69,280), `实施方案`(55,806), `股东大会预案`(26,108), `预披露`(409), `股东大会否决`(2), `董事会决议未通过`(1)；`IS_UNASSIGN`: `1`(93,454), `0`(58,152)；`ASSIGN_OBJECT`: `NULL`(77,399), `全体股东`(69,514), `A股股东`(3,154), `流通股股东`(1,262), `A股流通股股东`(76), `重整投资人,债权人`(35), `重整管理人,债权人`(25), `非流通股股东`(17)；`IMPL_PLAN_NEWPROFILE`: `不分配不转增`(93,429), `10派1元(实施方案)`(5,077), `10派2元(实施方案)`(2,863), `10派0.5元(实施方案)`(2,786), `10派1.5元(实施方案)`(2,078), `10派3元(实施方案)`(1,716), `10派0.3元(实施方案)`(1,258), `10派0.2元(实施方案)`(1,256)；`NEW_PROFILE`: `不分配不转增`(93,454), `10派1元(含税)`(5,205), `10派2元(含税)`(2,955), `10派0.5元(含税)`(2,871), `10派1.5元(含税)`(2,150), `10派3元(含税)`(1,777), `10派0.2元(含税)`(1,299), `10派0.3元(含税)`(1,298)
  - 少量值 / 长尾文本：长文本、题材、公告简述和证券简称只保留观察；同义归一化延后到 intermediate/mart。
  - 字段间强相关：本轮只执行 source-local 单表画像，未做跨字段因果或业务优先级判断。
- 时间字段合理性
  - 日期范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 0 行；`EQUITY_RECORD_DATE`: 1991-05-27 至 2026-07-10，NULL 95,808 行，`1970-01-01` 占位 0 行；`EX_DIVIDEND_DATE`: 1991-02-26 至 2026-07-09，NULL 96,900 行，`1970-01-01` 占位 0 行；`PAY_CASH_DATE`: 1992-03-23 至 2026-07-13，NULL 99,965 行，`1970-01-01` 占位 0 行；`REPORT_DATE`: 1990年报 至 2026重整计划，NULL 0 行，`1970-01-01` 占位 0 行；`GMDECISION_NOTICE_DATE`: 1991-04-17 至 2026-06-02，NULL 70,793 行，`1970-01-01` 占位 0 行；`DAT_YAGGR`: 2003-03-01 至 2026-06-02，NULL 100,343 行，`1970-01-01` 占位 0 行；`REPORT_TIME`: 1990-12-31 00:00:00 至 2026-09-30 00:00:00，NULL 1,621 行，`1970-01-01` 占位 0 行；`LAST_TRADE_DATE`: NULL 至 NULL，NULL 151,606 行，`1970-01-01` 占位 0 行
  - 日期先后关系异常：未执行跨字段先后关系过滤；涉及公告、股权登记、除权除息、派息等事件顺序时，在具体 staging 或 intermediate 设计中追加定向检查。
  - 批次时间范围：raw 表未暴露独立批次时间字段。
- 数值字段合理性
  - 负数 / 零值 / 极端值：已对 2 个数值字段执行 min/max、NULL、零值和负值检查；其中 0 个字段出现负值，2 个字段出现零值，0 个字段 NULL 数不低于 80%。
  - 单位判断：本报告保留 raw 字段单位；金额、股数、比例和价格单位必须在具体 staging YAML metadata 中记录。
- 其他观察
  - 对 staging 有影响的事实只限确定性格式、类型、NULL/占位和候选键；跨源主数据修正、业务口径和去重优先级不进入 staging。

## 3. 粒度与键

- 观察到的粒度：`SECUCODE`, `REPORT_DATE`。
- 候选自然键：`SECUCODE`, `REPORT_DATE`。
- 重复检查：发现 19 组重复键，单键最大 2 行。
- 粒度注意事项：staging 不做跨源去重、主数据修正或业务优先级裁决；候选键重复时保留 source-local 行并把版本选择延后。

## 4. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| SECUCODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,520 | 证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,520 | 证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 5,518 | 证券简称 |
| NOTICE_DATE | Date | 0 | `1970-01-01` 0 | 1991-05-27 至 2026-06-02; distinct 6,992 | 公告日期 |
| IMPL_PLAN_PROFILE | LowCardinality(Nullable(String)) | 85 | 空字符串 0；`1970-01-01` 0 | distinct 6,210 | 分红方案简述 |
| ASSIGN_PROGRESS | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 6 | 分配进度 |
| EQUITY_RECORD_DATE | Nullable(Date) | 95,808 | `1970-01-01` 0 | 1991-05-27 至 2026-07-10; distinct 5,223 | 股权登记日 |
| EX_DIVIDEND_DATE | Nullable(Date) | 96,900 | `1970-01-01` 0 | 1991-02-26 至 2026-07-09; distinct 5,106 | 除权除息日 |
| PAY_CASH_DATE | Nullable(Date) | 99,965 | `1970-01-01` 0 | 1992-03-23 至 2026-07-13; distinct 4,558 | 派息日 |
| IS_UNASSIGN | Bool | 0 | 零值 58,152 | min=0, max=1, distinct 2 | 是否不分配："0" 否，"1" 是 |
| REPORT_DATE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | 1990年报 至 2026重整计划; distinct 151 | 报告期 |
| ASSIGN_OBJECT | LowCardinality(Nullable(String)) | 77,399 | 空字符串 0；`1970-01-01` 0 | distinct 106 | 分配对象 |
| IMPL_PLAN_NEWPROFILE | LowCardinality(String) | 0 | 空字符串 0；`1970-01-01` 0 | distinct 6,592 | 方案简介 + 进度后缀 |
| NEW_PROFILE | LowCardinality(Nullable(String)) | 85 | 空字符串 0；`1970-01-01` 0 | distinct 6,237 | 分红方案（含税） |
| GMDECISION_NOTICE_DATE | Nullable(Date) | 70,793 | `1970-01-01` 0 | 1991-04-17 至 2026-06-02; distinct 5,020 | 股东大会决议公告日 |
| INFO_CODE | Nullable(String) | 70,499 | 空字符串 0；`1970-01-01` 0 | distinct 81,046 | 公告编号 |
| DAT_YAGGR | Nullable(Date) | 100,343 | `1970-01-01` 0 | 2003-03-01 至 2026-06-02; distinct 2,629 | 年度股东大会日期 |
| TOTAL_DIVIDEND | Nullable(Float64) | 434 | 零值 55,013；负值 0 | min=0, max=110,593,000,000, distinct 38,068 | 分红总额（元） |
| TOTAL_DIVIDEND_A | Nullable(Float64) | 393 | 零值 55,055；负值 0 | min=0, max=83,661,000,000, distinct 38,073 | A股分红总额（元） |
| REPORT_TIME | Nullable(String)；当前 contract 为 Nullable(Date) | 1,621 | 空字符串 0；`1970-01-01` 0 | 1990-12-31 00:00:00 至 2026-09-30 00:00:00; distinct 98 | 报告期截止日；历史 raw 观察为字符串，contract 已收敛为日期。 |
| DAT_YAGGR_TODAY | Bool | 0 | 零值 151,605 | min=0, max=1, distinct 2 | 是否今日年度股东大会 |
| NOTICE_TODAY | Bool | 0 | 零值 151,542 | min=0, max=1, distinct 2 | 是否今日公告 |
| GMDECISION_TODAY | Bool | 0 | 零值 151,594 | min=0, max=1, distinct 2 | 是否今日股东大会决议 |
| DIRECTORSUPERVISOR_TODAY | Bool | 0 | 零值 151,603 | min=0, max=1, distinct 2 | 是否今日监事会决议 |
| EQUITY_TODAY | Bool | 0 | 零值 151,524 | min=0, max=1, distinct 2 | 是否今日股权登记 |
| EX_DIVIDEND_TODAY | Bool | 0 | 零值 151,536 | min=0, max=1, distinct 2 | 是否今日除权除息 |
| PAYCASH_TODAY | Bool | 0 | 零值 151,538 | min=0, max=1, distinct 2 | 是否今日派息 |
| IS_PAYCASH | Bool | 0 | 零值 151,221 | min=0, max=1, distinct 2 | 是否派息 |
| IS_EQUITY_RECENT | Bool | 0 | 零值 151,276 | min=0, max=1, distinct 2 | 是否近期股权登记 |
| LAST_TRADE_DATE | Nullable(Date) | 151,606 | `1970-01-01` 0 | NULL 至 NULL; distinct 0 | 最后交易日 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`
- 观察到的格式：`SECUCODE`: canonical 后缀 151,606/151,606，供应商前缀 0/151,606，纯数字 0/151,606，空值 0/151,606；`SECURITY_CODE`: canonical 后缀 0/151,606，供应商前缀 0/151,606，纯数字 151,606/151,606，空值 0/151,606
- 无效样例：本轮聚合未发现空证券代码；格式差异按上方计数处理。
- 建议 staging 处理：canonical 后缀格式可直接作为证券代码；BaoStock 前缀格式可确定性转换；纯 6 位代码只能作为本地代码，交易所归属需要其他字段或主数据。

### 日期与时间字段

- 已画像字段：`NOTICE_DATE`, `EQUITY_RECORD_DATE`, `EX_DIVIDEND_DATE`, `PAY_CASH_DATE`, `REPORT_DATE`, `GMDECISION_NOTICE_DATE`, `DAT_YAGGR`, `REPORT_TIME`, `LAST_TRADE_DATE`
- 范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 0 行；`EQUITY_RECORD_DATE`: 1991-05-27 至 2026-07-10，NULL 95,808 行，`1970-01-01` 占位 0 行；`EX_DIVIDEND_DATE`: 1991-02-26 至 2026-07-09，NULL 96,900 行，`1970-01-01` 占位 0 行；`PAY_CASH_DATE`: 1992-03-23 至 2026-07-13，NULL 99,965 行，`1970-01-01` 占位 0 行；`REPORT_DATE`: 1990年报 至 2026重整计划，NULL 0 行，`1970-01-01` 占位 0 行；`GMDECISION_NOTICE_DATE`: 1991-04-17 至 2026-06-02，NULL 70,793 行，`1970-01-01` 占位 0 行；`DAT_YAGGR`: 2003-03-01 至 2026-06-02，NULL 100,343 行，`1970-01-01` 占位 0 行；`REPORT_TIME`: 1990-12-31 00:00:00 至 2026-09-30 00:00:00，NULL 1,621 行，`1970-01-01` 占位 0 行；`LAST_TRADE_DATE`: NULL 至 NULL，NULL 151,606 行，`1970-01-01` 占位 0 行
- 无效值或占位值：日期/时间字段合计 `1970-01-01` 0 行。
- 建议 staging 处理：ClickHouse Date/DateTime 类型保持类型；字符串日期在 staging 明确 cast；确定的 `1970-01-01` 占位可转 NULL 并记录 normalization。

### 枚举字段

- 已画像字段：`SECURITY_CODE`, `SECURITY_NAME_ABBR`, `IMPL_PLAN_PROFILE`, `ASSIGN_PROGRESS`, `IS_UNASSIGN`, `ASSIGN_OBJECT`, `IMPL_PLAN_NEWPROFILE`, `NEW_PROFILE`
- 取值：`SECURITY_CODE`: `000002`(71), `600663`(70), `000020`(70), `000001`(70), `600610`(70), `600601`(70), `600654`(70), `600602`(70)；`SECURITY_NAME_ABBR`: `东方明珠`(113), `百联股份`(104), `万科A`(71), `平安银行`(70), `中毅达`(70), `中安科`(70), `方正科技`(70), `深华发A`(70)；`IMPL_PLAN_PROFILE`: `不分配不转增`(93,454), `10派1元`(5,205), `10派2元`(2,955), `10派0.5元`(2,871), `10派1.5元`(2,150), `10派3元`(1,777), `10派0.2元`(1,299), `10派0.3元`(1,298)；`ASSIGN_PROGRESS`: `董事会预案`(69,280), `实施方案`(55,806), `股东大会预案`(26,108), `预披露`(409), `股东大会否决`(2), `董事会决议未通过`(1)；`IS_UNASSIGN`: `1`(93,454), `0`(58,152)；`ASSIGN_OBJECT`: `NULL`(77,399), `全体股东`(69,514), `A股股东`(3,154), `流通股股东`(1,262), `A股流通股股东`(76), `重整投资人,债权人`(35), `重整管理人,债权人`(25), `非流通股股东`(17)；`IMPL_PLAN_NEWPROFILE`: `不分配不转增`(93,429), `10派1元(实施方案)`(5,077), `10派2元(实施方案)`(2,863), `10派0.5元(实施方案)`(2,786), `10派1.5元(实施方案)`(2,078), `10派3元(实施方案)`(1,716), `10派0.3元(实施方案)`(1,258), `10派0.2元(实施方案)`(1,256)；`NEW_PROFILE`: `不分配不转增`(93,454), `10派1元(含税)`(5,205), `10派2元(含税)`(2,955), `10派0.5元(含税)`(2,871), `10派1.5元(含税)`(2,150), `10派3元(含税)`(1,777), `10派0.2元(含税)`(1,299), `10派0.3元(含税)`(1,298)
- 未知或异常取值：本轮只记录 top values；只有业务域封闭且取值稳定的字段才适合 accepted-values 测试。
- 建议 staging 处理：布尔/状态字段可保留原始语义；业务文本枚举和长尾主题文本不要在 staging 强行收敛为跨源枚举。

### 数值字段

- 已画像字段：全表 2 个数值字段。
- 最小/最大值：逐字段 min/max 已写入字段画像表。
- 负数/零值/极端值：已对 2 个数值字段执行 min/max、NULL、零值和负值检查；其中 0 个字段出现负值，2 个字段出现零值，0 个字段 NULL 数不低于 80%。
- 单位假设：保留 raw 单位；金额、比例、股数和价格单位在具体 staging 字段 meta 中补充。
- 建议 staging 处理：只做确定性 cast/rename/format normalization；指标口径、单位换算和异常阈值判断延后。

## 6. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `SECURITY_CODE` 为 6 位本地代码 | 中 | 151,606/151,606 行为纯数字 | 只作为 `security_local_code`，不可单独推出交易所 | 交易所归属或证券主数据修正延后 |
| 候选键存在重复 | 高 | `SECUCODE, REPORT_DATE` 重复键 19 组，单键最大 2 行 | staging 不做优先级去重，只保留 source-local 字段 | 版本选择和业务优先级延后 |

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

- 行数：151,606。
- 日期 / 分区范围：`NOTICE_DATE`: 1991-05-27 至 2026-06-02，NULL 0 行，`1970-01-01` 占位 0 行；`EQUITY_RECORD_DATE`: 1991-05-27 至 2026-07-10，NULL 95,808 行，`1970-01-01` 占位 0 行；`EX_DIVIDEND_DATE`: 1991-02-26 至 2026-07-09，NULL 96,900 行，`1970-01-01` 占位 0 行；`PAY_CASH_DATE`: 1992-03-23 至 2026-07-13，NULL 99,965 行，`1970-01-01` 占位 0 行；`REPORT_DATE`: 1990年报 至 2026重整计划，NULL 0 行，`1970-01-01` 占位 0 行；`GMDECISION_NOTICE_DATE`: 1991-04-17 至 2026-06-02，NULL 70,793 行，`1970-01-01` 占位 0 行；`DAT_YAGGR`: 2003-03-01 至 2026-06-02，NULL 100,343 行，`1970-01-01` 占位 0 行；`REPORT_TIME`: 1990-12-31 00:00:00 至 2026-09-30 00:00:00，NULL 1,621 行，`1970-01-01` 占位 0 行；`LAST_TRADE_DATE`: NULL 至 NULL，NULL 151,606 行，`1970-01-01` 占位 0 行
- 候选键重复：发现 19 组重复键，单键最大 2 行。
- 关键 NULL / 占位值：`SECUCODE` NULL 0 行；`REPORT_DATE` NULL 0 行；日期/时间 `1970-01-01` 合计 0 行。
- 枚举 / 文本分布：`SECURITY_CODE`: `000002`(71), `600663`(70), `000020`(70), `000001`(70), `600610`(70), `600601`(70), `600654`(70), `600602`(70)；`SECURITY_NAME_ABBR`: `东方明珠`(113), `百联股份`(104), `万科A`(71), `平安银行`(70), `中毅达`(70), `中安科`(70), `方正科技`(70), `深华发A`(70)；`IMPL_PLAN_PROFILE`: `不分配不转增`(93,454), `10派1元`(5,205), `10派2元`(2,955), `10派0.5元`(2,871), `10派1.5元`(2,150), `10派3元`(1,777), `10派0.2元`(1,299), `10派0.3元`(1,298)；`ASSIGN_PROGRESS`: `董事会预案`(69,280), `实施方案`(55,806), `股东大会预案`(26,108), `预披露`(409), `股东大会否决`(2), `董事会决议未通过`(1)；`IS_UNASSIGN`: `1`(93,454), `0`(58,152)；`ASSIGN_OBJECT`: `NULL`(77,399), `全体股东`(69,514), `A股股东`(3,154), `流通股股东`(1,262), `A股流通股股东`(76), `重整投资人,债权人`(35), `重整管理人,债权人`(25), `非流通股股东`(17)；`IMPL_PLAN_NEWPROFILE`: `不分配不转增`(93,429), `10派1元(实施方案)`(5,077), `10派2元(实施方案)`(2,863), `10派0.5元(实施方案)`(2,786), `10派1.5元(实施方案)`(2,078), `10派3元(实施方案)`(1,716), `10派0.3元(实施方案)`(1,258), `10派0.2元(实施方案)`(1,256)；`NEW_PROFILE`: `不分配不转增`(93,454), `10派1元(含税)`(5,205), `10派2元(含税)`(2,955), `10派0.5元(含税)`(2,871), `10派1.5元(含税)`(2,150), `10派3元(含税)`(1,777), `10派0.2元(含税)`(1,299), `10派0.3元(含税)`(1,298)
- 数值范围：已对 2 个数值字段执行 min/max、NULL、零值和负值检查；其中 0 个字段出现负值，2 个字段出现零值，0 个字段 NULL 数不低于 80%。

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
select `SECUCODE`, `REPORT_DATE`, `SECURITY_CODE`, `NOTICE_DATE`, `EQUITY_RECORD_DATE`, `EX_DIVIDEND_DATE`, `PAY_CASH_DATE`, `GMDECISION_NOTICE_DATE`, `DAT_YAGGR`, `REPORT_TIME` from fleur_raw.eastmoney__dividend_main limit 5
```

结果：

```text
[{'SECUCODE': '000001.SZ', 'REPORT_DATE': '1991年报', 'SECURITY_CODE': '000001', 'NOTICE_DATE': datetime.date(1991, 12, 31), 'EQUITY_RECORD_DATE': None, 'EX_DIVIDEND_DATE': datetime.date(1992, 3, 23), 'PAY_CASH_DATE': datetime.date(1992, 3, 23), 'GMDECISION_NOTICE_DATE': None, 'DAT_YAGGR': None, 'REPORT_TIME': '1991-12-31 00:00:00'}, {'SECUCODE': '000002.SZ', 'REPORT_DATE': '1990年报', 'SECURITY_CODE': '000002', 'NOTICE_DATE': datetime.date(1991, 5, 27), 'EQUITY_RECORD_DATE': datetime.date(1991, 5, 27), 'EX_DIVIDEND_DATE': datetime.date(1991, 6, 8), 'PAY_CASH_DATE': None, 'GMDECISION_NOTICE_DATE': datetime.date(1991, 4, 17), 'DAT_YAGGR': None, 'REPORT_TIME': '1990-12-31 00:00:00'}, {'SECUCODE': '000004.SZ', 'REPORT_DATE': '1990年报', 'SECURITY_CODE': '000004', 'NOTICE_DATE': datetime.date(1991, 6, 30), 'EQUITY_RECORD_DATE': datetime.date(1991, 6, 17), 'EX_DIVIDEND_DATE': datetime.date(1991, 6, 28), 'PAY_CASH_DATE': None, 'GMDECISION_NOTICE_DATE': None, 'DAT_YAGGR': None, 'REPORT_TIME': '1990-12-31 00:00:00'}, {'SECUCODE': '600601.SH', 'REPORT_DATE': '1991年报', 'SECURITY_CODE': '600601', 'NOTICE_DATE': datetime.date(1991, 12, 31), 'EQUITY_RECORD_DATE': None, 'EX_DIVIDEND_DATE': datetime.date(1991, 3, 11), 'PAY_CASH_DATE': None, 'GMDECISION_NOTICE_DATE': None, 'DAT_YAGGR': None, 'REPORT_TIME': '1991-12-31 00:00:00'}, {'SECUCODE': '600651.SH', 'REPORT_DATE': '1991年报', 'SECURITY_CODE': '600651', 'NOTICE_DATE': datetime.date(1991, 12, 31), 'EQUITY_RECORD_DATE': None, 'EX_DIVIDEND_DATE': datetime.date(1991, 8, 26), 'PAY_CASH_DATE': None, 'GMDECISION_NOTICE_DATE': None, 'DAT_YAGGR': None, 'REPORT_TIME': '1991-12-31 00:00:00'}]
```

### 行数统计

```sql
select count() from fleur_raw.eastmoney__dividend_main
```

结果：

```text
[[151606]]
```

### 候选键重复检查

```sql
select count() as duplicate_key_count, max(row_count) as max_rows_per_key
from (select `SECUCODE`, `REPORT_DATE`, count() as row_count from fleur_raw.eastmoney__dividend_main group by `SECUCODE`, `REPORT_DATE` having row_count > 1)
```

结果：

```text
{'duplicate_key_count': 19, 'max_rows_per_key': 2}
```

### 证券代码格式：SECUCODE

```sql
select countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__dividend_main
```

结果：

```text
{'canonical_suffix': 151606, 'vendor_prefix': 0, 'numeric_only': 0, 'empty_or_null': 0, 'row_count': 151606}
```

### 证券代码格式：SECURITY_CODE

```sql
select countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix, countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix, countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only, countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null, count() as row_count from fleur_raw.eastmoney__dividend_main
```

结果：

```text
{'canonical_suffix': 0, 'vendor_prefix': 0, 'numeric_only': 151606, 'empty_or_null': 0, 'row_count': 151606}
```
