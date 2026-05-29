# Plan 0009: 日频资产 Compact 压缩层

状态：草案

计划日期：2026-05-29

关联 RFC：

- 无独立 RFC，本计划自包含设计决策

参考资料：

- `docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0003-trade-calendar-driven-market-schedules.md`
- `docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md`
- `docs/plans/0002-baostock-aio-tcp-client.md`
- `docs/plans/0004-http-client-refactor-and-rfc0003-implementation.md`
- `docs/plans/0008-pipeline-rfc0006-quality-reusability-implementation.md`
- `pipeline/scheduler/src/scheduler/defs/`
- `pipeline/scheduler/tests/`

## 目标

为韭研事件（`jiuyan__action_field`）和同花顺涨停池（`ths__limit_up_pool`）两个日频 source 资产新增下游 compact 压缩层，将日分区 parquet 聚合为年度分区 parquet，降低后续分析和导入任务读取大量小文件的成本。

核心目标：

- 新增 2 个年度分区 compact 资产：
  - `jiuyan__action_field_compacted`
  - `ths__limit_up_pool_compacted`
- compact 资产作为 source 日频资产的下游派生层，不改变现有 source 资产采集逻辑、分区策略和 S3 路径。
- 新增一个可复用的 S3 parquet 日分区读取 helper，继续复用 `asset_key_to_parquet_object_key()` 生成对象路径。
- 当前年 compact 支持每日增量刷新语义：读取当年年初到本次触发交易日或当前交易日的数据。
- 历史年 compact 支持 Dagster backfill：每个年份一个 run，读取完整年份。
- 触发方式优先使用 Dagster declarative automation；若跨粒度 daily → yearly 触发验证不稳定，回退到 asset sensor。

## 非目标

本计划不包含：

- 修改 `jiuyan__action_field`、`ths__limit_up_pool` 的 HTTP 请求、schema flatten、日分区写入或调度时间。
- 为 BaoStock K 线、EastMoney F10 等已按年分区资产新增 compact 层。
- 新增 dbt 模型、ClickHouse 导入任务或下游消费逻辑。
- 修改 `S3IOManager.load_input()`。当前 compact 资产仍手动读取上游 parquet。
- 为 compact 层单独引入新的 object prefix。compact 文件随现有 source 路径规则写到 `source/{compact_asset_key}/year=YYYY/000000_0.parquet`。
- 在 `scheduler.defs.http.schedules` 中组装 compact job。第一版 job 在 `definitions.py` 内直接组装；等 compact 数量增加后再抽到独立模块。

## 设计调整摘要

原方案里有两个点需要按当前 `pipeline/scheduler` 架构调整：

1. **compact 是 source 内的下游派生资产，不属于 HTTP source 调度模块**

   `jiuyan__action_field` 和 `ths__limit_up_pool` 虽然来自 HTTP 数据源，但 compact 层的职责是 S3 parquet 派生聚合，不应继续扩展 `http/schedules.py`。第一版资产放在各自 source 子包内，靠近 source 资产和 schema；当 compact 资产扩展到多个数据源并出现共享 sensor/factory 管理需求时，再考虑抽成顶层 `scheduler.defs.compacted`。

2. **自动化触发先验证 Dagster 当前推荐条件**

   当前项目使用 Dagster `1.13.6`。Dagster 文档推荐用 `AutomationCondition.eager()` 传播上游更新；本地 API 中没有 `AutomationCondition.all_deps_updated()`，但有 `eager()`、`any_deps_updated()`、`all_deps_updated_since_cron()` 等条件。因此实施时不使用不存在的 `all_deps_updated()`，优先验证 `AutomationCondition.eager()` 对 daily → yearly 分区映射是否能按预期触发当前年 compact。

## 当前架构约束

### 现有模块边界

| 模块 | 当前职责 | compact 相关结论 |
|------|----------|------------------|
| `automation/` | 通用 job/schedule factory，如 `AssetJobSpec`、`build_asset_job()`、`build_schedule()` | 可复用 job factory，不放 source-specific compact 资产 |
| `market/` | 跨数据源市场概念，如交易日历读取和 A 股交易日 schedule | compact 应通过 `market.trade_calendar.read_trade_dates_from_s3()` 读取交易日 |
| `http/` | HTTP client、schema、pagination、partitioning，以及 HTTP source 资产 job/schedule 组装 | 不放 compact job/sensor |
| `sources/jiuyan`、`sources/ths` | 数据源业务采集逻辑 | source 资产保持不变 |
| `storage/` | S3、object key、parquet 读写 helper | 新增日分区 parquet 读取 helper |
| `io_managers/` | Dagster IOManager 输出写入 | 不实现 `load_input()` |
| `definitions.py` | 汇总注册 assets/jobs/schedules/resources | 第一版可直接组装 compact jobs，避免过早新增顶层子包 |

### source 日频资产

| 资产 | 分区 | 起始日 | 调度 | 写入路径 |
|------|------|--------|------|----------|
| `jiuyan__action_field` | `DailyPartitionsDefinition` | 2021-01-01 | 16:45 CST，仅交易日 | `source/jiuyan__action_field/trade_date=YYYY-MM-DD/000000_0.parquet` |
| `ths__limit_up_pool` | `DailyPartitionsDefinition` | 2025-01-01 | 16:45 CST，仅交易日 | `source/ths__limit_up_pool/trade_date=YYYY-MM-DD/000000_0.parquet` |

两个 source 资产当前通过 `materialize_trade_date_range()` 自行写 S3，并允许空表：

- metadata: `storage_mode=partitioned`
- metadata: `partition_key_name=trade_date`
- metadata: `allow_empty=True`
- backfill: `BackfillPolicy.single_run()`

### 年度分区参考

| 资产 | 分区 | 起始年 | 刷新方式 |
|------|------|--------|----------|
| `baostock__query_history_k_data_plus_daily` | `TimeWindowPartitionsDefinition(fmt="%Y")` | 1990 | `KLineDailyYearConfig.refresh_until_trade_date` |
| `eastmoney__*` | `TimeWindowPartitionsDefinition(fmt="%Y")` | 2015 | schedule 注入 `refresh_until_date` |

compact 资产复用年度分区、单年单 run 和 `S3IOManager` 写入模式，但不复用 BaoStock 的远端拉取逻辑。

## 目标设计

### 新增文件

```text
pipeline/scheduler/src/scheduler/defs/sources/jiuyan/
  action_field_compact.py        # jiuyan__action_field_compacted 资产定义

pipeline/scheduler/src/scheduler/defs/sources/ths/
  limit_up_pool_compact.py       # ths__limit_up_pool_compacted 资产定义

pipeline/scheduler/tests/unit/compacted/
  __init__.py
  test_action_field_compact.py
  test_limit_up_pool_compact.py

pipeline/scheduler/tests/unit/storage/
  test_parquet_readers.py
```

### 修改文件

```text
pipeline/scheduler/src/scheduler/defs/storage/parquet_readers.py
  # 新增 read_partitioned_parquet_tables_from_s3() 和 trade_date_partition_keys_for_year()

pipeline/scheduler/src/scheduler/defs/definitions.py
  # 注册 compact assets；第一版直接组装 compact jobs；必要时注册 compact sensors

pipeline/scheduler/tests/integration/test_definitions_and_schedules.py
  # 增加 definitions 注册和 job selection 验证
```

### 不修改文件

```text
pipeline/scheduler/src/scheduler/defs/http/schedules.py
pipeline/scheduler/src/scheduler/defs/http/partitioning.py
pipeline/scheduler/src/scheduler/defs/sources/jiuyan/action_field.py
pipeline/scheduler/src/scheduler/defs/sources/ths/limit_up_pool.py
pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py
```

### 暂不新增顶层子包

第一版不创建 `scheduler.defs.compacted`。理由：

- 当前只有 2 个 compact 资产，按 source 放置更贴合现有 `sources/jiuyan`、`sources/ths` 分包方式。
- compact 资产需要靠近 source 资产、source tag、schema helper 和 source 语义。
- 顶层 `compacted/` 会引入新的层级概念，但短期只承载两个文件，收益有限。

后续满足任一条件时再抽出顶层 `compacted/`：

- compact 资产扩展到 3 个以上数据源。
- 出现跨 source 共享的 compact asset factory、sensor factory 或批量注册逻辑。
- 下游消费需要以 “compact layer” 为主要维护边界，而不是以 source 为主要维护边界。

## 资产命名与分区

### Asset key

| source 资产 | compact 资产 |
|----------|--------------|
| `source/jiuyan__action_field` | `source/jiuyan__action_field_compacted` |
| `source/ths__limit_up_pool` | `source/ths__limit_up_pool_compacted` |

资产使用 `key_prefix=["source"]` 让 Global Asset Lineage 形成 `source/...` 层级；S3 写入路径也统一迁移到 `source/...`。`S3IOManager.object_prefix` 为 `source`，`asset_key_to_parquet_object_key()` 会在 asset key 已经以 `source` 开头时去重，避免生成 `source/source/...`：

```text
AssetKey(["source", "jiuyan__action_field"])
=> source/jiuyan__action_field/trade_date=YYYY-MM-DD/000000_0.parquet
```

### Group 与 tag

compact 资产和其他 S3 落地资产统一使用 `s3_sources` group：

```python
group_name="s3_sources"
tags={"source": "jiuyan", "layer": "compacted", "storage": "s3"}
tags={"source": "ths", "layer": "compacted", "storage": "s3"}
```

### 年度分区

```python
jiuyan_action_field_compacted_year_partitions = dg.TimeWindowPartitionsDefinition(
    start="2021",
    fmt="%Y",
    cron_schedule="0 0 1 1 *",
    timezone="Asia/Shanghai",
    end_offset=1,
)

ths_limit_up_pool_compacted_year_partitions = dg.TimeWindowPartitionsDefinition(
    start="2025",
    fmt="%Y",
    cron_schedule="0 0 1 1 *",
    timezone="Asia/Shanghai",
    end_offset=1,
)
```

## 分区依赖设计

compact 年分区依赖 source 日分区。需要显式声明 partition mapping，避免依赖 Dagster 默认行为产生歧义。

```python
deps=[
    dg.AssetDep(
        jiuyan__action_field,
        partition_mapping=dg.TimeWindowPartitionMapping(),
    )
]
```

期望语义：

- `jiuyan__action_field_compacted["2026"]` 依赖 `jiuyan__action_field["2026-01-01"..."2026-12-31"]` 中落在同一 time window 的分区。
- 自动刷新当前年时，compact 资产函数只读取已存在的交易日日分区，不要求全年 source 分区全部存在。
- 历史年 backfill 时，读取该年份内所有存在且非空的交易日日分区。

注意：`TimeWindowPartitionMapping()` 解决血缘和 backfill dependency 语义，不负责把上游 daily parquet 自动加载成输入参数。本计划仍由 compact asset 手动从 S3 读取 source parquet。

## S3 读取 helper

在 `storage/parquet_readers.py` 新增通用 helper，保持路径生成逻辑和 IOManager 一致。

推荐签名：

```python
@dataclass(frozen=True)
class PartitionedParquetReadResult:
    tables: list[pa.Table]
    read_partition_keys: list[str]
    missing_partition_keys: list[str]
    empty_partition_keys: list[str]


def read_partitioned_parquet_tables_from_s3(
    config: S3Config,
    asset_key: dg.AssetKey,
    *,
    partition_keys: Sequence[str],
    partition_key_name: str,
) -> PartitionedParquetReadResult:
    """Read existing non-empty parquet files for explicit partition keys."""
```

行为规格：

- 对每个 `partition_key` 调用 `asset_key_to_parquet_object_key()` 生成 object key。
- 使用 `build_s3_filesystem()` + `filesystem.open_input_file()` + `pyarrow.parquet.read_table()` 读取。
- `FileNotFoundError` 或 pyarrow S3 not-found 异常只记录为 missing，不中断 compact。
- `table.num_rows == 0` 记录为 empty，不参与 concat。
- 其他异常继续抛出，避免吞掉 schema 或权限问题。
- 返回读取到的非空表、实际读取分区、缺失分区、空分区。

年度日分区筛选可以作为第二个 helper：

```python
def trade_date_partition_keys_for_year(
    year: int,
    *,
    trade_dates: set[date],
    refresh_until_trade_date: date | None = None,
) -> list[str]:
    """Return sorted ISO trade-date partition keys for the selected year range."""
```

这样 `parquet_readers.py` 只处理存储读取；交易日筛选逻辑可以独立测试，也可后续复用于其他日频 compact 资产。

## compact asset 实现

第一版不强制抽 asset factory。两个 compact 资产分别放在 source 子包内：

- `sources/jiuyan/action_field_compact.py`
- `sources/ths/limit_up_pool_compact.py`

如果实现时发现两个资产只有 spec 不同，可以在其中一个模块内先提取私有 helper；不要为了两个资产提前创建顶层 factory。后续 compact 资产增加时，再迁移为共享 factory。

可选 factory 形态如下，作为后续抽象方向：

```python
@dataclass(frozen=True)
class DailyToYearCompactSpec:
    name: str
    source: str
    raw_asset: dg.AssetsDefinition
    raw_asset_key: dg.AssetKey
    partitions_def: dg.TimeWindowPartitionsDefinition


def build_daily_to_year_compact_asset(spec: DailyToYearCompactSpec) -> dg.AssetsDefinition:
    @dg.asset(
        name=spec.name,
        group_name="s3_sources",
        partitions_def=spec.partitions_def,
        deps=[dg.AssetDep(spec.raw_asset, partition_mapping=dg.TimeWindowPartitionMapping())],
        io_manager_key="s3_io_manager",
        backfill_policy=dg.BackfillPolicy.multi_run(max_partitions_per_run=1),
        automation_condition=dg.AutomationCondition.eager(),
        metadata={
            "storage_mode": "partitioned",
            "partition_key_name": "year",
            "partitions_def": "year_partitions",
            "input_partition_key_name": "trade_date",
            "input_asset": spec.raw_asset_key.to_user_string(),
        },
        tags={"source": spec.source, "layer": "compacted", "storage": "s3"},
    )
    def compact_asset(context: dg.AssetExecutionContext) -> dg.MaterializeResult[dict[str, pa.Table]]:
        ...

    return compact_asset
```

资产函数逻辑：

1. 解析 `context.partition_key` 为 `year`。
2. 从 `S3Config.from_env()` 获取对象存储配置。
3. 通过 `market.trade_calendar.read_trade_dates_from_s3()` 读取交易日历。
4. 按 `Asia/Shanghai` 计算当前日期，并计算该年应读取的交易日分区：
   - 历史年：该年所有交易日。
   - 当前年自动运行：默认读取该年年初到当前日期；如果后续用 sensor，可从 run tag 读取触发交易日并截断到该日。
5. 调用 `read_partitioned_parquet_tables_from_s3()` 读取 source 日分区。
6. 若没有非空 table：
   - 返回 `{year: empty table}` 需要 schema，当前 source schema helper 没有统一导出空表。
   - 更推荐第一版直接抛出 `RuntimeError`，metadata 中记录 missing/empty 情况，避免写出无 schema 空 parquet。
7. 使用 `pa.concat_tables(tables, promote_options="default")` 合并。
8. 返回 `MaterializeResult(value={context.partition_key: merged}, metadata=...)`，由 `S3IOManager` 写入 `source/{compact_asset}/year=YYYY/000000_0.parquet`。

第一版建议“不写空 compact 年分区”。理由：

- 现有 `S3IOManager` 对 partitioned 资产默认不允许空表，compact 资产也不应轻易打开 `allow_empty=True`。
- 空 compact 年分区容易让下游误认为该年份已完整处理。
- 对当前年自动触发来说，如果 source 当日没有非空数据，应让 run 明确失败或跳过，而不是产出空年度文件。

## 当前年截断策略

第一版采用 conservative 策略，并按 A 股交易日语境使用 `Asia/Shanghai` 日期：

```python
today = datetime.now(ZoneInfo("Asia/Shanghai")).date()
refresh_until = today if year == today.year else None
```

如果使用 sensor 回退方案，则 sensor 可将 source 事件的 `market.trade_date` 写入 run tag：

```python
tags={"market.trade_date": trade_date_str, "market.year": year}
```

compact asset 可优先读取 tag：

```python
trigger_trade_date = context.run.tags.get("market.trade_date")
refresh_until = date.fromisoformat(trigger_trade_date) if trigger_trade_date else default_refresh_until
```

这样可以避免自动运行发生在 UTC/CST 日期边界附近时读取范围偏大。若继续使用 `AutomationCondition.eager()`，则保留 `Asia/Shanghai` 当前日期默认行为，并依赖“缺失日分区跳过”保证读取不失败。

## compact job 组装

第一版直接在 `definitions.py` 附近组装两个 compact job，不新增 `compacted/schedules.py`，也不新增 cron schedule：

```python
from scheduler.defs.automation import schedules as automation_schedules
from scheduler.defs.sources.jiuyan.action_field_compact import (
    jiuyan__action_field_compacted,
)
from scheduler.defs.sources.ths.limit_up_pool_compact import (
    ths__limit_up_pool_compacted,
)

jiuyan__action_field_compacted_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="jiuyan__action_field_compacted_job",
        selection=[jiuyan__action_field_compacted],
    )
)

ths__limit_up_pool_compacted_job = automation_schedules.build_asset_job(
    automation_schedules.AssetJobSpec(
        name="ths__limit_up_pool_compacted_job",
        selection=[ths__limit_up_pool_compacted],
    )
)
```

`definitions.py` 注册：

- compact assets 加入 `assets=[...]`
- compact jobs 加入 `jobs=[...]`
- 不增加 schedules
- 如启用 sensor 回退，再加入 `sensors=[...]`

如果后续 job 数量增多，再抽到 `automation/compact_jobs.py` 或未来的 `compacted/schedules.py`。

## 自动触发策略

### 首选：Declarative automation

使用：

```python
automation_condition=dg.AutomationCondition.eager()
```

需要验证：

- source 当日分区 materialize 后，Dagster 是否请求对应当前年 compact 分区。
- daily → yearly 分区映射是否和 `TimeWindowPartitionMapping()` 一致。
- 同一当前年内多个 source 日分区连续完成时，automation 是否会重复触发当前年 compact；这属于可接受行为，因为 compact 是年度覆盖写入。

本计划不使用 `AutomationCondition.all_deps_updated()`，因为当前 Dagster `1.13.6` 本地 API 没有该方法。

### 回退：Asset sensor

如果 `eager()` 对跨粒度 daily → yearly 的请求行为不符合预期，第一版可新增 source-local sensor 模块：

- `sources/jiuyan/action_field_compact_sensor.py`
- `sources/ths/limit_up_pool_compact_sensor.py`

如果两个 sensor 完全同构，再提取到 `automation/compact_sensors.py`。

```python
@dataclass(frozen=True)
class DailyToYearCompactSensorSpec:
    name: str
    monitored_asset_key: dg.AssetKey
    compact_job: dg.UnresolvedAssetJobDefinition
    source: str


def build_daily_to_year_compact_sensor(
    spec: DailyToYearCompactSensorSpec,
) -> dg.SensorDefinition:
    ...
```

sensor 行为：

- 监听 source asset materialization。
- 从 materialization event 的 partition key 或 `market.trade_date` tag 解析交易日。
- 将交易日映射为年份 partition key。
- 发起 compact job run，`partition_key=str(year)`。
- run key 包含 source asset、trade date 和 source run id，避免重复提交。
- tags 写入 `market.trade_date`、`market.year`、`source`。

sensor factory 第一版不放顶层 `compacted/`。如果只服务某个 source，就先贴近 source；如果多个 source 复用，再上移到 `automation/`。

## 分阶段执行

### 阶段 0：补齐读取 helper

改动：

- `storage/parquet_readers.py`
- `tests/unit/storage/test_parquet_readers.py`

任务：

1. 新增 `PartitionedParquetReadResult`。
2. 新增 `read_partitioned_parquet_tables_from_s3()`。
3. 新增 `trade_date_partition_keys_for_year()`。
4. 测试：
   - 多个分区正常读取。
   - 部分分区不存在时记录 missing 并继续。
   - 空 parquet 记录 empty 并跳过。
   - `refresh_until_trade_date` 能截断当前年读取范围。
   - object key 通过 `asset_key_to_parquet_object_key()` 生成。

验证命令：

```bash
cd pipeline
uv run ruff check scheduler/src/scheduler/defs/storage/parquet_readers.py scheduler/tests/unit/storage/test_parquet_readers.py
uv run pyright scheduler/src/scheduler/defs/storage/parquet_readers.py scheduler/tests/unit/storage/test_parquet_readers.py
uv run pytest scheduler/tests/unit/storage/test_parquet_readers.py -v
```

### 阶段 1：新增 compact assets

改动：

- `sources/jiuyan/action_field_compact.py`
- `sources/ths/limit_up_pool_compact.py`
- `tests/unit/compacted/test_action_field_compact.py`
- `tests/unit/compacted/test_limit_up_pool_compact.py`

任务：

1. 定义两个年度 partition definitions。
2. 分别实现两个 source-local compact assets；如有重复逻辑，先提取模块私有 helper。
3. 定义 `jiuyan__action_field_compacted`。
4. 定义 `ths__limit_up_pool_compacted`。
5. 单元测试合并逻辑、空输入行为、metadata、partition key。

验证命令：

```bash
cd pipeline
uv run ruff check scheduler/src/scheduler/defs/sources/jiuyan/action_field_compact.py scheduler/src/scheduler/defs/sources/ths/limit_up_pool_compact.py scheduler/tests/unit/compacted
uv run pyright scheduler/src/scheduler/defs/sources/jiuyan/action_field_compact.py scheduler/src/scheduler/defs/sources/ths/limit_up_pool_compact.py scheduler/tests/unit/compacted
uv run pytest scheduler/tests/unit/compacted -v
```

### 阶段 2：注册 definitions 和 jobs

改动：

- `definitions.py`
- `tests/integration/test_definitions_and_schedules.py`

任务：

1. 在 `definitions.py` 中通过 `automation_schedules.build_asset_job()` 组装两个 compact job。
2. 在 `definitions.py` 注册 compact assets 和 jobs。
3. 不修改 `http/schedules.py`。
4. 集成测试验证：
   - definitions 可加载。
   - compact assets 存在。
   - compact jobs 存在。
   - 没有新增 compact cron schedule。

验证命令：

```bash
cd pipeline
uv run ruff check scheduler/src/scheduler/defs/sources/jiuyan/action_field_compact.py scheduler/src/scheduler/defs/sources/ths/limit_up_pool_compact.py scheduler/src/scheduler/defs/definitions.py scheduler/tests/integration/test_definitions_and_schedules.py
uv run pyright scheduler/src/scheduler/defs/sources/jiuyan/action_field_compact.py scheduler/src/scheduler/defs/sources/ths/limit_up_pool_compact.py scheduler/src/scheduler/defs/definitions.py scheduler/tests/integration/test_definitions_and_schedules.py
cd scheduler
uv run dg check defs
```

### 阶段 3：验证 declarative automation

任务：

1. 用 ephemeral Dagster instance 写一个 focused 测试，尽量通过 `dg.evaluate_automation_conditions()` 验证 `eager()` 是否会为 compact 请求分区。
2. 若单元测试无法稳定模拟跨粒度事件，则在本地 Dagster UI 手动验证：
   - materialize `jiuyan__action_field` 最近一个交易日分区。
   - 观察是否请求 `jiuyan__action_field_compacted` 当前年分区。
   - 对 `ths__limit_up_pool` 重复。
3. 记录验证结论到本文档执行记录。

验收：

- 如果 `eager()` 可用：保留 declarative automation。
- 如果 `eager()` 不可用或触发分区不正确：进入阶段 3B sensor 回退。

### 阶段 3B：sensor 回退

仅在阶段 3 失败时执行。

改动：

- `sources/jiuyan/action_field_compact_sensor.py`
- `sources/ths/limit_up_pool_compact_sensor.py`
- `definitions.py`
- `tests/unit/compacted/test_sensors.py`

任务：

1. 实现 `build_daily_to_year_compact_sensor()`。
2. 注册两个 sensor。
3. 从 compact assets 移除或禁用 `automation_condition`，避免双重触发。
4. 测试 partition key 映射、run key、run tags。

### 阶段 4：最小数据验证

任务：

1. 确保 `sina__trade_calendar` 已物化。
2. 对 `jiuyan__action_field` 物化最近 1-2 个交易日分区。
3. 触发或等待 `jiuyan__action_field_compacted` 当前年分区。
4. 验证 S3 输出：
   - `source/jiuyan__action_field_compacted/year=YYYY/000000_0.parquet`
   - row count 等于已读取非空日分区之和。
   - schema 与 source 日分区兼容。
5. 对 `ths__limit_up_pool` 重复。

## 验收条件

### 架构验收

- compact 资产位于对应 source 子包：`sources/jiuyan` 和 `sources/ths`。
- compact jobs 第一版在 `definitions.py` 直接组装，无新增 compact cron schedule。
- 如需 sensor 回退，第一版使用 source-local sensor 模块；复用需求明确后再上移到 `automation/`。
- `http/schedules.py` 未新增 compact job/sensor。
- S3 路径生成继续复用 `asset_key_to_parquet_object_key()`。
- 交易日历读取通过 `market.trade_calendar.read_trade_dates_from_s3()`。

### Definition 验收

- 新增 assets：
  - `jiuyan__action_field_compacted`
  - `ths__limit_up_pool_compacted`
- 新增 jobs：
  - `jiuyan__action_field_compacted_job`
  - `ths__limit_up_pool_compacted_job`
- 无新增 compact cron schedule。
- `dg check defs` 通过。

### 数据验收

- compact 输出路径：
  - `source/jiuyan__action_field_compacted/year=YYYY/000000_0.parquet`
  - `source/ths__limit_up_pool_compacted/year=YYYY/000000_0.parquet`
- 每个 compact 年份分区输出一个 parquet 文件。
- 文件压缩继续由 `S3IOManager` 使用 zstd。
- compact row count 等于实际读取的非空 source 日分区 row count 之和。
- metadata 至少包含：
  - `row_count`
  - `column_count`
  - `input_asset`
  - `requested_partition_count`
  - `read_partition_count`
  - `missing_partition_count`
  - `empty_partition_count`
  - `refresh_until_trade_date`

### 质量门禁

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

## 建议 PR 拆分

### PR 1：storage 读取 helper

- `parquet_readers.py`
- `tests/unit/storage/test_parquet_readers.py`

理由：纯 storage 能力，风险小，可独立验证。

### PR 2：compact assets 和 jobs

- `sources/jiuyan/action_field_compact.py`
- `sources/ths/limit_up_pool_compact.py`
- `definitions.py`
- `tests/unit/compacted/test_action_field_compact.py`
- `tests/unit/compacted/test_limit_up_pool_compact.py`
- `tests/integration/test_definitions_and_schedules.py`

理由：引入新资产但不改变现有 source 行为。

### PR 3：自动触发验证或 sensor 回退

- 若 `AutomationCondition.eager()` 验证通过：只补测试和验证记录。
- 若不通过：新增 source-local sensor 模块和 sensor 测试；多个 source 复用后再抽到 `automation/`。

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| `AutomationCondition.eager()` 对 daily → yearly 分区触发不符合预期 | compact 当前年不能自动刷新或触发分区错误 | 阶段 3 专门验证；失败时启用 compact asset sensor |
| 当前年日期计算和 A 股交易日时区不一致 | 在 UTC/CST 边界读取范围偏大或偏小 | 使用 `datetime.now(ZoneInfo("Asia/Shanghai")).date()`；sensor 回退时用 source event trade date 精准截断 |
| 日分区 schema 漂移 | `pa.concat_tables()` 失败或类型提升不符合预期 | 使用 `promote_options="default"`；失败时让 run 失败并暴露具体年份和分区 |
| 单年读取 250+ parquet 文件内存偏高 | run OOM | 第一版按年单 run；如出现压力，再引入批量 concat 或 streaming writer |
| source 某些交易日为空或缺失 | compact row count 低于交易日数预期 | metadata 记录 missing/empty 分区；不把缺失视为失败 |
| 写出空 compact 年分区 | 下游误判年份完整 | 第一版没有非空 table 时失败，不写空文件 |
| compact 仍使用 `source/` 前缀 | 下游可能混淆 source 和 compact | asset key 明确带 `_compacted`；后续如需要再引入独立 object prefix |

## 回滚策略

Compact 层是纯增量添加：

- 从 `definitions.py` 移除 compact assets、jobs、sensors。
- 删除 `sources/jiuyan/action_field_compact.py`、`sources/ths/limit_up_pool_compact.py`，以及可选 sensor 模块。
- 删除 `parquet_readers.py` 中仅服务 compact 且未被其他代码使用的 helper。
- 在 Dagster UI 中关闭 compact automation 或 sensors。
- 删除 S3 中：
  - `source/jiuyan__action_field_compacted/`
  - `source/ths__limit_up_pool_compacted/`
- source 日频资产不受影响。

## 执行完成记录

| 阶段 | 完成日期 | 执行人 | 结果 | 备注 |
|------|----------|--------|------|------|
| 阶段 0 | - | - | - | storage 读取 helper |
| 阶段 1 | - | - | - | compact assets |
| 阶段 2 | - | - | - | definitions 和 jobs |
| 阶段 3 | - | - | - | declarative automation 验证 |
| 阶段 3B | - | - | - | sensor 回退，仅必要时 |
| 阶段 4 | - | - | - | 最小数据验证 |
