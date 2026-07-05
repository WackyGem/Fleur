# Furnace Price Pattern production write zero input security failure（2026-07-05）

## 状态

Implemented - Pending Observation

## 摘要

Dagster asset `fleur_calculation/calc_stock_price_pattern_daily` 在执行 op `fleur_calculation__calc_stock_price_pattern_daily` 时失败。直接异常来自 `FurnaceCliResource.run_price_pattern()` 包装 Furnace CLI 非 0 退出码：

```text
RuntimeError: Furnace CLI failed with exit code 3: production Price Pattern writes require at least one input security
```

该错误不是 Dagster asset definition 加载失败，而是 Furnace Price Pattern runner 在 production 写入模式下发现输入证券集合为空后主动拒绝写入。

## 影响范围

- 失败资产：`fleur_calculation/calc_stock_price_pattern_daily`
- Dagster op：`fleur_calculation__calc_stock_price_pattern_daily`
- 下游 dbt wrapper：`int_stock_price_pattern_daily`
- 下游 mart：`mart_stock_price_pattern_daily`
- 可能受影响入口：
  - `backfill__fetch_history_sources_to_marts_job`：历史 source-to-marts 使用 Furnace `replace-cascade`
  - `daily__fetch_history_sources_to_marts_schedule_job`：daily source-to-marts 将 Furnace mode 从 `replace-cascade` 转为 `append-latest`

## 已确认事实

1. Dagster asset 调用链：
   - `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
   - `furnace__calc_stock_price_pattern_daily()`
   - `furnace_cli.run_price_pattern(config.to_cli_request(run_id=context.run_id))`
2. Python resource 失败点：
   - `pipeline/scheduler/src/scheduler/defs/resources/furnace.py`
   - `run_price_pattern()` 执行 Furnace CLI 后，如果 `completed.returncode != 0`，抛出 `RuntimeError`。
3. Furnace Price Pattern 输入证券解析：
   - `engines/crates/furnace-io/src/runners/price_pattern/planning.rs`
   - 未显式传 `symbols` 时，`resolve_price_pattern_symbols()` 从 `request.structure_input_table` 查询指定日期范围内的 `security_code`：

```sql
SELECT security_code
FROM {structure_input_table}
WHERE trade_date >= toDate('{request_from}')
  AND trade_date <= toDate('{request_to}')
GROUP BY security_code
ORDER BY security_code
```

4. Furnace 主动拒绝写入：
   - `engines/crates/furnace-io/src/runners/price_pattern/mod.rs`
   - `run_price_pattern()` 调用 `ensure_production_symbols("Price Pattern", request.mode.writes_applied(), &symbols)`。
   - `engines/crates/furnace-io/src/runners/shared/writing.rs`
   - 当 `writes_applied == true` 且 `symbols.is_empty()` 时返回 invalid request。
5. scheduler source-to-marts run config 当前会为 Furnace calculation 传：
   - `request_from`
   - `request_to`
   - `mode`
   - `symbols: []`
6. daily source-to-marts 的 Furnace mode 转换：
   - `pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py`
   - dry-run 外，daily 将 source-to-marts 生成的 `replace-cascade` 转为 `append-latest`。

## 初步判断

最可能的触发条件是 Price Pattern 运行区间在 `fleur_intermediate.int_stock_quotes_daily_adj` 中没有任何证券行。

后续确认：本次触发日期确实是非交易日，因此 daily source-to-marts 仍按自然日生成 `target_date..target_date` 的 Furnace append-latest 计算，最终 Price Pattern 在非交易日输入表中解析到 0 个证券并失败。

这通常对应以下场景之一：

1. daily job 使用了非交易日、尚未落库的交易日或 source/raw/int 尚未成功产出行情的日期。
2. source-to-marts 下游阶段以 `downstream_only` 或部分失败后的状态运行，Furnace calculation 先于 `int_stock_quotes_daily_adj` 数据可用。
3. backfill/daily run config 的 `request_from/request_to` 与实际行情数据日期错位。
4. 显式传入 `symbols` 的运行配置为空列表；当前这代表“全市场”，但全市场解析依赖输入表在日期范围内有行。

本次已确认属于场景 1 的“非交易日”。若后续在交易日仍出现相同错误，再按下面的 ClickHouse 行数核验继续排查上游数据缺口。

## 处理结果

2026-07-05 已按方向 B 完成 schedule-level 修复，见实施报告：

- `docs/jobs/reports/2026-07-05-daily-source-to-marts-trade-date-skip.md`

当前行为：

- `daily__fetch_history_sources_to_marts_schedule` 自动触发时读取 Sina 交易日历。
- 非 A 股交易日返回 `SkipReason`，不创建 `daily__fetch_history_sources_to_marts_schedule_job` controller run。
- Sina trade calendar parquet 不可用时返回 `SkipReason`，提示先 materialize `sina__trade_calendar`。
- 手动执行 `daily__fetch_history_sources_to_marts_schedule_job` 和历史 `backfill__fetch_history_sources_to_marts_job` 不受 schedule-level gate 限制。
- Furnace production 0 input security guard 保留；若交易日上游行情缺数导致 0 input security，仍应失败并按下方 ClickHouse 行数核验继续排查。

## 需要补充的证据

从失败 run 记录中确认：

- Dagster run id。
- op config：
  - `request_from`
  - `request_to`
  - `mode`
  - `symbols`
  - `structure_input_table`
  - `streak_input_table`
- run tags：
  - `backfill.target_scope` 或 `daily.target_scope`
  - `backfill.start_date/end_date` 或 `daily.target_date`
  - `backfill.execution_mode` 或 `daily.execution_mode`
- 上游 stage 是否成功：
  - source/raw
  - dbt staging
  - dbt intermediate

对 ClickHouse 执行最小数据核验：

```sql
SELECT
    count() AS rows,
    countDistinct(security_code) AS securities,
    min(trade_date) AS min_trade_date,
    max(trade_date) AS max_trade_date
FROM fleur_intermediate.int_stock_quotes_daily_adj
WHERE trade_date >= toDate('{request_from}')
  AND trade_date <= toDate('{request_to}');
```

如果 `rows = 0` 或 `securities = 0`，再确认原始日线输入是否存在：

```sql
SELECT
    count() AS rows,
    countDistinct(security_code) AS securities,
    min(trade_date) AS min_trade_date,
    max(trade_date) AS max_trade_date
FROM fleur_intermediate.int_stock_quotes_daily_unadj
WHERE trade_date >= toDate('{request_from}')
  AND trade_date <= toDate('{request_to}');
```

## 修复方向候选

### 方向 A：scheduler 在提交 Furnace 写入前做输入证券预检

在 source-to-marts controller 或 Furnace asset 入口增加可观测预检：对于 production write mode，在提交 `fleur_calculation/calc_stock_price_pattern_daily` 前确认 `int_stock_quotes_daily_adj` 在目标日期范围内有至少一个证券。

优点：

- 错误能在 Dagster 层带上 target date、scope、stage 和输入表行数。
- 可以区分“非交易日/无行情，跳过 calculation”与“上游数据缺失，应失败”。

注意：

- 不能简单吞掉所有 0 行场景。历史 backfill 范围若理论上应有行情，仍应失败。
- daily 非交易日是否跳过，需要和 trade calendar 语义对齐，不能仅凭当前日期猜测。

### 方向 B：daily schedule 基于交易日和上游数据可用性选择 Furnace stage

`daily__fetch_history_sources_to_marts_schedule_job` 当前按 `target_date..target_date` 生成 calculation append-latest。若 target date 不是 A 股交易日，或当日 BaoStock 日线尚未产出，Price Pattern 会解析到 0 个 symbols。

推荐策略：

- 在 `pipeline/scheduler/src/scheduler/defs/daily/definitions.py` 的 `daily_schedule_run_request()` 层增加交易日判断。
- 判断方式参考 K 线日行情、THS、Jiuyan 日任务使用的 `build_trade_date_schedule()`：通过 `S3TradeCalendarReader.from_s3_config(S3Config.from_env()).read_trade_dates()` 读取 Sina 交易日历，再用 `is_market_trade_date(target_date, trade_dates)` 判定。
- 非交易日直接返回 `dg.SkipReason(f"{target_date} is not an A-share trade date")`，不创建 `daily__fetch_history_sources_to_marts_schedule_job` controller run。
- 交易日照常返回 `RunRequest`，保持现有 source/raw/dbt/Furnace/mart/portfolio terminal step 编排不变。
- Sina trade calendar parquet 不可用时，也返回 `SkipReason("Sina trade calendar parquet is unavailable; materialize sina__trade_calendar first: ...")`，对齐 `build_trade_date_schedule()` 的行为。
- 手动执行 `daily__fetch_history_sources_to_marts_schedule_job` 或 `backfill__fetch_history_sources_to_marts_job` 不受这个 schedule-level gate 限制；历史修复、重跑和特殊 downstream-only 仍由人工传入配置负责。

不建议的策略：

- 不建议在 Furnace Price Pattern runner 中把 0 input security 静默视为成功。production write mode 的 0 输入保护仍然有价值，可以防止交易日上游数据缺失时误报成功。
- 不建议只跳过 Furnace calculation stage 后继续跑 calculation wrapper/mart。非交易日 source-to-marts 的整条日行情闭包都没有新输入，schedule 层 skip 更接近 K 线日行情采集语义，也避免 downstream mart 读取旧 calculation 状态造成观测歧义。
- 不建议改为读取 `mart_trade_calendar` 或 `int_trade_calendar` 作为 schedule gate 的第一版实现。现有 K 线日行情方案以 S3 Sina calendar source snapshot 为调度事实，复用同一入口能避免 schedule gate 依赖本次 run 内部 dbt 产物。

单测建议：

- 交易日：`daily__fetch_history_sources_to_marts_schedule.evaluate_tick()` 返回 1 个 `RunRequest`，run config 与当前行为一致。
- 非交易日：返回 `SkipReason`，`run_requests` 为空，skip message 包含 `is not an A-share trade date`。
- 交易日历不可用：mock reader 抛错，返回 `SkipReason`，message 指向先 materialize `sina__trade_calendar`。

### 方向 C：改进 Furnace CLI 错误上下文

Furnace 当前错误只说明 0 input security。可以在 CLI stderr 中补充：

- indicator
- mode
- request_from/request_to
- structure_input_table
- symbols 是否显式传入

这不改变写入保护语义，但能降低 Dagster 排障成本。

## 建议优先级

1. 优先做方向 B 的 schedule-level 非交易日 skip，复用 Sina trade calendar，行为对齐 K 线日行情采集。
2. 保留 Furnace production write 的 0 input security 失败保护，不在 Furnace 层吞掉该错误。
3. 交易日如果再次出现同类错误，再查询 `int_stock_quotes_daily_adj` 对应区间行数并追查 dbt intermediate 或 source/raw 同步。
4. 后续可补方向 C 的错误上下文，减少生产排障时间。

## 最小验证建议

scheduler 单测：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/daily/test_source_to_marts.py scheduler/tests/unit/automation/test_source_to_marts_backfill.py
```

Dagster definitions：

```bash
cd pipeline/scheduler
uv run dg check defs
```

Furnace Rust runner：

```bash
cd engines
cargo test -p furnace-io price_pattern
```

若实现涉及 CLI 错误消息或 Price Pattern guard，追加：

```bash
cd engines
cargo test --workspace
```
