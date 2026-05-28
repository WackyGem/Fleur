# Plan 0005: 韭研产业研究图片 OCR 实施规划

状态：草案

关联 RFC：

- `docs/RFC/0004-jiuyan-industry-list-ocr.md`

参考资料：

- `docs/RFC/0003-http-resource-market-event-ingestion.md`
- `docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/references/jiuyan_images/README.md`
- `docs/references/openapi/jiuyan__industry_list.yaml`
- `docs/references/openapi/jiuyan__industry_ocr.yaml`
- `pipeline/scheduler/src/scheduler/defs/`
- `pipeline/scheduler/src/scheduler/defs/http_resources/`

## 目标

本计划在现有 Dagster / S3 / PostgreSQL 架构上，实现 `jiuyan__industry_list` 的图片下载与 OCR 抽取流程，产出可回溯、可重试、可并发执行的单图 OCR 结果。

核心目标：

- 从 `raw/jiuyan__industry_list` 的 `imgs` 字段解析图片 URL。
- 下载图片到 S3，并用 PostgreSQL 记录图片级下载状态。
- 调用本地 OCR 服务抽取个股、题材路径、关联说明和来源。
- 按单张图片写出独立 OCR 结果对象，避免共享 bucket 覆盖写。
- 保证幂等重跑：已成功图片默认跳过，失败图片可重试。
- 保证输出 schema 简单扁平，适合 4B OCR 模型稳定执行。

## 非目标

本计划不包含：

- 重新采集 `jiuyan__industry_list`。
- 股票代码标准化、实体消歧、题材归一或人工审核流程。
- OCR 原始响应、原始 JSON array 字符串、prompt 全文或图片 base64 的持久化。
- `ocr_version` 体系设计。
- bucket 快照式 OCR 汇总写法。
- dbt 模型、ClickHouse 表或前端页面。

## 当前约束

- OCR 模型约 4B，prompt 和 schema 需要尽量短、扁平、稳定。
- OCR 结果的最小业务单元是“个股 × 题材路径 × 关联说明 × 来源”。
- 同一个个股出现在多个题材中时必须保留多条记录。
- 第一版不限制单张图片大小。
- 第一版 OCR 调用并发固定为 6。
- 第一版不保留 OCR 原始 JSON array 字符串。

## 输出约定

OCR 输出字段固定为：

```text
stock_name, theme_path, relation, source
```

字段含义：

- `stock_name`：个股、公司或标的名称。
- `theme_path`：多级题材路径，用英文逗号 `,` 连接。
- `relation`：该行描述个股与题材关系的原文说明。
- `source`：该行来源信息，没有则为空字符串。

重复规则：

- 同一张图片内，只有 `stock_name + theme_path + relation + source` 完全一致时才去重。
- `stock_name` 相同但 `theme_path` 不同，必须保留。
- `stock_name` 和 `theme_path` 相同但 `relation` 不同，第一版也保留。

## 当前代码现状

当前 scheduler 已具备以下基础能力：

- `pipeline/scheduler/src/scheduler/defs/http_resources/jiuyan__industry_list.py` 已实现 `jiuyan__industry_list` raw snapshot asset。
- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py` 已实现 S3 Parquet 写入、`latest_snapshot` 和 `partitioned` 两种存储模式。
- `pipeline/scheduler/src/scheduler/defs/util.py` 已提供 S3 filesystem、Parquet 路径生成、读写工具。
- `pipeline/scheduler/src/scheduler/defs/pipeline_defs.py` 集中注册 assets、jobs、schedules、sensors 和 `s3_io_manager`。
- `pipeline/scheduler/src/scheduler/defs/config.py` 已集中定义 RustFS/S3 和韭研 API 相关环境变量。

当前缺口：

- 没有 PostgreSQL pipeline 状态库访问层。
- 没有 Alembic migration 工程和 `jiuyan_industry_images` 状态表。
- 没有图片 URL 解析、图片下载、图片对象写入和状态 upsert。
- 没有 OCR client、prompt/schema 校验、单图结果写入和 OCR 状态领取逻辑。
- 没有 `jiuyan__industry_images` / `jiuyan__industry_ocr` assets、job、测试和真实样例联调。

## 改动范围

### 新增文件

建议新增：

```text
pipeline/migrate/
  alembic.ini
  env.py
  script.py.mako
  versions/
    0001_create_jiuyan_industry_images.py

pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/
  __init__.py
  assets.py
  config.py
  image_urls.py
  image_store.py
  ocr_client.py
  ocr_schema.py
  postgres.py
  schemas.py

pipeline/scheduler/tests/test_jiuyan_industry_ocr_image_urls.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_schema.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_assets.py
```

文件职责：

- `assets.py`：定义 `jiuyan__industry_images` 和 `jiuyan__industry_ocr` Dagster assets，以及必要 job。
- `config.py`：读取 `PIPELINE_DATABASE_URL`、`JIUYAN_OCR_BASE_URL`、`JIUYAN_OCR_MODEL_NAME`、OCR 超时/重试/并发配置。
- `image_urls.py`：解析 `imgs` JSON 字符串、正则提取图片 URL、生成 `image_filename`。
- `image_store.py`：下载图片、校验 MIME、计算 SHA256、写入 `img/jiuyan__industry_images/<image_filename>`。
- `ocr_client.py`：封装 OpenAI-compatible `POST /v1/chat/completions` 调用。
- `ocr_schema.py`：校验 OCR 返回 JSON array，做字段补全、扩展字段过滤和去重。
- `postgres.py`：封装状态表 upsert、下载状态更新、OCR 任务领取和 OCR 状态更新。
- `schemas.py`：集中定义 PyArrow schema、状态枚举、dataclass/TypedDict。

### 修改文件

预计修改：

```text
pipeline/pyproject.toml
pipeline/scheduler/pyproject.toml
pipeline/scheduler/src/scheduler/defs/config.py
pipeline/scheduler/src/scheduler/defs/pipeline_defs.py
pipeline/scheduler/src/scheduler/defs/util.py
pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py
```

修改内容：

- `pipeline/pyproject.toml`：确认 Alembic 迁移命令可从 `pipeline/` 执行。
- `pipeline/scheduler/pyproject.toml`：增加 PostgreSQL client 依赖，建议 `psycopg[binary]`；如 OCR client 复用 `aiohttp`，无需新增 HTTP 依赖。
- `config.py`：新增 `PIPELINE_DATABASE_URL`、`JIUYAN_OCR_BASE_URL`、`JIUYAN_OCR_MODEL_NAME`、`JIUYAN_OCR_TIMEOUT_SECONDS`、`JIUYAN_OCR_MAX_RETRIES`、`JIUYAN_OCR_MAX_CONCURRENT_REQUESTS`。
- `pipeline_defs.py`：注册新增 OCR assets/job；如果第一版不做 schedule，只注册 manual job。
- `util.py`：如现有 helper 不足，补充 S3 object 读写 bytes 的小函数；不改变现有 Parquet 路径语义。
- `s3_io_manager.py`：原则上不改；如果单图 OCR 结果不走 IO manager，而由 asset 内部直接写对象，则只通过 metadata 记录对象 key。

### 不改动范围

本计划不改：

- 现有 `jiuyan__industry_list` 抓取逻辑、分页逻辑和 raw schema。
- 现有 `jiuyan__action_field`、`ths__limit_up_pool`、EastMoney、BaoStock 资产。
- 现有 S3 Parquet `latest_snapshot` / `partitioned` 语义。
- dbt 项目、ClickHouse 表、前端页面。
- 已有 raw 数据路径规范，除非另有 ADR/RFC 变更。

## 总体实施顺序

本计划按以下顺序实施：

1. 梳理并冻结 OCR 输出 schema 和 prompt 规则。
2. 增加 PostgreSQL 状态表与迁移。
3. 实现图片下载 asset。
4. 实现 OCR 单图抽取 asset。
5. 补齐 Dagster 注册、测试和样例联调。
6. 如有需要，再补一个独立的汇总资产用于后续查询优化。

## 实施方案细节

### Asset 写入方式

`jiuyan__industry_images` 和 `jiuyan__industry_ocr` 都属于“有副作用的 pipeline asset”：

- `jiuyan__industry_images` 写入多个图片对象，并更新 PostgreSQL 状态。
- `jiuyan__industry_ocr` 读取多个图片对象，调用 OCR，写入多个单图 Parquet 对象，并更新 PostgreSQL 状态。

这两个 asset 第一版不应把返回值交给现有 `S3IOManager` 托管，因为当前 `S3IOManager` 只表达单个 latest snapshot 或 Dagster partitioned 输出。OCR 资产需要在一次 materialization 中写多个由 `image_filename` 决定的对象 key，因此建议：

- asset 函数内部显式使用 `S3Config.from_env()`、`build_s3_filesystem()` 和 PyArrow 写对象。
- asset 返回 `dg.MaterializeResult(metadata=...)`，不返回 `pa.Table`。
- definition metadata 标记为 `storage_mode=object_per_image` 或 `storage_mode=image_objects`，仅用于观测，不交给 `S3IOManager` 解释。
- `ocr_result_s3_key` 是单图 OCR 结果的事实位置；下游读取依赖 PostgreSQL 成功清单，而不是扫描固定 asset 目录。

如果后续新增 compaction/snapshot 资产，该资产可以重新使用 `S3IOManager` 写一个 latest snapshot 或少量 bucket 分区。

### Asset 配置

建议为两个 asset 增加 Dagster config，便于小批量验证和故障重跑：

```text
jiuyan__industry_images:
  limit: int | null
  force_download: bool = false
  image_filenames: list[str] = []

jiuyan__industry_ocr:
  limit: int | null
  force_ocr: bool = false
  image_filenames: list[str] = []
  max_concurrent_requests: int = 6
```

规则：

- `limit` 只限制本次处理数量，不改变状态表中的全量图片清单。
- `image_filenames` 非空时，只处理指定图片，便于单图调试。
- `force_download=true` 时允许重新下载已成功图片并覆盖 S3 图片对象。
- `force_ocr=true` 时允许重新 OCR 已成功图片并覆盖单图 OCR Parquet。
- 默认生产运行不传 `force_*`。

### PostgreSQL 事务边界

图片下载和 OCR 都按图片维度更新状态：

- 解析阶段只做 upsert，不把图片标记为下载成功。
- 图片写入 S3 成功后，再在同一图片状态行中更新 `download_status='success'`。
- OCR 领取使用 `select ... for update skip locked` 或等价原子更新，避免并发 run 重复领取。
- OCR Parquet 写入成功后，再更新 `ocr_status='success'` 和 `ocr_result_s3_key`。
- 如果 asset run 中途失败，已成功图片保持 success，未完成图片保留 running；后续运行按超时策略把 stale running 重置为 failed 或重新领取。

### 单图 OCR Parquet 写入

每张图片写一个固定对象：

```text
raw/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

写入要求：

- 写入前构造固定 PyArrow schema，保证空结果也能写 0 行表。
- 0 行 OCR 结果仍写 Parquet，并把 `ocr_result_row_count=0`。
- 同一张图片重跑成功时，覆盖该图片的 `000000_0.parquet`。
- 写入对象 key 记录到 `jiuyan_industry_images.ocr_result_s3_key`。
- 不保存 OCR 原始响应、prompt 全文或图片 base64。

### 失败率策略

第一版允许单图失败继续处理其它图片，但需要 asset 级失败保护：

- 如果本次领取图片数大于 0 且全部 OCR 失败，asset 失败。
- 如果 OCR 失败率超过 `20%`，asset 失败，但已写入的成功图片状态保留。
- 图片下载阶段同理：如果解析出图片 URL 但全部下载失败，asset 失败。
- 单图失败必须写回 `*_error_type` 和 `*_error_message`，metadata 只记录聚合统计和少量示例错误，避免日志过大。

## 第一阶段：状态与数据模型

### PostgreSQL 状态表

新增 `jiuyan_industry_images` 表，记录每张图片的下载与 OCR 状态。

建议字段：

- `image_filename`
- `image_url`
- `image_s3_key`
- `download_status`
- `download_error_type`
- `download_error_message`
- `download_sha256`
- `download_bytes`
- `ocr_status`
- `ocr_error_type`
- `ocr_error_message`
- `ocr_result_s3_key`
- `ocr_model`
- `ocr_started_at`
- `ocr_completed_at`
- `created_at`
- `updated_at`

建议首版 migration 包含：

```sql
create table if not exists jiuyan_industry_images (
    image_filename text primary key,
    image_url text not null,
    image_s3_key text,
    industry_id text,
    image_index integer,
    download_status text not null default 'pending',
    download_error_type text,
    download_error_message text,
    download_sha256 text,
    download_bytes bigint,
    downloaded_at timestamptz,
    ocr_status text not null default 'pending',
    ocr_error_type text,
    ocr_error_message text,
    ocr_result_s3_key text,
    ocr_result_row_count integer,
    ocr_model text,
    ocr_started_at timestamptz,
    ocr_completed_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists idx_jiuyan_industry_images_download_status
    on jiuyan_industry_images (download_status);

create index if not exists idx_jiuyan_industry_images_ocr_status
    on jiuyan_industry_images (ocr_status)
    where download_status = 'success';
```

首版不单独建引用表。`industry_id` 和 `image_index` 保存首次发现来源；如果未来确实需要一图多文完整追踪，再增加 `jiuyan_industry_image_refs`。

状态枚举：

```text
download_status: pending | success | failed
ocr_status: pending | running | success | failed
```

幂等规则：

- 按 `image_filename` upsert。
- 已下载成功的不重复下载。
- OCR 成功后记录 `ocr_result_s3_key`。
- 领取 OCR 任务时用行级锁或等价原子更新，避免并发重复处理。

### 数据库访问层

`postgres.py` 建议只暴露面向业务动作的函数，不在 asset 中拼 SQL：

```text
upsert_discovered_images(images)
mark_download_success(image_filename, image_s3_key, sha256, bytes)
mark_download_failed(image_filename, error_type, error_message)
claim_ocr_images(limit, image_filenames, stale_after_seconds)
mark_ocr_success(image_filename, result_s3_key, row_count, model)
mark_ocr_failed(image_filename, error_type, error_message)
list_successful_ocr_results(limit, image_filenames)
```

领取 OCR 的 SQL 语义：

```text
在事务中选择 download_status='success'
且 ocr_status in ('pending', 'failed')
或 running 已超过 stale_after_seconds 的图片；
按 image_filename 稳定排序；
for update skip locked；
更新为 running 并返回领取行。
```

### S3 路径

图片下载对象：

```text
img/jiuyan__industry_images/<image_filename>
```

OCR 结果对象：

```text
raw/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

## 第二阶段：图片下载 asset

### 输入处理

- 从 `raw/jiuyan__industry_list` 的 `imgs` 字段读取 JSON 字符串。
- 使用 `read_parquet_table_from_s3(S3Config.from_env(), dg.AssetKey("jiuyan__industry_list"), storage_mode="latest_snapshot")` 读取上游结果。
- 生产路径以现有 S3 IO manager 为准：`raw/jiuyan__industry_list/000000_0.parquet`。
- 空值、非法 JSON 直接跳过。
- 对数组元素用正则提取图片 URL，不按逗号分割。
- 同一 `industry_id` 内去重。
- 全局按 `image_filename` 去重下载；如果同一文件名对应不同 URL 或不同 SHA256，记录冲突并失败。

`image_urls.py` 需要覆盖：

```text
parse_image_urls(imgs: object) -> list[str]
normalize_image_url(url: str) -> str
image_filename_from_url(url: str) -> str
image_s3_key(image_filename: str) -> str
```

URL 提取边界：

- 支持 `.png`、`.jpg`、`.jpeg`。
- URL query string 不进入 `image_filename`。
- 不使用 `split(",")`。
- JSON 解析失败时，可以将原始字符串作为候选文本继续正则提取，但 metadata 需要记录 `imgs_parse_error_count`。

### 下载逻辑

- 使用 HTTP GET 下载图片。
- 设置合理的 User-Agent 和超时。
- 5xx 和 429 重试，4xx 不重试。
- 校验 `Content-Type` 为 `image/*`。
- 记录 SHA256 和字节数。
- 下载成功后再更新 PostgreSQL 状态。

建议复用 `aiohttp`，并限制图片下载并发，默认不超过 10。下载和 S3 写入都按图片级 try/except 捕获，单图失败不阻断其它图片。

### 输出

图片 asset 只负责把图片变成稳定可读的 S3 对象，并在 PostgreSQL 中建立图片级状态记录。

`jiuyan__industry_images` materialization metadata 至少包含：

```text
source_asset
source_path
article_count
article_with_imgs_count
parsed_image_url_count
unique_image_filename_count
postgres_upsert_count
download_request_count
download_success_count
download_skip_existing_count
download_failure_count
image_s3_write_count
image_duplicate_count
imgs_parse_error_count
asset_function_seconds
s3_bucket
s3_keys_sample
```

## 第三阶段：OCR asset

### 领取逻辑

- 只处理 `download_status='success'` 且 `ocr_status in ('pending', 'failed')` 的图片。
- 领取时原子更新 `ocr_status='running'`。
- 已成功图片默认跳过，不重复调用 OCR。
- 如果 `image_filenames` config 非空，只在指定图片集合内领取。
- 如果 `force_ocr=true`，先把指定图片或本次候选图片的 `ocr_status` 重置为 `pending`，再领取。
- `running` 超时图片可以重新领取；超时时间建议通过 `JIUYAN_OCR_STALE_RUNNING_SECONDS` 配置，默认 3600 秒。

### OCR 调用

- 输入从 S3 读取，不直接请求远端 OSS。
- 转成 `data:image/*;base64,...` 后提交给 OCR 服务。
- 并发数固定为 6。
- 单图失败按图片级记录，不因个别失败直接中断全批。

`ocr_client.py` 请求构造：

```text
POST {JIUYAN_OCR_BASE_URL}/v1/chat/completions
model={JIUYAN_OCR_MODEL_NAME}
messages=[system/user with image data URL]
response_format.type=json_schema
response_format.json_schema.strict=true
temperature=0.2
top_p=0.8
max_tokens=8192
```

响应解析：

- 只读取 `choices[0].message.content`。
- content 必须是 JSON array 字符串。
- 如果返回 Markdown、解释文字、object 或空字符串，记录 `ocr_response_parse_error`。
- 如果 JSON array 元素 schema 不合法，记录 `ocr_schema_error`。

### `jiuyan__industry_ocr` 执行流程

`jiuyan__industry_ocr` 每次 materialization 处理一批图片。批量大小由 Dagster asset config 的 `limit` 控制，OCR 服务并发度由 `max_concurrent_requests` 控制；二者含义不同：

- `limit`：本次最多从 PostgreSQL 领取多少张待 OCR 图片。小批联调建议传 `limit=5`，扩大验证可传 `limit=20` 或 `limit=60`；如果 `limit=null`，则领取所有符合条件的待处理图片，第一版不建议在未验证稳定前直接全量使用。
- `max_concurrent_requests`：本次同时调用 OCR 服务的最大请求数，第一版默认且生产固定为 `6`。因此即使 `limit=60`，也只是领取 60 张图片，然后按最多 6 个 in-flight OCR 请求滚动执行。

单次运行流程：

1. 读取 Dagster config：`limit`、`force_ocr`、`image_filenames`、`max_concurrent_requests`。
2. 如果 `force_ocr=true`，先把指定图片或本次候选图片的 `ocr_status` 重置为 `pending`，但只允许处理 `download_status='success'` 的图片。
3. 在 PostgreSQL 事务中领取图片：按 `image_filename` 稳定排序，筛选 `download_status='success'` 且 `ocr_status in ('pending', 'failed')`，并包含 stale `running`；使用 `for update skip locked` 或等价原子更新把本批图片标记为 `running`，返回不超过 `limit` 张图片。
4. 如果没有领取到图片，asset 成功结束，并在 metadata 中记录 `claimed_image_count=0`。
5. 为本批图片创建异步 OCR 任务，并用 `asyncio.Semaphore(max_concurrent_requests)` 限制并发；所有任务共享一个 HTTP client/session，单图任务内部自行处理超时、重试、schema 校验和错误捕获。
6. 每个单图任务从 S3 读取 `image_s3_key` 对应图片 bytes，规范化 MIME，构造 `data:image/*;base64,...`，调用 `{JIUYAN_OCR_BASE_URL}/v1/chat/completions`。
7. OCR 响应成功后，只读取 `choices[0].message.content`，按 `ocr_schema.py` 校验和规范化为固定字段列表；完全空行丢弃，同图内完全重复行去重。
8. 将规范化后的行加上 `industry_id`，构造固定 `JIUYAN_INDUSTRY_OCR_SCHEMA` 的 PyArrow Table；即使 OCR 结果为空数组，也写 0 行 Parquet。
9. 单图 Parquet 写入成功后，再更新 PostgreSQL：`ocr_status='success'`、`ocr_result_s3_key`、`ocr_result_row_count`、`ocr_model`、`ocr_completed_at`。
10. 单图任一步失败时，捕获错误并写回 PostgreSQL：`ocr_status='failed'`、`ocr_error_type`、`ocr_error_message`；不取消其它图片任务。
11. 全部任务结束后汇总 metadata。若本次领取数大于 0 且全部失败，或失败率超过 `20%`，asset 级别失败；已成功写入的单图结果和 PostgreSQL success 状态保留。

并发执行伪代码：

```python
claimed_images = claim_ocr_images(
    limit=config.limit,
    image_filenames=config.image_filenames,
    stale_after_seconds=settings.stale_running_seconds,
)

semaphore = asyncio.Semaphore(config.max_concurrent_requests)

async with aiohttp.ClientSession(timeout=...) as session:
    async def process_one(image: ClaimedImage) -> OcrImageResult:
        async with semaphore:
            try:
                image_bytes = read_image_bytes_from_s3(image.image_s3_key)
                rows = await ocr_client.extract_rows(session, image_bytes)
                normalized_rows = validate_and_normalize_ocr_rows(rows)
                result_s3_key = write_single_image_ocr_parquet(
                    image=image,
                    rows=normalized_rows,
                )
                mark_ocr_success(
                    image.image_filename,
                    result_s3_key,
                    row_count=len(normalized_rows),
                    model=settings.model_name,
                )
                return OcrImageResult.success(...)
            except Exception as exc:
                mark_ocr_failed(image.image_filename, type(exc).__name__, str(exc))
                return OcrImageResult.failed(...)

    results = await asyncio.gather(
        *(process_one(image) for image in claimed_images),
        return_exceptions=False,
    )
```

Parquet 写入不经过现有 `S3IOManager`，而是在单图任务内部显式写入对应对象 key：

```text
raw/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

写入步骤：

1. 对每条 OCR 结果补充 `industry_id`，并按 `industry_id, stock_name, theme_path, relation, source` 顺序生成 rows。
2. 使用 `JIUYAN_INDUSTRY_OCR_SCHEMA` 构造 `pa.Table`，所有字符串字段非 null，缺失值写空字符串。
3. 使用 PyArrow Parquet writer 写到 S3 filesystem 的目标 key；同一图片重跑成功时覆盖该图片自己的 `000000_0.parquet`。
4. S3 写入成功后才调用 `mark_ocr_success(...)`。如果 Parquet 写入失败，该图片保持 `failed`，不会产生“数据库 success 但对象不存在”的状态。
5. 下游全量读取时不扫描固定目录，而是从 PostgreSQL 查询 `ocr_status='success'` 的 `ocr_result_s3_key` 清单，再读取这些单图 Parquet。

### Prompt 规则

Prompt 保持短、直接、扁平：

- 只抽取图片中可见文字。
- 不补充常识，不做归一化，不做实体消歧。
- 每条记录对应一个“个股 × 题材路径”关联。
- 题材多级路径用英文逗号 `,` 连接。
- 合并单元格主题继承到每一行。
- 风险提示、免责声明、水印、二维码等不抽取。
- 看不清或不存在的字段填空字符串，不猜测。

### 输出校验

只接受如下 JSON array：

```json
[
  {
    "stock_name": "示例股份",
    "theme_path": "一级题材,二级题材",
    "relation": "与该题材相关的原文说明",
    "source": "资料来源"
  }
]
```

校验点：

- 顶层必须是数组。
- 每个元素必须是 object。
- 缺失字段填空字符串，不允许扩展字段污染输出。
- 四个字段都转成字符串并 strip。
- `theme_path` 不再输出数组，第一版统一为英文逗号 `,` 连接后的字符串。
- 完全空行（四个字段均为空）丢弃。
- 同图内按 `stock_name + theme_path + relation + source` 去重，保留首次出现顺序。

### 写入

- OCR 成功后写入对应单图 Parquet。
- 更新 PostgreSQL 中 `ocr_status='success'` 和 `ocr_result_s3_key`。
- 失败则写回错误类型和错误信息，供后续重试。

`jiuyan__industry_ocr` 单图 Parquet 只保存业务字段和必要业务关联键。`industry_id` 用于关联回 `jiuyan__industry_list`，必须保留；图片路径、S3 key、运行状态等技术字段不进入 Parquet。字段顺序必须稳定，0 行 OCR 结果也使用同一 schema 写空表。

| Column | PyArrow type | Nullable | 来源 | 说明 |
| --- | --- | --- | --- | --- |
| `industry_id` | `pa.string()` | no | `jiuyan_industry_images.industry_id` | 韭研产业研究文章 ID，用于关联回 `jiuyan__industry_list`。 |
| `stock_name` | `pa.string()` | no | OCR 输出 | 个股、公司或标的名称；缺失时为空字符串。 |
| `theme_path` | `pa.string()` | no | OCR 输出 | 多级题材路径，用英文逗号 `,` 连接；缺失时为空字符串。 |
| `relation` | `pa.string()` | no | OCR 输出 | 个股与题材关系的原文说明；缺失时为空字符串。 |
| `source` | `pa.string()` | no | OCR 输出 | 来源、信源或资料来源；缺失时为空字符串。 |

PyArrow schema 建议集中定义在 `schemas.py`：

```python
JIUYAN_INDUSTRY_OCR_SCHEMA = pa.schema(
    [
        pa.field("industry_id", pa.string(), nullable=False),
        pa.field("stock_name", pa.string(), nullable=False),
        pa.field("theme_path", pa.string(), nullable=False),
        pa.field("relation", pa.string(), nullable=False),
        pa.field("source", pa.string(), nullable=False),
    ]
)
```

规范化规则：

- 所有字符串字段在写入前执行 `strip()`；缺失值统一写空字符串，不写 null。
- `industry_id` 必须非空；如果缺失，视为状态数据错误，该图片 OCR 结果不写入成功。
- 完全空行（`stock_name`、`theme_path`、`relation`、`source` 四个字段均为空）不写入 Parquet。
- 同图内按 `stock_name + theme_path + relation + source` 去重，保留首次出现顺序。
- `image_index`、`image_filename`、`image_url`、`image_s3_key`、`ocr_result_s3_key` 等技术溯源字段只保存在 PostgreSQL 状态表、S3 对象路径或 Dagster metadata 中，不进入业务 Parquet。
- 第一版不写 `ocr_model`、`ocr_completed_at`、`run_id`、错误信息、prompt 或 OCR 原始 JSON；这些只存在于 PostgreSQL 状态表、Dagster metadata 或日志。

写入 metadata：

```text
claimed_image_count
ocr_request_count
ocr_success_count
ocr_empty_count
ocr_failure_count
ocr_result_row_count
ocr_skip_success_count
ocr_model
ocr_base_url_host
max_concurrent_requests
result_s3_keys_sample
asset_function_seconds
ocr_request_seconds
table_convert_seconds
```

## 第四阶段：Dagster 集成

### Assets

建议至少包含：

- `jiuyan__industry_images`
- `jiuyan__industry_ocr`

定义建议：

```text
group_name="jiuyan_industry_ocr"
tags={
  "source": "jiuyan",
  "layer": "raw",
  "storage": "s3",
  "state": "postgres",
}
```

依赖关系：

- `jiuyan__industry_images` 依赖 `jiuyan__industry_list`。
- `jiuyan__industry_ocr` 依赖 `jiuyan__industry_images`。

Jobs：

- `jiuyan__industry_images_job`：只下载/同步图片。
- `jiuyan__industry_ocr_job`：只处理待 OCR 图片。
- `jiuyan__industry_ocr_full_job`：可选，串起图片下载和 OCR。

第一版不加 schedule，先通过手动 materialize 或 `dg launch` 做小批量验证；稳定后再决定是否接入每日调度。

### Metadata

materialization metadata 建议包含：

- 文章数
- 含图文章数
- 解析 URL 数
- 成功下载数
- 成功 OCR 数
- 失败数
- 空结果数
- OCR model
- base URL host

不要记录认证信息、密钥或图片 base64。

### 注册点

需要在 `pipeline/scheduler/src/scheduler/defs/pipeline_defs.py` 中：

- import 新增 assets。
- 将 assets 加入 `assets=[...]`。
- 将手动 jobs 加入 `jobs=[...]`。
- 暂不新增 sensor；OCR 领取逻辑由 PostgreSQL 状态表控制。

## 第五阶段：测试与联调

### 单元测试

覆盖：

- `imgs` 解析。
- 多 URL 提取。
- 重复 URL 去重。
- URL query 参数包含逗号时不会误切。
- `image_filename` 和 S3 key 生成。
- 下载成功/失败状态更新。
- OCR 任务领取只领取 pending/failed，跳过 success。
- stale running 可以重新领取。
- OCR 输出 schema 校验。
- OCR 空数组写 0 行表。
- 重复个股在多个题材下保留多条记录。

建议测试命令：

```bash
cd pipeline
uv run pytest scheduler/tests/test_jiuyan_industry_ocr_image_urls.py
uv run pytest scheduler/tests/test_jiuyan_industry_ocr_schema.py
uv run pytest scheduler/tests/test_jiuyan_industry_ocr_assets.py
uv run dg check defs
```

### 样例联调

使用 `docs/references/jiuyan_images/` 下样例图片验证：

- 能抽出个股名称。
- 能抽出多级题材路径。
- 能抽出个股与题材的关联说明。
- 能抽出来源字段。
- 能忽略表格外的风险提示和装饰文字。

联调步骤：

1. `uv run alembic -c migrate/alembic.ini upgrade head` 创建状态表。
2. 使用真实 `jiuyan__industry_list` snapshot 小批量运行 `jiuyan__industry_images`，限制 `limit=20`。
3. 确认 S3 图片对象和 PostgreSQL 下载状态。
4. 使用样例图片或刚下载图片小批量运行 `jiuyan__industry_ocr`，限制 `limit=5`。
5. 检查单图 Parquet、`ocr_result_s3_key`、空结果和失败状态。
6. 逐步扩大到全量约 1200 张图片。

## 验收标准

1. **资产可见**：Dagster UI 中能看到 `jiuyan__industry_images` 和 `jiuyan__industry_ocr`。
2. **图片下载**：能从 `imgs` 解析 URL → 下载 → 写入 S3 → 记录 PostgreSQL 状态。
3. **OCR 抽取**：能从 PostgreSQL 领取图片 → 从 S3 读取 → 调用 OCR → schema 校验 → 写入单图 Parquet。
4. **幂等与重试**：失败项下次运行可重试，成功项默认跳过。
5. **数据完整性**：输出行保留 `industry_id`，可直接关联 `jiuyan__industry_list`；图片级溯源通过 PostgreSQL 状态表和 `ocr_result_s3_key` 完成。
6. **多题材保留**：同一个 `stock_name` 在多个 `theme_path` 下的关联记录不被错误去重。
7. **可观测性**：下载失败、OCR 失败、空结果可在 metadata 中观测。

## 待确认

- 是否需要后续增加独立的汇总/压缩资产，用于减少下游小文件读取成本。
- 单图结果 Parquet 的列顺序是否需要与现有 raw 资产保持一致风格。
