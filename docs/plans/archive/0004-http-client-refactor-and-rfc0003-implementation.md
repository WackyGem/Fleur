# Plan 0004: HTTP 客户端复用性重构与 RFC 0003 实施

状态：草案

关联 RFC：

- `docs/RFC/archive/0003-http-resource-market-event-ingestion.md`
- `docs/RFC/archive/0002-eastmoney-f10-ingestion.md`

参考资料：

- `docs/plans/0003-eastmoney-f10-ingestion.md`
- `docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0003-trade-calendar-driven-market-schedules.md`
- `pipeline/scheduler/src/scheduler/defs/http_resources/client.py`
- `pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/client.py`
- `pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/assets.py`
- `pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/schemas.py`
- `pipeline/scheduler/src/scheduler/defs/http_resources/sina__trade_calendar.py`
- `pipeline/scheduler/src/scheduler/defs/http_resources/schedules.py`
- aiohttp 当前文档：`ClientSession` 复用、`ClientTimeout`、`TCPConnector(limit, limit_per_host)`、`headers`、`params`、`json` 请求参数、响应读取。

## 目标

本计划先做 HTTP 客户端的一刀切复用性重构，再在重构后的基础上实施 RFC 0003。

第一阶段目标：

- 抽出一个可复用 `aiohttp` 客户端，统一 HTTP session、timeout、connector、headers、重试、响应读取和请求统计。
- 一刀切迁移现有 HTTP 代码：
  - `eastmoney` 包合并到 `http_resources/eastmoney/` 内，继续保持现有行为，但不再直接持有 `aiohttp.ClientSession` 和 `_request_json` 重试循环。
  - `http_resources/sina__trade_calendar.py` 从同步 `requests` 迁移到共享 `aiohttp` 客户端。
- 保留现有 EastMoney 分页、code 级并发、重复行检测、schema 和 S3 输出行为。
- 迁移完成后，项目内不再新增第二套 HTTP client 模式，也不保留顶层 `defs/eastmoney` 或 `defs/http_client` 包。

第二阶段目标：

- 按 RFC 0003 实施：
  - `jiuyan__action_field`
  - `ths__limit_up_pool`
  - `jiuyan__industry_list`
- 使用同一个共享 `aiohttp` 客户端。
- `jiuyan__action_field` 和 `ths__limit_up_pool` 使用由 `sina__trade_calendar` S3 Parquet 驱动的交易日动态分区。
- `jiuyan__action_field` 和 `ths__limit_up_pool` 必须支持 single-run 回填：一次 run 传入交易日范围，在 run 内并行查询多个交易日，并且每完成一个交易日就立即写入对应 S3 分区。
- raw 文件遵循 RFC 0003 当前约束：不写响应 envelope 字段，不新增请求/采集派生字段，内容字段完整保留并展平。

## 非目标

本计划不包含：

- ClickHouse 建表或加载。
- dbt 模型。
- Web 查询接口。
- 对 BaoStock TCP client 的重构。
- 修改 EastMoney 现有业务 schema 或输出路径语义。
- 为每个新 endpoint 引入专用 HTTP client。
- 保留顶层 `scheduler/defs/eastmoney` 包。
- 新增顶层 `scheduler/defs/http_client` 包。
- 在 source 层新增 `request_*`、`source_endpoint`、`ingested_at` 等派生列。
- 将 RFC 0003 的内容数组拆成派生子 asset；内容字段展平在当前 raw asset 内完成。

## 当前代码现状

现有 HTTP 相关代码分裂为两套：

1. EastMoney：
   - 当前代码仍在 `pipeline/scheduler/src/scheduler/defs/eastmoney/`。
   - 计划目标路径是 `pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/`。
   - 已使用 `aiohttp.ClientSession`、`ClientTimeout`、`TCPConnector`。
   - 内部实现了 request retry、JSON 解析、HTTP 状态分类、code 级 semaphore 和 fetch stats。
   - `EastmoneyAioHttpClient` 同时承担通用 HTTP client 和 EastMoney 分页业务 client 两类职责。

2. Sina trade calendar：
   - `pipeline/scheduler/src/scheduler/defs/http_resources/sina__trade_calendar.py`
   - 仍使用同步 `requests.get`、`time.sleep` 和手写 retry loop。
   - asset 函数是同步 Dagster asset，远端请求和解析耦合在同一模块内。

现有 EastMoney 行为必须在重构后保持：

- `aiohttp.ClientSession` 在一个 async context 内复用。
- timeout 等价于：
  - `total=60`
  - `sock_connect=5`
  - `sock_read=30`
- connector 等价于：
  - `limit=20`
  - `limit_per_host=20`
- HTTP 429 和 5xx 作为可重试 transient error。
- HTTP 4xx，除 429 外，作为不可重试错误。
- `aiohttp.ClientError`、`asyncio.TimeoutError`、JSON decode error 可重试。
- `DEFAULT_RETRY_POLICY` 最多 4 次请求，等待 1、2、4 秒。
- EastMoney 单个 code 内分页顺序请求，不做分页并发。
- EastMoney code 级并发上限保持 20。

## 总体实施顺序

本计划必须按顺序实施，不能先做 RFC 0003 新 endpoint。

1. 先将顶层 `defs/eastmoney` 包移动到 `defs/http_resources/eastmoney`，并修正 import 与 definition 注册。
2. 在 `http_resources` 包内新增共享 `aiohttp` client 基础设施，不新增顶层 `defs/http_client`。
3. 将 EastMoney 客户端改为组合共享 client。
4. 将 Sina trade calendar 从 `requests` 迁移到共享 client。
5. 删除或隔离旧 HTTP 请求路径，保证新代码不再使用第二套 HTTP 逻辑。
6. 跑现有 EastMoney 和 Sina 单元测试，补齐共享 client 测试。
7. 在共享 client 基础上实现 RFC 0003 的 schema/flatten/assets/schedules。
8. 注册新增 definitions。
9. 使用 `uv run dg check defs --target-path scheduler` 验证 Dagster definitions。
10. 使用小范围真实接口做人工验证。

## 第一阶段：共享 aiohttp 客户端

### 目标模块结构

本计划将 HTTP 资源类代码统一收敛到 `http_resources` 包内。目标目录结构：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/
  __init__.py
  client.py
  eastmoney/
  flatten.py
  jiuyan__action_field.py
  jiuyan__industry_list.py
  schemas.py
  ths__limit_up_pool.py
  schedules.py
  sina__trade_calendar.py
```

职责：

- `client.py`：共享 `AioHttpClient`、请求/响应模型、错误分类、重试、浏览器 header helper 和通用请求统计。
- `eastmoney/`：迁移后的 EastMoney assets、domain client、schemas、fields、schedules；保留 EastMoney 内部模块拆分，不再作为顶层 `defs/eastmoney` 包存在。
- `flatten.py`：RFC 0003 内容字段展平工具。
- `jiuyan__action_field.py`：韭研异动板块 asset、请求/响应校验和业务分页以外逻辑。
- `jiuyan__industry_list.py`：韭研产业研究列表 asset、请求/响应校验和 `hasNext` / `nextPage` 翻页。
- `schemas.py`：RFC 0003 三个 asset 的 schema、flatten column naming 和 content-to-table 转换。
- `ths__limit_up_pool.py`：同花顺涨停池 asset、分页请求、响应校验和跨页重复检测。
- `schedules.py`：保留 Sina trade calendar job/schedule，并新增 RFC 0003 jobs/schedules；EastMoney schedule 可继续放在 `eastmoney/schedules.py`，但由 `pipeline_defs.py` 从新路径注册。
- `sina__trade_calendar.py`：Sina 交易日历 asset。

共享 `aiohttp` client 明确放在 `scheduler/defs/http_resources/client.py`。不能新建顶层 `defs/http_client`，也不能让 `jiuyan__*`、`ths__*` 或 `sina__trade_calendar` 直接依赖 `http_resources/eastmoney`。

### 通用配置

建议常量：

```text
HTTP_TOTAL_TIMEOUT_SECONDS = 60
HTTP_CONNECT_TIMEOUT_SECONDS = 5
HTTP_READ_TIMEOUT_SECONDS = 30
HTTP_MAX_ATTEMPTS = 4
HTTP_CONNECTOR_LIMIT = 20
HTTP_CONNECTOR_LIMIT_PER_HOST = 20
CHROME_USER_AGENT = ...
```

第一版使用模块常量，保持与现有 EastMoney 行为一致。后续如需运行时调整，再集中迁移到 `config.py` 或 Dagster config。

### Client API

建议接口形态：

```python
@dataclass(frozen=True)
class HttpRequest:
    method: Literal["GET", "POST"]
    url: str
    params: Mapping[str, str] | None
    headers: Mapping[str, str] | None
    json_body: object | None


@dataclass(frozen=True)
class HttpTextResponse:
    status: int
    headers: Mapping[str, str]
    body: str


class AioHttpClient:
    async def __aenter__(self) -> AioHttpClient: ...
    async def __aexit__(...) -> None: ...
    async def request_text(self, request: HttpRequest) -> HttpTextResponse: ...
    async def request_json_object(self, request: HttpRequest) -> Mapping[str, object]: ...
```

设计要求：

- `AioHttpClient` 是 async context manager。
- 内部复用一个 `aiohttp.ClientSession`。
- session 使用 `aiohttp.ClientTimeout(...)`。
- session 使用 `aiohttp.TCPConnector(limit=..., limit_per_host=...)`。
- 支持 GET/POST。
- 支持 `params`、`headers`、`json_body`。
- `request_json_object` 使用 `await response.text()` + `json.loads()`，不依赖远端 Content-Type 是否正确。
- JSON 顶层不是 object 时失败。
- 不在通用 client 内解析业务状态码，例如 `errCode`、`status_code == -1`，这些由数据源 client 判断。

### 错误分类

新增通用错误：

```text
HttpRequestError
HttpTransientRequestError
HttpResponseDecodeError
```

重试范围：

- `aiohttp.ClientError`
- `asyncio.TimeoutError`
- JSON decode error
- HTTP 429
- HTTP 5xx

不重试范围：

- HTTP 4xx，除 429 外。
- 本地参数构造错误。
- 业务错误，例如：
  - EastMoney payload shape 不符合预期。
  - 韭研 `errCode != "0"`。
  - 同花顺 `status_code == -1` 且 `status_msg == "date参数不合法"`。

### 通用统计

`HttpFetchStats` 至少包含：

```text
request_count
retry_count
transient_error_count
http_4xx_count
http_5xx_count
decode_error_count
```

数据源 client 可以继续维护自己的业务统计：

- EastMoney：
  - `empty_response_count`
  - `page_count`
  - `duplicate_page_row_count`
- RFC 0003:
  - `empty_response_count`
  - `page_count`
  - `duplicate_page_row_count`
  - `result_page_count`

共享 client 的 stats 可以通过组合暴露给 domain client，再写入 materialization metadata。

### Header 设计

`client.py` 中的共享 header helper 提供：

```text
browser_json_headers()
browser_text_headers()
with_referer(headers, referer)
```

动态 header 使用 callable：

```python
HeaderFactory = Callable[[], Mapping[str, str]]
```

韭研 header 必须通过 factory 每次请求前生成，确保 `timestamp` 是请求发送时的当前毫秒时间戳，而不是 asset run 开始时的固定值。

## 第二阶段：一刀切迁移现有 HTTP 代码

### EastMoney 迁移

迁移原则：

- 先做包移动：`scheduler.defs.eastmoney` -> `scheduler.defs.http_resources.eastmoney`。
- 移动后删除顶层 `pipeline/scheduler/src/scheduler/defs/eastmoney/` 包。
- 所有内部 import、`pipeline_defs.py` 注册、测试 import 都改为新路径。
- 保留 EastMoney domain client，但移除其直接 `aiohttp.ClientSession` 管理和 `_request_json` retry loop。
- `EastmoneyAioHttpClient` 改为组合 `AioHttpClient`。
- `fetch_code_range`、分页、`parse_eastmoney_page`、重复 fingerprint 检测保持不变。
- `EastmoneyFetchStats` 保留业务统计，并合并共享 client stats。

迁移前：

```text
EastmoneyAioHttpClient
  owns aiohttp.ClientSession
  owns timeout/connector
  owns retry loop
  owns request_count/retry_count
  owns EastMoney pagination
```

迁移后：

```text
EastmoneyAioHttpClient
  owns EastMoney pagination
  owns code semaphore
  owns EastMoney business stats
  composes AioHttpClient for HTTP transport
```

保留现有行为：

- `EASTMONEY_CODE_CONCURRENCY = 20`。
- code 级 semaphore 仍在 EastMoney domain client 内。
- 单 code 内分页顺序请求。
- `build_request_params(...)` 不迁移到通用 client。
- `parse_eastmoney_page(...)` 不迁移到通用 client。
- EastMoney 业务错误继续抛 `EastmoneyRequestError`。
- EastMoney asset name、group、tags、metadata key、S3 输出路径语义不因包移动改变。

测试迁移：

- 更新 `pipeline/scheduler/tests/test_eastmoney.py`，确保：
  - import 路径改为 `scheduler.defs.http_resources.eastmoney...`。
  - request params 构造不变。
  - `parse_eastmoney_page` 行为不变。
  - HTTP 429/5xx 仍重试。
  - HTTP 4xx 仍失败且不重试。
  - JSON decode error 仍重试。
  - duplicate page row 仍失败。

### Sina trade calendar 迁移

迁移原则：

- `fetch_sina_trade_calendar` 改为 async，并使用共享 `AioHttpClient.request_text(...)`。
- `sina__trade_calendar` asset 内使用 `asyncio.run(...)` 调用 async fetch，保持 asset 对 Dagster 的同步函数形态。
- `SinaCalendarParser`、解码逻辑、`trade_calendar_dates_to_table` 不改。
- `REQUEST_TIMEOUT_SECONDS`、`RequestGet`、`Sleep`、`requests` import 删除。

迁移后形态：

```python
async def fetch_sina_trade_calendar() -> str:
    async with AioHttpClient(headers=browser_text_headers()) as client:
        response = await client.request_text(...)
        return response.body
```

测试迁移：

- 更新 `pipeline/scheduler/tests/test_sina_trade_calendar.py`。
- 旧的 `requests.Response` fake 改为共享 client fake 或 aiohttp client fake。
- 保留解析器测试，不引入网络。
- 覆盖 timeout/transient retry、最终失败、空解析结果失败。

### 删除旧路径

迁移完成后执行：

```bash
cd /storage/program/fleur
rg -n "requests|aiohttp.ClientSession|TCPConnector|ClientTimeout|raise_for_status" pipeline/scheduler/src/scheduler/defs
test ! -d pipeline/scheduler/src/scheduler/defs/eastmoney
test ! -d pipeline/scheduler/src/scheduler/defs/http_client
```

验收要求：

- `aiohttp.ClientSession`、`TCPConnector`、`ClientTimeout` 只在 `http_resources/client.py` 直接使用。
- `requests` 不再被 `scheduler/defs` 使用。
- 顶层 `scheduler/defs/eastmoney` 包不存在。
- 顶层 `scheduler/defs/http_client` 包不存在。
- 如果 `requests` 在 `pipeline/scheduler/pyproject.toml` 中不再有运行时用途，应删除该依赖。

## 第三阶段：RFC 0003 实施基础设施

### 模块结构

在 `http_resources` 目录内扩展并完成 EastMoney 包合并：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/
  __init__.py
  client.py
  eastmoney/
  flatten.py
  jiuyan__action_field.py
  jiuyan__industry_list.py
  schemas.py
  ths__limit_up_pool.py
  schedules.py
  sina__trade_calendar.py
```

职责：

- `client.py`：共享 `AioHttpClient`、请求/响应模型、错误分类、重试、浏览器 header helper 和通用请求统计。
- `eastmoney/`：从原 `defs/eastmoney/` 整包移动而来，包含 EastMoney assets、client、schemas、fields、schedules。
- `flatten.py`：内容字段展平工具。
- `schemas.py`：RFC 0003 三个 asset 的 schema 和 content-to-table 转换。
- `jiuyan__action_field.py`：韭研 header factory、`jiuyan__action_field` 请求/响应校验和 asset。
- `jiuyan__industry_list.py`：韭研 header factory、`jiuyan__industry_list` 翻页和 asset。
- `ths__limit_up_pool.py`：同花顺 header、分页请求、`ths__limit_up_pool`。
- `schedules.py`：保留 Sina trade calendar job/schedule，并新增 RFC 0003 jobs/schedules。

不新增 `defs/jiuyan`、`defs/ths`、`defs/eastmoney` 或 `defs/http_client` 顶层目录。

### Raw schema 规则

RFC 0003 raw schema 规则必须集中实现，不能散落在 asset 函数中：

- 响应 envelope 字段不进入 raw Parquet：
  - 韭研：`errCode`、`msg`、`serverTime`
  - 同花顺：`status_code`、`status_msg`
- raw 列只来自接口内容字段。
- 不新增：
  - `request_*`
  - `source_endpoint`
  - `ingested_at`
  - 父级补充字段
  - 展开索引
- object/struct 按字段内容展平成列。
- array/list 保留数组边界；数组元素中的 object/struct 同样展平为数组列。
- 展平列名第一版暂定取原始字段路径中的最短字段名，也就是叶子字段名；不保留 `list[]`、`info[]`、`result[]` 等数组容器前缀，也不为父级 object 增加命名前缀。
- 暂不考虑重名冲突；如果实现层或下游工具必须要求列名唯一，再另行确认一版消歧规则。
- 上游原本就是 JSON 字符串的字段，保持字符串，不解析为新增结构字段。
- 所有标量第一版写为 `pa.string()`。

待确认字段命名示例：

```text
list[].article.action_info.time -> time
info[].code -> code
result[].imgs -> imgs
page.count -> count
trade_status.start_time -> start_time  # 示例；ths__limit_up_pool 当前不保留 trade_status
```

本轮用户建议下，暂不采用可逆 path 命名或转义命名，例如不采用：

```text
list[].article.action_info.time
info[].time_preview[]
list__article__action_info__time
info__time_preview
```

待确认后在 `schemas.py` 中集中维护该命名规则，并在 metadata 中记录 `flatten_column_naming="shortest_leaf"`。

#### 待确认字段清单

`jiuyan__action_field` 拟保留字段名：

```text
action_field_id
name
date
reason
sort_no
is_delete
delete_time
create_time
update_time
count
code
time
num
price
day
edition
shares_range
expound
```

`ths__limit_up_pool` 拟保留字段名：

```text
date
open_num
first_limit_up_time
last_limit_up_time
code
limit_up_type
order_volume
is_new
limit_up_suc_rate
currency_value
market_id
is_again_limit
change_rate
turnover_rate
reason_type
order_amount
high_days
name
high_days_value
change_tag
market_type
latest
```

`ths__limit_up_pool` 不保留以下字段：

```text
page.*
msg
trade_status.*
limit_up_count.*
limit_down_count.*
info[].time_preview
```

`jiuyan__industry_list` 拟保留字段名：

```text
industry_id
title_red
title_bold
title
author
imgs
keyword
content
is_top
status
sort_no
forward_count
browsers_count
is_delete
delete_time
create_time
update_time
```

### 展平工具设计

`flatten.py` 提供：

```python
def flatten_content_object(
    value: Mapping[str, object],
    *,
    naming: FlattenNaming,
) -> dict[str, object]:
    ...
```

要求：

- 输入必须是接口内容对象。
- 不接受响应 envelope 作为输入。
- dict/object 递归展开。
- list 保留为 list 值；list 内 dict 元素递归展开为同一 list 元素内的扁平 dict。
- 不为 list 元素增加 index。
- 不为子元素补父级字段。
- JSON 字符串不解析。
- `None` 保持 null。
- 数字、布尔值、字符串在 table 转换阶段统一字符串化。

## 第四阶段：RFC 0003 assets

### Single-run 范围回填约定

`jiuyan__action_field` 和 `ths__limit_up_pool` 都是交易日分区资产，但必须支持 Dagster single-run backfill。

实现要求：

- 两个资产使用同一个 `trade_date_dynamic_partitions`。
- 两个资产都配置 single-run backfill policy；日常调度仍只 materialize 一个交易日分区。
- 日期范围通过 Dagster backfill 选择的 partition range 传入，资产代码读取当前 run 的 partition key range / partition keys。
- 不绕过动态分区：待回填日期必须先存在于 `trade_date_dynamic_partitions`，且来自 `sina__trade_calendar` S3 Parquet。
- 若传入范围包含非交易日或未注册分区，run 启动前或资产入口处失败，不静默跳过。
- single-run 回填在一个 run 内展开为多个 `trade_date` 任务，并发请求多个交易日。
- 并发上限使用显式配置，建议第一版默认 `max_concurrent_trade_dates=4`，且硬上限不超过 20 个交易日，避免韭研配额和同花顺限流误判。
- 单个交易日内的分页语义保持不变：韭研 `action_field` 每日单次请求；同花顺 `limit_up_pool` 每日内部页码顺序请求，不做同一交易日内分页并发。
- 每个交易日任务完成 schema 转换后立即写入 S3 对应分区路径，不等待整个日期范围全部完成。
- 写入路径仍使用 `source/<asset_name>/trade_date=YYYY-MM-DD/000000_0.parquet` 语义。
- 写入必须按交易日粒度原子化：一个交易日成功只覆盖该交易日分区，不能重写同一 run 内其他交易日分区。
- 如果某个交易日失败，整个 run 失败；已经成功写入的交易日 S3 文件保留，重跑同一范围时允许幂等覆盖这些交易日分区。
- run metadata 记录 `backfill_start_date`、`backfill_end_date`、`requested_trade_date_count`、`completed_trade_date_count`、`failed_trade_date_count`、`max_concurrent_trade_dates`。

实现方式：

- 新增共享 helper，例如 `materialize_trade_date_range(...)`，供两个资产复用。
- helper 输入为 asset name、trade date 列表、单日 fetch coroutine、单日 table builder、S3 config 和并发配置。
- helper 内部用 `asyncio.Semaphore` 控制交易日并发。
- 单日 fetch 成功后立即调用现有 S3 Parquet 写入工具写出该交易日分区。
- 不把整个回填范围聚合成一个大 `pa.Table` 再交给 IO manager；否则无法满足“每完成一个交易日就写入 S3”。

### `jiuyan__action_field`

请求：

```text
POST https://app.jiuyangongshe.com/jystock-app/api/v1/action/field
```

请求体：

```json
{"pc": "1", "date": "<trade_date>"}
```

raw 行：

- raw 列只保留响应 `data[]` 板块对象中的以下外层字段：
  - `action_field_id`
  - `name`
  - `date`
  - `reason`
  - `sort_no`
  - `is_delete`
  - `delete_time`
  - `create_time`
  - `update_time`
  - `count`
- raw 列只保留 `list[]` 个股对象中的 `code`、`name`。
- raw 列只保留 `list[].article.action_info` 中的 `time`、`num`、`price`、`day`、`edition`、`shares_range`、`reason`、`expound`。
- 一行对应一个 `list[]` 个股对象，并补充同一板块对象的外层字段。
- 不保留 `status`、`article` 其他字段、`article.user.*`、`article_id`、`action_info_id`、`stock_id` 等未出现在上述清单中的字段。
- `data=[]` 或板块 `list=[]` 时写 0 行，但保留完整 schema。

响应校验：

- `errCode == "0"` 成功。
- `data` 必须是 array。
- `errCode != "0"` 失败，错误信息进入日志和 metadata，不进入 raw 列。

配额：

- 日频 schedule 只请求一个 `trade_date`。
- single-run 回填支持一个交易日范围，但第一版建议每次不超过 20 个 trade_date。
- 回填时按交易日并发请求；每个交易日完成后立即写入该日 S3 分区。

### `ths__limit_up_pool`

请求：

```text
GET https://data.10jqka.com.cn/dataapi/limit_up/limit_up_pool
```

raw 行：

- raw 列只保留外层 `date` 和 `data.info[]` 内的个股字段。
- 一行对应一个 `info[]` 个股对象，并补充同页外层 `date` 值。
- 不保留 `page.*`、`msg`、`trade_status.*`、`limit_up_count.*`、`limit_down_count.*`。
- 不保留 `info[].time_preview`。
- `info=[]` 时写 0 行，但保留完整 schema。

分页：

- 第一页 `page=1`。
- `data.page.count` 作为总页数。
- 后续页 `2..count` 顺序请求。
- 使用 `len(data.info)` 作为当前页实际行数。
- 跨页 duplicate fingerprint 检测基于 `data.info[]` 个股对象；重复时 asset 失败。

响应校验：

- HTTP 403 提示浏览器 header 问题。
- `status_code == 0` 成功。
- `status_code == -1` 且 `status_msg == "date参数不合法"` 失败，不写空表。

回填：

- single-run 回填支持一个交易日范围，但第一版建议每次不超过 20 个 trade_date。
- 不同交易日之间可以并发查询；同一交易日内部仍按页码顺序请求。
- 每个交易日完成跨页重复检测和 schema 转换后立即写入该日 S3 分区。

### `jiuyan__industry_list`

请求：

```text
POST https://app.jiuyangongshe.com/jystock-app/api/v1/industry/list
```

请求体：

```json
{"keyword": "", "start": "0", "limit": "500"}
```

raw 行：

- raw 列只保留 `data.result[]` 内的文章内容字段。
- 不保留 `data` 外层分页字段，例如 `pageNo`、`pageSize`、`totalCount`、`totalPages`、`hasNext`、`nextPage`、`hasPre`、`prePage`、`first`、`orderBy`、`order`、`autoCount`、`map`、`params`。
- 一行对应一个 `result[]` 文章对象，直接保存为普通 `pa.Table` 行。
- 不把 `result[]` 聚合成数组列；翻页只是采集过程控制，不进入 raw 表结构。
- `result[].imgs` 按上游原始字符串保留；不解析图片 URL。

分页：

- 第一次请求 `start="0"`。
- 若响应 `data.hasNext == true`，下一页 `start=str(data.nextPage)`。
- 若 `data.hasNext == false`，停止。
- 不使用本地页码自增。
- 不依赖 `totalCount` 或 `totalPages` 判断结束。

空结果：

- 全量翻页后所有 `result=[]` 视为异常并失败。

## 第五阶段：交易日动态分区

新增共享动态分区：

```text
trade_date_dynamic_partitions
source_asset="sina__trade_calendar"
source_storage="s3 parquet"
partition_key_name="trade_date"
fmt="%Y-%m-%d"
```

实现要求：

- schedule 评估时读取 S3 中已物化的 `sina__trade_calendar` Parquet。
- 不重新请求新浪远端接口。
- 将日历中尚未注册的交易日加入 Dagster dynamic partitions。
- 非交易日返回 `SkipReason`，不创建自然日分区。
- 日历缺失或不可读时返回 `SkipReason`，提示先 materialize `sina__trade_calendar`。
- single-run 回填启动前也必须先同步动态分区，保证传入日期范围只包含已登记交易日。
- 回填范围的日期过滤只使用 `sina__trade_calendar` S3 Parquet，不调用任何远端日历接口。

### 动态分区预热补充方案

除日常 schedule 评估时同步动态分区外，还可以把动态分区同步接到“交易日历可用”这个生命周期后面。

推荐方案：

- 将当前 schedule 内部的动态分区同步逻辑抽成通用 helper，例如：
  - 输入：`DagsterInstance`、`set[date]`。
  - 行为：读取已有 `trade_date_dynamic_partitions`，只向 Dagster instance 追加缺失的交易日分区。
  - 输出：本次新增的 partition key 列表。
- 如果已有 sensor 会判断 `sina__trade_calendar` 是否已物化，并在缺失时触发物化，则动态分区同步应在“确认交易日历已经物化成功”之后执行。
- 如果 sensor 本次只是发起 `sina__trade_calendar` 物化 run，不应在同一 tick 立即同步动态分区，因为 S3 Parquet 可能尚未写出完成。
- 更清晰的实现是新增一个 `asset_sensor` 监听 `sina__trade_calendar` 的 materialization event：
  - materialization 成功后读取 `sina__trade_calendar` S3 Parquet。
  - 调用共享 helper 补齐 `trade_date_dynamic_partitions`。
  - 不需要触发新的 run 时返回 `SkipReason`，只把同步结果写入 sensor 日志。
- 第一版 sensor 名称：
  - `sina__trade_calendar_dynamic_partitions_sensor`
- `http_resources__market_event_daily_schedule` 中的同步逻辑仍建议保留为防御性补偿；该同步是幂等的，可以避免 sensor 暂停或漏 tick 时导致日常调度看不到新交易日分区。

该方案的语义优于单纯“启动时预热”：

- 动态分区的事实来源是 `sina__trade_calendar` S3 Parquet，而不是进程启动时间。
- 每次交易日历刷新成功后都能自动补齐依赖它的动态分区。
- Web UI 中的分区列表会在交易日历可用且 sensor tick 完成后出现。

限制：

- 该方案依赖 Dagster daemon 正常运行 sensor；如果只启动 webserver 而没有 daemon，sensor 不会执行。
- 它不能保证 webserver 刚启动的第一秒就已经能看到分区。若必须做到 UI 打开前已完成预热，需要部署启动脚本在启动 webserver/daemon 前显式执行一次动态分区同步。

新增 job：

```text
http_resources__market_event_daily_job
```

包含：

1. `jiuyan__action_field`
2. `ths__limit_up_pool`

新增 schedule：

```text
http_resources__market_event_daily_schedule
cron_schedule = "45 16 * * *"
execution_timezone = "Asia/Shanghai"
```

single-run 回填：

```text
jiuyan__action_field.backfill_policy = dg.BackfillPolicy.single_run()
ths__limit_up_pool.backfill_policy = dg.BackfillPolicy.single_run()
job = http_resources__market_event_daily_job
partition_range = <start_trade_date>...<end_trade_date>
```

要求：

- 回填和日常调度复用同一组 partitioned assets。
- 日常 schedule 产生单日 partition run；手动 backfill 选择日期范围时由 single-run policy 合并为一个 run。
- `jiuyan__action_field` 与 `ths__limit_up_pool` 在同一个 run 内各自按交易日范围并行抓取，并按交易日独立写 S3 分区。

产业研究 snapshot job/schedule：

```text
jiuyan__industry_list_snapshot_job
jiuyan__industry_list_snapshot_schedule
cron_schedule = "30 17 * * *"
execution_timezone = "Asia/Shanghai"
```

## Definition 注册

更新：

```text
pipeline/scheduler/src/scheduler/defs/pipeline_defs.py
```

要求：

- EastMoney imports 改为 `scheduler.defs.http_resources.eastmoney...`。
- 注册 RFC 0003 三个 assets。
- 注册 market event daily job/schedule。
- 注册 `sina__trade_calendar_dynamic_partitions_sensor`。
- `jiuyan__action_field` 和 `ths__limit_up_pool` 注册 single-run backfill policy。
- 注册 industry list snapshot job/schedule。
- 保留现有 Sina、BaoStock、EastMoney definitions。

注意当前代码中模块路径是 `http_resources`，Sina asset group 是 `http_sources`。Plan 0004 只强制 EastMoney 包路径并入 `http_resources/eastmoney`，不强制重命名已有 Dagster group；RFC 0003 新 assets 第一版应与现有 Sina 保持同一 group 口径，除非另行做一次明确的 group rename 变更。EastMoney 既有 group 也保持 `eastmoney`，避免包路径移动改变 Dagster UI 分组。

## 测试计划

### 共享 client

新增测试覆盖：

- async context manager 正常关闭 session。
- GET text。
- GET JSON object。
- POST JSON object。
- HTTP 429/5xx 重试。
- HTTP 4xx，除 429，不重试。
- timeout / `aiohttp.ClientError` 重试。
- JSON decode error 重试。
- stats 计数正确。
- dynamic header factory 每次请求都会重新调用。

### EastMoney 迁移

保留并更新现有 `test_eastmoney.py`：

- request params 构造不变。
- page parse 行为不变。
- duplicate page row 仍失败。
- code 级 concurrency 仍由 EastMoney domain client 控制。
- materialization metadata key 不删减。

### Sina 迁移

保留并更新现有 `test_sina_trade_calendar.py`：

- parser 行为不变。
- fetch 成功返回 text。
- transient error retry。
- 最终失败抛错。
- `sina__trade_calendar` materialization metadata 不删减。

### RFC 0003

新增测试覆盖：

- 韭研 header factory 每次请求动态生成 timestamp。
- 缺失 `JIUYAN_TOKEN` / `JIUYAN_COOKIE` 失败。
- `jiuyan__action_field` 不写 envelope raw 列。
- `jiuyan__action_field` raw 列只保留板块外层指定字段、`list[]` 的 `code`/`name`、以及 `article.action_info` 的 `time`、`num`、`price`、`day`、`edition`、`shares_range`、`reason`、`expound`。
- `jiuyan__action_field` 一行对应一个 `list[]` 个股对象，不保留 `article.user.*` 等其他字段。
- `jiuyan__action_field` single-run 回填按日期范围并发抓取多个交易日。
- `jiuyan__action_field` 每完成一个交易日就写入对应 S3 分区，不等待整个范围结束。
- `ths__limit_up_pool` 不写 `status_code` / `status_msg` raw 列。
- `ths__limit_up_pool` raw 列只保留外层 `date` 和 `info[]` 内个股字段，一行对应一个 `info[]` 个股对象。
- `ths__limit_up_pool` 不保留 `page.*`、`msg`、`trade_status.*`、`limit_up_count.*`、`limit_down_count.*`、`info[].time_preview` raw 列。
- `ths__limit_up_pool` 跨页重复 fingerprint 失败。
- `ths__limit_up_pool` 380 天外参数错误失败。
- `ths__limit_up_pool` single-run 回填按日期范围并发抓取多个交易日，但同一交易日内部分页顺序请求。
- `ths__limit_up_pool` 每完成一个交易日就写入对应 S3 分区，不等待整个范围结束。
- `jiuyan__industry_list` 只用 `hasNext` / `nextPage` 翻页。
- `jiuyan__industry_list` 不请求 `start=1` 作为第一页。
- `jiuyan__industry_list` raw 列只保留 `result[]` 内文章字段，不保留外层分页字段。
- `jiuyan__industry_list` 保留 `result[].imgs` 原字符串。
- 三个 assets 都不包含 `request_*`、`source_endpoint`、`ingested_at` raw 列。
- 交易日动态分区只来自 `sina__trade_calendar` S3 Parquet。
- 动态分区同步 helper 只追加缺失的 `trade_date` partition key，重复执行时保持幂等。
- `sina__trade_calendar_dynamic_partitions_sensor` 注册到 definitions，并在 `sina__trade_calendar` materialization 后同步动态分区。
- 回填范围包含非交易日或未注册动态分区时失败。
- 部分交易日失败时 run 失败，但已完成交易日的 S3 分区文件保留，重跑允许幂等覆盖。

## 验证命令

开发验证：

```bash
cd pipeline
uv run pytest scheduler/tests/test_eastmoney.py
uv run pytest scheduler/tests/test_sina_trade_calendar.py
uv run pytest scheduler/tests/test_http_resources_client.py
uv run pytest scheduler/tests/test_http_resources_market_events.py
uv run dg check defs --target-path scheduler
```

真实接口小范围验证：

```bash
cd pipeline
uv run dg launch --assets sina__trade_calendar
uv run dg launch --assets jiuyan__action_field --partition <recent-trade-date>
uv run dg launch --assets ths__limit_up_pool --partition <recent-trade-date>
uv run dg launch --job http_resources__market_event_daily_job --partition-range "<start_trade_date>...<end_trade_date>"
uv run dg launch --assets jiuyan__industry_list
```

真实接口验证必须使用最近交易日范围，避免同花顺 380 天保留期和韭研配额误判。`dg launch --partition-range` 使用三个点 `...` 表示 inclusive range。

## 验收标准

第一阶段完成后：

- `scheduler/defs` 内只有 `http_resources/client.py` 直接使用 `aiohttp.ClientSession`。
- `sina__trade_calendar` 不再使用 `requests`。
- EastMoney 现有测试通过。
- Sina trade calendar 现有测试通过。
- `uv run dg check defs --target-path scheduler` 通过。

第二阶段完成后：

- Dagster UI 中能看到 RFC 0003 三个新增 raw assets。
- `jiuyan__action_field` 和 `ths__limit_up_pool` 使用交易日动态分区。
- `jiuyan__action_field` 和 `ths__limit_up_pool` 支持 single-run 日期范围回填。
- single-run 回填在一个 run 内并行查询多个交易日，并在每个交易日完成后立即写入对应 S3 分区。
- `jiuyan__industry_list` 使用 latest snapshot。
- raw 文件不包含响应 envelope 字段。
- raw 文件不包含请求/采集派生字段。
- 内容字段完整展平，不裁剪嵌套子字段。
- `result[].imgs` 等上游 JSON 字符串按字符串原样保留。
- 所有 HTTP 请求都通过共享 `AioHttpClient`。
- 新增 tests 和 `dg check` 通过。
