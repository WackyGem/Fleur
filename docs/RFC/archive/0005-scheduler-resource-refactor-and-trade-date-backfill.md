# RFC 0005: Scheduler 资源抽象重组与交易日回填入口优化

状态：草案

## 摘要

本文定义 `pipeline/scheduler` 中三类已落地能力的结构调整方案：

1. 将 `defs/jiuyan_industry_ocr` 拆分为通用 OCR 模块、韭研 HTTP 业务模块和 PostgreSQL 状态/连接模块。
2. 将 `defs/http_resources/eastmoney` 改造成与一般 `http_resources` 资产一致的组织方式，最大化复用 HTTP client、分页、schema 转换、S3 写入和调度模式。
3. 清理 `trade_date_dynamic_partitions` 及其相关同步逻辑，交易日类 HTTP 资产改用自然日分区，并在运行时按 `sina__trade_calendar` 过滤交易日。

本 RFC 只定义目标架构和迁移边界，不要求立即修改代码。

## 背景

当前 `jiuyan_industry_ocr` 目录同时承担多类职责：

- 韭研产业研究图片 URL 解析、图片下载、OCR asset 定义。
- OpenAI-compatible OCR 请求构造、response 抽取、OCR schema 解析。
- S3 图片对象读写。
- PostgreSQL `jiuyan_industry_images` 状态表访问。

这些能力中只有“产业研究图片如何发现、如何映射成业务行”属于韭研 OCR 业务；OCR HTTP 调用、图片对象读写和 PostgreSQL 状态访问都有复用价值，不应长期锁在 `jiuyan_industry_ocr` 顶层业务目录内。

当前 EastMoney F10 资产位于 `defs/http_resources/eastmoney/` 子目录，但它也属于 HTTP raw resource。它已有独立 client、schemas、assets、schedules，与 `http_resources/client.py`、`partitioned.py`、`schemas.py` 和一般 HTTP resource 资产存在重复边界。后续 HTTP 资产继续增加时，EastMoney 若保持特殊结构，会放大重复代码和注册复杂度。

当前交易日类 HTTP 资产使用 `trade_date_dynamic_partitions`。这能保证合法分区来自 `sina__trade_calendar`，运行时表现符合业务需求，但 Dagster Web UI 回填入口只能按动态分区 key 列表复选。对于常见的“从 2024-01-01 到 2024-03-31 回填”需求，逐日勾选不可用，也容易漏选或误选。

本 RFC 决议调整该取舍：分区表达层改用自然日 `DailyPartitionsDefinition`，让 Dagster Web UI 原生支持日期范围输入；业务执行层继续以 `sina__trade_calendar` 为事实来源，只对范围内交易日发送请求和写入 Parquet，非交易日只记录跳过 metadata。

## 目标

- 建立通用 `ocr` 模块，承载与具体数据源无关的 OCR 服务抽象。
- 将韭研产业研究 OCR 的业务资产迁回 `http_resources` 范围内，使韭研相关 HTTP/OCR 资产在同一数据源边界下组织。
- 将 PostgreSQL 连接、状态仓储和数据库资源整理到 `io_managers` 或同级基础设施模块，避免业务目录直接拥有数据库基础设施。
- 将 EastMoney F10 资产改造成一般 `http_resources` 资产形态，减少单独子目录和单独注册逻辑。
- 保持已有资产 key、S3 路径和数据语义稳定，除非本 RFC 明确说明迁移兼容策略。
- 为交易日回填提供 Dagster Web UI 原生日期范围选择入口，继续使用 `sina__trade_calendar` 作为交易日事实来源。
- 保留日常调度的交易日判断能力，非交易日不产生无意义 run。

## 非目标

- 不改变 OCR prompt、OCR 输出业务字段或现有 `jiuyan__industry_ocr` Parquet schema。
- 不改变 EastMoney 八个资产的远端接口、日期过滤字段或分页排序。
- 不引入 ClickHouse、dbt 模型或下游应用变更。
- 不废弃 `sina__trade_calendar` 作为交易日事实来源。
- 不把所有数据源强行改为同一种分区方案；年分区、自然日分区和最新快照仍按业务语义选择。
- 不在本 RFC 中实现代码迁移。

## 参考资料

既有 RFC/ADR：

```text
docs/RFC/archive/0002-eastmoney-f10-ingestion.md
docs/RFC/archive/0003-http-resource-market-event-ingestion.md
docs/RFC/archive/0004-jiuyan-industry-list-ocr.md
docs/ADR/0001-market-data-raw-assets-on-dagster.md
docs/ADR/0002-s3-parquet-storage-layout.md
docs/ADR/0003-trade-calendar-driven-market-schedules.md
docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md
```

当前实现参考：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/client.py
pipeline/scheduler/src/scheduler/defs/http_resources/partitioned.py
pipeline/scheduler/src/scheduler/defs/http_resources/schedules.py
pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/
pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/
pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py
pipeline/scheduler/src/scheduler/defs/pipeline_defs.py
```

Dagster 当前文档要点：

- `DailyPartitionsDefinition` / time-window partitions 支持按日期范围选择分区，并在 asset context 中暴露分区 key 范围。
- `BackfillPolicy.single_run()` 可让多个分区在一次 asset run 内处理。
- 对本项目交易日资产，Web UI 交互优先级高于动态分区集合精确性；合法交易日判断下沉到运行时。

## 目标目录结构

建议目标结构如下：

```text
pipeline/scheduler/src/scheduler/defs/
  http_resources/
    client.py
    partitioned.py
    schemas.py
    schedules.py
    eastmoney.py
    jiuyan__action_field.py
    jiuyan__industry_list.py
    jiuyan__industry_ocr.py
    sina__trade_calendar.py
    ths__limit_up_pool.py
  ocr/
    __init__.py
    client.py
    schemas.py
    service.py
  io_managers/
    __init__.py
    s3_io_manager.py
    postgres.py
    image_object_store.py
```

说明：

- `http_resources` 放置具体 HTTP 数据源资产和与 HTTP raw resource 紧密相关的业务转换。
- `ocr` 放置“给定图片字节和 OCR schema，调用 OCR 服务并返回文本/结构化结果”的通用能力。
- `io_managers` 放置外部存储访问抽象，包括 S3 Parquet IO manager、图片对象存储、PostgreSQL 连接和状态仓储。
- `image_object_store.py` 明确归属 `io_managers`。它是外部对象存储访问抽象，不随当前使用场景移动到 `ocr` 或韭研业务目录。

## 通用 OCR 模块设计

新增 `defs/ocr`，第一版只抽象现有 OpenAI-compatible chat completions OCR 服务，不设计多厂商 plugin 系统。

建议职责：

```text
ocr/client.py
  OcrClient
  OcrRequest
  OcrResponse
  build_image_data_url
  extract_chat_completion_content

ocr/schemas.py
  OcrJsonSchema
  OcrSchemaError
  normalize_json_array_content

ocr/service.py
  process_ocr_batch
  OcrBatchConfig
  OcrBatchResult
```

通用边界：

- 输入：图片 bytes、MIME type、模型名、prompt/schema 配置、并发数。
- 输出：OCR 文本内容、解析后的 JSON array 或 schema 校验错误。
- 负责 HTTP 请求、超时、重试、并发限制、响应 envelope 校验。
- 不知道 `industry_id`、`stock_name`、`theme_path`、`image_filename` 等韭研业务字段。
- 不直接读写 PostgreSQL 状态表。
- 不直接决定 S3 结果路径；结果路径由业务资产或对象存储抽象决定。

韭研专用内容迁出：

- `StockThemeSchema`、产业研究 prompt、`stock_name/theme_path/relation/source` schema 属于 `http_resources/jiuyan__industry_ocr.py` 或相邻 `jiuyan_ocr_schema.py`。
- `normalize_ocr_content` 中只服务韭研 JSON array 到业务表的逻辑留在 HTTP resource 业务层。
- 通用 `ocr` 模块只提供“按给定 JSON schema 解析并返回对象列表”的基础能力。

## 韭研 OCR 业务迁移

`jiuyan_industry_ocr` 顶层目录应被拆除或转为空兼容导入层。韭研 OCR 资产迁入 `http_resources`：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/jiuyan__industry_ocr.py
```

保留资产 key：

```text
jiuyan__industry_images
jiuyan__industry_ocr
```

保留输出路径：

```text
img/jiuyan__industry_images/<image_filename>
source/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

保留依赖关系：

```text
jiuyan__industry_list -> jiuyan__industry_images -> jiuyan__industry_ocr
```

迁移后的职责：

- `jiuyan__industry_images`：读取 `jiuyan__industry_list` raw Parquet，解析 `imgs`，调用图片下载/对象存储，写入 PostgreSQL 状态。
- `jiuyan__industry_ocr`：从 PostgreSQL 领取图片，从对象存储读取图片 bytes，调用通用 OCR client，执行韭研 schema 转换，写入单图 Parquet，更新 PostgreSQL 状态。
- 韭研业务层只组装流程，不持有底层连接创建细节。

Dagster group 建议统一为：

```text
group_name="http_sources"
```

原因是 `jiuyan__industry_list` 已属于 HTTP raw resource，OCR 是该 HTTP resource 的派生采集步骤。可通过 tag 保留更细粒度识别：

```text
tags={"source": "jiuyan", "layer": "source", "storage": "s3", "state": "postgres", "modality": "ocr"}
```

### 韭研 OCR 组合调度

`jiuyan__industry_list`、`jiuyan__industry_images`、`jiuyan__industry_ocr` 应作为一条完整链路组合调度，不再分别暴露图片下载和 OCR 的独立日常 job。

建议保留或新增一个统一 job：

```text
jiuyan__industry_ocr_pipeline_job
```

资产选择：

```text
jiuyan__industry_list -> jiuyan__industry_images -> jiuyan__industry_ocr
```

该 job 的运行语义：

- 先刷新 `jiuyan__industry_list` 最新快照。
- 再从最新快照发现图片，执行下载和 PostgreSQL 状态 upsert。
- 最后领取已下载图片执行 OCR。
- OCR 和图片下载仍保持幂等；已成功项默认跳过。
- 可通过 job run config 传入下载/OCR limit、force_download、force_ocr、image_filenames、max_concurrent_requests 等现有控制参数。

需要移除的独立 job：

```text
jiuyan__industry_images_job
jiuyan__industry_ocr_job
```

如果当前存在 `jiuyan__industry_ocr_full_job`，应评估后统一到 `jiuyan__industry_ocr_pipeline_job` 的 run config 语义中；除非有明确的人工运维场景需要保留，否则不再额外暴露第二套 OCR job。

调度建议：

- 使用一个 schedule 触发完整链路，例如 `jiuyan__industry_ocr_pipeline_schedule`。
- schedule 时区为 `Asia/Shanghai`。
- schedule 不需要交易日过滤；`jiuyan__industry_list` 是长期累计列表，OCR 状态由 PostgreSQL 幂等控制。
- 不再单独调度 `jiuyan__industry_images` 或 `jiuyan__industry_ocr`。

## PostgreSQL 与对象存储整理

将当前 `jiuyan_industry_ocr/postgres.py` 中的连接和 repository 迁移到基础设施层：

```text
pipeline/scheduler/src/scheduler/defs/io_managers/postgres.py
```

建议命名：

```python
PipelinePostgresConfig
PipelinePostgresResource
PostgresIndustryImageRepository
connect_pipeline_database
```

设计约束：

- `PipelineDatabaseConfig.from_env()` 可以保留在 `config.py`，但连接创建函数和 repository 不应位于韭研业务目录。
- `PostgresIndustryImageRepository` 第一版可继续是韭研图片状态专用仓储；它的位置在基础设施层，不代表其表结构变成通用状态表。
- 后续若出现第二类 PostgreSQL 状态表，应在同一模块内提取连接池/transaction helper，而不是复制连接逻辑。
- Alembic 迁移仍由 `pipeline/migrate` 管理，不移动到 scheduler。

将当前 `image_store.py` 迁移到：

```text
pipeline/scheduler/src/scheduler/defs/io_managers/image_object_store.py
```

设计约束：

- 图片对象存储负责稳定生成 image key、读写 bytes、写 OCR result table。
- S3 Parquet dataset 写入仍复用 `util.write_parquet_dataset` 或 `S3IOManager` 语义。
- OCR 业务层不直接构造 pyarrow S3 filesystem。

## EastMoney HTTP Resource 改造

EastMoney 仍保留八个资产和年分区语义：

```text
eastmoney__balance
eastmoney__cashflow_sq
eastmoney__cashflow_ytd
eastmoney__dividend_allotment
eastmoney__dividend_main
eastmoney__equity_history
eastmoney__income_sq
eastmoney__income_ytd
```

目标是把当前子目录：

```text
http_resources/eastmoney/assets.py
http_resources/eastmoney/client.py
http_resources/eastmoney/schemas.py
http_resources/eastmoney/fields.py
http_resources/eastmoney/schedules.py
```

压平或改造为一般 HTTP resource 模块：

```text
http_resources/eastmoney.py
http_resources/eastmoney_fields.py
```

可接受的替代方案是保留 `http_resources/eastmoney/` 子目录，但必须满足：

- assets 由 endpoint config 工厂生成，避免八个资产重复装饰器和重复 `_materialize_eastmoney_asset` 包装。
- HTTP 请求复用 `http_resources/client.py` 中的 `AioHttpClient`、header、retry、stats 模式。
- 分页逻辑抽象为 HTTP resource 通用 helper，EastMoney 只提供 endpoint config 和 response path。
- schedule 注册进入 `http_resources/schedules.py`，不再有独立 `eastmoney/schedules.py` 作为特殊注册入口。
- `pipeline_defs.py` 从统一 `http_resources` 导入 EastMoney assets/jobs/schedules。

### EastMoney 资产工厂

建议引入资产工厂：

```python
def build_eastmoney_asset(endpoint: EastmoneyEndpointConfig) -> dg.AssetsDefinition:
    ...

EASTMONEY_ASSETS = [build_eastmoney_asset(endpoint) for endpoint in ENDPOINT_CONFIGS]
```

资产工厂必须保留：

- `io_manager_key="s3_io_manager"`
- `partitions_def=year_partitions`
- `backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1)`
- `metadata={"storage_mode": "partitioned", "partition_key_name": "year", "allow_empty": True}`
- `pool="eastmoney_run_pool"`
- `deps=[baostock__query_stock_basic]` 以及当前为了执行顺序引入的上一个 EastMoney asset 依赖

如果继续需要固定执行顺序，应显式保留 `execution_ordering_dependency` metadata；若后续确认远端限流能由 pool 和 code 并发控制独立保证，再单独 RFC/ADR 取消链式依赖。

### HTTP client 复用

当前 EastMoney client 与通用 `AioHttpClient` 已有部分复用，但还保留独立 stats 和 request wrapper。改造后：

- EastMoney 不再定义第二套 HTTP 重试常量，除非远端确实需要覆盖默认值。
- EastMoney 特有的 `code_concurrency_limit` 保留在业务 fetch 层，不进入通用 HTTP client。
- `parse_eastmoney_page` 保留为 EastMoney response adapter。
- `_request_json` 不应隐藏通用 `HttpRequestError` 的核心 metadata；业务层可以包装成 `EastmoneyRequestError`，但 stats 来源仍是 `AioHttpClient.stats`。

### Raw 字段策略

EastMoney 当前输出包含以下请求派生字段：

```text
request_code
request_start_date
request_end_date
partition_year
source_endpoint
ingested_at
```

这些字段一刀切不再写入 Parquet。RFC 0003 对 HTTP raw resource 的原则是“不新增字段”，EastMoney 改造必须同步整改当前实现中追加请求派生字段的行为。

整改后，EastMoney raw Parquet 只包含接口内容字段。请求参数、采集窗口、endpoint、ingest time 等运行信息进入 Dagster materialization metadata 或日志，不进入业务 Parquet。

字段处理要求：

- `request_code` 不写入 Parquet；记录到 metadata 中的请求统计、样本或错误上下文。
- `request_start_date` / `request_end_date` 不写入 Parquet；记录到 metadata 的 `requested_ranges`。
- `partition_year` 不写入 Parquet；分区路径已表达年份。
- `source_endpoint` 不写入 Parquet；记录到 metadata。
- `ingested_at` 不写入 Parquet；采集时间由 Dagster run/materialization 事件表达。
- EastMoney schema 生成函数必须只根据 OpenAPI/接口内容字段生成 Parquet schema。
- 空表 schema 也不得包含这些请求派生字段。

该整改允许改变 EastMoney Parquet schema。因这些字段属于 source 层不应存在的技术字段，本 RFC 不提供向后兼容列保留方案。

## 交易日自然日分区调整

### 问题

`jiuyan__action_field` 和 `ths__limit_up_pool` 当前使用 `trade_date_dynamic_partitions`。动态分区能够精确表达合法交易日集合，但 Dagster Web UI 的 backfill 入口以分区复选为主，不适合输入范围。

目标用户操作应是：

```text
partition range = 2024-01-01...2024-03-31
```

Dagster Web UI 应展示日期范围选择，而不是要求用户复选 80 个交易日。运行时仍必须根据 `sina__trade_calendar` 过滤出闭区间内的交易日，只对交易日发送请求和写入 Parquet。

### 决策

清理 `trade_date_dynamic_partitions` 及其相关内容，交易日类 HTTP 资产改用自然日 daily partitions。

两个资产使用不同的自然日分区起始日期：

```text
jiuyan__action_field: start_date = 2021-01-01
ths__limit_up_pool:  start_date = 2025-01-01
```

建议定义：

```python
jiuyan_action_field_daily_partitions = dg.DailyPartitionsDefinition(
    start_date="2021-01-01",
    timezone="Asia/Shanghai",
)

ths_limit_up_pool_daily_partitions = dg.DailyPartitionsDefinition(
    start_date="2025-01-01",
    timezone="Asia/Shanghai",
)
```

两个 asset 均使用：

```python
backfill_policy=dg.BackfillPolicy.single_run()
metadata={
    "storage_mode": "partitioned",
    "partition_key_name": "trade_date",
    "partitions_def": "daily_partitions",
    "trade_date_filter": "sina__trade_calendar",
    "allow_empty": True,
}
```

注意：Dagster 分区 key 是自然日，S3 物理分区名仍使用 `trade_date=YYYY-MM-DD`，以保持已有读取语义。

### 运行规则

1. 读取 `sina__trade_calendar` S3 Parquet。
2. 从 `context.partition_keys` 取得本次 Dagster 选择的自然日分区范围。
3. 将自然日分区 key 解析为日期。
4. 在本次自然日范围内筛选存在于 `sina__trade_calendar` 的 A 股交易日。
5. 对交易日发送远端请求，转换为 `pa.Table`。
6. 只为交易日写入 `source/<asset>/trade_date=YYYY-MM-DD/000000_0.parquet`。
7. 对非交易日不发送请求，不写入 Parquet 文件。
8. 如果范围内没有交易日，asset run 应成功跳过远端请求，并在 metadata 中记录 `skipped_non_trade_date_count` 和空的 `processed_trade_dates`。

范围回填使用 `BackfillPolicy.single_run()`，在 Web UI 选择 `2024-01-01...2024-03-31` 时，单个 run 内处理该范围内所有自然日，实际请求集合由交易日历过滤。

### 分区与调度关系

日常调度继续使用 partitioned asset job，但不再使用一个共同的 `http_resources__market_event_daily_job` 同时调度两个资产。`jiuyan__action_field` 和 `ths__limit_up_pool` 拆成各自的 daily job/schedule，以匹配两个资产不同的分区起始日期、上游保留窗口和运维节奏。

```text
jiuyan__action_field_daily_job
ths__limit_up_pool_daily_job
```

日常调度规则：

- 如果调度日期不是交易日，返回 `SkipReason`，不提交 run。
- 如果调度日期是交易日，提交对应自然日 partition key。
- 日常调度不依赖动态分区同步。
- 两个 schedule 都读取同一个 `sina__trade_calendar` S3 Parquet 作为交易日事实来源。
- 两个 schedule 可以使用相同 cron 时间，也可以按上游服务稳定性错峰；第一版只要求独立定义，不强制错峰。

人工回填规则：

- 在 Dagster Web UI 直接选择日期范围 materialize/backfill。
- 非交易日由 asset 运行时过滤。
- 不新增 `http_resources__market_event_range_backfill_job`。
- 如果需要同时回填两个资产，可以在 UI 中分别对两个 asset 发起范围 backfill；不再通过共同 daily job 作为组合入口。

由于两个资产的起始日期不同，联合选择时应遵守 Dagster 对不同 partition definition 的限制。需要跨两个资产同时回填时，优先选择二者共同支持的日期范围：

```text
start_date >= 2025-01-01
```

如需回填 `2021-01-01` 至 `2024-12-31` 的韭研异动板块，只选择 `jiuyan__action_field`。

### Metadata

自然日范围 run 必须记录：

```text
backfill_start_date
backfill_end_date
requested_partition_count
requested_natural_date_count
processed_trade_date_count
skipped_non_trade_date_count
completed_trade_date_count
processed_trade_dates
skipped_non_trade_dates_sample
partition_row_counts
```

`partition_keys` 可继续记录本次 Dagster 选择的自然日 key；`processed_trade_dates` 记录实际发送请求并写入 S3 的交易日 key。

### 对 S3 分区语义的约束

虽然 Dagster 分区定义改为自然日，但 S3 输出语义仍是交易日分区：

- 写出的 S3 路径仍为 `source/<asset>/trade_date=YYYY-MM-DD/000000_0.parquet`。
- 只有交易日写出 Parquet。
- 非交易日不写空 Parquet，不创建对应 `trade_date=<non_trade_date>` 目录。
- 下游按 S3 `trade_date` 分区读取时，只会看到交易日。
- `allow_empty=True` 仍用于处理交易日远端返回空内容的情况；它不表示非交易日也要写空文件。

因此，现有 `S3IOManager` 对 partitioned output 的“返回表 key 必须等于 Dagster 选择的全部 partition key”约束不适用于这两个资产的范围 materialization。实现时应采用专用 helper 写入交易日子集，或扩展 IO manager 支持 sparse partition output。该能力必须只对明确声明的 sparse 资产启用，避免其它分区资产意外漏写分区却被视为成功。

建议 metadata 显式声明：

```python
metadata={
    "storage_mode": "partitioned",
    "partition_key_name": "trade_date",
    "sparse_partition_output": True,
}
```

### 清理范围

本次调整应删除或废弃以下概念：

```text
TRADE_DATE_DYNAMIC_PARTITIONS_NAME
trade_date_dynamic_partitions
sync_trade_date_dynamic_partitions
sina__trade_calendar_dynamic_partitions_sensor
```

`http_resources/partitioned.py` 可以保留为通用“按自然日范围过滤交易日并写分区”的 helper，但不再包含 Dagster dynamic partitions 相关代码。

### 不采用的方案

不继续使用 `DynamicPartitionsDefinition`。

原因：

- Web UI 对动态分区的主交互是 key 列表选择，不适合大范围回填。
- 当前业务更需要 Web UI 范围输入，而不是在 Dagster 分区层精确隐藏非交易日。
- 非交易日过滤可以在运行时用 `sina__trade_calendar` 保证，不会向远端发送请求或写入 S3。

不新增 config-driven 范围回填 job。

原因：

- `DailyPartitionsDefinition` 已能让 Web UI 直接输入日期范围。
- 新增 job 会形成第二套人工入口，增加文档和运维成本。
- 交易日过滤逻辑应该与 asset materialization 逻辑保持一致，而不是分散在专用回填 job 中。

不把交易日资产改为纯 unpartitioned latest snapshot。

原因：

- 市场事件天然以交易日为幂等边界。
- 下游读取和回填都需要按交易日定位。
- 非交易日过滤和已处理交易日观测会变差。

## 兼容性

必须保持稳定：

- Dagster asset key：
  - `jiuyan__industry_images`
  - `jiuyan__industry_ocr`
  - `jiuyan__action_field`
  - `ths__limit_up_pool`
  - 所有 `eastmoney__*` 资产
- 已有 S3 raw 路径。
- `sina__trade_calendar` 作为交易日事实来源。
- `s3_io_manager` 的 `storage_mode`、`partition_key_name` 和 `allow_empty` metadata 语义。
- EastMoney `year` 分区和日频 schedule 行为。

允许变化：

- Python import path。
- Dagster group name 中韭研 OCR 从 `jiuyan_industry_ocr` 调整为 `http_sources`。
- `pipeline_defs.py` 的导入来源和注册方式。
- EastMoney Parquet schema 删除请求派生字段，只保留接口内容字段。
- 韭研 OCR 调度入口从图片/OCR 独立 job 改为完整链路组合 job。
- 交易日类 HTTP 资产从动态分区改为自然日分区。
- `http_resources__market_event_daily_job` 拆分为 `jiuyan__action_field_daily_job` 和 `ths__limit_up_pool_daily_job`。
- 删除动态分区同步 sensor。
- 不新增范围回填 job，人工范围回填使用 Dagster 原生自然日分区 UI。

迁移期间可提供兼容导入：

```text
defs/jiuyan_industry_ocr/__init__.py
```

但兼容层只能 re-export 新模块符号，并应在完成迁移后删除。不得在兼容目录中继续维护业务逻辑。

## 实施顺序

1. **测试基线**：在迁移前补齐或确认现有单元测试覆盖 OCR schema、PostgreSQL repository、HTTP client、EastMoney paging 和 trade_date range helper。
2. **PostgreSQL 和对象存储迁移**：移动 repository、连接 helper、image object store 到 `io_managers`，保持 public 方法签名和行为不变。
3. **通用 OCR 模块**：提取 OpenAI-compatible OCR client、data URL、chat completion response 抽取和通用 JSON schema 解析。
4. **韭研 OCR 业务迁移**：将 asset 和韭研 schema/prompt 移入 `http_resources`，更新导入和 `pipeline_defs.py` 注册，并将三段资产组合为统一链路 job/schedule。
5. **EastMoney 改造**：引入 endpoint-driven asset factory，统一 schedule 注册，复用 `http_resources` 通用 HTTP 能力，并删除 Parquet 中的请求派生字段。
6. **交易日分区改造**：删除动态分区定义和同步 sensor，改用两个自然日分区定义，按交易日历过滤实际请求和写入。
7. **清理兼容层**：删除空业务目录、更新 `pipeline/scheduler/README.md` 和相关文档索引。

每一步都应能独立通过质量门禁，避免一次 PR 同时移动 OCR、EastMoney 和 backfill 入口导致 review 面过大。

## 测试要求

单元测试覆盖：

- OCR 通用模块：
  - data URL 构造。
  - chat completion response content 抽取。
  - JSON schema 解析错误与空数组处理。
- 韭研 OCR 业务：
  - `imgs` URL 提取、去重、文件名稳定生成。
  - PostgreSQL 成功/失败/领取状态路径。
  - OCR 结果转换仍输出 `industry_id, stock_name, theme_path, relation, source`。
  - 统一链路 job 选择 `jiuyan__industry_list`、`jiuyan__industry_images`、`jiuyan__industry_ocr` 三个资产。
  - `jiuyan__industry_images_job` 和 `jiuyan__industry_ocr_job` 不再注册。
  - 组合 job 能通过 run config 继续传递图片下载和 OCR 控制参数。
- PostgreSQL 基础设施：
  - 连接参数不泄漏到 metadata/log。
  - repository SQL 行为保持幂等。
- EastMoney：
  - endpoint config 工厂生成八个资产。
  - `data/get` 与 `data/v1/get` 分页解析。
  - duplicate page row 检测。
  - `refresh_until_date` 仍只允许单 year partition。
  - Parquet schema 不包含 `request_code`、`request_start_date`、`request_end_date`、`partition_year`、`source_endpoint`、`ingested_at`。
  - 空表 schema 也不包含上述请求派生字段。
  - 请求范围、endpoint 和采集统计进入 materialization metadata。
- 交易日自然日分区：
  - 范围校验。
  - 非交易日被过滤，且不发送 HTTP 请求。
  - 非交易日不写 Parquet。
  - 范围内无交易日时 run 成功跳过并记录 metadata。
  - `jiuyan__action_field` 分区起始日期为 `2021-01-01`。
  - `ths__limit_up_pool` 分区起始日期为 `2025-01-01`。
  - `jiuyan__action_field_daily_job` 只选择 `jiuyan__action_field`。
  - `ths__limit_up_pool_daily_job` 只选择 `ths__limit_up_pool`。
  - `http_resources__market_event_daily_job` 不再注册。
  - 写出的 S3 `trade_date` 分区集合与交易日历筛选结果一致。

集成验证：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
uv run dg check defs
```

对 dbt 无直接影响，本 RFC 不要求运行 dbt。

## 风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 大规模移动文件导致 import 断裂 | Dagster definitions 加载失败 | 分阶段迁移，每阶段运行 `uv run dg check defs` |
| EastMoney 工厂化改变 asset key | 既有 materialization lineage 断裂 | 工厂必须显式设置原 asset name/key |
| EastMoney 删除请求派生字段影响下游临时读取 | 依赖这些技术列的临时查询会失败 | RFC 明确 raw Parquet 只保留接口字段；请求上下文改从 metadata/run 事件读取 |
| PostgreSQL repository 移动后行为变化 | OCR 幂等和并发领取出错 | 迁移前后保持 SQL 文本和测试夹具一致 |
| 韭研 OCR 独立 job 移除影响人工运维 | 不能直接只跑图片下载或只跑 OCR | 统一链路 job 保留 run config 控制参数；需要人工只处理局部时通过 asset selection 或 limit/image_filenames 控制 |
| 自然日分区包含非交易日 | Dagster UI 中会看到非交易日分区 | asset 运行时读取 `sina__trade_calendar` 过滤；非交易日不请求、不写文件，并记录 skip metadata |
| Dagster 分区与 S3 分区集合不完全一致 | UI 中自然日分区可能 materialized，但 S3 只存在交易日目录 | RFC 明确 S3 以交易日为物理输出边界；metadata 记录 `processed_trade_dates` 与 `skipped_non_trade_dates_sample` |
| Sparse output 掩盖真实漏写 | 交易日分区漏写但 run 成功 | 只允许非交易日被 sparse 跳过；交易日请求失败或写入失败仍必须让 run 失败 |
| OCR 通用化过度设计 | 增加抽象成本但无复用收益 | 第一版只支持 OpenAI-compatible chat completions，不做 provider plugin |
| 一次 PR 范围过大 | review 和回滚困难 | 按实施顺序拆成多个小 PR |

## 验收标准

完成本 RFC 后应满足：

1. `jiuyan_industry_ocr` 不再承载业务实现；韭研 OCR asset 位于 `http_resources`，通用 OCR 调用位于 `ocr`。
2. PostgreSQL 连接和 `PostgresIndustryImageRepository` 位于基础设施层，业务 asset 只依赖 repository 接口。
3. `jiuyan__industry_list`、`jiuyan__industry_images`、`jiuyan__industry_ocr` 通过统一链路 job/schedule 组合调度。
4. `jiuyan__industry_images_job` 和 `jiuyan__industry_ocr_job` 不再注册；如无明确人工运维必要，`jiuyan__industry_ocr_full_job` 也应合并到统一链路 job 的 run config 语义。
5. EastMoney 八个资产仍可见、asset key 不变、year 分区不变、日频 schedule 行为不变。
6. EastMoney 注册和 schedule 由 `http_resources` 统一出口提供，`pipeline_defs.py` 不再引用 EastMoney 特殊 schedules 子模块。
7. EastMoney Parquet 输出不再包含 `request_code`、`request_start_date`、`request_end_date`、`partition_year`、`source_endpoint`、`ingested_at`。
8. EastMoney 请求上下文进入 materialization metadata，而不是业务 Parquet。
9. `trade_date_dynamic_partitions`、动态分区同步 helper 和 `sina__trade_calendar_dynamic_partitions_sensor` 被清理。
10. `jiuyan__action_field` 使用起始日期为 `2021-01-01` 的自然日分区；`ths__limit_up_pool` 使用起始日期为 `2025-01-01` 的自然日分区。
11. `http_resources__market_event_daily_job` 不再注册，替换为 `jiuyan__action_field_daily_job` 和 `ths__limit_up_pool_daily_job`。
12. `jiuyan__action_field_daily_job` 只选择 `jiuyan__action_field`；`ths__limit_up_pool_daily_job` 只选择 `ths__limit_up_pool`。
13. Web UI 可通过自然日分区日期范围执行市场事件范围回填，不需要逐日复选。
14. 范围回填使用 `BackfillPolicy.single_run()`，并在单 run 内根据 `sina__trade_calendar` 过滤交易日。
15. 非交易日不发送远端请求、不写入 Parquet；交易日远端空响应仍可按 `allow_empty=True` 写出空表。
16. Sparse partition output 只允许跳过非交易日；交易日处理失败不能被当作 skip。
17. 现有质量门禁通过。
