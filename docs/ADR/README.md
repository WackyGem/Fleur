# ADR

本目录记录已经被项目接受或正在执行的架构决策。

约定：

- ADR 以当前代码为主，结合 RFC 和 plan 解释决策背景。
- `docs/plans/*` 可以包含实施细节和草案，ADR 只保留需要长期遵守的约束。
- 状态使用 `Accepted`、`Proposed`、`Superseded`。

## 索引

| ADR | 状态 | 主题 |
| --- | --- | --- |
| [0001](0001-market-data-raw-assets-on-dagster.md) | Accepted | 行情 raw 数据采用 Dagster asset 编排 |
| [0002](0002-s3-parquet-storage-layout.md) | Accepted | S3 Parquet 存储布局和 IO manager 语义 |
| [0003](0003-trade-calendar-driven-market-schedules.md) | Accepted | 市场采集调度以本地交易日历为事实来源 |
| [0004](0004-baostock-tcp-client-and-daily-kline-ranges.md) | Accepted | BaoStock TCP 客户端、分页聚合和日 K 范围过滤 |
| [0005](0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md) | Accepted | Dagster 负责 ClickHouse raw 同步，dbt 负责建模 |
| [0006](0006-clickhouse-python-client-selection.md) | Accepted | ClickHouse raw sync 使用官方 Python HTTP client |
| [0007](0007-dbt-staging-cleaning-boundary.md) | Accepted | dbt staging 清洗边界 |
| [0008](0008-raw-source-profiling-before-dbt-staging.md) | Accepted | dbt staging 前置 raw source profiling |
| [0009](0009-clickhouse-layered-databases.md) | Accepted | ClickHouse 按 dbt 建模层分库 |
| [0010](0010-technical-indicator-field-naming.md) | Accepted | 技术指标字段命名区分窗口参数和算子重数 |
