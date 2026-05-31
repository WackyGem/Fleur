# 001 - sina__trade_calendar

## 状态

- 结果：成功
- 时间：2026-05-31 05:47:36 UTC - 2026-05-31 05:47:38 UTC
- Run ID：`b9e2ae12-d3eb-4186-bb84-9fc07a6c88ad`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

```bash
cd pipeline
uv run dg launch --target-path scheduler --job sina__trade_calendar_job
```

## 关键输出

- `Parsed 8797 Sina trade-calendar rows`
- `ASSET_MATERIALIZATION - Materialized value source sina__trade_calendar`
- `RUN_SUCCESS - Finished execution of run for "sina__trade_calendar_job"`

## 备注

- 该资产是后续交易日过滤事实来源。

