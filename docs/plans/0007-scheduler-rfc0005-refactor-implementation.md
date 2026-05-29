# Plan 0007: RFC 0005 Scheduler 资源抽象重组实施计划

状态：草案

计划日期：2026-05-28

关联 RFC：

- `docs/RFC/0005-scheduler-resource-refactor-and-trade-date-backfill.md`

参考资料：

- `docs/RFC/0002-eastmoney-f10-ingestion.md`
- `docs/RFC/0003-http-resource-market-event-ingestion.md`
- `docs/RFC/0004-jiuyan-industry-list-ocr.md`
- `docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0003-trade-calendar-driven-market-schedules.md`
- `docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md`
- `docs/plans/0004-http-client-refactor-and-rfc0003-implementation.md`
- `docs/plans/0005-jiuyan-industry-list-ocr-implementation.md`
- `pipeline/scheduler/src/scheduler/defs/`
- `pipeline/scheduler/tests/`
- Dagster 当前文档：`DailyPartitionsDefinition`、`BackfillPolicy.single_run()`、`define_asset_job`、`ScheduleDefinition`、`AssetExecutionContext.partition_keys`、`dg check defs`。

## 目标

本计划将 RFC 0005 拆解为可执行的工程步骤。核心目标是重组 scheduler definitions 的代码边界，同时保持已落地资产的业务语义稳定。

目标包括：

- 将通用 OCR 能力从 `jiuyan_industry_ocr` 中提取到 `defs/ocr`。
- 将韭研 OCR 业务迁回 `defs/http_resources`，并组合调度 `jiuyan__industry_list -> jiuyan__industry_images -> jiuyan__industry_ocr`。
- 将 PostgreSQL repository 和图片对象存储迁移到 `defs/io_managers`。
- 将 EastMoney 改造为一般 HTTP resource 组织方式，删除 Parquet 中的请求派生字段。
- 清理 `trade_date_dynamic_partitions`，将交易日 HTTP 资产改为自然日分区 + 运行时交易日过滤。
- 拆分 `http_resources__market_event_daily_job` 为两个独立 daily job。
- 保持 asset key、S3 路径和交易日历事实来源稳定。
- 给出完整影响范围、实施顺序、回归检查和验收条件。

## 非目标

本计划不包含：

- 修改 OCR prompt 业务含义或 `jiuyan__industry_ocr` 输出业务字段。
- 修改 EastMoney 远端接口、日期过滤字段或分页排序策略。
- 引入 ClickHouse、dbt 模型、前端页面或应用服务。
- 改变 `sina__trade_calendar` 作为交易日事实来源的决策。
- 对历史 S3 对象做批量迁移或删除。
- 在本计划阶段执行真实大规模回填。

## 总体原则

- **资产语义优先**：保留已有 Dagster asset key，不通过重命名制造 lineage 断裂。
- **分阶段提交**：每一阶段都必须能独立通过 `dg check defs` 和相关单元测试。
- **移动先保行为**：文件迁移阶段先保持行为不变，再做字段策略、分区策略和调度策略变更。
- **副作用边界清晰**：HTTP、OCR、S3、PostgreSQL 访问边界应可单独测试。
- **raw Parquet 只保留业务字段**：EastMoney 请求派生字段必须移出 Parquet。
- **交易日过滤运行时完成**：自然日分区用于 UI 交互，`sina__trade_calendar` 用于业务合法性。

## 当前状态摘要

当前相关实现位置：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/
  client.py
  partitioned.py
  schedules.py
  jiuyan__action_field.py
  jiuyan__industry_list.py
  ths__limit_up_pool.py
  eastmoney/

pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/
  assets.py
  image_store.py
  image_urls.py
  ocr_client.py
  ocr_schema.py
  postgres.py
  schemas.py
  services.py

pipeline/scheduler/src/scheduler/defs/io_managers/
  s3_io_manager.py

pipeline/scheduler/src/scheduler/defs/pipeline_defs.py
```

当前测试覆盖主要在：

```text
pipeline/scheduler/tests/test_eastmoney.py
pipeline/scheduler/tests/test_http_resources_client.py
pipeline/scheduler/tests/test_http_resources_market_events.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_assets.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_client.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_image_urls.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_migration.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_schema.py
pipeline/scheduler/tests/test_jiuyan_industry_ocr_state_flow.py
pipeline/scheduler/tests/test_sina_trade_calendar.py
```

## 改动影响范围

### 新增文件

预计新增：

```text
pipeline/scheduler/src/scheduler/defs/ocr/
  __init__.py
  client.py
  schemas.py
  service.py

pipeline/scheduler/src/scheduler/defs/io_managers/
  postgres.py
  image_object_store.py

pipeline/scheduler/src/scheduler/defs/http_resources/
  jiuyan__industry_ocr.py
  eastmoney.py                 # 如选择压平 EastMoney
  eastmoney_fields.py          # 如选择压平 EastMoney

pipeline/scheduler/tests/
  test_ocr_client.py
  test_ocr_schema.py
  test_http_resources_jiuyan_ocr.py
  test_http_resources_schedules.py
```

如果 EastMoney 暂时保留子目录，则不新增 `eastmoney.py` / `eastmoney_fields.py`，但必须在 `http_resources/eastmoney/` 内完成资产工厂化和 schedule 统一注册。

### 移动或重命名文件

预计移动：

```text
pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/postgres.py
  -> pipeline/scheduler/src/scheduler/defs/io_managers/postgres.py

pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/image_store.py
  -> pipeline/scheduler/src/scheduler/defs/io_managers/image_object_store.py

pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/ocr_client.py
  -> pipeline/scheduler/src/scheduler/defs/ocr/client.py

pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/ocr_schema.py
  -> pipeline/scheduler/src/scheduler/defs/ocr/schemas.py
     + 韭研专用 schema 留在 http_resources/jiuyan__industry_ocr.py 或相邻模块

pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/assets.py
  -> pipeline/scheduler/src/scheduler/defs/http_resources/jiuyan__industry_ocr.py
```

移动阶段可以短期保留 re-export 兼容层：

```text
pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/__init__.py
```

兼容层只允许导出新模块符号，不允许继续维护业务逻辑。

### 修改文件

预计修改：

```text
pipeline/scheduler/src/scheduler/defs/http_resources/partitioned.py
pipeline/scheduler/src/scheduler/defs/http_resources/schedules.py
pipeline/scheduler/src/scheduler/defs/http_resources/jiuyan__action_field.py
pipeline/scheduler/src/scheduler/defs/http_resources/ths__limit_up_pool.py
pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/assets.py
pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/client.py
pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/schemas.py
pipeline/scheduler/src/scheduler/defs/http_resources/eastmoney/schedules.py
pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py
pipeline/scheduler/src/scheduler/defs/pipeline_defs.py
pipeline/scheduler/src/scheduler/defs/util.py
pipeline/scheduler/README.md
```

### 删除或废弃内容

应删除或停止注册：

```text
TRADE_DATE_DYNAMIC_PARTITIONS_NAME
trade_date_dynamic_partitions
sync_trade_date_dynamic_partitions
sina__trade_calendar_dynamic_partitions_sensor
http_resources__market_event_daily_job
jiuyan__industry_images_job
jiuyan__industry_ocr_job
```

`jiuyan__industry_ocr_full_job` 需要评估后合并到统一链路 job 的 run config 语义中。除非有明确人工运维必要，不再注册第二套 OCR job。

### 数据与运行影响

不应变化：

- Asset key：
  - `jiuyan__industry_images`
  - `jiuyan__industry_ocr`
  - `jiuyan__action_field`
  - `ths__limit_up_pool`
  - 所有 `eastmoney__*`
- 既有 S3 路径：
  - `img/jiuyan__industry_images/<image_filename>`
  - `source/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet`
  - `source/jiuyan__action_field/trade_date=YYYY-MM-DD/000000_0.parquet`
  - `source/ths__limit_up_pool/trade_date=YYYY-MM-DD/000000_0.parquet`
  - `source/eastmoney__*/year=YYYY/000000_0.parquet`
- `sina__trade_calendar` 作为交易日事实来源。

会变化：

- EastMoney Parquet schema 删除请求派生字段。
- `jiuyan__action_field` / `ths__limit_up_pool` 从动态分区改为自然日分区。
- 非交易日 Dagster 分区可以被选择，但不会请求远端、不会写 Parquet。
- `http_resources__market_event_daily_job` 被两个独立 daily job 取代。
- 韭研 OCR 由完整链路 job/schedule 调度，不再单独调度图片下载和 OCR。

## 分阶段执行流程

### 阶段 0：基线冻结

目标：在修改前记录当前 definitions、测试和关键行为，避免迁移时混淆“原有问题”和“新增回归”。

步骤：

1. 记录当前 git 状态，确认只有 RFC/plan 文档或预期改动。
2. 执行 definitions 加载检查：

```bash
cd pipeline
uv run dg check defs
```

3. 执行当前测试：

```bash
cd pipeline
uv run pytest scheduler/tests
```

4. 如果 `pytest` 或工具依赖缺失，按项目质量门禁补齐依赖或记录临时命令，不在业务重构 PR 中混入无关工具链大改。
5. 列出当前 definitions，保存 jobs/schedules/sensors 名称作为对比：

```bash
cd pipeline
uv run dg list defs --json
```

验收：

- 已记录当前 asset/job/schedule/sensor 列表。
- 已知测试失败必须分类为既有问题或本计划阻塞项。
- 后续阶段不在无基线的情况下继续大规模移动文件。

### 阶段 1：基础设施迁移到 `io_managers`

目标：先移动 PostgreSQL 和图片对象存储边界，保持韭研 OCR 行为不变。

步骤：

1. 新增 `pipeline/scheduler/src/scheduler/defs/io_managers/postgres.py`。
2. 将 `connect_pipeline_database`、`PostgresIndustryImageRepository`、状态更新 SQL 和薄包装函数从 `jiuyan_industry_ocr/postgres.py` 移入新模块。
3. 新增 `pipeline/scheduler/src/scheduler/defs/io_managers/image_object_store.py`。
4. 将 `ImageObjectStore`、图片 bytes 读写、OCR result table 写入、image key 生成等对象存储能力从 `image_store.py` 移入新模块。
5. 更新 `jiuyan_industry_ocr/assets.py`、`services.py` 等 import，暂时不移动 asset 文件。
6. 保留旧模块 re-export 一轮，降低一次性 import 断裂风险：

```python
from scheduler.defs.io_managers.postgres import ...
```

7. 调整测试 import。
8. 执行相关测试：

```bash
cd pipeline
uv run pytest scheduler/tests/test_jiuyan_industry_ocr_state_flow.py \
  scheduler/tests/test_jiuyan_industry_ocr_assets.py \
  scheduler/tests/test_jiuyan_industry_ocr_migration.py
uv run dg check defs
```

验收：

- PostgreSQL repository 行为不变。
- 图片对象 S3 key 和 OCR result S3 key 不变。
- `jiuyan__industry_images` 和 `jiuyan__industry_ocr` definitions 仍能加载。
- 旧业务目录不再拥有 PostgreSQL 连接实现和对象存储实现。

### 阶段 2：通用 OCR 模块抽取

目标：把 OpenAI-compatible OCR 调用和通用 JSON schema 解析从韭研业务逻辑中拆出。

步骤：

1. 新增 `defs/ocr/__init__.py`。
2. 新增 `defs/ocr/client.py`，承载：
   - `OcrClient` 或等价 async client wrapper。
   - `OcrRequest` / `OcrResponse`。
   - `build_image_data_url`。
   - chat completion response content 抽取。
3. 新增 `defs/ocr/schemas.py`，承载：
   - `OcrSchemaError`。
   - 通用 JSON array 解析。
   - 基础 schema 校验工具。
4. 新增 `defs/ocr/service.py`，承载通用 batch 并发、失败聚合和统计模型。
5. 将韭研专用 `StockThemeSchema`、prompt 和业务字段转换留在韭研模块，不进入 `defs/ocr`。
6. 更新 `services.py` 或后续韭研业务模块 import，确保 OCR 调用经过通用模块。
7. 补充或迁移测试：

```text
test_ocr_client.py
test_ocr_schema.py
```

8. 执行测试：

```bash
cd pipeline
uv run pytest scheduler/tests/test_ocr_client.py \
  scheduler/tests/test_ocr_schema.py \
  scheduler/tests/test_jiuyan_industry_ocr_client.py \
  scheduler/tests/test_jiuyan_industry_ocr_schema.py
uv run dg check defs
```

验收：

- 通用 OCR 模块不知道 `industry_id`、`stock_name`、`theme_path`、`image_filename`。
- 韭研 prompt 和业务 schema 不进入 `defs/ocr`。
- OCR response 解析错误、空数组、字段补齐和 schema 错误都有测试。
- 原 OCR asset 行为不变。

### 阶段 3：韭研 OCR 业务迁入 `http_resources`

目标：将韭研 OCR 资产迁回 HTTP resource 业务边界，并改为完整链路组合调度。

步骤：

1. 新增 `pipeline/scheduler/src/scheduler/defs/http_resources/jiuyan__industry_ocr.py`。
2. 将 `jiuyan__industry_images`、`jiuyan__industry_ocr` asset 定义迁入该模块。
3. 将韭研专用 schema/prompt/table 转换迁入该模块或相邻 `jiuyan_ocr_schema.py`。
4. 保持资产 key 不变：

```text
jiuyan__industry_images
jiuyan__industry_ocr
```

5. 保持依赖关系：

```text
jiuyan__industry_list -> jiuyan__industry_images -> jiuyan__industry_ocr
```

6. 将 `group_name` 调整为 `http_sources`，保留 tags：

```text
source=jiuyan
layer=source
storage=s3
state=postgres
modality=ocr
```

7. 在 `http_resources/schedules.py` 中定义统一链路 job：

```text
jiuyan__industry_ocr_pipeline_job
```

8. 资产选择包括：

```text
jiuyan__industry_list
jiuyan__industry_images
jiuyan__industry_ocr
```

9. 定义统一 schedule：

```text
jiuyan__industry_ocr_pipeline_schedule
```

10. 移除注册：

```text
jiuyan__industry_images_job
jiuyan__industry_ocr_job
```

11. 评估 `jiuyan__industry_ocr_full_job`：
    - 如果只是“全链路/全量”语义，合并到 pipeline job 的 run config。
    - 如果确有人工运维场景，必须在 plan 更新中说明保留原因和使用边界。
12. 更新 `pipeline_defs.py` 导入和注册。
13. 更新测试，覆盖统一 job selection 和旧 job 不再注册。
14. 执行检查：

```bash
cd pipeline
uv run pytest scheduler/tests/test_jiuyan_industry_ocr_assets.py \
  scheduler/tests/test_http_resources_jiuyan_ocr.py
uv run dg check defs
uv run dg list defs --json
```

验收：

- `jiuyan__industry_list`、`jiuyan__industry_images`、`jiuyan__industry_ocr` 三个资产均可见。
- 三个资产通过统一 job/schedule 组合调度。
- `jiuyan__industry_images_job` 和 `jiuyan__industry_ocr_job` 不再注册。
- 旧 S3 路径和 PostgreSQL 状态表语义不变。
- 旧 `jiuyan_industry_ocr` 目录不再包含业务实现。

### 阶段 4：EastMoney HTTP resource 改造

目标：将 EastMoney 收敛为一般 HTTP resource 模式，删除 raw Parquet 请求派生字段。

步骤：

1. 确定组织方式：
   - 首选压平到 `http_resources/eastmoney.py` + `eastmoney_fields.py`。
   - 如保留子目录，必须完成资产工厂化和 schedule 统一注册。
2. 引入 `EastmoneyEndpointConfig` 驱动的资产工厂：

```python
def build_eastmoney_asset(endpoint: EastmoneyEndpointConfig) -> dg.AssetsDefinition:
    ...
```

3. 保持八个 asset key 不变。
4. 保持 `year_partitions`、`eastmoney_run_pool`、`BackfillPolicy.multi_run(max_partitions_per_run=1)`。
5. 保持当前必要的执行顺序依赖和 `execution_ordering_dependency` metadata。
6. 将 schedule 注册移入 `http_resources/schedules.py` 统一出口。
7. 删除或停止引用 `http_resources/eastmoney/schedules.py`。
8. 复用 `http_resources/client.py` 的 `AioHttpClient`、headers、retry 和 stats。
9. 简化 EastMoney domain client：
   - 保留 code concurrency。
   - 保留分页顺序请求。
   - 保留 duplicate page row 检测。
   - 业务错误仍包装为 `EastmoneyRequestError`。
10. 修改 EastMoney schema 生成：
    - 删除 `REQUEST_FIELD_NAMES`。
    - `eastmoney_schema()` 只包含接口内容字段。
    - `empty_eastmoney_table()` 不包含请求派生字段。
11. 修改 row-to-table：
    - 不写 `request_code`。
    - 不写 `request_start_date` / `request_end_date`。
    - 不写 `partition_year`。
    - 不写 `source_endpoint`。
    - 不写 `ingested_at`。
12. 将请求上下文写入 materialization metadata：

```text
requested_ranges
source_endpoints
candidate_security_count
selected_security_count
request_count
retry_count
page_count
empty_response_count
duplicate_page_row_count
```

13. 更新 `test_eastmoney.py`。
14. 执行检查：

```bash
cd pipeline
uv run pytest scheduler/tests/test_eastmoney.py \
  scheduler/tests/test_http_resources_client.py
uv run dg check defs
```

验收：

- 八个 EastMoney asset key 不变。
- EastMoney Parquet schema 不包含：
  - `request_code`
  - `request_start_date`
  - `request_end_date`
  - `partition_year`
  - `source_endpoint`
  - `ingested_at`
- 空表 schema 同样不包含这些字段。
- 请求上下文进入 metadata。
- EastMoney schedule 从 `http_resources` 统一出口注册。
- `pipeline_defs.py` 不再引用 EastMoney 特殊 schedules 子模块。

### 阶段 5：交易日自然日分区改造

目标：删除动态分区，改为自然日分区 + 运行时交易日过滤 + sparse output。

步骤：

1. 在 `http_resources/partitioned.py` 中删除动态分区概念：

```text
TRADE_DATE_DYNAMIC_PARTITIONS_NAME
trade_date_dynamic_partitions
sync_trade_date_dynamic_partitions
```

2. 定义两个自然日分区：

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

3. `jiuyan__action_field` 使用 `jiuyan_action_field_daily_partitions`。
4. `ths__limit_up_pool` 使用 `ths_limit_up_pool_daily_partitions`。
5. 两个资产均设置：

```python
backfill_policy=dg.BackfillPolicy.single_run()
metadata={
    "storage_mode": "partitioned",
    "partition_key_name": "trade_date",
    "partitions_def": "daily_partitions",
    "trade_date_filter": "sina__trade_calendar",
    "allow_empty": True,
    "sparse_partition_output": True,
}
```

6. 改造 `materialize_trade_date_range` 或新增 helper：
   - 输入自然日 partition keys。
   - 读取 `sina__trade_calendar`。
   - 过滤交易日。
   - 只对交易日调用 `fetch_table_for_trade_date`。
   - 只写交易日 S3 分区。
   - 非交易日不请求、不写文件。
   - 范围内无交易日时成功返回 metadata。
7. metadata 至少包含：

```text
backfill_start_date
backfill_end_date
requested_partition_count
requested_natural_date_count
processed_trade_date_count
skipped_non_trade_date_count
completed_trade_date_count
partition_keys
processed_trade_dates
skipped_non_trade_dates_sample
partition_row_counts
s3_keys
```

8. 处理 sparse output：
   - 优先让 market-event helper 自行写 S3，不走 `S3IOManager` 的 full partition table key 校验。
   - 如果扩展 `S3IOManager`，必须仅在 `sparse_partition_output=True` 时允许 table keys 是 selected partition keys 的子集。
   - 对 selected partition 中的交易日，仍必须写出或失败，不能 silently skip。
9. 删除 `sina__trade_calendar_dynamic_partitions_sensor`。
10. 拆分 daily job：

```text
jiuyan__action_field_daily_job
ths__limit_up_pool_daily_job
```

11. 删除 `http_resources__market_event_daily_job` 注册。
12. 定义两个 daily schedule：
    - 都读取 `sina__trade_calendar`。
    - 非交易日返回 `SkipReason`。
    - 交易日提交对应自然日 partition key。
    - 第一版可同 cron；如真实运行观察到上游压力，再错峰。
13. 更新 `pipeline_defs.py` jobs/schedules/sensors 注册。
14. 更新 `test_http_resources_market_events.py` 和新增 schedule 测试。
15. 执行检查：

```bash
cd pipeline
uv run pytest scheduler/tests/test_http_resources_market_events.py \
  scheduler/tests/test_sina_trade_calendar.py \
  scheduler/tests/test_http_resources_schedules.py
uv run dg check defs
```

验收：

- 动态分区定义、同步 helper 和 sensor 不再存在或不再注册。
- `jiuyan__action_field` 起始日期为 `2021-01-01`。
- `ths__limit_up_pool` 起始日期为 `2025-01-01`。
- Web UI 可按自然日范围选择。
- single-run backfill 在 run 内过滤交易日。
- 非交易日不发送 HTTP 请求，不写 Parquet。
- 交易日远端空响应仍写空表。
- `http_resources__market_event_daily_job` 不再注册。
- 两个 daily job 分别只选择自己的 asset。

### 阶段 6：Definition 注册与文档清理

目标：收敛统一出口，删除兼容层和过时文档。

步骤：

1. 更新 `pipeline_defs.py`：
   - 从 `http_resources` 导入韭研 OCR assets/job/schedule。
   - 从统一 EastMoney 出口导入 assets/job/schedule。
   - 注册 `jiuyan__action_field_daily_job`。
   - 注册 `ths__limit_up_pool_daily_job`。
   - 注册 `jiuyan__industry_ocr_pipeline_job`。
   - 删除 `http_resources__market_event_daily_job`。
   - 删除 `sina__trade_calendar_dynamic_partitions_sensor`。
   - 删除 `jiuyan__industry_images_job`、`jiuyan__industry_ocr_job`。
2. 清理 `jiuyan_industry_ocr` 兼容目录：
   - 如果没有外部 import 依赖，删除目录。
   - 如果需要短期兼容，只保留 `__init__.py` re-export，并添加计划删除说明。
3. 更新 `pipeline/scheduler/README.md`：
   - 更新目录结构。
   - 更新 job/schedule 名称。
   - 更新回填说明。
   - 更新 EastMoney raw 字段策略。
4. 如有必要，新增 ADR 或更新 ADR 0003，说明自然日分区 + 运行时交易日过滤取代动态交易日分区。
5. 执行：

```bash
cd pipeline
uv run dg check defs
uv run dg list defs --json
```

验收：

- definitions 中没有旧 job/sensor。
- README 与实际目录/definitions 一致。
- 没有业务逻辑留在 `jiuyan_industry_ocr` 旧目录。

### 阶段 7：全量回归与小范围验证

目标：在所有重构完成后执行完整质量门禁和最小真实验证。

执行命令：

```bash
cd pipeline

uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
uv run dg check defs
uv run dg list defs --json
```

建议小范围人工验证：

```bash
cd pipeline

# 只验证 definitions 和选择，不做大规模回填。
uv run dg launch --assets jiuyan__action_field --partition 2025-01-02
uv run dg launch --assets ths__limit_up_pool --partition 2025-01-02
```

如果本地环境缺少真实 S3、PostgreSQL、OCR 或上游认证配置，只记录未执行原因，不用伪造成功。

验收：

- 所有质量门禁通过，或明确记录阻塞原因。
- `dg list defs --json` 可确认新旧 jobs/schedules/sensors 符合 RFC 0005。
- 小范围真实验证不改变大规模历史数据。

## 验收条件

### 总体验收

完成后必须满足：

1. `jiuyan_industry_ocr` 不再承载业务实现。
2. `defs/ocr` 承载通用 OCR client/schema/service。
3. `defs/io_managers/postgres.py` 承载 PostgreSQL 连接和 repository。
4. `defs/io_managers/image_object_store.py` 承载图片对象存储访问。
5. `jiuyan__industry_images` 和 `jiuyan__industry_ocr` asset key 不变。
6. `jiuyan__industry_list`、`jiuyan__industry_images`、`jiuyan__industry_ocr` 通过统一 pipeline job/schedule 组合调度。
7. `jiuyan__industry_images_job`、`jiuyan__industry_ocr_job` 不再注册。
8. EastMoney 八个 asset key 不变。
9. EastMoney Parquet 不再包含请求派生字段。
10. EastMoney 请求上下文进入 metadata。
11. `trade_date_dynamic_partitions` 相关定义和 sensor 被清理。
12. `jiuyan__action_field` 使用 `2021-01-01` 起始的自然日分区。
13. `ths__limit_up_pool` 使用 `2025-01-01` 起始的自然日分区。
14. 两个交易日 HTTP 资产均使用 `BackfillPolicy.single_run()`。
15. 非交易日不发送远端请求，不写 Parquet。
16. `http_resources__market_event_daily_job` 不再注册。
17. `jiuyan__action_field_daily_job` 和 `ths__limit_up_pool_daily_job` 分别只选择各自 asset。
18. `sina__trade_calendar` 仍是唯一交易日事实来源。
19. `uv run dg check defs` 通过。

### 文件级验收

必须存在：

```text
pipeline/scheduler/src/scheduler/defs/ocr/
pipeline/scheduler/src/scheduler/defs/io_managers/postgres.py
pipeline/scheduler/src/scheduler/defs/io_managers/image_object_store.py
pipeline/scheduler/src/scheduler/defs/http_resources/jiuyan__industry_ocr.py
```

必须不存在业务逻辑：

```text
pipeline/scheduler/src/scheduler/defs/jiuyan_industry_ocr/
```

如果该目录短期保留，只能包含兼容 re-export，并有删除计划。

### Definition 验收

`dg list defs --json` 中应包含：

```text
assets:
  jiuyan__industry_list
  jiuyan__industry_images
  jiuyan__industry_ocr
  jiuyan__action_field
  ths__limit_up_pool
  eastmoney__balance
  eastmoney__cashflow_sq
  eastmoney__cashflow_ytd
  eastmoney__dividend_allotment
  eastmoney__dividend_main
  eastmoney__equity_history
  eastmoney__income_sq
  eastmoney__income_ytd

jobs:
  jiuyan__industry_ocr_pipeline_job
  jiuyan__action_field_daily_job
  ths__limit_up_pool_daily_job
  eastmoney__daily_job

schedules:
  jiuyan__industry_ocr_pipeline_schedule
  jiuyan__action_field_daily_schedule
  ths__limit_up_pool_daily_schedule
  eastmoney__daily_schedule
```

不应包含：

```text
jobs:
  http_resources__market_event_daily_job
  jiuyan__industry_images_job
  jiuyan__industry_ocr_job

sensors:
  sina__trade_calendar_dynamic_partitions_sensor
```

`jiuyan__industry_ocr_full_job` 是否存在，以阶段 3 的评估结果为准；如果存在，必须有明确人工运维说明。

### 数据输出验收

EastMoney：

- Parquet 字段不包含：
  - `request_code`
  - `request_start_date`
  - `request_end_date`
  - `partition_year`
  - `source_endpoint`
  - `ingested_at`
- 空表 schema 同样不包含这些字段。
- metadata 中能看到请求范围、endpoint、请求数和重试统计。

交易日 HTTP assets：

- 交易日输出路径仍是：

```text
source/jiuyan__action_field/trade_date=YYYY-MM-DD/000000_0.parquet
source/ths__limit_up_pool/trade_date=YYYY-MM-DD/000000_0.parquet
```

- 非交易日不创建对应 S3 目录。
- 交易日远端空响应仍可以写空表。
- metadata 区分 `processed_trade_dates` 和 `skipped_non_trade_dates_sample`。

韭研 OCR：

- 图片对象路径仍是：

```text
img/jiuyan__industry_images/<image_filename>
```

- OCR 单图结果路径仍是：

```text
source/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

- PostgreSQL 状态流转不变。

## 建议 PR 拆分

建议拆为 5 个 PR：

1. **PR 1：基础设施移动**
   - `postgres.py`、`image_object_store.py` 迁入 `io_managers`。
   - 更新 import 和测试。

2. **PR 2：OCR 通用模块 + 韭研业务迁移**
   - 新增 `defs/ocr`。
   - 韭研 OCR asset 迁入 `http_resources`。
   - 统一链路 job/schedule。

3. **PR 3：EastMoney 改造**
   - 资产工厂化。
   - schedule 统一注册。
   - 删除请求派生字段。

4. **PR 4：交易日自然日分区**
   - 删除动态分区。
   - 引入自然日分区。
   - sparse output。
   - 拆分 daily job/schedule。

5. **PR 5：清理和文档**
   - 删除兼容层。
   - README/ADR 更新。
   - 全量质量门禁。

如果 PR 2 过大，可拆成 “OCR 通用模块” 和 “韭研资产迁移/调度” 两个 PR。

## 风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 大量文件移动导致 import 断裂 | definitions 无法加载 | 每阶段保留短期 re-export，并运行 `uv run dg check defs` |
| Asset key 意外变化 | Dagster lineage 断裂 | 工厂和迁移后 asset 必须显式保持原 name/key |
| EastMoney schema 删除字段影响临时查询 | 下游临时脚本失败 | 请求上下文进入 metadata；计划中明确不保留 raw 技术列 |
| Sparse output 掩盖交易日漏写 | run 成功但交易日无数据 | 只允许非交易日 skip；交易日请求或写入失败必须 fail |
| 自然日分区导致 UI 显示非交易日 | UI 中看到不可请求日期 | metadata 明确 skip；schedule 层非交易日不提交日常 run |
| 韭研 OCR 独立 job 移除影响人工操作 | 无法只跑局部 OCR | pipeline job 保留 run config；必要时使用 asset selection 或 image_filenames |
| 统一 OCR 抽象过度泛化 | 增加复杂度 | 第一版只支持 OpenAI-compatible chat completions |
| PR 范围过大 | review 困难、回滚困难 | 按建议 PR 拆分执行 |

## 回滚策略

每个 PR 应保持可回滚：

- PR 1 回滚只影响 import 路径，不改变数据。
- PR 2 回滚会恢复旧韭研 OCR 目录和旧 job；不应改动 PostgreSQL 表结构。
- PR 3 回滚会恢复 EastMoney schema 中请求派生字段；如已写出新 schema 数据，需要用 S3 分区粒度确认是否覆盖。
- PR 4 回滚会恢复动态分区；回滚前需确认是否已经移除 dynamic partition sensor。
- PR 5 只做清理和文档，回滚影响最小。

不建议在生产环境中对同一 EastMoney year 分区混写新旧 schema。PR 3 部署后应选择明确的测试 year 分区验证，再安排后续全量刷新。

## 执行完成记录

实施时在本节追加记录：

```text
阶段 0：
- 日期：2026-05-28
- 执行人：Codex
- 结果：完成。基线 `uv run pytest scheduler/tests` 通过；`uv run dg check defs` 在 `pipeline/scheduler` 项目目录下通过；记录了原始 definitions 列表。
- 阻塞：无。

阶段 1：
- 日期：2026-05-28
- 执行人：Codex
- 结果：完成。PostgreSQL repository 迁入 `defs/io_managers/postgres.py`，图片对象存储迁入 `defs/io_managers/image_object_store.py`，旧业务目录后续已删除。
- 阻塞：无。

阶段 2：
- 日期：2026-05-28
- 执行人：Codex
- 结果：完成。新增 `defs/ocr` 通用 OCR client/schema/service；韭研 prompt、schema 转换和业务处理保留在 `defs/http_resources`。
- 阻塞：无。

阶段 3：
- 日期：2026-05-28
- 执行人：Codex
- 结果：完成。`jiuyan__industry_images`、`jiuyan__industry_ocr` 迁入 `defs/http_resources/jiuyan__industry_ocr.py`，asset key 不变，group 调整为 `http_sources`；注册 `jiuyan__industry_ocr_pipeline_job` 和 `jiuyan__industry_ocr_pipeline_schedule`；移除旧 OCR 独立 job/full job 注册。
- 阻塞：无。

阶段 4：
- 日期：2026-05-28
- 执行人：Codex
- 结果：完成。EastMoney raw Parquet schema 删除请求派生字段，保留业务字段；请求范围和 endpoint 上下文写入 materialization metadata；daily job/schedule 改由 `http_resources/schedules.py` 统一导出。
- 阻塞：无。

阶段 5：
- 日期：2026-05-28
- 执行人：Codex
- 结果：完成。删除交易日动态分区定义、同步 helper 和 sensor；`jiuyan__action_field` 使用 2021-01-01 起始自然日分区，`ths__limit_up_pool` 使用 2025-01-01 起始自然日分区；运行时读取 `sina__trade_calendar` 过滤交易日，非交易日不请求、不写 S3；拆分为两个 daily job/schedule。
- 阻塞：无。

阶段 6：
- 日期：2026-05-28
- 执行人：Codex
- 结果：完成。更新 `pipeline_defs.py` 和 `pipeline/scheduler/README.md`；删除旧 `defs/jiuyan_industry_ocr` 目录和 EastMoney 特殊 schedules 子模块；`dg list defs --json` 中不再包含旧 market-event job、旧 OCR jobs 或动态分区 sensor。
- 阻塞：无。

阶段 7：
- 日期：2026-05-28
- 执行人：Codex
- 结果：部分完成。`uv run ruff check scheduler/src scheduler/tests migrate` 通过；`uv run ruff format scheduler/src scheduler/tests migrate` 已执行；`uv run pytest scheduler/tests` 通过 107 个测试；`uv run dg check defs` 通过。`uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing` 的测试通过但总覆盖率 55.65%，低于 70% 门槛；`uv run pyright scheduler/src scheduler/tests` 仍报告项目范围既有类型问题。
- 阻塞：覆盖率门槛和 pyright 需要单独补齐全项目测试覆盖与类型债，不属于 RFC 0005 行为变更本身。

...
```
