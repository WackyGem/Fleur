# 2026-07-05 指数 marts 透传实施报告

日期：2026-07-05

范围：

- Plan 0078：`docs/plans/archive/0078-index-marts-passthrough-plan.md`
- dbt：`pipeline/elt/models/marts/mart_index_basic_snapshot.*`、`pipeline/elt/models/marts/mart_index_quotes_daily.*`
- Dagster：`pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`
- 测试：`pipeline/scheduler/tests/unit/automation/test_source_to_marts_backfill.py`、`pipeline/scheduler/tests/unit/daily/test_source_to_marts.py`

## 实施结果

已完成：

1. 新增 `mart_index_basic_snapshot`，从 `int_index_basic_snapshot` 透传指数基础显示字段：`security_code`、`security_local_code`、`index_name`、`exchange_code`、`listing_status`、`is_listed`。
2. 新增 `mart_index_quotes_daily`，从 `int_index_quotes_daily` 透传指数日频行情字段：`security_code`、`trade_date`、OHLC、`prev_close_price`、`return_daily`、`volume`、`amount`。
3. `mart_index_quotes_daily.return_daily` 仅透传 intermediate 结果，mart 层不重算日收益、不扩展指数 universe、不补齐行情缺口。
4. 两个新增 mart 已补充 YAML 描述、`grain_zh`、`business_logic_zh` 和数据测试。
5. `mart_index_basic_snapshot` 和 `mart_index_quotes_daily` 已加入 `BAOSTOCK_MART_ASSET_KEYS`，通过 `all_source_to_marts` 被 backfill 和 daily 入口覆盖。
6. scheduler 单测已显式断言 BaoStock backfill mart step、`all_source_to_marts` coverage 和 daily planned dbt assets 包含两个新增 index marts。
7. 已刷新 Dagster dbt component state：`DAGSTER_HOME=/storage/program/fleur/.dagster uv run dg utils refresh-defs-state`。

## 验证结果

通过：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

通过：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/automation/test_source_to_marts_backfill.py scheduler/tests/unit/daily/test_source_to_marts.py
```

结果：`25 passed`。

通过：

```bash
cd pipeline/scheduler
DAGSTER_HOME=/storage/program/fleur/.dagster uv run dg check defs
```

结果：component YAML 和 definitions 均加载成功。

## 待补跑项

以下命令已执行，但本地 ClickHouse 认证失败，需修复 `fleur` 用户或 profiles 凭据后补跑：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select mart_index_basic_snapshot mart_index_quotes_daily --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run dbt show --project-dir elt --profiles-dir elt --select mart_index_basic_snapshot --limit 10 --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
uv run dbt show --project-dir elt --profiles-dir elt --select mart_index_quotes_daily --limit 10 --quiet --warn-error-options '{"error": ["NoNodesForSelectionCriteria"]}'
```

错误摘要：

```text
DB::Exception: fleur: Authentication failed: password is incorrect, or there is no user with such name. (AUTHENTICATION_FAILED) (for url http://127.0.0.1:34052)
```

## 覆盖结论

- `backfill__fetch_history_sources_to_marts_job` 的 BaoStock scope 和 `all_source_to_marts` scope 均覆盖 `mart_index_basic_snapshot`、`mart_index_quotes_daily`。
- `daily__fetch_history_sources_to_marts_schedule_job` 复用 `ALL_SOURCE_TO_MARTS_SCOPE`，planned dbt assets 已覆盖两个新增 index marts。
- `mart_benchmark_returns_daily` 继续保持 benchmark 子集日收益入口职责，不替代全量指数行情 mart。
