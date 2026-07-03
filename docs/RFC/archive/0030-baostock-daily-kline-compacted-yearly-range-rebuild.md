# RFC 0030: BaoStock 日 K range backfill 与 compacted 完整性设计

日期：2026-06-25

状态：Active

## 摘要

本文定义 BaoStock 日 K 历史修复和初始化补数的 range backfill 方案。核心目标是让 `source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet` 始终能由完整的 daily source 分区聚合得到，而不是让 yearly rebuild job 直接写 compacted final 对象。

BaoStock `query_history_k_data_plus` 支持 `start_date` / `end_date`。历史补数可以按“证券代码 + 日期区间”请求，避免逐日调用远端接口；但请求结果必须按返回行的 `date` 切分，写回现有 daily source layout：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet
```

随后继续使用现有 yearly compacted asset 聚合 daily partitions：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=* -> source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

这样 `compacted` 的唯一事实来源仍是 daily source partitions。当前年 2026 的缺口可以通过一次 `range_backfill` 覆盖从自然年初到日常调度开始前的候选窗口，实际补齐日期以 Sina trade calendar 中的有效交易日为准，再由日常 `daily` 模式持续写入后续增量。

## 背景

Plan 0054 已将 BaoStock 日 K 链路拆分为：

1. `source/baostock__query_history_k_data_plus_daily`：按 `trade_date` 分区的日常增量 source asset。
2. `source/baostock__query_history_k_data_plus_daily_compacted`：按 `year` 分区的年度 compacted source asset。
3. `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`：从 compacted S3 年分区同步到 ClickHouse raw。

这个形态适合未来交易日增量，但历史补齐时存在成本问题。如果完全通过 daily source 逐日回填，必须对每个交易日单独生成一个分区，并对每个证券逐日请求 BaoStock。BaoStock 接口本身支持日期区间请求，因此历史场景应该复用这个能力。

讨论过程中曾考虑让 yearly rebuild job 直接生成 compacted final 对象，或将 base 段写到 compacted 私有 `_tmp` 前缀后再与 daily 增量合并。但这会引入第二条 compacted 写入路径，使 compacted 同时依赖 daily source 和私有 base 文件，事实边界不清晰。

最终决策是：**base 回填也写入 daily source 分区**。`compacted` 不读取 private base，不读取旧 compacted，也不承担去重合并多个事实源的职责。

## 决策

`baostock__query_history_k_data_plus_daily` 支持两种执行模式：

| 模式 | 触发方式 | BaoStock 请求窗口 | S3 输出 |
| --- | --- | --- | --- |
| `daily` | 交易日 schedule | `start_date = end_date = partition_key` | 单个 `trade_date=YYYY-MM-DD` 分区 |
| `range_backfill` | 手动 Dagster backfill / launch | `start_date = backfill_start`，`end_date = backfill_end`，并按证券有效期收敛 | 多个 `trade_date=YYYY-MM-DD` 分区 |

`range_backfill` 模式必须仍然物化同一个 daily source asset，不新增 `base` asset，不新增独立 S3 prefix 作为长期事实源。

`range_backfill` 必须以 single-run partition range 执行。若 Dagster backfill 被拆成每个 `trade_date` 一个 run，则每个 run 只能看到单日 partition，远端请求会退化为逐日调用，失去本 RFC 的成本优势。`baostock__query_history_k_data_plus_daily` 已使用 `BackfillPolicy.single_run()`，实施时必须保留该策略，并在多分区 run 未显式配置 `mode="range_backfill"` 时失败。

`baostock__query_history_k_data_plus_daily_compacted` 保持现有职责：

```text
read source/baostock__query_history_k_data_plus_daily/trade_date=*
concat/cast/sort
write source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

它不应读取现有 compacted final 对象，也不应读取 `_tmp/base`、`_state/base` 或其他私有中间前缀。若同一 `(date, code)` 在 daily 分区内重复，应在 daily source 写入前或 compacted 聚合时显式报错；本 RFC 不定义静默去重或优先级覆盖规则。

## 目标

- 支持用 BaoStock 区间请求补齐 daily source 历史分区。
- 避免 `2026-01` 到日常调度启动日前的缺口必须逐日远端请求。
- 保持 compacted final layout 不变：

```text
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

- 保持 ClickHouse raw sync 和 dbt staging 路径不变。
- 保持证券范围与 Plan 0054 一致，只采集 BaoStock `type in {"1", "2"}`，不采集 ETF `type="5"`。
- 让 current year 的完整性由“range backfill daily partitions + daily 增量 partitions + yearly compacted”共同保证。
- 保留日常调度只有 daily job 的运维模型；同一个 daily asset 通过 run config 或 backfill context 切换执行模式。

## 非目标

- 不让 yearly rebuild job 直接写 compacted final 对象。
- 不将 base 段长期写入 `_tmp/base`、`_state/base` 或其他非 daily source 前缀。
- 不让 compacted asset 读取旧 compacted 再与 daily 增量做拼接。
- 不重新引入旧的 `source/baostock__query_history_k_data_plus_daily/year=YYYY` layout。
- 不为 ETF `type="5"` 补建行情资产。
- 不改变 dbt staging model 名称或下游 `ref()`。

## 任务输入

日常 `daily` 模式输入：

```text
mode = "daily"
partition_key = YYYY-MM-DD
start_date = partition_key
end_date = partition_key
```

手动 `range_backfill` 模式输入：

```text
mode = "range_backfill"
partition range = YYYY-MM-DD..YYYY-MM-DD
start_date = partition range start
end_date = partition range end
```

如果目标区间包含当前日期，`end_date` 应按当前可用交易日边界收敛，不能请求未来日期并把空结果当作完整数据。具体边界应来自 `sina__trade_calendar` 或 operator 明确传入的 cut-off trade date。

配置约束：

- 单分区 schedule run 可默认使用 `mode="daily"`。
- 多分区 run 必须显式使用 `mode="range_backfill"`。
- `mode="daily"` 遇到多个 `partition_keys` 必须失败。
- `mode="range_backfill"` 遇到单个 partition 可以运行，但仍按区间服务路径处理，便于测试和手动修复单日分区。
- `mode="range_backfill"` 默认不覆盖已存在的 daily source 分区；如需修复已存在分区，必须显式配置 `overwrite_existing_partitions=true` 并在 metadata 中记录覆盖的 partition keys。

## 执行流程

### daily 模式

1. 读取 BaoStock stock basic 最新快照。
2. 根据单个 `trade_date` 筛选当日有效证券，证券类型只允许 `{"1", "2"}`。
3. 对每个证券调用 BaoStock 日 K 接口：

```text
code = <security code>
start_date = <trade_date>
end_date = <trade_date>
frequency = "d"
adjustflag = "3"
```

4. 合并所有证券结果。
5. 按 `baostock__query_history_k_data_plus_daily` contract schema cast。
6. 写入：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet
```

### range_backfill 模式

1. 读取 BaoStock stock basic 最新快照。
2. 从 Dagster partition range 和 Sina trade calendar 得到需要补齐的交易日集合。
3. 根据目标日期区间筛选有效证券，证券类型只允许 `{"1", "2"}`。
4. 对每个证券调用 BaoStock 日 K 接口，使用该证券有效期与回填窗口的交集：

```text
code = <security code>
start_date = max(backfill_start, security_effective_start)
end_date = min(backfill_end, security_effective_end)
frequency = "d"
adjustflag = "3"
```

5. 合并所有证券结果。
6. 校验所有返回行的 `date` 都落在本次交易日集合内。
7. 校验返回结果在每个分区内没有重复 `(date, code)`。
8. 按返回行 `date` 分组，构造 `{trade_date: pa.Table}`。
9. 对本次需要补齐的每个交易日都构造输出分区；若某个有效交易日没有返回行，写入符合 contract schema 的空表，并在 metadata 中记录为空分区。
10. 按 `baostock__query_history_k_data_plus_daily` contract schema cast 每个分区。
11. 检查目标 `trade_date` 分区是否已存在；除非显式允许覆盖，否则遇到已存在分区必须失败。
12. 在所有证券请求、分区级校验和覆盖检查成功后，才并发写入 daily source 分区：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet
```

13. 记录 materialization metadata，包括 `source_mode="range_backfill"`、`backfill_start_date`、`backfill_end_date`、`processed_trade_dates`、`partition_row_counts`、`empty_partition_keys`、`overwritten_partition_keys`、`selected_security_count` 和失败证券计数。

`range_backfill` 对一个窗口采用 all-or-nothing 语义：只要任一证券请求失败、schema cast 失败、日期越界、重复键校验失败或覆盖检查失败，本次 run 不应写入任何 daily 分区。若内存中无法承载完整窗口结果，可以使用短生命周期临时对象完成 staging 和校验；临时对象只能用于本次 run 的 promotion，不能作为长期事实源。

### compacted 聚合

补齐 daily partitions 后，对目标 year 运行现有 compacted asset：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD
  -> source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

随后对同一 year 运行 ClickHouse raw sync。

对 current year 或明确声明要求完整性的 year，compacted materialization 不能只依赖“读到了部分分区也能写出年文件”的默认行为。运行后必须校验目标窗口内的 expected trade_date 分区全部存在；如果 `missing_partition_count > 0`，则不得继续 raw sync。

## 2026 缺口处理

若日常调度从 `2026-06-25` 开始运行，而目标是让 compacted 覆盖 2026 年首个有效交易日到最新可用交易日，则执行：

1. 手动运行 `baostock__query_history_k_data_plus_daily` 的 `range_backfill`，partition range 为 `2026-01-01..2026-06-24`，实际处理日期由 Sina trade calendar 收敛为交易日。
2. 日常 schedule 继续以 `daily` 模式写入 `2026-06-25` 及之后的交易日分区。
3. 运行 `baostock__query_history_k_data_plus_daily_compacted` 的 `2026` 年分区。
4. 运行 `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` 的 `2026` 年分区 raw sync。

这条路径不需要 daily 任务逐日远端请求 2026 年初至 6 月的数据；远端请求按证券区间执行，S3 写入仍按 `trade_date` 分区保存。

## 校验要求

`range_backfill` 写入 daily source 前必须满足：

- Parquet schema 等于 `baostock__query_history_k_data_plus_daily` contract schema。
- 所有 `date` 落在本次 backfill 交易日集合内。
- 所有证券均来自 `type in {"1", "2"}` 的 stock basic 选择结果。
- `type="5"` ETF 不进入输出。
- `frequency="d"`、`adjustflag="3"` 的语义不变。
- 每个输出分区的行只包含对应 `trade_date`。
- 每个输出分区内 `(date, code)` 无重复。
- 本次目标交易日集合中的每个 `trade_date` 都有一个写入结果，允许空表但不允许缺失对象。
- 任一证券请求失败时，不写入 partial daily partitions。
- 默认不覆盖已存在 daily source 分区；显式覆盖时必须记录 `overwrite_existing_partitions=true` 和 `overwritten_partition_keys`。
- 当前年份必须记录 cut-off trade date，不能把部分窗口误标为完整年份。

compacted 聚合后建议校验：

- `year=YYYY` compacted schema 等于 `baostock__query_history_k_data_plus_daily_compacted` contract schema。
- compacted 中 `date` 范围覆盖预期窗口。
- compacted 中 `(date, code)` 无重复。
- `row_count` 等于 daily source 目标 trade_date 分区行数之和。
- 目标窗口内 `missing_partition_count = 0`，除非 operator 明确声明允许缺失。
- `uniq(code)` 不应退化为单证券。

建议记录但不作为通用硬阈值：

- `selected_security_count`
- `requested_security_count`
- `successful_security_count`
- `failed_security_count`
- `row_count`
- `min(date)`
- `max(date)`
- `uniq(code)`
- `empty_partition_keys`
- `duplicate_key_count`
- `missing_partition_count`

## S3 写入策略

`range_backfill` 只写 daily source 分区：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet
```

允许使用短生命周期 staging key 支持 all-or-nothing promotion，但 staging key 必须带 run id，校验失败时不得 promotion，成功或失败后都不得作为下游事实源：

```text
source/baostock__query_history_k_data_plus_daily/_tmp/range_backfill/<run_id>/trade_date=YYYY-MM-DD/000000_0.parquet
```

不写以下位置作为长期事实源：

```text
source/baostock__query_history_k_data_plus_daily_compacted/_tmp/base/...
source/baostock__query_history_k_data_plus_daily_compacted/_state/base/...
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

最终 compacted 年分区仍由 compacted asset 写入：

```text
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

ClickHouse raw sync 只读取这个 final compacted object。

## 并发与运行边界

- `daily` 模式可由交易日 schedule 自动触发。
- `range_backfill` 模式必须手动触发，不挂 schedule、sensor 或 eager automation。
- `range_backfill` 必须以 single-run partition range 执行；禁止把同一回填窗口拆成逐日 run 后再声称完成区间回填。
- 同一 `trade_date` 分区不允许 `daily` 模式和 `range_backfill` 模式并发写入。
- 运行 `range_backfill` 覆盖当前年早期窗口时，应暂停或避开同一日期范围内的 daily materialization。
- BaoStock 远端请求并发沿用日 K 链路的固定连接池策略。
- 每个 BaoStock TCP 连接必须独立登录，未登录连接不得承载数据请求。

Dagster metadata 至少记录：

```text
source_mode = "daily" | "range_backfill"
start_date
end_date
cutoff_trade_date
selected_security_types = ["1", "2"]
selected_security_count
max_connections
row_count
partition_row_counts
empty_partition_keys
overwrite_existing_partitions
overwritten_partition_keys
processed_trade_dates
written_s3_keys
duplicate_key_count
missing_partition_count
```

## 与现有资产的关系

日常链路保持：

```text
trade_date daily source -> yearly compacted -> ClickHouse raw sync -> dbt staging
```

历史补数链路为：

```text
BaoStock range fetch -> split by date -> same trade_date daily source -> yearly compacted -> ClickHouse raw sync -> dbt staging
```

这意味着最终 compacted S3 目录仍只有一个受控生产者：`baostock__query_history_k_data_plus_daily_compacted` asset。BaoStock range backfill 只是 daily source asset 的另一种采集模式。

## 后续实施建议

1. 为 `baostock__query_history_k_data_plus_daily` 增加 run config：
   - `mode: "daily" | "range_backfill"`。
   - 可选 `cutoff_trade_date`，用于当前年窗口收敛。
   - 可选 `overwrite_existing_partitions`，默认 `false`。
   - 多分区 run 缺少 `mode="range_backfill"` 时失败，避免误走逐日路径。
2. 将现有单日 fetch service 扩展为区间 fetch service：
   - daily 模式传入同一天。
   - range_backfill 模式按证券有效期传入区间。
3. 新增按 `date` 切分 `pa.Table` 的服务层函数，输出 `{trade_date: pa.Table}`。
4. 新增 range backfill 校验层：
   - 校验日期窗口、schema、`(date, code)` 唯一性。
   - 为目标窗口内所有交易日生成输出分区，包括空表。
   - 默认拒绝覆盖已存在 daily partition。
   - 任一证券请求或校验失败时阻止 final daily partition 写入。
5. 复用现有 `S3DatasetService.write_partitioned()` 写入多个 `trade_date` 分区；如无法内存校验后直接写 final，新增短生命周期 staging-then-promote 工具。
6. 新增单元测试覆盖：
   - daily 模式请求 `start_date=end_date=partition_key`。
   - daily 模式遇到多个 partition keys 会失败。
   - range_backfill 模式对每个证券请求区间。
   - range_backfill 返回结果按 `date` 切分为 daily partitions。
   - range_backfill 为目标交易日集合中无返回行的日期写空表。
   - range_backfill 检出重复 `(date, code)`。
   - range_backfill 默认拒绝覆盖已存在 daily partition。
   - range_backfill 显式覆盖时记录被覆盖 partition keys。
   - range_backfill 任一证券请求失败时不写 final daily partition。
   - `type="5"` 被过滤。
   - 输出 schema 等于 daily source contract schema。
   - compacted 聚合只读取 daily source，不读取 base/private prefix 或旧 compacted。
   - compacted 在要求完整性的窗口内检出 missing daily partitions 时阻止 raw sync。
7. 新增 runbook，说明如何运行 2026 缺口补齐、如何检查 daily partition row count、如何运行 compacted 和 raw sync。

## 验收标准

- `baostock__query_history_k_data_plus_daily` 支持 `daily` 和 `range_backfill` 两种模式。
- `range_backfill` 以 single-run partition range 执行，多分区 run 未配置该模式时失败。
- `range_backfill` 写入的 S3 object 全部位于 `source/baostock__query_history_k_data_plus_daily/trade_date=*`。
- `range_backfill` 为目标交易日集合中的每个交易日写入一个 daily partition object，包括空表分区。
- `range_backfill` 默认拒绝覆盖已存在 daily partition；显式覆盖时 metadata 记录覆盖范围。
- 不新增长期 base prefix 作为事实源。
- `baostock__query_history_k_data_plus_daily_compacted` 仍只读取 daily source partitions。
- 对 2026 执行 `range_backfill(2026-01-01..2026-06-24)` 后，再运行 2026 compacted，final compacted 覆盖 2026 年首个有效交易日到最新可用交易日。
- range backfill、compacted 完整性或重复键校验失败时不运行 ClickHouse raw sync。
- 校验成功后 ClickHouse raw sync 可以读取同一个 compacted year 分区。
- dbt staging 仍只读取 `baostock__query_history_k_data_plus_daily_compacted` raw source。
