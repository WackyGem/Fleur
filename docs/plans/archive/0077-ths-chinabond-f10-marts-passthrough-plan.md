# Plan 0077: THS、ChinaBond 和 F10 marts 透传实施计划

日期：2026-07-04

状态：Completed With Follow-Up

领域：dbt, Dagster, ClickHouse, data-platform

关联系统：

- `pipeline/elt/`
- `pipeline/scheduler/`
- `docs/references/raw_profile/`
- `docs/references/data_dict/`

关联文档：

- [RFC 0048: 同花顺、中债和东方财富数据透传到 marts 层](../../RFC/0048-ths-chinabond-marts-passthrough.md)
- [Data Platform Architecture](../../architecture/data-platform.md)
- [ADR 0005: Dagster owns ClickHouse raw sync, dbt owns modeling](../../ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md)
- [ADR 0007: dbt staging cleaning boundary](../../ADR/0007-dbt-staging-cleaning-boundary.md)
- [ADR 0008: raw source profiling before dbt staging](../../ADR/0008-raw-source-profiling-before-dbt-staging.md)
- [2026-07-01 source-to-marts controller dry-run](../../jobs/reports/2026-07-01-backfill-source-to-marts-controller-dry-run.md)
- [2026-07-04 THS/ChinaBond/F10 marts passthrough implementation report](../../jobs/reports/2026-07-04-ths-chinabond-f10-marts-passthrough.md)

## 背景

RFC 0048 已完成同花顺涨停池、中债国债收益率曲线和东方财富 9 个 F10 staging 的 stg/int/mart 资源盘点，并确定第一版采用 thin passthrough：

```text
stg_ths__limit_up_pool_compacted
  -> int_stock_limit_up_pool_daily
  -> mart_stock_limit_up_pool_daily

stg_chinabond__government_bond
  -> int_government_bond_yields_daily
  -> mart_government_bond_yields_daily

stg_eastmoney__*
  -> int_stock_*
  -> mart_stock_*
```

本计划把 RFC 0048 拆成可实施阶段，覆盖 dbt 模型、YAML 文档、数据测试、字段完整性核验和 source-to-marts controller scope 更新。

## 目标

1. 新增同花顺涨停池 `int_stock_limit_up_pool_daily` 和 `mart_stock_limit_up_pool_daily`。
2. 新增中债完整收益率曲线 `mart_government_bond_yields_daily`，保留 `mart_risk_free_rate_daily` 的 worker-ready 职责。
3. 新增东方财富 9 个 F10 业务语义命名的 `int_stock_*` 和 `mart_stock_*` 透传模型。
4. 保证 intermediate 和 mart 模型名不携带 `_eastmoney` 数据源标识。
5. 保证东方财富 9 个 staging 的全部字段传递到对应 mart；宽表不得裁剪财务字段。
6. 为 `int_stock_dividend_plan` 建立可复现的记录唯一性：`dividend_plan_record_key` 唯一，`dividend_plan_group_key` 允许重复。
7. 扩展 `source_to_marts_backfill.py` scope，使 daily 和 history source-to-marts controller 覆盖新增资产。
8. 补齐最小数据测试、字段 lineage 文档和定向验证命令。

## 非目标

1. 不修改 source/raw 数据抓取、ClickHouse raw sync、S3 分区策略或数据契约。
2. 不改变 staging 清洗边界；除非字段完整性核验发现 staging 本身缺字段，否则不重写 staging。
3. 不在透传层做财报口径合并、分红配股合并、历史版本纠错、最新版本选择或指标重算。
4. 不解析 THS `reason_type` 题材文本，不新增涨停排序派生字段。
5. 不对 ChinaBond 15 年、20 年空值做填补、插值或删除。
6. 不替代现有 `int_stock_financial_valuation`、`int_stock_exrights_event`、`int_stock_shares_history` 或 `mart_stock_quotes_daily` 的业务派生职责。

## 实施前事实基线

| 区域 | 当前事实 |
|---|---|
| THS | `stg_ths__limit_up_pool_compacted` 当前无下游；`MARKET_EVENTS_SCOPE` 只覆盖 staging。 |
| ChinaBond | `int_government_bond_yields_daily` 已透传完整期限曲线；marts 只暴露 `mart_risk_free_rate_daily`。 |
| EastMoney F10 | 9 个 `stg_eastmoney__*` 已存在；现金流两张 staging 无下游，其余只被业务派生模型间接消费。 |
| F10 命名 | RFC 0048 要求 downstream model 使用业务语义名，`_eastmoney` 只留在 raw/source/staging 层。 |
| F10 字段 | RFC 0048 要求 9 个 staging 字段全部进入 mart，受控重命名必须在 YAML `meta.source_columns` 记录上游 staging column lineage。 |
| 分红唯一性 | `stg_eastmoney__dividend_main` 不能用 `info_code` 或 `security_code + report_period_label` 做唯一键。 |
| 编排 | `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py` 是 source-to-marts scope registry。 |

## 设计约束

1. dbt SQL 必须显式列字段，不使用 `select *`。
2. 新增 int/mart SQL 只使用 `ref()`，不硬编码 ClickHouse schema 或表名。
3. F10 下游模型文件名和 model name 不允许出现 `_eastmoney`。
4. F10 字段名不新增数据源标识；已有 staging 字段含供应商标识时，下游改为业务泛化名，并在 YAML 记录来源字段。
5. F10 mart 必须包含对应 intermediate 的全部字段；intermediate 必须覆盖对应 staging 的全部字段。
6. `int_stock_dividend_plan` 不按公告日期、进度、除权日或股东大会日期挑选“正确”历史记录。
7. `dividend_plan_record_key` 基于规范化后的全部 staging base 字段生成；受控重命名使用下游字段名，字段顺序固定，NULL 显式编码，不包含派生键本身。
8. 字段完整性验收必须有机械校验脚本，不能只依赖人工列数核对。
9. controller scope 更新必须保持 `all_source_to_marts`、daily controller 和单 scope backfill 的覆盖集合一致。

## 实施阶段

### Phase 0: 前置 profile 与字段清单冻结

目标：先确认 RFC 0048 的关键数据假设仍成立，避免在实现中用推测补键或裁字段。

实施项：

1. 运行 `dbt parse`，确认当前项目可解析。
2. 补跑 `stg_eastmoney__dividend_main` profile：
   - `info_code` NULL 数量和非 NULL 基数。
   - `security_code, report_period_label` 重复组数量。
   - 全字段 normalized fingerprint 是否存在重复。
3. 从 9 个 EastMoney staging SQL/YAML 抽取字段清单，冻结 staging -> int -> mart 字段覆盖表。
4. 对 `holder_eastmoney_code -> holder_identifier` 等受控重命名列出 lineage 表。
5. 若 ClickHouse HTTP endpoint 不可用，记录失败原因；后续实施可继续做 SQL/YAML 和 parse，但不得跳过最终数据 profile 验收。

完成标准：

1. `int_stock_dividend_plan` 的 record/group key 设计有当前 profile 证据支撑。
2. 9 个 F10 staging 的字段清单已可用于实现和验收。
3. 发现 staging 缺字段时，先暂停对应链路并补充 staging 事实，不在 int/mart 中猜字段。

### Phase 1: THS 涨停池 int/mart

目标：把 `stg_ths__limit_up_pool_compacted` 透传到 marts。

实施项：

1. 新增 `pipeline/elt/models/intermediate/int_stock_limit_up_pool_daily.sql`。
2. 新增 `pipeline/elt/models/intermediate/int_stock_limit_up_pool_daily.yml`。
3. 新增 `pipeline/elt/models/marts/mart_stock_limit_up_pool_daily.sql`。
4. 新增 `pipeline/elt/models/marts/mart_stock_limit_up_pool_daily.yml`。
5. 添加 `trade_date, security_code` 粒度说明和唯一性测试。
6. 在 mart YAML 复用 `trade_date, security_code` grain 测试和关键 not null/格式测试。
7. 保留 staging 已规范字段，不解析 `reason_type`，不新增 market event 派生字段。

完成标准：

1. `int_stock_limit_up_pool_daily` 和 `mart_stock_limit_up_pool_daily` 字段与 RFC 0048 列表一致。
2. `trade_date`、`security_code` not null，`security_code` 保留 A 股代码格式测试。
3. int 和 mart 都有 `trade_date, security_code` 唯一性测试。
4. 新增模型可被定向 `dbt build` 选中并通过。

### Phase 2: ChinaBond 完整收益率曲线 mart

目标：在 marts 层暴露完整国债收益率曲线宽表。

实施项：

1. 新增 `pipeline/elt/models/marts/mart_government_bond_yields_daily.sql`。
2. 新增 `pipeline/elt/models/marts/mart_government_bond_yields_daily.yml`。
3. 从 `int_government_bond_yields_daily` 透传 `trade_date` 和 11 个期限收益率字段。
4. 保持收益率单位为百分比点，不转换为小数比例。
5. 保持 `mart_risk_free_rate_daily` 不改名、不并入新 mart。

完成标准：

1. `trade_date` unique + not null 测试通过。
2. 当前上游应非空期限保留 not null 测试；15 年、20 年继续 nullable。
3. `mart_government_bond_yields_daily` 和 `mart_risk_free_rate_daily` 职责在 YAML 中明确区分。

### Phase 3: EastMoney F10 9 条 int/mart 透传链路

目标：新增 9 个 F10 passthrough intermediate 和 9 个 mart，模型名使用业务语义且字段完整。

实施项：

1. 新增 9 个 intermediate SQL/YAML：
   - `int_stock_balance_sheet`
   - `int_stock_cashflow_statement_quarterly`
   - `int_stock_cashflow_statement_ytd`
   - `int_stock_allotment_event`
   - `int_stock_dividend_plan`
   - `int_stock_share_capital_history`
   - `int_stock_free_float_shareholder_top10`
   - `int_stock_income_statement_quarterly`
   - `int_stock_income_statement_ytd`
2. 新增 9 个 mart SQL/YAML：
   - `mart_stock_balance_sheet`
   - `mart_stock_cashflow_statement_quarterly`
   - `mart_stock_cashflow_statement_ytd`
   - `mart_stock_allotment_event`
   - `mart_stock_dividend_plan`
   - `mart_stock_share_capital_history`
   - `mart_stock_free_float_shareholder_top10`
   - `mart_stock_income_statement_quarterly`
   - `mart_stock_income_statement_ytd`
3. 每个 int 只从对应 `stg_eastmoney__*` 选择字段，不 join，不合并 SQ/YTD，不重算指标。
4. 每个 mart 只从对应 `int_stock_*` 选择字段。
5. 宽表 SQL 按 staging YAML 字段清单显式列出全部字段。
6. YAML 中为受控重命名字段写 `meta.source_columns`，指向上游 staging model column；至少覆盖 `holder_eastmoney_code -> holder_identifier` 和 `info_code -> announcement_identifier`。
7. 对 `int_stock_dividend_plan` 先对完全相同 normalized row 做受控 `distinct`，再生成 `dividend_plan_record_key`；若 profile 要求保留物理重复行，则停止并提出 raw ingestion row id 前置改造。
8. 新增 `pipeline/elt/scripts/validate_f10_passthrough_coverage.py`，读取 9 组 staging/int/mart YAML 和 dbt manifest，校验：
   - 每个 staging 字段在 int 和 mart 中都有同名字段或白名单受控重命名。
   - 受控重命名字段必须有 `config.meta.source_columns` 指回上游 staging model column，例如 `source: model, model: stg_eastmoney__freeholders, column: holder_eastmoney_code`。
   - 只允许白名单新增派生字段，例如 `dividend_plan_record_key`、`dividend_plan_group_key`；`announcement_identifier` 属于 `info_code` 的受控重命名，不属于派生字段。
   - int/mart model name 和 column name 不含 `_eastmoney`。
9. 为 9 个 mart 复用对应 int 的 grain 唯一性、`security_code` not null/格式和日期主键 not null 测试。

完成标准：

1. `pipeline/elt/models/intermediate/` 和 `pipeline/elt/models/marts/` 中新增模型名不含 `_eastmoney`。
2. 9 条 F10 staging -> int -> mart 字段覆盖无缺失；受控重命名有 YAML lineage。
3. `dividend_plan_record_key` unique 测试通过。
4. `dividend_plan_group_key` 不加 unique 测试，允许历史版本重复。
5. `announcement_identifier` 不加 not_null 或 unique 测试。

### Phase 4: source-to-marts controller scope

目标：让 daily 和 history source-to-marts controller 包含新增 THS、ChinaBond 和 F10 mart。

实施项：

1. 更新 `pipeline/scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py`：
   - 新增 THS intermediate/mart asset keys。
   - 将 `mart_government_bond_yields_daily` 加入 `CHINABOND_MART_ASSET_KEYS`。
   - 新增 F10 passthrough intermediate/mart asset key tuple。
   - 将 F10 passthrough keys 合并进 `EASTMONEY_F10_SCOPE`。
2. 更新 `pipeline/scheduler/tests/unit/automation/test_source_to_marts_backfill.py` 中 scope 覆盖断言；`test_market_events_scope_filters_jiuyan_and_keeps_ths_staging` 应改名并断言 `int_stock_limit_up_pool_daily`、`mart_stock_limit_up_pool_daily` 也在计划内。
3. 更新 `pipeline/scheduler/tests/unit/daily/test_source_to_marts.py` 中 daily registry 复用断言。
4. 若 definitions check 暴露新 dbt asset key 缺失，先修 dbt manifest/asset 加载问题，再调整测试。

完成标准：

1. `market_events` scope 包含 THS staging、int、mart。
2. `chinabond` scope 同时包含完整曲线 mart 和 risk-free mart。
3. `eastmoney_f10` scope 包含原业务派生链路和 9 条 F10 passthrough mart。
4. `all_source_to_marts` 覆盖集合自动包含新增 dbt asset，且继续排除 Jiuyan 和 portfolio 独立域。

### Phase 5: 验证、报告和归档准备

目标：用定向命令证明新增链路可解析、可构建、字段完整且 controller 展开正确。

实施项：

1. 运行 dbt parse 和定向 build。
2. 对 THS、ChinaBond 和至少 4 个代表性 F10 mart 执行 `dbt show --limit 10`。
3. 对 9 个 F10 链路执行字段完整性核验。
4. 运行 scheduler 定向 ruff、pytest 和 `dg check defs`。
5. 运行文档检查。
6. 若生产或 dev ClickHouse 不可用，在实施报告中记录不可执行命令、错误摘要和仍已通过的静态验证。

完成标准：

1. 本计划的完成结果写入 `docs/jobs/reports/YYYY-MM-DD-ths-chinabond-f10-marts-passthrough.md`。
2. RFC 0048 状态可从 `Proposed` 更新为实施完成后的状态。
3. 本计划完成后移入 `docs/plans/archive/`，并更新 `docs/plans/README.md`。

## 最小验证命令

dbt 解析和构建：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_limit_up_pool_daily mart_stock_limit_up_pool_daily mart_government_bond_yields_daily int_stock_balance_sheet int_stock_cashflow_statement_quarterly int_stock_cashflow_statement_ytd int_stock_allotment_event int_stock_dividend_plan int_stock_share_capital_history int_stock_free_float_shareholder_top10 int_stock_income_statement_quarterly int_stock_income_statement_ytd mart_stock_balance_sheet mart_stock_cashflow_statement_quarterly mart_stock_cashflow_statement_ytd mart_stock_allotment_event mart_stock_dividend_plan mart_stock_share_capital_history mart_stock_free_float_shareholder_top10 mart_stock_income_statement_quarterly mart_stock_income_statement_ytd --warn-error-options '{"error":["NoNodesForSelectionCriteria"]}'
```

代表性数据预览：

```bash
cd pipeline
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_limit_up_pool_daily --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_government_bond_yields_daily --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_balance_sheet --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_cashflow_statement_quarterly --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_dividend_plan --limit 10
uv run dbt show --project-dir elt --profiles-dir elt --select mart_stock_free_float_shareholder_top10 --limit 10
```

分红唯一性 profile：

```bash
cd pipeline
uv run dbt show --project-dir elt --profiles-dir elt --inline "select count() as rows, countIf(info_code is null) as info_code_null_rows, uniqExactIf(info_code, info_code is not null) as distinct_info_code_nonnull from {{ ref('stg_eastmoney__dividend_main') }}"
uv run dbt show --project-dir elt --profiles-dir elt --inline "select count() as duplicate_group_count, max(row_count) as max_rows_per_group from (select security_code, report_period_label, count() as row_count from {{ ref('stg_eastmoney__dividend_main') }} group by security_code, report_period_label having row_count > 1)"
uv run dbt show --project-dir elt --profiles-dir elt --inline "select count() as duplicate_record_fingerprint_count, max(row_count) as max_rows_per_fingerprint from (select security_code, security_name_abbr, notice_date, report_period_label, report_date, assign_progress, is_unassign, impl_plan_profile, impl_plan_newprofile, new_profile, assign_object, equity_record_date, ex_dividend_date, pay_cash_date, gmdecision_notice_date, annual_general_meeting_date, info_code, total_dividend, total_dividend_a, count() as row_count from {{ ref('stg_eastmoney__dividend_main') }} group by security_code, security_name_abbr, notice_date, report_period_label, report_date, assign_progress, is_unassign, impl_plan_profile, impl_plan_newprofile, new_profile, assign_object, equity_record_date, ex_dividend_date, pay_cash_date, gmdecision_notice_date, annual_general_meeting_date, info_code, total_dividend, total_dividend_a having row_count > 1)"
```

F10 字段覆盖校验：

```bash
cd pipeline
uv run python elt/scripts/validate_f10_passthrough_coverage.py
```

controller 和 Dagster 验证：

```bash
cd pipeline
uv run ruff check scheduler/src/scheduler/defs/automation/source_to_marts_backfill.py scheduler/tests/unit/automation/test_source_to_marts_backfill.py scheduler/tests/unit/daily/test_source_to_marts.py
uv run pytest scheduler/tests/unit/automation/test_source_to_marts_backfill.py scheduler/tests/unit/daily/test_source_to_marts.py
cd scheduler
uv run dg check defs
```

文档和格式验证：

```bash
make docs-check
git diff --check
```

## 完成标准

1. RFC 0048 列出的 1 个 THS intermediate、1 个 THS mart、1 个 ChinaBond mart、9 个 F10 intermediate 和 9 个 F10 mart 全部存在。
2. 新增 SQL/YAML 均通过 dbt parse 和定向 build。
3. F10 下游模型名和文件名不含 `_eastmoney`。
4. F10 9 条链路通过字段完整性核验，宽表字段没有裁剪。
5. `int_stock_dividend_plan` 的 `dividend_plan_record_key` 唯一性测试通过，历史版本不被静默合并为单条正确记录。
6. 新增字段覆盖校验脚本通过，并纳入本计划验收命令。
7. source-to-marts controller scope 和 daily registry 测试通过。
8. `dg check defs` 通过，新增 dbt asset 能被 Dagster definitions 加载。
9. 文档检查和 whitespace 检查通过。
