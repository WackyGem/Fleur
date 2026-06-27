# MA30 and Adjusted Quote Mart Rerun

日期：2026-06-15

## 结论

Plan 0038 的两个数据前置项已完成并重跑全量数据：

- `fleur_calculation.calc_stock_ma_daily` 已通过 Furnace 全市场、全历史 `replace-cascade` 重算，新增并写入 `price_ma_30`。
- `fleur_marts.mart_stock_trend_indicator` 已重建并暴露 `price_ma_30`。
- `fleur_marts.mart_stock_quotes_daily` 已重建并暴露前复权、后复权 OHLC、复权前收和复权因子字段。
- Rearview metric catalog 已同步 `price_ma_30`。

ClickHouse 约束：

- Per `schema-pk-plan-before-creation`：未改变现有 mart `ORDER BY`。
- Per `schema-pk-filter-on-orderby`：抽样查询使用排序键相关过滤。
- Per `query-join-filter-before`：新增 mart 测试和后续 API 查询约束保持先过滤再 join 的设计。
- Per `query-join-use-any`：`mart_stock_quotes_daily` 对一行一键的复权行情使用 `LEFT ANY JOIN`。
- Per `insert-mutation-avoid-update`：没有用 `ALTER TABLE UPDATE` 回填 MA30，历史值通过 Furnace `replace-cascade` 写入。

## 范围

输入范围来自 `fleur_intermediate.int_stock_quotes_daily_adj` 与 `fleur_intermediate.int_stock_quotes_daily_unadj`：

| min_trade_date | max_trade_date | input_rows | valid_close_rows | valid_volume_rows | symbols |
|---|---:|---:|---:|---:|---:|
| 1995-01-03 | 2026-06-01 | 17,990,764 | 17,990,764 | 17,990,764 | 5,532 |

## Furnace MA 全量重算

Release build：

```bash
cd engines
cargo build --release -p furnace
```

全市场 dry-run：

```bash
cd engines
set -a; . ../.env; set +a
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_QUERY_TIMEOUT_SECONDS=900 \
RAYON_NUM_THREADS=8 \
target/release/furnace ma \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode dry-run \
  --insert-batch-size 100000 \
  --output-format json
```

Dry-run 摘要：

```json
{
  "mode": "dry-run",
  "symbols_count": 5532,
  "input_rows": 17990764,
  "output_rows": 17990764,
  "valid_close_rows": 17990764,
  "valid_volume_rows": 17990764,
  "null_indicator_rows": 11064,
  "price_ma_windows": [3, 5, 6, 10, 12, 14, 20, 24, 28, 30, 57, 60, 114, 250],
  "writes_applied": false,
  "performance_metrics": {
    "total_ms": 12117,
    "read_input_ms": 7535,
    "group_ms": 2354,
    "compute_ms": 854,
    "worker_threads": 8
  }
}
```

全市场 `replace-cascade`：

```bash
cd engines
set -a; . ../.env; set +a
FURNACE_CLICKHOUSE_CLIENT=docker \
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i fleur-clickhouse clickhouse-client' \
CLICKHOUSE_HOST=127.0.0.1 \
CLICKHOUSE_NATIVE_PORT=9000 \
CLICKHOUSE_QUERY_TIMEOUT_SECONDS=900 \
RAYON_NUM_THREADS=8 \
target/release/furnace ma \
  --from 1995-01-03 \
  --to 2026-06-01 \
  --mode replace-cascade \
  --run-id furnace_ma30_full_market_20260615 \
  --insert-batch-size 100000 \
  --output-format json
```

Replace-cascade 摘要：

```json
{
  "mode": "replace-cascade",
  "symbols_count": 5532,
  "input_rows": 17990764,
  "output_rows": 17990764,
  "valid_close_rows": 17990764,
  "valid_volume_rows": 17990764,
  "null_indicator_rows": 11064,
  "affected_years": [1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026],
  "staging_table": "fleur_calculation.calc_stock_ma_daily__staging__furnace_ma30_full_market_20260615",
  "staging_validation": {"status": "passed", "duplicate_keys": 0},
  "partition_replace": {"status": "replaced"},
  "price_ma_windows": [3, 5, 6, 10, 12, 14, 20, 24, 28, 30, 57, 60, 114, 250],
  "writes_applied": true,
  "performance_metrics": {
    "total_ms": 47678,
    "read_input_ms": 6478,
    "group_ms": 2272,
    "compute_ms": 6163,
    "write_ms": 26800,
    "staging_ms": 3366,
    "partition_replace_ms": 581,
    "worker_threads": 8
  }
}
```

## dbt 重建

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_ma_daily mart_stock_trend_indicator mart_stock_quotes_daily \
  --quiet \
  --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

结果：

| 资源 | 状态 | 耗时 |
|---|---|---:|
| `int_stock_ma_daily` | success | 0.10s |
| `mart_stock_quotes_daily` | success | 116.12s |
| `mart_stock_trend_indicator` | success | 67.01s |
| `mart_stock_quotes_daily_adjusted_key_coverage` | pass | 9.03s |
| `mart_stock_quotes_daily_adjusted_passthrough_matches` | pass | 37.45s |
| `mart_stock_quotes_daily_key_set_matches_quotes` | pass | 8.77s |
| `mart_stock_quotes_daily_quote_passthrough_matches` | pass | 64.15s |
| `mart_stock_quotes_daily_financial_valuation_asof_matches` | pass | 28.33s |
| `mart_stock_trend_indicator_boll_bands_order` | pass | 1.96s |
| unique / not_null / code format tests | pass | all selected tests passed |

## 数据核验

补充说明：直接执行 `dbt show --select mart_stock_trend_indicator --limit 20`
会重新执行模型 SQL 的 join，而不是读取已物化 mart 表；本次在 ClickHouse
构建 join 右表时触发内存限制。随后改用带证券和日期过滤的
`dbt show --inline` 读取已物化 mart，避免无意义全表 join。

`calc_stock_ma_daily` schema：

| name | type | position |
|---|---|---:|
| `price_ma_28` | `Nullable(Float64)` | 11 |
| `price_ma_30` | `Nullable(Float64)` | 12 |
| `price_ma_57` | `Nullable(Float64)` | 13 |

表级计数：

| table | rows | symbols | duplicate_keys | min_date | max_date | key metric |
|---|---:|---:|---:|---|---|---:|
| `fleur_calculation.calc_stock_ma_daily` | 17,990,764 | 5,532 | 0 | 1995-01-03 | 2026-06-01 | `price_ma_30_non_null = 17,830,484` |
| `fleur_marts.mart_stock_trend_indicator` | 17,990,764 | 5,532 | 0 | 1995-01-03 | 2026-06-01 | `price_ma_30_non_null = 17,830,484` |
| `fleur_marts.mart_stock_quotes_daily` | 17,990,764 | 5,532 | 0 | 1995-01-03 | 2026-06-01 | `adjusted_open_non_null = 17,990,764` |

`000001.SZ` MA30 样本：

| security_code | trade_date | price_ma_5 | price_ma_10 | price_ma_30 |
|---|---|---:|---:|---:|
| 000001.SZ | 1995-02-20 | 0.4830652749055047 | 0.48495801919747006 | 0.4864968356950029 |
| 000001.SZ | 1995-02-21 | 0.48288061692580087 | 0.48468103222791414 | 0.48612751973559504 |
| 000001.SZ | 1995-02-22 | 0.48971296217484667 | 0.4879125468727331 | 0.4862506250553977 |

对应 `dbt show --inline`：

```bash
cd pipeline
uv run dbt show --project-dir elt --profiles-dir elt \
  --inline "select security_code, trade_date, price_ma_5, price_ma_10, price_ma_30 from {{ ref('mart_stock_trend_indicator') }} where trade_date >= '1995-01-01' and security_code = '000001.SZ' and price_ma_30 is not null order by trade_date" \
  --limit 5
```

`000001.SZ` 三种价格口径样本：

| trade_date | open_price | close_price | open_price_forward_adj | close_price_forward_adj | open_price_backward_adj | close_price_backward_adj | forward_adjustment_factor | backward_adjustment_factor |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1995-01-03 | 10.5 | 10.72 | 0.4847271967228402 | 0.49488338560655687 | 10.5 | 10.72 | 0.04616449492598478 | 1 |
| 2026-06-01 | 10.9 | 10.99 | 10.9 | 10.99 | 236.11219056029645 | 238.0617407575833 | 1 | 21.661668858742793 |

对应 `dbt show --inline`：

```bash
cd pipeline
uv run dbt show --project-dir elt --profiles-dir elt \
  --inline "select security_code, trade_date, open_price, close_price, open_price_forward_adj, close_price_forward_adj, open_price_backward_adj, close_price_backward_adj from {{ ref('mart_stock_quotes_daily') }} where security_code = '000001.SZ' and trade_date in ('1995-01-03', '2026-06-01') order by trade_date" \
  --limit 5
```

## Rearview Catalog

```bash
cd engines
set -a; . ../.env; set +a
cargo run -p rearview -- catalog check
cargo run -p rearview -- catalog sync
```

结果：

- `metric catalog check passed: 15 metrics`
- `metric catalog sync completed: 15 metrics, 15 rows affected`

## 质量门禁

已通过：

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p rearview`
- `uv run dbt parse --project-dir elt --profiles-dir elt`
- `uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_ma_daily mart_stock_trend_indicator mart_stock_quotes_daily --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'`

文档门禁已通过：

- `make docs-check`
- `git diff --check`
- 新增未跟踪文档和 SQL 测试文件的 `git diff --check --no-index`
