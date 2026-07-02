# Furnace Price Pattern Rebuild-Table Rerun

日期：2026-07-02

## 结论

本次重跑完成价格行为结构指标的新 N 字结构字段落库：

- `fleur_calculation.calc_stock_price_pattern_daily` 已通过 Furnace `rebuild-table` 全量重建。
- `fleur_intermediate.int_stock_price_pattern_daily` 和 `fleur_marts.mart_stock_price_pattern_daily` 已通过 dbt build 重建。
- calculation、intermediate、mart 三层行数和日期范围一致。
- 新字段 `n_structure_20_stage`、`n_structure_20_higher_low_ratio`、`n_structure_20_pullback_depth`、`n_structure_20_rebound_ratio` 已可查询。

ClickHouse 约束：

- Per `insert-mutation-avoid-delete`：没有使用 `ALTER TABLE DELETE` 或 mutation 清理旧数据；本次使用 `DROP TABLE IF EXISTS` 后按 canonical DDL 重建输出表。
- Per `insert-batch-size`：Furnace 继续使用默认 100,000 行/批写入，保持在 ClickHouse 推荐批量范围内。
- Per `schema-pk-plan-before-creation`：新 schema 通过重建 canonical 输出表生效，不尝试修改既有 `ORDER BY`。

## 输入范围

后复权和未复权输入表日期范围一致：

```sql
select
  min(trade_date) as min_trade_date,
  max(trade_date) as max_trade_date,
  count() as rows
from {{ ref('int_stock_quotes_daily_adj') }}
```

```text
min_trade_date: 2021-01-04
max_trade_date: 2026-06-30
rows: 6,515,075
```

`int_stock_quotes_daily_unadj` 抽查得到相同范围和行数。

## Furnace 重建

Release build：

```bash
cd engines
cargo build -p furnace --release
```

全市场 rebuild-table：

```bash
cd engines
set -a; source ../.env; set +a
./target/release/furnace price-pattern \
  --from 2021-01-04 \
  --to 2026-06-30 \
  --mode rebuild-table \
  --run-id price-pattern-rebuild-20260702 \
  --output-format json
```

摘要：

```json
{
  "indicator": "price_pattern",
  "request_from": "2021-01-04",
  "request_to": "2026-06-30",
  "effective_output_to": "2026-06-30",
  "mode": "rebuild-table",
  "symbols_count": 5407,
  "input_rows": 6515075,
  "output_rows": 6515075,
  "valid_streak_rows": 6515074,
  "valid_structure_bar_rows": 6515074,
  "null_streak_rows": 1,
  "null_n_structure_rows": 1733562,
  "affected_years": [2021, 2022, 2023, 2024, 2025, 2026],
  "retained_rows": 0,
  "staging_table": null,
  "staging_validation": {"status": "not_applicable", "duplicate_keys": 0},
  "partition_replace": {"status": "not_applicable", "years": []},
  "state_source": "full-history",
  "n_structure_window": 20,
  "run_id": "price-pattern-rebuild-20260702",
  "writes_applied": true,
  "performance_metrics": {
    "total_ms": 19094,
    "read_input_ms": 2888,
    "group_ms": 1371,
    "compute_ms": 2746,
    "write_ms": 11807,
    "staging_ms": 11,
    "partition_replace_ms": 0,
    "parallelism": "rayon",
    "worker_threads": 20
  }
}
```

## dbt 重建

```bash
cd pipeline
uv run dbt build \
  --project-dir elt \
  --profiles-dir elt \
  --select int_stock_price_pattern_daily mart_stock_price_pattern_daily \
  --quiet \
  --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

结果：通过。

## 数据核验

Calculation source：

| min_trade_date | max_trade_date | rows | valid_n_rows |
|---|---:|---:|---:|
| 2021-01-04 | 2026-06-30 | 6,515,075 | 3,180,019 |

Intermediate：

| min_trade_date | max_trade_date | rows | valid_n_rows | stage_rows |
|---|---:|---:|---:|---:|
| 2021-01-04 | 2026-06-30 | 6,515,075 | 3,180,019 | 6,515,075 |

Mart：

| min_trade_date | max_trade_date | rows | valid_n_rows | stage_rows |
|---|---:|---:|---:|---:|
| 2021-01-04 | 2026-06-30 | 6,515,075 | 3,180,019 | 6,515,075 |

Mart `n_structure_20_stage` 分布：

| n_structure_20_stage | rows |
|---|---:|
| breakout | 2,725,887 |
| none | 1,733,562 |
| higher_low | 1,601,494 |
| rebound | 454,132 |

## 后续使用

本次确认 `rebuild-table` 可用于 Furnace 输出表 schema 或算法变化后的全量刷新。调用方必须把请求范围视为重建后新表的完整内容；局部范围会重建成局部结果表。
