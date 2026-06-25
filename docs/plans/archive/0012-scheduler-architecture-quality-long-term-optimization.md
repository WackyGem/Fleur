# Plan 0012: Scheduler 架构质量长期优化实施计划

状态：已实施

计划日期：2026-05-30

关联文档：

- `docs/optimize/archive/scheduler-architecture-quality-optimization.md`
- `docs/plans/0011-backfill-window-optimization.md`
- `docs/RFC/0005-scheduler-resource-refactor-and-trade-date-backfill.md`
- `docs/RFC/0006-pipeline-code-quality-and-reusability.md`

参考资料：

- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/http/partitioning.py`
- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py`
- `pipeline/scheduler/src/scheduler/defs/sources/jiuyan/industry_ocr.py`
- `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py`
- Dagster 当前文档：Context7 `/dagster-io/dagster`

## 目标

按照 `scheduler-architecture-quality-optimization.md` 的长期优化方向，逐步把 `pipeline/scheduler` 从“中心化装配 + 资产内聚合业务流程”的结构，演进为：

1. 以数据源为边界的 `SourceBundle` 注册体系。
2. Dagster asset 只保留编排边界，业务流程下沉到 service/use case。
3. 配置、对象存储、Repository、OCR settings 等依赖通过 Dagster resource 注入。
4. S3 parquet 写入路径统一到共享 writer，IOManager 与 sparse partition 写入不再分叉。
5. OCR 图片状态流拥有专门状态服务、批量写入策略和明确状态迁移。
6. 分区选择、交易日过滤、回填限制、失败策略抽象为可测试 policy。
7. typed `dg.Config`、asset metadata、owners、kinds、description 和测试契约标准化。

本计划优先走长期方案，但以小步迁移为原则：每个阶段都必须保持 Dagster definitions 可加载、现有 asset key/job/schedule 名称稳定、质量门禁可运行。

## 非目标

本计划不包含：

- 修改业务数据口径、S3 路径兼容性或已公开 asset key。
- 一次性移动所有 scheduler 目录。
- 引入复杂继承框架或 Dagster Component YAML 体系。
- 将所有上游读取立即改为 IOManager `load_input()`。
- 改造 dbt raw 层、ClickHouse 表结构或下游消费模型。
- 调整 production schedule cron，除非某阶段明确需要修复装配归属。

## 当前状态

当前 scheduler 基线较健康：

- `defs/definitions.py` 手工维护 assets、jobs、schedules、resources。
- `defs/http/schedules.py` 聚合多个 HTTP 数据源 schedule，职责越过 HTTP helper 边界。
- `jiuyan__industry_images` 和 `jiuyan__industry_ocr` 仍使用旧式 `config_schema`。
- 多个 asset 内直接调用 `S3Config.from_env()`、`PipelineDatabaseConfig.from_env()`、`JiuyanOcrConfig.from_env()`。
- `S3IOManager.handle_output()` 与 `http/partitioning.py::materialize_partition_range()` 各自写 S3 parquet。
- OCR workflow 在 async 并发流程里调用同步 psycopg repository，且状态更新粒度偏细。
- `test_definitions_and_schedules.py` 用大字典断言全量 definitions，新增数据源时改动面较大。

## 长期目标结构

目标目录形态如下。实施时可以分阶段引入，不要求一次性搬迁：

```text
scheduler/defs/
├── definitions.py
├── source_bundle.py
├── asset_contracts.py
├── resources/
│   ├── s3.py
│   ├── database.py
│   ├── http.py
│   └── ocr.py
├── storage/
│   ├── dataset_writer.py
│   ├── dataset_reader.py
│   ├── object_store.py
│   └── parquet.py
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

## 设计决策

### 决策 1：先采用轻量 SourceBundle，不引入 Dagster Component

选择：

```python
from collections.abc import Sequence
from dataclasses import dataclass, field

import dagster as dg


@dataclass(frozen=True)
class SourceBundle:
    name: str
    assets: Sequence[dg.AssetsDefinition] = field(default_factory=tuple)
    jobs: Sequence[dg.UnresolvedAssetJobDefinition | dg.JobDefinition] = field(
        default_factory=tuple
    )
    schedules: Sequence[dg.ScheduleDefinition] = field(default_factory=tuple)
```

理由：

1. 当前项目以 Python definitions 为主，没有 YAML Component 装配体系。
2. SourceBundle 能先解决装配分散和测试大字典问题。
3. Dagster Component 可作为后续更高阶的复用手段，不阻塞当前长期架构演进。

### 决策 2：asset key、job name、schedule name 保持稳定

所有迁移阶段必须保持现有对外标识稳定：

- asset key 不变，例如 `source/jiuyan__action_field`。
- job name 不变，例如 `jiuyan__action_field_daily_job`。
- schedule name 不变，例如 `jiuyan__action_field_daily_schedule`。
- resource key 迁移时保留兼容路径，尤其是 `s3_io_manager`。

理由：

1. 避免 Dagster 历史记录和下游依赖断裂。
2. 便于用 `dg list defs --json` 做迁移前后 diff。
3. 每阶段可独立验收。

### 决策 3：resource 化优先覆盖显式依赖，不立即替换 IOManager

优先新增：

- `S3SettingsResource`
- `ObjectStoreResource`
- `IndustryImageRepositoryResource`
- `JiuyanOcrSettingsResource`
- 可选 `HttpClientFactoryResource`

`S3IOManager` 初期继续保留现有配置字段与 resource key，等共享 writer 稳定后再决定是否拆分 settings。

理由：

1. OCR、repository、object store 是当前依赖分散最明显的位置。
2. 保持 IOManager 稳定可以降低 S3 写入兼容风险。
3. resource 替换后，测试可以从 monkeypatch 环境变量转向 fake resource。

### 决策 4：共享 writer 先统一写入和 metadata，再评估 load_input

新增 `S3DatasetWriter` 和 `PartitionedDatasetWriter`，由以下路径共同委托：

- `S3IOManager.handle_output()`
- `materialize_partition_range()`
- 后续 compact asset 写入 helper

本阶段不强制实现 `S3IOManager.load_input()`。

理由：

1. 当前最大分叉风险在写入 layout、metadata、空表策略、column count。
2. 上游读取路径改造涉及更多 asset 参数和测试，适合作为后续阶段。
3. 先统一 writer 能快速形成单一权威实现。

### 决策 5：OCR 状态服务采用“同步 repository + 线程隔离 + 批量 flush”作为第一步

短期选择：

- 保留 psycopg 同步 repository。
- 在 async worker 内用 `asyncio.to_thread()` 隔离 DB 写入。
- 引入批量状态 flush，减少每图一次连接/事务。
- 定义状态枚举和状态迁移 contract。

中期再评估：

- psycopg async connection。
- connection pool 生命周期由 resource 管理。

理由：

1. 第一阶段不额外引入 async DB 驱动复杂度。
2. 能先解决 event loop 阻塞和连接次数失控问题。
3. 状态服务接口稳定后，底层 repository 实现可替换。

### 决策 6：partition/backfill 使用 dataclass policy

新增 policy 时优先使用 frozen dataclass + 小函数：

- `PartitionSelectionPolicy`
- `TradeDateFilterPolicy`
- `BackfillLimitPolicy`
- `PartialFailurePolicy`
- `RunConfigPolicy`

理由：

1. 当前差异主要是配置和策略组合，不需要继承层级。
2. policy 可直接单元测试。
3. BaoStock、EastMoney、JiuYan、THS 的回填差异能显式表达。

## 实施阶段

### 阶段 0：迁移基线与保护网

目标：建立迁移前后可对比基线，避免长期重构时无意改变 Dagster contract。

实施内容：

1. 新增 definitions snapshot helper，输出 asset key、dependency key、job name、schedule name、resource key。
2. 新增迁移前后 diff 测试工具，允许测试比较“数量和名称不变”，而不是维护巨型 metadata 字典。
3. 记录当前 `dg list defs --json` 的关键统计：asset/job/schedule/resource 数量。
4. 将现有 `test_definitions_and_schedules.py` 拆分前先保留，不在本阶段删除。

验收：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

### 阶段 1：asset contract、typed config 与低风险清理

目标：先把 metadata/tag/config 规范集中，清理旧式 config，为后续 resource/service 迁移减小噪声。

实施内容：

1. 新增 `scheduler.defs.asset_contracts`：
   - tag 常量：`source`、`layer`、`storage`、`state`、`modality`。
   - metadata key 常量：`storage_mode`、`partition_key_name`、`allow_empty`、`flatten_column_naming`、`execution_ordering_dependency`。
   - 常用 metadata builder：latest snapshot、year partition、daily sparse partition、compact asset。
2. 为 OCR asset 增加 description、owners、kinds。
3. 将 `jiuyan__industry_images` 和 `jiuyan__industry_ocr` 从 `config_schema` 改为 typed `dg.Config`。
4. 删除或复用重复 helper，例如重复 MIME 判断、未使用并发常量。
5. 保持 asset 内部流程不大改，只完成边界类型与 contract 标准化。

验收：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/repositories scheduler/tests/unit/storage scheduler/tests/unit/http
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

### 阶段 2：SourceBundle 注册体系

目标：把顶层 definitions 从手工列表改为按数据源 bundle 合并。

实施内容：

1. 新增 `scheduler.defs.source_bundle.SourceBundle`。
2. 为每个数据源新增 `definitions.py`：
   - `sources/sina/definitions.py`
   - `sources/ths/definitions.py`
   - `sources/jiuyan/definitions.py`
   - `baostock/definitions.py`
   - `sources/eastmoney/definitions.py`
3. 将 compact asset/job 放入对应业务 bundle：
   - `jiuyan__action_field_compacted` 归入 JiuYan bundle。
   - `ths__limit_up_pool_compacted` 归入 THS bundle。
4. 将 `http/schedules.py` 从跨数据源聚合点降级：
   - 第一小步只保留兼容 re-export。
   - 第二小步把 schedule 组装移到各 source `definitions.py`。
   - 最终删除跨 source 聚合职责。
5. `defs/definitions.py` 只负责：
   - 声明 `SOURCE_BUNDLES`。
   - 合并 assets/jobs/schedules。
   - 注册 resources。

验收：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

迁移成功标准：

- `dg list defs --json` 中 asset/job/schedule 数量与迁移前一致。
- 单个 source 新增 asset 时，只需要修改该 source bundle 和对应测试。
- `http/schedules.py` 不再定义或 re-export 通用工厂。

### 阶段 3：definitions 测试重构为契约测试

目标：把“大快照断言”转为更稳定的契约测试。

实施内容：

1. 新增 bundle contract 测试：
   - 每个 bundle name 唯一。
   - bundle 内 asset key/job/schedule 无重复。
   - bundle 声明的 schedule 对应 job 存在。
2. 新增全局唯一性测试：
   - asset key 全局唯一。
   - job name 全局唯一。
   - schedule name 全局唯一。
   - resource key 全局唯一。
3. 新增 metadata policy 测试：
   - S3 asset 必须声明 storage metadata。
   - partitioned asset 必须声明 `partition_key_name`。
   - sparse daily asset 必须声明 trade calendar source。
4. 保留少量关键 dependency 测试：
   - trade calendar 是市场日级资产上游。
   - EastMoney ordering dependency 只表达限流顺序。
   - OCR asset 依赖 image asset。

验收：

```bash
cd pipeline
uv run pytest scheduler/tests/integration
uv run pytest scheduler/tests/unit
```

### 阶段 4：resource 注入与 JiuYan OCR service 化

目标：优先改造依赖最复杂、收益最高的 JiuYan OCR workflow。

实施内容：

1. 新增 resource：
   - `IndustryImageRepositoryResource`
   - `ImageObjectStoreResource`
   - `JiuyanOcrSettingsResource`
2. 新增 service/use case：
   - `JiuyanIndustryImageWorkflow`
   - `JiuyanIndustryOcrWorkflow`
   - `IndustryImageStateService`
3. asset 函数目标形态：
   - 读取 typed config。
   - 从 Dagster 注入 resource。
   - 调用 workflow。
   - 返回 `dg.MaterializeResult(metadata=...)`。
4. 将 `discover_images_from_table()`、download、OCR、metadata 拼装逐步迁入 workflow/service。
5. 测试迁移：
   - 用 fake repository/resource 替代 monkeypatch `from_env()`。
   - service 单元测试不依赖 fake Dagster context。

验收：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/repositories scheduler/tests/unit/sources/jiuyan scheduler/tests/integration
cd scheduler
uv run dg check defs
```

### 阶段 5：S3 parquet writer 统一

目标：消除 IOManager 与 sparse partition 手写 S3 写入的分叉。

实施内容：

1. 新增 `storage/dataset_writer.py`：
   - `S3DatasetWriter`
   - `DatasetWriteResult`
   - latest snapshot 写入。
   - partitioned dataset 写入。
   - metadata 构建。
2. `S3IOManager.handle_output()` 委托 writer。
3. `http/partitioning.py::materialize_partition_range()` 委托 writer。
4. 保留现有 object key layout 和 compression。
5. 增加 writer contract 测试：
   - latest snapshot。
   - year partition。
   - daily sparse partition。
   - 空表允许/拒绝策略。
   - column count 和 partition row counts。

验收：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/storage scheduler/tests/unit/http
uv run pytest scheduler/tests/integration
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

### 阶段 6：partition/backfill policy 抽象

目标：把回填窗口、交易日过滤、失败策略从 `http/partitioning.py` 的流程函数中拆出。

实施内容：

1. 新增 `defs/partitioning/policies.py`：
   - `PartitionSelectionPolicy`
   - `TradeDateFilterPolicy`
   - `BackfillLimitPolicy`
   - `PartialFailurePolicy`
2. 将 `materialize_partition_range()` 拆分为：
   - partition 选择。
   - 并发 fetch。
   - writer 写入。
   - metadata 汇总。
3. 将 `materialize_trade_date_range()` 的交易日过滤和窗口限制改为 policy 组合。
4. BaoStock/EastMoney year range 构造逻辑逐步复用 policy。
5. metadata 中统一输出：
   - requested/processed/skipped/completed/failed count。
   - skipped window sample。
   - policy name 或 policy 参数。

验收：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/http/test_market_event_partitioning_and_schemas.py
uv run pytest scheduler/tests/unit/baostock
uv run pytest scheduler/tests/integration
uv run pyright scheduler/src/scheduler scheduler/tests
```

### 阶段 7：OCR 状态流强化

目标：解决 OCR 并发流程中的同步 DB 阻塞、状态迁移不集中、每图一次状态更新成本高的问题。

实施内容：

1. 定义状态枚举：
   - download: `pending/running/success/failed`
   - ocr: `pending/running/success/failed`
2. `IndustryImageStateService` 负责：
   - claim OCR 图片。
   - stale running 重新 claim。
   - 批量标记下载成功/失败。
   - 批量标记 OCR 成功/失败。
   - 失败原因截断和分类。
3. async worker 中状态写入通过 `asyncio.to_thread()` 包装。
4. 增加批量 flush 策略：
   - 按 batch size flush。
   - workflow 结束时 final flush。
5. 将 `run_bounded_ocr_batch()` 泛化为 `run_bounded_tasks()`，统一并发、错误收集和统计模型。
6. 下载、OCR、后续可选 EastMoney/BaoStock 分片抓取复用 bounded task helper。

验收：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/repositories
uv run pytest scheduler/tests/unit/sources/jiuyan
uv run pytest scheduler/tests/integration
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
```

新增测试场景：

- 全部成功。
- 部分下载失败。
- 部分 OCR 失败。
- 全失败。
- stale running 被重新 claim。
- force OCR 覆盖 success。
- 状态 flush 失败时 workflow metadata 能暴露错误。

### 阶段 8：BaoStock 与 EastMoney service 化

目标：将较重资产函数的业务流程下沉，复用 resource、writer 和 policy。

实施内容：

1. BaoStock：
   - 新增 `BaostockStockBasicRefreshService`。
   - 新增 `BaostockDailyKlineRefreshService`。
   - year range 和交易日过滤使用 policy。
2. EastMoney：
   - 新增 `EastmoneyYearRefreshService`。
   - 动态 asset 生成封装为 `build_eastmoney_bundle()`。
   - ordering dependency 命名为 `SequentialEndpointPolicy` 或等价常量，并添加注释。
3. asset 函数只做 Dagster 边界适配。
4. service 层单元测试覆盖 API 空返回、部分失败、schema 转换失败、分区 metadata。

验收：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/baostock scheduler/tests/unit/http scheduler/tests/integration
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

### 阶段 9：HTTP client decoder 与错误上下文

目标：压缩 HTTP client 重复分支，提升远端接口问题的可观测性。

实施内容：

1. 抽出 `_raise_for_status(status, body_preview)`。
2. 引入 response decoder：
   - text。
   - bytes。
   - JSON object。
3. `request_json_object()` 只解析 JSON 一次。
4. HTTP stats 增加可截断聚合：
   - `status_code_counts`
   - `endpoint_host_counts`
   - retry/error counts
5. 更新 HTTP client 单元测试，覆盖 status error body preview、JSON decode error、bytes 响应。

验收：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/http/test_client.py
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
```

### 阶段 10：EastMoney generated 边界

目标：让动态生成 schema 与手写业务逻辑边界更明确。

实施内容：

1. 新建 `sources/eastmoney/generated/`。
2. 移动生成文件：
   - `fields.py`
   - `schemas.py`
3. 保留或新增生成脚本、输入来源说明、校验入口。
4. 新增 generated stability 测试：
   - 运行生成脚本后 diff 为空，或至少校验 endpoint/schema 数量和字段列表稳定。
5. 更新导入路径，保留兼容 re-export 一段时间，避免大范围一次性改动。

验收：

```bash
cd pipeline
uv run pytest scheduler/tests/unit scheduler/tests/integration
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
```

## 推荐实施顺序

推荐按以下 PR 或工作批次推进：

1. `0012-0-baseline-contracts`：阶段 0。
2. `0012-1-asset-contracts-typed-config`：阶段 1。
3. `0012-2-source-bundles`：阶段 2。
4. `0012-3-definition-contract-tests`：阶段 3。
5. `0012-4-jiuyan-resources-services`：阶段 4。
6. `0012-5-s3-dataset-writer`：阶段 5。
7. `0012-6-partition-policies`：阶段 6。
8. `0012-7-ocr-state-service`：阶段 7。
9. `0012-8-baostock-eastmoney-services`：阶段 8。
10. `0012-9-http-eastmoney-generated`：阶段 9 和阶段 10。

如果需要压缩批次，阶段 2 和阶段 3 可以合并；阶段 5 和阶段 6 可以合并。但阶段 4 与阶段 7 不建议合并，因为 OCR 资源注入和状态流语义变更都需要独立验证。

## 风险与控制

### 风险 1：definitions contract 被无意改变

控制：

- 阶段 0 先建立 asset/job/schedule/resource diff 工具。
- 每个阶段都运行 `dg check defs` 和 `dg list defs --json`。
- 保持 asset key、job name、schedule name 不变。

### 风险 2：S3 写入兼容性回归

控制：

- writer 迁移前新增 object key layout 测试。
- `S3IOManager` 与 sparse writer 迁移到同一 writer 后，先保留旧 metadata 字段。
- 不在同一阶段修改 compression、文件命名或 object prefix。

### 风险 3：OCR 状态语义改变导致重复处理或漏处理

控制：

- 先建立状态枚举和状态迁移测试。
- claim、success、failed、stale running 均用 repository/service 测试覆盖。
- 批量 flush 要保证 workflow 结束时 final flush。

### 风险 4：resource 注入影响测试可读性

控制：

- 为 resource 定义窄接口和 fake 实现。
- service 单元测试不依赖 Dagster context。
- asset 测试只覆盖 Dagster config/resource 适配。

## 最终验收

完成本计划后，需要通过完整质量门禁：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
uv run dg list defs --json
```

最终交付状态：

- 顶层 `defs/definitions.py` 只做 bundle 合并和 resource 注册。
- 每个数据源目录拥有自己的 `definitions.py` 或等价 bundle 入口。
- OCR、BaoStock、EastMoney 的主要业务流程在 service/use case 中，可脱离 Dagster context 测试。
- S3 parquet 写入由共享 writer 统一实现。
- 分区、回填、失败策略可单独测试。
- definitions 测试以 bundle contract 和 policy contract 为主，不再依赖巨型全量字典。
