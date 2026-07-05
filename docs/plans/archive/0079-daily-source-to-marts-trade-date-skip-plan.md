# Plan 0079: Daily source-to-marts 非交易日跳过实施计划

日期：2026-07-05

状态：Completed

领域：Dagster, scheduler, data-platform

关联系统：

- `pipeline/scheduler/src/scheduler/defs/daily/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py`
- `pipeline/scheduler/src/scheduler/defs/market/schedules.py`
- `pipeline/scheduler/src/scheduler/defs/market/readers.py`
- `pipeline/scheduler/tests/unit/daily/test_source_to_marts.py`

关联文档：

- [Furnace Price Pattern zero input security issue](../../issues/furnace-price-pattern-zero-input-security-2026-07-05.md)
- [Scheduler Architecture](../../architecture/scheduler-architecture.md)
- [Dagster definitions lineage 2026-06-10](../../jobs/dagster-definitions-lineage-2026-06-10.md)
- [RFC 0040: Dagster stg-to-mart asset inventory](../../RFC/archive/0040-dagster-stg-to-mart-asset-inventory.md)
- [Plan 0067: Daily source-to-marts clean-slate orchestration](0067-daily-source-to-marts-clean-slate-orchestration-plan.md)

## 背景

`daily__fetch_history_sources_to_marts_schedule` 当前按自然日生成 `target_date`，并提交 `daily__fetch_history_sources_to_marts_schedule_job`。该 controller 会把 `target_date` 映射成 source/raw/dbt/Furnace/mart 的单日闭包，其中 Furnace calculation 在 daily 非 dry-run 下使用 `append-latest`。

在非交易日，`fleur_intermediate.int_stock_quotes_daily_adj` 对该日期没有证券行情行。Price Pattern 运行时从 `structure_input_table` 解析到 0 个证券，Furnace production 写入保护触发：

```text
production Price Pattern writes require at least one input security
```

K 线日行情、THS 和 Jiuyan 日任务已有同类方案：schedule evaluation 通过 Sina 交易日历判定非交易日，并返回 `SkipReason`，不创建 run。本计划将 daily source-to-marts schedule 对齐该行为。

## 目标

1. `daily__fetch_history_sources_to_marts_schedule` 在非 A 股交易日返回 `SkipReason`，不创建 controller run。
2. 交易日行为保持不变，继续生成现有 `RunRequest`、run config 和 tags。
3. 交易日历不可用时返回 `SkipReason`，提示先 materialize `sina__trade_calendar`。
4. 复用现有 `S3TradeCalendarReader` 和 `is_market_trade_date` 语义，对齐 K 线日行情 schedule。
5. 保留 `daily__fetch_history_sources_to_marts_schedule_job` 手动执行能力；手动 run 不受 schedule-level gate 限制。
6. 保留 Furnace production 0 input security 失败保护；交易日上游缺数仍应失败。

## 非目标

1. 不修改 Furnace CLI、Furnace Rust runner 或 Price Pattern 算法。
2. 不在 Furnace asset 内把 0 input security 静默视为成功。
3. 不修改 `backfill__fetch_history_sources_to_marts_job`；历史回填仍由人工配置日期和 mode。
4. 不只跳过 Furnace calculation stage 后继续跑 dbt wrappers/marts。
5. 不在第一版改用 `int_trade_calendar` 或 `mart_trade_calendar` 作为 schedule gate 事实来源。
6. 不改变 daily cron、default stopped 状态、target scope、execution mode 或 run config 字段。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| daily schedule | `pipeline/scheduler/src/scheduler/defs/daily/definitions.py` 的 `daily_schedule_run_request()` 只从 `scheduled_execution_time` 派生 `target_date`，不判断交易日。 |
| daily controller | `daily__fetch_history_sources_to_marts_schedule_job` 会把 `target_date` 展开为 source/raw/dbt/Furnace/mart 阶段化 child runs。 |
| Furnace daily mode | `pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py` 将非 dry-run Furnace mode 从 `replace-cascade` 转为 `append-latest`。 |
| K 线日行情 skip | `pipeline/scheduler/src/scheduler/defs/market/schedules.py` 的 `build_trade_date_schedule()` 读取 `S3TradeCalendarReader`，非交易日返回 `SkipReason`。 |
| 交易日事实来源 | `source/sina__trade_calendar` 的 S3 snapshot 是现有 schedule-level 交易日事实来源。 |
| 本次 issue | `docs/issues/furnace-price-pattern-zero-input-security-2026-07-05.md` 已确认失败运行日为非交易日。 |

## 设计约束

1. 非交易日 skip 应发生在 schedule evaluation 层，而不是 controller plan 或 Furnace asset 层。
2. schedule gate 只影响 Dagster schedule 自动触发；手动执行 job 仍允许。
3. 交易日历读取失败必须是 skip，不应创建一个注定失败的 daily source-to-marts run。
4. 新逻辑应复用 `S3TradeCalendarReader.from_s3_config(S3Config.from_env())` 和 `is_market_trade_date()`。
5. 测试应通过 fake reader 或 monkeypatch 固定交易日集合，不依赖真实 S3。
6. 不引入多套日期事实来源，避免和 K 线日行情、THS、Jiuyan schedule 行为漂移。

## 实施阶段

### Phase 0: 前置事实确认

目标：确认当前 schedule 与现有交易日 skip 组件的真实接口。

实施项：

1. 读取 `daily/definitions.py`，确认 `daily_schedule_run_request()` 当前 run config 和 tags。
2. 读取 `market/schedules.py`，确认 `build_trade_date_schedule()` 的 skip message、reader factory 和 timezone 行为。
3. 读取 `market/readers.py`，确认 `S3TradeCalendarReader` 和 `TradeCalendarReader` 接口。
4. 读取 `tests/unit/daily/test_source_to_marts.py`，确认现有 schedule 单测结构。

完成标准：

1. 交易日 skip 行为有现有代码事实支撑。
2. 新增测试能在不访问 S3 的情况下覆盖 run/skip 两条路径。

### Phase 1: daily schedule 交易日 gate

目标：让 schedule 自动触发时跳过非交易日。

实施项：

1. 在 `pipeline/scheduler/src/scheduler/defs/daily/definitions.py` 引入：
   - `S3Config`
   - `S3TradeCalendarReader`
   - `TradeCalendarReader`
   - `is_market_trade_date`
2. 增加 module-level reader factory，默认读取 `S3TradeCalendarReader.from_s3_config(S3Config.from_env())`。
3. 在 `daily_schedule_run_request()` 中：
   - 保持 `scheduled_execution_time is None` 的现有 `SkipReason`。
   - 使用 `DAILY_SCHEDULE_TIMEZONE` 将 scheduled time 转为 `target_date`。
   - 读取交易日集合。
   - 日历读取异常时返回：

```text
Sina trade calendar parquet is unavailable; materialize sina__trade_calendar first: {error}
```

   - `target_date` 非交易日时返回：

```text
{target_date} is not an A-share trade date
```

   - 交易日时按当前逻辑返回 `RunRequest`。

完成标准：

1. 交易日 run config 和 tags 与现有行为一致。
2. 非交易日不创建 controller run。
3. 日历不可用不创建 controller run。

### Phase 2: 单元测试覆盖

目标：用机械测试固定 schedule 行为。

实施项：

1. 更新 `pipeline/scheduler/tests/unit/daily/test_source_to_marts.py`。
2. 为 `daily__fetch_history_sources_to_marts_schedule.evaluate_tick()` 增加 fake trade calendar reader。
3. 修改现有 `test_daily_schedule_is_stopped_and_emits_target_date_config`，确保目标日期在 fake trade_dates 中。
4. 新增非交易日 skip 测试：
   - scheduled time 为 `2026-07-05 17:45 Asia/Shanghai`。
   - fake trade_dates 不包含 `2026-07-05`。
   - 断言 `run_requests` 为空，skip message 包含 `2026-07-05 is not an A-share trade date`。
5. 新增日历不可用 skip 测试：
   - fake reader 抛出 `RuntimeError("calendar unavailable")`。
   - 断言 skip message 包含 `materialize sina__trade_calendar first`。
6. 保持 plan/controller 单测不引入交易日 gate；这些测试覆盖手动 job 语义，不能被 schedule gate 误伤。

完成标准：

1. schedule 单测覆盖交易日、非交易日和日历不可用。
2. controller plan 测试继续证明手动 run config 能按目标日期展开。

### Phase 3: Definitions 和质量门禁

目标：确认 Dagster definitions 和 scheduler 单测通过。

实施项：

1. 运行 daily 定向单测。
2. 运行 scheduler definitions check。
3. 运行 Python lint/format 检查。
4. 运行文档门禁。

完成标准：

1. daily schedule tests 通过。
2. `dg check defs` 通过。
3. `ruff check` 和 `ruff format --check` 通过。
4. `make docs-check` 和 `git diff --check` 通过。

### Phase 4: issue 收敛和报告

目标：让 issue 处理结果可追溯。

实施项：

1. 在 `docs/jobs/reports/YYYY-MM-DD-daily-source-to-marts-trade-date-skip.md` 记录实施结果。
2. 更新 `docs/issues/furnace-price-pattern-zero-input-security-2026-07-05.md` 状态：
   - 若实现完成并通过验证，标记为已转实施完成或待观测关闭。
   - 保留交易日上游缺数仍应失败的说明。
3. 完成后将本计划移入 `docs/plans/archive/`。
4. 更新 `docs/plans/README.md`，从 Active Plans 移除并加入 Recently Completed。

完成标准：

1. issue 有实施报告和验证结果链接。
2. active plan 状态与 `docs/plans/README.md` 一致。

## 最小验证命令

scheduler 单测：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/daily/test_source_to_marts.py
```

Dagster definitions：

```bash
cd pipeline/scheduler
uv run dg check defs
```

Python 检查：

```bash
cd pipeline
uv run ruff check scheduler/src/scheduler/defs/daily scheduler/tests/unit/daily
uv run ruff format --check scheduler/src/scheduler/defs/daily scheduler/tests/unit/daily
```

文档检查：

```bash
make docs-check
git diff --check
```

## 完成标准

1. `daily__fetch_history_sources_to_marts_schedule` 在非交易日返回 `SkipReason`。
2. 交易日 `RunRequest` 的 run key、run config 和 tags 保持现有行为。
3. Sina trade calendar 不可用时返回可诊断的 `SkipReason`。
4. 手动 `daily__fetch_history_sources_to_marts_schedule_job` 和 `backfill__fetch_history_sources_to_marts_job` 行为不变。
5. Furnace 0 input security production 写入保护保留。
6. daily schedule 单测、`dg check defs`、ruff 和文档门禁通过。
7. issue 文档更新并记录实施报告。
