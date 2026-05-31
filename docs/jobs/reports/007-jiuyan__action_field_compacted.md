# 007 - jiuyan__action_field_compacted

## 状态

- 结果：部分成功
- 失败年份：2021、2022、2023、2024、2025
- 成功年份：2026
- 时间：2026-05-31 06:20:47 UTC - 2026-05-31 06:21:17 UTC
- 成功 Run ID：`8c169988-5ebf-4491-876b-dc2b77ce5bb3`
- 失败 Run ID：`03a73122-1472-46a4-bed5-8fe2415edd83`、`45caec68-1d08-46dc-a43a-647926b0399d`、`e835f27f-59a6-4cac-80aa-63634a81b33d`、`9956d619-f117-4f1d-ae71-74f51da4c983`、`751ac124-0299-4dee-9aa5-c297fe8d3aaa`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

```bash
cd pipeline
for year in $(seq 2021 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/jiuyan__action_field_compacted" \
    --partition "$year"
done
```

## 结果摘要

- 2021-2025：`No non-empty daily partitions found for source/jiuyan__action_field in year partition ...`
- 2026：成功，`STEP_SUCCESS - Finished execution of step "source__jiuyan__action_field_compacted" in 1.04s`

## 备注

- 该 compacted 资产只能压缩已有 daily 分区的年度数据。
- 当前回填窗口只覆盖 2026 年，因此 2021-2025 为空是预期结果。
