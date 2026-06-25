# 回填矩阵

## Source / S3 目标资产

| Asset key | Job | 分区类型 | 策略 | 推荐命令 |
| --- | --- | --- | --- | --- |
| `source/sina__trade_calendar` | `sina__trade_calendar_job` | none | n/a | `uv run dg launch --target-path scheduler --job sina__trade_calendar_job` |
| `source/baostock__query_stock_basic` | `baostock__daily_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"` |
| `source/baostock__query_history_k_data_plus_daily` | `baostock__daily_job` | trade date（日） | `single_run()` | 单日用 `--partition YYYY-MM-DD`；区间补数用 `--partition-range` 且必须配置 `mode=range_backfill` |
| `source/baostock__query_history_k_data_plus_daily_compacted` | `baostock__query_history_k_data_plus_daily_compacted_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/jiuyan__industry_list` | `jiuyan__industry_list_snapshot_job` | none | n/a | `uv run dg launch --target-path scheduler --job jiuyan__industry_list_snapshot_job` |
| `source/jiuyan__industry_images` | `jiuyan__industry_ocr_pipeline_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_images"` |
| `source/jiuyan__industry_ocr` | `jiuyan__industry_ocr_pipeline_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_ocr"` |
| `source/jiuyan__industry_ocr_snapshot` | `jiuyan__industry_ocr_snapshot_job` | none | n/a | `uv run dg launch --target-path scheduler --job jiuyan__industry_ocr_snapshot_job` |
| `source/jiuyan__action_field` | `jiuyan__action_field_daily_job` | trade date（日） | `single_run()` | 用 `--partition-range` |
| `source/jiuyan__action_field_compacted` | `jiuyan__action_field_compacted_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/ths__limit_up_pool` | `ths__limit_up_pool_daily_job` | trade date（日） | `single_run()` | 用 `--partition-range` |
| `source/ths__limit_up_pool_compacted` | `ths__limit_up_pool_compacted_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__balance` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__cashflow_sq` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__cashflow_ytd` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__dividend_allotment` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__dividend_main` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__equity_history` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__income_sq` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__income_ytd` | `eastmoney__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |

## ClickHouse Raw 目标资产

| Asset key | Job | 分区类型 | 策略 | 推荐命令 |
| --- | --- | --- | --- | --- |
| `clickhouse/raw/sina__trade_calendar` | `clickhouse__raw_sync_snapshot_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/sina__trade_calendar"` |
| `clickhouse/raw/baostock__query_stock_basic` | `clickhouse__raw_sync_snapshot_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/baostock__query_stock_basic"` |
| `clickhouse/raw/jiuyan__industry_list` | `clickhouse__raw_sync_snapshot_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/jiuyan__industry_list"` |
| `clickhouse/raw/jiuyan__industry_ocr_snapshot` | `clickhouse__raw_sync_snapshot_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/jiuyan__industry_ocr_snapshot"` |
| `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` | `clickhouse__raw_sync_baostock_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__balance` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__cashflow_sq` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__cashflow_ytd` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__dividend_allotment` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__dividend_main` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__equity_history` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__income_sq` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__income_ytd` | `clickhouse__raw_sync_eastmoney_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/jiuyan__action_field_compacted` | `clickhouse__raw_sync_jiuyan_market_event_job` | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/ths__limit_up_pool_compacted` | `clickhouse__raw_sync_ths_market_event_job` | year | follows source partition | 按 `--partition YYYY` 循环 |

## 说明

- `--partition-range` 是包含式范围，必须用三个点：`start...end`
- `--partition-range` 的起止值必须是 Dagster 已存在的 partition key；如果当天是周末或未来无效分区，需要落到最近有效 partition key
- 优先精确 asset key，不要一上来就用宽 tag
- 只有 job 和目标资产集合一一对应时才优先用 job
- 临时启动时，`--assets "key:..."` 通常比 `--job` 更稳
- `jiuyan__industry_ocr_pipeline_job` 会串起 `industry_list`、`industry_images`、`industry_ocr`、`industry_ocr_snapshot`；只补图片、OCR 或 snapshot 时用精确 asset selection
- `jiuyan__industry_ocr` 支持通过 op config 设置 `limit`、`force_ocr`、`image_filenames`、`max_concurrent_requests`；asset key 带 `source/` 前缀时，op config key 是 `source__jiuyan__industry_ocr`
- `jiuyan__industry_ocr_snapshot` 依赖 `jiuyan__industry_ocr` 的成功结果和 PostgreSQL 状态；只发布 snapshot 时用 `jiuyan__industry_ocr_snapshot_job`
- `jiuyan__action_field` 默认单次只处理最近窗口内的交易日；回填时用最近自然日范围
- `jiuyan__action_field_compacted` 是年分区资产，必须按 `--partition YYYY` 运行
- `ths__limit_up_pool` 默认单次只处理最近窗口内的交易日；回填时按自然日范围分段
- `ths__limit_up_pool_compacted` 是年分区资产，必须按 `--partition YYYY` 运行
- `baostock__query_history_k_data_plus_daily` 是日分区 source；ClickHouse raw 同步使用 `_compacted` 年分区资产
- `baostock__query_history_k_data_plus_daily` 的多日区间补数必须显式传 `mode="range_backfill"`；默认 `daily` 模式只允许单日分区
- BaoStock range backfill 默认拒绝覆盖已存在的 `trade_date=*` 分区；修复已存在分区时必须显式传 `overwrite_existing_partitions=true`
- 当前年 BaoStock range backfill 建议显式传 `cutoff_trade_date`；该日期不能晚于 `--partition-range` 的结束日
- 当前年 BaoStock compacted 重建建议同步传 `cutoff_trade_date`，避免上海时钟已进入下一交易日时误要求尚未回填的 daily 分区
- 如果 range backfill 在 S3 final 写入阶段失败，错误会列出 `attempted_partition_keys`、`written_partition_keys`、`failed_partition_keys`；记录旧对象 row count / ETag / size 后，用 `overwrite_existing_partitions=true` 修复失败窗口，再重新运行 compacted 和 raw sync
- `clickhouse/raw/*` 资产依赖对应的 `source/*` 资产；补 ClickHouse 前先确认 S3/source 分区或 snapshot 已存在
- ClickHouse snapshot 资产没有分区；补单表时用精确 `key:clickhouse/raw/...`，只有需要同步全部 snapshot raw 表时才用 `clickhouse__raw_sync_snapshot_job`
- ClickHouse year 资产沿用 source 的年分区；按 `--partition YYYY` 运行，不要对年分区使用 `--partition-range`

## 示例

先在仓库根目录加载 `.env`，并确保 `DAGSTER_HOME` 已生效：

```bash
set -a
. ./.env
set +a
make dagster-home
```

### 日范围

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2024-01-01...2024-01-31"
```

### 单年分区

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily_compacted" --partition 2024
```

### BaoStock 日 K range backfill

BaoStock 日 K 区间补数使用同一个 daily source asset，但必须显式配置 `range_backfill`。示例补齐 2026 年首个有效交易日到 2026-06-24 的候选窗口，实际处理日期由 Sina trade calendar 收敛：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-24" \
  --config-json '{
    "ops": {
      "source__baostock__query_history_k_data_plus_daily": {
        "config": {
          "mode": "range_backfill",
          "overwrite_existing_partitions": false,
          "cutoff_trade_date": "2026-06-24"
        }
      }
    }
  }'
```

如果 `2026-06-25` daily partition 尚未存在，且 compacted 验收 cut-off 是 `2026-06-25`，直接把 range 窗口延长到 cut-off：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-25" \
  --config-json '{
    "ops": {
      "source__baostock__query_history_k_data_plus_daily": {
        "config": {
          "mode": "range_backfill",
          "overwrite_existing_partitions": false,
          "cutoff_trade_date": "2026-06-25"
        }
      }
    }
  }'
```

如果是明确修复已存在的 daily partitions，才允许覆盖：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-24" \
  --config-json '{
    "ops": {
      "source__baostock__query_history_k_data_plus_daily": {
        "config": {
          "mode": "range_backfill",
          "overwrite_existing_partitions": true,
          "cutoff_trade_date": "2026-06-24"
        }
      }
    }
  }'
```

range backfill 成功后，先运行 compacted，再运行 ClickHouse raw sync：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026 \
  --config-json '{
    "ops": {
      "source__baostock__query_history_k_data_plus_daily_compacted": {
        "config": {
          "cutoff_trade_date": "2026-06-24"
        }
      }
    }
  }'

uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026
```

上面的 compacted `cutoff_trade_date` 必须等于本次声明完整的最新有效交易日。如果实际验收 cut-off 是 `2026-06-25`，则 compacted 命令应传：

```json
{"ops":{"source__baostock__query_history_k_data_plus_daily_compacted":{"config":{"cutoff_trade_date":"2026-06-25"}}}}
```

raw sync 前必须确认 compacted run metadata 中 `missing_partition_count = 0` 且 `duplicate_key_count = 0`。当前年验收口径是 2026 年首个有效交易日到 `cutoff_trade_date`；如果 `2026-06-25..cutoff_trade_date` 的 daily 增量还没落地，先补跑对应 daily 分区，或把 range backfill 窗口延长到 `cutoff_trade_date`。

### 按年循环

```bash
cd pipeline
for year in 2020 2021 2022 2023 2024; do
  uv run dg launch --target-path scheduler --assets "key:source/eastmoney__balance" --partition "$year"
done
```

### Jiuyan OCR 限制数量

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__industry_ocr" \
  --config-json '{"ops":{"source__jiuyan__industry_ocr":{"config":{"limit":50}}}}'
```

### Jiuyan OCR snapshot

```bash
cd pipeline
uv run dg launch --target-path scheduler --job jiuyan__industry_ocr_snapshot_job
```

### 最近 90 个自然日

```bash
cd pipeline
end_date="$(date +%F)"
start_date="$(date -d "$end_date -89 days" +%F)"
uv run dg launch --target-path scheduler \
  --assets "key:source/jiuyan__action_field" \
  --partition-range "${start_date}...${end_date}"
```

如果 `end_date` 不是有效 partition key，先把它改成最近有效日分区。例如 2026-05-31 是周日，成功执行时使用 `2026-03-02...2026-05-29`。

### THS 全量自然日

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/ths__limit_up_pool" \
  --partition-range "2025-01-01...$(date +%F)"
```

### Jiuyan action_field compacted 年分区

```bash
cd pipeline
for year in $(seq 2021 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/jiuyan__action_field_compacted" \
    --partition "$year"
done
```

### ClickHouse snapshot raw 单表同步

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/jiuyan__industry_ocr_snapshot"
```

### ClickHouse 年分区 raw 同步

```bash
cd pipeline
for year in $(seq 2021 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler \
    --assets "key:clickhouse/raw/jiuyan__action_field_compacted" \
    --partition "$year"
done
```

### THS compacted 年分区

```bash
cd pipeline
for year in $(seq 2025 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/ths__limit_up_pool_compacted" \
    --partition "$year"
done
```
