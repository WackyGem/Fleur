# 009 - ths__limit_up_pool_compacted

## 状态

- 结果：成功
- 时间：
  - 2025 段：2026-05-31 06:26:31 UTC - 2026-05-31 06:26:36 UTC
  - 2026 段：2026-05-31 06:26:39 UTC - 2026-05-31 06:26:42 UTC
- Run ID：
  - `08d2fee5-bd8a-4af2-91eb-3aa4f9434838`
  - `cfc35c78-8f0e-41e2-a5af-2d28f731fa59`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool_compacted" --partition 2025
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool_compacted" --partition 2026
```

## 关键输出

- 2025：`STEP_SUCCESS - Finished execution of step "source__ths__limit_up_pool_compacted" in 2.26s`
- 2026：`STEP_SUCCESS - Finished execution of step "source__ths__limit_up_pool_compacted" in 1.37s`
- 两段均 `RUN_SUCCESS`

## 备注

- 该 compacted 资产只需要覆盖有 daily 数据的年度分区。
