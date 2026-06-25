# Plan 0055: BaoStock 日 K range backfill 实施计划

日期：2026-06-25

状态：Completed

## 背景

RFC 0030 确定 BaoStock 日 K 历史补数不再由 yearly rebuild job 直接写 `source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet`。历史补数必须落回现有 daily source 分区：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet
```

随后由现有 yearly compacted asset 聚合成：

```text
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

相关代码入口：

- `baostock__query_history_k_data_plus_daily` 定义在 `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`，使用 `DailyPartitionsDefinition` 和 `BackfillPolicy.single_run()`。
- 单日路径通过 `_materialize_daily_kline_range()` 调用 `materialize_trade_date_range()`，每个 `trade_date` 分区请求一次 BaoStock 单日行情。
- `fetch_k_history_table_for_trade_date()` 定义在 `pipeline/scheduler/src/scheduler/defs/baostock/services.py`，单日路径传入 `start_date=end_date=trade_date`。
- `filter_active_security_ranges()` 定义在 `pipeline/scheduler/src/scheduler/defs/market/securities.py`，已经支持请求区间与证券有效期的交集。
- `compact_daily_asset_by_year()` 定义在 `pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py`，BaoStock compacted 应继续只读取 daily source 的 `trade_date` 分区并聚合为 year 分区。

本计划将 RFC 0030 转化为可执行开发步骤、测试矩阵和验收流程。

## 关键边界定义

| 名称 | 定义 | 用途 |
| --- | --- | --- |
| 候选回填窗口 | operator 传入的 partition range，例如 `2026-01-01...2026-06-24` | 控制 Dagster 本次 materialization 覆盖的自然日范围 |
| 目标交易日集合 | 候选回填窗口与 `sina__trade_calendar` 有效交易日的交集 | 决定 `range_backfill` 必须写出的 `trade_date=*` 分区集合 |
| 首个有效交易日 | `sina__trade_calendar` 中目标年份内第一个有效交易日 | 2026 compacted 日期下界，不能写成自然日 `2026-01-01` |
| range cut-off trade date | `range_backfill` 本次允许写入的最新有效交易日 | 防止 range run 请求未来日期或把窗口外日期误写成 daily partition |
| compacted cut-off trade date | 本次声明 compacted “完整”的最新有效交易日，来自交易日历或显式 run config | 2026 compacted 日期上界，避免把未来日期或尚未落地的增量误判为完整 |
| daily 起跑日 | 日常 schedule 开始负责写入的第一个交易日，例如 `2026-06-25` | 决定 base/range backfill 与日常增量的交接点 |

最终验收口径是：`year=2026` compacted 覆盖 `sina__trade_calendar` 中 2026 年首个有效交易日到 `compacted cut-off trade date` 的所有有效交易日。若 `daily 起跑日..compacted cut-off trade date` 中任一日常增量分区尚未成功写入，验收前必须先补跑对应 daily 分区，或把 `range_backfill` 候选窗口延长到 `compacted cut-off trade date`。

## 目标

- `baostock__query_history_k_data_plus_daily` 支持 `daily` 和 `range_backfill` 两种模式。
- `daily` 模式保持现有调度语义：单交易日分区请求 `start_date=end_date=partition_key`。
- `range_backfill` 模式以 single-run partition range 执行，对每个证券执行区间请求，再按返回行 `date` 切分写回 daily source 分区。
- `range_backfill` 默认拒绝覆盖已存在 daily partition；显式 repair 覆盖必须记录 metadata。
- `range_backfill` 对一个窗口采用 all-or-nothing 语义，失败时不写 final daily partition。
- `baostock__query_history_k_data_plus_daily_compacted` 仍只读取 daily source partitions。
- 2026 缺口可通过 `range_backfill(2026-01-01..daily 起跑日前一日)` 补齐，再与 daily 增量段共同通过 2026 compacted 和 raw sync 验收。

## 非目标

- 不新增长期 `base` asset。
- 不新增长期 `_tmp/base` 或 `_state/base` 事实源。
- 不让 yearly rebuild job 直接写 compacted final 对象。
- 不让 compacted asset 读取旧 compacted 对象再与 daily 增量合并。
- 不改变 BaoStock daily / compacted contracts 字段语义。
- 不改变 dbt staging model 名称或下游 `ref()`。

## 设计约束

- 多分区 run 必须显式配置 `mode="range_backfill"`；否则失败。
- `mode="daily"` 遇到多个 partition keys 必须失败。
- `mode="range_backfill"` 可以处理单分区，但仍走区间服务路径。
- `range_backfill` 目标交易日集合来自 Dagster partition range 与 `sina__trade_calendar` 的交集。
- `range_backfill` 为目标交易日集合中的每个交易日生成输出分区；无返回行时写 contract schema 空表。
- 每个输出分区内 `(date, code)` 必须唯一。
- 默认不覆盖已存在 daily source 分区。
- 若内存无法承载完整窗口结果，可使用短生命周期 staging key，但不能把 staging key 作为下游事实源。
- raw sync 只能在 range backfill、compacted 完整性和重复键校验全部通过后执行。

## Dagster 执行模型

Dagster 支持本计划需要的执行形态：

- `baostock__query_history_k_data_plus_daily` 保留 `BackfillPolicy.single_run()`，让一个 backfill run 能覆盖整个 partition range。
- asset 内通过 `context.partition_keys` 或 `context.partition_key_range` 获取本次 run 覆盖的分区集合。
- operator 使用 `dg launch --partition-range` 和 `--config-json` 显式切换 `mode="range_backfill"`。

命令模板：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-24" \
  --config-json '{
    "ops": {
      "source__baostock__query_history_k_data_plus_daily": {
        "config": {
          "mode": "range_backfill",
          "overwrite_existing_partitions": false,
          "cutoff_trade_date": "2026-06-24"
        }
      }
    }
  }'
```

`range_backfill` 的 `cutoff_trade_date` 可省略，但当前年补数建议显式传入。它只能约束本次 range run 自己处理的窗口：若该值晚于 partition range end，则 range asset 必须失败，因为本次 range run 不会写出 range end 之后的 daily partitions。最终 compacted 的完整性上界由 compacted asset 的 `cutoff_trade_date` 单独决定；该日期可以晚于 range backfill end，但前提是中间 daily 增量分区已经存在并通过 compacted 完整性校验。

## 实施阶段

### Phase 1: daily asset run config 与模式分流

修改范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/tests/unit/...` 中 BaoStock asset 或 service 相关测试

开发任务：

1. 新增 Dagster config 类型，例如 `BaostockDailyKlineRunConfig`：
   - `mode: Literal["daily", "range_backfill"] = "daily"`
   - `overwrite_existing_partitions: bool = False`
   - `cutoff_trade_date: str | None = None`
2. 修改 `baostock__query_history_k_data_plus_daily()` 签名，接收 config。
3. 在 asset 入口做 partition 形态校验：
   - `mode="daily"` 且 `len(context.partition_keys) != 1` 时失败。
   - 多分区 run 未配置 `mode="range_backfill"` 时失败。
   - `mode="range_backfill"` 使用 sorted `context.partition_keys` 推导窗口。
4. 保留 `baostock__daily_schedule` 默认不传 run config，让单日 schedule 继续走 `daily` 默认模式。
5. metadata 中新增：
   - `source_mode`
   - `overwrite_existing_partitions`
   - `backfill_start_date`
   - `backfill_end_date`

完成标准：

- 单日 schedule 行为不变。
- 多分区 run 如果没有显式 `mode="range_backfill"`，测试证明会失败。
- `range_backfill` 单分区和多分区均能进入区间服务路径。
- 当前年 range backfill 显式 `cutoff_trade_date` 时，metadata 能记录该日期；若该日期超出本次 partition range，range run 必须失败。若最终 compacted cut-off 晚于 range end，则必须由 daily 增量分区补齐中间窗口，并由 compacted 阶段校验。

### Phase 2: 区间 fetch service

修改范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- BaoStock service 单元测试

开发任务：

1. 保留现有 `fetch_k_history_table_for_trade_date()` 作为 daily 模式入口，或将其改为区间函数的单日 wrapper。
2. 新增区间服务函数，例如：

```text
fetch_k_history_tables_for_trade_date_range(
    stock_basic,
    start_date,
    end_date,
    trade_dates,
    client_factory,
) -> dict[str, pa.Table], metadata
```

3. 使用 `filter_active_security_ranges()` 按 `requested_start_date` / `requested_end_date` 筛选证券，并使用每个 `SecurityDateRange.start_date` / `end_date` 调用 BaoStock。
4. 远端请求保持固定连接池：
   - `BAOSTOCK_DAILY_KLINE_CONNECTIONS = 4`
   - `max_concurrent_security_requests = 4`
5. 所有证券请求完成后再进入切分和写入阶段。
6. 任一证券请求失败时，本次 range backfill 失败，不写 final daily partition。

完成标准：

- daily 模式仍请求 `start_date=end_date=trade_date`。
- range backfill 模式对每个证券请求证券有效期与回填窗口的交集。
- `type="5"` ETF 继续被过滤。
- 请求失败不会留下 partial final daily partition。

### Phase 3: 按 date 切分、schema cast 与重复键校验

修改范围：

- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
- 可选新增 `pipeline/scheduler/src/scheduler/defs/baostock/range_backfill.py`
- BaoStock service 单元测试

开发任务：

1. 新增按 `date` 切分 `pa.Table` 的 helper，输出 `{trade_date: pa.Table}`。
2. 对返回行执行日期校验：
   - `date` 必须落在目标交易日集合内。
   - 非交易日返回行必须失败，而不是静默丢弃。
3. 为目标交易日集合中的每个交易日生成输出表：
   - 有数据：该日期 rows。
   - 无数据：`empty_k_history_table()` 或等价 contract schema 空表。
4. 对每个输出分区 cast 到 `PARQUET_SCHEMAS["baostock__query_history_k_data_plus_daily"]`。
5. 校验每个分区内 `(date, code)` 无重复。
6. metadata 中记录：
   - `processed_trade_dates`
   - `partition_row_counts`
   - `empty_partition_keys`
   - `duplicate_key_count`
   - `min(date)`
   - `max(date)`
   - `uniq(code)`

完成标准：

- 区间返回表可以稳定切分为 daily partition tables。
- 目标交易日集合中没有缺失输出分区。
- 重复 `(date, code)` 会失败。
- 非交易日返回行会失败。

### Phase 4: 覆盖检查与 all-or-nothing 写入

修改范围：

- `pipeline/scheduler/src/scheduler/defs/storage/`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/services.py`
- storage / BaoStock 单元测试

开发任务：

1. 新增或复用 S3 object existence 检查能力，判断目标 daily partition object 是否已存在：

```text
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet
```

2. `overwrite_existing_partitions=false` 时，若任一目标 partition 已存在，本次 run 失败且不写 final。
3. `overwrite_existing_partitions=true` 时允许覆盖，并记录：
   - `overwritten_partition_keys`
   - `overwrite_existing_partitions=true`
4. 优先采用内存校验后一次性写 final daily partitions。
5. 若内存不可接受，新增短生命周期 staging-then-promote 工具：

```text
source/baostock__query_history_k_data_plus_daily/_tmp/range_backfill/<run_id>/trade_date=YYYY-MM-DD/000000_0.parquet
```

6. staging key 只用于本次 run 的 promotion，不能被 compacted 或 raw sync 读取。
7. promotion 前必须完成 schema、日期、重复键和覆盖检查。

失败语义：

- 请求失败、schema cast 失败、日期越界、重复键、覆盖检查失败属于 validation failure；这些失败发生时不得写入任何 final daily partition。
- final object 写入阶段的 S3/IO failure 可能发生在部分 partition 已成功写入之后。若不实现 staging-then-promote，必须在 run metadata 和错误日志中列出 `attempted_partition_keys`、`written_partition_keys`、`failed_partition_keys`，并要求 operator 通过显式 `overwrite_existing_partitions=true` repair 后重新 compacted。
- 若要满足严格 all-or-nothing，即 IO failure 后 final daily prefix 也不留下 partial partitions，则本阶段必须实现 staging-then-promote，并在验收中模拟 promotion 前失败和 promotion 中失败。

完成标准：

- 默认拒绝覆盖已存在 daily partition。
- 显式覆盖时 metadata 记录覆盖范围。
- 任一校验失败时 final daily partitions 不变化。
- staging key 不成为下游事实源。
- 若未实现 staging-then-promote，文档和 metadata 必须明确 IO failure 后的 partial write repair 流程；不能把该语义描述为严格 all-or-nothing。

### Phase 5: compacted 完整性和重复键门禁

修改范围：

- `pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- compacted 单元测试

开发任务：

1. 为 `compact_daily_asset_by_year()` 增加可观测校验能力：
   - 目标 year 的 expected trade_date partition keys。
   - read / missing / empty partition keys。
   - compacted 输出中 `(date, code)` 重复计数。
2. 对 BaoStock compacted 增加完整性门禁：
   - 对 current year 或显式要求完整性的 year，`missing_partition_count > 0` 时失败。
   - 失败时不写 compacted final year object。
3. 保留 THS / Jiuyan 等其他 compacted asset 的现有行为，避免通用 helper 修改扩大 blast radius。若 helper 需要新增参数，默认值必须维持原行为。
4. metadata 中记录：
   - `expected_partition_count`
   - `read_partition_count`
   - `missing_partition_count`
   - `duplicate_key_count`
   - `completeness_required`

完成标准：

- BaoStock 2026 compacted 在缺少目标 trade_date 分区时失败。
- compacted 输出中 `(date, code)` 重复时失败。
- THS / Jiuyan compacted 测试不受影响。

### Phase 6: raw sync 操作门禁与 runbook

修改范围：

- `docs/skills/fleur-dagster-backfill-runbook/` 或新增 BaoStock runbook 文档
- 可选更新 `docs/jobs/reports/` 模板说明

开发任务：

1. 增加 BaoStock 2026 缺口补齐流程：
   - 先确定 `compacted cut-off trade date`，例如真实最新交易日 `2026-06-25`。
   - 若 `2026-06-25` daily 增量已存在，运行 `range_backfill(2026-01-01..2026-06-24)`。
   - 若 `2026-06-25` daily 增量不存在，运行 `range_backfill(2026-01-01..2026-06-25)`，或先补跑 `2026-06-25` daily 单日分区。
   - 检查 daily partition row counts 和缺失分区。
   - 运行 `baostock__query_history_k_data_plus_daily_compacted` 的 `2026` 分区，并传入 compacted cut-off。
   - 检查 compacted metadata 和 S3 row count。
   - 运行 ClickHouse raw sync 的 `2026` 分区。
2. 明确 raw sync 只允许在以下条件满足后执行：
   - range backfill 成功。
   - compacted 成功。
   - `missing_partition_count = 0`。
   - `duplicate_key_count = 0`。
3. 增加失败恢复说明：
   - 已存在 partition 的 repair 需要显式 `overwrite_existing_partitions=true`。
   - 覆盖前记录旧对象 row count / ETag / size。
   - repair 后重新运行 compacted 和 raw sync。

完成标准：

- 开发者可以按 runbook 完成 2026 缺口补齐。
- runbook 明确禁止绕过 compacted 校验直接 raw sync。

## 2026 验收运行流程

分支 A 候选窗口：

```text
2026-01-01..2026-06-24
```

实际处理日期：

```text
Sina trade calendar 中落在候选窗口内的有效交易日
```

完整性窗口：

```text
2026 年首个有效交易日..compacted cut-off trade date
```

若日常增量从 `2026-06-25` 开始，且真实最新交易日也是 `2026-06-25`，推荐执行路径如下：

- 若 `trade_date=2026-06-25` daily partition 已存在：`range_backfill` 只补 `2026-01-01..2026-06-24`，compacted 使用 `cutoff_trade_date="2026-06-25"`。
- 若 `trade_date=2026-06-25` daily partition 不存在：`range_backfill` 直接补 `2026-01-01..2026-06-25`，或先补跑 `2026-06-25` daily 单日分区；compacted 仍使用 `cutoff_trade_date="2026-06-25"`。

如果真实最新交易日晚于 `2026-06-25`，则把上述 `compacted cut-off trade date` 替换为真实最新交易日，并确保 `2026-06-25..compacted cut-off trade date` 中每个有效交易日都有 daily partition。

执行顺序：

1. 确认 `source/baostock__query_stock_basic` 最新快照可用。
2. 确认 `source/sina__trade_calendar` 可用。
3. 手动运行 daily source range backfill。
4. 校验 daily source 分区：
   - 每个目标交易日都有 object。
   - 非空交易日 row count 合理。
   - `type="5"` 不存在。
   - 分区内 `(date, code)` 无重复。
5. 运行 2026 compacted。
6. 校验 compacted：
   - schema 等于 compacted contract schema。
   - `missing_partition_count = 0`。
   - `duplicate_key_count = 0`。
   - 日期范围覆盖 2026 年首个有效交易日到 compacted cut-off trade date。
   - `uniq(code)` 不退化为单证券。
7. 运行 2026 ClickHouse raw sync。
8. 校验 ClickHouse raw：
   - raw 2026 row count 等于 compacted row count。
   - raw max(date) 等于 compacted cut-off trade date。
   - raw 中 `(date, code)` 无重复。
   - join stock basic 后 `type="5"` 行数为 0。
9. 写入 `docs/jobs/reports/` 运行报告。

建议执行命令：

```bash
cd pipeline

# 分支 A：2026-06-25 daily partition 已存在，只补历史缺口。
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-24" \
  --config-json '{"ops":{"source__baostock__query_history_k_data_plus_daily":{"config":{"mode":"range_backfill","overwrite_existing_partitions":false,"cutoff_trade_date":"2026-06-24"}}}}'

uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026 \
  --config-json '{"ops":{"source__baostock__query_history_k_data_plus_daily_compacted":{"config":{"cutoff_trade_date":"2026-06-25"}}}}'

uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026
```

分支 B：`2026-06-25` daily partition 不存在时，直接把 range 窗口延长到 compacted cut-off：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-25" \
  --config-json '{"ops":{"source__baostock__query_history_k_data_plus_daily":{"config":{"mode":"range_backfill","overwrite_existing_partitions":false,"cutoff_trade_date":"2026-06-25"}}}}'
```

分支 B 的 range run 成功后，继续执行分支 A 中的 compacted 和 ClickHouse raw sync 命令。

也可以先补跑单日 daily 分区：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition 2026-06-25
```

## 测试矩阵

### Unit Tests

- Run config：
  - 默认单分区为 `daily`。
  - `daily` + 多分区失败。
  - `range_backfill` + 多分区成功进入区间路径。
  - `range_backfill` 默认 `overwrite_existing_partitions=false`。
- BaoStock request：
  - daily 请求 `start_date=end_date=trade_date`。
  - range backfill 请求证券有效期与窗口交集。
  - `type="5"` 被过滤。
  - 任一证券请求失败使 run 失败。
- Table split：
  - 按 `date` 切分。
  - 非交易日返回行失败。
  - 目标交易日无返回行时写空表。
  - 重复 `(date, code)` 失败。
- S3 write：
  - 默认拒绝覆盖已存在 partition。
  - 显式覆盖记录 `overwritten_partition_keys`。
  - 校验失败不写 final daily partition。
  - 若采用 staging-then-promote，promotion 前失败不会生成 final object。
  - 若不采用 staging-then-promote，IO failure 测试必须证明 metadata 能列出 partial write repair 所需分区。
- Compacted：
  - 只读取 daily source。
  - missing daily partitions 在完整性要求下失败。
  - duplicate `(date, code)` 失败。
  - THS / Jiuyan compacted 行为不变。

### Integration / Definition Tests

- `dg list defs` 能看到 BaoStock daily asset、compacted asset 和相关 jobs。
- `baostock__daily_schedule` 仍只触发 daily job，且默认单日模式。
- `baostock__query_history_k_data_plus_daily` 保留 `BackfillPolicy.single_run()`。
- `baostock__query_history_k_data_plus_daily_compacted` 支持 current-year `cutoff_trade_date` config，用于把完整性窗口钉到真实最新交易日。
- ClickHouse raw specs 仍只消费 `source/baostock__query_history_k_data_plus_daily_compacted`。

## 验证命令

文档和格式：

```bash
make docs-check
git diff --check
```

Python 质量门禁：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
```

定向测试：

```bash
cd pipeline
uv run pytest \
  scheduler/tests/unit/compacted \
  scheduler/tests/unit/http/test_market_event_partitioning_and_schemas.py \
  scheduler/tests/unit/storage \
  scheduler/tests/integration/test_definitions_and_schedules.py
```

补充 BaoStock 专项测试文件后，加入：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/baostock
```

Dagster definitions：

```bash
cd pipeline/scheduler
uv run dg check defs
```

合同校验通常不需要，因为本计划不改变 contract 字段；若实施中改动 contract 或 generated outputs，则额外运行：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
```

## 完成标准

- `baostock__query_history_k_data_plus_daily` 已支持 `daily` 和 `range_backfill`。
- 多分区 run 未显式 `mode="range_backfill"` 会失败。
- `range_backfill` 按证券区间请求，按返回 `date` 切分并写回 daily source 分区。
- `range_backfill` 为目标交易日集合中每个交易日写一个 daily partition object。
- `range_backfill` 默认拒绝覆盖已存在 daily partition。
- `range_backfill` 任一校验失败时不写 final daily partition。
- `baostock__query_history_k_data_plus_daily_compacted` 仍只读取 daily source partitions。
- BaoStock compacted 在完整性要求下会阻止 missing daily partitions 进入 raw sync。
- 2026 缺口补齐流程在 dev 环境完成并有 job report，报告中的日期口径为 2026 年首个有效交易日到 `compacted cut-off trade date`，不是自然日 `2026-01-01` 到今天。
- dbt staging 仍读取 `baostock__query_history_k_data_plus_daily_compacted` raw source。

## 风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 区间返回数据量过大导致内存压力 | range backfill run 失败或内存峰值过高 | 初版限制回填窗口；必要时使用 staging-then-promote，但 staging 不作为事实源 |
| BaoStock 部分证券请求失败 | 产生不完整 daily partitions | all-or-nothing，失败不写 final |
| 覆盖已有 daily partition | 误伤已验证增量 | 默认拒绝覆盖，repair 必须显式配置并记录 metadata |
| compacted 读到部分 daily partition 也写年文件 | raw sync 得到不完整 2026 | BaoStock compacted 增加完整性门禁 |
| 隐式去重掩盖上游重复 | 数据质量问题进入 raw | `(date, code)` 重复直接失败 |
| 多分区 backfill 被拆成逐日 run | 失去区间请求成本优势 | 保留 `BackfillPolicy.single_run()`，多分区未配置 `range_backfill` 失败 |

## 交付物

- Scheduler code changes for BaoStock daily range backfill.
- Unit and integration tests listed in this plan.
- BaoStock 2026 range backfill runbook.
- 2026 dev execution job report after implementation.
