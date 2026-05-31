# 002 - baostock__query_stock_basic

## 状态

- 结果：成功
- 时间：2026-05-31 05:48:26 UTC - 2026-05-31 05:48:40 UTC
- Run ID：`ec547352-c0d6-4cb3-b2e1-22157454a014`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"
```

## 关键输出

- `ASSET_MATERIALIZATION - Materialized value source baostock__query_stock_basic`
- `STEP_SUCCESS - Finished execution of step "source__baostock__query_stock_basic" in 12.29s`
- `RUN_SUCCESS - Finished execution of run for "__ASSET_JOB"`

## 备注

- 该资产是 Eastmoney 和 BaoStock 日线资产的上游基础证券信息。

