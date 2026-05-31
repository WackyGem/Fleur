# 005 - jiuyan__industry_ocr

## 状态

- 结果：成功（50 张 OCR 限量回填完成，48 成功，2 失败）
- 时间：2026-05-31 06:10:14 UTC - 2026-05-31 06:14:25 UTC
- Run ID：`f66f2406-6248-4750-bc40-42d8b01f9544`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__industry_ocr" \
  --config-json '{"ops":{"source__jiuyan__industry_ocr":{"config":{"limit":50}}}}'
```

## 关键输出

- `STEP_SUCCESS - Finished execution of step "source__jiuyan__industry_ocr" in 4m8s`
- `ASSET_MATERIALIZATION - Materialized value source jiuyan__industry_ocr`
- `RUN_SUCCESS - Finished execution of run for "__ASSET_JOB"`
- PostgreSQL 状态汇总：`success=48`、`failed=2`、`pending=1142`
- `success_with_result=48`

## 警告

- 有 2 张图片 OCR 失败，错误信息为 `OCR response content is not valid JSON`：
  - `02731EBF-4D96-4B77-8C28-515A46B32E6D.jpg`
  - `051A7AA4-1F8-4086-8B09-15523A450A55.png`
- 失败数低于资产的部分失败阈值，Dagster run 成功。

## 纠偏记录

- 首次启动时使用了错误 op config key：`jiuyan__industry_ocr`。
- Dagster 接收了 run config，但实际 step 名是 `source__jiuyan__industry_ocr`，导致 `limit=50` 未生效并将 1185 条记录 claim 为 `running`。
- 已停止该错误 run，并将 1185 条 `running` OCR 状态恢复为 `pending` 后重新执行。
