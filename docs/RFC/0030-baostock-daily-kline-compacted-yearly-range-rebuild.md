# RFC 0030: BaoStock 日 K compacted 年度范围回填任务设计

日期：2026-06-25

状态：Active

## 摘要

本文定义一个专门用于 BaoStock 日 K 历史修复和初始化补数的年度范围回填任务。该任务直接使用 BaoStock `query_history_k_data_plus` 的 `start_date` / `end_date` 能力，按证券代码拉取目标年份的日频不复权 K 线，生成年度 compacted Parquet，并在校验通过后写入现有 compacted S3 目录：

```text
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

该任务不替代日常 `trade_date` 分区 source asset，也不作为常规调度链路运行。它是显式手动触发的 rebuild/backfill 工具，用于避免历史补数必须先 materialize 大量 daily partitions 再 compact。

## 背景

Plan 0054 已将 BaoStock 日 K 链路拆分为：

1. `source/baostock__query_history_k_data_plus_daily`：按 `trade_date` 分区的日常增量 source asset。
2. `source/baostock__query_history_k_data_plus_daily_compacted`：按 `year` 分区的年度 compacted source asset。
3. `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`：从 compacted S3 年分区同步到 ClickHouse raw。

这个形态适合未来交易日增量，但历史补齐时存在明显成本：如果完全通过 daily source 回填，必须为每个交易日生成一个分区，再由 compacted asset 合并成年度文件。BaoStock K 线接口本身支持 `start_date` 和 `end_date`，历史场景可以按“单个证券代码 + 年度日期范围”请求，直接得到一年内的多日数据。

因此需要一条受控的手动回填路径：利用 BaoStock 年度范围请求提高历史补数效率，但最终仍产出同一个 compacted 数据集和同一个 S3 final layout，保证 ClickHouse raw sync 和 dbt staging 不需要再分叉。

## 决策

新增一个手动专用 Dagster job，用于按 year rebuild BaoStock 日 K compacted 分区。

建议命名：

```text
baostock__query_history_k_data_plus_daily_compacted_yearly_rebuild_job
```

该 job 可以由 op 组成，也可以实现为一个 distinct asset key 的 rebuild/promotion asset。但不应在 Dagster asset graph 中新增另一个与现有 compacted asset 相同 asset key 的普通资产。原因是同一逻辑数据集如果存在两个常规资产写入者，会削弱 lineage、重算归属和故障排查边界。

最终写入目标仍是现有 compacted S3 prefix。写入必须采用 temp-then-promote：

```text
# 临时输出
source/baostock__query_history_k_data_plus_daily_compacted/_tmp/yearly_rebuild/<run_id>/year=YYYY/000000_0.parquet

# 校验通过后的最终对象
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

## 目标

- 支持按 year 手动重建 BaoStock 日 K compacted 分区。
- 使用 BaoStock `start_date` / `end_date` 范围请求，避免历史补数必须逐日 materialize。
- 最终产物写入现有 compacted S3 目录，保持 raw sync 和 dbt source 路径不变。
- 复用 compacted dataset contract schema。
- 保持证券范围与 Plan 0054 一致，只采集 BaoStock `type in {"1", "2"}`，不采集 ETF `type="5"`。
- 在替换最终对象前执行明确校验，避免不完整数据覆盖已存在年分区。
- 替换后触发或要求执行对应 year 的 ClickHouse raw sync。

## 非目标

- 不替代日常 `trade_date` 分区 source asset。
- 不挂 schedule、sensor 或 eager automation。
- 不通过该 job 写入 daily source prefix。
- 不重新引入旧的 `source/baostock__query_history_k_data_plus_daily/year=YYYY` layout。
- 不为 ETF `type="5"` 补建行情资产。
- 不改变 dbt staging model 名称或下游 `ref()`。

## 任务输入

必填输入：

```text
year = YYYY
```

派生输入：

```text
start_date = YYYY-01-01
end_date = YYYY-12-31
```

如果目标 year 是当前年份，`end_date` 应按当前可用交易日边界收敛，不能请求未来日期并把空结果当作完整年份。具体边界应来自 `sina__trade_calendar` 或 operator 明确传入的 cut-off trade date。

## 执行流程

1. 读取 BaoStock stock basic 最新快照。
2. 根据目标年份筛选当年有效证券，证券类型只允许 `{"1", "2"}`。
3. 对每个证券调用 BaoStock 日 K 接口：

```text
code = <security code>
start_date = <year window start>
end_date = <year window end>
frequency = "d"
adjustflag = "3"
```

4. 合并所有证券结果。
5. 按 `baostock__query_history_k_data_plus_daily_compacted` contract schema cast 和排序。
6. 写入临时 S3 key。
7. 校验临时对象。
8. 校验通过后复制或上传到最终 S3 key：

```text
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

9. 记录 materialization metadata。
10. 对同一 year 运行 ClickHouse raw sync。

## 校验要求

临时对象 promotion 前必须满足：

- Parquet schema 等于 `baostock__query_history_k_data_plus_daily_compacted` contract schema。
- 所有 `date` 落在目标 year 和本次 cut-off 范围内。
- 所有证券均来自 `type in {"1", "2"}` 的 stock basic 选择结果。
- `type="5"` ETF 不进入输出。
- `frequency="d"`、`adjustflag="3"` 的语义不变。
- 行数大于 0，除非目标年份在证券市场开始前或被显式声明为空年份。
- 当前年份必须记录 cut-off trade date，不能把半年度或单证券快照误标为完整年份。

建议额外记录但不作为通用硬阈值：

- `selected_security_count`
- `requested_security_count`
- `successful_security_count`
- `failed_security_count`
- `row_count`
- `min(date)`
- `max(date)`
- `uniq(code)`

## S3 替换策略

该 job 不应先删除最终对象再写新对象。必须先写临时对象并完成校验，再替换最终对象。

如果 S3 bucket 开启 versioning，metadata 应记录被替换对象的 version id、size 和 ETag。如果未开启 versioning，也应记录替换前对象是否存在、size、ETag 和 row count，便于回滚和审计。

最终 compacted 年分区当前是单文件对象：

```text
year=YYYY/000000_0.parquet
```

本 RFC 不引入多文件 output。保持单文件可以让现有 ClickHouse raw sync 继续按确定 object key 读取。

## 并发与运行边界

- 该 job 必须手动触发。
- 同一时间只允许一个 BaoStock 日 K compacted rebuild 运行。
- 不允许与同 year 的日常 compacted asset materialization 并发写最终对象。
- BaoStock 远端请求并发沿用日 K 链路的固定连接池策略。
- 每个 BaoStock TCP 连接必须独立登录，未登录连接不得承载数据请求。

Dagster metadata 至少记录：

```text
source_mode = "baostock_range_backfill"
year
start_date
end_date
cutoff_trade_date
selected_security_types = ["1", "2"]
selected_security_count
max_connections
row_count
temp_s3_key
final_s3_key
previous_final_object
```

## 与现有资产的关系

日常链路保持不变：

```text
trade_date daily source -> yearly compacted -> ClickHouse raw sync -> dbt staging
```

年度范围 rebuild job 是一条手动 repair path：

```text
BaoStock yearly range fetch -> temp compacted parquet -> validated promotion -> same yearly compacted final key -> ClickHouse raw sync
```

这意味着最终 compacted S3 目录是 ClickHouse raw sync 的唯一输入事实源，但它可能由两种受控流程产生：

1. 日常 compact asset 从 daily partitions 聚合产生。
2. 手动 yearly rebuild job 从 BaoStock 范围请求直接产生。

两者不能在同一 year 上并发运行。每次手动 rebuild 后，必须把 Dagster run id 和 metadata 作为该 year 的审计事实。

## 后续实施建议

1. 新增 rebuild job 和服务层函数，服务层只负责生成目标 year 的 compacted table。
2. 新增 temp-then-promote S3 写入工具或在现有 storage service 中增加明确方法。
3. 新增单元测试覆盖：
   - 年度请求传入 `start_date` 和 `end_date`。
   - `type="5"` 被过滤。
   - 输出 schema 等于 compacted contract schema。
   - 校验失败不会覆盖最终对象。
   - 成功 promotion 后 final key 为 `_compacted/year=YYYY/000000_0.parquet`。
4. 新增文档 runbook，说明如何选择 year、如何检查 row count、如何运行 raw sync。
5. 对 2026 年这类已知不完整快照，使用该 job 单独 rebuild 并在 job report 中记录完整性验收。

## 验收标准

- `dg list defs` 能看到手动 yearly rebuild job。
- rebuild job 不被任何 schedule、sensor 或 automation 触发。
- 对单个 year 执行 rebuild 后，S3 final key 与 compacted contract layout 一致。
- 校验失败时 final key 不变化。
- 校验成功后 ClickHouse raw sync 可以读取同一个 compacted year 分区。
- dbt staging 仍只读取 `baostock__query_history_k_data_plus_daily_compacted` raw source。

