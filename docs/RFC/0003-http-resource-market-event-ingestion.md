# RFC 0003: HTTP 资源类题材与涨停数据采集资产设计

状态：草案

## 摘要

本文定义 Mono Fleur 第三阶段 HTTP 资源类市场事件数据采集设计。

初始范围包含 3 个上游 endpoint：

1. `jiuyan__action_field`：韭研公社 APP 端异动板块与个股异动解析。
2. `ths__limit_up_pool`：同花顺涨停板池个股明细与当日涨跌停统计。
3. `jiuyan__industry_list`：韭研公社 APP 端产业研究文章列表。

设计目标是在复用现有 Dagster asset、S3 Parquet、S3 IO manager、交易日调度、退避重试和 `http_resources` 工程目录的基础上，补齐一组偏“HTTP 资源抓取”的 raw assets。

本 RFC 不新增独立顶层数据源目录。工程实现优先扩展：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/
```

## 目标

- 以 Dagster asset 管理 3 个 HTTP endpoint 的 raw 数据。
- 复用现有 `S3IOManager`、`write_parquet_dataset`、`allow_empty=True`、materialization metadata 模式。
- 复用 Sina 交易日历驱动交易日调度。
- 复用现有 `ExponentialBackoffPolicy` / `DEFAULT_RETRY_POLICY`。
- 在 `http_resources` 内沉淀通用 JSON HTTP 请求、浏览器 header、韭研认证 header、分页状态机和响应校验能力。
- 对按交易日查询的 endpoint 使用由已物化 `sina__trade_calendar` S3 Parquet 驱动的自定义交易日动态分区。
- 对长期累计且无日期参数的 `jiuyan__industry_list` 使用最新快照。
- 所有 raw 输出遵循“不新增字段”原则：Parquet 列只来自接口内容字段，不写入 `request_*`、`source_endpoint`、`ingested_at`、父级补充字段或展开索引等派生字段。
- 响应 envelope 顶层字段不进入 raw 文件列，例如 `errCode`、`msg`、`serverTime`、`status_code`、`status_msg`；这些只用于校验和 materialization metadata。
- raw 行粒度以接口内容为准：`data` 为数组时按数组元素写行；`data` 为分页对象时按分页内容对象写行。
- 接口内容里的嵌套字段必须完整保留并展平：struct/object 字段按原始字段路径展平成列；数组字段保留数组边界，数组元素内的 struct/object 也按原始字段路径展平；不只保留摘要、不裁剪子字段。
- 展平列名必须能可逆映射回原始字段路径；例如 `page.limit`、`info[].code`、`list[].article.action_info.time`。如果具体实现不能使用点号或 `[]`，可以采用等价转义命名，但不得引入非接口字段含义。
- 如果上游字段本身就是 JSON 字符串，raw 层保持原字符串，不解析成新增结构字段。

## 非目标

- 不实现 ClickHouse 建表或加载。
- 不实现 dbt 模型。
- 不实现网页查询接口。
- 不实现韭研产业研究图片 OCR；该能力由后续 RFC/ADR 设计。
- 不对题材、行业、涨停原因做归一化实体建模。
- 不对价格、涨跌幅、市值等数值做单位换算后覆盖原始值；换算留给下游模型。
- 不绕过韭研 SESSION 配额，也不设计多账号轮换。

## 参考资料

接口说明：

```text
docs/references/remote_endpoint/jiuyan__action_field.md
docs/references/remote_endpoint/ths__limit_up_pool.md
docs/references/remote_endpoint/jiuyan__industry_list.md
```

OpenAPI schema：

```text
docs/references/openapi/jiuyan__action_field.yaml
docs/references/openapi/ths__limit_up_pool.yaml
docs/references/openapi/jiuyan__industry_list.yaml
```

项目内设计约束：

```text
docs/RFC/0001-market-data-ingestion.md
docs/RFC/0002-eastmoney-f10-ingestion.md
docs/ADR/0001-market-data-raw-assets-on-dagster.md
docs/ADR/0002-s3-parquet-storage-layout.md
docs/ADR/0003-trade-calendar-driven-market-schedules.md
```

## 资产矩阵

| Asset | Endpoint | 分区 | 存储模式 | 调度 | 主要行粒度 |
| --- | --- | --- | --- | --- | --- |
| `jiuyan__action_field` | `POST /jystock-app/api/v1/action/field` | `trade_date=YYYY-MM-DD`，仅交易日动态分区 | partitioned | 交易日盘后 | 一个 `data[]` 板块内容行 |
| `ths__limit_up_pool` | `GET /dataapi/limit_up/limit_up_pool` | `trade_date=YYYY-MM-DD`，仅交易日动态分区 | partitioned | 交易日盘后 | 一个 `data` 分页内容行 |
| `jiuyan__industry_list` | `POST /jystock-app/api/v1/industry/list` | 无 | latest_snapshot | 每日或交易日盘后 | 一个 `data` 分页内容行 |

### 分区决策

`jiuyan__action_field` 和 `ths__limit_up_pool` 都有明确的日期请求参数，且语义是“某交易日市场事件”，因此使用 `trade_date` 分区。但这些资产不使用普通自然日 `DailyPartitionsDefinition`，而是使用由已物化 `sina__trade_calendar` S3 Parquet 读取出的 A 股交易日集合生成/同步的自定义动态分区。

```text
raw/jiuyan__action_field/trade_date=YYYY-MM-DD/000000_0.parquet
raw/ths__limit_up_pool/trade_date=YYYY-MM-DD/000000_0.parquet
```

分区 key 格式固定为 `YYYY-MM-DD`，partition key name 固定为 `trade_date`。合法分区 key 必须存在于 `sina__trade_calendar` 的 S3 Parquet 结果中；自然日但非交易日不应被加入动态分区，也不应由默认调度 materialize。

`jiuyan__industry_list` 是长期累计列表，没有日期请求参数；响应会随服务端新增、置顶、更新而变化。第一版按最新快照保存：

```text
raw/jiuyan__industry_list/000000_0.parquet
```

如果后续需要研究文章历史变更审计，再新增按 `snapshot_date` 分区的资产或新增 ADR，不在第一版中扩大范围。

## 工程组织

继续复用并扩展 `http_resources` 目录：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/
  __init__.py
  clients.py
  jiuyan.py
  ths.py
  schedules.py
  sina__trade_calendar.py
```

建议职责：

- `clients.py`：通用 JSON HTTP 请求、浏览器 headers、重试、超时、响应 JSON 解析、请求统计。
- `jiuyan.py`：韭研认证 header、`action_field` 和 `industry_list` 请求/响应转换、对应 Dagster assets。
- `ths.py`：同花顺浏览器 header、`limit_up_pool` 分页请求/响应转换、对应 Dagster asset。
- `schedules.py`：继续承载 Sina 交易日历 job/schedule，并新增 HTTP resource 类 assets 的 job/schedule。

不建议为 `jiuyan` 和 `ths` 另建顶层 `defs/jiuyan` 或 `defs/ths` 目录。它们都属于轻量 HTTP 资源抓取，放在 `http_resources` 内可复用已有 Sina 资源模式，也避免在 defs 层过早拆分数据源目录。

## 共享 HTTP 资源设计

### 通用 JSON Client

新增轻量同步 JSON HTTP client 即可满足第一版需求；这三个 endpoint 请求量很小，不需要引入 `aiohttp` 并发模型。

建议接口形态：

```python
class JsonHttpClient:
    def get_json(...)
    def post_json(...)
```

复用能力：

- `requests.Session` 复用 TCP 连接。
- `REQUEST_TIMEOUT_SECONDS = (5, 20)`，与现有 Sina 资源保持一致。
- `DEFAULT_RETRY_POLICY`，默认最多 4 次尝试，等待 1、2、4 秒。
- 重试范围：
  - `requests.Timeout`
  - `requests.ConnectionError`
  - HTTP 429
  - HTTP 5xx
  - 响应体 JSON 解析失败
- 不重试范围：
  - 本地参数构造错误
  - 鉴权缺失
  - 明确的业务参数错误，例如同花顺 380 天外日期返回 `status_code=-1`

### Header 复用

浏览器类 header 统一由 helper 生成：

```text
User-Agent: Chrome UA
Accept: application/json,text/plain,*/*
```

同花顺请求必须额外携带：

```text
Referer: https://data.10jqka.com.cn/
```

韭研请求必须额外携带：

```text
token: <JIUYAN_TOKEN>
cookie: <JIUYAN_COOKIE>
platform: 3
timestamp: <current unix milliseconds>
Content-Type: application/json
```

`timestamp` 必须在每次请求发送前动态生成，不能在 asset run 开始时生成一次后复用到所有请求。

新增环境变量：

```text
JIUYAN_TOKEN
JIUYAN_COOKIE
```

配置建议放入 `pipeline/scheduler/src/scheduler/defs/config.py`：

```python
JIUYAN_TOKEN = dg.EnvVar("JIUYAN_TOKEN")
JIUYAN_COOKIE = dg.EnvVar("JIUYAN_COOKIE")
```

## Asset 设计

### `jiuyan__action_field`

请求：

```text
POST https://app.jiuyangongshe.com/jystock-app/api/v1/action/field
```

请求体：

```json
{
  "pc": "1",
  "date": "<trade_date>"
}
```

使用 `pc="1"`，因为第一版 raw asset 需要保留板块内股票明细。`pc="2"` 只返回摘要和聚合行，不作为初始采集模式。

响应校验：

- `errCode == "0"` 才视为成功。
- 缺少任一必填韭研 header 时可能返回 `errCode == "9"`，应失败并提示鉴权或版本 header 配置问题。
- `data` 必须是 array；空 array 合法。

输出行粒度：

- 一行对应响应 `data[]` 中的一个板块内容对象。
- 响应 envelope 字段 `errCode`、`msg`、`serverTime` 不进入 raw 文件列；只用于响应校验和 materialization metadata。
- `"简图"`、`"全部"`、`"一字板"` 等聚合或特殊板块对象也保留为内容行。
- `data[].list` 及其内部 `article`、`action_info`、`user` 等嵌套对象/数组必须按原始字段路径完整展平，不保存为 JSON 字符串，也不新增请求/采集派生字段。

内容字段：

```text
action_field_id
name
date
reason
status
sort_no
is_delete
delete_time
create_time
update_time
count
list[].code
list[].name
list[].article.article_id
list[].article.title
list[].article.create_time
list[].article.comment_count
list[].article.like_count
list[].article.forward_count
list[].article.step_count
list[].article.is_like
list[].article.is_step
list[].article.user_id
list[].article.action_info.*
list[].article.user.*
```

字段类型：

- 标量字段第一版全部写为 `pa.string()`。
- `list` 保留数组边界；数组元素内的对象字段按原始路径展平，字段集合不得裁剪。
- 缺失字段写 null。

配额与回填：

- 韭研 `action_field` 每 SESSION 每日 80 个交易日请求配额。
- 日频调度每天只请求 1 个 trade_date。
- 历史回填应限制为小批量，建议每次不超过 20 个 trade_date；不要默认一次性全历史回填。
- 如果后续需要大规模回填，应新增运行配置或操作手册，明确配额风险。

### `ths__limit_up_pool`

请求：

```text
GET https://data.10jqka.com.cn/dataapi/limit_up/limit_up_pool
```

固定参数：

```text
field=199112,10,9001,330323,330324,330325,9002,330329,133971,133970,1968584,3475914,9003,9004
filter=HS,GEM2STAR
order_field=330324
order_type=0
limit=500
date=<YYYYMMDD>
_=<current unix milliseconds>
```

分页：

- 第一页 `page=1`。
- `data.page.count` 在参考文档中实测为总页数，不是当前页记录数。
- 后续页按 `page=2..count` 顺序请求。
- 当前页实际记录数使用 `len(data.info)`。
- 对同一 trade_date 跨页做 row fingerprint 重复检测；重复时 asset 失败。

响应校验：

- HTTP 403 通常表示缺少浏览器 header，应失败并提示 header 配置。
- `status_code == 0` 视为成功。
- `status_code == -1` 且 `status_msg == "date参数不合法"` 表示超出 380 天保留期，应失败，不写空表。
- 非交易日或未来日期可能返回 `status_code == 0` 且 `total == 0`、`info == []`；这在手工 materialize 时可以写 0 行，但默认交易日调度不会请求非交易日。

输出行粒度：

- 一行对应一个分页内容对象 `data`。
- 响应 envelope 字段 `status_code`、`status_msg` 不进入 raw 文件列；只用于响应校验和 materialization metadata。
- `data.page`、`data.info`、`data.limit_up_count`、`data.limit_down_count`、`data.date`、`data.msg`、`data.trade_status` 等内容字段都必须完整保留并展平。
- `data.info[]` 中的个股对象及其 `time_preview` 等嵌套数组必须按原始字段路径完整展平，不保存为 JSON 字符串，也不新增请求/采集派生字段。
- 如果当日 `info` 为空，仍写入该分页内容行，保留 `info=[]` 以及统计字段。

内容字段：

```text
page.limit
page.total
page.count
page.page
info[].open_num
info[].first_limit_up_time
info[].last_limit_up_time
info[].code
info[].limit_up_type
info[].order_volume
info[].is_new
info[].limit_up_suc_rate
info[].currency_value
info[].market_id
info[].is_again_limit
info[].change_rate
info[].turnover_rate
info[].reason_type
info[].order_amount
info[].high_days
info[].name
info[].high_days_value
info[].change_tag
info[].market_type
info[].latest
info[].time_preview[]
limit_up_count.*
limit_down_count.*
date
msg
trade_status.*
```

字段类型：

- 标量字段第一版全部写为 `pa.string()`。
- `info` 和 `time_preview` 保留数组边界；数组元素内的对象字段按原始路径展平，字段集合不得裁剪。
- 保留原始单位，不做价格或百分比换算。
- 不为请求参数、采集时间或 endpoint 新增 raw 列。

保留期：

- 服务端最大保留约 380 天。
- 交易日动态分区 backfill 起点应动态限制为 `today - 380 days`，或在 materialize 超出范围时明确失败。
- 不要将超出保留期返回的参数错误解释为空结果。

### `jiuyan__industry_list`

请求：

```text
POST https://app.jiuyangongshe.com/jystock-app/api/v1/industry/list
```

请求体：

```json
{
  "keyword": "",
  "start": "0",
  "limit": "500"
}
```

第一版只采集全量列表，`keyword=""`。关键词搜索是查询能力，不作为 raw asset 分区维度。

分页状态机：

1. 第一次请求固定 `start="0"`。
2. 读取响应 `data.result`。
3. 如果 `data.hasNext == true`，下一次请求 `start=str(data.nextPage)`。
4. 如果 `data.hasNext == false`，停止。

禁止：

- 不使用本地页码自增推导下一页。
- 不依赖 `totalCount` 或 `totalPages` 判断是否结束。
- 不把 `start=0` 和 `start=1` 都请求一遍。

响应校验：

- `errCode == "0"` 才视为成功。
- `data` 必须是 object。
- `data.result` 必须是 array。

输出行粒度：

- 一行对应一个分页内容对象 `data`。
- 响应 envelope 字段 `errCode`、`msg`、`serverTime` 不进入 raw 文件列；只用于响应校验和 materialization metadata。
- `data` 内部分页字段和 `result` 数组都必须完整保留并展平。
- `data.result[].imgs` 如果上游返回为 JSON 字符串，则 raw 层原样保存该字符串；不解析为新增 list 字段，也不拆成图片行。

内容字段：

```text
pageNo
pageSize
totalCount
totalPages
hasNext
nextPage
hasPre
prePage
first
orderBy
order
autoCount
map
params
result[].industry_id
result[].title_red
result[].title_bold
result[].title
result[].author
result[].imgs
result[].keyword
result[].content
result[].is_top
result[].status
result[].sort_no
result[].forward_count
result[].browsers_count
result[].is_delete
result[].delete_time
result[].create_time
result[].update_time
```

字段类型：

- 标量字段第一版全部写为 `pa.string()`。
- `result` 保留数组边界；数组元素内的对象字段按原始路径展平，字段集合不得裁剪。
- 缺失字段写 null。

存储：

- `metadata={"storage_mode": "latest_snapshot"}`。
- 默认拒绝空表；产业研究全量列表为空应视为异常。

## 调度设计

### 交易日动态分区 assets

新增共享交易日动态分区：

```text
trade_date_dynamic_partitions
source_asset="sina__trade_calendar"
source_storage="s3 parquet"
partition_key_name="trade_date"
fmt="%Y-%m-%d"
```

`jiuyan__action_field` 和 `ths__limit_up_pool` 使用同一个 `trade_date_dynamic_partitions`。

分区同步行为：

- schedule 评估时读取 S3 中已物化的 `sina__trade_calendar` Parquet，不请求新浪远端接口。
- 将日历中尚未注册的交易日加入 Dagster dynamic partitions。
- 如果交易日历不存在或不可读，schedule 返回 `SkipReason`，提示先 materialize `sina__trade_calendar`。
- 如果评估日期不是交易日，schedule 返回 `SkipReason`，且不创建自然日分区。
- 历史回填只能选择已经存在于动态分区集合中的交易日；批量补分区时同样以 `sina__trade_calendar` Parquet 为事实来源。

新增 job：

```text
http_resources__market_event_daily_job
```

包含资产：

1. `jiuyan__action_field`
2. `ths__limit_up_pool`

执行顺序不是业务血缘要求。第一版可不强制顺序；如需控制远端压力，可通过 job selection 或显式 execution-ordering dependency 实现，并在 metadata 或注释中说明。

新增 schedule：

```text
http_resources__market_event_daily_schedule
cron_schedule = "45 16 * * *"
execution_timezone = "Asia/Shanghai"
```

行为：

- 复用 `build_trade_date_schedule`。
- 读取 `sina__trade_calendar` 的 S3 Parquet 结果作为交易日事实来源。
- 同步/确认当日交易日动态分区。
- 非交易日跳过且不注册分区。
- partition key 为当日 `YYYY-MM-DD`。
- tags 包含：
  - `market.trade_date`
  - `source=http_resources`

### 产业研究快照 asset

新增 job：

```text
jiuyan__industry_list_snapshot_job
```

新增 schedule：

```text
jiuyan__industry_list_snapshot_schedule
cron_schedule = "30 17 * * *"
execution_timezone = "Asia/Shanghai"
```

建议每天自然日刷新一次。产业研究不是严格交易日资源，节假日可能仍有更新；因此第一版不使用交易日跳过逻辑。

如果后续希望减少请求，也可以改为复用交易日调度，但应记录为行为变更。

## 存储与空表语义

`jiuyan__action_field`：

```text
metadata={
  "storage_mode": "partitioned",
  "partition_key_name": "trade_date",
  "partitions_def": "trade_date_dynamic_partitions",
  "allow_empty": True,
}
```

`ths__limit_up_pool`：

```text
metadata={
  "storage_mode": "partitioned",
  "partition_key_name": "trade_date",
  "partitions_def": "trade_date_dynamic_partitions",
  "allow_empty": True,
}
```

`jiuyan__industry_list`：

```text
metadata={"storage_mode": "latest_snapshot"}
```

空表策略：

- `jiuyan__action_field` 某交易日没有异动板块时可以写 0 行，但 schema 仍应完整。
- `ths__limit_up_pool` 手工请求非交易日或未来日期时，仍写 1 行 `data` 分页内容行，`info=[]`，并完整保留 `page`、涨跌停统计和 `trade_status` 等内容字段。
- `jiuyan__industry_list` 每个成功分页响应写 1 行 `data` 分页内容行；如果全量翻页后所有 `result=[]`，应视为异常并失败。

## Metadata

本节 metadata 是 Dagster materialization metadata，不是 raw Parquet 字段。请求参数、endpoint、采集时间、调度分区和响应 envelope 状态等运行信息只能写入 metadata 或路径，不能写入 raw 文件列。接口内容字段即使也复制到 metadata 便于观测，也必须完整保留并展平到 raw Parquet 中，metadata 不能替代 raw 字段。

所有 asset 至少包含：

- `row_count`
- `column_count`
- `source_endpoint`
- `request_count`
- `retry_count`
- `empty_response_count`
- `source_status_code` 或 `source_err_code`
- `asset_function_seconds`
- `http_fetch_seconds`
- `table_convert_seconds`
- `s3_bucket`
- `s3_keys`

分区 assets 额外包含：

- `partition_keys`
- `request_trade_date`
- `partition_key_name`
- `partitions_source_asset`
- `dynamic_partition_added`

分页 assets 额外包含：

- `page_count`
- `duplicate_page_row_count`

`jiuyan__industry_list` 额外包含：

- `result_page_count`
- `industry_total_rows`
- `has_next_terminal_value`

`ths__limit_up_pool` 额外包含：

- `page_total`
- `source_response_date`
- `trade_status`
- `limit_up_count`
- `limit_down_count`

## 数据质量与校验

单元测试要求：

- 韭研 header 构造包含 `token`、`cookie`、`platform` 和动态 `timestamp`。
- 缺失 `JIUYAN_TOKEN` 或 `JIUYAN_COOKIE` 时 asset 失败。
- `jiuyan__action_field` 不包含 `list_json` 字段。
- `jiuyan__action_field` 不包含响应 envelope 字段 `errCode`、`msg`、`serverTime` 作为 raw 列。
- `jiuyan__action_field` 以 `data[]` 内容对象为行，并完整展平 `list[].article.action_info`、`list[].article.user` 等原始嵌套字段。
- `jiuyan__action_field` 不新增 `request_*`、`source_endpoint`、`ingested_at` 等 raw 列。
- `jiuyan__industry_list` 使用 `hasNext` / `nextPage` 翻页，不依赖 `totalCount` / `totalPages`。
- `jiuyan__industry_list` 不请求 `start=1` 作为第一页重复页。
- `jiuyan__industry_list` 不包含响应 envelope 字段 `errCode`、`msg`、`serverTime` 作为 raw 列。
- `jiuyan__industry_list` 以 `data` 分页内容对象为行，并完整展平 `result[]` 文章字段。
- `jiuyan__industry_list` 保留接口原始 `result[].imgs` 字段；如果上游返回 JSON 字符串，则 raw 中仍为原字符串，不解析为新增字段。
- `jiuyan__industry_list` 不新增 `request_*`、`source_endpoint`、`ingested_at` 等 raw 列。
- `ths__limit_up_pool` 携带 `User-Agent` 和 `Referer`。
- `ths__limit_up_pool` 使用 `data.page.count` 作为总页数，使用 `len(data.info)` 作为当前页行数。
- `ths__limit_up_pool` 遇到跨页重复 fingerprint 时失败。
- `ths__limit_up_pool` 遇到 380 天外 `date参数不合法` 时失败。
- `ths__limit_up_pool` 不包含响应 envelope 字段 `status_code`、`status_msg` 作为 raw 列。
- `ths__limit_up_pool` 以 `data` 分页内容对象为行，并完整展平 `page`、`info[].time_preview`、`limit_up_count`、`limit_down_count`、`trade_status` 等原始内容字段。
- `ths__limit_up_pool` 不新增请求参数、endpoint 或采集时间 raw 列。
- 交易日分区集合由 `sina__trade_calendar` S3 Parquet 驱动，非交易日不会被注册为动态分区。
- `data=[]` 可写 0 行但必须保留 schema；`data.info=[]` 的成功响应仍写出 `data` 分页内容行，并保留 schema 完整 Parquet。

真实接口验证：

- `jiuyan__action_field` 使用一个最近交易日，验证 `pc="1"` 返回 `data[]` 内容行，且 `list[]` 及其嵌套对象完整展平。
- `ths__limit_up_pool` 使用一个最近交易日，验证浏览器 header、分页和字段完整性。
- `jiuyan__industry_list` 使用 `limit=500` 全量翻页，验证只依赖 `hasNext`，且 `result[].imgs` 按上游原始类型保留。

## 验收标准

设计实现完成后应满足：

- Dagster UI 中能看到 3 个 `http_resources` group 下的新增 assets。
- `jiuyan__action_field` 和 `ths__limit_up_pool` 都按 `trade_date` 交易日动态分区。
- `jiuyan__industry_list` 为 latest snapshot。
- 两个分区资产路径分别为：
  - `raw/jiuyan__action_field/trade_date=YYYY-MM-DD/000000_0.parquet`
  - `raw/ths__limit_up_pool/trade_date=YYYY-MM-DD/000000_0.parquet`
- `jiuyan__industry_list` 路径为 `raw/jiuyan__industry_list/000000_0.parquet`。
- 交易日动态分区和调度都以 `sina__trade_calendar` S3 Parquet 为事实来源，非交易日跳过且不注册分区。
- 韭研请求每次动态生成 timestamp。
- 韭研凭证只来自环境变量，不写入代码、metadata 或日志。
- 同花顺请求携带浏览器 header 和 Referer。
- `jiuyan__industry_list` 翻页严格以 `hasNext` / `nextPage` 为准。
- `ths__limit_up_pool` 自动请求全部页面，跨页重复时失败。
- raw Parquet 不包含响应 envelope 字段，只包含接口内容字段。
- 接口内容中的对象字段按原始字段路径展平成列；数组字段保留数组边界，数组元素内对象也按原始字段路径完整展平，不裁剪子字段。
- 上游原本就是 JSON 字符串的字段按字符串原样保留，不解析为新增字段。
- 共享 HTTP 请求、header、重试、超时逻辑在 `http_resources` 内复用，不为每个 endpoint 重复实现一套客户端。

## 实施顺序

1. 在 `config.py` 增加 `JIUYAN_TOKEN` 和 `JIUYAN_COOKIE`。
2. 在 `http_resources/clients.py` 增加共享 JSON HTTP client、浏览器 header、韭研 header helper。
3. 在 `http_resources/jiuyan.py` 实现 `jiuyan__action_field` 和 `jiuyan__industry_list`。
4. 在 `http_resources/ths.py` 实现 `ths__limit_up_pool`。
5. 在 `http_resources/schedules.py` 增加 `sina__trade_calendar` S3 Parquet 驱动的交易日动态分区同步、market event job/schedule 和产业研究 snapshot job/schedule。
6. 更新 `pipeline_defs.py` 注册新增 assets、jobs、schedules。
7. 增加单元测试覆盖 header、分页、空表、错误码和字段转换。
8. 使用 `uv run dg check defs --target-path scheduler` 验证 definitions。
9. 使用最近一个交易日做真实接口小范围验证。

## 待确认

- `jiuyan__action_field` 的历史数据保留期未在参考文档中明确。大规模回填前需要用少量历史日期验证。
- `ths__limit_up_pool` 380 天保留期会随当前日期滚动，回填起点需要运行时动态计算。
- `jiuyan__industry_list` 是否需要历史快照审计。如果需要，应新增 `snapshot_date` 分区资产，而不是改变当前 latest snapshot 语义。
- 是否需要在后续 staging/dbt 层展开 `jiuyan__action_field.list` 或 `jiuyan__industry_list.imgs`。raw 层第一版不新增字段、不拆子行。
