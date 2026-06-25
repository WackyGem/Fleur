# pipeline/scheduler 代码质量一刀切重构实施计划

日期：2026-05-30

关联设计文档：

- `docs/optimize/archive/scheduler-2026-05-30-quality-maintainability-optimization.md`

## 1. 目标

本计划用于指导 `pipeline/scheduler` 的长期代码质量重构。策略采用“一刀切”方案：不保留不必要的兼容 re-export，不为旧测试导入路径保留桥接层，不在新旧抽象之间长期双轨运行。

目标是一次性把主要架构问题收敛到新的长期形态：

- 模块边界唯一：每个 symbol 只有一个 canonical import path。
- Dagster asset 只保留边界适配职责，业务流程下沉到 service/use case。
- 环境配置只通过 resource、config model 或显式 gateway 注入，不在业务流程中隐式读取 env。
- S3 Parquet 写入、读取、object key、metadata 由统一 service 管理。
- async 并发、partial failure、状态更新使用统一 runner 和明确策略。
- Repository 公共 API 类型化，不再向业务层暴露 `dict[str, object]`。
- EastMoney 动态资产生成显式化，移除 `globals()` 动态导出和 compat re-export。
- Asset metadata、owners、kinds、tags 通过统一 contract builder 生成。
- 测试在代码质量问题基本解决后集中重写，避免边改架构边反复维护旧测试快照。

## 2. 执行原则

### 2.1 一刀切规则

本轮重构不做长期兼容层：

- 删除 `scheduler.defs.http.schedules` 这类 re-export 聚合模块。
- 删除 EastMoney `fields.py`、`schemas.py` compat re-export，或将其迁移为短期内部步骤后立即移除。
- 删除通过 `globals()` 注入的动态符号导出。
- 删除旧测试依赖的非 canonical import path。
- 删除不再需要的 helper、wrapper、fallback 和双轨实现。
- 不为了让旧大快照测试继续通过而保留旧 metadata 形态。

允许短期存在的过渡只限同一个重构分支内，最终落地前必须清掉。

### 2.2 测试策略

用户目标是先集中解决代码质量问题，再集中补测试，减少反复调整测试的成本。因此本计划采用测试冻结策略：

- 阶段 A 到阶段 E 不改或少改测试，除非测试文件阻塞 import 或阻塞 `dg check defs`。
- 阶段 A 到阶段 E 主要依赖静态质量门禁和 Dagster definitions 加载检查。
- 阶段 F 才集中重写测试结构、contract 测试和 policy 测试。

仍必须保留的最低验证：

- `uv run ruff check scheduler/src`
- `uv run pyright scheduler/src/scheduler`
- `cd pipeline/scheduler && uv run dg check defs`

说明：完全不跑任何检查会让大重构风险失控；这里冻结的是测试维护，不是放弃最小静态和 definitions 验证。

### 2.3 代码风格规则

- Python 版本按项目基线 `>=3.12`，使用现代类型语法。
- 复杂函数 5 个及以上参数时使用 keyword-only 参数。
- 外部库边界使用 `Protocol`，内部拥有实现的核心接口可使用 ABC 或 dataclass service。
- 普通业务分支优先显式前置条件，异常用于边界转换和附加上下文。
- 不新增 speculative abstraction；新增抽象必须承接本文列出的重复模式。
- 不新增嵌套 re-export；`__init__.py` 保持空或只保留 package docstring。

## 3. 总体重构顺序

本计划分 6 个阶段。

| 阶段 | 名称 | 是否改测试 | 主要产出 |
| --- | --- | --- | --- |
| A | 模块边界清理 | 否 | 删除 re-export、确立 canonical import path |
| B | Resource 与配置注入统一 | 否 | 移除业务路径中的 `from_env()` |
| C | S3 Dataset Service 统一 | 否 | 统一读写、metadata、object key |
| D | 并发 runner 与 Repository 类型化 | 否 | 统一 async 执行与状态流 |
| E | EastMoney 与 asset contract 一刀切规范 | 否 | 移除 `globals()`、统一 metadata/kinds/owners |
| F | 测试重建与最终验收 | 是 | 新 contract/policy 测试、删除旧快照测试 |

## 4. 阶段 A：模块边界清理

目标：先清掉最明显的边界问题，避免后续重构继续被旧导入路径牵制。

### A.1 删除 HTTP schedule re-export

操作：

1. 删除 `pipeline/scheduler/src/scheduler/defs/http/schedules.py`。
2. 所有 schedule 行为引用改为：
   - source definitions 模块内直接引用；或
   - 从 `SOURCE_BUNDLES` 查询；或
   - 顶层 definitions 加载后按 schedule name 查询。
3. `defs/http/` 只保留 HTTP client、protocol、pagination、partitioning、schema 相关代码。

完成标准：

- `rg "defs.http.schedules|from scheduler.defs.http import schedules" pipeline/scheduler/src` 无结果。
- `defs/http` 不再 import `scheduler.defs.sources.*`。

### A.2 清理 package re-export

操作：

1. 检查所有 `__init__.py`。
2. 删除非必要 `__all__`。
3. 删除 compat import path，只保留模块真实定义位置。

完成标准：

- 新代码中不再从 package `__init__.py` 间接导入业务 symbol。
- 每个核心 symbol 在文档中明确 canonical import path。

### A.3 固化模块边界说明

操作：

1. 在实施完成后更新 `AGENTS.md` 或新增 `docs/architecture/scheduler-module-boundaries.md`。
2. 明确禁止：
   - `defs/http` 聚合 source definitions。
   - storage 反向依赖 source 业务模块。
   - repositories 依赖 Dagster context。

阶段 A 最低检查：

```bash
cd pipeline
uv run ruff check scheduler/src
uv run pyright scheduler/src/scheduler
cd scheduler
uv run dg check defs
```

## 5. 阶段 B：Resource 与配置注入统一

目标：消除业务代码中的隐式环境读取，让依赖关系通过 Dagster resource、service constructor 或 gateway 显式传递。

### B.1 资产函数统一注入 `S3SettingsResource`

处理对象：

- `defs/baostock/assets.py`
- `defs/sources/eastmoney/assets.py`
- `defs/sources/daily_compact.py`
- `defs/http/partitioning.py` 的调用方
- `defs/sources/sina/trade_calendar.py`

操作：

1. 所有 asset 函数需要 S3 时，显式接收 `s3_settings: S3SettingsResource`。
2. Asset 函数中不再调用 `S3Config.from_env()`。
3. Service constructor 接收 `S3Config`，不在 service 内部读取 env。

完成标准：

- `rg "S3Config\\.from_env\\(" pipeline/scheduler/src/scheduler/defs/sources pipeline/scheduler/src/scheduler/defs/baostock pipeline/scheduler/src/scheduler/defs/http` 无业务路径结果。
- env 读取只存在于 `defs/config`、resource 默认值、少数 schedule factory adapter。

### B.2 schedule evaluation 引入 reader gateway

处理对象：

- `defs/market/schedules.py`

操作：

1. 新增 `TradeCalendarReader` Protocol。
2. 新增 `S3TradeCalendarReader`。
3. `build_trade_date_schedule()` 接收 `trade_calendar_reader_factory` 或 `trade_calendar_reader`。
4. 默认实现仍从 `S3SettingsResource` 等价配置构造，但具体读取逻辑不散落在 schedule closure 内。

建议形态：

```python
class TradeCalendarReader(Protocol):
    def read_trade_dates(self) -> set[date]: ...


@dataclass(frozen=True)
class S3TradeCalendarReader:
    s3_config: S3Config

    def read_trade_dates(self) -> set[date]:
        return read_trade_dates_from_s3(self.s3_config)
```

完成标准：

- schedule closure 只负责 schedule time 到 run request 的转换。
- trade calendar 读取失败只在 reader adapter 边界转换为 `SkipReason`。

### B.3 HTTP client factory resource

处理对象：

- Sina、JiuYan、THS、EastMoney、ObjectStore image download。

操作：

1. 新增 `HttpClientFactory` Protocol 或 dataclass。
2. 将 `AioHttpClient(...)` 的默认配置集中到 resource/factory。
3. 保留 source 层 header/request 构造，不把业务请求参数塞进通用 HTTP resource。

完成标准：

- source service 可传入 fake HTTP client 或 factory。
- HTTP timeout、connector limit、retry policy 的默认值集中管理。

阶段 B 最低检查：

```bash
cd pipeline
uv run ruff check scheduler/src
uv run pyright scheduler/src/scheduler
cd scheduler
uv run dg check defs
```

## 6. 阶段 C：S3 Dataset Service 统一

目标：统一 S3 layout、Parquet 写入、读取和 Dagster metadata 构造。

### C.1 新增 `S3DatasetService`

新增模块建议：

- `defs/storage/dataset_service.py`
- 或将现有 `dataset_writer.py` 扩展为 service 层，并保留 writer 为底层实现。

职责：

- asset key 到 base dir 的计算。
- latest snapshot 写入。
- partitioned 写入。
- sparse partition 写入。
- object key 截断。
- metadata 构造。
- latest snapshot 读取。
- partitioned dataset 读取。

建议对象：

```python
@dataclass(frozen=True)
class DatasetLocation:
    bucket: str
    object_prefix: str
    asset_key: dg.AssetKey


@dataclass(frozen=True)
class DatasetWriteOptions:
    storage_mode: str
    allow_empty: bool
    partition_key_name: str | None


class S3DatasetService:
    def write_latest_snapshot(
        self,
        location: DatasetLocation,
        table: pa.Table,
        options: DatasetWriteOptions,
    ) -> DatasetWriteResult: ...

    def write_partitioned(
        self,
        location: DatasetLocation,
        tables: Mapping[str, pa.Table],
        options: DatasetWriteOptions,
    ) -> DatasetWriteResult: ...
```

### C.2 IOManager 委托 service

处理对象：

- `defs/io_managers/s3_io_manager.py`

操作：

1. `handle_output()` 只负责从 Dagster context 解析 asset key、metadata、partition keys。
2. 表校验和写入委托 `S3DatasetService`。
3. output metadata 委托 service 统一生成。

完成标准：

- `S3IOManager` 不再手写 S3 metadata 字典。
- `S3IOManager` 不再直接调用 `S3DatasetWriter`。

### C.3 partitioning 委托 service

处理对象：

- `defs/http/partitioning.py`

操作：

1. `materialize_partition_range()` 保留 partition selection、并发抓取、partial failure 统计。
2. S3 写入和 metadata 交给 `S3DatasetService`。
3. `_asset_base_dir()`、`_path_to_object_key()` 等重复 helper 删除。

完成标准：

- `http/partitioning.py` 不再自行计算 S3 base dir。
- partitioning metadata 与 IOManager metadata 字段一致。

### C.4 ObjectStore 委托 service

处理对象：

- `defs/storage/object_store.py`

操作：

1. `ObjectStore.write_table()` 改为调用 `S3DatasetService.write_latest_snapshot()` 或专用 object table 写入方法。
2. 图片 bytes 存储继续保留 `ObjectStore.write_bytes()`。
3. OCR result table 的 base dir 规则移动到 image/object domain adapter，不留在通用 storage service。

### C.5 实现最小 `load_input()`

处理对象：

- `S3IOManager.load_input()`

操作：

1. 支持 latest snapshot asset 输入读取为 `pa.Table`。
2. 支持 partitioned asset 输入读取为 `dict[str, pa.Table]`，只覆盖当前项目需要的场景。
3. 对暂不支持的类型抛明确 `NotImplementedError`，不要静默 fallback。

完成标准：

- 常用上游读取逐步从 `parquet_readers.py` 收敛到 dataset service。
- `parquet_readers.py` 保留为 adapter 或被 service 替代。

阶段 C 最低检查：

```bash
cd pipeline
uv run ruff check scheduler/src
uv run pyright scheduler/src/scheduler
cd scheduler
uv run dg check defs
```

## 7. 阶段 D：并发 Runner 与 Repository 类型化

目标：把下载、OCR、分页抓取、TCP 分片抓取中的并发和失败处理收敛为统一模型，并让状态流有明确类型。

### D.1 新增 `BoundedTaskRunner`

新增模块建议：

- `defs/common/concurrency.py`
- 或 `defs/common/tasks.py`

职责：

- 按 concurrency 限制运行异步 worker。
- 收集成功项、失败项、错误类型、错误消息 sample。
- 支持 partial failure policy。
- 输出统一 timing metadata。

建议对象：

```python
@dataclass(frozen=True)
class TaskFailure:
    item_key: str
    error_type: str
    error_message: str


@dataclass(frozen=True)
class BoundedTaskResult[T]:
    successes: list[T]
    failures: list[TaskFailure]
    elapsed_seconds: float


class BoundedTaskRunner:
    async def run[T, R](
        self,
        items: Sequence[T],
        *,
        item_key: Callable[[T], str],
        worker: Callable[[T], Awaitable[R]],
    ) -> BoundedTaskResult[R]: ...
```

完成标准：

- JiuYan 下载和 OCR 请求使用同一个 runner。
- EastMoney/BaoStock 可先不立即迁移，但新 runner API 必须能覆盖它们的模式。

### D.2 Repository 输入输出类型化

处理对象：

- `defs/repositories/industry_images.py`
- `defs/sources/jiuyan/state_service.py`
- `defs/sources/jiuyan/ocr_schema.py`

操作：

1. 定义领域 dataclass：
   - `IndustryImageRecord`
   - `ExistingImageUrl`
   - `DownloadSuccessUpdate`
   - `DownloadFailureUpdate`
   - `OcrSuccessUpdate`
   - `OcrFailureUpdate`
   - `ClaimedIndustryImage`
2. Repository public methods 只接受/返回领域对象。
3. row dict 只存在于 repository 私有函数内部。
4. 状态字段使用 `Literal` 或 enum。

完成标准：

- `PostgresIndustryImageRepository.fetch_images()` 不再返回 `list[dict[str, object]]`。
- `mark_*_many()` 不再接受 `Sequence[Mapping[str, object]]`。
- `_required_int()` 等解析 helper 只在 row-to-domain 边界使用。

### D.3 同步 DB I/O 隔离

操作：

1. 短期保留同步 psycopg repository。
2. 从 async workflow 调用 repository 时，用 `asyncio.to_thread()` 包装批量方法。
3. 禁止在单个 async worker 内逐条同步写库。
4. 中长期再评估 async psycopg 或连接池，不在本轮强制引入。

完成标准：

- OCR/download worker 内不直接调用同步 repository。
- 所有状态更新先聚合为 batch，再统一 flush。

### D.4 失败策略集中

新增对象：

- `PartialFailurePolicy`
- `FailureThreshold`

适用：

- HTTP sparse partition。
- OCR。
- 图片下载。
- 未来 EastMoney/BaoStock 分片抓取。

完成标准：

- “全部失败则失败”“失败率超过 20% 则失败”等规则不散落在 workflow 中。
- metadata 中统一输出 failure count、failure ratio、failure sample。

阶段 D 最低检查：

```bash
cd pipeline
uv run ruff check scheduler/src
uv run pyright scheduler/src/scheduler
cd scheduler
uv run dg check defs
```

## 8. 阶段 E：EastMoney 与 Asset Contract 规范化

目标：清理最后一批长期代码品味问题，完成 asset 级治理标准。

### E.1 EastMoney 动态资产显式化

处理对象：

- `defs/sources/eastmoney/assets.py`
- `defs/sources/eastmoney/definitions.py`

操作：

1. 删除 `globals()[_endpoint.asset_name] = _eastmoney_asset`。
2. 新增 `build_eastmoney_assets()`。
3. `eastmoney_bundle` 通过 factory 返回资产集合。
4. 如需按名称索引，保留 `EASTMONEY_ASSETS_BY_NAME`，但只作为 dict，不注入模块全局 symbol。

完成标准：

- `rg "globals\\(" pipeline/scheduler/src/scheduler/defs/sources/eastmoney` 无结果。
- Asset key 和 job/schedule name 不变。

### E.2 EastMoney ordering dependency 策略化

操作：

1. 新增 `SequentialEndpointExecutionPolicy` 或等价小函数。
2. metadata 中明确记录 ordering dependency 原因：外部接口限流。
3. 避免把执行顺序伪装成业务数据依赖。

完成标准：

- ordering dependency 构造逻辑集中。
- 代码注释说明这是 rate-limit execution policy。

### E.3 删除 generated compat re-export

处理对象：

- `defs/sources/eastmoney/fields.py`
- `defs/sources/eastmoney/schemas.py`

操作：

1. 新代码统一从：
   - `defs/sources/eastmoney/generated/fields.py`
   - `defs/sources/eastmoney/generated/schemas.py`
   - 或 `defs/sources/eastmoney/schema.py`
   导入。
2. 删除 compat re-export 文件。
3. 更新生成脚本 README，说明 canonical path。

完成标准：

- `rg "sources.eastmoney import fields|sources.eastmoney import schemas|compat" pipeline/scheduler/src` 无结果。

### E.4 Asset contract 标准化

处理对象：

- `defs/asset_contracts.py`
- 所有 `@dg.asset`

操作：

1. 将 metadata key 抽为常量：
   - `storage_mode`
   - `partition_key_name`
   - `allow_empty`
   - `state_backend`
   - `object_store`
   - `external_service`
   - `execution_ordering_dependency`
2. 新增 metadata builder：
   - `latest_snapshot_metadata()`
   - `partitioned_metadata()`
   - `sparse_partition_metadata()`
   - `stateful_asset_metadata()`
   - `generated_endpoint_metadata()`
3. 新增 owner/kind helper：
   - `source_owners()`
   - `s3_parquet_kinds()`
   - `stateful_ocr_kinds()`
4. 所有 asset 明确 description 或 docstring。

完成标准：

- asset metadata 不再散落手写字典。
- OCR/stateful asset 不再 metadata 为空。
- 所有 source asset 有统一 owner、kinds、tags。

阶段 E 最低检查：

```bash
cd pipeline
uv run ruff check scheduler/src
uv run pyright scheduler/src/scheduler
cd scheduler
uv run dg check defs
```

## 9. 阶段 F：测试重建与最终验收

目标：在架构形态稳定后集中修改测试，避免每个阶段都维护旧快照。

### F.1 删除旧大快照测试

处理对象：

- `tests/integration/test_definitions_and_schedules.py`

操作：

1. 删除 asset dependency、metadata、jobs、schedules 的全量大字典断言。
2. 保留最小顶层 definitions 测试：
   - definitions 可加载。
   - bundle 合并结果与 loaded defs 一致。
   - asset/job/schedule/resource name 全局唯一。

### F.2 新增 source contract 测试

新增文件建议：

- `tests/integration/sources/test_sina_contract.py`
- `tests/integration/sources/test_jiuyan_contract.py`
- `tests/integration/sources/test_ths_contract.py`
- `tests/integration/sources/test_baostock_contract.py`
- `tests/integration/sources/test_eastmoney_contract.py`

每个 source 测试只验证自己的：

- assets。
- jobs。
- schedules。
- key prefix。
- dependency contract。
- partition contract。

### F.3 新增 policy 测试

新增文件建议：

- `tests/integration/test_asset_contract_policy.py`
- `tests/unit/storage/test_dataset_service.py`
- `tests/unit/common/test_bounded_task_runner.py`
- `tests/unit/repositories/test_industry_image_repository_types.py`

policy 覆盖：

- 所有 S3 parquet asset 必须有 `storage_mode`。
- partitioned asset 必须有 `partition_key_name`。
- stateful asset 必须有 `state_backend`。
- source asset 必须有 owner、kinds、source tag。
- `defs/http` 不得 import `defs/sources`。
- EastMoney 不得使用 `globals()`。
- generated schema 可通过脚本稳定性检查覆盖。

### F.4 更新 fake 与 helper

操作：

1. 根据新的 Protocol/dataclass 更新 fakes。
2. 删除旧 monkeypatch `S3Config.from_env()` 的测试写法。
3. fake 只实现生产 interface 需要的方法，不新增 speculative 参数。

### F.5 最终全量质量门禁

最终必须执行：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

如果涉及 Dagster definitions 行为变化，额外执行：

```bash
cd pipeline/scheduler
uv run dg list defs --json
```

## 10. 建议实施批次

虽然目标是一刀切，仍建议按 commit 批次组织，避免单次 diff 无法 review。

### Commit 1：边界删除

- 删除 `http/schedules.py`。
- 删除 package re-export。
- 调整源码导入路径。
- 不改测试或只做必要 import 修复。

### Commit 2：resource 注入

- 资产函数接收 resource。
- service constructor 显式接收 config/gateway。
- schedule 引入 trade calendar reader。

### Commit 3：S3DatasetService

- 新增 service。
- IOManager 委托 service。
- partitioning 委托 service。
- ObjectStore table 写入委托 service。

### Commit 4：并发与状态流

- 新增 bounded runner。
- OCR/download 迁移 runner。
- repository API dataclass 化。
- async 中同步 DB I/O 隔离。

### Commit 5：EastMoney 与 asset contract

- 移除 `globals()`。
- 删除 compat generated re-export。
- ordering dependency 策略化。
- metadata/owners/kinds 标准化。

### Commit 6：测试重建

- 删除大快照测试。
- 新增 source contract。
- 新增 policy 测试。
- 更新 fakes。
- 跑全量质量门禁。

## 11. 回滚策略

本计划是大重构，但每个 commit 仍应保持可回滚。

回滚原则：

- 如果阶段 A/B 失败，直接回滚对应 commit。
- 如果阶段 C S3 行为不确定，不继续阶段 D；先比对 object key 和 metadata。
- 如果阶段 D 状态流风险过高，保留 dataclass API，暂缓 async DB 改造。
- 如果阶段 E EastMoney asset key 发生变化，立即停止并恢复旧 key 生成逻辑。
- 测试重建阶段发现行为差异时，优先检查生产代码 contract，而不是把测试改到通过。

## 12. 完成定义

本轮重构完成必须满足：

- `defs/http` 不再聚合 source definitions。
- 业务路径中不再隐式读取 `S3Config.from_env()`。
- S3 写入 metadata 只有一个权威 builder。
- `S3IOManager.load_input()` 至少支持项目内主要读取场景，或有明确替代的 `S3DatasetService` 读取入口。
- OCR/download 并发使用统一 runner。
- Repository 公共 API 不暴露裸 `dict[str, object]`。
- EastMoney 不使用 `globals()` 动态导出。
- EastMoney generated compat re-export 被删除或冻结为非生产依赖。
- 所有 source asset 有统一 metadata、owners、kinds、tags。
- 旧 definitions 大快照测试已替换为 source contract 和 policy 测试。
- 全量质量门禁通过。

## 13. 不纳入本轮的事项

以下事项暂不纳入本轮，避免扩大范围：

- 更换 Dagster 版本。
- 引入新的数据库迁移框架。
- 改变 S3 object key layout 的业务语义。
- 改变现有 asset key、job name、schedule name。
- 重写 BaoStock TCP 协议实现。
- 改变 EastMoney endpoint 列表。
- 引入复杂 plugin 系统。

如果重构过程中发现必须改变 asset key 或 S3 layout，应单独写迁移 RFC，不在本计划中顺手修改。
