# 回填矩阵

## 首选统一入口

Source 到 ClickHouse raw 的手动修复优先使用 `backfill__fetch_history_sources_to_raw_job`。用户必须配置 `target_scope`、`start_date`、`end_date` 和少量执行参数，controller 会展开 source、compacted source 和 raw sync 子 run。`snapshot_reference_data` 也走这个入口；日期字段保持必填，但该 scope 主动忽略日期。

历史 source/raw 修复后需要继续重建 dbt、Furnace 和 marts 时，使用 `backfill__fetch_history_sources_to_marts_job`。Jiuyan 异动、行业列表、图片/OCR/OCR snapshot 后续单独设计 job，不纳入本轮 source-to-marts 统一入口。

### Raw-only 入口

| target_scope | 覆盖范围 | 日期要求 | 说明 |
| --- | --- | --- | --- |
| `baostock_daily_kline` | BaoStock 日 K daily source -> yearly compacted -> raw | 必填 | 支持 `overwrite_source_partitions`，partial year 传 `cutoff_trade_date` |
| `market_events` | Jiuyan action field、THS limit up pool 两条 daily -> compacted -> raw 链路 | 必填 | raw-only 历史能力；source-to-marts 会排除 Jiuyan 异动，只保留 THS 链路 |
| `eastmoney_f10` | EastMoney 9 个 year source/raw assets | 必填 | partial year source 传 `refresh_until_date` |
| `chinabond` | ChinaBond government bond year source/raw | 必填 | partial year source 传 `refresh_until_date` |
| `snapshot_reference_data` | Sina trade calendar、BaoStock stock basic 及其 snapshot raw | 必填但忽略 | 不使用日期做 partition selection |
| `all_raw_yearly` | 当前所有 year raw 链路 | 必填 | 不包含 snapshot reference data 和 OCR pipeline |

Raw-only dry-run 示例：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_history_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_history_sources_to_raw_controller":{"config":{"target_scope":"baostock_daily_kline","start_date":"2026-01-01","end_date":"2026-06-30","dry_run":true}}}}'
```

Snapshot reference data raw-only dry-run 示例：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_history_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_history_sources_to_raw_controller":{"config":{"target_scope":"snapshot_reference_data","start_date":"2024-01-01","end_date":"2024-01-01","dry_run":true}}}}'
```

覆盖本轮 source/raw 能力时，在 Web UI 中分两次启动：`all_raw_yearly` 加真实日期区间，`snapshot_reference_data` 加占位日期区间。Jiuyan 异动、行业列表和 OCR 后续独立规划，不再通过统一模板拼入本轮 source-to-marts 入口。

### Source-to-marts 入口

| target_scope | source/raw 范围 | downstream 范围 |
| --- | --- | --- |
| `baostock_daily_kline` | BaoStock daily source、compacted、raw | BaoStock/Sina staging、核心 quote/index/benchmark intermediate、6 个 Furnace stock calculation、calculation wrappers、stock/benchmark/calendar marts |
| `market_events` | THS limit up pool source、compacted、raw；排除 Jiuyan 异动 | `stg_ths__limit_up_pool_compacted`；当前无 mart 消费 |
| `eastmoney_f10` | EastMoney F10 source/raw | EastMoney staging、shares/exrights/valuation/quotes downstream |
| `chinabond` | ChinaBond source/raw | Government bond yields、risk free rate mart |
| `snapshot_reference_data` | Sina trade calendar、BaoStock stock basic snapshot source/raw；排除 Jiuyan industry list | calendar/stock basic/index/benchmark snapshot downstream |
| `all_raw_yearly` | 本轮允许的日期型 source/raw；排除 Jiuyan | 全部非 Jiuyan、非 portfolio downstream + Furnace calculation |
| `all_source_to_marts` | 本轮允许的日期型 + 非 Jiuyan snapshot reference source/raw | 全部非 Jiuyan、非 portfolio source/raw/dbt/Furnace/marts |

Source-to-marts dry-run 示例：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --job backfill__fetch_history_sources_to_marts_job \
  --config-json '{"ops":{"backfill__fetch_history_sources_to_marts_controller":{"config":{"target_scope":"all_source_to_marts","start_date":"2024-01-01","end_date":"2024-12-31","execution_mode":"full","dry_run":true}}}}'
```

`execution_mode=downstream_only` 可用于 source/raw 已完成后的 downstream 重建。历史 Furnace child config 使用 `replace-cascade`；controller `dry_run=true` 时 Furnace child config 使用 `dry-run`。

## Source / S3 目标资产

| Asset key | 当前入口 | 分区类型 | 策略 | 推荐命令 |
| --- | --- | --- | --- | --- |
| `source/sina__trade_calendar` | `snapshot_reference_data` 或精确 asset selection | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/sina__trade_calendar"` |
| `source/baostock__query_stock_basic` | `snapshot_reference_data` 或精确 asset selection | none | n/a | `uv run dg launch --target-path scheduler --assets "key:source/baostock__query_stock_basic"` |
| `source/baostock__query_history_k_data_plus_daily` | `baostock_daily_kline` 或精确 asset selection | trade date（日） | `single_run()` | 单日用 `--partition YYYY-MM-DD`；区间补数用 `--partition-range`，无需配置 mode |
| `source/baostock__query_history_k_data_plus_daily_compacted` | `baostock_daily_kline` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/jiuyan__industry_list` | 后续独立设计 | none | n/a | 不在本轮统一模板中 |
| `source/jiuyan__industry_images` | 后续独立设计 | none | n/a | 不在本轮统一模板中 |
| `source/jiuyan__industry_ocr` | 后续独立设计 | none | n/a | 不在本轮统一模板中 |
| `source/jiuyan__industry_ocr_snapshot` | 后续独立设计 | none | n/a | 不在本轮统一模板中 |
| `source/jiuyan__action_field` | 后续独立设计 | trade date（日） | `single_run()` | 不在本轮统一模板中 |
| `source/jiuyan__action_field_compacted` | 后续独立设计 | year | `multi_run(max_partitions_per_run=1)` | 不在本轮统一模板中 |
| `source/ths__limit_up_pool` | `market_events` 或精确 asset selection | trade date（日） | `single_run()` | 用 `--partition-range` |
| `source/ths__limit_up_pool_compacted` | `market_events` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__balance` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__cashflow_sq` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__cashflow_ytd` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__dividend_allotment` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__dividend_main` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__equity_history` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__income_sq` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |
| `source/eastmoney__income_ytd` | `eastmoney_f10` 或精确 asset selection | year | `multi_run(max_partitions_per_run=1)` | 按 `--partition YYYY` 循环 |

## ClickHouse Raw 目标资产

| Asset key | 当前入口 | 分区类型 | 策略 | 推荐命令 |
| --- | --- | --- | --- | --- |
| `clickhouse/raw/sina__trade_calendar` | `snapshot_reference_data` 或精确 asset selection | none | n/a | `uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/sina__trade_calendar"` |
| `clickhouse/raw/baostock__query_stock_basic` | `snapshot_reference_data` 或精确 asset selection | none | n/a | `uv run dg launch --target-path scheduler --assets "key:clickhouse/raw/baostock__query_stock_basic"` |
| `clickhouse/raw/jiuyan__industry_list` | 后续独立设计 | none | n/a | 不在本轮统一模板中 |
| `clickhouse/raw/jiuyan__industry_ocr_snapshot` | 后续独立设计 | none | n/a | 不在本轮统一模板中 |
| `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` | `baostock_daily_kline` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__balance` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__cashflow_sq` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__cashflow_ytd` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__dividend_allotment` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__dividend_main` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__equity_history` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__income_sq` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/eastmoney__income_ytd` | `eastmoney_f10` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |
| `clickhouse/raw/jiuyan__action_field_compacted` | 后续独立设计 | year | follows source partition | 不在本轮统一模板中 |
| `clickhouse/raw/ths__limit_up_pool_compacted` | `market_events` 或精确 asset selection | year | follows source partition | 按 `--partition YYYY` 循环 |

## 说明

- `--partition-range` 是包含式范围，必须用三个点：`start...end`
- `--partition-range` 的起止值必须是 Dagster 已存在的 partition key；如果当天是周末或未来无效分区，需要落到最近有效 partition key
- 优先精确 asset key，不要一上来就用宽 tag
- 只有已注册 job 和目标资产集合一一对应时才优先用 job；旧 source-specific 和 ClickHouse raw sync jobs 当前不再 registered
- 临时启动时，`--assets "key:..."` 通常比 `--job` 更稳
- Jiuyan 异动、行业列表、图片/OCR/OCR snapshot 后续需要独立 job/runbook，不属于本轮 unified source-to-marts 模板；历史执行事实见 `docs/jobs/dagster-backfill-2026.md`
- `ths__limit_up_pool` 默认单次只处理最近窗口内的交易日；回填时按自然日范围分段
- `ths__limit_up_pool_compacted` 是年分区资产，必须按 `--partition YYYY` 运行
- `baostock__query_history_k_data_plus_daily` 是日分区 source；ClickHouse raw 同步使用 `_compacted` 年分区资产
- `baostock__query_history_k_data_plus_daily` 的单日增量和多日区间补数都由 partition selection 推导请求窗口，无需配置 mode
- BaoStock 日 K 默认拒绝覆盖已存在的 `trade_date=*` 分区；修复已存在分区时必须显式传 `overwrite_existing_partitions=true`
- 当前年 BaoStock range backfill 建议显式传 `cutoff_trade_date`；该日期不能晚于 `--partition-range` 的结束日
- 当前年 BaoStock compacted 重建建议同步传 `cutoff_trade_date`，避免上海时钟已进入下一交易日时误要求尚未回填的 daily 分区
- BaoStock TCP 默认 connect timeout 为 15s、request read/write timeout 为 20s、login timeout 为 15s、request attempts 为 4；可通过 `BAOSTOCK_CONNECT_TIMEOUT_SECONDS`、`BAOSTOCK_REQUEST_TIMEOUT_SECONDS`、`BAOSTOCK_LOGIN_TIMEOUT_SECONDS`、`BAOSTOCK_MAX_REQUEST_ATTEMPTS` 覆盖
- BaoStock 日 K 遇到持续网络超时会在累计网络失败达到阈值后停止调度剩余证券；run 最终失败且不会写出 daily partition，修复服务端或网络后重跑同一 partition selection
- 如果 range backfill 在 S3 final 写入阶段失败，错误会列出 `attempted_partition_keys`、`written_partition_keys`、`failed_partition_keys`；记录旧对象 row count / ETag / size 后，用 `overwrite_existing_partitions=true` 修复失败窗口，再重新运行 compacted 和 raw sync
- `clickhouse/raw/*` 资产依赖对应的 `source/*` 资产；补 ClickHouse 前先确认 S3/source 分区或 snapshot 已存在
- ClickHouse snapshot 资产没有分区；补单表时用精确 `key:clickhouse/raw/...`，需要同步全部 snapshot reference raw 表时优先用 `backfill__fetch_history_sources_to_raw_job` 的 `snapshot_reference_data` scope
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

### BaoStock 日 K 区间补数

BaoStock 日 K 区间补数使用同一个 daily source asset，由 `--partition-range` 自动推导请求窗口。示例补齐 2026 年首个有效交易日到 2026-06-24 的候选窗口，实际处理日期由 Sina trade calendar 收敛：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/baostock__query_history_k_data_plus_daily" \
  --partition-range "2026-01-01...2026-06-24" \
  --config-json '{
    "ops": {
      "source__baostock__query_history_k_data_plus_daily": {
        "config": {
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

### THS 全量自然日

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/ths__limit_up_pool" \
  --partition-range "2025-01-01...$(date +%F)"
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
