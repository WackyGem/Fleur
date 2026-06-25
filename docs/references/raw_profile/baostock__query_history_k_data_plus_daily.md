# Raw 数据画像：baostock__query_history_k_data_plus_daily

日期：2026-06-25

状态：Superseded

`baostock__query_history_k_data_plus_daily` 已在 Plan 0054 中改为 source-only daily `trade_date` 分区资产，不再作为 ClickHouse raw/dbt source 读取。

当前 staging model `stg_baostock__query_history_k_data_plus_daily` 的 active raw source 是：

- dbt source：`source('raw', 'baostock__query_history_k_data_plus_daily_compacted')`
- raw profile：`docs/references/raw_profile/baostock__query_history_k_data_plus_daily_compacted.md`
- 数据契约：`pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily_compacted.yml`
- 迁移报告：`docs/jobs/reports/2026-06-25-baostock-daily-kline-compaction.md`

保留本文件仅用于旧文档链接的迁移指引。
