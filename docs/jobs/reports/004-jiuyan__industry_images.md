# 004 - jiuyan__industry_images

## 状态

- 结果：成功（首次失败，迁移后重跑成功）
- 首次失败时间：2026-05-31 05:49:45 UTC - 2026-05-31 05:49:47 UTC
- 首次失败 Run ID：`346e6221-cd15-481f-a5e9-d2642f0a3fad`
- 重跑成功时间：2026-05-31 05:52:37 UTC - 2026-05-31 05:53:18 UTC
- 重跑成功 Run ID：`2b7d389a-cd88-4544-9178-cd4da0ac1467`
- Dagster home：`/storage/program/mono-fleur/.dagster`

## 命令

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_images"
```

## 失败原因

```text
psycopg.errors.UndefinedTable: relation "jiuyan_industry_images" does not exist
```

失败位置：

```text
scheduler/defs/repositories/industry_images.py
fetch_existing_image_urls()
```

## 处理动作

- 已执行 PostgreSQL Alembic 迁移：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini upgrade head
```

- 当前数据库版本：`0001_jiuyan_industry_images`
- 已确认 `public.jiuyan_industry_images` 表存在。
- 迁移完成后重跑 `jiuyan__industry_images` 成功。

## 重跑关键输出

- `ASSET_MATERIALIZATION - Materialized value source jiuyan__industry_images`
- `STEP_SUCCESS - Finished execution of step "source__jiuyan__industry_images" in 39.24s`
- `RUN_SUCCESS - Finished execution of run for "__ASSET_JOB"`

## 警告

- 重跑期间有多张图片下载返回 HTTP 400，错误信息包含 `font content is too large`。
- 该类下载失败被资产逻辑记录为 warning，未导致 run 失败。
