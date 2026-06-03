# Raw 数据画像：eastmoney__dividend_main

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/eastmoney__dividend_main.yml`
- dbt source：`source('raw', 'eastmoney__dividend_main')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：待补充

## 1. 范围与执行信息

- source 名称：`raw`
- raw 表：`eastmoney__dividend_main`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table eastmoney__dividend_main --execute --output ../docs/references/raw_profile/eastmoney__dividend_main.md`
- 行数：待补充
- 数据范围：待补充
- 分区范围：待补充
- 契约数据集：`eastmoney__dividend_main`
- ClickHouse raw 表：`fleur_raw.eastmoney__dividend_main`
- 表说明：EastMoney dividend main F10 rows by natural-year raw partition.

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
| SECUCODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECUCODE`。 原始字段说明：证券代码（含市场后缀） |
| SECURITY_CODE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECURITY_CODE`。 原始字段说明：证券代码（纯数字） |
| SECURITY_NAME_ABBR | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `SECURITY_NAME_ABBR`。 原始字段说明：证券简称 |
| NOTICE_DATE | Date | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTICE_DATE`。 原始字段说明：公告日期 |
| IMPL_PLAN_PROFILE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IMPL_PLAN_PROFILE`。 原始字段说明：分红方案简述 |
| ASSIGN_PROGRESS | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSIGN_PROGRESS`。 原始字段说明：分配进度 |
| EQUITY_RECORD_DATE | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EQUITY_RECORD_DATE`。 原始字段说明：股权登记日 |
| EX_DIVIDEND_DATE | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EX_DIVIDEND_DATE`。 原始字段说明：除权除息日 |
| PAY_CASH_DATE | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAY_CASH_DATE`。 原始字段说明：派息日 |
| IS_UNASSIGN | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IS_UNASSIGN`。 原始字段说明：是否不分配："0" 否，"1" 是 |
| REPORT_DATE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REPORT_DATE`。 原始字段说明：报告期 |
| ASSIGN_OBJECT | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `ASSIGN_OBJECT`。 原始字段说明：分配对象 |
| IMPL_PLAN_NEWPROFILE | LowCardinality(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IMPL_PLAN_NEWPROFILE`。 原始字段说明：方案简介 + 进度后缀 |
| NEW_PROFILE | LowCardinality(Nullable(String)) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NEW_PROFILE`。 原始字段说明：分红方案（含税） |
| GMDECISION_NOTICE_DATE | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `GMDECISION_NOTICE_DATE`。 原始字段说明：股东大会决议公告日 |
| INFO_CODE | Nullable(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `INFO_CODE`。 原始字段说明：公告编号 |
| DAT_YAGGR | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DAT_YAGGR`。 原始字段说明：年度股东大会日期 |
| TOTAL_DIVIDEND | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_DIVIDEND`。 原始字段说明：分红总额（元） |
| TOTAL_DIVIDEND_A | Nullable(Float64) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `TOTAL_DIVIDEND_A`。 原始字段说明：A股分红总额（元） |
| REPORT_TIME | Nullable(String) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `REPORT_TIME`。 原始字段说明：报告期截止日 |
| DAT_YAGGR_TODAY | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DAT_YAGGR_TODAY`。 原始字段说明：是否今日年度股东大会 |
| NOTICE_TODAY | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `NOTICE_TODAY`。 原始字段说明：是否今日公告 |
| GMDECISION_TODAY | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `GMDECISION_TODAY`。 原始字段说明：是否今日股东大会决议 |
| DIRECTORSUPERVISOR_TODAY | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `DIRECTORSUPERVISOR_TODAY`。 原始字段说明：是否今日监事会决议 |
| EQUITY_TODAY | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EQUITY_TODAY`。 原始字段说明：是否今日股权登记 |
| EX_DIVIDEND_TODAY | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `EX_DIVIDEND_TODAY`。 原始字段说明：是否今日除权除息 |
| PAYCASH_TODAY | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `PAYCASH_TODAY`。 原始字段说明：是否今日派息 |
| IS_PAYCASH | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IS_PAYCASH`。 原始字段说明：是否派息 |
| IS_EQUITY_RECENT | Bool | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `IS_EQUITY_RECENT`。 原始字段说明：是否近期股权登记 |
| LAST_TRADE_DATE | Nullable(Date) | 待补充 | 待补充 | 待补充 | 来自 `eastmoney` 原始字段 `LAST_TRADE_DATE`。 原始字段说明：最后交易日 |

## 5. 关键字段发现

### 证券代码字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `INFO_CODE`
- 观察到的格式：待补充
- 无效样例：待补充
- 建议 staging 处理：待补充

### 日期与时间字段

- 已画像字段：`NOTICE_DATE`, `EQUITY_RECORD_DATE`, `EX_DIVIDEND_DATE`, `PAY_CASH_DATE`
- 范围：待补充
- 无效值或占位值：待补充
- 建议 staging 处理：待补充

### 枚举字段

- 已画像字段：`SECUCODE`, `SECURITY_CODE`, `SECURITY_NAME_ABBR`, `IMPL_PLAN_PROFILE`, `ASSIGN_PROGRESS`, `IS_UNASSIGN`
- 取值：待补充
- 未知或异常取值：待补充
- 建议 staging 处理：待补充

### 数值字段

- 已画像字段：`TOTAL_DIVIDEND`, `TOTAL_DIVIDEND_A`
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
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:31:56  Running with dbt=1.11.11
21:31:56  Registered adapter: clickhouse=1.10.0
21:31:56  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:31:57  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:31:57
21:31:57  Concurrency: 1 threads (target='dev')
21:31:57
Previewing inline node:
| SECUCODE  | SECURITY_CODE | SECURITY_NAME_ABBR | NOTICE_DATE | IMPL_PLAN_PROFILE | ASSIGN_PROGRESS | ... |
| --------- | ------------- | ------------------ | ----------- | ----------------- | --------------- | --- |
| 000001.SZ | 000001        | 平安银行               |  1991-12-31 | 10送5派2元           | 实施方案            | ... |
| 000002.SZ | 000002        | 万科A                |  1991-05-27 | 10送2              | 实施方案            | ... |
| 000004.SZ | 000004        | *ST国华              |  1991-06-30 | 10送2              | 实施方案            | ... |
| 600601.SH | 600601        | 方正科技               |  1991-12-31 |                   | 实施方案            | ... |
| 600651.SH | 600651        | 飞乐音响               |  1991-12-31 |                   | 实施方案            | ... |
| 600653.SH | 600653        | 申华控股               |  1991-12-31 |                   | 实施方案            | ... |
| 000002.SZ | 000002        | 万科A                |  1992-06-30 | 10送2              | 实施方案            | ... |
| 000002.SZ | 000002        | 万科A                |  1992-06-30 | 不分配不转增            | 董事会预案           | ... |
| 000004.SZ | 000004        | *ST国华              |  1992-04-01 | 10送2              | 实施方案            | ... |
| 000007.SZ | 000007        | 全新好                |  1992-05-15 | 10送2派0.5元         | 实施方案            | ... |
| 000011.SZ | 000011        | 深物业A               |  1992-08-20 | 不分配不转增            | 董事会预案           | ... |
| 000012.SZ | 000012        | 南玻A                |  1992-08-13 | 不分配不转增            | 董事会预案           | ... |
| 000016.SZ | 000016        | *ST康佳A             |  1992-08-18 | 不分配不转增            | 董事会预案           | ... |
| 000017.SZ | 000017        | 深中华A               |  1992-08-31 | 不分配不转增            | 董事会预案           | ... |
| 000018.SZ | 000018        | 神城A退               |  1992-08-29 | 不分配不转增            | 董事会预案           | ... |
| 000020.SZ | 000020        | 深华发A               |  1992-08-25 | 不分配不转增            | 董事会预案           | ... |
| 000504.SZ | 000504        | *ST生物              |  1992-12-31 | 10送3派0.6元         | 实施方案            | ... |
| 600601.SH | 600601        | 方正科技               |  1992-06-30 | 不分配不转增            | 董事会预案           | ... |
| 600602.SH | 600602        | 云赛智联               |  1992-03-29 | 10派100元           | 实施方案            | ... |
| 600602.SH | 600602        | 云赛智联               |  1992-06-30 | 不分配不转增            | 董事会预案           | ... |
| 600603.SH | 600603        | 广汇物流               |  1992-06-30 | 不分配不转增            | 董事会预案           | ... |
| 600604.SH | 600604        | 市北高新               |  1992-06-30 | 不分配不转增            | 董事会预案           | ... |
| 600607.SH | 600607        | 上实医药               |  1992-06-30 | 不分配不转增            | 董事会预案           | ... |
| 600608.SH | 600608        | *ST沪科              |  1992-06-30 | 不分配不转增            | 董事会预案           | ... |
| 600609.SH | 600609        | 金杯汽车               |  1992-12-18 | 10送3              | 实施方案            | ... |
| 600609.SH | 600609        | 金杯汽车               |  1992-12-31 | 不分配不转增            | 实施方案            | ... |
| 600654.SH | 600654        | 中安科                |  1992-06-30 |                   | 实施方案            | ... |
| 000001.SZ | 000001        | 平安银行               |  1993-05-09 | 10送3.5转5派3元       | 实施方案            | ... |
| 000001.SZ | 000001        | 平安银行               |  1993-08-11 | 不分配不转增            | 董事会预案           | ... |
| 000002.SZ | 000002        | 万科A                |  1993-03-25 | 10送5派0.6元         | 实施方案            | ... |
| 000002.SZ | 000002        | 万科A                |  1993-08-22 | 不分配不转增            | 董事会预案           | ... |
| 000003.SZ | 000003        | PT金田A              |  1993-06-30 | 不分配不转增            | 董事会预案           | ... |
| 000004.SZ | 000004        | *ST国华              |  1993-05-12 | 10送3派0.8元         | 实施方案            | ... |
| 000006.SZ | 000006        | 深振业A               |  1993-05-26 | 10送3              | 实施方案            | ... |
| 000007.SZ | 000007        | 全新好                |  1993-05-16 | 10送2              | 实施方案            | ... |
| 000008.SZ | 000008        | 神州高铁               |  1993-05-19 | 10送1.5派0.5元       | 实施方案            | ... |
| 000009.SZ | 000009        | 中国宝安               |  1993-01-01 | 10送3派0.9元         | 实施方案            | ... |
| 000011.SZ | 000011        | 深物业A               |  1993-06-30 | 10送3派0.08元        | 实施方案            | ... |
| 000011.SZ | 000011        | 深物业A               |  1993-08-21 | 不分配不转增            | 董事会预案           | ... |
| 000012.SZ | 000012        | 南玻A                |  1993-05-13 | 10送3派0.7元         | 实施方案            | ... |
| 000012.SZ | 000012        | 南玻A                |  1993-07-29 | 不分配不转增            | 董事会预案           | ... |
| 000013.SZ | 000013        | *ST石化A             |  1993-06-06 | 不分配不转增            | 董事会预案           | ... |
| 000013.SZ | 000013        | *ST石化A             |  1993-06-06 | 10送2派0.57元        | 实施方案            | ... |
| 000013.SZ | 000013        | *ST石化A             |  1993-07-30 | 不分配不转增            | 董事会预案           | ... |
| 000014.SZ | 000014        | 沙河股份               |  1993-04-17 | 10送3派0.8元         | 实施方案            | ... |
| 000016.SZ | 000016        | *ST康佳A             |  1993-04-15 | 10送3.5派0.9元       | 实施方案            | ... |
| 000016.SZ | 000016        | *ST康佳A             |  1993-08-18 | 不分配不转增            | 董事会预案           | ... |
| 000017.SZ | 000017        | 深中华A               |  1993-08-20 | 10送3派0.81元        | 实施方案            | ... |
| 000017.SZ | 000017        | 深中华A               |  1993-08-20 | 不分配不转增            | 董事会预案           | ... |
| 000018.SZ | 000018        | 神城A退               |  1993-05-01 | 10送1派0.6元         | 实施方案            | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:32:00  Running with dbt=1.11.11
21:32:00  Registered adapter: clickhouse=1.10.0
21:32:01  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:32:01  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:01
21:32:01  Concurrency: 1 threads (target='dev')
21:32:01
Previewing inline node:
| row_count |
| --------- |
|    151606 |
```

### 日期范围

```sql
select
    min(`NOTICE_DATE`) as min_notice_date,
    max(`NOTICE_DATE`) as max_notice_date,
    countIf(isNull(`NOTICE_DATE`)) as null_notice_date,
    countIf(`NOTICE_DATE` = toDate('1970-01-01')) as placeholder_notice_date,
    min(`EQUITY_RECORD_DATE`) as min_equity_record_date,
    max(`EQUITY_RECORD_DATE`) as max_equity_record_date,
    countIf(isNull(`EQUITY_RECORD_DATE`)) as null_equity_record_date,
    countIf(`EQUITY_RECORD_DATE` = toDate('1970-01-01')) as placeholder_equity_record_date,
    min(`EX_DIVIDEND_DATE`) as min_ex_dividend_date,
    max(`EX_DIVIDEND_DATE`) as max_ex_dividend_date,
    countIf(isNull(`EX_DIVIDEND_DATE`)) as null_ex_dividend_date,
    countIf(`EX_DIVIDEND_DATE` = toDate('1970-01-01')) as placeholder_ex_dividend_date,
    min(`PAY_CASH_DATE`) as min_pay_cash_date,
    max(`PAY_CASH_DATE`) as max_pay_cash_date,
    countIf(isNull(`PAY_CASH_DATE`)) as null_pay_cash_date,
    countIf(`PAY_CASH_DATE` = toDate('1970-01-01')) as placeholder_pay_cash_date
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:32:05  Running with dbt=1.11.11
21:32:05  Registered adapter: clickhouse=1.10.0
21:32:05  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:32:06  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:06
21:32:06  Concurrency: 1 threads (target='dev')
21:32:06
Previewing inline node:
| min_notice_date | max_notice_date | null_notice_date | placeholder_notic... | min_equity_record... | max_equity_record... | ... |
| --------------- | --------------- | ---------------- | -------------------- | -------------------- | -------------------- | --- |
|      1991-05-27 |      2026-06-02 |                0 |                    0 |           1991-05-27 |           2026-07-10 | ... |
```

### 格式分布：SECUCODE

```sql
select
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECUCODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECUCODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECUCODE`) or toString(`SECUCODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:32:09  Running with dbt=1.11.11
21:32:10  Registered adapter: clickhouse=1.10.0
21:32:10  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:32:11  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:11
21:32:11  Concurrency: 1 threads (target='dev')
21:32:11
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|           151606 |             0 |            0 |             0 |    151606 |
```

### 格式分布：SECURITY_CODE

```sql
select
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`SECURITY_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`SECURITY_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`SECURITY_CODE`) or toString(`SECURITY_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:32:14  Running with dbt=1.11.11
21:32:14  Registered adapter: clickhouse=1.10.0
21:32:15  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:32:15  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:15
21:32:15  Concurrency: 1 threads (target='dev')
21:32:15
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |       151606 |             0 |    151606 |
```

### 格式分布：INFO_CODE

```sql
select
    countIf(match(toString(`INFO_CODE`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`INFO_CODE`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`INFO_CODE`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`INFO_CODE`) or toString(`INFO_CODE`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:32:18  Running with dbt=1.11.11
21:32:19  Registered adapter: clickhouse=1.10.0
21:32:19  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:32:19  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:19
21:32:19  Concurrency: 1 threads (target='dev')
21:32:19
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |             0 |            0 |         70499 |    151606 |
```

### 高频取值：SECUCODE

```sql
select
    `SECUCODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
group by `SECUCODE`
order by row_count desc
```


结果（成功）：

```text
21:32:23  Running with dbt=1.11.11
21:32:23  Registered adapter: clickhouse=1.10.0
21:32:23  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:32:24  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:24
21:32:24  Concurrency: 1 threads (target='dev')
21:32:24
Previewing inline node:
| value     | row_count |
| --------- | --------- |
| 000002.SZ |        71 |
| 600601.SH |        70 |
| 600663.SH |        70 |
| 000020.SZ |        70 |
| 600654.SH |        70 |
| 600610.SH |        70 |
| 000001.SZ |        70 |
| 600602.SH |        70 |
| 600613.SH |        69 |
| 600637.SH |        69 |
| 000017.SZ |        69 |
| 600608.SH |        69 |
| 000012.SZ |        69 |
| 000011.SZ |        69 |
| 600660.SH |        69 |
| 600615.SH |        69 |
| 600686.SH |        69 |
| 600604.SH |        69 |
| 600609.SH |        69 |
| 600612.SH |        69 |
```

### 高频取值：SECURITY_CODE

```sql
select
    `SECURITY_CODE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
group by `SECURITY_CODE`
order by row_count desc
```


结果（成功）：

```text
21:32:27  Running with dbt=1.11.11
21:32:28  Registered adapter: clickhouse=1.10.0
21:32:28  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:32:28  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:28
21:32:28  Concurrency: 1 threads (target='dev')
21:32:28
Previewing inline node:
| value  | row_count |
| ------ | --------- |
| 000002 |        71 |
| 600610 |        70 |
| 600602 |        70 |
| 600654 |        70 |
| 600601 |        70 |
| 000001 |        70 |
| 000020 |        70 |
| 600663 |        70 |
| 000012 |        69 |
| 600615 |        69 |
| 600608 |        69 |
| 600612 |        69 |
| 000016 |        69 |
| 600604 |        69 |
| 000011 |        69 |
| 600660 |        69 |
| 000017 |        69 |
| 600613 |        69 |
| 600637 |        69 |
| 600686 |        69 |
```

### 高频取值：SECURITY_NAME_ABBR

```sql
select
    `SECURITY_NAME_ABBR` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
group by `SECURITY_NAME_ABBR`
order by row_count desc
```


结果（成功）：

```text
21:32:32  Running with dbt=1.11.11
21:32:32  Registered adapter: clickhouse=1.10.0
21:32:32  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:32:33  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:33
21:32:33  Concurrency: 1 threads (target='dev')
21:32:33
Previewing inline node:
| value | row_count |
| ----- | --------- |
| 东方明珠  |       113 |
| 百联股份  |       104 |
| 万科A   |        71 |
| 深华发A  |        70 |
| 中毅达   |        70 |
| 方正科技  |        70 |
| 陆家嘴   |        70 |
| 云赛智联  |        70 |
| 平安银行  |        70 |
| 中安科   |        70 |
| 市北高新  |        69 |
| 金杯汽车  |        69 |
| 鑫源智造  |        69 |
| 深物业A  |        69 |
| *ST沪科 |        69 |
| 神奇制药  |        69 |
| 老凤祥   |        69 |
| 金龙汽车  |        69 |
| 深中华A  |        69 |
| 福耀玻璃  |        69 |
```

### 高频取值：IMPL_PLAN_PROFILE

```sql
select
    `IMPL_PLAN_PROFILE` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
group by `IMPL_PLAN_PROFILE`
order by row_count desc
```


结果（成功）：

```text
21:32:36  Running with dbt=1.11.11
21:32:37  Registered adapter: clickhouse=1.10.0
21:32:37  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:32:37  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:37
21:32:37  Concurrency: 1 threads (target='dev')
21:32:37
Previewing inline node:
| value   | row_count |
| ------- | --------- |
| 不分配不转增  |     93454 |
| 10派1元   |      5205 |
| 10派2元   |      2955 |
| 10派0.5元 |      2871 |
| 10派1.5元 |      2150 |
| 10派3元   |      1777 |
| 10派0.2元 |      1299 |
| 10派0.3元 |      1298 |
| 10派0.6元 |      1125 |
| 10派2.5元 |      1085 |
| 10派0.8元 |      1010 |
| 10派1.2元 |      1010 |
| 10派5元   |       982 |
| 10派0.1元 |       824 |
| 10派4元   |       802 |
| 10派0.4元 |       726 |
| 10派0.7元 |       659 |
| 10派1.8元 |       514 |
| 10派3.5元 |       477 |
| 10派6元   |       454 |
```

### 高频取值：ASSIGN_PROGRESS

```sql
select
    `ASSIGN_PROGRESS` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
group by `ASSIGN_PROGRESS`
order by row_count desc
```


结果（成功）：

```text
21:32:41  Running with dbt=1.11.11
21:32:41  Registered adapter: clickhouse=1.10.0
21:32:41  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:32:42  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:42
21:32:42  Concurrency: 1 threads (target='dev')
21:32:42
Previewing inline node:
| value    | row_count |
| -------- | --------- |
| 董事会预案    |     69280 |
| 实施方案     |     55806 |
| 股东大会预案   |     26108 |
| 预披露      |       409 |
| 股东大会否决   |         2 |
| 董事会决议未通过 |         1 |
```

### 高频取值：IS_UNASSIGN

```sql
select
    `IS_UNASSIGN` as value,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
group by `IS_UNASSIGN`
order by row_count desc
```


结果（成功）：

```text
21:32:45  Running with dbt=1.11.11
21:32:46  Registered adapter: clickhouse=1.10.0
21:32:46  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:32:46  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:46
21:32:46  Concurrency: 1 threads (target='dev')
21:32:46
Previewing inline node:
| value | row_count |
| ----- | --------- |
|  True |     93454 |
| False |     58152 |
```

### 数值范围：TOTAL_DIVIDEND

```sql
select
    min(`TOTAL_DIVIDEND`) as min_value,
    max(`TOTAL_DIVIDEND`) as max_value,
    countIf(`TOTAL_DIVIDEND` = 0) as zero_count,
    countIf(`TOTAL_DIVIDEND` < 0) as negative_count,
    countIf(isNull(`TOTAL_DIVIDEND`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:32:50  Running with dbt=1.11.11
21:32:50  Registered adapter: clickhouse=1.10.0
21:32:50  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.intermediate
- models.elt.marts
21:32:51  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:51
21:32:51  Concurrency: 1 threads (target='dev')
21:32:51
Previewing inline node:
| min_value |       max_value | zero_count | negative_count | null_count | row_count |
| --------- | --------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 110,593,000,000 |      55013 |              0 |        434 |    151606 |
```

### 数值范围：TOTAL_DIVIDEND_A

```sql
select
    min(`TOTAL_DIVIDEND_A`) as min_value,
    max(`TOTAL_DIVIDEND_A`) as max_value,
    countIf(`TOTAL_DIVIDEND_A` = 0) as zero_count,
    countIf(`TOTAL_DIVIDEND_A` < 0) as negative_count,
    countIf(isNull(`TOTAL_DIVIDEND_A`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'eastmoney__dividend_main') }}
```


结果（成功）：

```text
21:32:54  Running with dbt=1.11.11
21:32:55  Registered adapter: clickhouse=1.10.0
21:32:55  [WARNING]: Configuration paths exist in your dbt_project.yml file which do not apply to any resources.
There are 2 unused configuration paths:
- models.elt.marts
- models.elt.intermediate
21:32:55  Found 3 models, 3 operations, 9 data tests, 1 sql operation, 15 sources, 528 macros
21:32:55
21:32:55  Concurrency: 1 threads (target='dev')
21:32:55
Previewing inline node:
| min_value |      max_value | zero_count | negative_count | null_count | row_count |
| --------- | -------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 83,661,000,000 |      55055 |              0 |        393 |    151606 |
```
