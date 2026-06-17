# Index and Benchmark Intermediate Build

日期：2026-06-16

## 结论

Plan 0042 已完成并归档：

- 新增并全量构建 `fleur_intermediate.int_index_basic_snapshot`。
- 新增并全量构建 `fleur_intermediate.int_index_quotes_daily`。
- 新增并全量构建 `fleur_intermediate.int_benchmark_basic_snapshot`。
- 新增并全量构建 `fleur_intermediate.int_benchmark_returns_daily`。
- 第一版 benchmark 只保留 raw profile 已验证可用的 6 个价格指数口径 benchmark。

## 范围

输入：

- `fleur_staging.stg_baostock__query_stock_basic`
- `fleur_staging.stg_baostock__query_history_k_data_plus_daily`

输出：

- `fleur_intermediate.int_index_basic_snapshot`
- `fleur_intermediate.int_index_quotes_daily`
- `fleur_intermediate.int_benchmark_basic_snapshot`
- `fleur_intermediate.int_benchmark_returns_daily`

## 执行命令

```bash
cd pipeline
set -a
. ../.env
set +a
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_index_basic_snapshot int_index_quotes_daily int_benchmark_basic_snapshot int_benchmark_returns_daily \
  --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run python elt/scripts/validate_field_glossary.py
```

文档和空白检查：

```bash
make docs-check
git diff --check
```

## dbt 结果

定向 `dbt build` 结果：

| 资源类型 | 结果 |
|---|---:|
| table models | 4 created |
| data tests | 41 passed |
| warnings | 0 |
| errors | 0 |
| total selected resources | 48 |

执行中修复了两个 ClickHouse inline CTE 别名解析问题：

- `int_benchmark_basic_snapshot` 的 mapping 代码列改为 `benchmark_security_code` 后 join。
- `int_benchmark_returns_daily` 的 benchmark 侧代码列改为 `benchmark_security_code` 后 join。

## 数据核验

表级核验：

| model_name | row_count | min_date | max_date |
|---|---:|---|---|
| `int_index_basic_snapshot` | 596 | 1991-04-04 | 2015-10-29 |
| `int_index_quotes_daily` | 2,100,980 | 2006-01-04 | 2026-06-01 |
| `int_benchmark_basic_snapshot` | 6 |  |  |
| `int_benchmark_returns_daily` | 27,012 | 2006-01-04 | 2026-06-01 |

Benchmark 覆盖：

| benchmark_key | benchmark_name | security_code | rows | first_trade_date | last_trade_date | null_return_rows |
|---|---|---|---:|---|---|---:|
| `cnindex_1000` | 国证1000 | `399311.SZ` | 4,955 | 2006-01-04 | 2026-06-01 | 0 |
| `csi_1000` | 中证1000 | `000852.SH` | 2,824 | 2014-10-17 | 2026-06-01 | 1 |
| `csi_300` | 沪深300 | `000300.SH` | 4,955 | 2006-01-04 | 2026-06-01 | 0 |
| `csi_500` | 中证500 | `000905.SH` | 4,707 | 2007-01-15 | 2026-06-01 | 0 |
| `csi_800` | 中证800 | `000906.SH` | 4,707 | 2007-01-15 | 2026-06-01 | 1 |
| `csi_a100` | 中证A100 | `000903.SH` | 4,864 | 2006-05-29 | 2026-06-01 | 1 |

`return_daily` NULL 行来自首日或 BaoStock `prev_close_price` 不可用于收益计算的记录；模型保留这些行并将收益置为 NULL。

## 验证结论

- 四个 intermediate 模型已物化为 ClickHouse table。
- 组合键、not-null、accepted-values、security code format 和 relationships 测试均通过。
- `int_benchmark_basic_snapshot` 只包含 6 个已验证可用 benchmark。
- `int_benchmark_returns_daily` 已按 benchmark 全量填充至 2026-06-01。
- Field glossary lint、docs governance 和 diff whitespace 检查均通过。
