# ADR 0002: S3 Parquet 存储布局和 IO manager 语义

状态：Accepted

日期：2026-05-27

## 背景

Plan 0001 要求新浪交易日历以最新快照写入 S3，Plan 0002 要求 BaoStock 日频 K 线按年份分区写入 S3。当前实现将路径生成、S3 filesystem、读写和空表保护集中在 `pipeline/scheduler/src/scheduler/defs/util.py` 与 `S3IOManager` 中。

## 决策

raw 数据统一以 Parquet 写入 S3 兼容对象存储，默认对象前缀为 `raw`。

非分区快照资产使用固定对象：

```text
raw/{asset_key}/000000_0.parquet
```

当前快照资产：

```text
raw/sina__trade_calendar/000000_0.parquet
raw/baostock__query_stock_basic/000000_0.parquet
```

分区资产使用 Hive 风格目录：

```text
raw/{asset_key}/{partition_key_name}={partition_key}/000000_0.parquet
```

当前分区资产：

```text
raw/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```

`storage_mode` 只允许两类语义：

- `latest_snapshot`：每次 materialize 覆盖同一个最新快照对象。
- `partitioned`：每次 materialize 写入 Dagster 选中的分区目录。

IO manager 默认必须拒绝空表、拒绝空分区映射，并校验分区输出键与 Dagster asset partition keys 完全一致。

资产可以通过 definition metadata 显式设置 `allow_empty=True` 允许写入 0 行 Parquet。该 opt-in 只放开空表行数限制，不放开 schema、分区映射、分区键一致性或确定性对象路径校验。分区资产写入空表时仍写入带完整 schema 的：

```text
raw/{asset_key}/{partition_key_name}={partition_key}/000000_0.parquet
```

## 依据

当前实现：

- `asset_key_to_parquet_object_key`
- `write_parquet_dataset`
- `read_parquet_table_from_s3`
- `S3IOManager.handle_output`

当前测试覆盖：

- `test_asset_key_to_parquet_object_key_uses_raw_prefix_by_default`
- `test_asset_key_to_parquet_object_key_supports_hive_partition_path`
- `test_write_parquet_dataset_round_trips_unpartitioned_table`
- `test_write_parquet_dataset_round_trips_partitioned_table`

设计来源：

- `docs/plans/0001-sina-trade-calendar-s3-ingestion.md`
- `docs/plans/0002-baostock-aio-tcp-client.md`

## 后果

- S3 对象路径可以从 Dagster asset key 和分区键稳定推导。
- 交易日历和证券基础信息只保留最新快照，不保留历史 snapshot 目录。
- 日频 K 线按 `year` 分区，便于年度回填和当年增量刷新。
- 稀疏数据集可以在 `allow_empty=True` 下写入 schema 完整的 0 行分区文件，materialization metadata 记录 `allow_empty` 和 `empty_partition_keys`。
- 默认空表保护不变，未声明 `allow_empty=True` 的资产仍会失败，避免远端/API 异常把有效数据覆盖为空文件。
- 下游读取 raw 数据应使用这些稳定对象路径，不应直接发起远端 API 请求。
- 如果未来需要保留多版本快照，应新增 ADR，并扩展 `storage_mode`，不能复用 `latest_snapshot` 表达多版本语义。

## 备注

当前 materialization metadata 声明 `compression=zstd`。后续修改 Parquet 写入逻辑时，应保持写入配置和 metadata 一致，并用测试验证实际文件压缩格式。
