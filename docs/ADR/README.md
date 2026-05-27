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
