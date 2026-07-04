# 2026-07-04 THS、ChinaBond 和 F10 marts 透传实施报告

日期：2026-07-04

范围：

- Plan 0077：`docs/plans/archive/0077-ths-chinabond-f10-marts-passthrough-plan.md`
- RFC 0048：`docs/RFC/0048-ths-chinabond-marts-passthrough.md`
- dbt：`pipeline/elt/models/intermediate/`、`pipeline/elt/models/marts/`
- Dagster：`pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`

## 实施结果

已完成：

1. 新增 THS 涨停池透传链路：`int_stock_limit_up_pool_daily`、`mart_stock_limit_up_pool_daily`。
2. 新增 ChinaBond 完整收益率曲线 mart：`mart_government_bond_yields_daily`；`mart_risk_free_rate_daily` 保持 worker-ready 无风险收益率职责。
3. 新增 EastMoney F10 9 条业务语义命名 passthrough 链路，downstream model name 不携带 `_eastmoney`。
4. `HOLDER_NEW -> holder_identifier` 和 `INFO_CODE -> announcement_identifier` 已前移到 staging；下游 int/mart 直接透传 canonical staging 字段。
5. `int_stock_dividend_plan` 先对完全相同 normalized row 执行 `select distinct`，再用全部基础字段生成 `dividend_plan_record_key`；`dividend_plan_group_key` 仅作为允许重复的业务分组键。
6. 新增 `pipeline/elt/scripts/validate_f10_passthrough_coverage.py`，机械校验 9 条 F10 staging -> int -> mart 字段完整性、受控重命名 lineage 和下游 `_eastmoney` 命名约束。
7. 更新 source-to-marts registry 和 daily controller 测试，使 `market_events`、`chinabond`、`eastmoney_f10` 和 `all_source_to_marts` 覆盖新增 dbt assets。
8. 刷新 Dagster dbt component state：`uv run dg utils refresh-defs-state`。
9. dev 运行态验收发现 THS ClickHouse DDL 的 `order_by` 不能直接使用 nullable `security_code`，已将 `int_stock_limit_up_pool_daily` 和 `mart_stock_limit_up_pool_daily` 的排序键改为 `assumeNotNull(security_code)`；输出字段仍原样透传 `security_code`。

## 静态验证

通过：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt --quiet
uv run python elt/scripts/validate_f10_passthrough_coverage.py
uv run ruff check scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py scheduler/tests/unit/automation/test_source_to_marts_backfill.py scheduler/tests/unit/daily/test_source_to_marts.py elt/scripts/validate_f10_passthrough_coverage.py
uv run pytest scheduler/tests/unit/automation/test_source_to_marts_backfill.py scheduler/tests/unit/daily/test_source_to_marts.py
uv run dg utils refresh-defs-state --target-path scheduler
uv run dg check defs --target-path scheduler
```

通过：

```bash
make docs-check
git diff --check
```

## dev 运行态验收

dev 基础设施已重建，prod RustFS S3 volume 已迁移到 dev RustFS volume。dev ClickHouse raw sync 和 dbt build 使用同一份迁移后的 S3 数据完成验收。

Dagster raw sync：

```bash
cd pipeline
uv run dg launch --target-path scheduler --job backfill__fetch_history_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_history_sources_to_raw_controller":{"config":{"target_scope":"chinabond","start_date":"2006-01-01","end_date":"2026-07-04","execution_mode":"raw_only","refresh_prerequisite_snapshots":false,"overwrite_source_partitions":false,"dry_run":false}}}}'
uv run dg launch --target-path scheduler --job backfill__fetch_history_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_history_sources_to_raw_controller":{"config":{"target_scope":"eastmoney_f10","start_date":"1990-01-01","end_date":"2026-07-04","execution_mode":"raw_only","refresh_prerequisite_snapshots":false,"overwrite_source_partitions":false,"dry_run":false}}}}'
uv run dg launch --target-path scheduler --job backfill__fetch_history_sources_to_raw_job \
  --config-json '{"ops":{"backfill__fetch_history_sources_to_raw_controller":{"config":{"target_scope":"market_events","start_date":"2025-01-01","end_date":"2026-07-04","execution_mode":"raw_only","refresh_prerequisite_snapshots":false,"overwrite_source_partitions":false,"dry_run":false}}}}'
```

raw 行数：

| raw table | rows |
|---|---:|
| `fleur_raw.ths__limit_up_pool_compacted` | 16,917 |
| `fleur_raw.chinabond__government_bond` | 5,087 |
| `fleur_raw.eastmoney__balance` | 284,265 |
| `fleur_raw.eastmoney__cashflow_sq` | 274,016 |
| `fleur_raw.eastmoney__cashflow_ytd` | 283,613 |
| `fleur_raw.eastmoney__dividend_allotment` | 1,156 |
| `fleur_raw.eastmoney__dividend_main` | 151,613 |
| `fleur_raw.eastmoney__equity_history` | 147,567 |
| `fleur_raw.eastmoney__freeholders` | 2,737,351 |
| `fleur_raw.eastmoney__income_sq` | 279,918 |
| `fleur_raw.eastmoney__income_ytd` | 298,396 |

说明：

- `market_events` raw-only controller 已调整为 THS-only，dry-run 和真实执行均只提交 `clickhouse/raw/ths__limit_up_pool_compacted`。
- Jiuyan 异动不再进入统一 `market_events` raw-only 或 source-to-marts 入口。

dbt build：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt --quiet
uv run dbt build --project-dir elt --profiles-dir elt --select +int_stock_limit_up_pool_daily +mart_stock_limit_up_pool_daily +mart_government_bond_yields_daily +int_stock_balance_sheet +int_stock_cashflow_statement_quarterly +int_stock_cashflow_statement_ytd +int_stock_allotment_event +int_stock_dividend_plan +int_stock_share_capital_history +int_stock_free_float_shareholder_top10 +int_stock_income_statement_quarterly +int_stock_income_statement_ytd +mart_stock_balance_sheet +mart_stock_cashflow_statement_quarterly +mart_stock_cashflow_statement_ytd +mart_stock_allotment_event +mart_stock_dividend_plan +mart_stock_share_capital_history +mart_stock_free_float_shareholder_top10 +mart_stock_income_statement_quarterly +mart_stock_income_statement_ytd --quiet --warn-error-options '{"error":["NoNodesForSelectionCriteria"]}'
```

结果：

| status | count |
|---|---:|
| model success | 36 |
| test pass | 322 |

目标 mart 行数：

| mart table | rows |
|---|---:|
| `fleur_marts.mart_stock_limit_up_pool_daily` | 16,917 |
| `fleur_marts.mart_government_bond_yields_daily` | 5,087 |
| `fleur_marts.mart_stock_balance_sheet` | 284,265 |
| `fleur_marts.mart_stock_dividend_plan` | 151,613 |
| `fleur_marts.mart_stock_free_float_shareholder_top10` | 2,737,351 |

代表性 `dbt show --limit 10` 已通过：

- `mart_stock_limit_up_pool_daily`
- `mart_government_bond_yields_daily`
- `mart_stock_balance_sheet`
- `mart_stock_cashflow_statement_quarterly`
- `mart_stock_dividend_plan`
- `mart_stock_free_float_shareholder_top10`

分红唯一性 profile：

| check | result |
|---|---:|
| `stg_eastmoney__dividend_main` rows | 151,613 |
| `announcement_identifier` NULL rows | 70,499 |
| distinct non-NULL `announcement_identifier` | 81,073 |
| duplicate `security_code, report_period_label` groups | 20 |
| max rows per duplicate group | 2 |
| duplicate full-record fingerprint groups | 0 |
| max rows per duplicate full-record fingerprint | 0 |

结论：

- dev ClickHouse 运行态 build、show、字段覆盖和分红唯一性 profile 均已完成。
- `announcement_identifier` 不能作为唯一键；`security_code + report_period_label` 也存在历史重复，继续使用 `dividend_plan_record_key` 作为唯一记录键、`dividend_plan_group_key` 作为允许重复的业务分组键。
