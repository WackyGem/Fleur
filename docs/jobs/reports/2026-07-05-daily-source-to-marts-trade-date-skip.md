# 2026-07-05 Daily source-to-marts 非交易日跳过实施报告

日期：2026-07-05

范围：

- Plan 0079：`docs/plans/archive/0079-daily-source-to-marts-trade-date-skip-plan.md`
- Issue：`docs/issues/furnace-price-pattern-zero-input-security-2026-07-05.md`
- Dagster schedule：`pipeline/scheduler/src/scheduler/defs/daily/definitions.py`
- 测试：`pipeline/scheduler/tests/unit/daily/test_source_to_marts.py`

## 实施结果

已完成：

1. `daily__fetch_history_sources_to_marts_schedule` 在 schedule evaluation 层读取 Sina trade calendar。
2. 交易日历读取方式复用 `S3TradeCalendarReader.from_s3_config(S3Config.from_env())` 和 `is_market_trade_date()`。
3. 非 A 股交易日返回 `SkipReason("{target_date} is not an A-share trade date")`，不创建 controller run。
4. Sina trade calendar parquet 不可用时返回 `SkipReason`，提示先 materialize `sina__trade_calendar`。
5. 交易日 run key、run config、tags、cron、default stopped 状态保持不变。
6. `daily__fetch_history_sources_to_marts_schedule_job` 手动执行路径未加 gate，仍由手动配置的 `target_date` 展开 source-to-marts plan。
7. Furnace production 0 input security 写入保护未修改；交易日上游缺数仍应失败。

## 验证结果

通过：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/daily/test_source_to_marts.py
```

结果：`14 passed`。

通过：

```bash
cd pipeline
uv run ruff check scheduler/src/scheduler/defs/daily scheduler/tests/unit/daily
uv run ruff format --check scheduler/src/scheduler/defs/daily scheduler/tests/unit/daily
```

通过：

```bash
cd pipeline/scheduler
DAGSTER_HOME=/storage/program/fleur/.dagster uv run dg check defs
```

结果：component YAML 和 definitions 均加载成功。

## 行为边界

- schedule 自动触发受交易日 gate 保护。
- 手动执行 `daily__fetch_history_sources_to_marts_schedule_job` 不受 schedule-level gate 限制。
- `backfill__fetch_history_sources_to_marts_job` 未修改。
- 交易日如果 `fleur_intermediate.int_stock_quotes_daily_adj` 没有输入证券，Furnace Price Pattern 仍会按 production guard 失败，避免误报成功。
