# ADR 0005: Dagster 负责 ClickHouse raw 同步，dbt 负责建模

状态：Accepted

日期：2026-05-31

## 背景

RFC 0009 定义了从 S3 Parquet source assets 同步到 ClickHouse raw 层的设计。当前 source 层已经形成三类稳定输入：

- latest snapshot：例如 `sina__trade_calendar`、`baostock__query_stock_basic`、`jiuyan__industry_list`、`jiuyan__industry_ocr_snapshot`。
- 年度分区：例如 `baostock__query_history_k_data_plus_daily`、`eastmoney__*`。
- 由日分区 compact 后形成的年度资产：例如 `jiuyan__action_field_compacted`、`ths__limit_up_pool_compacted`。

这些 raw 同步需要处理 S3 object key 推导、Dagster partition key、ClickHouse staging table、row count/schema 校验、分区替换或 snapshot 替换，以及失败时保持旧 raw 表可用。

项目也计划引入 dagster-dbt。dagster-dbt 能把 dbt models、seeds、snapshots 映射为 Dagster assets，并继承 dbt `ref()`、`source()` lineage 和 tests-as-checks 能力。因此需要明确：ClickHouse raw 写入应由 Dagster 直接负责，还是由 dbt model/materialization 在 dagster-dbt 编排下负责。

## 决策

ClickHouse raw 层写入由 Dagster raw sync assets 负责。dbt 不负责从 S3 写入 ClickHouse raw，也不负责执行 raw 层的分区替换或 snapshot 替换协议。

分层职责如下：

```text
Dagster source assets
  -> S3 Parquet source objects
  -> Dagster ClickHouse raw sync assets
  -> ClickHouse raw tables
  -> dbt staging models
  -> dbt marts models
```

具体约束：

- Dagster raw sync assets 读取上游 S3 Parquet，写入 ClickHouse staging 表，校验通过后替换生产 raw 表或分区。
- 年度 source assets 使用同一个 `year` partition key 同步到 ClickHouse raw。
- latest snapshot assets 使用完整快照替换协议。
- `jiuyan__industry_ocr` 仍是 OCR work-queue processor，不直接同步到 ClickHouse raw；ClickHouse raw 消费 `jiuyan__industry_ocr_snapshot`。
- dbt 只把 ClickHouse raw tables 声明为 `source()`，在 staging 层做字段命名、类型收敛、轻清洗和基础过滤。
- dbt marts 负责面向查询的宽表、聚合和物化策略。
- dagster-dbt 可以用于把 dbt staging/marts 纳入 Dagster asset graph，但不改变 raw 写入职责。

## 依据

项目设计来源：

- `docs/RFC/archive/0009-dagster-clickhouse-raw-sync.md`
- `docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/plans/0016-jiuyan-industry-ocr-snapshot-source-layer.md`

技术依据：

- dbt `source()` 语义用于描述已经由外部 extract/load 工具加载到 warehouse 的 raw tables，并让 dbt models 从这些 source 读取。
- dagster-dbt 的主要价值是把 dbt project 中的 models、seeds、snapshots 和 tests 映射到 Dagster asset graph，而不是替代自定义外部装载协议。
- ClickHouse raw 同步需要 staging、校验、分区替换和 snapshot 替换。该流程包含外部对象读取、运行时分区、失败恢复和 metadata 记录，比普通 dbt transformation 更接近 ingestion/loading protocol。
- 对 `jiuyan__industry_ocr_snapshot` 这类小规模可变状态输出，完整 snapshot 替换比维护增量去重或频繁 mutation 更简单可恢复。

## 后果

- raw 层边界稳定：ClickHouse raw 是 S3 Parquet source data 的物化副本，不承载业务清洗和 mart 聚合。
- 失败恢复路径明确：如果 ClickHouse raw 表损坏或同步失败，可以从 S3 source asset 重跑对应分区或快照恢复。
- Dagster materialization metadata 必须记录 S3 object key、ClickHouse table、partition key、row count、schema hash 和替换耗时，便于排查同步问题。
- dbt 项目只依赖稳定 ClickHouse raw tables，避免直接读取 S3 或远端 API。
- dagster-dbt 集成应优先用于 dbt staging/marts 的编排、lineage、选择性运行和 dbt tests-as-checks。
- 如果未来某个 raw 输入已经完全存在于 ClickHouse 内部，且只需要简单 SQL 转换，可以在新的 RFC/ADR 中评估是否交给 dbt；不能默认把本 ADR 的 raw sync 协议迁移到 dbt macro 或 custom materialization。
