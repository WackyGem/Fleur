# 008 - ths__limit_up_pool

## 状态

- 结果：成功
- 时间：
  - 2025 段：2026-05-31 06:23:27 UTC - 2026-05-31 06:23:54 UTC
  - 2026 段：2026-05-31 06:25:17 UTC - 2026-05-31 06:25:31 UTC
- Run ID：
  - `aff98901-157a-410b-bb2c-93d8ce29f065`
  - `d4d3e1bc-76f0-43b9-ae5b-444ce15734da`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2025-01-01...2025-12-31"
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2026-01-01...2026-05-29"
```

## 关键输出

- 2025 段：365 个 materialization，`STEP_SUCCESS - Finished execution of step "source__ths__limit_up_pool" in 24.39s`
- 2026 段：149 个 materialization，`STEP_SUCCESS - Finished execution of step "source__ths__limit_up_pool" in 11.45s`
- 两段均 `RUN_SUCCESS`

## 备注

- `ths__limit_up_pool` 按交易日分区回填，符合 380 天自然日窗口限制。
- 这次按两段覆盖到当前回填窗口上界。
