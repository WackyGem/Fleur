# Plan 0008: RFC 0006 Pipeline 代码质量与可复用性一体化改造实施计划

状态：草案

计划日期：2026-05-29

关联 RFC：

- `docs/RFC/archive/0006-pipeline-code-quality-and-reusability.md`

参考资料：

- `docs/plans/0006-scheduler-engineering-quality-assessment-and-optimization.md`
- `docs/plans/0007-scheduler-rfc0005-refactor-implementation.md`
- `docs/RFC/archive/0005-scheduler-resource-refactor-and-trade-date-backfill.md`
- `docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0003-trade-calendar-driven-market-schedules.md`
- `docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md`
- `pipeline/scheduler/src/scheduler/defs/`
- `pipeline/scheduler/tests/`

## 目标

本计划针对 RFC 0006 指出的代码质量、模块化、功能抽象和可复用性问题，采用一次性整体改造方案，而不是边测边改的小步迁移。

核心目标：

- 先设计并实现 `pipeline/scheduler` 的最终代码目标结构。
- 一次性消除重复 helper、死代码、职责混杂和临时兼容层。
- 建立统一的 HTTP、分页、schema、metadata、分区物化、对象存储和 schedule/job 工厂抽象。
- 在代码结构稳定并通过代码框架 review 后，重写测试框架，使测试围绕新架构而不是旧模块路径补丁。
- 保持现有 Dagster asset key、S3 路径、Parquet schema、远端 API 语义和调度业务语义不变。
- 最终质量门禁同时覆盖 `ruff`、`pyright`、`dg check defs`、单元测试和覆盖率。

## 非目标

本计划不包含：

- 新增数据源。
- 修改远端 API 请求业务含义、分页排序策略或日期过滤语义。
- 修改 dbt 模型、ClickHouse、下游应用或部署拓扑。
- 迁移历史 S3 对象或重算历史分区。
- 为旧 import path 提供长期兼容层。
- 在代码结构迁移期间逐个修补旧测试。

## 总体策略

### 一刀切原则

本次改造将以目标架构为中心，允许在一个实施分支中大范围移动、重命名和重组模块。旧测试在结构迁移期间只作为行为参考，不作为阶段性通过标准。

原因：

- RFC 0006 涉及 `util.py`、HTTP 资源、S3 IO、metadata、schedule、测试 fake 和类型接口，修改点高度耦合。
- 若边移动代码边修旧测试，测试会不断绑定中间态路径和临时兼容层，形成测试黑洞。
- 新测试框架必须以最终抽象为边界，否则会把旧设计重新固化。

### 阶段门禁

改造分为三类阶段：

- **结构改造阶段**：以 import graph、Dagster definitions 加载、静态类型方向为主，不要求旧测试通过。
- **代码框架 review 阶段**：在阶段 5 清理旧结构后暂停测试重写，专门审查新代码框架是否消除了代码异味，并确认抽象能力、复用性和可扩展性得到改善。
- **测试重建阶段**：废弃旧测试组织方式，按新抽象分层重写测试，最终恢复完整质量门禁。

结构改造阶段允许短期测试失败，但不允许：

- 资产 key 变化。
- S3 object key 变化。
- Parquet 字段名或字段类型变化。
- schedule/job 名称非预期变化。
- 远端请求参数语义变化。

## 当前问题基线

RFC 0006 复验后确认仍存在：

- `_elapsed_seconds()` 分散在 4 个模块。
- `_required_string()` 分散在 2 个模块。
- `_positive_int_or_default()` 和 `_row_fingerprint()` 分散在 EastMoney 与 THS 逻辑中。
- `defs/util.py` 同时承载重试、S3、Parquet、日期、资产 key 和 BaoStock 证券过滤。
- `defs/config.py` 混合 `dg.EnvVar` 声明和配置数据类，并产生 `str | None` / `int | None` 类型问题。
- `io_managers/postgres.py` 存在 9 个未使用模块级包装函数。
- HTTP schema 转换和字符串转换分散。
- 分页、metadata、schedule/job 注册缺少统一抽象。
- 测试 fake、mock 和 helper 分散在多个测试文件中。
- `pyright` 存在大量结构性类型问题，主要来自 Dagster metadata、EnvVar、aiohttp session fake、pyarrow stub、测试替身接口不一致。

## 最终代码目标结构

目标结构如下：

```text
pipeline/scheduler/src/scheduler/
  definitions.py
    defs/
      __init__.py
      definitions.py              # 组装 assets/jobs/schedules/resources；替代 pipeline_defs.py
    automation/
      __init__.py
      schedules.py                # 通用 Dagster job/schedule 工厂
    config/
      __init__.py
      env.py                    # Dagster EnvVar 声明与 required getter
      models.py                 # S3Config、BaostockClientConfig、PipelineDatabaseConfig、JiuyanOcrConfig
    common/
      __init__.py
      clock.py                  # elapsed_seconds 等时间辅助
      strings.py                # required_string、optional_string、string_or_null
      numbers.py                # positive_int_or_default
      fingerprint.py            # row_fingerprint
      retry.py                  # ExponentialBackoffPolicy、DEFAULT_RETRY_POLICY
      metadata.py               # Dagster metadata builder 和 RawMetadataValue 类型收敛
      dates.py                  # parse_date_or_none、is_trade_date
    storage/
      __init__.py
      s3.py                     # S3 filesystem、object key、bytes read/write
      parquet.py                # parquet dataset read/write
      object_store.py           # 通用二进制对象存储
    market/
      __init__.py
      asset_keys.py             # 跨源资产 key 常量
      securities.py             # SecurityDateRange、filter_active_security_ranges
      trade_calendar.py         # 交易日历读取与过滤
      schedules.py              # A 股交易日调度工厂
    http/
      __init__.py
      client.py                 # AioHttpClient、request/response 类型、hook
      protocols.py              # HttpClientProtocol、HttpSessionProtocol、测试替身协议
      pagination.py             # 页码、游标、重复行检测
      schemas.py                # SchemaConversionConfig、TableConversionResult、字符串表构造
      partitioning.py           # 通用分区物化框架
      schedules.py              # HTTP 数据源 job/schedule 组装
    sources/
      __init__.py
      sina/
        __init__.py
        trade_calendar.py
      jiuyan/
        __init__.py
        action_field.py
        industry_list.py
        industry_ocr.py
        image_urls.py
        ocr_schema.py
        ocr_services.py
      ths/
        __init__.py
        limit_up_pool.py
      eastmoney/
        __init__.py
        assets.py
        client.py
        fields.py
        schema.py
    baostock/
      __init__.py
      assets.py
      client.py
      protocol.py
      schemas.py
      schedules.py
    ocr/
      __init__.py
      client.py
      schemas.py
      service.py
    repositories/
      __init__.py
      industry_images.py        # PostgresIndustryImageRepository，仅保留类 API
    io_managers/
      __init__.py
      s3_io_manager.py
```

### 包边界规则

- `common/` 只能放无业务含义的纯函数和轻量数据类。
- `storage/` 只处理 S3、object key、bytes、Parquet，不知道具体数据源。
- `market/` 只放跨数据源市场概念，如交易日、证券范围、共享 asset key。
- `automation/` 只放跨数据源 Dagster job/schedule 工厂，不依赖具体数据源协议。
- `market/` 可放依赖 A 股交易日历的调度工厂。
- `http/` 只放 HTTP 基础设施、分页、schema、分区物化和 HTTP 数据源 job/schedule 组装。
- `sources/` 放数据源业务逻辑；数据源模块可调用 `common/`、`storage/`、`market/`、`http/`。
- `repositories/` 只放数据库 repository，不暴露模块级便捷函数。
- `io_managers/` 只保留 Dagster IOManager 实现，不承载业务 repository 或对象存储业务规则。
- `defs/__init__.py` 和各子包 `__init__.py` 默认不 re-export 业务符号。

### 兼容层策略

本计划不保留长期兼容层。迁移分支内可以短期创建临时导入桥接文件，但在最终提交前必须删除：

- `defs/util.py`
- `defs/pipeline_defs.py`
- 旧 `http_resources/*` 业务模块
- `io_managers/postgres.py` 中的模块级包装函数
- 任何只为旧测试服务的 re-export

最终代码中每个符号只有一个 canonical import path。

## 最终抽象设计

### 配置抽象

`config/env.py` 提供 required getter，统一处理 `dg.EnvVar.get_value()` 返回 `None` 的类型问题：

```python
def required_env_str(name: str) -> str: ...
def required_env_int(name: str) -> int: ...
```

`config/models.py` 只定义配置数据类和 `from_env()`。业务模块不得直接调用 `dg.EnvVar`。

### HTTP 客户端抽象

`http/client.py` 负责真实 aiohttp 实现：

- `HttpRequest`
- `HttpTextResponse`
- `HttpBytesResponse`
- `HttpFetchStats`
- `HttpHooks`
- `AioHttpClient`

`http/protocols.py` 负责测试和业务依赖的结构化接口：

- `HttpJsonClientProtocol`
- `HttpTextClientProtocol`
- `HttpBytesClientProtocol`
- `HttpSessionProtocol`
- `HttpResponseContextProtocol`

业务函数依赖 protocol，不依赖具体 `AioHttpClient`。这可以同时解决可测试性和 pyright fake 类型问题。

### 分页抽象

`http/pagination.py` 提供两类 helper：

- 页码分页：适用于 EastMoney、THS。
- 游标/offset 分页：适用于 JiuYan industry list。

分页抽象只负责：

- 下一页参数生成。
- 结束条件。
- 重复 row fingerprint 检测。
- page/request 统计。

数据源仍负责：

- 构造业务请求参数。
- 解析远端 payload。
- 定义空响应语义。
- 决定重复页是否报错或跳过。

### Schema 转换抽象

`http/schemas.py` 提供：

- `SchemaConversionConfig`
- `TableConversionResult`
- `string_or_null()`
- `rows_to_string_table()`
- `unknown_field_count()`
- `copy_selected_fields()`

EastMoney、JiuYan、THS 只保留字段配置和嵌套结构展开逻辑。布尔值字符串大小写必须在迁移时明确：

- 若现有 Parquet 已依赖 `"true"/"false"`，保留 EastMoney 行为并命名为 `eastmoney_string_or_null()`。
- 若无业务差异，统一为一个 `string_or_null()`。

### Metadata 抽象

`common/metadata.py` 提供 Dagster metadata builder，目标是：

- 所有 asset materialization metadata 使用统一字段名。
- `row_count`、`column_count`、`partition_keys`、`s3_keys`、`request_count`、`retry_count`、`elapsed_seconds` 语义一致。
- builder 返回 Dagster 可接受的 metadata 类型，避免 `dict[str, object]` 传入 Dagster API。

建议定义：

```text
AssetMetadataBuilder
HttpStatsMetadata
PartitionMetadata
StorageMetadata
TimingMetadata
```

### 分区物化抽象

`http/partitioning.py` 将现有 `materialize_trade_date_range()` 泛化为：

- 任意 partition key。
- 可插拔 partition filter。
- 可插拔 fetch function。
- 可插拔 write function。
- 标准 per-partition metadata。

交易日资产通过 `market.trade_calendar` 注入过滤器；EastMoney 年分区通过自然年 range builder 注入分区解释器。

### 对象存储抽象

`storage/object_store.py` 提供通用二进制对象存储：

- `write_bytes(key, data) -> str`
- `read_bytes(key) -> bytes`
- `write_table(base_dir, table) -> str`

JiuYan 图片逻辑只负责：

- image filename 到 object key 的业务映射。
- content-type 校验。
- sha256/byte count 业务 metadata。

### Schedule/Job 工厂

`automation/schedules.py` 提供跨数据源声明式注册：

- `AssetJobSpec`
- `ScheduleSpec`
- `build_asset_job(spec)`
- `build_schedule(spec)`
- `build_year_refresh_schedule(spec)`

`market/schedules.py` 提供依赖 A 股交易日历的调度工厂：

- `build_trade_date_schedule(...)`

`http/schedules.py` 只组装 HTTP 数据源的 job/schedule 实例。

目标是让 `definitions.py` 只组装 spec，不再散落重复 `define_asset_job()` 和 `_evaluate_*_schedule()` 包装函数。

## 代码改造实施顺序

### 阶段 0：冻结基线

不改代码，只记录当前外部契约：

- asset key 列表。
- job 名称列表。
- schedule 名称列表。
- partition key name。
- S3 object key 模板。
- 每个 Parquet schema 字段名和类型。
- 每个远端 API 的核心请求参数。

基线命令：

```bash
cd pipeline/scheduler
uv run dg check defs
```

注意：当前 `dg` 需要在 `pipeline/scheduler` 目录运行，不能只在 `pipeline/` 下运行。

### 阶段 1：建立目标目录和公共抽象

新增最终目录，不迁移业务资产：

- `config/`
- `common/`
- `storage/`
- `market/`
- `http/`
- `sources/`
- `repositories/`

在这些目录中先实现稳定公共抽象：

- required env getter。
- elapsed/time helper。
- string/number/fingerprint helper。
- retry policy。
- metadata builder。
- S3/parquet/object store。
- HTTP protocol 和 hooks。
- pagination helper。
- schema conversion helper。
- schedule/job spec。

旧模块暂时不接入，避免半迁移状态影响业务逻辑。

### 阶段 2：迁移基础设施调用点

一次性替换旧基础设施引用：

- `defs/util.py` 的 S3/Parquet 能力迁移到 `storage/`。
- retry 迁移到 `common/retry.py`。
- asset key、trade calendar、security range 迁移到 `market/`。
- config 迁移到 `config/`。
- PostgreSQL repository 迁移到 `repositories/industry_images.py`。
- image object store 迁移到 `storage/object_store.py`。

阶段结束时删除：

- `io_managers/postgres.py` 的 9 个模块级包装函数。
- 所有重复 helper 的旧定义。

### 阶段 3：迁移 HTTP 数据源

将 `http_resources` 中的业务资产迁移到 `sources/`：

- `sina__trade_calendar.py` -> `sources/sina/trade_calendar.py`
- `jiuyan__action_field.py` -> `sources/jiuyan/action_field.py`
- `jiuyan__industry_list.py` -> `sources/jiuyan/industry_list.py`
- `jiuyan__industry_ocr.py` -> `sources/jiuyan/industry_ocr.py`
- `jiuyan_image_urls.py` -> `sources/jiuyan/image_urls.py`
- `jiuyan_ocr_schema.py` -> `sources/jiuyan/ocr_schema.py`
- `jiuyan_ocr_services.py` -> `sources/jiuyan/ocr_services.py`
- `ths__limit_up_pool.py` -> `sources/ths/limit_up_pool.py`
- `eastmoney.py`、`eastmoney_client.py`、`eastmoney_schema.py`、`eastmoney_fields.py` -> `sources/eastmoney/`

迁移时同步接入：

- `http/client.py`
- `http/protocols.py`
- `http/pagination.py`
- `http/schemas.py`
- `http/partitioning.py`
- `common/metadata.py`

阶段结束时删除旧 `http_resources` 业务模块，只保留 `http/` 基础设施。

### 阶段 4：重组 Dagster definitions

新增 `defs/definitions.py`，集中组装：

- assets
- jobs
- schedules
- resources

`scheduler/definitions.py` 只导入最终 `defs.definitions.defs`。

这一阶段要求：

- asset 函数名和 asset key 不变。
- group name 不变，除非明确记录。
- job/schedule 名称不变，除非 RFC 已要求替换。
- IO manager key 不变。
- metadata 中旧字段名继续输出，新增标准字段只能作为补充。

### 阶段 5：清理旧结构

删除所有中间结构：

- `defs/util.py`
- `defs/pipeline_defs.py`
- `defs/http_resources/` 旧业务文件。
- 重复 helper。
- 未使用 wrapper。
- 仅为旧测试存在的 fake、patch path 或 re-export。

阶段结束时必须能够：

```bash
cd pipeline/scheduler
uv run dg check defs
```

并且 `rg` 不再找到 RFC 0006 指出的重复函数定义。

### 阶段 6：代码框架 Review 门禁

阶段 5 完成后，不直接进入测试框架重写。必须先对优化调整后的代码框架做一次集中 review，确认新的代码结构本身是可维护、可复用、可扩展的。

该阶段只 review 生产代码框架和外部契约，不以旧测试是否恢复为判断标准。旧测试在此时仍可失败，但 `dg check defs` 必须通过，且 asset/job/schedule/S3/schema/request contract 不得变化。

Review 目标：

- 确认没有引入新的代码异味。
- 确认 RFC 0006 指出的重复、死代码、职责混杂已经被实际消除。
- 确认新抽象不是简单搬家，而是在多个数据源或模块中形成真实复用。
- 确认新增数据源的接入路径更短、更明确。
- 确认测试框架重写不会被迫围绕中间态兼容层或不成熟抽象展开。

Review 检查清单：

```text
代码质量
- 不存在新的万能模块、循环依赖、隐式 re-export。
- 不存在只为旧测试存在的生产代码、fake、patch hook。
- 不存在长期兼容导入层。
- 不存在 RFC 0006 已点名的重复 helper。
- 模块级代码不读取环境、不连接数据库、不访问网络、不做文件系统 I/O。

抽象边界
- common/ 只包含无业务含义的纯 helper。
- storage/ 不知道具体数据源。
- market/ 只表达跨数据源市场概念。
- automation/ 只提供跨数据源 Dagster job/schedule 工厂。
- market/ 只表达跨数据源市场概念和交易日调度。
- http/ 只提供 client、protocol、pagination、schema、partitioning 和 HTTP 数据源 job/schedule 组装。
- sources/ 只保留数据源业务逻辑。
- repositories/ 只保留 repository 类 API。

复用性
- EastMoney、THS、JiuYan 不再复制分页或 row fingerprint 核心算法。
- Sina、JiuYan、THS、EastMoney 共享 schema 表构造、字符串转换和未知字段统计能力。
- metadata 通过统一 builder 输出，不再散落 dict[str, object]。
- schedule/job 注册由 spec 或工厂统一生成。
- 对象存储抽象不再限定于图片。

扩展性
- 新增一个 HTTP 数据源时，只需实现请求构造、payload 解析、schema 配置和资产 spec。
- 新增一种二进制对象类型时，不需要复制 ImageObjectStore。
- 新增一种分区类型时，不需要复制 trade-date materialization 流程。
- 新增 schedule 时，不需要复制 evaluate wrapper。

外部契约
- asset key 不变。
- job/schedule 名称不变，除非计划中明确记录。
- S3 object key 模板不变。
- Parquet schema 字段名和类型不变。
- 远端 API 请求核心参数不变。
- 旧 metadata 字段继续存在。
```

Review 产出：

- `docs/plans/0008-code-framework-review.md` 或等价 review note。
- 旧模块到新模块迁移映射。
- RFC 0006 原问题逐项处理状态。
- 新发现代码异味列表及处理结论。
- “允许进入测试框架重写”或“退回代码框架调整”的明确结论。

若 review 未通过，必须回到阶段 1-5 调整代码框架。不得通过编写测试来固化有问题的抽象。

## 测试框架重写计划

测试框架在代码目标结构稳定并通过阶段 6 代码框架 review 后整体重写。旧测试只保留有价值的 case 数据和断言意图，不保留旧模块 patch path。

### 新测试目录结构

```text
pipeline/scheduler/tests/
  conftest.py
  fakes/
    __init__.py
    http.py                 # FakeHttpClient、FakeHttpSession、FakeHttpResponse
    dagster.py              # FakeAssetContext、schedule context helper
    storage.py              # Local/Memory filesystem helper
    database.py             # repository connection/cursor fake
    baostock.py             # FakeBaostockClient、FakeBaostockConnection
  helpers/
    __init__.py
    tables.py               # pyarrow table assertions
    metadata.py             # metadata assertions
    paths.py                # S3 key/path assertions
    snapshots.py            # schema/request baseline helper
  unit/
    common/
    storage/
    market/
    http/
    repositories/
    sources/
  integration/
    test_definitions_load.py
    test_asset_contracts.py
    test_schedule_contracts.py
```

### 测试分层

1. **纯函数单元测试**
   - `common/`
   - `market/`
   - `http/pagination.py`
   - `http/schemas.py`
   - `storage/s3.py`
   - 不使用 Dagster context，不 patch 网络。

2. **边界协议测试**
   - `AioHttpClient` 与 fake session。
   - repository 与 fake connection。
   - object store 与 local filesystem。
   - OCR client 与 fake HTTP。

3. **数据源业务测试**
   - Sina、JiuYan、THS、EastMoney 的请求构造、payload 解析、schema 转换。
   - 所有 fake 使用 protocol，不继承真实 client。

4. **Dagster contract 测试**
   - definitions 可加载。
   - asset key、group、partition、job、schedule 名称保持稳定。
   - schedule evaluation 的 run request/skip reason 行为。
   - metadata 字段 contract。

5. **覆盖率补齐测试**
   - 优先补 `baostock/assets.py`、`baostock/client.py`、`s3_io_manager.py`、`sources/jiuyan/industry_ocr.py`、OCR service。

### 测试重写顺序

前置条件：阶段 6 review 明确允许进入测试框架重写。

1. 建立 `tests/fakes/` 和 `tests/helpers/`。
2. 写 common/storage/http 基础设施测试。
3. 写 repository/object store 测试。
4. 写数据源 schema 和 request tests。
5. 写资产 contract tests。
6. 写 schedule/job contract tests。
7. 删除旧测试文件或按新目录拆分迁移。
8. 调整 coverage 门槛到可执行状态，并确保不低于当前项目要求。

### 测试禁止事项

- 不 patch 已删除旧模块路径。
- 不为临时兼容层写测试。
- 不让 fake 继承真实 client 来绕过类型检查。
- 不在测试中直接依赖 private helper，除非 helper 已提升为公共抽象。
- 不以覆盖率为目的测试 Dagster 装饰器内部行为。

## 类型检查整改计划

pyright 整改与测试框架重写同步完成。

重点：

- Dagster metadata 统一由 builder 返回正确类型。
- EnvVar 统一由 required getter 收敛为非 Optional。
- aiohttp session fake 通过 Protocol 适配。
- pyarrow stub 导出问题通过局部类型别名或 `TYPE_CHECKING` 边界处理。
- `dict[str, object]` 只用于外部 payload 边界，进入业务层后尽快转换为明确类型。
- 测试 fake 的 requests 记录使用数据类，不使用 `dict[str, object]` 混装。

最终命令：

```bash
cd pipeline
uv run pyright scheduler/src/scheduler scheduler/tests
```

## 验收标准

代码结构验收：

- `defs/util.py` 删除。
- `defs/pipeline_defs.py` 删除或不再作为 definitions 主入口。
- `http_resources/` 旧业务模块删除，业务进入 `sources/`。
- `io_managers/postgres.py` 的 9 个模块级包装函数删除。
- RFC 0006 指出的重复 helper 只保留一个 canonical 定义。
- `config.py` 不再混合 `dg.EnvVar` 声明和配置数据类。
- 测试 fake 全部位于 `tests/fakes/` 或 `tests/helpers/`。

代码框架 review 验收：

- 阶段 6 review note 已完成。
- review note 明确记录旧模块到新模块的迁移映射。
- review note 逐项确认 RFC 0006 原问题已解决或记录剩余处置。
- review note 确认未引入新的明显代码异味。
- review note 明确允许进入测试框架重写阶段。

行为验收：

- 所有现有 Dagster asset key 不变。
- 所有现有 S3 路径模板不变。
- 所有现有 Parquet schema 字段名和类型不变。
- EastMoney 请求字段、分页排序和日期过滤语义不变。
- JiuYan OCR 状态流转语义不变。
- trade-date 过滤仍以 `sina__trade_calendar` 为事实来源。

质量门禁：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing

cd pipeline/scheduler
uv run dg check defs
```

验收时测试覆盖率必须达到项目配置要求，不得通过降低 coverage 门槛完成计划。

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 一次性改造范围过大 | 中间状态不可运行 | 在独立分支完成，结构阶段只看 import graph 和 definitions load |
| 旧测试失效导致行为回归不可见 | 业务语义回归 | 阶段 0 先冻结 asset/S3/schema/request contract，测试重写时优先恢复 contract tests |
| 新抽象过度复杂 | 后续维护成本升高 | 抽象只覆盖已有 4 类重复模式：schema、pagination、metadata、partitioning |
| 测试框架过早重写 | 测试固化不成熟抽象 | 阶段 5 后强制代码框架 review，通过后才能进入测试重写 |
| import path 大范围变化 | Dagster definitions 加载失败 | 最终统一由 `defs/definitions.py` 组装，验收强制 `dg check defs` |
| metadata builder 改变字段 | 下游观测或运维脚本受影响 | 旧字段继续输出，新标准字段作为补充 |
| pyright 修复影响运行代码 | 类型收敛引入行为变化 | 类型修复必须以 runtime contract tests 兜底 |
| 覆盖率短期下降 | 无法过质量门禁 | 测试重写阶段先补低覆盖核心模块，再删除旧测试 |

## 实施产物

本计划完成后应形成：

- 新目标目录结构。
- 删除旧工具箱和旧业务模块。
- 统一公共抽象。
- 阶段 6 代码框架 review note。
- 新测试框架和 fake/helper 库。
- 通过质量门禁的 scheduler 工程。
- 一份 migration note，列出旧模块到新模块的映射，供后续 review 和排查使用。

## 推荐执行方式

该计划不适合拆成多个已合并 PR。推荐流程：

1. 创建单独长期分支。
2. 阶段 0 冻结 contract。
3. 阶段 1-5 完成目标代码结构。
4. 阶段 6 进行代码框架 review，确认没有引入新的代码异味，且抽象能力、复用性和可扩展性得到改善。
5. review 通过后，阶段 7 重写测试框架。
6. 阶段 8 统一修复 pyright、ruff、coverage 和 definitions load。
7. 最终以一个大 PR review，review 重点放在 contract 是否保持稳定，以及阶段 6 review 发现的问题是否已处理。

在最终 PR 合并前，不应把中间兼容层或半迁移测试合入主干。
