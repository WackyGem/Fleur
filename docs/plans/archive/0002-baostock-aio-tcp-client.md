# Plan 0002: BaoStock 异步 TCP 客户端与连接池

状态：草案

关联 RFC：

- `docs/RFC/archive/0001-market-data-ingestion.md`

参考资料：

- `docs/references/remote_server/baostock_tcp_server.md`
- BaoStock 官方 Python API 文档：`login`、`query_stock_basic`、`query_history_k_data_plus`
- Dagster 当前文档：资产返回数据对象，IO manager 负责持久化；`MaterializeResult[T]` 可携带输出值和元数据。

## 目标

实现一个用于 BaoStock TCP 服务的异步客户端，为后续两个 raw asset 提供稳定、可限流、可复用的查询能力：

1. `baostock__query_stock_basic`
2. `baostock__query_history_k_data_plus_daily`

客户端需要满足：

- 基于 `asyncio` TCP stream 实现。
- 按需创建连接并复用连接。
- 使用连接池限制并发，最大连接数为 30。
- 客户端创建时自动完成一次登录。
- BaoStock 只执行登录，不执行登出。
- 登录状态由 BaoStock 服务端保持约 1 小时；首个连接登录成功后，后续连接复用该服务端登录状态。
- 避免并发任务重复执行登录。
- 支持分页查询、压缩响应解码和统一错误处理。

## 非目标

本计划不包含：

- Dagster asset 的完整实现。
- ClickHouse 写入。
- dbt 模型。
- 分钟级 K 线接口。
- BaoStock Python 官方包的封装；本项目直接实现 TCP 协议客户端。

## BaoStock TCP 约束

基础连接信息：

```text
host = ${BAOSTOCK_HOST}
port = ${BAOSTOCK_PORT}
client_version = 00.9.10
server_version = 00.9.00
user_id = ${BAOSTOCK_USERNAME}
```

登录接口：

```text
api_name = login
request_code = 00
response_code = 01
password = ${BAOSTOCK_PASSWORD}
option = 0
```

环境变量：

```text
BAOSTOCK_HOST=public-api.baostock.com
BAOSTOCK_PORT=10030
BAOSTOCK_USERNAME=anonymous
BAOSTOCK_PASSWORD=123456
```

维护规则：

- `.env.example` 保存可提交的默认示例值。
- 本地 `.env` 保存运行时实际值，不提交到 Git。
- 客户端实现不得在代码中硬编码 BaoStock host、port、用户名或密码。
- `BAOSTOCK_PORT` 读取后转换为 `int`，转换失败应在客户端初始化阶段报错。

本项目假设：

- 登录状态保存在 BaoStock 服务端，生命周期约 1 小时。
- 不需要主动调用 `logout`。
- 首次登录后，新建 TCP 连接可以直接查询业务接口。
- 如果服务端返回 `10001001` 用户未登录，则客户端需要串行刷新登录并重试该请求。

## 总体设计

新增模块建议：

```text
pipeline/scheduler/src/scheduler/defs/config.py
pipeline/scheduler/src/scheduler/defs/baostock/
  __init__.py
  client.py
  protocol.py
  schemas.py
```

职责划分：

- `config.py`：集中声明和读取项目内 Dagster definitions 需要的环境变量与配置结构。
- `protocol.py`：BaoStock 报文编解码、CRC、分页、压缩响应处理。
- `client.py`：异步 TCP 连接、连接池、登录门闩、查询 API。
- `schemas.py`：查询字段常量、返回字段名、`pa.Table` schema 转换。

## 集中配置设计

新增配置模块：

```text
pipeline/scheduler/src/scheduler/defs/config.py
```

该模块负责整个 scheduler definitions 层所需的 `dg.EnvVar` 和运行时配置集中管理。其他需要环境变量的模块不得直接散落声明 `dg.EnvVar("...")`，必须从 `config.py` 引入对应配置或 EnvVar 常量。

建议结构：

```python
from dataclasses import dataclass

import dagster as dg


RUSTFS_ENDPOINT = dg.EnvVar("RUSTFS_ENDPOINT")
RUSTFS_BUCKET = dg.EnvVar("RUSTFS_BUCKET")
RUSTFS_ACCESS_KEY = dg.EnvVar("RUSTFS_ACCESS_KEY")
RUSTFS_SECRET_KEY = dg.EnvVar("RUSTFS_SECRET_KEY")
RUSTFS_REGION_NAME = "us-east-1"

BAOSTOCK_HOST = dg.EnvVar("BAOSTOCK_HOST")
BAOSTOCK_PORT = dg.EnvVar.int("BAOSTOCK_PORT")
BAOSTOCK_USERNAME = dg.EnvVar("BAOSTOCK_USERNAME")
BAOSTOCK_PASSWORD = dg.EnvVar("BAOSTOCK_PASSWORD")


@dataclass(frozen=True)
class S3Config:
    endpoint: str
    bucket: str
    access_key: str
    secret_key: str
    region_name: str = RUSTFS_REGION_NAME

    @classmethod
    def from_env(cls) -> S3Config:
        ...


@dataclass(frozen=True)
class BaostockClientConfig:
    host: str
    port: int
    username: str
    password: str
    max_connections: int = 1

    @classmethod
    def from_env(cls) -> BaostockClientConfig:
        ...
```

使用要求：

- `S3IOManager` 从 `config.py` 引入 RustFS EnvVar 常量作为资源默认值。
- S3 parquet 读取工具从 `S3Config.from_env()` 获取运行时字符串配置。
- `BaostockAioTcpClient` 从 `BaostockClientConfig.from_env()` 获取连接配置。
- `BAOSTOCK_PORT` 的整数转换和错误处理集中在 `config.py`，客户端初始化阶段只接收 `int`。
- `env.example` 仍然是可提交示例值来源；`config.py` 不保存真实密钥。

## 协议层设计

### 请求编码

实现函数：

```python
def encode_request(
    request_code: str,
    api_name: str,
    user_id: str,
    params: list[str],
    page: int = 1,
    page_size: int = 10000,
) -> bytes:
    ...
```

规则：

- 字段分隔符为 `\x01`。
- `body_length` 为从 `api_name` 开始的消息体长度，10 位左补零。
- `login` 和 `logout` 不带分页字段。
- 追加 `zlib.crc32(message_bytes)`。
- 结尾使用 `\n`。

### 响应解码

实现数据结构：

```python
@dataclass(frozen=True)
class BaostockResponse:
    response_code: str
    error_code: str
    error_message: str
    api_name: str
    user_id: str
    page: int
    page_size: int
    records: list[list[str]]
    field_names: list[str]
    params: list[str]
```

解码规则：

- 响应结束标记为 `<![CDATA[]]>\n`。
- 响应编码 `96` 需要按参考文档进行 zlib 解压。
- `records` 从 JSON 字段 `{"record":[...]}` 中解析。
- `field_names` 从响应中的字段列表解析。
- 分钟线 `time` 字段格式转换逻辑先保留在协议层，但第一阶段日线不会使用。

分页规则：

- 当 `record_count == page_size` 且 `record_count > 0` 时，认为可能存在下一页。
- 下一页请求沿用上一页的 `api_name`、`params`、`page_size`，`page += 1`。
- 客户端高层 API 默认聚合所有页。

## 异步连接设计

### 单连接对象

新增内部类：

```python
class BaostockTcpConnection:
    reader: asyncio.StreamReader
    writer: asyncio.StreamWriter
    lock: asyncio.Lock
```

约束：

- 单个 TCP 连接上同一时间只允许一个请求飞行。
- 每次请求都需要持有该连接自己的 `lock`。
- 请求写入后读取直到 `<![CDATA[]]>\n`。
- 网络错误、解码错误或服务端断开后，该连接标记为不可复用并关闭。

### 连接池

新增公共客户端：

```python
class BaostockAioTcpClient:
    max_connections: int = 1
```

客户端配置从集中配置模块引入：

```python
from scheduler.defs.config import BaostockClientConfig
```

`BaostockClientConfig.from_env()` 在 `config.py` 中实现并集中读取 `BAOSTOCK_HOST`、`BAOSTOCK_PORT`、`BAOSTOCK_USERNAME`、`BAOSTOCK_PASSWORD`。

连接池策略：

- 使用 `asyncio.Semaphore(max_connections)` 限制同时借出的连接数量。
- semaphore 容量必须与连接池最大连接数保持一致，默认都是 30。
- semaphore 是连接池入口门闩：协程必须先获得 semaphore，才能从空闲队列借连接或创建新连接。
- 这样可以避免大量协程同时进入连接创建/借用逻辑，导致连接池内部锁和队列被无意义阻塞。
- 使用 `asyncio.LifoQueue[BaostockTcpConnection]` 保存空闲连接，优先复用最近使用的连接。
- 按需创建连接：
  - 借连接时先尝试从空闲队列取连接。
  - 没有空闲连接且当前连接数小于 `max_connections` 时创建新连接。
  - 达到上限时等待 semaphore 和归还连接。
- 归还连接时：
  - 健康连接放回空闲队列。
  - 失效连接关闭并减少连接计数。
  - 无论连接健康与否，都必须在归还/关闭路径释放 semaphore。

并发模型：

- `max_connections` 同时表示最大 TCP 连接数和最大并发请求数。
- 单连接仍然有自己的 `asyncio.Lock`，防止同一连接上并发写入多个请求。
- semaphore 控制跨连接的全局并发；连接自己的 lock 控制单连接内的请求串行化。
- 不再单独设置第二个并发限制，避免连接池容量与请求并发上限不一致。

连接生命周期：

- 客户端作为 async context manager 使用。
- `__aenter__` 自动创建首个连接并登录。
- `__aexit__` 关闭所有空闲和已知连接。
- 不调用 BaoStock `logout`。

## 登录设计

### 登录门闩

客户端维护：

```python
_login_lock: asyncio.Lock
_logged_in: bool
_login_expires_at: float | None
```

登录流程：

1. 客户端创建时进入 `start()` 或 `__aenter__()`。
2. 创建首个 TCP 连接。
3. 在 `_login_lock` 内发送一次 `login` 请求。
4. 登录成功后设置：
   - `_logged_in = True`
   - `_login_expires_at = monotonic() + 55 * 60`
5. 首个连接放回池中复用。

并发保护：

- 所有需要刷新登录的路径都必须先获取 `_login_lock`。
- 获取锁后再次检查 `_logged_in` 和 `_login_expires_at`，避免重复登录。
- 同一时刻最多一个协程执行登录请求。

过期处理：

- 服务端登录约 1 小时，客户端按 55 分钟主动视为过期。
- 如果业务请求返回 `10001001` 用户未登录：
  1. 进入 `_ensure_logged_in(force=True)`。
  2. 串行重新登录。
  3. 对原请求重试一次。
- 如果重试后仍未登录，抛出认证错误，让调用方失败。

不登出原因：

- BaoStock 当前使用 anonymous 登录。
- 服务端登录状态会自然过期。
- 用户要求只登录不登出，减少连接池运行期间破坏其他连接状态的风险。

## 查询 API 设计

### `query_stock_basic`

方法签名建议：

```python
async def query_stock_basic(
    self,
    code: str = "",
    code_name: str = "",
) -> BaostockResponse:
    ...
```

请求参数：

```text
code
code_name
```

默认用于全量证券基础信息：

```python
await client.query_stock_basic()
```

输出转换：

- 高层可提供 `query_stock_basic_table()`，返回 `pa.Table`。
- 字段保持服务端字段名：
  - `code`
  - `code_name`
  - `ipoDate`
  - `outDate`
  - `type`
  - `status`

### `query_history_k_data_plus_daily`

方法签名建议：

```python
async def query_history_k_data_plus_daily(
    self,
    code: str,
    start_date: date,
    end_date: date,
) -> BaostockResponse:
    ...
```

固定参数：

```text
frequency = d
adjustflag = 3
```

字段：

```text
date,code,open,high,low,close,preclose,volume,amount,adjustflag,turn,tradestatus,pctChg,isST
```

重要约束：

- 历史回填按“单个 code + 日期范围”请求。
- 不按“单个 code + 单个日期”请求，避免请求量爆炸。

## 证券代码范围过滤组件

`baostock__query_history_k_data_plus_daily` 依赖 `baostock__query_stock_basic` 的证券基础信息。为减少 BaoStock K 线请求量，需要在公共工具中增加一个可复用的证券代码过滤组件。

建议放在：

```text
pipeline/scheduler/src/scheduler/defs/util.py
```

数据来源：

- `baostock__query_stock_basic` 已物化到 S3 的 parquet 快照。
- K 线资产通过公共 S3/PyArrow 读取工具读取 `source/baostock__query_stock_basic/000000_0.parquet`，得到 `pa.Table` 后再调用过滤组件。
- 必需字段：
  - `code`
  - `ipoDate`
  - `outDate`
  - `type`

BaoStock `type` 字段说明：

```text
1 股票
2 指数
3 其它
4 可转债
5 ETF
```

日频 K 线数据范围约束：

```text
type=1 股票：1990-12-19 至今
type=2 指数：2006-01-01 至今
type=5 ETF：2026-01-05 至今
```

第一阶段默认请求范围：

- 包含 `type in {"1", "2", "5"}`。
- 不请求 `type=3` 其它和 `type=4` 可转债，除非后续明确补充其日频 K 线数据范围。

设计数据结构：

```python
@dataclass(frozen=True)
class SecurityDateRange:
    code: str
    security_type: str
    start_date: date
    end_date: date
```

函数签名建议：

```python
def filter_active_security_ranges(
    stock_basic: pa.Table,
    requested_start_date: date,
    requested_end_date: date,
    allowed_security_types: frozenset[str] = frozenset({"1", "2", "5"}),
) -> list[SecurityDateRange]:
    ...
```

过滤规则：

1. 从 `stock_basic` 读取 `code`、`ipoDate`、`outDate`、`type` 字段。
2. 根据 `type` 找到 BaoStock 日频 K 线数据范围起始日期。
3. 计算证券存续区间：
   - `security_start = max(ipoDate, type_data_start_date)`
   - `security_end = outDate`，如果 `outDate` 为空，则视为开放结束日期。
4. 与请求区间取交集：
   - `effective_start = max(requested_start_date, security_start)`
   - `effective_end = min(requested_end_date, security_end)`，如果没有 `security_end`，则使用 `requested_end_date`。
5. 仅保留：
   - `type` 在 `allowed_security_types` 内。
   - `code` 非空。
   - `effective_start <= effective_end`。
6. 输出每个 code 的有效请求日期范围，K 线采集按该范围请求：

```python
await client.query_history_k_data_plus_daily(
    code=security_range.code,
    start_date=security_range.start_date,
    end_date=security_range.end_date,
)
```

日期解析要求：

- `ipoDate` 和 `outDate` 使用 `YYYY-MM-DD` 格式。
- 空字符串、空值、缺失 `outDate` 都表示未退市。
- 缺失或无法解析的 `ipoDate` 不能静默放大请求范围；该 code 应跳过并记录到 metadata。
- `outDate < ipoDate` 或与请求区间无交集时跳过。

K 线资产使用要求：

- 日常刷新和年度回填都必须通过该组件生成请求列表。
- K 线资产不得通过 Dagster function parameter 自动加载 `baostock__query_stock_basic` 的 output；应通过公共 S3/PyArrow 读取工具显式读取最新快照。
- 不允许直接对 `stock_basic` 全量 code 发起 K 线请求。
- asset metadata 需要记录：
  - `requested_start_date`
  - `requested_end_date`
  - `candidate_security_count`
  - `selected_security_count`
  - `skipped_security_count`
  - `selected_security_types`

## 限流与并发策略

连接池最大连接数为 1。

含义：

- 同时最多 1 个 TCP 请求在服务端侧进行。
- 这既是连接池上限，也是 BaoStock 查询限流机制。
- Dagster asset 内部并发采集时必须共享同一个客户端实例。

推荐调用方式：

```python
async with BaostockAioTcpClient(max_connections=1) as client:
    ...
```

批量采集 K 线时：

- 顺序调度多个证券代码，所有请求复用同一个已登录连接。
- 不额外再加大并发 semaphore；连接池本身就是全局限流。
- 任一 code 的日期范围请求失败时，K 线资产整体失败，不物化部分成功的年度分区。

## 错误处理

错误类型建议：

```python
class BaostockError(Exception): ...
class BaostockNetworkError(BaostockError): ...
class BaostockProtocolError(BaostockError): ...
class BaostockAuthenticationError(BaostockError): ...
class BaostockResponseError(BaostockError): ...
```

处理规则：

- TCP 连接失败、读写超时：关闭连接，按公共指数退避策略重试。
- CRC 或响应格式异常：关闭连接，抛出协议错误。
- `error_code == "0"`：成功。
- `error_code == "10001001"`：触发登录刷新并重试一次。
- 其他非 0 错误：抛出 `BaostockResponseError`，保留 `error_code`、`error_message`、`api_name` 和参数。

超时建议：

- TCP connect timeout：5 秒。
- 单次请求 read timeout：30 秒。
- 登录请求 read timeout：15 秒。

重试建议：

- BaoStock TCP 网络类错误复用 `scheduler.defs.util.DEFAULT_RETRY_POLICY`。
- 默认最多 3 次重试；加上首次请求，总共最多 4 次请求。
- 默认指数退避等待间隔固定为 1、2、4 秒。
- 协议错误不自动重试，除非明确是连接中断导致半包。
- BaoStock 业务参数错误不重试。

## Dagster 接入计划

第一阶段先实现客户端并直接在 dev 真实网络环境验证；随后资产接入：

### 前置改造

在接入 BaoStock 资产前，先完成当前 Dagster definitions 和 S3 IO 的结构性调整。该调整可以硬切，不需要兼容旧实现。

#### definitions 加载文件重命名

当前 definitions 加载文件：

```text
pipeline/scheduler/src/scheduler/defs/sina_trade_calendar_defs.py
```

该文件目前只加载 `sina__trade_calendar` 单个资产，文件名也绑定了 Sina 交易日历语义。BaoStock 接入后，该文件会同时注册交易日历、BaoStock 基础信息、BaoStock K 线、相关 jobs、schedules 和 shared resources，因此需要改成更通用的 pipeline definitions 文件。

重命名建议：

```text
pipeline/scheduler/src/scheduler/defs/pipeline_defs.py
```

迁移要求：

- 删除或重命名 `sina_trade_calendar_defs.py`，不保留兼容 shim。
- `pipeline_defs.py` 负责聚合 pipeline 域内的 assets、jobs、schedules、resources。
- 先迁移已落地的 `sina__trade_calendar`、`sina__trade_calendar_job`、`sina__trade_calendar_schedule` 和 `S3IOManager`。
- 后续在同一 definitions 文件中接入：
  - `baostock__query_stock_basic`
  - `baostock__query_history_k_data_plus_daily`
  - `baostock__daily_job`
  - `baostock__daily_schedule`
- 文件名必须反映领域聚合语义，不能继续使用只描述单个资产的 `sina_trade_calendar_defs.py`。

#### S3IOManager 硬切 PyArrow

当前 `S3IOManager` 已服务于 `sina__trade_calendar`，BaoStock 接入前必须先完成硬切：

- 移除 `boto3` object put 写入路径。
- 移除 `pyarrow.parquet.write_table` bytes 序列化写入路径。
- 统一使用 `pyarrow.fs.S3FileSystem` + `pyarrow.dataset.write_dataset` 写 parquet。
- 不需要兼容旧的 `table_to_parquet_bytes` 写入工具。
- 已落地的 `sina__trade_calendar` 必须随迁到新 writer，路径保持不变：

```text
source/sina__trade_calendar/000000_0.parquet
```

`sina__trade_calendar` 适配要求：

- 作为不分区资产写入，`write_dataset(..., partitioning=None)`。
- `base_dir` 为 `{bucket}/source/sina__trade_calendar`。
- `basename_template` 为 `000000_{i}.parquet`。
- 实际只允许输出一个文件：`000000_0.parquet`。
- 现有交易日历读取工具继续读取同一路径。
- 现有 `sina__trade_calendar` 相关测试需要同步更新，验证 PyArrow writer 仍能 round-trip 交易日历 parquet。

1. `baostock__query_stock_basic`
   - Dagster 不分区。
   - 使用 `BaostockAioTcpClient` 查询全量证券基础信息。
   - 返回 `pa.Table`。
   - 物理存储始终覆盖最新快照：

```text
source/baostock__query_stock_basic/000000_0.parquet
```

2. `baostock__query_history_k_data_plus_daily`
   - Dagster 按 `year` 分区。
   - 依赖最新的 `baostock__query_stock_basic` 快照。
   - 日常调度由交易日调度器触发，但提交的是当年 `year` 分区。
   - 日常刷新和年度回填都使用“单个 code + 日期范围”请求。
   - 每个 `year` 分区输出一个年度 parquet 文件。
   - 物理路径：

```text
source/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```

## 交易日历依赖设计

BaoStock 相关任务不能只依赖 cron 的自然日语义，必须依赖 `sina__trade_calendar` 资产产出的交易日历。

设计原则：

- `sina__trade_calendar` 是 BaoStock 调度与 K 线分区的事实来源。
- BaoStock schedule 每天按固定时间评估，但只有评估日期在交易日历中时才提交 run。
- 非交易日返回 `dagster.SkipReason`，不创建空 run。
- 如果 S3 中不存在交易日历 parquet，schedule 返回 `SkipReason`，并提示先物化 `sina__trade_calendar`。
- 不在 schedule 里重新请求新浪接口；schedule 只读取已物化的交易日历资产。

交易日判断：

```python
def is_trade_date(candidate: date, trade_dates: set[date]) -> bool:
    return candidate in trade_dates
```

调度时间建议：

- `baostock__daily_job`：交易日 `17:35 Asia/Shanghai`。
- 该 job 选中 `baostock__query_stock_basic` 和 `baostock__query_history_k_data_plus_daily`。
- BaoStock 官方参考说明日 K 线通常在当前交易日 `17:30` 完成入库，因此交易日调度不应早于 `17:30 Asia/Shanghai`。

## 交易日 Schedule Factory 设计

新增一个可复用的 `ScheduleDefinition` factory，命名为“交易日调度器”。它不是 Dagster 原生 cron 之外的自定义 scheduler 类型，而是用每日 cron 作为 tick 来源，在 schedule evaluation 阶段读取交易日历，并返回 `RunRequest` 或 `SkipReason`。

业务 schedule 不直接写交易日 `if` 判断，而是通过该 factory 声明：

- 固定 cron tick。
- 交易日历读取方式。
- 交易日到 Dagster partition key 的映射。
- 交易日到 run config/tags 的映射。
- 需要自动注册的 dynamic partition。

建议放在：

```text
pipeline/scheduler/src/scheduler/defs/baostock/schedules.py
```

签名建议：

```python
def build_trade_date_schedule(
    name: str,
    job: dg.UnresolvedAssetJobDefinition,
    cron_schedule: str,
    partition_key_fn: Callable[[date], str | None],
    run_config_fn: Callable[[date], dict[str, object]] | None = None,
    tags_fn: Callable[[date], dict[str, str]] | None = None,
    dynamic_partitions: list[tuple[dg.DynamicPartitionsDefinition, Callable[[date], str]]] | None = None,
    execution_timezone: str = "Asia/Shanghai",
) -> dg.ScheduleDefinition:
    ...
```

评估逻辑：

1. 从 `ScheduleEvaluationContext.scheduled_execution_time` 取得本次调度时间。
2. 转换到 `Asia/Shanghai`。
3. 取本地日期作为 `trade_date`。
4. 通过公共 S3 读取工具读取 `sina__trade_calendar` parquet。
5. 如果 `trade_date` 不在交易日历中，返回 `SkipReason`。
6. 使用 `dynamic_partitions` 注册调度需要的 dynamic partition，例如 `year=2026`。
7. 使用 `partition_key_fn(trade_date)` 生成 Dagster run 的 partition key。
8. 使用 `run_config_fn(trade_date)` 生成本次 run 的配置，例如刷新截止日期。
9. 返回 `RunRequest(partition_key=..., run_config=..., tags=...)`。

伪代码：

```python
def evaluate_trade_date_schedule(context: dg.ScheduleEvaluationContext):
    trade_date = context.scheduled_execution_time.astimezone(
        ZoneInfo("Asia/Shanghai")
    ).date()
    trade_dates = read_sina_trade_calendar_dates_from_s3(...)

    if trade_date not in trade_dates:
        return dg.SkipReason(f"{trade_date.isoformat()} is not an A-share trade date")

    for partitions_def, key_fn in dynamic_partitions:
        partition_key = key_fn(trade_date)
        ensure_dynamic_partition(context.instance, partitions_def, partition_key)

    return dg.RunRequest(
        partition_key=partition_key_fn(trade_date),
        run_config=run_config_fn(trade_date) if run_config_fn else {},
        tags=tags_fn(trade_date) if tags_fn else {},
    )
```

注意：

- Dagster daemon 仍然按 cron 触发 schedule evaluation；交易日过滤由交易日调度器在 evaluation 阶段完成。
- 不使用“周一到周五”代替交易日历，因为法定节假日和调休无法由简单 cron 表达。
- schedule evaluation 读取 S3 是有意设计，用真实已物化日历避免代码内硬编码节假日。
- 交易日调度器是可复用的 schedule factory；业务侧不得在每个 schedule 中重复实现交易日 `if` 判断。
- 如果未来需要完全事件驱动或更高频轮询，可以另行实现 sensor；当前“每日固定时间评估一次”的场景优先使用 schedule factory。

日常 BaoStock 行情任务使用方式：

```python
baostock__daily_job = dg.define_asset_job(
    name="baostock__daily_job",
    selection=[
        baostock__query_stock_basic,
        baostock__query_history_k_data_plus_daily,
    ],
)

baostock__daily_schedule = build_trade_date_schedule(
    name="baostock__daily_schedule",
    job=baostock__daily_job,
    cron_schedule="35 17 * * *",
    partition_key_fn=lambda trade_date: str(trade_date.year),
    run_config_fn=lambda trade_date: {
        "ops": {
            "baostock__query_history_k_data_plus_daily": {
                "config": {
                    "refresh_until_trade_date": trade_date.isoformat(),
                }
            }
        }
    },
    tags_fn=lambda trade_date: {
        "market.trade_date": trade_date.isoformat(),
        "market.year": str(trade_date.year),
    },
    dynamic_partitions=[
        (year_partitions, lambda trade_date: str(trade_date.year)),
    ],
)
```

## 日常自动化与回填设计

最终采用一个 K 线资产、一套年度分区物理布局、两种执行路径：

```text
baostock__query_history_k_data_plus_daily
  分区：year
  日常刷新：交易日调度器按交易日触发，刷新当年 year 分区到本次 trade_date
  历史回填：显式 materialize 某个 year 分区，刷新该完整年份
  写入：source/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```

不再设计 `baostock__query_history_k_data_plus_daily_compacted`。原因：

- 年度文件本身就是 canonical raw 资产，不再需要 raw daily + compacted 两层。
- 日常刷新和年度 backfill 都写同一个 `year=YYYY` 分区，语义一致。
- 下游读取只需要读取一个资产路径，不需要组合 compacted 和 recent delta。
- 交易日调度由可复用 ScheduleDefinition factory 集中封装，不使用 `AutomationCondition.eager()`。

日常刷新：

```text
17:35 Asia/Shanghai
  交易日调度器判断 scheduled date 是否为 A 股交易日
  如果是交易日：
    materialize baostock__query_stock_basic
    materialize baostock__query_history_k_data_plus_daily[YYYY]
    run config: refresh_until_trade_date=YYYY-MM-DD
```

K 线资产定义建议：

```python
class KLineDailyYearConfig(dg.Config):
    refresh_until_trade_date: str | None = None


year_partitions = dg.DynamicPartitionsDefinition(name="year")


@dg.asset(
    partitions_def=year_partitions,
    deps=[baostock__query_stock_basic],
    backfill_policy=dg.BackfillPolicy.single_run(),
)
def baostock__query_history_k_data_plus_daily(
    context: dg.AssetExecutionContext,
    config: KLineDailyYearConfig,
) -> dg.MaterializeResult[pa.Table]:
    ...
```

执行逻辑：

- 日常刷新 run 使用单个 year 分区，`context.partition_key` 是年份，例如 `2026`。
- single-run backfill 可覆盖多个 year 分区，必须使用 `context.partition_keys` 获取本次 run 选中的年份集合，例如 `["2025", "2026"]`。
- 如果 `config.refresh_until_trade_date` 存在，表示日常交易日刷新：
  - `start_date = YYYY-01-01`
  - `end_date = refresh_until_trade_date`
  - 校验 `refresh_until_trade_date` 属于当前 `year` 且在交易日历中。
- 如果 `config.refresh_until_trade_date` 不存在，表示年度回填：
  - 对 `context.partition_keys` 中的每个年份分别计算 `start_date = YYYY-01-01` 和 `end_date = YYYY-12-31`。
  - 每个年份只保留交易日历中属于该年份的日期。
- 先通过公共 S3/PyArrow 读取工具读取 `baostock__query_stock_basic` 最新快照。
- 对每个年份，使用证券代码范围过滤组件从该快照中过滤有效 code 和对应有效请求日期范围。
- 对每个年份、每个有效 `SecurityDateRange` 发起 `query_history_k_data_plus_daily(code, start_date, end_date)`。
- 使用 BaoStock TCP 连接池 30 并发限流。
- 写入前必须按年份组装 `pa.Table`：
  - 单个年份 run 生成一个年度 `pa.Table`。
  - 多年份 single-run backfill 生成 `dict[str, pa.Table]`，key 为年份。
  - 禁止把多个年份的数据合并成一个没有按 `year` 拆分的 `pa.Table` 后写入，避免两年的数据进入同一个 parquet 文件。
- 每个年度 `pa.Table` 必须带有 `year` 分区列，值固定为对应年份。
- 通过 PyArrow 写入一个或多个 `year=YYYY/000000_0.parquet`。

历史回填：

```text
输入：
  partition keys: 2025, 2026

请求：
  每个 code 请求一次 code + 2025-01-01..2025-12-31
  每个 code 请求一次 code + 2026-01-01..2026-12-31

输出：
  source/baostock__query_history_k_data_plus_daily/year=2025/000000_0.parquet
  source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet

run 数量：
  因为配置了 BackfillPolicy.single_run()，选择 2025 和 2026 两个分区时可以由一个 Dagster run 处理。
```

日常刷新：

```text
输入：
  scheduled trade_date: 2026-05-25
  partition key: 2026

请求：
  每个 code 请求一次 code + 2026-01-01..2026-05-25

输出：
  source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet
```

注意：

- 日常刷新会重写当年的年度 parquet 文件，这是本方案为简化读取和资产语义接受的成本。
- 补某一天的数据也通过重写对应 `year` 分区完成。
- 如果单个年度 table 内存压力过大，可以在资产内部按季度请求和拼接，但最终仍写一个 `year=YYYY/000000_0.parquet` 文件。
- 多年份 single-run backfill 的并行化边界是“年份 + code”；可以跨年份并行请求，但写入前必须先按年份归并。

## `stock_basic` 单文件存储设计

`baostock__query_stock_basic` 在 Dagster 逻辑上不分区，物理存储始终保持单个最新快照文件。

资产语义：

```text
baostock__query_stock_basic
```

物理路径：

```text
source/baostock__query_stock_basic/000000_0.parquet
```

这样设计的目的：

- K 线年度刷新只需要一个最新证券基础信息快照。
- 证券基础信息只保留最新快照，不为每个交易日写一份重复文件。
- 避免在同一个日常 job 中混用 `trade_date` 和 `year` 两套 partition key。

注意：

- S3 只保留一个最新快照文件。
- 该资产可由交易日调度器触发的 `baostock__daily_job` 每个交易日刷新。

## K 线年度分区设计

`baostock__query_history_k_data_plus_daily` 以年份作为 Dagster 分区和物理分区。

分区 key：

```text
YYYY
```

示例：

```text
2026
```

分区定义建议：

```python
year_partitions = dg.DynamicPartitionsDefinition(name="year")
```

使用动态分区的原因：

- 年份集合来自交易日历和实际回填范围，而不是代码常量。
- 避免在 code location import 阶段直接访问 S3 生成 `StaticPartitionsDefinition`。

调度时：

- 交易日调度器读取 S3 交易日历。
- 如果当天是交易日，确保当年 `year` 已注册到 `year_partitions`。
- 对 `baostock__daily_job` 发出 `RunRequest(partition_key=str(trade_date.year))`。
- run config 中传入 `refresh_until_trade_date=trade_date.isoformat()`。

回填时：

- 回填前先从 S3 读取交易日历。
- 将目标年份注册到 `year_partitions`。
- 对 K 线年份 partition 发起显式 materialize。
- 不为非交易日创建任何分区；非交易日只用于过滤 BaoStock 返回数据和校验。

K 线存储路径：

```text
source/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```

其中 `year=YYYY` 必须来自 partition key。

## PyArrow S3 与 Parquet 写入决议

S3/RustFS 上的 Parquet 读写统一使用 PyArrow，不再在新的 S3 读写路径中直接使用 `boto3` 上传或下载对象。

采用的 PyArrow 能力：

- `pyarrow.fs.S3FileSystem`：连接 S3-compatible storage，包括 RustFS。
- `pyarrow.dataset.write_dataset`：统一写 parquet dataset；不分区资产写成单文件 dataset，分区资产写成 Hive-style 分区 dataset。
- `pyarrow.parquet.read_table`：从 S3 直接读取单个 parquet 文件，适用于交易日历读取。
- `pyarrow.dataset.dataset` 或 `pyarrow.parquet.read_table`：读取已物化的 parquet 数据，适用于下游分析和测试校验。

S3 filesystem 构造建议：

```python
import pyarrow.fs as pafs


def build_s3_filesystem(config: S3Config) -> pafs.S3FileSystem:
    return pafs.S3FileSystem(
        access_key=config.access_key,
        secret_key=config.secret_key,
        endpoint_override=config.endpoint,
        region=config.region_name,
    )
```

路径约定：

- PyArrow S3 路径使用 `{bucket}/{object_key}` 形式传入。
- `endpoint_override` 只放在 `S3FileSystem` 配置中，不拼进 parquet path。
- 所有 object key 仍由统一路径解析函数生成。

统一写入示例：

```python
import pyarrow as pa
import pyarrow.dataset as ds


partitioning = ds.partitioning(
    pa.schema([("year", pa.string())]),
    flavor="hive",
)

ds.write_dataset(
    table,
    base_dir=f"{config.bucket}/source/baostock__query_history_k_data_plus_daily",
    filesystem=s3_filesystem,
    format="parquet",
    partitioning=partitioning,
    basename_template="000000_{i}.parquet",
    existing_data_behavior="delete_matching",
    use_threads=True,
)
```

不分区资产也使用同一写入入口：

```python
ds.write_dataset(
    table,
    base_dir=f"{config.bucket}/source/sina__trade_calendar",
    filesystem=s3_filesystem,
    format="parquet",
    basename_template="000000_{i}.parquet",
    existing_data_behavior="delete_matching",
    use_threads=True,
)
```

实际输出：

```text
source/sina__trade_calendar/000000_0.parquet
```

写入约束：

- 接受分区列不写入 parquet 文件内容；分区值由 Hive-style 路径表达。
- 每个分区只写一个文件；`write_dataset` 的 `basename_template` 必须包含 `{i}`，因此使用 `000000_{i}.parquet`，在每个分区只产生一个文件时实际文件名为 `000000_0.parquet`。
- 不分区资产也必须只生成一个 `000000_0.parquet` 文件；如果生成 `000000_1.parquet` 等额外文件，测试必须失败。
- 写入前必须保证传入 `write_dataset` 的 table 已按目标粒度整理好；K 线年度 table 必须包含 `year` 分区列。
- 多年份 single-run backfill 写入前必须按 `year` 组装数据；允许一次 `write_dataset` 接收包含多个 `year` 分区值的 table，但必须确认输出为多个 `year=YYYY/000000_0.parquet` 文件，不能产生跨年份单文件。
- 对 `baostock__query_history_k_data_plus_daily`，`base_dir` 必须是 `source/baostock__query_history_k_data_plus_daily`，分区列必须是 `year`。
- 不使用 `asyncio + boto3` 并发上传作为第一方案；分区并发、parquet 编码和 S3 写入交给 PyArrow dataset writer 的线程化实现。
- `existing_data_behavior="delete_matching"` 用于重写目标分区，确保重跑或补数时不会留下同一分区内的旧文件。
- 若 PyArrow 因输入 table 组织或 writer 参数在同一分区生成 `000000_1.parquet`、`000000_2.parquet` 等多文件，测试必须失败；第一阶段每个分区只接受 `000000_0.parquet`。

## S3 Parquet 读取工具

在公共工具模块中增加 S3 parquet 读取能力：

```text
pipeline/scheduler/src/scheduler/defs/util.py
```

配置结构从集中配置模块引入：

```python
from scheduler.defs.config import S3Config
```

新增通用读取函数建议：

```python
def read_parquet_table_from_s3(
    config: S3Config,
    asset_key: dg.AssetKey,
    *,
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    storage_mode: str = "partitioned",
) -> pa.Table:
    ...
```

该函数负责：

1. 使用统一路径解析规则得到 parquet object key。
2. 使用 `pyarrow.fs.S3FileSystem` 连接 RustFS。
3. 使用 `pyarrow.parquet.read_table` 从 `{bucket}/{object_key}` 直接读取 parquet table。
4. 对对象不存在、parquet 无法读取等错误抛出明确异常。

交易日历读取函数基于通用读取函数实现：

```python
def read_sina_trade_calendar_dates_from_s3(config: S3Config) -> set[date]:
    ...
```

读取流程：

1. 调用 `read_parquet_table_from_s3(config, dg.AssetKey("sina__trade_calendar"), storage_mode="latest_snapshot")`。
2. 从 `trade_date` 列解析为 `set[date]`。
3. 如果对象不存在、列不存在或为空，抛出明确异常，由 schedule 转换为 `SkipReason`。

该工具只负责读取已经物化的交易日历，不负责刷新交易日历。

BaoStock stock basic 快照读取函数基于通用读取函数实现：

```python
def read_baostock_stock_basic_from_s3(config: S3Config) -> pa.Table:
    return read_parquet_table_from_s3(
        config,
        dg.AssetKey("baostock__query_stock_basic"),
        storage_mode="latest_snapshot",
    )
```

`baostock__query_history_k_data_plus_daily` 必须通过该函数读取最新证券基础信息快照，再调用 `filter_active_security_ranges(...)`。

## 统一路径解析规则

路径解析规则必须从 `s3_io_manager` 中抽取到公共 util，供写入和读取共同复用。该抽取属于前置改造的一部分，必须先适配已落地的 `sina__trade_calendar`。

建议放在：

```text
pipeline/scheduler/src/scheduler/defs/util.py
```

函数建议：

```python
def asset_key_to_parquet_object_key(
    asset_key: dg.AssetKey,
    object_prefix: str = "raw",
    partition_key: str | None = None,
    partition_key_name: str | None = None,
    storage_mode: str = "partitioned",
) -> str:
    ...
```

规则：

- 非分区资产：

```text
{object_prefix}/{asset_key}/000000_0.parquet
```

- 单字段分区资产：

```text
{object_prefix}/{asset_key}/{partition_key_name}={partition_key}/000000_0.parquet
```

- 逻辑分区但单文件快照资产：

```text
{object_prefix}/{asset_key}/000000_0.parquet
```

示例：

```text
source/sina__trade_calendar/000000_0.parquet
source/baostock__query_stock_basic/000000_0.parquet
source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet
```

复用要求：

- `s3_io_manager` 写入 parquet 时必须调用该函数生成 object key。
- `read_parquet_table_from_s3`、`read_sina_trade_calendar_dates_from_s3`、`read_baostock_stock_basic_from_s3` 读取 parquet 时必须调用同一个函数生成 object key。
- 后续任何直接读取 raw parquet 的工具也必须复用该函数，避免读写路径漂移。
- `sina__trade_calendar` 的对象路径在硬切 PyArrow 后必须仍为 `source/sina__trade_calendar/000000_0.parquet`。

后续需要重构 `s3_io_manager`：

- 底层 S3 访问从 `boto3` 改为 `pyarrow.fs.S3FileSystem`。
- 所有 parquet 写入统一使用 `pyarrow.dataset.write_dataset`。
- 不分区资产通过 `partitioning=None` 写入 `{asset_key}/000000_0.parquet`。
- 分区资产通过 Hive-style `partitioning` 写入 `{asset_key}/{partition_key_name}={partition_key}/000000_0.parquet`。
- 支持通过 `context.partition_key` 写入分区 asset key 规则。
- 支持 asset 级别 `storage_mode`：
  - `latest_snapshot`：忽略 partition key，写单个快照文件。
  - `partitioned`：使用 partition key 写入分区路径。
- 支持 asset 级别 `partition_key_name`：
  - `year`：用于 `baostock__query_history_k_data_plus_daily`。
- 对分区资产输出类似：

```text
source/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```

`s3_io_manager` 的职责边界：

- 普通单分区 materialization：由 IO manager 根据当前 `context.partition_key` 写入一个 parquet 文件。
- K 线年度刷新：资产逻辑构造包含一个或多个 `year` 分区值的 table，并通过 PyArrow `write_dataset` 写入对应的 `year=YYYY/000000_0.parquet`。
- single-run backfill 覆盖多个年份时，writer 必须在 metadata 中列出每个年份对应的 object key 和 row count。
- IO manager 或公共 writer 必须把写入的分区数量、object key 列表、row count 写入 Dagster metadata。
- 不在 IO manager 中隐藏使用 `boto3` 执行多对象 fan-out；所有 S3 parquet I/O 统一走 PyArrow。

## 测试计划

不做 fake TCP server 测试，也不做单独的集成测试开关。原因是 BaoStock TCP 服务端的登录状态、压缩响应、分页行为、连接复用和限流表现都依赖真实服务端，fake server 难以暴露这些问题。

实现后直接用 dev 环境访问真实 BaoStock TCP 服务。测试环境必须从 `env` 读取：

- `BAOSTOCK_HOST`
- `BAOSTOCK_PORT`
- `BAOSTOCK_USERNAME`
- `BAOSTOCK_PASSWORD`

固定测试日期：

```text
2026-05-25
```

dev 真实网络测试内容：

- 登录成功。
- `query_stock_basic()` 返回非空记录。
- `query_history_k_data_plus_daily("sh.600000", date(2026, 5, 25), date(2026, 5, 25))` 能完成一次真实请求。
- 如果 `2026-05-25` 非交易日导致返回空记录，客户端仍应正确处理成功响应和空结果，不应误判为网络或协议失败。
- 连续单连接复用请求不会出现登录冲突。
- 触发网络错误或超时时，BaoStock TCP 客户端使用与新浪交易日历相同的 1、2、4 秒指数退避策略。
- dev 测试失败时直接暴露真实错误，不使用 mock 替代。

## 实现顺序

1. 新增 `config.py`，集中声明 `dg.EnvVar`、`S3Config` 和 `BaostockClientConfig`。
2. 将 `sina_trade_calendar_defs.py` 重命名为 `pipeline_defs.py`，并迁移已落地的 Sina 交易日历 definitions。
3. 将 `s3_io_manager` 硬切为 `pyarrow.fs.S3FileSystem` + `pyarrow.dataset.write_dataset` 统一写入，并从 `config.py` 引入 RustFS EnvVar 常量。
4. 更新 `sina__trade_calendar` 相关测试，确认路径仍为 `source/sina__trade_calendar/000000_0.parquet` 且 parquet round-trip 正常。
5. 新增 `protocol.py`，实现请求编码、响应解码、分页和压缩处理。
6. 复用 `scheduler.defs.util.DEFAULT_RETRY_POLICY` 作为 TCP 网络错误的指数退避策略。
7. 新增 `client.py`，实现 `BaostockTcpConnection` 和 `BaostockAioTcpClient`，并从 `config.py` 引入 `BaostockClientConfig`。
8. 实现客户端启动自动登录和 `_login_lock`。
9. 实现连接池：默认最大 1、按需创建、复用、关闭失效连接。
10. 实现 `query_stock_basic`。
11. 实现 `query_history_k_data_plus_daily`。
12. 使用真实 BaoStock dev 环境验证登录、查询、单连接复用和退避行为。
13. 新增 `schemas.py`，将返回记录转换为 `pa.Table`。
14. 在 `util.py` 中实现通用 S3 parquet 读取工具，以及交易日历和 BaoStock stock basic 的读取函数，并从 `config.py` 引入 `S3Config`。
15. 在 `util.py` 中实现证券代码范围过滤组件，基于 `ipoDate`、`outDate`、`type` 和请求日期范围生成有效 K 线请求列表。
16. 接入 `baostock__query_stock_basic` Dagster asset。
17. 实现交易日调度器 `build_trade_date_schedule`。
18. 接入 `baostock__query_history_k_data_plus_daily` 年度分区 asset，并强制通过 S3 读取 stock basic 快照后使用证券代码范围过滤组件减少请求量。
19. 接入 `baostock__daily_job` 和交易日调度。
