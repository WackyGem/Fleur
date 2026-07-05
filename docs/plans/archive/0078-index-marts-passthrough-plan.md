# Plan 0078: 指数 marts 透传实施计划

日期：2026-07-05

状态：Completed With Follow-Up

领域：dbt, data-platform

关联系统：

- `pipeline/elt/`
- `pipeline/elt/models/intermediate/int_index_basic_snapshot.sql`
- `pipeline/elt/models/intermediate/int_index_quotes_daily.sql`
- `pipeline/elt/models/marts/mart_stock_basic_snapshot.sql`
- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`
- `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`
- `pipeline/scheduler/src/scheduler/defs/daily/source_to_marts.py`

关联文档：

- [Data Platform Architecture](../../architecture/data-platform.md)
- [dbt layer architecture](../../architecture/dbt_layer/)
- [ADR 0007: dbt staging cleaning boundary](../../ADR/0007-dbt-staging-cleaning-boundary.md)
- [Plan 0042: Index benchmark intermediate implementation](0042-index-benchmark-intermediate-implementation-plan.md)

## 背景

`int_index_basic_snapshot` 和 `int_index_quotes_daily` 已在 intermediate 层提供 BaoStock 指数基础信息与指数日频行情。当前 marts 层已有 `mart_stock_basic_snapshot` 和 `mart_stock_quotes_daily` 作为股票消费层模型，但没有对应的通用指数 mart。已有 `mart_benchmark_returns_daily` 只覆盖组合绩效 benchmark 指数子集，不适合作为全量指数行情消费入口。

本计划按 stock mart 的风格新增指数 marts 透传层：

```text
int_index_basic_snapshot
  -> mart_index_basic_snapshot

int_index_quotes_daily
  -> mart_index_quotes_daily
```

## 目标

1. 新增 `mart_index_basic_snapshot`，作为指数基础显示信息 mart 当前快照。
2. 新增 `mart_index_quotes_daily`，作为指数日频行情和简单日收益 mart。
3. 模型命名参考 stock 风格，使用 `mart_index_*`，不使用数据源名或 benchmark 子集语义。
4. mart 层只透传 intermediate 层已确认字段，不重算指数日收益、不扩展指数 universe、不补齐行情缺口。
5. YAML 文档、字段描述、grain、业务逻辑说明和数据测试对齐现有 stock mart 风格。
6. 将新增 `mart_index_basic_snapshot` 和 `mart_index_quotes_daily` 纳入 `backfill__fetch_history_sources_to_marts_job` 与 `daily__fetch_history_sources_to_marts_schedule_job` 覆盖范围。
7. 通过定向 dbt parse/build/show、scheduler 单测和 `dg check defs` 验证新增 mart 可解析、可构建、可预览且编排可触达。

## 非目标

1. 不修改 `int_index_basic_snapshot` 或 `int_index_quotes_daily` 的现有逻辑。
2. 不新增指数成分股、行业指数、概念指数或 benchmark 配置表。
3. 不把 `int_benchmark_basic_snapshot` 或 `int_benchmark_returns_daily` 合并进通用指数 mart。
4. 不在 mart 层重新计算 `return_daily`、前收、成交量或成交金额。
5. 不新增指数复权、技术指标、估值指标或跨市场映射。
6. 不修改 source/raw/staging 数据抓取、ClickHouse raw sync 或 Dagster 编排。

## 当前事实基线

| 区域 | 当前事实 |
|---|---|
| 指数基础 int | `int_index_basic_snapshot` 从 `stg_baostock__query_stock_basic` 过滤 `security_type = 'index'`，一行代表一个 BaoStock 指数证券。 |
| 指数行情 int | `int_index_quotes_daily` 从 `stg_baostock__query_history_k_data_plus_daily` 取行情，并 inner join `int_index_basic_snapshot` 限定指数 universe。 |
| 日收益 | `int_index_quotes_daily.return_daily` 已按 `close_price / prev_close_price - 1` 计算，前收缺失或小于等于 0 时为 NULL。 |
| stock 基础 mart | `mart_stock_basic_snapshot` 使用 `mart_stock_*` 命名、`MergeTree()`、`order_by='security_code'`，只暴露消费层基础显示字段。 |
| stock 行情 mart | `mart_stock_quotes_daily` 使用 `order_by='(trade_date, security_code)'`、`partition_by='toYear(trade_date)'`，显式列字段并通过 `ref()` 读取 intermediate 模型。 |
| benchmark mart | `mart_benchmark_returns_daily` 是组合绩效 benchmark 子集，不替代全量指数行情 mart。 |
| source-to-marts backfill | `backfill__fetch_history_sources_to_marts_job` 通过 `source_to_marts_backfill.py` 的 `DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE` 和 `BAOSTOCK_MART_ASSET_KEYS` 选择 dbt marts。 |
| daily source-to-marts | `daily__fetch_history_sources_to_marts_schedule_job` 复用 source-to-marts registry；`ALL_SOURCE_TO_MARTS_SCOPE` 进入 daily schedule job 后应自动覆盖新增 index marts。 |

## 设计约束

1. 新增 SQL 必须显式列字段，不使用 `select *`。
2. 新增 SQL 只使用 `ref()`，不硬编码 ClickHouse database、schema 或物理表名。
3. `mart_index_basic_snapshot` 和 `mart_index_quotes_daily` 只读取对应 `int_index_*` 模型。
4. marts 层不写多来源 fallback，不猜测字段来源；字段必须来自已读过的 intermediate YAML/SQL。
5. 类型和 ClickHouse 表配置参考 stock mart，但不得为了“看起来一致”引入 stock 专属字段。
6. YAML 必须包含 `config.meta.grain_zh` 和 `business_logic_zh`，说明透传边界。
7. 测试策略随模型同时落地，不把测试留到后续补充。
8. 新增 mart asset keys 必须进入 `BAOSTOCK_MART_ASSET_KEYS`，从而同时被 BaoStock 单 scope、`all_source_to_marts`、backfill job 和 daily schedule job 覆盖。
9. scheduler 单测必须显式断言 `mart_index_basic_snapshot` 和 `mart_index_quotes_daily` 出现在 backfill/daily planned assets 中。

## 实施阶段

### Phase 0: 事实确认和字段冻结

目标：确认新增 mart 的字段范围完全来自现有 intermediate 模型。

实施项：

1. 读取并确认 `int_index_basic_snapshot.sql/.yml` 字段与 grain。
2. 读取并确认 `int_index_quotes_daily.sql/.yml` 字段与 grain。
3. 读取 `mart_stock_basic_snapshot.sql/.yml` 和 `mart_stock_quotes_daily.sql/.yml`，对齐命名、配置、YAML 文档和测试风格。
4. 确认 `mart_benchmark_returns_daily` 仍保持 benchmark 子集职责，不作为通用指数行情入口。

完成标准：

1. `mart_index_basic_snapshot` 字段清单冻结。
2. `mart_index_quotes_daily` 字段清单冻结。
3. 新增 mart 与 benchmark mart 的职责边界明确。

### Phase 1: 新增 `mart_index_basic_snapshot`

目标：把 `int_index_basic_snapshot` 透传为消费层指数基础信息快照。

实施项：

1. 新增 `pipeline/elt/models/marts/mart_index_basic_snapshot.sql`。
2. 新增 `pipeline/elt/models/marts/mart_index_basic_snapshot.yml`。
3. SQL 配置：
   - `materialized='table'`
   - `engine='MergeTree()'`
   - `order_by='security_code'`
4. 字段建议：
   - `security_code`
   - `security_local_code`
   - `index_name`
   - `exchange_code`
   - `listing_status`
   - `is_listed`
5. 参考 stock 基础 mart，对低基数字符串字段做受控 cast：
   - `cast(exchange_code, 'LowCardinality(String)') as exchange_code`
   - `cast(listing_status, 'LowCardinality(String)') as listing_status`
6. YAML 增加模型描述、`grain_zh: 每指数一行。` 和 `business_logic_zh`。
7. 添加数据测试：
   - `security_code`: `not_null`、`unique`、`cn_security_code_format`
   - model 级 `unique_combination_of_columns` on `security_code`
   - `exchange_code`: `not_null`、`accepted_values` for `SH/SZ/BJ`
   - `listing_status`: `accepted_values` for `listed/delisted`
   - `is_listed`: `not_null`

完成标准：

1. `mart_index_basic_snapshot` 可被 dbt parse 识别。
2. 字段只来自 `int_index_basic_snapshot`。
3. YAML 文档和测试与 stock 基础 mart 风格一致。

### Phase 2: 新增 `mart_index_quotes_daily`

目标：把 `int_index_quotes_daily` 透传为消费层指数日频行情 mart。

实施项：

1. 新增 `pipeline/elt/models/marts/mart_index_quotes_daily.sql`。
2. 新增 `pipeline/elt/models/marts/mart_index_quotes_daily.yml`。
3. SQL 配置：
   - `materialized='table'`
   - `engine='MergeTree()'`
   - `order_by='(trade_date, security_code)'`
   - `partition_by='toYear(trade_date)'`
4. 字段建议完整透传 `int_index_quotes_daily`：
   - `security_code`
   - `trade_date`
   - `open_price`
   - `high_price`
   - `low_price`
   - `close_price`
   - `prev_close_price`
   - `return_daily`
   - `volume`
   - `amount`
5. YAML 增加模型描述、`grain_zh: 每指数、交易日一行。` 和 `business_logic_zh`。
6. 明确 `return_daily` 来自 `int_index_quotes_daily`，mart 层不重算。
7. 添加数据测试：
   - model 级 `unique_combination_of_columns` on `security_code, trade_date`
   - `security_code`: `not_null`、`cn_security_code_format`
   - `trade_date`: `not_null`
   - `security_code` relationship 指向 `ref('mart_index_basic_snapshot')` 的 `security_code`

完成标准：

1. `mart_index_quotes_daily` 可被 dbt parse 识别。
2. `security_code, trade_date` 粒度测试通过。
3. 字段只来自 `int_index_quotes_daily`，不引入 stock 专属字段或 benchmark 子集字段。

### Phase 3: 纳入 source-to-marts 编排

目标：让新增指数 mart 在历史回填和每日调度中可触达。

实施项：

1. 更新 `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`。
2. 将 `mart_index_basic_snapshot` 和 `mart_index_quotes_daily` 加入 `BAOSTOCK_MART_ASSET_KEYS`。
3. 保持 `BAOSTOCK_INTERMEDIATE_ASSET_KEYS` 中既有 `int_index_basic_snapshot` 和 `int_index_quotes_daily` 覆盖，不重复定义新的 index scope。
4. 确认 `DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[BAOSTOCK_DAILY_KLINE_SCOPE][STAGE_DBT_MARTS]` 包含新增 index marts。
5. 确认 `DOWNSTREAM_STAGE_ASSET_KEYS_BY_SCOPE[ALL_SOURCE_TO_MARTS_SCOPE]` 通过 union 自动包含新增 index marts。
6. 更新 `pipeline/scheduler/tests/unit/automation/test_source_to_marts_backfill.py`：
   - 在 BaoStock scope 测试中断言 `mart_index_basic_snapshot` 和 `mart_index_quotes_daily` 被计划。
   - 在 `all_source_to_marts` coverage 测试中保留 `expected_dbt_coverage()` 对新增 assets 的覆盖。
7. 更新 `pipeline/scheduler/tests/unit/daily/test_source_to_marts.py`：
   - 在 daily registry 复用测试中断言 `mart_index_basic_snapshot` 和 `mart_index_quotes_daily` 出现在 `planned_dbt_assets`。
   - 保持 daily job 不直接维护独立资产列表，只验证它复用 source-to-marts registry。

完成标准：

1. `backfill__fetch_history_sources_to_marts_job` 的 BaoStock scope 和 `all_source_to_marts` scope 均覆盖两个新增 index marts。
2. `daily__fetch_history_sources_to_marts_schedule_job` 通过 `ALL_SOURCE_TO_MARTS_SCOPE` 覆盖两个新增 index marts。
3. scheduler 单测能机械证明 backfill 与 daily 两条入口都纳入新增资产。
4. 未新增独立 index scope，避免和 BaoStock daily kline/source lineage 脱节。

### Phase 4: 定向验证和数据预览

目标：证明新增模型可解析、可构建、可预览，且测试覆盖最小消费层契约。

实施项：

1. 运行 dbt parse。
2. 定向 build 两个新增 mart。
3. 对两个新增 mart 执行 `dbt show --limit 10`。
4. 如 ClickHouse 不可用，记录失败命令和错误摘要；静态 parse 仍必须通过。
5. 运行 scheduler 定向单测，验证 backfill 与 daily source-to-marts 覆盖。
6. 运行 `dg check defs`，确认新增 dbt asset keys 和 Dagster definitions 一致。
7. 运行文档-only 门禁。

完成标准：

1. dbt parse 通过。
2. 定向 dbt build 通过，或外部 ClickHouse 连接失败被记录为待补跑。
3. `dbt show` 能返回样例，或连接失败被记录为待补跑。
4. scheduler 定向单测通过。
5. `dg check defs` 通过。
6. `make docs-check` 和 `git diff --check` 通过。

### Phase 5: 完成记录和归档

目标：计划实施后留下可追溯结果。

实施项：

1. 将实施结果记录到 `docs/jobs/reports/YYYY-MM-DD-index-marts-passthrough.md`。
2. 实施报告必须记录 `backfill__fetch_history_sources_to_marts_job` 和 `daily__fetch_history_sources_to_marts_schedule_job` 的覆盖验证结果。
3. 完成后将本计划移入 `docs/plans/archive/`。
4. 更新 `docs/plans/README.md`：从 Active Plans 移除，加入 Recently Completed。

完成标准：

1. 实施报告包含命令、范围、结果和任何待补跑项。
2. 本计划状态更新并归档。
3. `docs/plans/README.md` 与计划状态一致。

## 最小验证命令

dbt 解析和构建：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select mart_index_basic_snapshot mart_index_quotes_daily --warn-error-options '{"error":["NoNodesForSelectionCriteria"]}'
```

数据预览：

```bash
cd pipeline
uv run dbt show --project-dir elt --profiles-dir elt --select mart_index_basic_snapshot --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_index_quotes_daily --limit 10
```

scheduler 编排验证：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/automation/test_source_to_marts_backfill.py scheduler/tests/unit/daily/test_source_to_marts.py
cd scheduler
uv run dg check defs
```

文档检查：

```bash
make docs-check
git diff --check
```

## 完成标准

1. `pipeline/elt/models/marts/mart_index_basic_snapshot.sql/.yml` 已新增。
2. `pipeline/elt/models/marts/mart_index_quotes_daily.sql/.yml` 已新增。
3. 两个 mart 均按 stock mart 风格命名、配置、显式列字段和编写 YAML。
4. 两个 mart 均只透传对应 `int_index_*` 模型，不新增计算逻辑。
5. `BAOSTOCK_MART_ASSET_KEYS` 包含 `mart_index_basic_snapshot` 和 `mart_index_quotes_daily`。
6. `backfill__fetch_history_sources_to_marts_job` 可通过 BaoStock scope 和 `all_source_to_marts` scope 计划两个新增 marts。
7. `daily__fetch_history_sources_to_marts_schedule_job` 可通过 `ALL_SOURCE_TO_MARTS_SCOPE` 计划两个新增 marts。
8. 定向 dbt parse/build/show 完成，或外部连接失败有明确待补跑记录。
9. scheduler 定向单测和 `dg check defs` 完成。
10. 文档门禁通过。
11. 实施报告和计划归档动作完成。
