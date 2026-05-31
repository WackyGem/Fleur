# RFC 0002: 东方财富 F10 年度分区采集资产设计

状态：草案

## 摘要

本文定义 Mono Fleur 第二阶段东方财富 F10 数据采集设计。

初始范围包含 8 个上游数据资产：

1. `eastmoney__balance`：资产负债表。
2. `eastmoney__cashflow_sq`：现金流量表（单季度）。
3. `eastmoney__cashflow_ytd`：现金流量表（累计）。
4. `eastmoney__dividend_allotment`：配股明细。
5. `eastmoney__dividend_main`：分红方案明细。
6. `eastmoney__equity_history`：股本变动历史。
7. `eastmoney__income_sq`：利润表（单季度）。
8. `eastmoney__income_ytd`：利润表（累计）。

设计目标是在复用 RFC 0001 已落地 raw Parquet、year 分区、Dagster asset、S3 IO manager 和退避重试模式的基础上，用 `aiohttp` 并行请求东方财富接口，并避免分页排序不稳定导致重复数据或漏数。

## 目标

- 以 Dagster asset 管理 8 个东方财富 F10 raw 数据资产。
- 每个资产按 `year` 分区，写入 S3 Parquet。
- 每个远端请求固定为“单个证券代码 + 开始日期 + 结束日期”。
- 依赖 `baostock__query_stock_basic` 最新快照过滤股票范围。
- 使用 `aiohttp` 按证券代码并行请求，并限制单 asset run 内 code 并发度。
- 自动处理东方财富接口分页。
- 按 OpenAPI 文档要求使用稳定排序，避免跨页重复。
- 使用自然日调度，每天盘后 16:00 启动 8 个资产的日频刷新。
- 8 个资产在日频 job 内按固定顺序执行，一个完成后再执行下一个。
- 使用 `eastmoney_run_pool` 控制 EastMoney 相关 job 并发，最大 3 个 job 并行。

## 非目标

- 不实现东方财富接口以外的数据源。
- 不实现 ClickHouse 建表或加载。
- 不实现 dbt 数仓建模。
- 不在本 RFC 中复制 8 个宽表的完整字段清单；字段名以 `docs/references/openapi` 为准，字段类型统一保留为 string。
- 不设计网页查询接口。
- 不改变 RFC 0001 的 BaoStock K 线资产语义。

## 参考资料

接口说明：

```text
docs/references/remote_endpoint/eastmoney__balance.md
docs/references/remote_endpoint/eastmoney__cashflow_sq.md
docs/references/remote_endpoint/eastmoney__cashflow_ytd.md
docs/references/remote_endpoint/eastmoney__dividend_allotment.md
docs/references/remote_endpoint/eastmoney__dividend_main.md
docs/references/remote_endpoint/eastmoney__equity_history.md
docs/references/remote_endpoint/eastmoney__income_sq.md
docs/references/remote_endpoint/eastmoney__income_ytd.md
```

OpenAPI schema：

```text
docs/references/openapi/eastmoney__balance.yaml
docs/references/openapi/eastmoney__cashflow_sq.yaml
docs/references/openapi/eastmoney__cashflow_ytd.yaml
docs/references/openapi/eastmoney__dividend_allotment.yaml
docs/references/openapi/eastmoney__dividend_main.yaml
docs/references/openapi/eastmoney__equity_history.yaml
docs/references/openapi/eastmoney__income_sq.yaml
docs/references/openapi/eastmoney__income_ytd.yaml
```

项目内设计约束：

```text
docs/RFC/0001-market-data-ingestion.md
docs/ADR/0001-market-data-raw-assets-on-dagster.md
docs/ADR/0002-s3-parquet-storage-layout.md
docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md
```

`aiohttp` 当前文档要点：

- 复用单个 `aiohttp.ClientSession`。
- 使用 `aiohttp.ClientTimeout` 配置总超时、连接超时和读取超时。
- 使用 `aiohttp.TCPConnector(limit=..., limit_per_host=...)` 限制连接池并发。

## 资产矩阵

| Asset | 参考 endpoint | OpenAPI | 接口族 | 固定标识 | 日期过滤字段 | 稳定排序 |
| --- | --- | --- | --- | --- | --- | --- |
| `eastmoney__balance` | `eastmoney__balance.md` | `eastmoney__balance.yaml` | `data/get` | `type=RPT_F10_FINANCE_GBALANCE`, `sty=F10_FINANCE_GBALANCE` | `NOTICE_DATE` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` |
| `eastmoney__cashflow_sq` | `eastmoney__cashflow_sq.md` | `eastmoney__cashflow_sq.yaml` | `data/get` | `type=RPT_F10_FINANCE_GCASHFLOWQC`, `sty=PC_F10_GCASHFLOWQC` | `NOTICE_DATE` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` |
| `eastmoney__cashflow_ytd` | `eastmoney__cashflow_ytd.md` | `eastmoney__cashflow_ytd.yaml` | `data/get` | `type=RPT_F10_FINANCE_GCASHFLOW`, `sty=APP_F10_GCASHFLOW` | `NOTICE_DATE` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` |
| `eastmoney__dividend_allotment` | `eastmoney__dividend_allotment.md` | `eastmoney__dividend_allotment.yaml` | `data/v1/get` | `reportName=RPT_F10_DIVIDEND_ALLOTMENT`, `columns=ALL` | `NOTICE_DATE` | `sortColumns=NOTICE_DATE,SECURITY_CODE`, `sortTypes=-1,-1` |
| `eastmoney__dividend_main` | `eastmoney__dividend_main.md` | `eastmoney__dividend_main.yaml` | `data/v1/get` | `reportName=RPT_F10_DIVIDEND_MAIN`, `columns=ALL` | `NOTICE_DATE` | `sortColumns=NOTICE_DATE,SECURITY_CODE`, `sortTypes=-1,-1` |
| `eastmoney__equity_history` | `eastmoney__equity_history.md` | `eastmoney__equity_history.yaml` | `data/v1/get` | `reportName=RPT_F10_EH_EQUITY`, `columns=ALL` | `NOTICE_DATE` | `sortColumns=NOTICE_DATE,SECURITY_CODE`, `sortTypes=-1,-1` |
| `eastmoney__income_sq` | `eastmoney__income_sq.md` | `eastmoney__income_sq.yaml` | `data/get` | `type=RPT_F10_FINANCE_GINCOMEQC`, `sty=PC_F10_GINCOMEQC` | `NOTICE_DATE` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` |
| `eastmoney__income_ytd` | `eastmoney__income_ytd.md` | `eastmoney__income_ytd.yaml` | `data/get` | `type=RPT_F10_FINANCE_GINCOME`, `sty=APP_F10_GINCOME` | `NOTICE_DATE` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` |

### 日期过滤字段决策

8 个资产的 `year` 分区统一表示采集窗口的自然公告年份，过滤字段使用 `NOTICE_DATE`。

原因：

- 日频调度是自然日盘后刷新，目标是捕获当天或近期公告的新数据。
- 财务报表记录可能在当前自然年公告上一年度报告。如果按 `REPORT_DATE` 分区，日频刷新当前年分区会漏掉上一年度报告的最新公告。
- 8 个参考 OpenAPI 均支持按 `NOTICE_DATE` 区间过滤。
- 记录中仍保留 `REPORT_DATE`，下游可按报告期建模。

分区、日频刷新和历史回填方案与 BaoStock K 线年分区资产对齐：日频调度刷新当年分区并截止到调度自然日，历史回填显式 materialize 目标 `year` 分区并覆盖整年窗口。

## 接口族

### `data/get`

请求地址：

```text
GET https://datacenter.eastmoney.com/securities/api/data/get
```

固定参数：

```text
type=<asset-specific>
sty=<asset-specific>
p=<page number, from 1>
ps=<page size>
sr=-1,-1
st=REPORT_DATE,SECURITY_CODE
source=HSF10
client=PC
```

过滤参数：

```text
(SECUCODE="<code>")(NOTICE_DATE>='<start_date>')(NOTICE_DATE<='<end_date>')
```

响应读取：

- 数据位于 `result.data`。
- 总页数位于 `result.pages`。
- 如果 `result` 或 `result.data` 为空，视为当前 code + date range 无数据。

### `data/v1/get`

请求地址：

```text
GET https://datacenter.eastmoney.com/securities/api/data/v1/get
```

固定参数：

```text
reportName=<asset-specific>
columns=ALL
pageNumber=<page number, from 1>
pageSize=<page size>
sortColumns=NOTICE_DATE,SECURITY_CODE
sortTypes=-1,-1
source=HSF10
client=PC
```

过滤参数：

```text
(SECUCODE="<code>")(NOTICE_DATE>='<start_date>')(NOTICE_DATE<='<end_date>')
```

响应读取：

- 数据位于 `result.data`。
- 总页数位于 `result.pages`。
- `eastmoney__dividend_allotment` 无数据时可能返回 `result: null, code: 9201`，应视为空结果而不是错误。

## 证券过滤

8 个资产依赖已物化的 `baostock__query_stock_basic` 最新快照。

过滤规则：

- 复用 `pipeline/scheduler/src/scheduler/defs/util.py` 中已有的 `filter_active_security_ranges`。
- 调用时显式传入 `allowed_security_types=frozenset({"1"})`，只采集 BaoStock `type="1"` 的股票。
- 证券有效区间、必需字段校验、`ipoDate`/`outDate` 与请求窗口求交集的逻辑由该函数负责。
- 如果交集为空，不发起该 code 请求。
- `status` 可作为观测字段记录，但不作为唯一过滤依据；历史回填需要保留请求窗口内曾经有效但当前已退市的股票。

代码格式转换：

- BaoStock `sh.600000` 转为 EastMoney `600000.SH`。
- BaoStock `sz.000001` 转为 EastMoney `000001.SZ`。
- 不支持的市场前缀应跳过并计入 metadata。

请求范围：

```text
start_date = max(partition_year-01-01, ipoDate)
end_date = min(requested_end_date, outDate or requested_end_date)
```

日频调度时，与 K 线资产一致刷新当年分区：

```text
partition_key = scheduled_date.year
requested_start_date = partition_year-01-01
requested_end_date = scheduled_date
```

历史回填时，与 K 线资产一致显式 materialize 目标 `year` 分区：

```text
requested_start_date = partition_year-01-01
requested_end_date = partition_year-12-31
```

## 分区与存储

8 个资产全部使用 `year` 分区，与 K 线资产保持一致。

路径：

```text
source/eastmoney__balance/year=YYYY/000000_0.parquet
source/eastmoney__cashflow_sq/year=YYYY/000000_0.parquet
source/eastmoney__cashflow_ytd/year=YYYY/000000_0.parquet
source/eastmoney__dividend_allotment/year=YYYY/000000_0.parquet
source/eastmoney__dividend_main/year=YYYY/000000_0.parquet
source/eastmoney__equity_history/year=YYYY/000000_0.parquet
source/eastmoney__income_sq/year=YYYY/000000_0.parquet
source/eastmoney__income_ytd/year=YYYY/000000_0.parquet
```

输出格式：

- Parquet。
- 字段名来自对应 `docs/references/openapi/eastmoney__*.yaml` 的 `result.data.items.properties`。
- 宽表字段名不在资产代码中手写重复维护，优先从本地 OpenAPI YAML 生成或加载字段配置。
- 字段名保持东方财富原始大写字段名。
- 所有东方财富业务字段统一使用 `pa.string()` 写入 Parquet。
- 不按 OpenAPI 的 `number`、`integer` 或日期语义做类型转换；原始 JSON 值只做稳定字符串化，null 保持为 string 列中的 null。
- 每行补充采集元数据字段：
  - `request_code`
  - `request_start_date`
  - `request_end_date`
  - `partition_year`
  - `source_endpoint`
  - `ingested_at`

### 空表语义

部分资产天然稀疏，尤其是 `eastmoney__dividend_allotment`。某个 year 分区没有任何记录应被视为合法状态。

当前 `S3IOManager` 会拒绝写入空 `pyarrow.Table`。实现 RFC 0002 前需要补充以下能力之一：

1. 为 EastMoney asset 增加 `allow_empty=True` metadata，并扩展 IO manager 允许写入带 schema 的 0 行 Parquet。
2. 或者为稀疏资产设计显式空分区 marker，但这会让下游读取复杂化，不推荐。

推荐方案是扩展 IO manager 支持 `allow_empty=True`，并同步更新 ADR 0002。

## 并发、重试与超时

### Dagster run pool

新增 run pool：

```text
eastmoney_run_pool
```

所有 8 个 EastMoney asset 绑定：

```text
pool="eastmoney_run_pool"
```

本地和部署环境需要配置：

```text
dagster instance concurrency set eastmoney_run_pool 3
```

含义：

- 最多 3 个 EastMoney asset/job run 并行。
- 日频 job 内仍按固定顺序执行 8 个资产。
- 历史回填或手工 materialize 时，run pool 保护东方财富接口，避免过多 job 同时打满远端服务。

### aiohttp 并发

每个 asset run 内使用一个共享 `aiohttp.ClientSession`。

建议配置：

```text
ClientTimeout(total=60, sock_connect=5, sock_read=30)
TCPConnector(limit=20, limit_per_host=20)
asyncio.Semaphore(20)
```

设计约束：

- semaphore 是业务级 code 并发限制。
- connector limit 是底层连接池限制。
- 两者都设置为 20，避免 code 任务数量和连接数量出现不一致。
- 不为每个请求创建新的 `ClientSession`。
- 并发粒度是证券代码，不是分页。
- 单个 code 的分页必须顺序请求：先请求第 1 页获取 `pages`，再按页码顺序请求第 2 页到第 N 页。
- 不允许在同一个 code 内并发请求多个页面，避免远端分页状态、排序或缓存行为造成重复或漏页时难以定位。

### 重试

复用现有 `ExponentialBackoffPolicy` 和 `DEFAULT_RETRY_POLICY`。

默认：

```text
max_attempts=4
nominal_delays=1,2,4 seconds
```

重试范围：

- 连接错误。
- 请求超时。
- HTTP 429。
- HTTP 5xx。
- 响应 JSON 解析失败但响应体非空。
- 东方财富返回临时性错误码。

不重试范围：

- 参数构造错误。
- OpenAPI 字段名配置不匹配导致的本地转换错误。
- `data/v1/get` 的 `code=9201, result=null` 空结果。

## 分页与稳定排序

自动分页流程：

1. 请求第一页。
2. 读取 `result.pages`。
3. 如果 `pages` 缺失但第一页有数据，按 1 页处理并记录 warning metadata。
4. 对同一个 code 按页码顺序请求第 2 页到第 N 页。
5. 合并所有页面的 `result.data`。
6. 对同一 code + year 窗口做跨页重复检测。

稳定排序要求：

- 不允许只使用单字段 `REPORT_DATE` 或 `NOTICE_DATE` 排序。
- `data/get` 财务报表资产必须使用 `REPORT_DATE,SECURITY_CODE` 与 `-1,-1`。
- `data/v1/get` 事件类资产必须使用 `NOTICE_DATE,SECURITY_CODE` 与 `-1,-1`。
- 如果后续 OpenAPI 文档补充更稳定的唯一排序字段，应更新本 RFC 或新增 ADR。

重复检测：

- 每个返回行生成稳定 fingerprint。
- 同一 code + year + asset 的跨页重复 fingerprint 计数必须为 0。
- 一旦出现跨页重复，asset 应失败，并在错误中包含 asset、code、year、page。
- 不应静默去重后继续写入，因为重复通常表示分页排序不稳定，可能同时伴随漏数。

## Dagster 资产与调度

### 资产组织

建议新增：

```text
pipeline/scheduler/src/scheduler/defs/eastmoney/
  __init__.py
  assets.py
  client.py
  schemas.py
  schedules.py
```

资产属性：

- `group_name="eastmoney"`
- `io_manager_key="s3_io_manager"`
- `partitions_def=year_partitions`
- `metadata={"storage_mode": "partitioned", "partition_key_name": "year", "allow_empty": True}`
- `pool="eastmoney_run_pool"`
- tags:
  - `source=eastmoney`
  - `layer=source`
  - `storage=s3`

### 日频 job

新增一个日频 job：

```text
eastmoney__daily_job
```

资产顺序必须固定为：

1. `eastmoney__balance`
2. `eastmoney__cashflow_sq`
3. `eastmoney__cashflow_ytd`
4. `eastmoney__dividend_allotment`
5. `eastmoney__dividend_main`
6. `eastmoney__equity_history`
7. `eastmoney__income_sq`
8. `eastmoney__income_ytd`

顺序执行是调度控制需求，不代表这些资产之间存在业务数据依赖。实现时可通过专用 job/op 编排或显式 Dagster 依赖链实现，但必须在代码注释或 metadata 中说明这是 execution ordering dependency。

### 日频 schedule

新增自然日调度：

```text
eastmoney__daily_schedule
cron_schedule = "0 16 * * *"
execution_timezone = "Asia/Shanghai"
```

行为：

- 不读取 `sina__trade_calendar`。
- 不跳过非交易日。
- 每天 16:00 提交当年 year 分区。
- run config 传入 `refresh_until_date=<scheduled natural date>`。
- tags 包含：
  - `market.natural_date`
  - `market.year`
  - `source=eastmoney`

## Metadata

每个 asset materialization 应包含：

- `row_count`
- `column_count`
- `partition_keys`
- `candidate_security_count`
- `selected_security_count`
- `skipped_security_count`
- `request_count`
- `empty_response_count`
- `page_count`
- `retry_count`
- `duplicate_page_row_count`
- `selected_date_field`
- `sort_columns`
- `sort_types`
- `source_endpoint`
- `s3_bucket`
- `s3_keys`
- `asset_function_seconds`
- `eastmoney_remote_fetch_seconds`
- `table_convert_seconds`
- `code_concurrency_limit`

## 验收标准

设计实现完成后应满足：

- Dagster UI 中能看到 8 个 `eastmoney` group 下的 asset。
- 8 个 asset 都按 `year` 分区。
- 日频 schedule 每天 `Asia/Shanghai` 16:00 触发。
- 日频 job 内 8 个资产按 RFC 顺序执行。
- 非交易日也会按自然日运行。
- 每个 asset 读取 `baostock__query_stock_basic` 快照并只请求股票。
- 每个 HTTP 请求只包含一个 EastMoney code 和一个日期范围。
- 每个 asset run 内 `aiohttp` code 并发不超过 20。
- EastMoney run pool 最大 3 个 job 并行。
- 分页会自动请求所有页。
- 同一个 code 内分页按页码顺序请求，不做分页并发。
- 所有分页请求使用稳定多字段排序。
- 如果出现跨页重复，asset 失败而不是静默写入。
- 稀疏资产空 year 分区可以写入 0 行 Parquet。
- 输出字段名与对应 OpenAPI YAML 保持一致，东方财富业务字段类型全部为 string。
- S3 中存在 `source/<asset>/year=YYYY/000000_0.parquet`。

## 实施顺序

1. 更新 ADR 0002，明确 `allow_empty=True` 的空表写入语义。
2. 扩展 `S3IOManager` 支持 `allow_empty=True` 的 0 行 Parquet。
3. 新增 EastMoney endpoint 配置结构，覆盖 8 个资产的接口族、固定参数、日期字段和排序字段。
4. 新增 OpenAPI 字段名读取或生成工具，避免手写宽表字段；所有业务字段映射为 string。
5. 新增 EastMoney code 转换逻辑，并复用 `filter_active_security_ranges(..., allowed_security_types=frozenset({"1"}))` 生成股票有效请求范围。
6. 新增 `aiohttp` EastMoney client，支持 session 复用、code 级 semaphore、timeout、重试和顺序分页。
7. 新增 8 个 `year` 分区 asset。
8. 新增 `eastmoney__daily_job`，保证 8 个资产顺序执行。
9. 新增 `eastmoney__daily_schedule`，按自然日每天 16:00 运行。
10. 更新本地 Dagster concurrency 配置，加入 `eastmoney_run_pool=3`。
11. 使用小范围年份和少量股票做真实接口验证。
12. 使用完整 year 分区回填验证 S3 输出、metadata、code 并发、顺序分页和空表行为。

## 待确认

- `data/v1/get` 三个事件类接口是否都稳定支持 `sortColumns=NOTICE_DATE,SECURITY_CODE` 多字段排序；如果真实接口不接受，应回退为 OpenAPI 已验证的稳定排序字段，并记录到 ADR。
