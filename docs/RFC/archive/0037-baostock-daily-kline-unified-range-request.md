# RFC 0037: BaoStock 日 K 统一区间请求与取消 mode 分支

日期：2026-06-28

状态：Completed

## 摘要

BaoStock TCP server 当前约束为每日最多约 5 万次调用，并且最多并发 4 个 login。日 K 资产已经从旧的年度 source layout 调整为：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet
  -> source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
  -> clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted
```

当前实现仍通过 `BaostockDailyKlineRunConfig.mode` 区分 `daily` 与 `range_backfill`。本 RFC 建议取消这个人工 mode 分支，让 `source/baostock__query_history_k_data_plus_daily` 始终根据 Dagster 本次 run 的 partition selection 推导请求窗口：

- 单个 `trade_date` partition：`start_date = end_date = partition_key`。
- 多个 `trade_date` partitions：`start_date = min(partition_keys)`，`end_date = max(partition_keys)`。

无论日常调度还是历史回填，都统一按“证券代码 + start_date/end_date”调用 BaoStock `query_history_k_data_plus`，再按 Sina 有效交易日集合过滤和按返回行 `date` 拆分，最终仍写入 daily source 分区。

## 背景

RFC 0030 已明确：历史补数不能直接写 compacted final object，必须落回 daily source partitions，再由 compacted asset 聚合成年分区。

当前代码已经实现了 RFC 0030 的主要数据路径：

- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py` 中 `baostock__query_history_k_data_plus_daily` 是 daily-partitioned asset，并使用 `BackfillPolicy.single_run()`。
- `baostock__query_history_k_data_plus_daily_compacted` 按 year 分区读取 daily source partitions，开启 `require_complete_partitions=True`，并校验 `(date, code)` 唯一性。
- `pipeline/scheduler/src/scheduler/defs/baostock/services.py` 中 `fetch_k_history_tables_for_trade_date_range()` 已支持按证券有效期发起区间请求，并按返回行 `date` 拆分为 `{trade_date: pa.Table}`。
- BaoStock 日 K 远端并发固定为 `BAOSTOCK_DAILY_KLINE_CONNECTIONS = 4`。
- 证券选择只允许 `type in {"1", "2"}`，ETF `type="5"` 不进入日 K 输出。
- ClickHouse raw sync 只消费 `source/baostock__query_history_k_data_plus_daily_compacted`。

因此新的问题不是数据路径缺失，而是 asset 执行入口仍保留两套 mode 语义。

## 当前实现事实

### Dagster asset 和调度

`baostock__query_history_k_data_plus_daily` 当前 run config 为：

```python
class BaostockDailyKlineRunConfig(dg.Config):
    mode: Literal["daily", "range_backfill"] = "daily"
    overwrite_existing_partitions: bool = False
    cutoff_trade_date: str | None = None
```

资产入口 `_materialize_daily_kline()` 当前按 `config.mode` 分支：

- `mode="daily"`：要求本次 run 正好 1 个 partition，然后调用 `_materialize_daily_kline_range()`。
- `mode="range_backfill"`：允许 partition range，然后调用 `_materialize_daily_kline_range_backfill()`。

`baostock__daily_schedule` 只触发 `baostock__daily_job`，并通过 `build_trade_date_schedule()` 在交易日生成单个日分区 run。它没有显式传入 `mode`，因此使用默认 `daily`。

### daily 路径

当前 daily 路径通过 `materialize_trade_date_range()` 执行，但内部 `max_concurrent_trade_dates=1`，且 `_materialize_daily_kline()` 已经保证只有一个 partition。

对每个证券调用：

```text
code = <security code>
start_date = trade_date
end_date = trade_date
```

该路径会立即写入对应 `trade_date=YYYY-MM-DD` 分区，并记录 `source_mode="daily"`。

### range_backfill 路径

当前 range backfill 路径已经接近目标形态：

1. 从 `context.partition_keys` 推导 natural date range。
2. 可选用 `cutoff_trade_date` 收敛结束日期。
3. 从 S3 读取 Sina trade calendar，得到目标有效交易日集合。
4. 调用 `fetch_k_history_tables_for_trade_date_range()`。
5. 对每个有效证券按有效期交集请求：

```text
code = <security code>
start_date = max(range_start, security_effective_start)
end_date = min(range_end, security_effective_end)
```

6. 校验返回行 `date` 必须在目标交易日集合中。
7. 校验每个输出分区内 `(date, code)` 无重复。
8. 为目标交易日生成 daily partition table，包括空表。
9. 默认拒绝覆盖已存在 daily partition，除非 `overwrite_existing_partitions=true`。
10. 写入 `source/baostock__query_history_k_data_plus_daily/trade_date=*`。

该路径记录 `source_mode="range_backfill"`。

### compacted 和 raw sync

`baostock__query_history_k_data_plus_daily_compacted` 当前按 year 读取 daily partitions，并写：

```text
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

它不读取旧 compacted，也不读取 private base prefix。ClickHouse raw sync 继续从 compacted year partition 导入。

## 设计差异

| 主题 | 当前实现 | 目标设计 |
| --- | --- | --- |
| 执行模式选择 | 依赖 `config.mode`，默认 `daily` | 不暴露 `mode`，只由本次 Dagster partition selection 决定 |
| 单日增量 | `mode="daily"`，单分区，逐证券请求单日 | 单分区自动退化为 range 请求，`start_date=end_date=partition_key` |
| 历史回填 | `mode="range_backfill"`，多分区，逐证券请求区间 | 多分区自动使用 range 请求，`start_date=min(partitions)`、`end_date=max(partitions)` |
| 写盘路径 | daily 和 range_backfill 两个 asset 内部分支 | 一个统一 materialization path，统一写 daily partition |
| metadata | `source_mode="daily"` 或 `source_mode="range_backfill"` | 不记录人工 mode；可记录派生的 `request_start_date`、`request_end_date`、`requested_partition_count`、`processed_trade_dates` |
| 操作入口 | 回填必须记住配置 `mode="range_backfill"` | 回填只需要选择 partition range；覆盖和 cutoff 仍可作为独立安全配置 |

## 决策

取消 `BaostockDailyKlineRunConfig.mode`。

`baostock__query_history_k_data_plus_daily` 统一执行以下流程：

1. 读取 `context.partition_keys`，必须至少包含一个 partition。
2. 解析 partition keys 为 natural dates。
3. 按 `cutoff_trade_date` 收敛结束日期，如果保留该安全配置。
4. 从 Sina trade calendar 过滤目标有效交易日。
5. 用 `range_start`、`range_end` 和目标交易日集合调用 `fetch_k_history_tables_for_trade_date_range()`。
6. 服务层按证券有效期交集请求 BaoStock。
7. 服务层按返回行 `date` 拆分为 daily partition tables。
8. 写入 `source/baostock__query_history_k_data_plus_daily/trade_date=*`。

这意味着日常调度和历史回填没有业务模式差异，只有 partition selection 的窗口大小不同。

## 保留的控制项

取消 `mode` 不等于取消所有 run config。以下配置仍有明确职责，可以保留：

- `overwrite_existing_partitions`：是否允许覆盖已存在 daily partition，默认仍应为 `false`。
- `cutoff_trade_date`：当前年或手动修复场景下的安全截止日期。它只收敛窗口，不决定执行模式。

如果后续确认所有手动回填都可以通过精确 partition selection 表达截止日期，则可以再单独移除 `cutoff_trade_date`。

## BaoStock TCP 超时与服务端不稳定

当前 BaoStock client 已经对 TCP connect、login、request write/read 包裹 timeout，并对 `BaostockNetworkError` 做 request 级重试。但这些参数仍是代码常量：

```text
CONNECT_TIMEOUT_SECONDS = 5
REQUEST_TIMEOUT_SECONDS = 30
LOGIN_TIMEOUT_SECONDS = 15
MAX_REQUEST_ATTEMPTS = 4
```

针对当前 BaoStock TCP server 不稳定的运行事实，本 RFC 建议先调整为：

```text
CONNECT_TIMEOUT_SECONDS = 15
REQUEST_TIMEOUT_SECONDS = 20
LOGIN_TIMEOUT_SECONDS = 15
MAX_REQUEST_ATTEMPTS = 4
```

理由：

- connect timeout 从 5s 提高到 15s，给服务端连接建立和 TCP 握手抖动更多恢复空间，减少短暂网络抖动导致的误失败。
- request read/write timeout 从 30s 降到 20s，避免服务端已接受连接但单个 query 卡住时，一个证券请求长期占用 4 个有限连接之一。
- login timeout 暂不调整，仍保持 15s；login 既可能受服务端状态影响，也直接决定连接是否可复用。

后续应将这些值迁移为 `BaostockClientFactoryResource` 配置或环境变量，而不是继续硬编码。

### 失败放大场景

当前设计不会瞬时把 5K 个证券并发打到 BaoStock server；并发仍由 `BAOSTOCK_DAILY_KLINE_CONNECTIONS = 4` 和连接池限制为 4。

但当前设计在服务端持续不稳定时，可能会把约 5K 个证券**逐个尝试完**。原因是：

1. 日 K 服务层用 `BoundedTaskRunner(max_concurrent_tasks=4, max_failure_ratio=0)` 调度所有目标证券。
2. `BoundedTaskRunner` 默认 `fail_fast=False`。
3. 失败阈值校验发生在 `asyncio.gather(...)` 等所有证券任务结束之后。
4. 单个证券请求在 client 内最多重试 4 次；每次网络错误会关闭连接并重新借用或重建连接。

因此以下场景会导致“打满证券列表”：

- BaoStock server 可连接但多数 `query_history_k_data_plus` 请求 read timeout。
- BaoStock server 间歇性接受连接，但 login 或 query 经常超时。
- 网络或服务端异常持续时间覆盖整次日 K run，导致每个证券都经历 4 次失败后才计入失败。
- 服务端返回连接后挂起，不快速拒绝请求；此时每个证券都会消耗 request timeout。

这类场景下，虽然同时最多只有 4 条连接，但总调用尝试可能接近：

```text
selected_security_count * MAX_REQUEST_ATTEMPTS
```

对日常约 5K 个证券来说，最坏情况下可能接近 2 万次请求尝试，且 run 会拖到证券列表耗尽后才失败。

### 建议的保护策略

timeout 调整只能降低单次卡顿成本，不能阻止失败放大。后续实现应增加任务级 fail-fast 或 circuit breaker：

- 网络类错误连续达到阈值后停止调度后续证券。
- 失败数量达到小阈值时提前失败，例如连续 20 个 `BaostockNetworkError`。
- 失败率在 warm-up 窗口内明显异常时提前失败，例如前 100 个证券失败率超过 20%。
- 对业务响应错误和网络错误分开统计；网络错误触发熔断，单个证券业务错误仍按当前 all-or-nothing 失败处理。

即使增加 fail-fast，也应保持写盘 all-or-nothing：只要目标证券请求未全部成功并通过校验，就不写 daily partition，避免生成缺证券的 K 线分区。

## 非目标

- 不改变 daily source S3 layout。
- 不改变 compacted year layout。
- 不改变 ClickHouse raw 表名和 raw sync asset。
- 不改变 dbt staging source。
- 不重新引入旧的 `year=YYYY` daily source layout。
- 不让 compacted asset 读取旧 compacted 或 private base prefix。
- 不放宽 BaoStock 并发限制；仍使用最多 4 个连接。
- 不采集 `type="5"` ETF。

## 迁移建议

1. 删除 `BaostockDailyKlineRunConfig.mode` 字段。
2. 删除 `_materialize_daily_kline()` 中的 mode 分支。
3. 将 `_materialize_daily_kline_range_backfill()` 改名为中性名称，例如 `_materialize_daily_kline_partition_selection()`。
4. 让单分区和多分区都走当前 range backfill 服务路径。
5. daily metadata 删除 `source_mode`，或改为派生字段，例如：

```text
request_start_date
request_end_date
requested_partition_count
processed_trade_date_count
processed_trade_dates
partition_row_counts
empty_partition_keys
selected_security_count
max_connections
max_concurrent_security_requests
overwrite_existing_partitions
overwritten_partition_keys
```

6. 更新测试：
   - 删除 `test_daily_mode_rejects_multiple_partitions`。
   - 新增“多分区未传 mode 也会走区间请求”的测试。
   - 新增“单分区通过统一路径请求 `start_date=end_date`”的测试。
   - 新增 TCP timeout 常量或 resource 配置测试：connect timeout 为 15s，request timeout 为 20s。
   - 新增服务端持续 timeout 时提前停止调度后续证券的测试。
   - 保留已有日期越界、重复 `(date, code)`、空交易日、覆盖保护和 partial write repair 测试。
7. 更新 Dagster runbook，删除手动回填命令中的 `mode="range_backfill"` 配置。

## 风险与约束

- 当前 range path 会先完成远端请求和校验，再写多个 daily partitions；如果写盘过程中部分失败，现有代码会记录 repair context，但不是严格 atomic promotion。取消 mode 不会引入这个风险，但会让日常单日也走同一写盘路径。
- 如果 operator 选择了过大的 partition range，单次 run 会按证券区间请求并在内存中拆表。仍需要运维上控制回填窗口大小。
- DailyPartitionsDefinition 包含自然日，目标交易日集合必须继续由 Sina trade calendar 收敛，非交易日不能请求 BaoStock，也不能写误导性的行情分区。
- `BackfillPolicy.single_run()` 必须保留，否则 Dagster 可能把一个历史窗口拆成逐日 run，导致远端调用退化。
- 仅调整 timeout 不能解决服务端持续不可用时的失败放大；必须配合任务级 fail-fast 或 circuit breaker，避免把完整证券列表逐个重试完。

## 验收标准

- `baostock__query_history_k_data_plus_daily` 不再有 `mode` run config。
- 日常 schedule 触发的单分区 run 成功写入单个 `trade_date` 分区。
- 手动选择多个 `trade_date` partitions 时，不需要传 `mode`，仍按证券发起区间请求。
- 多分区 run 的 BaoStock 调用次数近似为目标证券数，而不是目标证券数乘以交易日数。
- 返回结果仍按 `date` 拆为 daily partitions。
- 非交易日 partition 被 trade calendar 跳过。
- 已存在分区默认拒绝覆盖，显式覆盖时记录覆盖范围。
- compacted asset 仍只读取 daily partitions，并在要求完整性时拒绝缺失分区。
- ClickHouse raw sync 和 dbt staging 不需要改名或改 source。
- BaoStock TCP connect timeout 调整为 15s，request read/write timeout 调整为 20s。
- BaoStock server 持续 timeout 时，日 K run 不应继续调度完整证券列表；应在明确的网络失败阈值后提前失败。

## 证据

本 RFC 基于以下当前代码和文档核对：

- `docs/RFC/archive/0030-baostock-daily-kline-compacted-yearly-range-rebuild.md`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/client.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/schedules.py`
- `pipeline/scheduler/src/scheduler/defs/common/concurrency.py`
- `pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py`
- `pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py`
- `pipeline/scheduler/tests/unit/baostock/test_baostock.py`
