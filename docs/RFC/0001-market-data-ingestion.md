# RFC 0001: 行情数据采集资产设计

状态：草案

## 摘要

本文定义 Mono Fleur 第一阶段行情数据采集设计。

初始范围包含三个上游数据资产：

1. `sina__trade_calendar`：来自新浪财经的 A 股交易日历。
2. `baostock__query_stock_basic`：来自 BaoStock TCP 的证券基础信息。
3. `baostock__query_history_k_data_plus_daily`：来自 BaoStock TCP 的日频不复权 K 线行情。

设计目标是在保持 Dagster 可观测性和回填控制简单的同时，避免低效的“按证券代码、按日期逐个请求”导致请求量爆炸。

## 目标

- 维护一份本地交易日历，用于驱动调度和分区选择。
- 以可复现的快照形式保存证券基础信息。
- 以 Parquet 保存日频不复权 K 线数据，并按年份分区。
- 支持收盘后的日频增量采集。
- 支持按年执行高效历史回填。
- 保持 Web 页面查询和回测 worker 读取方式与后续 ClickHouse 查询层兼容。

## 非目标

- 分钟级 K 线采集。
- 前复权或后复权 K 线变体。
- 实时报价。
- 除权除息和公司行为归一化。
- 完整 dbt 数仓建模。

## 数据源

### `sina__trade_calendar`

数据源：

```text
GET https://finance.sina.com.cn/realstock/company/klc_td_sh.txt
```

参考文档：

```text
docs/references/remote_endpoint/sina__calendar.md
docs/references/openapi/sina__calendar.yaml
```

响应是一个 JavaScript 变量，内部包含编码后的交易日期。`SinaCalendarParser` 负责将 payload 解码为交易日历数组。

解析后的输出是二维数组，每一行只有一个交易日期字符串：

```text
[
  ["1990-12-19"],
  ["1990-12-20"],
  ["1990-12-21"]
]
```

数组内日期格式为 `YYYY-MM-DD`。


### `baostock__query_stock_basic`

数据源：

```text
BaoStock TCP query_stock_basic
```

参考文档：

```text
docs/references/remote_server/baostock_tcp_server.md
```

重要字段：

```text
code
code_name
ipoDate
outDate
type
status
```

该资产表示证券基础信息。可以每天或每周刷新。K 线采集会使用它判断某个日期范围内哪些证券代码是有效的。

### `baostock__query_history_k_data_plus_daily`

数据源：

```text
BaoStock TCP query_history_k_data_plus
```

日频不复权参数：

```text
frequency = "d"
adjustflag = "3"
```

字段：

```text
date
code
open
high
low
close
preclose
volume
amount
adjustflag
turn
tradestatus
pctChg
isST
```

接口参数：

```text
code
start_date
end_date
frequency
adjustflag
```

这一点很重要：历史回填时应该按“单个 code + 日期范围”请求，而不是按“单个 code + 单个日期”请求。

## 存储布局

### Raw Parquet

日频 K 线 raw 数据按年份写入。日常交易日调度会重写当年文件，历史回填会重写目标年份文件：

```text
data/raw/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```

## Dagster 资产设计

### 资产概览

```text
sina__trade_calendar
  输出：本地交易日历
  分区：不分区
  调度：每年的最后一周

baostock__query_stock_basic
  输出：证券基础信息快照
  分区：不分区
  调度：当前交易日 17:35 

baostock__query_history_k_data_plus_daily
  输出：日频不复权 K 线 raw parquet
  分区：year
  调度：交易日调度器在交易日收盘后刷新当年分区
  回填：显式 materialize 目标 year 分区
  路径：raw/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```


## 数据质量检查

不做数据质量检查

## 实现顺序

1. 实现 `sina__trade_calendar`。
2. 持久化本地交易日历。
3. 实现 `baostock__query_stock_basic`。
4. 实现可复用交易日调度器。
5. 实现 `baostock__query_history_k_data_plus_daily` 年度分区模式。

