# ADR 0001: 行情 raw 数据采用 Dagster asset 编排

状态：Accepted

日期：2026-05-27

## 背景

RFC 0001 定义了第一阶段行情数据采集范围：

- `sina__trade_calendar`
- `baostock__query_stock_basic`
- `baostock__query_history_k_data_plus_daily`

Plan 0001 和 Plan 0002 进一步设计了新浪交易日历、BaoStock 证券基础信息、BaoStock 日频 K 线的落地方式。当前代码已经在 `pipeline/scheduler` 中实现这些资产，并通过 `pipeline/scheduler/src/scheduler/defs/pipeline_defs.py` 注册到 Dagster definitions。

## 决策

行情 raw 数据采集使用 Dagster asset 作为编排边界。

- 新浪交易日历使用 `sina__trade_calendar` asset，属于 `http_sources` 组。
- BaoStock 证券基础信息使用 `baostock__query_stock_basic` asset，属于 `baostock` 组。
- BaoStock 日频 K 线使用 `baostock__query_history_k_data_plus_daily` asset，属于 `baostock` 组。
- raw 资产统一通过 `s3_io_manager` 写入 S3 兼容对象存储。
- asset 返回 `pyarrow.Table` 或分区键到 `pyarrow.Table` 的映射，由 IO manager 负责持久化。
- Dagster materialization metadata 必须包含行数、列数、文件格式、存储位置和关键耗时指标，便于排查远端接口、过滤、拼表和写入阶段的问题。

## 依据

当前实现：

- `pipeline/scheduler/src/scheduler/defs/http_resources/sina__trade_calendar.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/src/scheduler/defs/pipeline_defs.py`
- `pipeline/scheduler/src/scheduler/defs/io_managers/s3_io_manager.py`

设计来源：

- `docs/RFC/0001-market-data-ingestion.md`
- `docs/plans/0001-sina-trade-calendar-s3-ingestion.md`
- `docs/plans/0002-baostock-aio-tcp-client.md`

## 后果

- 采集、调度、回填、元数据观测都通过 Dagster 管理。
- 下游不直接依赖远端接口，而依赖已经物化的 raw Parquet。
- asset 函数保持业务采集职责，S3 路径和 Parquet 写入细节集中在 IO manager 和 util 中。
- 新增行情 raw 资产时，应优先复用现有 asset、IO manager、S3 key、metadata 和 retry 模式。
