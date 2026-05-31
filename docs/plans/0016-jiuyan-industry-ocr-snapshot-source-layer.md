# Plan 0016: `source/jiuyan__industry_ocr_snapshot` source 层发布计划

日期：2026-05-31

关联设计文档：

- `docs/RFC/0009-dagster-clickhouse-raw-sync.md`
- `docs/RFC/archive/0004-jiuyan-industry-list-ocr.md`
- `docs/plans/archive/0005-jiuyan-industry-list-ocr-implementation.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`

## 1. 目标

本计划为 RFC 0009 的 ClickHouse raw 同步做前置准备：新增一个稳定的 source 层快照资产 `source/jiuyan__industry_ocr_snapshot`，将当前按单图写出的 OCR parquet 结果汇总为一个 latest snapshot parquet。

目标产物：

```text
source/jiuyan__industry_ocr_snapshot/000000_0.parquet
```

该资产后续作为 ClickHouse raw 层整表替换的输入。当前 `source/jiuyan__industry_ocr` 继续保留为 OCR 工作队列处理资产，不直接进入 ClickHouse raw。

## 2. 非目标

本计划不做以下事情：

- 不重写 OCR prompt、OCR client 或图片下载逻辑。
- 不引入 `image_id`、图片引用表、OCR 版本表等复杂模型。
- 不改变现有 `jiuyan_industry_images.image_filename` 主键。
- 不把 OCR 按日期分区。
- 不实现 ClickHouse resource、raw sync asset 或 dbt 模型。
- 不改变 `source/jiuyan__industry_list` 的 latest snapshot 语义。

## 3. 当前事实基线

当前链路：

```text
source/jiuyan__industry_list
  -> source/jiuyan__industry_images
  -> source/jiuyan__industry_ocr
```

当前资产语义：

| Asset | 当前职责 | 当前输出 |
|------|----------|----------|
| `source/jiuyan__industry_list` | 拉取韭研产业研究列表 | S3 latest snapshot parquet |
| `source/jiuyan__industry_images` | 从列表解析图片 URL、下载图片、维护 Postgres 下载状态 | S3 图片对象 + Postgres 状态 |
| `source/jiuyan__industry_ocr` | 从 Postgres claim 待 OCR 图片、调用 OCR、写单图 OCR parquet | 单图 parquet + Postgres OCR 状态 |

当前 OCR 单图结果路径：

```text
source/jiuyan__industry_ocr/image_filename=<image_filename>/000000_0.parquet
```

当前 `source/jiuyan__industry_ocr` 的 materialization 语义是“处理了一批 OCR 队列任务”，不是“发布了完整 source 表”。运行报告 `docs/jobs/reports/005-jiuyan__industry_ocr.md` 中，50 张 OCR 限量回填完成后仍有 `pending=1142`，但 asset run 成功。这说明该 asset 不能直接作为 ClickHouse raw source。

当前数据量预期：

- OCR 业务结果约 3 万多行。
- 对 ClickHouse 而言，3 万行整表替换非常小；因此本计划采用 latest snapshot 全量发布，不做增量 raw 设计。

## 4. 更新语义

### 4.1 历史全量

`jiuyan__industry_ocr` 的历史全量不按日期定义，而是状态队列补齐：

```text
历史全量 = 反复运行 OCR processor，直到当前列表中已下载图片的 pending/failed/stale running 被处理到可接受状态。
```

推荐命令模式：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job jiuyan__industry_ocr_pipeline_job \
  --config-json '{"ops":{"source__jiuyan__industry_ocr":{"config":{"limit":100}}}}'
```

重复执行，直到 Postgres 状态汇总满足发布要求。

### 4.2 每日增量

每日增量同样不按日期定义，而是状态增量：

```text
每日增量 = 当天刷新 industry_list 后，新发现图片、上次失败图片、stale running 图片进入待处理状态；OCR processor 只处理这些状态行。
```

默认生产配置：

```text
force_download = false
force_ocr = false
limit = 100 或按运行窗口调整
```

该默认值必须落到 `jiuyan__industry_ocr_pipeline_schedule` 的 run config 中，不能只停留在文档说明。当前 `IndustryOcrConfig.limit` 默认是 `None`，如果 schedule 不传 config，会一次 claim 所有符合条件的图片。

### 4.3 source 层发布

`source/jiuyan__industry_ocr_snapshot` 每次运行都从当前 Postgres 成功清单读取所有成功 OCR 结果，合并为一个完整 latest snapshot。

该 snapshot 的语义：

```text
当前已成功 OCR 的产业研究图片结果全集。
```

它不负责补跑 OCR，也不修改 OCR 状态。

## 5. 目标架构

```text
source/jiuyan__industry_list
  - latest snapshot
       |
       v
source/jiuyan__industry_images
  - stateful downloader
  - writes image objects
  - updates jiuyan_industry_images
       |
       v
source/jiuyan__industry_ocr
  - stateful OCR processor
  - writes per-image OCR parquet
  - updates jiuyan_industry_images
       |
       v
source/jiuyan__industry_ocr_snapshot
  - deterministic publisher
  - reads successful OCR result keys from Postgres
  - reads per-image parquet from S3
  - writes latest snapshot parquet through s3_io_manager
       |
       v
RFC 0009 ClickHouse raw sync
  - full snapshot replace into raw.jiuyan__industry_ocr_snapshot
```

## 6. 输出 Schema

当前单图 OCR parquet 只有业务字段：

```text
industry_id
stock_name
theme_path
relation
source
```

`jiuyan__industry_ocr_snapshot` 建议在合并时补充轻量溯源字段，形成 source 层 raw-ish schema：

| 字段 | 类型 | 来源 | 说明 |
|------|------|------|------|
| `industry_id` | string | 单图 OCR parquet / Postgres | 韭研产业研究文章 ID |
| `image_filename` | string | Postgres | 图片文件名，当前稳定处理键 |
| `image_index` | int32 | Postgres | 图片在文章中的位置 |
| `ocr_row_index` | int32 | 合并时生成 | 同一图片 OCR 结果行号 |
| `stock_name` | string | 单图 OCR parquet | 个股、公司或标的名称 |
| `theme_path` | string | 单图 OCR parquet | 题材路径，英文逗号连接 |
| `relation` | string | 单图 OCR parquet | 个股与题材关系说明 |
| `source` | string | 单图 OCR parquet | 来源或信源 |

说明：

- `image_filename` 和 `ocr_row_index` 使 snapshot 行有稳定的技术定位。
- dbt staging 可以隐藏 `image_filename`、`image_index`、`ocr_row_index` 等技术字段。
- 第一阶段不引入 `image_id`。如果未来出现同名不同 URL 或重算版本需求，再单独设计迁移。

## 7. 模块改动

### 7.1 新增或修改文件

```text
pipeline/scheduler/src/scheduler/defs/sources/jiuyan/
  industry_ocr_snapshot.py      # 新增：snapshot asset + service boundary
  ocr_snapshot_schema.py        # 可选：snapshot PyArrow schema

pipeline/scheduler/src/scheduler/defs/storage/
  object_store.py               # 新增按 S3 key 读取 parquet table 的 helper

pipeline/scheduler/src/scheduler/defs/repositories/
  industry_images.py            # 新增 success 清单和状态汇总查询

pipeline/scheduler/src/scheduler/defs/sources/jiuyan/
  definitions.py                # 注册 snapshot asset/job

pipeline/scheduler/tests/unit/sources/jiuyan/
  test_industry_ocr_snapshot.py # 新增 snapshot service/asset 单测

pipeline/scheduler/tests/unit/sources/jiuyan/
  test_industry_ocr_state_flow.py # 扩展 repository 查询测试

pipeline/scheduler/tests/integration/
  test_definitions_and_schedules.py # 更新 definitions 基线
```

### 7.2 Repository API

新增只读查询：

```python
@dataclass(frozen=True)
class SuccessfulOcrResultRecord:
    image_filename: str
    industry_id: str
    image_index: int
    ocr_result_s3_key: str
    ocr_result_row_count: int

@dataclass(frozen=True)
class OcrStatusSummary:
    download_success_count: int
    ocr_success_count: int
    ocr_failed_count: int
    ocr_pending_count: int
    ocr_running_count: int
    ocr_success_result_row_count: int

def list_successful_ocr_results(self) -> list[SuccessfulOcrResultRecord]: ...

def summarize_ocr_status(self) -> OcrStatusSummary: ...
```

查询规则：

- `list_successful_ocr_results()` 只返回 `download_status='success'`、`ocr_status='success'`、`ocr_result_s3_key is not null` 的行。
- 按 `image_filename` 排序，保证 snapshot 输出稳定。
- 如果 `ocr_result_row_count = 0`，仍返回该记录；snapshot metadata 需要统计 0 行结果图片数量，但该图片不会贡献业务行。

### 7.3 Snapshot service

新增 service 负责：

1. 从 repository 获取成功 OCR 清单。
2. 按 `ocr_result_s3_key` 读取每个单图 parquet。
3. 校验每个 parquet schema 与 `JIUYAN_INDUSTRY_OCR_SCHEMA` 兼容。
4. 给每行补充：
   - `image_filename`
   - `image_index`
   - `ocr_row_index`
5. 合并为一个 `pa.Table`。
6. 返回 table 和 metadata。

读取单图 parquet 的落点：

- 在 `ObjectStore` 增加 `read_table_by_key(key: str) -> pa.Table`，按完整 S3 object key 读取 parquet。
- `ImageObjectStore` 可薄封装为 `read_ocr_result_table(key: str) -> pa.Table`，但底层读取能力应放在 source-neutral 的 `storage/object_store.py`。
- 不使用 `S3DatasetService.read_latest_snapshot()` 读取单图结果，因为单图结果 key 已由 Postgres `ocr_result_s3_key` 给出，不是普通 asset key latest snapshot。

失败规则：

- 如果 Postgres 标记 success 但 S3 parquet 不存在，snapshot asset 失败，不写出新 snapshot。
- 如果单图 parquet schema 不兼容，snapshot asset 失败。
- 如果成功 OCR 清单为空，snapshot asset 失败，避免发布空 source 表。
- 0 行 OCR 结果图片不是失败；只记录 metadata。

### 7.4 Snapshot asset

新增 asset：

```python
@dg.asset(
    name="jiuyan__industry_ocr_snapshot",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    deps=[jiuyan__industry_ocr],
    io_manager_key="s3_io_manager",
    metadata=latest_snapshot_metadata(),
    owners=source_owners(),
    kinds=s3_parquet_kinds("postgres", "ocr", "snapshot"),
    tags=source_tags("jiuyan"),
)
def jiuyan__industry_ocr_snapshot(...) -> dg.MaterializeResult[pa.Table]:
    ...
```

注意：

- 该 asset 返回 `pa.Table`，交给 `s3_io_manager` 写 latest snapshot。
- 它只读取状态和 S3 单图结果，不调用远端 OCR。
- 它依赖 `jiuyan__industry_ocr`，表达“发布前至少经过 OCR processor”。
- 历史补齐时，可以单独运行该 snapshot asset 来发布当前成功结果。

## 8. Job 和 Schedule

新增 job：

```text
jiuyan__industry_ocr_snapshot_job
```

selection：

```text
[jiuyan__industry_ocr_snapshot]
```

调整现有 pipeline job 的候选方案：

| 方案 | 内容 | 选择 |
|------|------|------|
| A | `jiuyan__industry_ocr_pipeline_job` 继续只跑 list/images/ocr，snapshot 单独 job | 不采用 |
| B | pipeline job 改为 list/images/ocr/snapshot | 采用 |

采用方案 B。原因：

- `jiuyan__industry_ocr_snapshot` 是轻量合并发布，当前约 3 万行，全量发布成本可接受。
- 每次 OCR processor 完成后立即刷新 snapshot，ClickHouse raw 后续只需消费最新稳定快照。
- Dagster lineage 更清晰：`industry_list -> industry_images -> industry_ocr -> industry_ocr_snapshot`。
- 历史补齐时，每批 OCR 后都会发布一次当前成功结果全集；这不会影响下一批 OCR 继续处理 pending/failed/stale 图片。

同时保留单独 snapshot job，便于在不重新 OCR 的情况下重发 source snapshot：

```text
jiuyan__industry_ocr_snapshot_job
```

现有 `jiuyan__industry_ocr_pipeline_schedule` 继续调度 pipeline job，因此每天 OCR 执行完会自动运行 snapshot asset。

schedule 必须同步传入 OCR 批量配置，推荐第一阶段固定为：

```python
run_config={
    "ops": {
        "source__jiuyan__industry_ocr": {
            "config": {
                "limit": 100,
                "force_ocr": False,
            }
        }
    }
}
```

实现方式可以复用 `ScheduleSpec.execution_fn` 返回 `dg.RunRequest`，或新增专用 schedule factory；不要依赖 `IndustryOcrConfig.limit=None` 的默认行为。

## 9. 历史全量和每日增量操作

### 9.1 历史全量补齐

流程：

1. 运行 `jiuyan__industry_list`。
2. 运行 `jiuyan__industry_images`，发现并下载图片。
3. 重复运行 `jiuyan__industry_ocr(limit=100, force_ocr=false)`，直到 backlog 到可接受状态。
4. 每批 OCR 后由 pipeline job 自动运行 `jiuyan__industry_ocr_snapshot` 发布当前成功结果全集。
5. 后续 RFC 0009 将该 snapshot 导入 ClickHouse raw。

历史补齐命令应运行 `jiuyan__industry_ocr_pipeline_job`，而不是只选择 `source/jiuyan__industry_ocr`。只选择 OCR asset 时不会自动运行 downstream snapshot。

完成标准：

```text
ocr_pending_count = 0
ocr_running_count = 0
ocr_failed_count <= 人工接受阈值
source/jiuyan__industry_ocr_snapshot/000000_0.parquet 存在
snapshot row_count 约 3w 行
```

### 9.2 每日增量

流程：

1. 每日刷新 `jiuyan__industry_list`。
2. 每日运行 `jiuyan__industry_images(force_download=false)`。
3. 每日运行 `jiuyan__industry_ocr(limit=N, force_ocr=false)`。
4. OCR 执行完后自动运行 `jiuyan__industry_ocr_snapshot`。

每日增量不需要日期参数。增量边界由 Postgres 状态决定：

```text
newly discovered images
previous failed OCR images
stale running OCR images
```

## 10. Snapshot metadata

`jiuyan__industry_ocr_snapshot` materialization metadata 至少包含：

| 字段 | 说明 |
|------|------|
| `snapshot_row_count` | 输出业务行数 |
| `successful_image_count` | success OCR 图片数 |
| `zero_row_image_count` | OCR 成功但结果 0 行图片数 |
| `ocr_result_file_count` | 读取的单图 parquet 文件数 |
| `ocr_pending_count` | 当前 pending 图片数 |
| `ocr_failed_count` | 当前 failed 图片数 |
| `ocr_running_count` | 当前 running 图片数 |
| `ocr_success_result_row_count` | Postgres 记录的成功结果行数总和 |
| `snapshot_schema_version` | snapshot schema 版本，首版为 `1` |
| `s3_keys_sample` | 读取的单图 parquet key 样例 |
| `asset_function_seconds` | 资产执行耗时 |

如果 `snapshot_row_count` 与 `ocr_success_result_row_count` 不一致，asset 应失败，避免发布不完整 snapshot。

## 11. 测试策略

### 11.1 Repository tests

扩展 `test_industry_ocr_state_flow.py`：

- `list_successful_ocr_results()` 只返回 success 且 result key 非空的行。
- 查询结果按 `image_filename` 稳定排序。
- `summarize_ocr_status()` 正确统计 pending/running/failed/success。
- 0 行 OCR 成功图片计入 success，但 row count 为 0。

### 11.2 Snapshot service tests

新增 `test_industry_ocr_snapshot.py`：

- 多个单图 parquet 能合并为一个 snapshot table。
- 输出补充 `image_filename`、`image_index`、`ocr_row_index`。
- 0 行单图 parquet 不贡献业务行，但 metadata 计数正确。
- success 记录指向缺失 S3 key 时失败。
- schema 不兼容时失败。
- 合并行数与 Postgres row count 汇总不一致时失败。
- `ObjectStore.read_table_by_key()` 或 `ImageObjectStore.read_ocr_result_table()` 能按 Postgres 返回的 object key 读取 parquet。

### 11.3 Definitions tests

更新 definitions 集成测试：

- 新增 asset key `source/jiuyan__industry_ocr_snapshot`。
- 新增 job `jiuyan__industry_ocr_snapshot_job`。
- 原有 `jiuyan__industry_ocr_pipeline_job` 不被破坏。
- `jiuyan__industry_ocr_pipeline_job` selection 包含 `jiuyan__industry_ocr_snapshot`。
- `jiuyan__industry_ocr_pipeline_schedule` 会传入 OCR `limit=100` 的 run config。
- 资源数量不应因为 snapshot asset 增加外部 resource；继续复用 `s3_settings`、`image_object_store`、`industry_image_repository`。

## 12. 实施阶段

### 阶段 A：Repository 只读查询

修改：

- `pipeline/scheduler/src/scheduler/defs/repositories/industry_images.py`
- `pipeline/scheduler/tests/unit/sources/jiuyan/test_industry_ocr_state_flow.py`

完成标准：

- 成功 OCR 清单和状态汇总查询有单测覆盖。
- 不改变现有下载/OCR 状态更新行为。

验证：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/sources/jiuyan/test_industry_ocr_state_flow.py
```

### 阶段 B：Snapshot service

修改：

- `pipeline/scheduler/src/scheduler/defs/storage/object_store.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py`
- `pipeline/scheduler/tests/unit/sources/jiuyan/test_industry_ocr_snapshot.py`

完成标准：

- source-neutral object store 具备按 key 读取 parquet table 的能力。
- fake repository + fake object store 能验证 snapshot 合并逻辑。
- 失败场景不会返回可写出的 table。

验证：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/sources/jiuyan/test_industry_ocr_snapshot.py
```

### 阶段 C：Dagster asset/job 注册

修改：

- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr_snapshot.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/definitions.py`
- `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py`

完成标准：

- `dg list defs --json` 中出现 `source/jiuyan__industry_ocr_snapshot`。
- `jiuyan__industry_ocr_snapshot_job` 可见。
- `jiuyan__industry_ocr_pipeline_job` selection 包含 `jiuyan__industry_ocr_snapshot`，顺序依赖为 `industry_list -> industry_images -> industry_ocr -> industry_ocr_snapshot`。
- `jiuyan__industry_ocr_pipeline_schedule` 显式传入 `source__jiuyan__industry_ocr.config.limit=100`。
- `source/jiuyan__industry_ocr_snapshot` 使用 `s3_io_manager` 写 latest snapshot。

验证：

```bash
cd pipeline
uv run pytest scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
```

### 阶段 D：真实小批发布验证

前置：

- PostgreSQL migration 已执行。
- 已有部分 `ocr_status='success'` 记录。
- 单图 OCR parquet 存在于 S3。

命令：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__industry_ocr_snapshot"
```

核验：

- Dagster run 成功。
- materialization metadata 中 `snapshot_row_count`、`successful_image_count`、`ocr_pending_count` 等字段合理。
- S3 生成 `source/jiuyan__industry_ocr_snapshot/000000_0.parquet`。
- 读取 snapshot parquet，确认行数约等于 Postgres 成功结果行数总和。

## 13. 最小质量门禁

文档-only 变更：

```bash
git diff --check
```

实现完成后：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

## 14. 与 RFC 0009 的衔接

本计划完成后，RFC 0009 的 ClickHouse raw sync 可将 `source/jiuyan__industry_ocr_snapshot` 作为普通 latest snapshot asset 处理：

```text
source/jiuyan__industry_ocr_snapshot/000000_0.parquet
  -> ClickHouse staging table
  -> validate row_count/schema
  -> full snapshot replace raw.jiuyan__industry_ocr_snapshot
```

ClickHouse raw 层第一阶段不需要感知 OCR 增量。增量状态仍由 `jiuyan__industry_ocr` processor 和 Postgres 管理；ClickHouse 只消费稳定发布后的 3 万行左右 source snapshot。

## 15. 风险和缓解

| 风险 | 缓解 |
|------|------|
| Postgres success 记录存在但 S3 单图 parquet 缺失 | snapshot asset 失败，不覆盖上一版 snapshot |
| 单图 parquet schema 变化 | snapshot service 做 schema 校验并失败 |
| 3 万行全量合并耗时增长 | 先记录 `ocr_result_file_count` 和执行耗时；必要时后续再做分桶 compact |
| OCR backlog 未清完导致 snapshot 不完整 | metadata 暴露 pending/failed/running；发布策略允许先发布成功结果，但 ClickHouse raw 消费前应看完整度 |
| pipeline job 每批 OCR 后频繁发布 snapshot | 当前数据量可接受；若耗时增长，再将 snapshot 拆回独立 job 或降低 OCR processor 触发频率 |

## 16. 验收标准

计划完成时应满足：

1. `source/jiuyan__industry_ocr_snapshot` asset 存在，返回 `pa.Table` 并由 `s3_io_manager` 写 latest snapshot。
2. `jiuyan__industry_ocr_snapshot_job` 存在，可单独发布 snapshot。
3. `jiuyan__industry_ocr_pipeline_job` 每次运行 OCR 后都会运行 snapshot asset。
4. snapshot 表包含业务字段和轻量溯源字段。
5. snapshot metadata 能说明 OCR 完整度和输入文件数量。
6. success 状态但 S3 文件缺失时，asset 失败且不发布新 snapshot。
7. RFC 0009 可将该 asset 纳入 latest snapshot full replace 范围。
