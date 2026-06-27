# pipeline/scheduler 架构与代码质量优化设计

## 背景

本次扫描范围为 `pipeline/scheduler`，重点覆盖 Dagster definitions、资产定义、调度工厂、HTTP/TCP 数据源、S3 Parquet 写入、OCR 状态流、Repository 与测试结构。

当前基线较健康：

- `uv run ruff check scheduler/src scheduler/tests` 通过
- `uv run pyright scheduler/src/scheduler scheduler/tests` 通过
- `uv run dg check defs` 通过
- `uv run dg list defs --json` 可正常加载 definitions

因此本文不把问题定位为“代码不可用”，而是面向后续数据源继续增加、资产数量增长、回填压力上升、OCR 状态流复杂化之后的维护性优化。

## 当前架构概览

源码约 73 个 Python 文件，测试约 29 个 Python 文件。主要分层如下：

- `defs/definitions.py`：集中装配所有 assets、jobs、schedules、resources。
- `defs/automation/`：通用 job/schedule 工厂。
- `defs/market/`：A 股交易日、证券范围、市场调度。
- `defs/http/`：HTTP 客户端、分区物化、schema 与 pagination。
- `defs/storage/` 与 `defs/io_managers/`：S3、Parquet、对象存储与 Dagster IOManager。
- `defs/sources/`：Sina、JiuYan、THS、EastMoney 等 HTTP 数据源。
- `defs/baostock/`：BaoStock TCP 数据源。
- `defs/repositories/`：PostgreSQL 状态 Repository。

Dagster 当前注册了 18 个 assets、9 个 jobs、7 个 schedules、1 个 resource，并启用了默认 automation condition sensor。

## 主要优化方向

### 1. definitions 装配从手工列表改为 SourceBundle 注册

现状：

- `defs/definitions.py` 手工维护所有 assets/jobs/schedules。
- `defs/http/schedules.py` 同时聚合多个 HTTP 数据源的 job/schedule。
- 新增一个数据源时，需要在资产文件、schedule 文件、definitions 文件、集成测试大字典里多处同步修改。

建议：

引入轻量注册对象，例如：

```python
from dataclasses import dataclass, field
from collections.abc import Sequence
import dagster as dg

@dataclass(frozen=True)
class SourceBundle:
    name: str
    assets: Sequence[dg.AssetsDefinition] = field(default_factory=tuple)
    jobs: Sequence[dg.UnresolvedAssetJobDefinition | dg.JobDefinition] = field(default_factory=tuple)
    schedules: Sequence[dg.ScheduleDefinition] = field(default_factory=tuple)
```

每个数据源提供自己的 `bundle`：

- `defs/sources/sina/definitions.py`
- `defs/sources/jiuyan/definitions.py`
- `defs/sources/ths/definitions.py`
- `defs/sources/eastmoney/definitions.py`
- `defs/baostock/definitions.py`

顶层只负责合并 bundles：

```python
SOURCE_BUNDLES = (
    sina_bundle,
    jiuyan_bundle,
    ths_bundle,
    baostock_bundle,
    eastmoney_bundle,
)
```

收益：

- 新增数据源时只改该数据源目录。
- `http/schedules.py` 不再承载跨数据源聚合职责，只保留 HTTP 调度 helper 或彻底移除。
- definitions 集成测试可以从“完整硬编码列表”改成“bundle 契约 + 全局唯一性”。

### 2. Dagster 资产边界瘦身，业务流程下沉到 Use Case / Service

现状：

- `baostock/assets.py`、`sources/eastmoney/assets.py`、`sources/jiuyan/industry_ocr.py` 中，资产函数同时做环境配置读取、上游 S3 读取、远端请求、状态写入、表转换和 metadata 拼装。
- 这类函数可测试性还可以，但职责偏多，后续调试和复用成本会升高。

建议：

资产函数保留 Dagster 边界职责：

- 读取 Dagster context/config。
- 调用一个明确的 service/use case。
- 返回 `MaterializeResult`。

业务流程下沉，例如：

- `BaostockDailyKlineRefreshService`
- `EastmoneyYearRefreshService`
- `MarketEventPartitionRefreshService`
- `JiuyanIndustryImageWorkflow`
- `JiuyanIndustryOcrWorkflow`

典型资产函数目标形态：

```python
def baostock__query_history_k_data_plus_daily(
    context: dg.AssetExecutionContext,
    config: KLineDailyYearConfig,
    service: BaostockDailyKlineRefreshService,
) -> dg.MaterializeResult[dict[str, pa.Table]]:
    result = service.refresh(context.partition_keys, config)
    return dg.MaterializeResult(value=result.tables, metadata=result.metadata)
```

收益：

- Dagster 适配层与业务逻辑分离。
- service 可用普通单元测试覆盖，不依赖 fake Dagster context。
- 后续 CLI、一次性修复脚本或回填工具可复用同一套流程。

### 3. 将配置、存储、Repository 抽成 Dagster resources

现状：

- 多个资产内部直接调用 `S3Config.from_env()`、`PipelineDatabaseConfig.from_env()`、`JiuyanOcrConfig.from_env()`。
- `PostgresIndustryImageRepository(database_config.url)`、`ImageObjectStore.from_s3_config(s3_config)` 在资产内部即时构造。
- 测试常通过 monkeypatch `from_env` 或替换模块函数完成隔离。

建议：

引入 ConfigurableResource：

- `S3SettingsResource`
- `ObjectStoreResource`
- `IndustryImageRepositoryResource`
- `JiuyanOcrSettingsResource`
- 可选：`HttpClientFactoryResource`

短期可以先保持 `S3IOManager` 不变，只把显式读取路径和 OCR workflow 所需依赖资源化。

收益：

- 环境变量读取集中，启动时失败更明确。
- 测试可以替换 resource，不需要 monkeypatch 模块级函数。
- Dagster UI 中资源配置和资产依赖更清晰。

### 4. 统一 S3 Parquet 写入路径，避免 IOManager 与手写分区写入分叉

现状：

- 常规 snapshot/year partition 通过 `S3IOManager.handle_output()` 写入。
- `http/partitioning.py::materialize_partition_range()` 为了支持 sparse daily partition，直接调用 `write_parquet_dataset()` 写 S3，并自己拼 metadata。
- `daily_compact.py` 又通过读取 helper 手动聚合分区。

问题：

- object key 规则、metadata 字段、空表策略、column count 逻辑分散。
- IOManager 没有 `load_input()`，资产读上游时又绕过 IOManager 使用 reader helper。

建议：

新增共享写入服务：

- `S3DatasetWriter`
- `PartitionedDatasetWriter`
- `SparsePartitionWriter`
- `S3DatasetReadService`

`S3IOManager` 与 `materialize_partition_range()` 都委托同一套 writer。是否实现 `load_input()` 可以分两步：

1. 先统一写入与 metadata。
2. 再评估是否让 Dagster input loading 接管部分读取。

收益：

- S3 layout 规则只有一个权威实现。
- sparse partition 与普通 partition 的差异变成显式策略，而不是两套写入代码。
- 后续修改压缩、文件命名、metadata 字段时不容易漏改。

### 5. OCR 状态流需要专门的状态服务与批量写入策略

现状：

- `sources/jiuyan/ocr_services.py` 在 async 并发任务中调用同步 psycopg repository。
- 每张图片成功/失败都会打开连接并执行一次状态更新。
- `defs/ocr/service.py::run_bounded_ocr_batch()` 已有通用并发 helper，但 JiuYan OCR 主流程没有复用。
- `_image_mime_type()`、下载并发常量等存在重复或未使用痕迹。

风险：

- 同步 DB 调用会阻塞 event loop，OCR 或下载并发提高后会放大。
- 每图一次连接的成本较高，状态更新失败和远端请求失败的边界不够清晰。
- 通用 OCR helper 与实际业务流程割裂，后续容易形成两套并发模型。

建议：

新增 `IndustryImageStateService`：

- claim OCR 图片。
- 批量标记下载成功/失败。
- 批量标记 OCR 成功/失败。
- 对状态迁移定义明确枚举：`pending/running/success/failed`。

数据库访问策略二选一：

- 短期：保留同步 psycopg，但在 async worker 中通过 `asyncio.to_thread()` 包住状态写入，且使用批量 flush。
- 中期：引入 psycopg async connection 或 connection pool，由 repository 显式管理生命周期。

同时把 `run_bounded_ocr_batch()` 泛化为 `run_bounded_tasks()`，让下载、OCR、EastMoney/BaoStock 分片抓取都能复用统一的并发与失败统计模型。

### 6. 分区与回填策略抽象为显式 Policy

现状：

- `http/partitioning.py` 同时处理自然日、交易日过滤、回填窗口、并发、失败收集、S3 写入。
- BaoStock 和 EastMoney 分别有自己的 year range 构造逻辑。
- `build_year_refresh_schedule()` 与 `build_trade_date_schedule()` 的 run_config/tag 规则分散在不同模块。

建议：

引入策略对象：

- `PartitionSelectionPolicy`
- `TradeDateFilterPolicy`
- `BackfillLimitPolicy`
- `PartialFailurePolicy`
- `RunConfigPolicy`

先不做复杂继承，使用 frozen dataclass + 小函数即可。

收益：

- “哪些 partition 会处理、哪些会跳过、失败是否允许继续”可单独测试。
- BaoStock、EastMoney、JiuYan、THS 的差异被表达为配置，而不是复制流程。
- 回填限制和失败阈值能在 metadata 中统一呈现。

### 7. typed Dagster config 替换旧式 `config_schema`

现状：

- 多数资产已经使用 `dg.Config`。
- `jiuyan__industry_images` 与 `jiuyan__industry_ocr` 仍使用 `config_schema`，资产函数参数 `context` 未显式标注。
- 内部又通过 `_images_asset_config(context.op_config)` 和 `_ocr_asset_config(context.op_config)` 做二次解析。

建议：

改为：

```python
class IndustryImagesConfig(dg.Config):
    limit: int | None = None
    force_download: bool = False
    image_filenames: list[str] = []

class IndustryOcrConfig(dg.Config):
    limit: int | None = None
    force_ocr: bool = False
    image_filenames: list[str] = []
    max_concurrent_requests: int | None = None
```

并在 config 内或 service 入参处统一处理 `0` 与 `None` 的语义。

收益：

- 类型检查能覆盖资产 config。
- 删除手写解析函数，减少“默认值在 schema 和 parser 两处定义”的重复。
- Dagster UI 配置结构更清晰。

### 8. HTTP client 可进一步压缩重复分支并提升错误上下文

现状：

- `AioHttpClient._send_once()` 中 text/bytes 分支的 status 处理逻辑重复。
- `request_json_object()` 调用 `_request_with_retries(decode_json=True)` 后又 `json.loads()` 一次，JSON 解析发生两遍。
- HTTP stats 已有基础字段，但缺少按 endpoint/status 的聚合维度。

建议：

- 抽出 `_raise_for_status(status, body_preview)`。
- 引入 decoder：`TextDecoder`、`BytesDecoder`、`JsonObjectDecoder`。
- JSON 请求只解析一次。
- stats 中可选记录 `status_code_counts`、`endpoint_host_counts`，metadata 输出时截断。

收益：

- HTTP 错误处理路径更短。
- 新增 XML、CSV、图片等响应类型时不需要继续扩展大分支。
- 排查远端接口波动时 metadata 更有用。

### 9. EastMoney 动态资产与生成 schema 需要更明确的生成边界

现状：

- `sources/eastmoney/assets.py` 通过 endpoint config 动态生成 assets，并通过 `globals()[asset_name]` 暴露。
- `fields.py` 与 `schemas.py` 是大体量生成文件，单文件超过 1500 行。
- EastMoney 资产之间用链式 ordering dependency 限制执行顺序。

建议：

- 将动态注册封装为 `build_eastmoney_bundle()`，由 bundle 显式返回 assets/job/schedule。
- 生成文件移动到更明确的 `sources/eastmoney/generated/`，保留生成脚本、输入来源和校验测试。
- 对 generated schema 增加稳定性测试：运行生成脚本后 diff 为空。
- 对 ordering dependency 增加注释或 policy 名称，例如 `SequentialEndpointPolicy`，说明是为了外部接口限流而非数据依赖。

收益：

- 动态资产行为更容易理解和测试。
- generated 与手写业务逻辑分离。
- 未来 endpoint 增减时审查成本降低。

### 10. Asset metadata、owners、kinds 与描述标准化

现状：

- 资产 tags 基本统一，但 owners/kinds 为空。
- `jiuyan__industry_images`、`jiuyan__industry_ocr` 在 `dg list defs` 输出中没有 description。
- metadata key 字符串散落在多个模块，例如 `storage_mode`、`partition_key_name`、`allow_empty`、`flatten_column_naming`。

建议：

新增 `defs/asset_contracts.py` 或 `defs/common/assets.py`：

- tag 常量：`source/layer/storage/state/modality`
- metadata key 常量
- 常用 metadata builder：`latest_snapshot_metadata()`、`partitioned_metadata()`
- owner/kind 标准：如 `owners=["team:data-platform"]`、`kinds={"s3", "parquet", "postgres", "http"}`

收益：

- Dagster UI 可检索性更好。
- contract 测试能引用常量，不再复制字面量。
- 新资产更容易符合项目规范。

### 11. 测试结构从“大快照断言”转向契约与表驱动

现状：

- `tests/integration/test_definitions_and_schedules.py` 对所有资产依赖、metadata、jobs、schedules 做完整字典断言。
- 这种测试能防回归，但新增资产时改动范围大，且失败信息不总能指向真正问题。

建议：

拆成几类测试：

- definitions 全局唯一性：asset key/job/schedule/resource 不重复。
- bundle 契约测试：每个 source bundle 声明自己的资产、job、schedule。
- metadata policy 测试：所有 S3 asset 必须有 storage metadata。
- schedule behavior 测试：交易日跳过、year refresh run_config。
- generated schema 稳定性测试。

收益：

- 新增数据源时只补该数据源测试。
- 顶层 integration 测试更稳定。
- contract 失败能更快定位到具体 source。

## 建议目标结构

```text
scheduler/defs/
├── definitions.py
├── asset_contracts.py
├── resources/
│   ├── s3.py
│   ├── database.py
│   ├── http.py
│   └── ocr.py
├── storage/
│   ├── dataset_writer.py
│   ├── dataset_reader.py
│   └── object_store.py
├── partitioning/
│   ├── policies.py
│   ├── trade_dates.py
│   └── materialization.py
├── sources/
│   ├── sina/definitions.py
│   ├── jiuyan/definitions.py
│   ├── ths/definitions.py
│   └── eastmoney/
│       ├── definitions.py
│       ├── services.py
│       └── generated/
└── baostock/
    ├── definitions.py
    └── services.py
```

这不是一次性重构目标，而是后续迭代的归宿。短期可以保持现有目录，只先引入 bundle、resource 和共享 writer。

## 分阶段落地计划

### 第 1 阶段：低风险清理

- 为 `jiuyan__industry_images`、`jiuyan__industry_ocr` 增加 description 和显式类型标注。
- 删除或复用重复的 `_image_mime_type()`、重复并发常量。
- 将 OCR 资产 config 改为 `dg.Config`。
- 把 metadata key/tag 常量集中到一个模块。
- 将 `defs/ocr/service.py` 改名为更通用的 bounded task helper，或并入实际 OCR workflow。

验收：

- `ruff`、`pyright`、`pytest scheduler/tests/unit/sources/jiuyan` 通过。
- `dg list defs --json` 中 OCR 两个资产有 description。

### 第 2 阶段：definitions bundle 化

- 定义 `SourceBundle`。
- 先迁移 Sina、THS、JiuYan 三个 HTTP 数据源。
- 再迁移 BaoStock 与 EastMoney。
- 顶层 `defs/definitions.py` 改为合并 bundles。
- 重写 definitions 集成测试为 bundle 契约测试。

验收：

- 新增或删除单个 source asset 时，只需要改该 source bundle 和对应测试。
- `dg check defs` 通过。
- `dg list defs --json` 资产、job、schedule 数量与迁移前一致。

### 第 3 阶段：资源注入与 service 化

- 引入 S3/ObjectStore/Repository/OCR settings resources。
- 把 `S3Config.from_env()` 等环境读取从核心业务函数中移出。
- 为 BaoStock、EastMoney、JiuYan OCR 建立 use case service。
- 测试从 monkeypatch env/function 转向 fake resource。

验收：

- 核心 service 单元测试无需 Dagster context。
- 资产函数平均长度明显下降，只做边界适配。
- 关键 workflow 的失败路径测试覆盖到 service 层。

### 第 4 阶段：统一分区写入与回填策略

- 新增共享 `S3DatasetWriter`。
- `S3IOManager` 与 `materialize_partition_range()` 共用 writer。
- 引入 `PartitionSelectionPolicy`、`BackfillLimitPolicy`、`PartialFailurePolicy`。
- 对 sparse partition 写入增加 contract 测试。

验收：

- S3 object key、metadata、空表策略只有一套测试。
- daily sparse partition 与 yearly partition 的写入行为可用同一测试矩阵描述。

### 第 5 阶段：OCR 状态流强化

- 引入 `IndustryImageStateService`。
- 支持批量状态更新或连接池。
- 明确 claim/running/success/failed 状态迁移。
- 将同步 DB 写入从 event loop 中隔离。
- 将失败率阈值配置化，并输出更完整 metadata。

验收：

- OCR 并发提高时 DB 连接数可控。
- 单张图片失败不会影响状态一致性。
- 全失败、部分失败、状态抢占、stale running 重新 claim 都有测试。

## 优先级建议

优先级 P0：

- OCR 状态流的同步 DB 写入与连接策略。
- S3 写入路径统一。
- SourceBundle 注册，降低新增数据源修改面。

优先级 P1：

- Dagster resources 注入。
- typed config 替换 `config_schema`。
- partition/backfill policy 抽象。

优先级 P2：

- HTTP client decoder 重构。
- generated schema 目录调整。
- owners/kinds/metadata 标准化。

## 不建议立即做的事

- 不建议一次性大规模移动所有目录。当前质量门禁通过，大搬迁会制造无业务收益的回归风险。
- 不建议引入复杂继承层级。当前场景更适合 dataclass policy、小型 service 和组合。
- 不建议为了“纯 Dagster IOManager”立刻删除显式 reader。先统一 writer 和 metadata，再决定是否实现 `load_input()`。

## 结论

`pipeline/scheduler` 当前已经具备较清晰的分层和较好的静态质量。下一阶段最值得投入的是降低新增数据源的装配成本、把 Dagster 边界与业务流程分离、统一 S3 分区写入、强化 OCR 状态流的并发与事务模型。这些优化可以分阶段落地，不需要破坏当前可运行状态。
