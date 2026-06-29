# Plan 0063: BaoStock 日 K 统一区间请求与 TCP 超时防护实施计划

日期：2026-06-28

状态：Completed

## 背景

RFC 0037 确定 BaoStock 日 K asset 不应继续依赖 `mode="daily" | "range_backfill"` 区分增量和回填。Dagster 已经通过 partition selection 提供执行窗口：

- 单个 `trade_date` partition 表示日常增量。
- 多个 `trade_date` partitions 表示历史或修复窗口。

当前服务层已经具备核心能力：按证券有效期与请求窗口交集发起 `query_history_k_data_plus` 区间请求，再按返回行 `date` 拆成 daily source partitions。剩余实施重点是收敛 asset 入口、去掉人工 mode，并补强 BaoStock TCP server 不稳定时的 timeout 和 fail-fast 防护。

相关依据：

- RFC：[0037-baostock-daily-kline-unified-range-request.md](../../RFC/archive/0037-baostock-daily-kline-unified-range-request.md)
- 当前实现：
  - `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
  - `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
  - `pipeline/scheduler/src/scheduler/defs/baostock/client.py`
  - `pipeline/scheduler/src/scheduler/defs/common/concurrency.py`

## 目标

- 删除 `BaostockDailyKlineRunConfig.mode`。
- `baostock__query_history_k_data_plus_daily` 统一由 `context.partition_keys` 推导请求窗口。
- 单分区 run 请求 `start_date=end_date=partition_key`。
- 多分区 run 请求 `start_date=min(partition_keys)`、`end_date=max(partition_keys)`，并按 Sina trade calendar 收敛有效交易日集合。
- 保持写盘目标不变：只写 `source/baostock__query_history_k_data_plus_daily/trade_date=*`。
- 保持 compacted 和 raw sync 路径不变。
- 调整 BaoStock TCP timeout：
  - connect timeout：`15s`
  - request read/write timeout：`20s`
  - login timeout：`15s`
  - request attempts：`4`
- 增加服务端持续不稳定时的任务级 fail-fast 或 circuit breaker，避免把完整证券列表逐个重试完。

## 非目标

- 不改变 S3 daily source layout。
- 不改变 yearly compacted layout。
- 不改变 ClickHouse raw 表名、raw sync asset 或 dbt source。
- 不改变 BaoStock 日 K 字段 contract。
- 不采集 ETF `type="5"`。
- 不放宽 BaoStock 并发限制；仍最多 4 个 TCP login / connection。
- 不允许部分证券成功时写出不完整 daily partition。

## 当前差异

| 主题 | 当前实现 | 目标实现 |
| --- | --- | --- |
| 执行入口 | `config.mode` 分支到 daily 或 range_backfill | 单一 partition-selection 路径 |
| 单分区 | daily path，经 `materialize_trade_date_range()` 单日写盘 | 统一 range path，`start_date=end_date` |
| 多分区 | 必须传 `mode="range_backfill"` | 无需 mode，partition range 自动决定 |
| timeout | connect 5s，request 30s，login 15s | connect 15s，request 20s，login 15s |
| 网络失败 | 最多 4 并发，但默认会把所有证券任务调度完再失败 | 达到网络失败阈值后提前停止调度 |

## 设计约束

- `BackfillPolicy.single_run()` 必须保留，避免 partition range 被拆成逐日 run。
- 有效交易日集合必须来自 `sina__trade_calendar`。
- 非交易日 partition 不请求 BaoStock。
- 返回行 `date` 必须在目标交易日集合内；否则失败。
- 每个输出分区内 `(date, code)` 必须唯一。
- 默认拒绝覆盖已存在 daily partition；显式覆盖继续由 `overwrite_existing_partitions` 控制。
- 所有证券请求和分区校验成功后才允许写盘。
- 网络 fail-fast 只阻止继续请求，不改变 all-or-nothing 写盘语义。

## 实施阶段

### Phase 1: 收敛 run config 和 asset 入口

修改范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/tests/unit/baostock/test_baostock.py`

开发任务：

1. 从 `BaostockDailyKlineRunConfig` 删除 `mode` 字段。
2. 保留：
   - `overwrite_existing_partitions: bool = False`
   - `cutoff_trade_date: str | None = None`
3. 删除 `_materialize_daily_kline()` 中基于 `config.mode` 的分支。
4. 将 `_materialize_daily_kline_range_backfill()` 改名为中性名称，例如 `_materialize_daily_kline_partition_selection()`。
5. 让单分区和多分区都进入统一函数。
6. metadata 删除 `source_mode`，改为记录派生事实：
   - `request_start_date`
   - `request_end_date`
   - `requested_partition_count`
   - `processed_trade_date_count`
   - `processed_partition_keys`
   - `cutoff_trade_date`
   - `effective_cutoff_trade_date`

完成标准：

- 单分区 schedule run 不需要 run config。
- 多分区 launch 不需要 `mode="range_backfill"`。
- 所有 metadata 字段表达事实窗口，不表达人工 mode。

### Phase 2: 统一请求窗口和日分区写盘

修改范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
- `pipeline/scheduler/tests/unit/baostock/test_baostock.py`

开发任务：

1. 从 `context.partition_keys` 推导 sorted natural dates。
2. 计算：
   - `range_start = min(partition_dates)`
   - `range_end = max(partition_dates)`
3. 如果配置了 `cutoff_trade_date`：
   - 大于 partition range end 时失败。
   - 小于 range end 时收敛 range end。
4. 读取 Sina trade calendar。
5. 生成目标交易日集合：
   - 在 partition selection 内。
   - 在 `range_start..range_end` 内。
   - 属于 Sina 有效交易日。
6. 调用 `fetch_k_history_tables_for_trade_date_range()`。
7. 保持按 `date` 拆分、重复键校验、空表分区和覆盖检查。
8. 使用 `S3DatasetService.write_partitioned()` 写 daily partitions。

完成标准：

- 单分区请求传入 `start_date=end_date=partition_key`。
- 多分区请求按证券有效期传入区间。
- 非交易日被跳过且不会写 misleading partition。
- 目标交易日每个都有输出分区，包括空表。
- 已存在分区默认失败；显式覆盖记录 `overwritten_partition_keys`。

### Phase 3: TCP timeout 调整和可配置化

修改范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/client.py`
- `pipeline/scheduler/src/scheduler/defs/resources/baostock.py`
- `pipeline/scheduler/src/scheduler/defs/config/models.py`
- `pipeline/scheduler/src/scheduler/defs/config/env.py`
- `.env.example`
- `pipeline/scheduler/tests/unit/baostock/test_baostock.py`

开发任务：

1. 将默认 timeout 调整为：
   - connect：15s
   - request：20s
   - login：15s
   - max attempts：4
2. 优先将这些值放进 `BaostockClientConfig` 和 `BaostockClientFactoryResource`，而不是继续只用模块常量。
3. 增加环境变量默认入口：
   - `BAOSTOCK_CONNECT_TIMEOUT_SECONDS`
   - `BAOSTOCK_REQUEST_TIMEOUT_SECONDS`
   - `BAOSTOCK_LOGIN_TIMEOUT_SECONDS`
   - `BAOSTOCK_MAX_REQUEST_ATTEMPTS`
4. `BaostockAioTcpClient` 使用 config 中的 timeout 和 attempts。
5. 测试覆盖默认值和 resource 传参。

完成标准：

- 默认 connect timeout 为 15s。
- 默认 request timeout 为 20s。
- timeout 参数能从 resource/config 进入 client。
- 现有 retry 行为不退化。

### Phase 4: 网络 fail-fast / circuit breaker

修改范围：

- `pipeline/scheduler/src/scheduler/defs/common/concurrency.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
- `pipeline/scheduler/tests/unit/common/test_concurrency.py`
- `pipeline/scheduler/tests/unit/baostock/test_baostock.py`

开发任务：

1. 扩展 `BoundedTaskOptions`，支持网络失败提前停止调度。建议字段：
   - `fail_fast_error_types: tuple[type[BaseException], ...]`
   - 或专用 `max_failure_count_before_stop: int`
   - 或 `failure_window_threshold`，按前 N 个任务失败率熔断。
2. 在 BaoStock 日 K 服务层对 `BaostockNetworkError` 启用 fail-fast 策略。
3. 推荐第一版保守实现：
   - 连续或累计 `BaostockNetworkError` 达到 20 个后停止调度剩余证券。
   - 已经开始执行的最多 4 个任务允许结束。
   - run 最终失败，不写盘。
4. 保持业务错误和网络错误分开统计。
5. metadata 中记录：
   - `network_failure_count`
   - `circuit_breaker_triggered`
   - `skipped_due_to_circuit_breaker_count`

完成标准：

- BaoStock server 持续 timeout 时，不会把完整 5K 证券列表逐个请求完。
- 同时并发仍最多 4。
- fail-fast 后不写 daily partition。
- 错误信息能说明触发阈值和失败样本。

### Phase 5: runbook 和文档收敛

修改范围：

- `docs/skills/fleur-dagster-backfill-runbook/references/backfill-matrix.md`
- `docs/skills/fleur-dagster-backfill-runbook/SKILL.md`
- `docs/jobs/reports/` 运行报告按实际执行补充
- 可选更新 `docs/RFC/archive/0037-baostock-daily-kline-unified-range-request.md`

开发任务：

1. 删除手动回填命令中的 `mode="range_backfill"`。
2. 保留 `overwrite_existing_partitions` 和 `cutoff_trade_date` 示例。
3. 增加 timeout 和 circuit breaker 参数说明。
4. 记录日常 run、历史 partition range run、compacted、raw sync 的执行顺序。

完成标准：

- 文档中的回填命令与新 run config 一致。
- operator 不需要记忆 `mode`。
- 服务端不稳定时的失败形态和修复步骤有记录。

## 测试矩阵

| 场景 | 期望 |
| --- | --- |
| 单个交易日 partition | 调用区间服务，`start_date=end_date=partition_key` |
| 多个交易日 partition | 调用区间服务，按证券有效期请求窗口交集 |
| 多分区不传 mode | 成功进入统一路径 |
| 传入旧 `mode` 配置 | Dagster config 校验失败或测试确认字段已不存在 |
| cutoff 晚于 partition range end | 失败 |
| cutoff 早于 range end | 只处理 cutoff 及之前有效交易日 |
| 非交易日 partition | 跳过，不请求 BaoStock |
| 返回非目标交易日行 | 失败 |
| 重复 `(date, code)` | 失败 |
| 已存在 partition 且未允许覆盖 | 失败，不写盘 |
| 已存在 partition 且允许覆盖 | 写盘并记录覆盖 keys |
| connect timeout 默认值 | 15s |
| request timeout 默认值 | 20s |
| 持续网络 timeout | 触发 circuit breaker，不调度完整证券列表，不写盘 |

## 验证命令

代码变更后至少运行：

```bash
cd pipeline

uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests/unit/baostock/test_baostock.py scheduler/tests/unit/common/test_concurrency.py -q
uv run pytest scheduler/tests/integration/test_definitions_and_schedules.py -q

cd scheduler
uv run dg check defs
uv run dg list defs --json
```

文档变更运行：

```bash
make docs-check
git diff --check
```

## 手动验收建议

在 dev 环境选择一个较小窗口，例如 2 个交易日：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-06-24...2026-06-25"
```

预期：

- 不需要 `mode` config。
- materialization metadata 中有 `request_start_date`、`request_end_date` 和 `processed_trade_dates`。
- S3 只新增或覆盖对应 `trade_date=*` daily partitions。

随后运行当前年 compacted 和 raw sync：

```bash
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026

uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026
```

## 完成标准

- RFC 0037 的执行入口设计落地：无 `mode`，只由 partition selection 决定请求窗口。
- BaoStock 日 K 单分区和多分区 run 均通过统一路径。
- 默认 timeout 已调整为 connect 15s、request 20s。
- 持续网络 timeout 不会把完整证券列表逐个请求完。
- all-or-nothing 写盘语义保持不变。
- compacted、ClickHouse raw sync 和 dbt staging 不需要改名或改 source。
- 测试和 `dg check defs` 通过。
