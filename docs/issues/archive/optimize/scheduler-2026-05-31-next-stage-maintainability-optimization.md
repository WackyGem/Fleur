# pipeline/scheduler 下一阶段代码质量与架构优化设计

日期：2026-05-31

## 1. 扫描范围与基线

本次扫描范围为 `pipeline/scheduler`，覆盖 Dagster definitions、资产函数、调度工厂、HTTP/TCP 数据源、S3 Parquet 存储、JiuYan OCR 状态流、Repository、测试与工程卫生。

本次扫描基于当前实际代码，而不是早期优化文档中的历史状态。当前工程已经完成了一批关键重构：

- `defs/definitions.py` 已通过 `SOURCE_BUNDLES` 聚合 Sina、JiuYan、THS、BaoStock、EastMoney。
- `defs/source_bundle.py` 已提供轻量 bundle 抽象。
- `defs/asset_contracts.py` 已集中管理 owner、tags、kinds 与部分 metadata contract。
- `S3IOManager` 已委托 `S3DatasetService`，并已实现 `load_input()`。
- JiuYan 图片下载与 OCR 已拆出 workflow、state service，并使用 `asyncio.to_thread()` 隔离同步 repository。
- `PostgresIndustryImageRepository` 已返回领域 dataclass，不再直接向业务层暴露裸 `dict[str, object]`。
- EastMoney 已改为 `build_eastmoney_assets()`，不再使用 `globals()` 动态注入资产符号。
- `defs/http/` 已不再聚合 source definitions，测试也已有边界约束。

当前 Dagster 注册定义：

- assets：18
- jobs：9
- schedules：7
- resources：5
- sensors：1 个默认 automation condition sensor

代码规模：

- `scheduler/defs` 约 11,879 行 Python。
- 其中 EastMoney generated schema/fields 约 3,194 行。
- 源码和测试目录下存在 131 个 `__pycache__` 文件，虽然通常被 gitignore 忽略，但会干扰人工扫描和目录噪声。

因此本文重点不是“修复坏代码”，而是定义下一阶段面向规模增长的抽象收敛、可维护性优化和质量门禁升级。

## 2. 总体判断

当前架构已经从“按文件堆功能”演进到“按边界组织职责”的阶段。下一阶段最值得投入的是减少剩余分叉，而不是继续增加大而全的框架。

核心优化原则：

1. Dagster asset 只做边界适配：读取 context/config/resource，调用 use case，返回 `MaterializeResult`。
2. 重复的并发、失败统计、partial failure、metadata 拼装进入共享策略。
3. S3 数据读写只通过一个 dataset service/gateway 暴露给业务服务。
4. Repository 继续保持 SQL 集中，但要升级连接生命周期、批量执行和状态枚举。
5. 资产依赖只表达真实数据依赖，不用 lineage 伪装执行限流。
6. 测试从“枚举清单”升级为“架构约束 + contract + 策略单测”。

## 3. 优先级 P0：工程卫生与架构约束固化

### 3.1 清理源码树中的 `__pycache__`

现状：

- `pipeline/scheduler/src/scheduler` 和 `pipeline/scheduler/tests` 下存在大量 `__pycache__`。
- `git status` 当前干净，说明这些文件大概率被忽略，但目录扫描噪声明显。

方案：

1. 清理现有 `__pycache__`。
2. 在开发命令或文档中建议本地扫描前执行：

```bash
find pipeline/scheduler -type d -name __pycache__ -prune -exec rm -rf {} +
```

3. 可选：在 CI 静态检查前增加“源码目录不得包含 pycache”的检查。

验收标准：

- `find pipeline/scheduler/src pipeline/scheduler/tests -path '*/__pycache__/*' -type f | wc -l` 输出 `0`。

### 3.2 固化禁止模式测试

当前已有：

- 禁止 `defs/http` import `scheduler.defs.sources`。
- 禁止 EastMoney assets 使用 `globals()`。
- SourceBundle 唯一性与注册 definitions 对齐测试。

建议新增：

- 资产和 source service 不直接调用 `S3Config.from_env()`。
- `storage/` 不 import `sources/`。
- `repositories/` 不 import Dagster。
- `sources/*/assets.py` 中不直接构造数据库连接。
- 除 `config/env.py` 和 resource 默认值外，不直接使用 `dg.EnvVar`。

验收标准：

- 架构边界通过集成测试表达，而不是只写在文档里。
- 新增数据源如果破坏边界，会在单测阶段失败。

## 4. 优先级 P1：统一并发执行与 partial failure 模型

### 4.1 扩展 `BoundedTaskRunner`

现状：

- `common/concurrency.py` 已有 `BoundedTaskRunner`。
- JiuYan 图片下载和 OCR 已使用它。
- `http/partitioning.py`、BaoStock K 线、EastMoney F10 仍各自用 `asyncio.TaskGroup()` 管理任务和失败统计。

问题：

- partition、证券代码、图片等任务的执行模式相同：限流、逐项执行、收集成功/失败、生成 metadata。
- 失败阈值目前分散在 `PartialFailurePolicy`、partitioning 本地字段、OCR workflow 中。
- `BoundedTaskRunner` 当前没有 `fail_fast`、`max_failure_ratio`、结果顺序控制、失败样本截断等能力。

方案：

扩展 runner：

```python
@dataclass(frozen=True)
class BoundedTaskOptions:
    max_concurrent_tasks: int
    fail_fast: bool = False
    max_failure_ratio: float | None = None
    fail_when_all_failed: bool = True
    preserve_order: bool = False
```

输出统一结果：

```python
@dataclass(frozen=True)
class BoundedTaskResult[T]:
    successes: list[T]
    failures: list[TaskFailure]
    elapsed_seconds: float

    def metadata(self, *, item_name: str) -> dict[str, RawMetadataValue]: ...
```

迁移顺序：

1. 先迁移 `http/partitioning.py::materialize_partition_range()`。
2. 再迁移 `EastmoneyYearRefreshService.fetch_eastmoney_tables()`。
3. 最后迁移 `BaostockDailyKlineRefreshService.fetch_k_history_tables()`。

验收标准：

- 失败 metadata 字段统一包含 `failed_item_count`、`failed_items_sample`、`failed_item_errors_sample`、`max_concurrent_tasks`。
- OCR、HTTP daily partition、EastMoney、BaoStock 使用同一个 runner。
- `PartialFailurePolicy` 只在 runner 或 use case 边界调用，不散落在 asset 函数中。

### 4.2 明确 TaskGroup 失败语义

现状：

- BaoStock 和 EastMoney 使用 `TaskGroup()`；任一任务异常会取消同组任务。
- HTTP partitioning 捕获单 partition 失败并继续。
- OCR 捕获单图片失败并继续。

建议：

- 对“可部分失败”的数据源使用 runner 继续执行并按阈值失败。
- 对“必须全成功”的数据源使用 runner + `fail_fast=True`，而不是隐式依赖 `TaskGroup()` 行为。
- 每个资产的 metadata 标明 `partial_failure_policy`。

## 5. 优先级 P1：S3 Dataset Service 完整接管读写

现状：

- `S3IOManager` 写入和读取已通过 `S3DatasetService`。
- `http/partitioning.py` 写入也使用 `S3DatasetService`。
- 但业务服务仍直接调用 `read_parquet_table_from_s3()`、`read_partitioned_parquet_tables_from_s3()`。
- `daily_compact.py` 自己拼 compact metadata。

问题：

- `parquet_readers.py`、`dataset_service.py`、`daily_compact.py` 仍共同暴露 S3 layout 和读取细节。
- 读写 metadata 规则没有完全由同一个 builder 生成。
- 业务服务知道“读哪个 parquet object key”的细节，测试仍需要关心 S3 布局。

方案：

1. 将 `S3DatasetService.read_latest_snapshot()` 和 `read_partitioned()` 作为业务层唯一读取入口。
2. 把 `read_baostock_stock_basic_from_s3()`、`read_trade_dates_from_s3()` 这类 source-specific reader 改为小 gateway：

```python
class SecurityUniverseReader(Protocol):
    def read_stock_basic(self) -> pa.Table: ...

class TradeCalendarReader(Protocol):
    def read_trade_dates(self) -> set[date]: ...
```

3. `BaostockDailyKlineRefreshService`、`EastmoneyYearRefreshService` 不再接收裸 `S3Config`，改接收 reader/gateway。
4. `daily_compact.py` 使用 `S3DatasetService.read_partitioned()`，metadata 由 compact metadata builder 生成。

验收标准：

- source service 不 import `storage.parquet_readers`。
- S3 object key 计算只在 `storage/` 内部发生。
- compact asset metadata 与普通 partition metadata 字段命名一致。

## 6. 优先级 P1：HTTP/TCP client factory 资源化

现状：

- 通用 `HttpClientFactory` 已存在。
- Sina、JiuYan、THS 多处直接 `HttpClientFactory(retry_policy=DEFAULT_RETRY_POLICY)`。
- EastMoney client 内部构造自己的 `HttpClientFactory`。
- BaoStock service 曾直接构造多连接 TCP client；当前实现已改为通过 `BaostockClientFactoryResource` 注入，日频 K 线固定单连接顺序复用。

问题：

- timeout、connector limit、retry、request delay、TCP 连接数等运行参数分散。
- 测试虽然可以传 fake client，但生产配置不易从 Dagster resource 层统一观察。
- EastMoney 当前用资产依赖表达外部 API 限流顺序，这会污染 lineage 语义。

方案：

1. 新增 `HttpClientFactoryResource`，集中默认 retry/timeout/connector limit。
2. 新增 `BaostockClientFactory` 或 `BaostockClientSettingsResource`。
3. EastMoney 的 endpoint 串行策略优先用 Dagster pool、source client concurrency 或 schedule 分批表达；资产依赖只保留真实数据依赖 `baostock__query_stock_basic`。
4. `generated_endpoint_metadata(ordering_dependency=...)` 迁移为 `rate_limit_policy` metadata。

验收标准：

- `rg "HttpClientFactory\\(" pipeline/scheduler/src/scheduler/defs/sources` 仅保留 factory/resource 适配层。
- EastMoney 资产之间不再因为限流互相依赖，除非确有数据依赖。
- `dg list defs --json` 中 EastMoney lineage 不再呈现虚假的 endpoint 链式依赖。

## 7. 优先级 P1：Repository 生命周期与批量 SQL 优化

现状：

- `PostgresIndustryImageRepository` 每个 public method 内部 `psycopg.connect()`。
- `*_many()` 方法虽然批量接收 updates，但内部仍逐条 `cursor.execute()`。
- `IndustryImageRecord.download_status`、`ocr_status` 仍是 `str`。
- `ImageWorkflowStatus` 已存在，但未用于 repository dataclass。

问题：

- 高频 OCR 状态更新会产生重复连接开销。
- 批量 SQL 语义和事务边界不够明确。
- 状态字段缺少类型约束，状态机规则依赖 SQL 字符串。

方案：

1. 引入 repository connection factory 或 psycopg pool resource。
2. `mark_download_success_many()`、`mark_download_failed_many()`、`mark_ocr_success_many()`、`mark_ocr_failed_many()` 使用 `executemany()` 或 `with cursor.executemany(...)` 等批量模式。
3. 将状态字段改为 `StrEnum`：

```python
class DownloadStatus(StrEnum):
    PENDING = "pending"
    SUCCESS = "success"
    FAILED = "failed"

class OcrStatus(StrEnum):
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"
```

4. row mapper 统一负责 `str -> enum` 转换，非法状态直接报错。
5. 将单条 update 方法降级为 convenience wrapper，核心 API 只维护 many/batch。

验收标准：

- repository public dataclass 不再暴露状态裸字符串。
- 单次 OCR 批次的 DB 更新连接数固定且可测。
- 状态迁移测试覆盖 pending/running/success/failed 与 stale running claim。

## 8. 优先级 P2：Dagster asset 边界继续瘦身

现状：

- JiuYan asset 已经较薄。
- BaoStock、EastMoney asset 已把大流程放到 service。
- Sina trade calendar asset 仍包含 fetch、parse、metadata 拼装、日志和异常处理。
- 多个同步 asset 内部使用 `asyncio.run()`。

方案：

1. 为 Sina 新增 `SinaTradeCalendarRefreshService`。
2. 增加 `run_async_boundary()` helper，统一同步 asset 调用 async use case 的错误上下文。
3. asset 函数目标形态统一为：

```python
def asset_name(
    context: dg.AssetExecutionContext,
    config: AssetConfig,
    resource: SomeResource,
) -> dg.MaterializeResult[Payload]:
    result = use_case.refresh(...)
    return dg.MaterializeResult(value=result.value, metadata=result.metadata)
```

4. 所有复杂 metadata 拼装下沉到 result/builder，不在 asset 函数内散写。

验收标准：

- source asset 函数保持在 20-40 行以内。
- asset 文件只包含 Dagster decorator、config、thin adapter。
- 业务流程可在不构造 Dagster context 的情况下单测。

## 9. 优先级 P2：Metadata contract 与观测字段收敛

现状：

- `asset_contracts.py` 管理静态 metadata。
- `S3DatasetService.metadata()` 生成 S3 写入 metadata。
- `http/partitioning.py`、BaoStock、EastMoney、JiuYan workflow 分别拼运行 metadata。

问题：

- `row_count`、`column_count`、`file_format`、`compression`、`partition_keys` 等字段分散。
- 不同资产的失败统计字段名称不完全一致。
- metadata 的 schema 没有测试级别的 contract。

方案：

1. 新增 metadata builder：
   - `DatasetMetadataBuilder`
   - `FetchStatsMetadataBuilder`
   - `PartitionRunMetadataBuilder`
   - `FailureMetadataBuilder`
2. 将字段分为四类：
   - storage：bucket、keys、format、compression、storage_mode。
   - shape：row_count、column_count、partition_row_counts。
   - execution：duration、concurrency、request_count、retry_count。
   - quality：failed_count、empty_count、unknown_field_count、partial_failure_policy。
3. 集成测试验证所有 source asset 至少满足 storage/shape/execution 的基础 contract。

验收标准：

- `row_count`、`column_count`、`failed_*` 字段命名统一。
- metadata contract 变更需要同步测试。
- Dagster UI 中同类资产的观测字段可以横向比较。

## 10. 优先级 P2：类型检查与测试质量升级

现状：

- pyright 为 `basic`。
- 源码仍有必要但偏多的 `Any`，集中在 pyarrow、psycopg、测试 fake 和少量 service metadata。
- 测试已覆盖核心流程，但还可以更偏策略和 contract。

方案：

1. 保持全项目 `basic`，先对 `scheduler.defs.common`、`scheduler.defs.storage`、`scheduler.defs.partitioning` 局部提升到更严格规则。
2. 将 `dict[str, Any]` metadata 逐步替换为 `dict[str, RawMetadataValue]`。
3. 新增以下测试类型：
   - runner partial failure 策略测试。
   - dataset service read/write metadata contract 测试。
   - source service 不依赖 Dagster context 的纯单元测试。
   - import boundary 测试。
   - EastMoney lineage 不包含伪执行依赖测试。

验收标准：

- 新增共享抽象都有 focused unit tests。
- 测试不再通过过度 monkeypatch env/config 完成隔离。
- `Any` 的使用主要留在第三方库边界，并配合 `cast()` 或 Protocol 包住。

## 11. 建议实施顺序

### 阶段 A：低风险收敛

1. 清理 `__pycache__`。
2. 增加禁止模式测试。
3. 补充 source service 不直接读 env、不直接读 S3 object key 的约束测试。

### 阶段 B：共享执行模型

1. 扩展 `BoundedTaskRunner`。
2. 迁移 `http/partitioning.py`。
3. 迁移 JiuYan OCR/download metadata 到 runner metadata。
4. 再迁移 EastMoney 和 BaoStock。

### 阶段 C：存储 gateway

1. 扩展 `S3DatasetService` 读接口。
2. 引入 `SecurityUniverseReader`、`TradeCalendarReader`。
3. 改造 BaoStock、EastMoney、daily compact。

### 阶段 D：client/resource 与 lineage 修正

1. HTTP/TCP client factory resource 化。
2. EastMoney 从“链式资产依赖限流”迁移到 pool/concurrency/rate limit policy。
3. 更新 asset contract tests。

### 阶段 E：Repository 与状态机

1. 状态 enum 化。
2. connection factory/pool 化。
3. batch update 改造。
4. 补状态迁移测试。

## 12. 风险与取舍

- 不建议马上引入重量级框架或插件系统；当前 `SourceBundle + service + resource + gateway` 足够承接规模增长。
- 不建议为了消除所有 `Any` 而包装 pyarrow/psycopg 的每个细节；应只在项目边界处收敛。
- EastMoney 的执行限流从 asset dependency 中移走前，需要确认 Dagster pool、调度窗口和远端 API 限制足够表达当前运行约束。
- Repository pool 化会改变连接生命周期，需要配合集成测试或本地 PostgreSQL 验证。
- `asyncio.run()` 统一 helper 是短期治理；长期是否改 async assets 要结合 Dagster 版本能力和执行器行为单独评估。

## 13. 最终目标形态

下一阶段完成后，`pipeline/scheduler` 应达到以下状态：

- 每个数据源目录只暴露自己的 `SourceBundle`。
- Asset 函数薄，业务流程都在 service/use case 中。
- HTTP/TCP/S3/PostgreSQL 都通过 resource 或 gateway 注入。
- 并发执行、失败统计、partial failure 只有一套模型。
- S3 layout、读写、metadata 只有一个权威实现。
- Repository API 类型化，状态机显式化。
- Asset lineage 表达真实数据依赖，执行限流通过执行策略表达。
- 架构边界由测试保护，而不是依赖人工记忆。
