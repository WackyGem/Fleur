# Raw 数据画像：baostock__query_history_k_data_plus_daily

日期：2026-06-02

状态：Accepted

关联：

- 数据契约：`pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml`
- dbt source：`source('raw', 'baostock__query_history_k_data_plus_daily')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：`pipeline/elt/models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.sql`

## 1. 范围

- source 名称：`raw`
- raw 表：`baostock__query_history_k_data_plus_daily`
- profiling 命令：`cd pipeline && uv run python elt/scripts/profile_raw_source.py --source raw --table baostock__query_history_k_data_plus_daily --execute --output ../docs/references/raw_profile/baostock__query_history_k_data_plus_daily.md`
- 行数：20,335,243
- 数据范围：`1990-12-19` 至 `2026-06-01`
- 分区范围：按自然年生成的 raw 分区已汇总到同一张 ClickHouse raw 表
- 契约数据集：`baostock__query_history_k_data_plus_daily`
- ClickHouse raw 表：`raw.baostock__query_history_k_data_plus_daily`
- 表说明：BaoStock 日 K 线数据，raw 侧按自然年分区采集。

## 2. 粒度与键

- 观察到的粒度：每个 BaoStock 证券代码、每个交易日一行
- 候选自然键：`code`, `date`
- 重复检查：profiling 查询未返回重复的 `(code, date)`
- 粒度注意事项：价格和成交量字段存在零值；staging 不应在没有字段级策略时静默过滤这些记录

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|
| date | Date | 0 | 不适用 | 1990-12-19 至 2026-06-01 | 来自 `baostock` 原始字段 `date`。原始字段说明：BaoStock 行情接口返回的交易日期。 |
| code | LowCardinality(String) | 0 | 0 个空字符串/NULL | 所有行均匹配 `sh.600000`/`sz.000001` 这类供应商前缀格式 | 来自 `baostock` 原始字段 `code`。原始字段说明：BaoStock 行情接口返回的证券代码。 |
| open | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `open`。原始字段说明：交易日开盘价。 |
| high | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `high`。原始字段说明：交易日最高价。 |
| low | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `low`。原始字段说明：交易日最低价。 |
| close | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `close`。原始字段说明：交易日收盘价。 |
| preclose | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `preclose`。原始字段说明：上一交易日收盘价。 |
| volume | Int64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `volume`。原始字段说明：交易日成交量。 |
| amount | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `amount`。原始字段说明：交易日成交金额。 |
| adjustflag | Int8 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `adjustflag`。原始字段说明：行情复权标记，用于区分不复权、前复权和后复权。 |
| turn | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `turn`。原始字段说明：交易日换手率。 |
| tradestatus | Int8 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `tradestatus`。原始字段说明：交易日交易状态。 |
| pctChg | Float64 | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `pctChg`。原始字段说明：交易日涨跌幅。 |
| isST | Bool | 待补充 | 待补充 | 待补充 | 来自 `baostock` 原始字段 `isST`。原始字段说明：证券是否为 ST 或风险警示状态。 |

## 4. 关键字段发现

### 证券代码字段

- 已画像字段：`code`
- 观察到的格式：20,335,243 行全部匹配小写交易所前缀格式，例如 `sh.600601`
- 无效样例：正则分布查询未发现无效样例
- 建议 staging 处理：使用 `normalize_cn_security_code('code', input_format='baostock_prefix')`；基于同一个 raw 字段派生 `security_local_code` 和 `exchange_code`

### 日期与时间字段

- 已画像字段：`date`
- 范围：`1990-12-19` 至 `2026-06-01`
- 无效值或占位值：NULL 计数 profiling 未发现无效值
- 建议 staging 处理：将 `date` 直接映射为规范字段 `trade_date`

### 枚举字段

- 已画像字段：`adjustflag`, `tradestatus`, `isST`
- 取值：`adjustflag` 全部为 `3`；`tradestatus` 包含 `1` 和 `0`；`isST` 包含 `False` 和 `True`
- 未知或异常取值：已画像枚举字段未观察到未知或异常取值
- 建议 staging 处理：当前 baseline staging 不暴露这些字段；后续暴露时应记录取值域，并增加定向 accepted-values 测试

### 数值字段

- 已画像字段：`open`, `close`, `volume`, `amount`
- 最小/最大值：已画像 `open`、`close`、`volume` 和 `amount`；未观察到负值
- 负数/零值/极端值：价格和成交量/成交金额字段存在零值；零成交量/成交金额较常见，过滤前需要业务域确认
- 单位假设：除非字段显式补充单位元数据，否则 staging 保留 BaoStock 原始单位
- 建议 staging 处理：当前 baseline staging 仅暴露证券和日期字段；价格、成交量字段后续加入时必须同步明确单位和零值策略

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|
| `code` 使用供应商前缀格式 | 中 | 正则分布显示所有行均为 `sh.`/`sz.` 前缀，没有 canonical 后缀格式 | 标准化为 `security_code`，并拆出本地代码和交易所字段 | 无 |
| 价格和成交字段存在零值 | 中 | 数值范围查询显示价格、成交量和成交金额存在零值 | 在字段级策略明确前，不暴露也不过滤这些字段 | 价格/成交量质量规则放到后续价格 staging 增强中处理 |

## 6. 建议的 Staging 转换

- 重命名：`date -> trade_date`; `code -> security_code`, `security_local_code`, `exchange_code`
- 类型转换：当前暴露字段无需转换；raw `date` 已经是 `Date`
- 标准化：使用 `normalize_cn_security_code`、`cn_security_local_code` 和 `cn_exchange_code`，`input_format` 为 `baostock_prefix`
- NULL 处理：当前暴露字段未观察到 NULL
- 测试：`security_code` 添加 `not_null` + `cn_security_code_format`；`trade_date` 添加 `not_null`；`exchange_code` 添加 accepted values（允许值）`SH`、`SZ`、`BJ`
- YAML 元数据：source lineage（源字段血缘）指向 `raw.baostock__query_history_k_data_plus_daily.code` 和 `.date`；代码派生字段补充 normalization metadata（标准化元数据）

## 7. 延后到 Intermediate/Mart

- 跨源 join：staging 不做；与证券主数据的 join 放到 intermediate/mart
- 需要优先级判断的去重：未观察到 `(code, date)` 重复
- 主数据修正：staging 只做确定性的格式标准化，不修正证券标识主数据
- 粒度变化：staging 不做粒度变化
- 业务指标逻辑：价格/成交量分析和零值解释延后处理

## 8. 待确认问题

- [ ] 后续确认价格/成交量零值表示有效无交易记录，还是需要增加质量标记。

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
select *
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:36:32  Running with dbt=1.11.11
15:36:33  Registered adapter: clickhouse=1.10.0
15:36:33  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:33  
15:36:33  Concurrency: 1 threads (target='dev')
15:36:33  
Previewing inline node:
|       date | code      |  open |  high |   low | close | ... |
| ---------- | --------- | ----- | ----- | ----- | ----- | --- |
| 1990-12-19 | sh.600601 | 185.3 | 185.3 | 185.3 | 185.3 | ... |
| 1990-12-20 | sh.600601 | 185.3 | 194.6 | 185.3 | 194.6 | ... |
| 1990-12-21 | sh.600601 | 204.3 | 204.3 | 204.3 | 204.3 | ... |
| 1990-12-24 | sh.600601 | 214.6 | 214.6 | 214.5 | 214.5 | ... |
| 1990-12-25 | sh.600601 | 214.5 | 214.5 | 214.5 | 214.5 | ... |
| 1990-12-26 | sh.600601 | 236.5 | 236.5 | 236.5 | 236.5 | ... |
| 1990-12-27 | sh.600601 | 236.5 | 236.5 | 236.5 | 236.5 | ... |
| 1990-12-28 | sh.600601 | 238.9 | 238.9 | 238.9 | 238.9 | ... |
| 1990-12-31 | sh.600601 | 241.3 | 241.3 | 241.3 | 241.3 | ... |
| 1990-12-19 | sh.600602 | 365.7 | 384.0 | 365.7 | 384.0 | ... |
| 1990-12-20 | sh.600602 | 403.2 | 403.2 | 403.2 | 403.2 | ... |
| 1990-12-21 | sh.600602 | 400.0 | 423.4 | 400.0 | 423.4 | ... |
| 1990-12-24 | sh.600602 | 423.4 | 444.6 | 423.4 | 444.6 | ... |
| 1990-12-25 | sh.600602 | 466.8 | 466.8 | 466.8 | 466.8 | ... |
| 1990-12-26 | sh.600602 | 490.2 | 490.2 | 490.2 | 490.2 | ... |
| 1990-12-27 | sh.600602 | 490.2 | 490.2 | 490.2 | 490.2 | ... |
| 1990-12-28 | sh.600602 | 495.1 | 495.1 | 495.1 | 495.1 | ... |
| 1990-12-31 | sh.600602 | 500.1 | 500.1 | 500.1 | 500.1 | ... |
| 1990-12-19 | sh.600651 | 320.3 | 320.3 | 320.3 | 320.3 | ... |
| 1990-12-20 | sh.600651 | 320.3 | 320.3 | 320.3 | 320.3 | ... |
| 1990-12-21 | sh.600651 | 336.3 | 336.3 | 336.3 | 336.3 | ... |
| 1990-12-24 | sh.600651 | 336.3 | 353.2 | 336.3 | 353.2 | ... |
| 1990-12-25 | sh.600651 | 353.2 | 353.2 | 353.2 | 353.2 | ... |
| 1990-12-26 | sh.600651 | 389.5 | 389.5 | 389.5 | 389.5 | ... |
| 1990-12-27 | sh.600651 | 393.4 | 393.4 | 393.4 | 393.4 | ... |
| 1990-12-28 | sh.600651 | 397.3 | 397.3 | 397.3 | 397.3 | ... |
| 1990-12-31 | sh.600651 | 401.3 | 401.3 | 401.3 | 401.3 | ... |
| 1990-12-19 | sh.600652 | 193.0 | 193.0 | 193.0 | 193.0 | ... |
| 1990-12-20 | sh.600652 | 193.0 | 193.0 | 193.0 | 193.0 | ... |
| 1990-12-21 | sh.600652 | 193.0 | 193.0 | 193.0 | 193.0 | ... |
| 1990-12-24 | sh.600652 | 193.0 | 193.0 | 193.0 | 193.0 | ... |
| 1990-12-25 | sh.600652 | 193.0 | 193.0 | 193.0 | 193.0 | ... |
| 1990-12-26 | sh.600652 | 193.0 | 193.0 | 193.0 | 193.0 | ... |
| 1990-12-27 | sh.600652 | 193.0 | 193.0 | 193.0 | 193.0 | ... |
| 1990-12-28 | sh.600652 | 204.7 | 204.7 | 204.7 | 204.7 | ... |
| 1990-12-31 | sh.600652 | 206.7 | 206.7 | 206.7 | 206.7 | ... |
| 1990-12-19 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-20 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-21 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-24 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-25 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-26 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-27 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-28 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-31 | sh.600653 | 327.9 | 327.9 | 327.9 | 327.9 | ... |
| 1990-12-19 | sh.600654 | 323.5 | 323.5 | 323.5 | 323.5 | ... |
| 1990-12-20 | sh.600654 | 323.5 | 323.5 | 323.5 | 323.5 | ... |
| 1990-12-21 | sh.600654 | 339.7 | 339.7 | 339.7 | 339.7 | ... |
| 1990-12-24 | sh.600654 | 356.7 | 356.7 | 356.7 | 356.7 | ... |
| 1990-12-25 | sh.600654 | 374.6 | 374.6 | 374.6 | 374.6 | ... |
```

### 行数统计

```sql
select count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:36:37  Running with dbt=1.11.11
15:36:37  Registered adapter: clickhouse=1.10.0
15:36:38  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:38  
15:36:38  Concurrency: 1 threads (target='dev')
15:36:38  
Previewing inline node:
| row_count |
| --------- |
|  20335243 |
```

### 日期范围

```sql
select
    min(`date`) as min_date,
    max(`date`) as max_date,
    countIf(isNull(`date`)) as null_date
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:36:42  Running with dbt=1.11.11
15:36:43  Registered adapter: clickhouse=1.10.0
15:36:44  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:44  
15:36:44  Concurrency: 1 threads (target='dev')
15:36:44  
Previewing inline node:
|   min_date |   max_date | null_date |
| ---------- | ---------- | --------- |
| 1990-12-19 | 2026-06-01 |         0 |
```

### 候选键重复检查

```sql
select
    `code`, `date`,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
group by `code`, `date`
having row_count > 1
order by row_count desc
```


结果（成功）：

```text
15:36:47  Running with dbt=1.11.11
15:36:47  Registered adapter: clickhouse=1.10.0
15:36:48  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:48  
15:36:48  Concurrency: 1 threads (target='dev')
15:36:48  
Previewing inline node:
||
|  |
```

### 格式分布：code

```sql
select
    countIf(match(toString(`code`), '^[0-9]{6}\\.(SH|SZ|BJ)$')) as canonical_suffix,
    countIf(match(toString(`code`), '^(sh|sz|bj)\\.[0-9]{6}$')) as vendor_prefix,
    countIf(match(toString(`code`), '^[0-9]{6}$')) as numeric_only,
    countIf(isNull(`code`) or toString(`code`) = '') as empty_or_null,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:36:56  Running with dbt=1.11.11
15:36:56  Registered adapter: clickhouse=1.10.0
15:36:57  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:36:57  
15:36:57  Concurrency: 1 threads (target='dev')
15:36:57  
Previewing inline node:
| canonical_suffix | vendor_prefix | numeric_only | empty_or_null | row_count |
| ---------------- | ------------- | ------------ | ------------- | --------- |
|                0 |      20335243 |            0 |             0 |  20335243 |
```

### 高频取值：adjustflag

```sql
select
    `adjustflag` as value,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
group by `adjustflag`
order by row_count desc
```


结果（成功）：

```text
15:37:02  Running with dbt=1.11.11
15:37:02  Registered adapter: clickhouse=1.10.0
15:37:03  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:03  
15:37:03  Concurrency: 1 threads (target='dev')
15:37:03  
Previewing inline node:
| value | row_count |
| ----- | --------- |
|     3 |  20335243 |
```

### 高频取值：tradestatus

```sql
select
    `tradestatus` as value,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
group by `tradestatus`
order by row_count desc
```


结果（成功）：

```text
15:37:06  Running with dbt=1.11.11
15:37:06  Registered adapter: clickhouse=1.10.0
15:37:07  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:07  
15:37:07  Concurrency: 1 threads (target='dev')
15:37:07  
Previewing inline node:
| value | row_count |
| ----- | --------- |
|     1 |  19748906 |
|     0 |    586337 |
```

### 高频取值：isST

```sql
select
    `isST` as value,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
group by `isST`
order by row_count desc
```


结果（成功）：

```text
15:37:11  Running with dbt=1.11.11
15:37:11  Registered adapter: clickhouse=1.10.0
15:37:12  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:12  
15:37:12  Concurrency: 1 threads (target='dev')
15:37:12  
Previewing inline node:
| value | row_count |
| ----- | --------- |
| False |  19406262 |
|  True |    928981 |
```

### 数值范围：open

```sql
select
    min(`open`) as min_value,
    max(`open`) as max_value,
    countIf(`open` = 0) as zero_count,
    countIf(`open` < 0) as negative_count,
    countIf(isNull(`open`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:37:15  Running with dbt=1.11.11
15:37:15  Registered adapter: clickhouse=1.10.0
15:37:16  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:16  
15:37:16  Concurrency: 1 threads (target='dev')
15:37:16  
Previewing inline node:
| min_value |   max_value | zero_count | negative_count | null_count | row_count |
| --------- | ----------- | ---------- | -------------- | ---------- | --------- |
|         0 | 44,610.455… |      70545 |              0 |          0 |  20335243 |
```

### 数值范围：close

```sql
select
    min(`close`) as min_value,
    max(`close`) as max_value,
    countIf(`close` = 0) as zero_count,
    countIf(`close` < 0) as negative_count,
    countIf(isNull(`close`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:37:21  Running with dbt=1.11.11
15:37:21  Registered adapter: clickhouse=1.10.0
15:37:22  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:22  
15:37:22  Concurrency: 1 threads (target='dev')
15:37:22  
Previewing inline node:
| min_value |  max_value | zero_count | negative_count | null_count | row_count |
| --------- | ---------- | ---------- | -------------- | ---------- | --------- |
|         0 | 43,972.016 |      70545 |              0 |          0 |  20335243 |
```

### 数值范围：volume

```sql
select
    min(`volume`) as min_value,
    max(`volume`) as max_value,
    countIf(`volume` = 0) as zero_count,
    countIf(`volume` < 0) as negative_count,
    countIf(isNull(`volume`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:37:27  Running with dbt=1.11.11
15:37:27  Registered adapter: clickhouse=1.10.0
15:37:28  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:28  
15:37:28  Concurrency: 1 threads (target='dev')
15:37:28  
Previewing inline node:
| min_value |     max_value | zero_count | negative_count | null_count | row_count |
| --------- | ------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 1055245090000 |     665304 |              0 |          0 |  20335243 |
```

### 数值范围：amount

```sql
select
    min(`amount`) as min_value,
    max(`amount`) as max_value,
    countIf(`amount` = 0) as zero_count,
    countIf(`amount` < 0) as negative_count,
    countIf(isNull(`amount`)) as null_count,
    count(*) as row_count
from {{ source('raw', 'baostock__query_history_k_data_plus_daily') }}
```


结果（成功）：

```text
15:37:33  Running with dbt=1.11.11
15:37:33  Registered adapter: clickhouse=1.10.0
15:37:34  Found 3 models, 9 data tests, 1 sql operation, 15 sources, 527 macros
15:37:34  
15:37:34  Concurrency: 1 threads (target='dev')
15:37:34  
Previewing inline node:
| min_value |            max_value | zero_count | negative_count | null_count | row_count |
| --------- | -------------------- | ---------- | -------------- | ---------- | --------- |
|         0 | 3,951,842,694,367.29 |     665306 |              0 |          0 |  20335243 |
```
