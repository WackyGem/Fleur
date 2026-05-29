# RFC 0006: Pipeline Scheduler 代码质量、模块化与可复用性优化

状态：草案（2026-05-29 复验后修订）

## 摘要

本文档针对 `pipeline/scheduler` 现有实现，从四个维度提出优化建议：

1. **代码质量**：消除跨模块重复函数、清理死代码、统一命名风格。
2. **模块化**：拆分职责过宽的 `util.py`、重新审视 `http_resources/` 包边界、整理配置模块。
3. **功能抽象**：提取 schema 转换、分页、metadata 构造、schedule/job 注册等通用抽象。
4. **可复用性**：沉淀共享测试工具、泛化对象存储和异步桥接模式、为后续数据源接入降低接入成本。

本 RFC 只定义优化方向和目标结构，不要求立即修改代码。

## 2026-05-29 复验结论

本次复验覆盖 `pipeline/scheduler`、`pipeline/migrate`、`pipeline/elt` 的当前文件结构，并重点核对 RFC 0006 已指出的问题。总体结论：

- **大多数代码质量问题仍然成立**：`_elapsed_seconds()`、`_required_string()`、`_positive_int_or_default()`、`_row_fingerprint()` 仍存在跨模块重复；`util.py` 仍承担重试、S3、Parquet、日期、证券过滤和资产 key 等多类职责；`config.py` 仍混合 Dagster `EnvVar` 声明和配置数据类。
- **东方财富相关描述需要修正**：当前实现已经从旧的 `http_resources/eastmoney/` 子目录扁平化为 `eastmoney.py`、`eastmoney_client.py`、`eastmoney_schema.py`、`eastmoney_fields.py`，并且 `EastmoneyAioHttpClient` 已经复用通用 `AioHttpClient`。因此“扁平化东方财富模块”和“让 eastmoney client 复用 AioHttpClient”不再应作为待办事项。
- **抽象建议仍然合理，但优先级应收敛**：schema 转换、分页、metadata builder、schedule/job 工厂仍有复用价值；但不建议先做大规模目录重组，应先做低风险去重和小型 helper 抽象，再评估是否需要按数据源拆包。
- **`postgres.py` 死代码数量已变化**：模块级便捷包装函数当前为 9 个，而不是原文的 8 个。所有业务调用点仍直接使用 `PostgresIndustryImageRepository`。
- **测试工具复用问题仍存在**：HTTP session/response fake、EastMoney client fake、数据库连接 mock、资产 context fake 分散在多个测试文件中，适合沉淀到 `tests/fakes/` 与 `tests/helpers.py`。
- **源码树存在本地生成产物**：当前工作区可见 `pipeline/scheduler/src/scheduler/**/__pycache__` 与 `.pyc` 文件。它们未被 Git 跟踪，但应避免进入提交和 review 范围。

## 目标

- 消除已识别的跨模块函数重复（至少 6 处），每处只保留单一来源。
- 将 `util.py` 从"万能工具箱"拆分为职责明确的子模块。
- 为 HTTP 资产层建立可复用的 schema 转换、分页和 metadata 抽象，降低新数据源接入成本。
- 清理 `postgres.py` 中的模块级冗余包装函数，统一调用路径。
- 将测试中的 Fake 对象和辅助函数沉淀为共享 test utilities，减少测试文件间复制。
- 为 schedule/job 注册引入声明式工厂，减少模板代码。
- 在完成目标代码框架和旧结构清理后、重写测试框架前，设置代码框架 review 门禁，确认没有引入新的代码异味，且抽象能力、复用性和可扩展性确实得到改善。
- 保持所有已有 asset key、S3 路径、数据语义和 API 行为不变。

## 非目标

- 不改变 Dagster asset key、S3 路径、Parquet schema 或分区策略。
- 不引入新的数据源或改变现有数据源的采集逻辑。
- 不引入 dbt 模型、ClickHouse 变更或下游应用改动。
- 不引入新的外部依赖（标准库和已有依赖范围内解决）。
- 不在本 RFC 中实现代码迁移。
- 不推翻 RFC 0005 的架构决策；本文档是 RFC 0005 的补充而非替代。

## 参考资料

既有 RFC/ADR：

```text
docs/RFC/0001-market-data-ingestion.md
docs/RFC/0002-eastmoney-f10-ingestion.md
docs/RFC/0003-http-resource-market-event-ingestion.md
docs/RFC/0004-jiuyan-industry-list-ocr.md
docs/RFC/0005-scheduler-resource-refactor-and-trade-date-backfill.md
```

当前实现参考：

```text
pipeline/scheduler/src/scheduler/defs/util.py
pipeline/scheduler/src/scheduler/defs/config.py
pipeline/scheduler/src/scheduler/defs/pipeline_defs.py
pipeline/scheduler/src/scheduler/defs/http_resources/client.py
pipeline/scheduler/src/scheduler/defs/http_resources/schemas.py
pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney_schema.py
pipeline/scheduler/src/scheduler/defs/http_resources/schedules.py
pipeline/scheduler/src/scheduler/defs/http_resources/partitioned.py
pipeline/scheduler/src/scheduler/defs/io_managers/postgres.py
pipeline/scheduler/src/scheduler/defs/baostock/assets.py
pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py
```

## 代码质量问题

当前代码库存在多处跨模块重复和命名不一致，增加维护成本并提高引入差异行为的风险。

### 函数重复

以下函数在多个文件中存在完全相同的实现：

| 函数 | 出现次数 | 所在文件 |
|------|---------|---------|
| `_elapsed_seconds()` | 4 | `baostock/assets.py`, `http_resources/eastmoney.py`, `http_resources/partitioned.py`, `io_managers/s3_io_manager.py` |
| `_required_string()` | 2 | `http_resources/jiuyan__action_field.py`, `http_resources/jiuyan__industry_list.py` |
| `_positive_int_or_default()` | 2 | `http_resources/eastmoney_client.py`, `http_resources/ths__limit_up_pool.py` |
| `_row_fingerprint()` | 2 | `http_resources/eastmoney_client.py`, `http_resources/ths__limit_up_pool.py` |

这些重复函数的行为逻辑完全一致，应当提取到共享模块中，消除维护时的同步风险。

### 命名不一致

`util.py` 中定义了两个日期解析函数，实现逻辑完全相同：

```python
def _parse_required_date(value: object) -> date | None:
    # ... implementation

def _parse_optional_date(value: object) -> date | None:
    # ... implementation (identical)
```

两者的唯一区别是函数名。`_parse_required_date` 语义上暗示日期必须存在，但实际返回 `None` 表示解析失败，与 `_parse_optional_date` 行为一致。建议统一为单一函数，或根据调用场景重新命名以明确语义差异。

### Schema 转换函数命名差异

`http_resources/schemas.py` 和 `http_resources/eastmoney_schema.py` 各自定义了字符串转换函数：

- `schemas.py`: `_string_or_null(value)`
- `eastmoney_schema.py`: `_stringify_value(value)`

两者逻辑相似但存在细微差异（如布尔值处理方式）。应当统一为单一实现，或在明确差异的前提下分别命名。

### 死代码

`io_managers/postgres.py` 当前定义了 9 个模块级便捷函数，每个函数都创建 `PostgresIndustryImageRepository` 实例并委托调用：

```python
def fetch_existing_image_urls(url: str, image_filenames: Sequence[str]) -> dict[str, str]:
    return PostgresIndustryImageRepository(url).fetch_existing_image_urls(image_filenames)

def fetch_images(url: str, image_filenames: Sequence[str]) -> list[dict[str, object]]:
    return PostgresIndustryImageRepository(url).fetch_images(image_filenames)

# ... 另外 7 个类似的包装函数
```

当前代码库中所有调用点都直接使用 `PostgresIndustryImageRepository` 类，这些模块级函数从未被调用。应当删除这些死代码，避免维护无用的 API 表面。

### 测试工具重复

测试文件中定义了多个 `FakeXxxClient` 和 `FakeXxxSession` 类，用于模拟 HTTP 客户端和会话。这些 fake 对象在多个测试文件中重复定义，缺乏共享的测试工具库。

## 模块化问题

当前模块边界存在职责混杂和过度聚合，影响代码的可维护性和可理解性。

### `util.py` 职责过宽

`defs/util.py` 当前承载了多类不相关的能力：

- **重试策略**：`ExponentialBackoffPolicy` 和 `DEFAULT_RETRY_POLICY`
- **S3 文件系统操作**：`build_s3_filesystem()`、`write_bytes_to_filesystem()`、`read_bytes_from_filesystem()`
- **Parquet 数据集写入**：`write_parquet_dataset()`
- **Parquet 读取**：`read_parquet_table_from_s3()`、`read_sina_trade_calendar_dates_from_s3()`、`read_baostock_stock_basic_from_s3()`
- **证券类型过滤**：`filter_active_security_ranges()`、`SecurityDateRange`
- **日期解析**：`is_trade_date()`、`_parse_required_date()`、`_parse_optional_date()`
- **资产 key 常量**：`SINA_TRADE_CALENDAR_ASSET_KEY`、`BAOSTOCK_STOCK_BASIC_ASSET_KEY` 等
- **S3 路径构造**：`asset_key_to_parquet_object_key()`

这些能力中只有重试策略和 S3 基础设施属于真正的"通用工具"，其余都是特定业务领域的辅助函数。建议拆分为：

```text
defs/util/
  __init__.py              # 只 re-export 通用能力
  retry.py                 # ExponentialBackoffPolicy
  s3.py                    # S3 文件系统、Parquet 读写
  constants.py             # 资产 key 常量
  dates.py                 # 日期解析辅助
  baostock.py              # BaoStock 证券过滤逻辑
```

或者保持扁平结构但明确职责边界：

```text
defs/
  retry_policy.py          # 重试策略
  s3_io.py                 # S3 基础设施
  baostock_helpers.py      # BaoStock 专用辅助
```

### `http_resources/` 过度聚合

`http_resources/` 当前包含所有通过 HTTP 协议采集的数据源资产：

- Sina 交易日历
- 韭研异动板块
- 韭研产业研究列表
- 韭研产业研究 OCR
- 同花顺涨停池
- 东方财富 8 个财务报表资产

这些数据源共享 HTTP 客户端基础设施，但业务逻辑、schema 转换、分页策略和调度节奏各不相同。随着数据源增加，该包会持续膨胀。

建议保留 `http_resources/` 作为共享基础设施层，但明确区分：

```text
defs/http_resources/
  client.py                # 通用 HTTP 客户端（已有）
  schemas.py               # 通用 schema 转换辅助
  partitioned.py           # 通用分区物化框架
  schedules.py             # 通用 schedule/job 注册
  sina/                    # Sina 数据源专用
    trade_calendar.py
  jiuyan/                  # 韭研数据源专用
    action_field.py
    industry_list.py
    industry_ocr.py
  ths/                     # 同花顺数据源专用
    limit_up_pool.py
  eastmoney.py             # 东方财富资产工厂（当前已扁平化）
  eastmoney_client.py      # 东方财富请求构造、分页和统计
  eastmoney_schema.py      # 东方财富字段映射和 schema 转换
  eastmoney_fields.py      # 东方财富字段常量（自动生成）
```

或者保持当前扁平结构，但通过命名约定区分数据源：

```text
defs/http_resources/
  client.py
  schemas.py
  sina__trade_calendar.py
  jiuyan__action_field.py
  jiuyan__industry_list.py
  jiuyan__industry_ocr.py
  ths__limit_up_pool.py
  eastmoney__balance.py
  eastmoney__cashflow.py
  # ... 其他东方财富资产
```

### `config.py` 混杂原始环境变量与数据类

`defs/config.py` 同时定义了：

1. **原始环境变量声明**：`RUSTFS_ENDPOINT = dg.EnvVar("RUSTFS_ENDPOINT")` 等
2. **配置数据类**：`S3Config`、`BaostockClientConfig`、`PipelineDatabaseConfig`、`JiuyanOcrConfig`

原始环境变量声明是 Dagster 特有的配置加载机制，应当与业务配置数据类分离。建议：

```text
defs/
  env_vars.py              # 所有 dg.EnvVar 声明
  config.py                # 只包含配置数据类
```

或者将环境变量声明内联到数据类的 `from_env()` 方法中：

```python
@dataclass(frozen=True)
class S3Config:
    endpoint: str
    bucket: str
    access_key: str
    secret_key: str
    region_name: str = "us-east-1"

    @classmethod
    def from_env(cls) -> S3Config:
        return cls(
            endpoint=dg.EnvVar("RUSTFS_ENDPOINT").get_value(),
            bucket=dg.EnvVar("RUSTFS_BUCKET").get_value(),
            access_key=dg.EnvVar("RUSTFS_ACCESS_KEY").get_value(),
            secret_key=dg.EnvVar("RUSTFS_SECRET_KEY").get_value(),
        )
```

这样可以消除模块级的全局变量，使配置加载更加显式。

### 东方财富当前差异与剩余优化

东方财富数据源当前已经扁平化到 `http_resources/` 顶层：

- `eastmoney.py`：资产工厂和自然年分区物化逻辑
- `eastmoney_client.py`：东方财富请求构造、分页和统计
- `eastmoney_schema.py`：字段映射和 schema 转换
- `eastmoney_fields.py`：字段常量（自动生成）

当前 `EastmoneyAioHttpClient` 已经复用 `AioHttpClient` 处理底层 HTTP、重试和连接池。因此原先“扁平化东方财富模块”和“复用 `AioHttpClient`”两项已经完成，不应再作为实施目标。

剩余问题主要是：

- `eastmoney_schema.py` 仍维护独立的 `_stringify_value()`，和 `schemas.py` 的 `_string_or_null()` 行为接近但布尔值大小写、JSON separators 不完全一致。应明确这是业务语义差异还是历史偶然差异，再决定合并或重命名。
- `eastmoney_client.py` 的页码分页、重复行检测、`EastmoneyFetchStats` 同步逻辑仍是东方财富专用实现；如果 THS 或后续数据源继续出现类似分页，应提取轻量分页 helper。
- `code_concurrency_limit` 当前由 `EastmoneyAioHttpClient` 的业务层 semaphore 管理，同时底层 `AioHttpClient` 使用连接池限制。这种双层限制可以接受，因为它表达的是“按证券代码并发”而不是纯 HTTP 连接并发；不建议强行下沉到 `AioHttpClient`，除非后续多个数据源都需要同类业务并发维度。

## 功能抽象问题

当前代码库缺少通用的功能抽象层，导致相似逻辑在不同模块中重复实现。

### Schema 转换缺乏统一抽象

每个数据源都独立实现了 schema 转换逻辑：

- `baostock/schemas.py`：`response_to_table()`、`stock_basic_response_to_table()`
- `http_resources/schemas.py`：`jiuyan_action_field_to_table()`、`ths_limit_up_pool_to_table()`
- `http_resources/eastmoney_schema.py`：`eastmoney_endpoint_to_table()`
- `http_resources/jiuyan__industry_list.py`：内联的 `industry_list_to_table()`

这些函数遵循相似的模式：

1. 接收原始响应数据（`BaostockResponse`、`Mapping[str, object]` 等）
2. 验证字段存在性
3. 转换为 `pa.Table`
4. 记录未知字段数量

建议引入统一的 schema 转换抽象：

```python
@dataclass(frozen=True)
class SchemaConversionConfig:
    field_mapping: dict[str, pa.DataType]
    required_fields: frozenset[str]
    unknown_field_handler: Callable[[set[str]], None]

def convert_response_to_table(
    response: Mapping[str, object] | list[Mapping[str, object]],
    config: SchemaConversionConfig,
) -> pa.Table:
    # 统一实现
```

### 分页逻辑重复

多个数据源实现了分页逻辑，但缺乏共享抽象：

- 东方财富：基于 `page` 参数的分页，检测重复行
- 韭研产业研究列表：基于 `start/limit` 游标的分页
- 同花顺涨停池：基于 `page` 参数的分页，检测重复行

建议引入通用分页抽象：

```python
@dataclass(frozen=True)
class PaginationConfig:
    page_param_name: str | None
    limit_param_name: str | None
    cursor_param_name: str | None
    dedup_strategy: Callable[[list[Mapping]], list[Mapping]]

async def fetch_paginated(
    client: AioHttpClient,
    request: HttpRequest,
    config: PaginationConfig,
) -> list[Mapping[str, object]]:
    # 统一实现
```

### Metadata 构造缺乏标准化

每个资产函数独立构造 Dagster materialization metadata，导致字段命名和结构不一致：

- 部分资产记录 `row_count`，部分记录 `total_rows`
- 部分资产记录 `elapsed_seconds`，部分记录 `duration_seconds`
- S3 路径的表示方式不统一

建议引入 metadata builder：

```python
@dataclass
class AssetMetadata:
    row_count: int
    elapsed_seconds: float
    s3_keys: list[str]
    request_stats: HttpFetchStats | None

    def to_dagster_metadata(self) -> dict[str, object]:
        # 统一转换为 Dagster metadata 格式
```

### Schedule/Job 注册模板代码

`http_resources/schedules.py` 中存在大量重复的 job 和 schedule 定义：

```python
jiuyan__action_field_daily_job = dg.define_asset_job(
    name="jiuyan__action_field_daily_job",
    selection=[jiuyan__action_field],
)

ths__limit_up_pool_daily_job = dg.define_asset_job(
    name="ths__limit_up_pool_daily_job",
    selection=[ths__limit_up_pool],
)

# ... 另外 5 个类似的 job 定义
```

以及 schedule 评估函数的重复：

```python
def _evaluate_jiuyan_action_field_daily_schedule(
    context: dg.ScheduleEvaluationContext,
) -> dg.RunRequest | dg.SkipReason:
    return _evaluate_trade_date_daily_schedule(context, source="jiuyan")

def _evaluate_ths_limit_up_pool_daily_schedule(
    context: dg.ScheduleEvaluationContext,
) -> dg.RunRequest | dg.SkipReason:
    return _evaluate_trade_date_daily_schedule(context, source="ths")
```

建议引入声明式工厂：

```python
def build_trade_date_daily_job(
    asset: dg.AssetsDefinition,
    source_name: str,
) -> dg.JobDefinition:
    return dg.define_asset_job(
        name=f"{asset.key.to_python_identifier()}_daily_job",
        selection=[asset],
    )

def build_trade_date_daily_schedule(
    job: dg.JobDefinition,
    cron_schedule: str,
    source_name: str,
) -> dg.ScheduleDefinition:
    return dg.ScheduleDefinition(
        name=f"{job.name}_schedule",
        job=job,
        cron_schedule=cron_schedule,
        execution_timezone="Asia/Shanghai",
        execution_fn=lambda ctx: _evaluate_trade_date_daily_schedule(ctx, source_name),
    )
```

## 可复用性问题

当前代码库中的部分基础设施缺乏扩展性设计，增加新数据源的接入成本。

### AioHttpClient 缺乏中间件机制

`AioHttpClient` 当前是一个封闭实现，不支持请求/响应拦截或统计扩展。例如：

- 无法在全局层面记录所有 HTTP 请求的日志
- 无法在请求前后注入自定义逻辑（如 circuit breaker）
- 无法扩展 `HttpFetchStats` 以记录特定数据源的指标

建议引入中间件或 hook 机制：

```python
@dataclass
class HttpHooks:
    on_request_start: Callable[[HttpRequest], None] | None = None
    on_request_end: Callable[[HttpRequest, HttpTextResponse | HttpBytesResponse], None] | None = None
    on_error: Callable[[HttpRequest, Exception], None] | None = None

class AioHttpClient:
    def __init__(
        self,
        *,
        hooks: HttpHooks | None = None,
        # ... 其他参数
    ):
        self._hooks = hooks or HttpHooks()
```

### `partitioned.py` 框架过于特定

`http_resources/partitioned.py` 中的 `materialize_trade_date_range()` 专为交易日分区场景设计，难以复用于其他分区类型（如东方财富的年份分区）。

建议泛化为通用的分区物化框架：

```python
@dataclass(frozen=True)
class PartitionMaterializationConfig:
    partition_key_name: str
    filter_fn: Callable[[str], bool] | None
    max_partitions_per_run: int
    write_fn: Callable[[str, pa.Table], list[str]]

async def materialize_partitions(
    partition_keys: list[str],
    config: PartitionMaterializationConfig,
) -> dict[str, list[str]]:
    # 统一实现
```

### 对象存储抽象过度特化

`io_managers/image_object_store.py` 专为韭研产业研究图片设计，难以复用于其他二进制对象存储场景（如 PDF、音频文件等）。

建议泛化为通用对象存储抽象：

```python
@dataclass(frozen=True)
class ObjectStoreConfig:
    key_prefix: str
    content_type_validator: Callable[[str], bool] | None

class ObjectStore:
    def __init__(self, config: ObjectStoreConfig, s3_config: S3Config):
        self._config = config
        self._s3_config = s3_config

    async def write_bytes(self, key: str, data: bytes) -> str:
        # 统一实现

    async def read_bytes(self, key: str) -> bytes:
        # 统一实现
```

### 测试工具缺乏共享库

测试文件中定义了多个 fake 对象和辅助函数：

- `FakeAioHttpClient`、`FakeAioHttpSession`
- `FakeBaostockClient`、`FakeBaostockConnection`
- `mock_database_connection()` 上下文管理器
- `FakeResponse` 对象

这些工具分散在各个测试文件中，缺乏统一的测试工具库。建议创建：

```text
scheduler/tests/
  __init__.py
  conftest.py              # 共享 pytest fixtures
  fakes/
    __init__.py
    http_client.py         # FakeAioHttpClient
    baostock.py            # FakeBaostockClient
    database.py            # mock_database_connection
    responses.py           # FakeResponse
  helpers.py               # 通用测试辅助函数
```

## 优化建议汇总

### 代码质量优化

1. **提取重复函数**：将 `_elapsed_seconds()`、`_required_string()`、`_positive_int_or_default()`、`_row_fingerprint()` 提取到 `defs/util/helpers.py`。
2. **统一日期解析**：删除 `_parse_required_date()` 或 `_parse_optional_date()` 中的冗余函数，或根据语义重新命名。
3. **统一 schema 转换函数**：将 `_string_or_null()` 和 `_stringify_value()` 合并为单一实现，或在明确差异的前提下分别命名。
4. **删除死代码**：移除 `io_managers/postgres.py` 中的 9 个模块级便捷函数。

### 模块化优化

1. **拆分 `util.py`**：将重试策略、S3 基础设施、日期解析、证券过滤分离到独立模块。
2. **重组 `http_resources/`**：明确区分共享基础设施层和数据源专用层，避免过度聚合。
3. **分离环境变量与配置数据类**：将 `dg.EnvVar` 声明与配置数据类分离，或内联到 `from_env()` 方法中。
4. **收敛东方财富剩余重复**：保留当前扁平化结构，优先统一字符串转换、分页去重和 metadata 构造等仍重复的实现。

### 功能抽象优化

1. **引入 schema 转换框架**：定义 `SchemaConversionConfig` 和通用转换函数，减少各数据源的重复实现。
2. **引入分页抽象**：定义 `PaginationConfig` 和通用分页函数，支持多种分页策略。
3. **引入 metadata builder**：定义 `AssetMetadata` 数据类，统一 materialization metadata 的结构和命名。
4. **引入 schedule/job 工厂**：定义声明式工厂函数，减少模板代码。

### 可复用性优化

1. **为 `AioHttpClient` 添加 hook 机制**：支持请求/响应拦截和统计扩展。
2. **泛化分区物化框架**：将 `partitioned.py` 从交易日专用扩展为通用分区物化框架。
3. **泛化对象存储抽象**：将 `image_object_store.py` 从图片专用扩展为通用对象存储抽象。
4. **创建共享测试工具库**：将 fake 对象和辅助函数沉淀到 `scheduler/tests/fakes/` 和 `helpers.py`。

## 目标目录结构

建议的目标结构如下：

```text
pipeline/scheduler/src/scheduler/
  defs/
    __init__.py
    pipeline_defs.py
    config.py                # 只包含配置数据类
    env_vars.py              # 所有 dg.EnvVar 声明
    util/
      __init__.py            # re-export 通用能力
      retry.py               # ExponentialBackoffPolicy
      s3.py                  # S3 文件系统、Parquet 读写
      helpers.py             # _elapsed_seconds, _required_string 等
      dates.py               # 日期解析辅助
      constants.py           # 资产 key 常量
    http_resources/
      __init__.py
      client.py              # 通用 HTTP 客户端
      schemas.py             # 通用 schema 转换框架
      pagination.py          # 通用分页框架
      partitioned.py         # 通用分区物化框架
      metadata.py            # AssetMetadata builder
      schedules.py           # schedule/job 工厂
      sina/
        trade_calendar.py
      jiuyan/
        action_field.py
        industry_list.py
        industry_ocr.py
      ths/
        limit_up_pool.py
      eastmoney.py           # 当前已扁平化；资产工厂和自然年分区物化
      eastmoney_client.py    # 请求构造、分页、按证券代码并发控制
      eastmoney_schema.py    # 字段映射和 schema 转换
      eastmoney_fields.py    # 字段常量（自动生成）
    baostock/
      __init__.py
      assets.py
      client.py
      protocol.py
      schemas.py             # 复用通用 schema 转换
      schedules.py
    ocr/
      __init__.py
      client.py
      schemas.py
      service.py
    io_managers/
      __init__.py
      s3_io_manager.py
      object_store.py        # 通用对象存储抽象
      postgres.py            # 只保留 PostgresIndustryImageRepository
  tests/
    __init__.py
    conftest.py              # 共享 pytest fixtures
    fakes/
      __init__.py
      http_client.py
      baostock.py
      database.py
      responses.py
    helpers.py
    test_*.py                # 现有测试文件
```

## 实施顺序

1. **代码质量清理**（低风险，高收益）：
   - 提取重复函数到 `util/helpers.py`
   - 删除 `postgres.py` 中的死代码
   - 统一日期解析和 schema 转换函数命名

2. **创建功能抽象框架**（中风险，高收益）：
   - 引入 schema 转换框架
   - 引入分页抽象
   - 引入 metadata builder
   - 引入 schedule/job 工厂

3. **拆分 `util.py`**（中风险，中收益）：
   - 创建 `util/` 子包
   - 迁移重试策略、S3 基础设施、日期解析到独立模块

4. **重组 `http_resources/`**（高风险，高收益）：
   - 创建数据源子包
   - 迁移现有资产文件到对应子包
   - 更新 `pipeline_defs.py` 的导入路径

5. **增强基础设施可扩展性**（中风险，中收益）：
   - 为 `AioHttpClient` 添加 hook 机制
   - 泛化分区物化框架
   - 泛化对象存储抽象

6. **代码框架 Review 门禁**（高价值，强制）：
   - 在目标代码结构、公共抽象、数据源迁移和旧结构清理完成后执行
   - 在任何测试框架重写前执行
   - 重点确认没有新增代码异味，抽象边界清晰，复用能力和扩展能力得到实质改善

7. **沉淀测试工具库**（低风险，中收益）：
   - 创建 `tests/fakes/` 和 `helpers.py`
   - 迁移现有 fake 对象和辅助函数
   - 创建 `conftest.py` 共享 fixtures

如果采用 `docs/plans/0008-pipeline-rfc0006-quality-reusability-implementation.md` 的一刀切实施方式，阶段 1-5 可以在同一长期分支中整体完成；但阶段 5 完成后必须先通过代码框架 review，再进入测试框架调整。不得用新测试重写来掩盖代码框架本身的抽象问题。

## 阶段 5 后代码框架 Review 门禁

在完成目标代码结构搭建、基础设施迁移、HTTP 数据源迁移、Dagster definitions 重组和旧结构清理后，必须暂停测试框架调整，先进行一次专门的代码框架 review。

该 review 的目标不是检查测试是否已经恢复，而是确认代码框架本身是否达到 RFC 0006 的优化目标：

- 是否消除了原有重复、死代码和职责混杂。
- 是否没有引入新的万能模块、循环依赖、隐式 re-export、临时兼容层或测试专用生产代码。
- 公共抽象是否真正在多个数据源或模块中复用，而不是只把旧逻辑搬到新的文件名下。
- 数据源业务逻辑是否仍留在 source 层，HTTP、storage、market、metadata 等基础设施是否保持业务无关。
- 新增一个 HTTP 数据源时，是否只需要实现请求构造、payload 解析、schema 字段配置和资产注册，而不需要复制分页、metadata、S3、schedule 模板代码。
- Dagster asset key、job、schedule、partition、IO manager key 和 metadata 旧字段是否保持稳定。

### Review 检查清单

代码质量：

- `rg` 不再找到 RFC 0006 指出的重复 helper 定义。
- `defs/util.py`、旧 `pipeline_defs.py`、旧 `http_resources` 业务模块和临时兼容层已删除或不再承载逻辑。
- `repositories/` 中只暴露 repository 类，不保留模块级便捷包装函数。
- 模块级代码不做环境读取、网络连接、数据库连接或文件系统 I/O。
- 生产代码中没有为旧测试保留的 fake、patch hook 或兼容导入。

抽象能力：

- `common/` 只包含无业务含义的纯 helper。
- `storage/` 不知道具体数据源，只处理 S3、bytes 和 Parquet。
- `market/` 只表达跨数据源市场概念。
- `http/` 只提供 HTTP client、protocol、pagination、schema、partitioning、schedule/job 工厂。
- `sources/` 只保留数据源业务规则，并通过公共抽象组合能力。
- metadata builder 能覆盖当前资产的共同 metadata 模式，并保留旧字段兼容。

复用性与可扩展性：

- EastMoney、THS、JiuYan 的分页或去重逻辑不再复制核心算法。
- Sina、JiuYan、THS、EastMoney 的 schema 转换共享统一表构造和未知字段计数能力。
- 交易日分区资产和年分区资产能复用同一分区物化框架的核心流程。
- 对象存储能力不再限定于图片，JiuYan 图片逻辑只保留业务 key 映射和 content-type 校验。
- schedule/job 注册通过声明式 spec 或工厂表达，不再散落重复 wrapper。

类型与边界：

- `EnvVar.get_value()` 的 Optional 返回值在配置边界收敛。
- Dagster metadata 由明确 builder 生成，不再向 Dagster API 传入裸 `dict[str, object]`。
- HTTP fake 依赖 protocol，而不是继承真实 client。
- 外部 payload 可以使用 `Mapping[str, object]`，但进入业务核心后应尽快转换为明确结构。

### Review 产出

代码框架 review 必须产出一份简短记录，至少包括：

- 目标目录结构是否完成。
- 旧模块到新模块的迁移映射是否清晰。
- RFC 0006 原问题逐项是否已解决。
- 新发现的代码异味和处理结论。
- 是否允许进入测试框架重写阶段。

如果 review 发现抽象能力或复用性不足，应先回到代码框架调整，不得直接通过补测试来固化有问题的设计。

## 验收标准

完成本 RFC 后应满足：

1. 所有跨模块重复函数（`_elapsed_seconds`、`_required_string`、`_positive_int_or_default`、`_row_fingerprint`）只保留单一来源。
2. `util.py` 中的 `_parse_required_date()` 和 `_parse_optional_date()` 统一或明确语义差异。
3. `schemas.py` 和 `eastmoney_schema.py` 中的字符串转换函数统一或分别命名。
4. `io_managers/postgres.py` 中的 9 个模块级便捷函数被删除。
5. `util.py` 被拆分为职责明确的子模块。
6. `config.py` 中的 `dg.EnvVar` 声明与配置数据类分离。
7. 所有 schema 转换函数复用 `SchemaConversionConfig` 框架。
8. 所有分页逻辑复用 `PaginationConfig` 框架。
9. 所有 asset metadata 通过 `AssetMetadata` builder 构造，字段命名统一。
10. 所有 trade-date daily job/schedule 通过工厂函数生成。
11. `AioHttpClient` 支持 hook 机制。
12. `partitioned.py` 框架支持任意分区类型，不再局限于交易日。
13. `object_store.py` 支持任意二进制对象，不再局限于图片。
14. 阶段 5 后代码框架 review 已完成，并明确允许进入测试框架重写阶段。
15. 测试 fake 对象和辅助函数沉淀到 `tests/fakes/` 和 `helpers.py`。
16. 现有质量门禁通过。
17. 所有已有 asset key、S3 路径、数据语义和 API 行为保持不变。

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 大规模重构导致 import 路径断裂 | Dagster definitions 加载失败 | 分阶段迁移，每阶段运行 `uv run dg check defs` |
| 功能抽象过度设计 | 增加复杂度但无复用收益 | 第一版只支持现有数据源，不做多租户或插件化设计 |
| Schema 转换框架改变输出行为 | Parquet schema 或数据内容变化 | 迁移前后保持测试夹具一致，对比输出文件 |
| 分页抽象隐藏业务逻辑 | 难以调试特定数据源的分页问题 | 抽象层只处理通用逻辑，数据源特有逻辑保留在业务层 |
| Metadata builder 改变字段命名 | 下游依赖 metadata 字段的查询失败 | 明确 metadata schema，提供迁移指南 |
| 测试工具库迁移破坏现有测试 | 测试套件无法运行 | 迁移前后保持测试行为一致，逐步替换 |
| 一次 PR 范围过大 | review 和回滚困难 | 按实施顺序拆成多个小 PR |

## 兼容性

必须保持稳定：

- Dagster asset key：所有现有资产 key 不变
- S3 路径：所有 `source/<asset>/<partition_key>=<value>/000000_0.parquet` 路径不变
- Parquet schema：所有现有字段类型和名称不变
- API 行为：所有现有函数的输入输出语义不变
- Dagster metadata 字段：现有字段名称可以继续支持，新字段作为补充

允许变化：

- Python import path：模块重组会导致导入路径变化
- 函数名称：重复函数统一命名后，部分调用点需要更新
- 配置加载方式：环境变量声明位置可能变化
- 测试文件组织：fake 对象和辅助函数位置变化

迁移期间可提供兼容导入：

```python
# defs/util/__init__.py
from scheduler.defs.util.retry import ExponentialBackoffPolicy, DEFAULT_RETRY_POLICY
from scheduler.defs.util.s3 import build_s3_filesystem, write_parquet_dataset
from scheduler.defs.util.helpers import elapsed_seconds, required_string
# ... 其他 re-export

# 保持旧 import path 继续可用
```

但兼容层只能 re-export 新模块符号，并应在完成迁移后删除。不得在兼容模块中继续维护业务逻辑。

## 总结

本 RFC 识别了 `pipeline/scheduler` 当前实现中的四类优化机会：

1. **代码质量**：6 处跨模块重复函数、2 处命名不一致、1 处死代码、分散的测试工具
2. **模块化**：`util.py` 职责过宽、`http_resources/` 过度聚合、`config.py` 混杂环境变量声明；东方财富旧并行结构已消除，但仍有局部 helper 重复
3. **功能抽象**：缺乏 schema 转换框架、分页抽象、metadata builder、schedule/job 工厂
4. **可复用性**：`AioHttpClient` 缺乏 hook 机制、`partitioned.py` 过于特定、对象存储抽象过度特化、测试工具缺乏共享库

优化方向是：

- **消除重复**：每处逻辑只保留单一来源
- **明确边界**：每个模块只负责一类能力
- **提取框架**：将重复模式抽象为可配置组件
- **增强扩展**：为基础设施添加 hook 和插件机制

实施策略是：

- **目标态先行**：先完成代码框架目标结构，再重建测试框架
- **阶段 5 Review**：旧结构清理后先审查代码框架质量、抽象能力、复用性和扩展性
- **保持兼容**：所有已有 API 和数据语义不变
- **测试后置重建**：通过 review 后再围绕新架构重写测试，避免测试固化中间态设计

完成本 RFC 后，代码库将具备：

- 更低的维护成本（消除重复和死代码）
- 更好的可理解性（明确的模块边界）
- 更高的开发效率（可复用的框架和工具）
- 更低的接入成本（新数据源只需实现业务逻辑）

本 RFC 与 RFC 0005 互补：RFC 0005 关注架构层面的重组（OCR 模块、EastMoney 扁平化、交易日分区改造），本 RFC 关注代码质量和工程实践的持续改进。
