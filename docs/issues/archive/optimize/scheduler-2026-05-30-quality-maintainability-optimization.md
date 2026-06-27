# pipeline/scheduler 代码质量与可维护性优化设计

日期：2026-05-30

## 1. 扫描范围与结论

本次扫描范围为 `pipeline/scheduler`，重点查看：

- Dagster definitions、assets、jobs、schedules 的装配方式。
- HTTP/TCP 数据源实现，包括 Sina、JiuYan、THS、BaoStock、EastMoney。
- S3 Parquet 写入、ObjectStore、IOManager、Repository、OCR 状态流。
- 类型、异常、测试结构、模块边界、生成代码边界与可维护性风险。

当前工程不是“坏代码”状态，整体已经有比较清晰的分层：

- `SourceBundle` 已经把各数据源的 assets/jobs/schedules 从顶层聚合中拆开。
- `S3DatasetWriter`、`S3IOManager`、`ObjectStore`、`ImageObjectStore` 已经把部分存储职责抽出。
- `JiuyanIndustryImageWorkflow`、`JiuyanIndustryOcrWorkflow`、`BaostockDailyKlineRefreshService`、`EastmoneyYearRefreshService` 已经开始把 Dagster asset 边界和业务流程分离。
- EastMoney 大体量 schema 已移动到 `generated/`，手写逻辑和生成代码边界比早期设计更明确。
- 质量门禁当前通过：
  - `uv run ruff check scheduler/src scheduler/tests`
  - `uv run pyright scheduler/src/scheduler scheduler/tests`
  - `uv run dg check defs`

因此本文重点不是修复可运行性，而是面向资产继续增长、回填窗口扩大、OCR 并发提高、更多数据源接入后的架构演进。

## 2. 当前架构摘要

源码当前约 90 个非 generated Python 文件，测试约 30 个 Python 文件。核心目录职责如下：

| 目录 | 当前职责 | 评价 |
| --- | --- | --- |
| `defs/definitions.py` | 顶层装配 bundles 与 resources | 已较清晰 |
| `defs/source_bundle.py` | SourceBundle 聚合抽象 | 合适，可继续加强 contract |
| `defs/automation/` | 通用 job/schedule 工厂 | 合适 |
| `defs/market/` | A 股交易日、证券范围、市场调度 | 合适，但 schedule 读取 S3 的依赖注入还弱 |
| `defs/http/` | HTTP client、schema、partitioning、legacy schedule 聚合 | 职责偏宽，`http/schedules.py` 应收敛 |
| `defs/storage/` | S3、Parquet、dataset writer、readers | 基础良好，但读写抽象还未完全统一 |
| `defs/io_managers/` | S3IOManager | 写入可用，读取未实现 |
| `defs/repositories/` | PostgreSQL 状态 repository | SQL 集中，但同步连接和批量边界可优化 |
| `defs/sources/*` | 数据源业务逻辑 | 基本按 source 分包，少量通用模式重复 |
| `defs/baostock/` | BaoStock TCP 数据源 | 边界正确，没有混入 HTTP 调度 |

## 3. 需要优先处理的问题

### 3.1 `http/schedules.py` 仍是跨数据源 re-export 聚合层

现状：

- `scheduler.defs.http.schedules` 从 EastMoney、JiuYan、Sina、THS 的 definitions 模块导入并 re-export job/schedule。
- `tests/integration/test_definitions_and_schedules.py` 仍通过 `from scheduler.defs.http import schedules` 使用这些 re-export。
- 项目指南已经明确：`http/schedules.py` 只组装 HTTP 数据源具体 job/schedule，不定义或 re-export 通用工厂；更理想的状态是各数据源 job/schedule 由自己的 `definitions.py` 暴露，顶层通过 `SourceBundle` 合并。

问题：

- 同一个 schedule 有两个导入路径，降低定位性。
- 新数据源容易误把聚合职责放回 `http/`。
- 测试依赖 re-export，会固化不推荐边界。

改进方案：

1. 将集成测试改为从 `scheduler.defs.sources.<source>.definitions` 或 `SOURCE_BUNDLES` 获取 schedule。
2. 删除或清空 `defs/http/schedules.py`；如果短期需要兼容，先标记为 deprecated 并只保留一轮迁移。
3. 新增边界测试：禁止 `defs/http/schedules.py` import `scheduler.defs.sources.*`。

验收标准：

- `rg "scheduler.defs.http import schedules|defs.http.schedules" pipeline/scheduler` 只剩迁移说明或完全为空。
- `uv run dg check defs` 通过。
- integration 测试仍覆盖 schedule 行为，但不依赖 re-export。

### 3.2 配置和 resource 注入不一致

现状：

- 部分流程已经使用 resource，例如 OCR asset 通过 `S3SettingsResource`、`ImageObjectStoreResource`、`IndustryImageRepositoryResource`、`JiuyanOcrSettingsResource` 注入。
- 仍有不少服务或 helper 直接调用 `S3Config.from_env()`：
  - `baostock/assets.py`
  - `http/partitioning.py`
  - `sources/daily_compact.py`
  - `sources/eastmoney/assets.py`
  - `market/schedules.py`
  - `sources/sina/trade_calendar.py` 间接构造 HTTP client。

问题：

- 测试需要 monkeypatch `S3Config.from_env()`，而不是替换明确的 resource 或 service。
- Dagster UI 看不到这些隐式环境依赖。
- 一部分 config 生命周期由 resource 管，一部分由业务函数临时读取，定位故障时路径不统一。

改进方案：

1. 先把资产函数中的 `S3Config.from_env()` 改为通过 `s3_settings: S3SettingsResource` 注入。
2. 对 schedule evaluation 这类 Dagster 不直接注入 resource 的场景，新增小型 factory：
   - `TradeCalendarReader`
   - `S3TradeCalendarReader`
   - `build_trade_date_schedule(..., trade_calendar_reader=...)`
3. Service 构造函数只接收显式 config 或 gateway，不再内部读取 env。

建议目标形态：

```python
class TradeCalendarReader(Protocol):
    def read_trade_dates(self) -> set[date]: ...


@dataclass(frozen=True)
class S3TradeCalendarReader:
    s3_config: S3Config

    def read_trade_dates(self) -> set[date]:
        return read_trade_dates_from_s3(self.s3_config)
```

验收标准：

- 资产模块内不再直接调用 `S3Config.from_env()`。
- 单测不再 monkeypatch `S3Config.from_env()`，而是传 fake reader/resource/service。
- 所有 env 读取集中在 `defs/config` 和 Dagster resource 默认值。

### 3.3 S3 读写抽象仍然分叉

现状：

- 普通 latest snapshot/year partition 通过 `S3IOManager.handle_output()` 写入。
- `http/partitioning.py::materialize_partition_range()` 为 sparse daily partition 直接构造 `S3DatasetWriter` 并拼装 metadata。
- `ObjectStore.write_table()` 直接调用 `write_parquet_dataset()`。
- 上游读取通过 `storage/parquet_readers.py`，`S3IOManager.load_input()` 仍是 `NotImplementedError`。

问题：

- object key 规则、metadata 字段、空表策略、partition row count 等逻辑仍分散在 IOManager、partitioning、ObjectStore 中。
- 如果将来修改 S3 layout、压缩方式、metadata 命名，很容易漏改。
- Dagster asset 间的数据读取绕过 IOManager，导致资产依赖和实际读取机制不完全统一。

改进方案：

1. 提取 `S3DatasetService`，统一：
   - base dir 计算
   - latest snapshot 写入
   - partitioned 写入
   - object key 截断
   - metadata 构造
   - optional read latest/partition
2. `S3IOManager`、`materialize_partition_range()`、`ImageObjectStore.write_ocr_result_table()` 都委托该 service。
3. 第二阶段再实现 `S3IOManager.load_input()`，优先支持 `pa.Table` latest snapshot 和 partition mapping 两种明确形态。

建议接口：

```python
@dataclass(frozen=True)
class DatasetLocation:
    bucket: str
    object_prefix: str
    asset_key: dg.AssetKey
    storage_mode: str


class S3DatasetService:
    def write_latest_snapshot(self, location: DatasetLocation, table: pa.Table) -> DatasetWriteResult: ...
    def write_partitioned(self, location: DatasetLocation, tables: Mapping[str, pa.Table], partition_key_name: str) -> DatasetWriteResult: ...
    def metadata_for(self, result: DatasetWriteResult, *, storage_mode: str, allow_empty: bool) -> dict[str, RawMetadataValue]: ...
```

验收标准：

- S3 metadata 字段只有一个 builder。
- IOManager 和 sparse partition 写入复用同一个 service。
- `partition_column_count()`、object key 截断逻辑不再重复。

### 3.4 异步流程里混用同步阻塞 I/O

现状：

- `ocr_services.py` 在 async 下载/OCR 任务完成后批量调用 state service，已经比每张图同步写库更好。
- 但 repository 本身仍是同步 `psycopg.connect()`，且 service 方法被 async workflow 直接 `await` 包装调用。
- BaoStock、EastMoney 使用 `asyncio.TaskGroup()` 并发远端请求，然后同步进行 pyarrow 表构造，这部分合理；但失败策略、并发统计、批量 flush 模式各自实现。

问题：

- 如果 repository 批量更新耗时上升，会阻塞 event loop。
- 下载、OCR、EastMoney、BaoStock 都有“并发执行一组任务、收集成功失败、生成 metadata”的相似结构，但没有统一 runner。
- 广泛的 `except Exception` 捕获当前多用于隔离单个 partition/image 失败，意图合理，但错误分类和阈值策略还分散。

改进方案：

1. 对 sync repository 调用做明确边界：
   - 短期使用 `asyncio.to_thread()` 包裹批量 DB 写入。
   - 中期引入 psycopg async connection 或 connection pool。
2. 提取 `BoundedTaskRunner`：
   - 输入 item 列表、concurrency、worker。
   - 输出 successes、failures、duration、failure rate。
   - 支持 `fail_fast`、`allow_partial`、`max_failure_ratio` 策略。
3. 将 `ocr/service.py::run_bounded_ocr_batch()` 与 `sources/jiuyan/ocr_services.py` 的并发逻辑合并，避免两套模型。

验收标准：

- OCR 下载和 OCR 请求使用同一个 bounded runner。
- partial failure metadata 格式统一，包含 item key、error type、message sample。
- DB 状态更新明确在 async 边界之外或通过 `to_thread()` 执行。

### 3.5 Repository 返回 `dict[str, object]`，领域类型边界不够清晰

现状：

- `PostgresIndustryImageRepository.fetch_images()` 返回 `list[dict[str, object]]`。
- claim 路径会转换为 `ClaimedIndustryImage`，但其他查询仍由调用方解释 row 字段。
- update 方法接收 `Sequence[Mapping[str, object]]`，内部再做 `str(update["..."])` 和 `_required_int()`。

问题：

- 字段名和状态值缺乏编译期约束。
- 更新 payload 的合法字段由约定保证，调用方传错 key 时只会在运行时报错。
- 状态流规则散落在 SQL 和 service 层。

改进方案：

1. 为 repository 输入输出定义 dataclass：
   - `IndustryImageRecord`
   - `DownloadSuccessUpdate`
   - `DownloadFailureUpdate`
   - `OcrSuccessUpdate`
   - `OcrFailureUpdate`
2. Repository 只接受这些领域对象，不接受裸 `Mapping[str, object]`。
3. 状态值使用 `Literal` 或 enum：
   - `DownloadStatus = Literal["pending", "success", "failed"]`
   - `OcrStatus = Literal["pending", "running", "success", "failed"]`
4. SQL 层保持集中，但 row-to-domain 转换放在私有函数中统一处理。

验收标准：

- `repositories/industry_images.py` 公共方法不再暴露 `dict[str, object]`。
- 状态 service 单测覆盖 pending/running/success/failed 转换。
- `pyright` 能检查 update payload 字段。

### 3.6 EastMoney 动态资产生成仍有可读性成本

现状：

- `sources/eastmoney/assets.py` 通过 `ENDPOINT_CONFIGS` 循环构造资产。
- 用 `globals()[_endpoint.asset_name] = _eastmoney_asset` 暴露动态资产符号。
- 为限制外部接口压力，资产之间用 ordering dependency 串联。

优点：

- endpoint 增减集中在 config。
- 避免 8 个资产函数重复。

问题：

- `globals()` 是隐式符号注册，新读者需要理解 Dagster asset decorator 和 Python 模块导出行为。
- ordering dependency 看起来像数据依赖，但实际更偏执行限流策略。
- 兼容 re-export 文件 `fields.py`、`schemas.py` 还保留 `__all__`，与项目“一个 canonical import path”的长期目标不完全一致。

改进方案：

1. 用显式 bundle factory 替代模块级 `globals()`：

```python
def build_eastmoney_assets() -> list[dg.AssetsDefinition]:
    previous: dg.AssetsDefinition | None = None
    assets: list[dg.AssetsDefinition] = []
    for endpoint in ENDPOINT_CONFIGS:
        asset = build_eastmoney_asset(endpoint, ordering_dependency=previous)
        assets.append(asset)
        previous = asset
    return assets
```

2. 如确实需要按名称索引，保留 `EASTMONEY_ASSETS_BY_NAME`，但不写入 `globals()`。
3. 将 ordering dependency 命名为策略，例如 `SequentialEndpointExecutionPolicy`，并在 metadata 注明原因：外部接口限流，不是业务数据依赖。
4. 退出 generated 兼容模块：
   - EastMoney 新代码统一从 `schema.py` 或 `scheduler.defs.contract_schemas` 获取 schema/field facts。
   - 测试不再验证 compat re-export，改为验证 contract boundary 稳定性。

验收标准：

- `assets.py` 中不再出现 `globals()`。
- EastMoney 顺序依赖有单独策略名和测试。
- generated schema 的测试关注“生成输入到输出稳定”，而不是兼容 re-export。

### 3.7 测试结构仍有大快照断言

现状：

- `tests/integration/test_definitions_and_schedules.py` 对所有 asset dependency、metadata、jobs、schedules 做大字典断言。
- 这种测试防回归能力强，但新增资产时改动面积大。

问题：

- 失败 diff 很长，定位成本高。
- 单个 source 的变化会迫使维护顶层全量快照。
- 容易鼓励测试导入聚合 re-export，例如当前的 `http.schedules`。

改进方案：

保留一个轻量顶层测试，只验证：

- definitions 能加载。
- asset key/job/schedule/resource 全局唯一。
- `SOURCE_BUNDLES` 合并结果和 loaded defs 一致。

将细节拆到 source 级 contract：

- `tests/integration/sources/test_jiuyan_contract.py`
- `tests/integration/sources/test_eastmoney_contract.py`
- `tests/integration/sources/test_baostock_contract.py`

再新增 policy 测试：

- 所有 S3 source asset 必须有 `storage_mode` 或明确声明非 parquet asset。
- partitioned asset 必须有 `partition_key_name`。
- OCR/状态类 asset 必须声明 `kinds` 和 `owners`。

验收标准：

- 顶层 integration 文件长度显著降低。
- 新增一个数据源只需新增/修改该 source 的 contract 测试。
- policy 测试能自动发现 metadata 缺失，而不是靠全量快照。

### 3.8 Asset metadata、owners、kinds 标准还不完整

现状：

- `asset_contracts.py` 已经提供 source tags、metadata builder 等基础能力。
- 部分 asset 有 `owners`、`kinds`、description，例如 OCR 相关资产。
- 仍有一些 asset metadata 为空或不完整，例如 `jiuyan__industry_images`、`jiuyan__industry_ocr` 的 definition metadata 在集成测试中为空。

问题：

- Dagster UI 中检索和治理能力不一致。
- contract 测试对 metadata 字面量依赖较重。
- 新资产容易遗漏 owners/kinds。

改进方案：

1. 扩展 `asset_contracts.py`：
   - metadata key 常量。
   - `latest_snapshot_metadata()`、`partitioned_metadata()`、`stateful_asset_metadata()`。
   - owners/kinds 标准函数。
2. 为非 parquet 资产明确 metadata：
   - `state_backend="postgres"`
   - `object_store="s3"`
   - `external_service="ocr"` 或 `http`
3. 写 policy 测试约束：
   - 所有 asset 必须有 description/docstring。
   - source asset 必须有 source tag。
   - stateful asset 必须声明 state backend。

验收标准：

- `asset_contracts.py` 成为 asset metadata 的唯一入口。
- 集成测试不再复制 `storage_mode` 等字符串字面量。
- Dagster UI 中 asset kinds/owners 覆盖一致。

## 4. 分阶段实施计划

### 阶段 1：边界收敛和测试去耦

目标：低风险清理，减少未来重构阻力。

任务：

1. 移除 `http/schedules.py` re-export 使用。
2. 拆分 `test_definitions_and_schedules.py` 中的 source contract。
3. 新增模块边界测试，禁止 `defs/http` 反向依赖 `defs/sources` 的聚合职责。
4. 为 EastMoney ordering dependency 增加策略注释和测试命名。

建议先做这一阶段，因为它不改变运行行为。

### 阶段 2：配置和存储 service 统一

目标：让资源、S3 layout、metadata 的权威路径唯一。

任务：

1. 资产函数通过 `S3SettingsResource` 获取 S3 config。
2. 新增 `S3DatasetService`，统一 IOManager、partitioning、ObjectStore 的写入 metadata。
3. 将 `materialize_partition_range()` 的 S3 写入委托给 service。
4. 评估并实现 `S3IOManager.load_input()` 的最小可用版本。

注意：

- 这一阶段要重点跑 storage、http partitioning、definitions 测试。
- 不建议同时改 S3 object key layout；先保持行为一致。

### 阶段 3：并发任务和状态流重构

目标：提高 OCR、下载、批量抓取流程的可观测性和并发稳定性。

任务：

1. 提取 `BoundedTaskRunner`。
2. OCR 下载和 OCR 请求共用 runner。
3. Repository 更新 payload 改为领域 dataclass。
4. 同步 DB 调用从 async worker 中隔离，短期使用 `asyncio.to_thread()`，中期评估 async psycopg 或连接池。

### 阶段 4：生成代码与动态资产进一步规范

目标：降低 EastMoney endpoint 扩展和 review 成本。

任务：

1. 移除 `globals()` 动态导出。
2. 用 `build_eastmoney_assets()` 或 `build_eastmoney_bundle()` 显式返回资产集合。
3. 增加生成脚本稳定性测试。
4. 移除或冻结 compat re-export 模块。

## 5. 推荐质量门禁

每个阶段完成后至少执行：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

对阶段 2 和阶段 3，建议额外定向运行：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/storage scheduler/tests/unit/http scheduler/tests/unit/sources/jiuyan
```

## 6. 风险与取舍

| 风险 | 说明 | 缓解 |
| --- | --- | --- |
| 抽象过度 | 当前数据源数量还不算很大，过早引入复杂 class hierarchy 会增加理解成本 | 优先使用 dataclass、Protocol、小函数，避免深继承 |
| S3 行为变更 | 统一 writer 时可能改变 object key 或 metadata | 第一阶段只委托，不改 layout；用现有测试锁定行为 |
| OCR 状态迁移复杂 | 状态流涉及 PostgreSQL、S3、远端 OCR，失败边界多 | 先类型化 payload，再改并发 runner，最后考虑 async DB |
| 测试拆分期间漏掉 contract | 大快照拆散后可能降低覆盖 | 先新增 source contract，再删除全量断言 |
| EastMoney 动态资产变更影响 Dagster key | 移除 `globals()` 不应改变 asset key，但可能影响测试导入路径 | 先从 tests 中停止按动态全局变量导入，再调整实现 |

## 7. 优先级清单

P0：

- 去除 `http/schedules.py` re-export 使用。
- 拆分 definitions 大快照测试。
- 新增模块边界测试。

P1：

- `S3Config.from_env()` 从资产和业务服务中迁出，改为 resource/factory 注入。
- 提取 `S3DatasetService`，统一 metadata 与 object key 处理。
- Repository 公共 API 类型化，减少 `dict[str, object]`。

P2：

- 统一 bounded async task runner。
- OCR/下载状态更新隔离同步 DB I/O。
- EastMoney 移除 `globals()` 动态导出。
- generated schema 增加生成稳定性测试。

## 8. 最小下一步建议

建议下一步从 P0 开始，提交一个行为不变的小 PR：

1. 修改 schedule 行为测试，直接从 source bundle 或 source definitions 获取目标 schedule。
2. 删除 `scheduler.defs.http.schedules` 的 re-export 依赖。
3. 增加一个边界测试，防止 `defs/http/schedules.py` 再次聚合 source definitions。

这一步改动小，但能消除当前最明确的模块边界违背点，为后续 resource 和 storage service 重构减少耦合。
