# 006 - jiuyan__action_field

## 状态

- 结果：成功（首次使用无效周末终点失败，调整有效 partition 边界后成功）
- 首次失败时间：2026-05-31 06:16 UTC
- 成功时间：2026-05-31 06:18:32 UTC - 2026-05-31 06:18:44 UTC
- 成功 Run ID：`d6852019-9d6e-47da-9068-c3d03716fde9`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

首次失败命令：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__action_field" \
  --partition-range "2026-03-03...2026-05-31"
```

成功命令：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__action_field" \
  --partition-range "2026-03-02...2026-05-29"
```

## 关键输出

- `STEP_SUCCESS - Finished execution of step "source__jiuyan__action_field" in 10.2s`
- `RUN_SUCCESS - Finished execution of run for "__ASSET_JOB"`
- 实际 materialized partition 数：89
- 实际 partition 范围：`2026-03-02...2026-05-29`

## 失败原因

- `2026-05-31` 是周日，不是该资产可用 partition key。
- `dg launch --partition-range` 要求 range 起止值本身都是 Dagster 已存在的 partition key。

错误摘要：

```text
DagsterUnknownPartitionError: Could not find a partition with key `2026-05-31`.
DagsterInvalidSubsetError: All selected assets must have a PartitionsDefinition containing the passed partition key `2026-03-03` or have no PartitionsDefinition.
```

## 备注

- 该资产内部会基于 `sina__trade_calendar` 过滤交易日。
- 对最近 90 个自然日回填时，应先把自然日起止落到有效日分区，再提交 `--partition-range`。
