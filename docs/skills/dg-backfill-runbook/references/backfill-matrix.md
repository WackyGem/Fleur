# 回填矩阵

## 目标资产

| Asset key | Job | 分区类型 | 策略 | 推荐命令 |
| --- | --- | --- | --- | --- |
| `source/sina__trade_calendar` | `sina__trade_calendar_job` | none | n/a | `uv run dg launch --target-path scheduler --job sina__trade_calendar_job` |
| `source/baostock__query_stock_basic` | `baostock__daily_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"` |
| `source/baostock__query_history_k_data_plus_daily` | `baostock__daily_job` | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/jiuyan__industry_list` | `jiuyan__industry_list_snapshot_job` | none | n/a | `uv run dg launch --target-path scheduler --job jiuyan__industry_list_snapshot_job` |
| `source/jiuyan__industry_images` | `jiuyan__industry_ocr_pipeline_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_images"` |
| `source/jiuyan__industry_ocr` | `jiuyan__industry_ocr_pipeline_job` | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/jiuyan__industry_ocr"` |
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

## 说明

- `--partition-range` 是包含式范围，必须用三个点：`start...end`
- 优先精确 asset key，不要一上来就用宽 tag
- 只有 job 和目标资产集合一一对应时才优先用 job
- 临时启动时，`--assets "key:..."` 通常比 `--job` 更稳
- `jiuyan__industry_ocr_pipeline_job` 会串起 `industry_list`、`industry_images`、`industry_ocr`；只补图片或 OCR 时用精确 asset selection
- `jiuyan__industry_ocr` 支持通过 op config 设置 `limit`、`force_ocr`、`image_filenames`、`max_concurrent_requests`
- `jiuyan__action_field` 默认单次只处理最近窗口内的交易日；回填时用最近自然日范围
- `jiuyan__action_field_compacted` 是年分区资产，必须按 `--partition YYYY` 运行
- `ths__limit_up_pool` 默认单次只处理最近窗口内的交易日；回填时按自然日范围分段
- `ths__limit_up_pool_compacted` 是年分区资产，必须按 `--partition YYYY` 运行

## 示例

### 日范围

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/ths__limit_up_pool" --partition-range "2024-01-01...2024-01-31"
```

### 单年分区

```bash
cd pipeline
uv run dg launch --target-path scheduler --assets "key:source/baostock__query_history_k_data_plus_daily" --partition 2024
```

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
  --config-json '{"ops":{"jiuyan__industry_ocr":{"config":{"limit":50}}}}'
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

### THS compacted 年分区

```bash
cd pipeline
for year in $(seq 2025 "$(date +%Y)"); do
  uv run dg launch --target-path scheduler \
    --assets "key:source/ths__limit_up_pool_compacted" \
    --partition "$year"
done
```
