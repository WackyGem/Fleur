# pipeline/scheduler 下一阶段一刀切重构实施计划

日期：2026-05-31

关联设计文档：

- `docs/optimize/archive/scheduler-2026-05-31-next-stage-maintainability-optimization.md`

## 1. 目标

本计划用于把 `pipeline/scheduler` 从当前“主要边界已建立，但仍有局部分叉”的状态，一次性推进到优化设计文档定义的最终目标形态。

本轮采用一刀切策略：

- 不保留旧抽象和新抽象长期双轨。
- 不为旧测试、旧 metadata、旧 import path 增加兼容层。
- 不把执行限流继续伪装成数据依赖。
- 不接受“局部迁移后以后再统一”的中间状态进入主干。
- 每个阶段结束后必须能加载 Dagster definitions，并且静态检查通过。

最终目标：

- 每个数据源目录只暴露自己的 `SourceBundle`。
- Asset 函数只保留 Dagster 边界适配职责。
- HTTP/TCP/S3/PostgreSQL 都通过 resource、factory 或 gateway 注入。
- 并发执行、失败统计、partial failure 只有一套模型。
- S3 layout、读写、metadata 只有一个权威实现。
- Repository API 类型化，状态机显式化。
- Asset lineage 只表达真实数据依赖，执行限流通过执行策略表达。
- 架构边界由测试保护。

## 2. 一刀切规则

### 2.1 删除和禁止的模式

本轮重构完成时必须删除或禁止：

- source service 直接 import `scheduler.defs.storage.parquet_readers`。
- source service 内部直接读取 `S3Config.from_env()`。
- source service 直接构造 `HttpClientFactory`、`AioHttpClient`、`BaostockAioTcpClient`。
- EastMoney endpoint 之间用于限流的链式 asset dependency。
- Repository public dataclass 中的状态裸字符串。
- 批量 repository 方法内部逐条无策略 `cursor.execute()`。
- asset 函数内大段 metadata 拼装。
- 新增 package-level compatibility re-export。
- 源码和测试目录中的 `__pycache__`。

允许保留：

- 第三方库边界处必要的 `Any` 和 `cast()`。
- `config/env.py` 与 Dagster resource 默认值中的 `dg.EnvVar`。
- source-specific schema/generated 文件中的大体量常量。

### 2.2 测试策略

本次不是“先冻结测试、最后补测试”的历史重构方式。当前代码已经有较好的基线，新增抽象必须伴随测试一起落地。

规则：

- 每个阶段修改生产代码时，同阶段补对应单测或 contract 测试。
- 删除旧行为时同步删除旧测试，不写兼容断言。
- 架构边界类测试优先使用文本扫描，避免加载大量外部依赖。
- Dagster definitions 行为用 `dg check defs` 和 bundle contract 测试共同保护。

### 2.3 最小验证门禁

每个阶段结束至少执行：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
cd scheduler
uv run dg check defs
```

如阶段只改文档，可只执行：

```bash
git diff --check
```

## 3. 阶段 A：工程卫生与边界护栏

目标：先清理噪声并把下一阶段禁止模式固化为测试，防止后续重构反复回退。

### A.1 清理 `__pycache__`

操作：

1. 删除 `pipeline/scheduler/src` 和 `pipeline/scheduler/tests` 下全部 `__pycache__`。
2. 确认 `.gitignore` 已覆盖 `__pycache__/` 和 `*.py[cod]`。
3. 新增工程卫生测试或脚本检查，禁止源码树出现 pycache。

完成标准：

```bash
find pipeline/scheduler/src pipeline/scheduler/tests -path '*/__pycache__/*' -type f | wc -l
```

输出为 `0`。

### A.2 增强架构边界测试

修改位置：

- `pipeline/scheduler/tests/integration/test_asset_contract_policy.py`
- 或新增 `pipeline/scheduler/tests/integration/test_architecture_boundaries.py`

新增断言：

- `defs/storage` 不 import `scheduler.defs.sources`。
- `defs/repositories` 不 import `dagster`。
- `defs/sources` 和 `defs/baostock` 不调用 `S3Config.from_env()`。
- 除 resource/factory 适配层外，source service 不直接构造通用 HTTP/TCP client。
- `defs/sources/eastmoney/assets.py` 中不出现 endpoint 间链式 dependency metadata。

完成标准：

- 禁止模式由测试表达。
- 后续阶段可以先改代码，再用这些测试确认最终边界。

## 4. 阶段 B：统一并发执行模型

目标：把 OCR、HTTP daily partition、EastMoney、BaoStock 的并发执行收敛到同一套 runner、失败策略和 metadata。

### B.1 扩展 `BoundedTaskRunner`

修改位置：

- `pipeline/scheduler/src/scheduler/defs/common/concurrency.py`
- `pipeline/scheduler/tests/unit/common/test_concurrency.py`

新增对象：

```python
@dataclass(frozen=True)
class BoundedTaskOptions:
    max_concurrent_tasks: int
    fail_fast: bool = False
    max_failure_ratio: float | None = None
    fail_when_all_failed: bool = True
    preserve_order: bool = False
```

扩展能力：

- `fail_fast`：第一个失败后停止调度新任务。
- `max_failure_ratio`：失败率超过阈值时报错。
- `fail_when_all_failed`：全部失败时强制报错。
- `preserve_order`：需要时保持输出顺序。
- `metadata(item_name=...)`：统一生成失败和耗时 metadata。

完成标准：

- runner 单测覆盖成功、部分失败、全失败、fail fast、顺序保持和阈值失败。
- 原有 OCR 调用不破坏。

### B.2 迁移 `http/partitioning.py`

修改位置：

- `pipeline/scheduler/src/scheduler/defs/http/partitioning.py`
- `pipeline/scheduler/tests/unit/partitioning/`
- `pipeline/scheduler/tests/unit/http/`

操作：

1. 删除本地 semaphore + TaskGroup + failed dict 实现。
2. 使用 `BoundedTaskRunner` 执行 partition worker。
3. 将失败字段改为 runner 标准 metadata。
4. 将 `PartialFailurePolicy` 调用集中在 runner 或 materialization result 边界。

完成标准：

- partitioning 不再直接管理 `failed_partition_errors` 字典。
- daily partition metadata 与 OCR failure metadata 字段同构。

### B.3 迁移 EastMoney 和 BaoStock

修改位置：

- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/services.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
- 对应 unit tests。

操作：

1. 将 `asyncio.TaskGroup()` 替换为 `BoundedTaskRunner`。
2. 对 EastMoney 使用 partial failure 策略，失败样本进入 metadata。
3. 对 BaoStock 明确 `fail_fast` 或“全部成功”策略，不依赖 TaskGroup 隐式取消语义。
4. 保留每个数据源特有的统计字段，如 selected security count、unknown field count。

完成标准：

- `rg "asyncio\\.TaskGroup" pipeline/scheduler/src/scheduler/defs` 不再命中 source service 并发抓取流程。
- EastMoney、BaoStock、OCR、daily partition 都使用统一 runner。

## 5. 阶段 C：S3 Dataset Gateway 一刀切

目标：S3 layout、读写、metadata 全部由 `S3DatasetService` 和 gateway 接管，业务 service 不知道 object key 细节。

### C.1 扩展 storage gateway

修改位置：

- `pipeline/scheduler/src/scheduler/defs/storage/dataset_service.py`
- `pipeline/scheduler/src/scheduler/defs/storage/parquet_readers.py`
- `pipeline/scheduler/tests/unit/storage/`

操作：

1. 将 `read_parquet_table_from_s3()` 和 `read_partitioned_parquet_tables_from_s3()` 降级为 `S3DatasetService` 内部 helper。
2. 新增 source-neutral gateway：

```python
class DatasetReader(Protocol):
    def read_latest_snapshot(self, location: DatasetLocation) -> pa.Table: ...
    def read_partitioned(
        self,
        location: DatasetLocation,
        *,
        partition_keys: Sequence[str],
        partition_key_name: str,
    ) -> PartitionedParquetReadResult: ...
```

3. `S3DatasetService` 负责所有 asset key 到 object key 的转换。
4. `S3IOManager` 继续只调用 `S3DatasetService`。

完成标准：

- `storage.parquet_readers` 不再被 source service import。
- 所有 S3 路径计算都在 `storage/` 内部。

### C.2 引入业务 reader gateway

新增或调整位置：

- `pipeline/scheduler/src/scheduler/defs/market/trade_calendar.py`
- `pipeline/scheduler/src/scheduler/defs/market/securities.py`
- 可新增 `pipeline/scheduler/src/scheduler/defs/market/readers.py`

新增接口：

```python
class TradeCalendarReader(Protocol):
    def read_trade_dates(self) -> set[date]: ...

class SecurityUniverseReader(Protocol):
    def read_stock_basic(self) -> pa.Table: ...
```

操作：

1. BaoStock 和 EastMoney service constructor 接收 reader gateway，不接收裸 `S3Config`。
2. daily compact 使用 `DatasetReader` 读取上游 daily partitions。
3. schedule 中已有 `TradeCalendarReader` 保留，但默认实现转为复用统一 reader。

完成标准：

- `BaostockDailyKlineRefreshService` 和 `EastmoneyYearRefreshService` 不 import `storage.parquet_readers`。
- 测试可以用 fake reader 构造 service，不需要 S3 fake。

### C.3 统一 compact metadata

修改位置：

- `pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py`
- `pipeline/scheduler/src/scheduler/defs/common/metadata.py`
- `pipeline/scheduler/src/scheduler/defs/asset_contracts.py`

操作：

1. 新增 compact metadata builder。
2. `daily_compact.py` 不再手写 storage/shape 字段。
3. compact metadata 字段与普通 partition metadata 对齐。

完成标准：

- compact asset 与 source daily asset 的 row、partition、empty、missing 字段命名一致。

## 6. 阶段 D：Client Factory Resource 化与 EastMoney lineage 修正

目标：把远端访问配置集中到 resource/factory，去掉用 asset dependency 表达限流的做法。

### D.1 HTTP client factory resource

新增或修改位置：

- `pipeline/scheduler/src/scheduler/defs/resources/http.py`
- `pipeline/scheduler/src/scheduler/defs/http/client_factory.py`
- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- 各 source assets/workflows/services。

操作：

1. 新增 `HttpClientFactoryResource`，包含 retry、timeout、connector limit、request delay 默认值。
2. asset 或 workflow 从 resource 获取 factory。
3. source service 接收 factory 或更窄的 client protocol。
4. 删除 source service 内直接 `HttpClientFactory(...)`。

完成标准：

- `rg "HttpClientFactory\\(" pipeline/scheduler/src/scheduler/defs/sources pipeline/scheduler/src/scheduler/defs/baostock` 只命中 resource/factory 适配层或测试 fake。
- Dagster definitions 注册 `http_client_factory` resource。

### D.2 BaoStock client settings resource

新增或修改位置：

- `pipeline/scheduler/src/scheduler/defs/resources/baostock.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/client.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`

操作：

1. 新增 BaoStock client factory 或 settings resource。
2. TCP host、port、账号、连接数从 resource/factory 注入。
3. service 不直接构造 `BaostockAioTcpClient(max_connections=...)`。

完成标准：

- BaoStock 远端连接参数在 Dagster resources 中可见。
- BaoStock service 可通过 fake client factory 纯单测。

### D.3 EastMoney 去掉伪依赖 lineage

修改位置：

- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/assets.py`
- `pipeline/scheduler/src/scheduler/defs/sources/eastmoney/services.py`
- `pipeline/scheduler/tests/integration/test_asset_contract_policy.py`

操作：

1. `build_eastmoney_asset()` 不再接收 `ordering_dependency`。
2. 每个 EastMoney endpoint 的 `deps` 只保留 `baostock__query_stock_basic`。
3. 限流由 Dagster pool、client concurrency、schedule 分批或 runner options 表达。
4. 删除 `METADATA_EXECUTION_ORDERING_DEPENDENCY` 和 `generated_endpoint_metadata()` 中的 ordering 字段。
5. 新增 lineage contract test：EastMoney endpoint 之间不得互相依赖。

完成标准：

- `dg list defs --json` 中 EastMoney 资产只依赖 BaoStock stock basic。
- EastMoney endpoint 间没有链式 dependency。

## 7. 阶段 E：Repository 状态机与批量 SQL

目标：Repository public API 类型化到底，连接生命周期和批量更新明确。

### E.1 状态 enum 化

修改位置：

- `pipeline/scheduler/src/scheduler/defs/repositories/industry_images.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/state_service.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/ocr_schema.py`
- repository/state tests。

操作：

1. 新增 `DownloadStatus` 和 `OcrStatus`。
2. `IndustryImageRecord`、`ClaimedIndustryImage` 使用 enum 或明确 Literal。
3. row mapper 负责 DB 字符串到 enum 的转换。
4. 非法状态抛 `RuntimeError`，错误消息包含字段和值。

完成标准：

- repository public dataclass 不再暴露状态裸字符串。
- pyright 可检查状态比较。

### E.2 Connection factory/pool 化

修改位置：

- `pipeline/scheduler/src/scheduler/defs/resources/database.py`
- `pipeline/scheduler/src/scheduler/defs/repositories/industry_images.py`

操作：

1. 定义 `PipelineDatabaseConnectionFactory` 或 pool resource。
2. Repository constructor 接收 connection factory，不只接收 url。
3. 默认 resource 从 `PIPELINE_DATABASE_URL` 构造 factory。
4. 测试 fake repository 不依赖真实 psycopg connect。

完成标准：

- repository 方法不直接调用全局 `connect_pipeline_database()`。
- 单测可统计连接获取次数。

### E.3 批量 update 改造

操作：

1. `*_many()` 使用批量执行。
2. 单条方法只保留 wrapper，内部调用 many。
3. 批量方法返回实际影响行数，而不是输入长度。
4. 对影响行数小于输入数的情况写入 metadata 或日志。

完成标准：

- OCR 一个批次成功/失败更新的连接数固定。
- batch update 单测覆盖空输入、全部更新、部分未命中。

## 8. 阶段 F：Asset 边界瘦身与 Metadata Builder

目标：asset 文件只保留 Dagster adapter，metadata contract 统一生成。

### F.1 新增 async boundary helper

新增位置：

- `pipeline/scheduler/src/scheduler/defs/common/async_boundary.py`

操作：

1. 新增 `run_async_boundary()`，统一 `asyncio.run()` 错误上下文。
2. 资产函数统一使用 helper 调用 async use case。
3. 禁止各 asset 文件直接散落 `asyncio.run()`。

完成标准：

- `rg "asyncio\\.run" pipeline/scheduler/src/scheduler/defs/sources pipeline/scheduler/src/scheduler/defs/baostock` 只命中 helper 或允许清单。

### F.2 Sina trade calendar service 化

修改位置：

- `pipeline/scheduler/src/scheduler/defs/sources/sina/trade_calendar.py`

操作：

1. 新增 `SinaTradeCalendarRefreshService`。
2. fetch、parse、table conversion、metadata builder 下沉到 service。
3. asset 函数只负责 context/resource/config 到 service request 的适配。
4. `context` 补 `dg.AssetExecutionContext` 类型标注。

完成标准：

- Sina asset 函数控制在 40 行以内。
- parser/service 可不依赖 Dagster context 单测。

### F.3 Metadata builder 收敛

新增或修改位置：

- `pipeline/scheduler/src/scheduler/defs/common/metadata.py`
- `pipeline/scheduler/src/scheduler/defs/storage/dataset_service.py`
- `pipeline/scheduler/src/scheduler/defs/asset_contracts.py`

新增 builder：

- `DatasetMetadataBuilder`
- `FetchStatsMetadataBuilder`
- `PartitionRunMetadataBuilder`
- `FailureMetadataBuilder`

操作：

1. `S3DatasetService.metadata()` 委托 `DatasetMetadataBuilder`。
2. runner 失败 metadata 委托 `FailureMetadataBuilder`。
3. BaoStock、EastMoney、JiuYan、Sina 的 fetch stats 统一字段。
4. 删除各 service 中重复的 `row_count`、`column_count`、`file_format`、`compression` 拼装。

完成标准：

- storage、shape、execution、quality 四类 metadata 字段有统一 builder。
- asset contract 测试覆盖必需字段。

## 9. 阶段 G：最终测试重建与质量门禁升级

目标：删除重构过程中废弃的旧测试，补齐最终形态的 contract 和 policy 测试。

### G.1 测试重建

新增或调整测试：

- `tests/unit/common/test_concurrency.py`
- `tests/unit/storage/test_dataset_service.py`
- `tests/unit/market/test_readers.py`
- `tests/unit/sources/eastmoney/test_lineage_and_rate_limit.py`
- `tests/unit/repositories/test_industry_image_state_machine.py`
- `tests/integration/test_architecture_boundaries.py`
- `tests/integration/test_asset_contract_policy.py`

删除：

- 依赖旧 metadata 字段的测试。
- 依赖 EastMoney endpoint 链式依赖的测试。
- 通过 monkeypatch env 或 S3 path 细节才能通过的测试。

### G.2 类型和 lint 收紧

操作：

1. 先保持全局 pyright `basic`。
2. 对 `common`、`storage`、`partitioning`、`repositories` 增加局部严格约束，或通过测试/ruff 规则禁止裸 `Any` 扩散。
3. 新增文本边界测试替代过度复杂的 lint 插件。

完成标准：

- 新增共享抽象都有 focused unit tests。
- 架构边界测试覆盖主要禁止模式。
- `Any` 主要保留在 pyarrow、psycopg、aiohttp 边界。

## 10. 最终验收清单

代码结构：

- `SOURCE_BUNDLES` 是顶层 definitions 的唯一 source 聚合入口。
- `defs/http` 不 import `defs/sources`。
- `defs/storage` 不 import `defs/sources`。
- `defs/repositories` 不 import Dagster。
- source service 不 import `storage.parquet_readers`。
- source service 不直接构造远端 client。

Dagster：

- 所有 asset 函数有明确返回类型。
- 复杂 asset 使用 `MaterializeResult[T]`。
- EastMoney endpoint 间无伪数据依赖。
- 资源中可见 HTTP/TCP/S3/PostgreSQL 运行配置。
- `dg check defs` 通过。

运行模型：

- 并发抓取统一使用 `BoundedTaskRunner`。
- partial failure 策略统一。
- failure metadata 字段统一。
- S3 read/write 统一经 `S3DatasetService`。

Repository：

- 状态字段 enum 化。
- batch update 返回实际影响行数。
- connection factory/pool 可测试。

测试与质量：

- `ruff check` 通过。
- `ruff format --check` 通过。
- `pyright` 通过。
- `pytest` 通过。
- `dg check defs` 通过。
- 源码和测试目录无 `__pycache__`。

## 11. 推荐执行命令

每个阶段完成后：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests
cd scheduler
uv run dg check defs
```

最终完成后再跑覆盖率：

```bash
cd pipeline
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
```

## 12. 退出标准

只有满足以下条件，本轮一刀切重构才算完成：

- 本文阶段 A 到 G 全部完成。
- `docs/optimize/archive/scheduler-2026-05-31-next-stage-maintainability-optimization.md` 中的最终目标形态全部可在代码中验证。
- 不存在为了兼容旧路径、旧 metadata、旧测试而保留的桥接层。
- 架构边界测试可以阻止主要反模式回归。
- 最小验证门禁全部通过。
