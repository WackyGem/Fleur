# Plan 0062: BaoStock dbt 下游存量作业性能优化实施计划

日期：2026-06-29

状态：Completed

## 背景

[RFC 0036](../../RFC/0036-dbt-baostock-downstream-performance-optimization.md) 已经明确 BaoStock 下游优化的首期方向：不把 int 层改为复杂的分区感知增量，不引入 `_sync_at` 水位线状态机，而是先删除低价值 mart 字段匹配测试，并把 raw ClickHouse 最新 `year` 分区刷新后的下游动作收敛为相关 int/mart 全量重建。

当前性能基线来自 [2026-06-26 dbt BaoStock 下游性能报告](../../jobs/reports/2026-06-26-dbt-baostock-downstream-performance.md)：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+
```

该 full downstream build 总耗时 `374.86s`，其中：

| 类别 | 耗时 | 主要对象 |
| --- | ---: | --- |
| 表模型构建 | `205.89s` | `mart_stock_quotes_daily`, `int_stock_quotes_daily_adj`, `int_stock_quotes_daily_unadj`, `int_stock_adjustment_factor` |
| 数据测试 | `168.06s` | mart 字段逐列匹配、ASOF 和 key coverage 回归测试 |

前三个 mart 字段逐列匹配测试合计 `131.27s`，是首期最直接的可删除成本。

## 目标

- 删除 3 个低价值 mart 字段逐列匹配 singular tests。
- 保留 `mart_stock_quotes_daily` 的 key coverage、key set、唯一性、not null 和 security code format 等基础门禁。
- 将存量日常作业从“全 dbt model group + calculation”收敛到 BaoStock 股票行情链路所需的固定 selector 和必要 calculation 资产。
- raw ClickHouse 最新 `year` 分区刷新成功后，触发相关 int/mart 全量重建，不使用 `_sync_at` 或 raw sync 状态表做局部选择。
- 保留现有 full build 能力作为手动完整验证路径。
- 对 `mart_stock_quotes_daily` 建立 SQL/query log 基准；只有收益明确时才进入 SQL 改写。
- 输出至少一份优化后运行报告，对比 RFC 0036 的 `374.86s` baseline。

## 非目标

- 不把 `int_stock_quotes_daily_unadj`、`int_stock_adjustment_factor` 或 `int_stock_quotes_daily_adj` 改为 dbt incremental。
- 不改变 raw ClickHouse sync 的按年分区替换语义。
- 不新增 `_sync_at` 水位线、raw sync 变更状态表或 dbt vars 增量窗口。
- 不改变 `mart_stock_quotes_daily` 字段契约。
- 不删除 key coverage、key set、唯一性、not null、security code format 等基础质量门禁。
- 不新增 current-year/window/full 三套长期 dbt schedule。

## 存量作业盘点

| 作业或命令 | 当前用途 | 优化动作 |
| --- | --- | --- |
| `clickhouse__raw_sync_baostock_job` | 将 BaoStock compacted year 分区同步到 ClickHouse raw。 | 保持 raw year 分区刷新语义；成功后触发瘦身后的股票日常下游作业。 |
| `dbt__staging_build_job` | 构建 dbt staging group。 | 保留，不作为 BaoStock 股票日常链路的主要性能优化对象。 |
| `dbt__marts_build_job` | 当前覆盖 `dbt_staging`、`dbt_intermediate`、`dbt_marts`。 | 保留为手动完整验证或发版验证入口；不用于 latest year 日常路径。 |
| `stock__daily_build_job` / `stock__daily_build_schedule` | 当前 selection 为 `dbt_staging + dbt_intermediate + dbt_marts + calculation`。 | 收敛为股票行情链路固定 selector 和必要 calculation 资产，避免每天重建所有 dbt staging/intermediate/mart。 |
| 手动 dbt full downstream build | `stg_baostock__query_history_k_data_plus_daily+` 全链路验证。 | 保留为历史修复、模型语义变更和发版前验证命令。 |

目标日常链路：

```text
clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted year=<latest_year> refresh
  -> dbt build stock quote int chain
  -> Furnace/calculation append-latest or replace-cascade, using existing asset config
  -> dbt build mart_stock_quotes_daily
  -> keep key and basic tests
```

dbt 固定 selector：

```text
int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_quotes_daily
```

注意：`mart_stock_quotes_daily` 读取 `int_stock_kdj_daily` wrapper。实施时必须先确认 Dagster asset graph 中 calculation 资产与 dbt wrapper/mart 的依赖顺序；如果现有 graph 不能保证 KDJ 先于 mart，则必须把日常作业拆成有序步骤或补充显式依赖，不能依赖调度偶然顺序。

## 预期执行依赖图

优化后的日常链路按“raw latest year 刷新成功后，触发固定下游重建”组织。raw 层仍按 `year` 分区刷新；int/mart 层首期仍做全量 table rebuild，不做 `_sync_at` 或 changed partition 推断。

```text
Legend
======

[R] rebuilt/materialized in the optimized daily chain
[V] dbt view read at query time
[P] pre-existing prerequisite, not rebuilt by the P0 daily selector
[O] outside the P0 daily selector; keep for manual/full validation


Dagster source/raw chain
========================

[P] source/baostock__query_stock_basic
        |
        v
[P] source/baostock__query_history_k_data_plus_daily
        |
        v
[P] source/baostock__query_history_k_data_plus_daily_compacted
        |
        v
[R] clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted
    partition: year=<latest_year>
        |
        | raw sync success triggers fixed downstream chain
        v


dbt quote rebuild chain
=======================

[V] stg_baostock__query_history_k_data_plus_daily
        |
        |                 side inputs read by int_stock_quotes_daily_unadj
        |                 -----------------------------------------------
        |                 [P] int_stock_basic_snapshot
        |                 [P] int_trade_calendar
        |                 [P] int_stock_shares_history
        |                 [P] int_stock_exrights_event
        |                              |
        +------------------------------+
        |
        v
[R] int_stock_quotes_daily_unadj
        |
        v
[R] int_stock_adjustment_factor
        |
        v
[R] int_stock_quotes_daily_adj
        |
        v


calculation / wrapper chain
===========================

                                      +--> [R] fleur_calculation.calc_stock_kdj_daily
                                      |        |
                                      |        v
[R] int_stock_quotes_daily_adj -------+    [V] int_stock_kdj_daily
                                      |        |
[R] int_stock_quotes_daily_unadj -----+        +------------------+
                                      |                           |
                                      +--> [R] fleur_calculation.calc_stock_ma_daily
                                      |        |
                                      |        v
                                      |    [V] int_stock_ma_daily
                                      |
                                      +--> [R] fleur_calculation.calc_stock_rsi_daily
                                      |        |
                                      |        v
                                      |    [V] int_stock_rsi_daily
                                      |
                                      +--> [R] fleur_calculation.calc_stock_boll_daily
                                      |        |
                                      |        v
                                      |    [V] int_stock_boll_daily
                                      |
                                      +--> [R] fleur_calculation.calc_stock_macd_daily
                                      |        |
                                      |        v
                                      |    [V] int_stock_macd_daily
                                      |
                                      +--> [R] fleur_calculation.calc_stock_price_pattern_daily
                                               |
                                               v
                                           [V] int_stock_price_pattern_daily


mart rebuild chain
==================

[R] int_stock_quotes_daily_unadj --------+
                                         |
[R] int_stock_quotes_daily_adj ----------+
                                         |
[P] int_stock_financial_valuation -------+--> [R] mart_stock_quotes_daily
                                         |          |
[V] int_stock_kdj_daily -----------------+          v
                                               retained quality gates


calculation outputs not rebuilt into mart in P0
===============================================

[V] int_stock_ma_daily ---------------------> [O] mart_stock_trend_indicator_daily
           |                                  [O] mart_stock_volume_indicator_daily
           |
[V] int_stock_boll_daily -------------------> [O] mart_stock_trend_indicator_daily
           |
[V] int_stock_macd_daily -------------------> [O] mart_stock_trend_indicator_daily

[V] int_stock_rsi_daily --------------------> [O] mart_stock_momentum_indicator_daily
[V] int_stock_kdj_daily --------------------> [O] mart_stock_momentum_indicator_daily

[V] int_stock_price_pattern_daily ----------> [O] mart_stock_price_pattern_daily

retained quality gates:
  - unique_combination_of_columns(security_code, trade_date)
  - not_null(security_code, trade_date)
  - cn_security_code_format(security_code)
  - mart_stock_quotes_daily_key_set_matches_quotes
  - mart_stock_quotes_daily_adjusted_key_coverage
```

`int_stock_financial_valuation` 在 P0 日常链路中按既有表读取，不随 BaoStock latest-year quote refresh 一起默认重建。若本次 raw 修复会影响历史 report-date 附近行情、股本或 EastMoney 财报输入，必须改走手动完整验证或另行加入 valuation affected-window 重建；不能把它混入 latest-year quote 日常路径后仍称为固定 P0 selector。

有序执行口径：

```text
1. raw latest year sync
   clickhouse__raw_sync_baostock_job(year=<latest_year>)

2. dbt quote int rebuild
   dbt build --select \
     int_stock_quotes_daily_unadj \
     int_stock_adjustment_factor \
     int_stock_quotes_daily_adj

3. calculation append/replace
   materialize existing calculation assets
   (KDJ, MA, RSI, BOLL, MACD, price pattern)
   using current stock_daily_run_config()

4. mart rebuild and retained tests
   dbt build --select mart_stock_quotes_daily
```

如果 Dagster asset graph 能稳定表达上述依赖顺序，Phase 2 可以把它收敛成一个瘦身后的 `stock__daily_build_job`。如果 graph 无法保证 calculation/KDJ 先于 `mart_stock_quotes_daily`，则必须保留上面的有序步骤，或补显式依赖后再合并为单个 job。

P0 日常链路虽然会物化全部现有 calculation 资产，但只把 `mart_stock_quotes_daily` 纳入 mart rebuild。`mart_stock_trend_indicator_daily`、`mart_stock_volume_indicator_daily`、`mart_stock_momentum_indicator_daily` 和 `mart_stock_price_pattern_daily` 继续保留在手动完整验证或后续专项优化范围内，避免本次优化又退回“全 dbt marts”日常构建。

## 设计约束

- raw `year` 是摄入和存储边界，不等于 int/mart affected business window。
- `_sync_at` 只能用于 raw 审计和排障，不进入 dbt selector、model SQL 或测试窗口变量。
- dbt 日常路径不做 changed-partition 推断；Dagster 在 raw sync 成功后触发确定的下游重建。
- `int_stock_adjustment_factor` 使用证券内未来窗口，`int_stock_quotes_daily_adj` 继承其全量重建要求。
- `int_stock_quotes_daily_unadj` 包含前一交易日、股本 ASOF、分红 ASOF 和 TTM 股息逻辑，不做首期分区增量。
- 所有 SQL 优化必须先有 before/after 基准；单项收益低于 `10%` 不实施。
- 任何作业 selection 收窄都必须保留现有基础测试门禁。

## 实施阶段

### Phase 0: 基线复核与作业图确认

目标：确认当前 Dagster/dbt asset graph 和测试节点，给后续优化提供可复现起点。

实施项：

1. 记录当前 git 状态和待处理变更，避免混入无关回滚。
2. 运行 dbt parse，保存 manifest 中以下节点是否存在：
   - `mart_stock_quotes_daily_quote_passthrough_matches`
   - `mart_stock_quotes_daily_adjusted_passthrough_matches`
   - `mart_stock_quotes_daily_financial_valuation_asof_matches`
   - `mart_stock_quotes_daily_key_set_matches_quotes`
   - `mart_stock_quotes_daily_adjusted_key_coverage`
3. 用 Dagster definitions 检查现有作业：
   - `clickhouse__raw_sync_baostock_job`
   - `dbt__marts_build_job`
   - `stock__daily_build_job`
   - `stock__daily_build_schedule`
4. 确认 dbt asset keys 与 model 名称的映射，尤其是：
   - `int_stock_quotes_daily_unadj`
   - `int_stock_adjustment_factor`
   - `int_stock_quotes_daily_adj`
   - `mart_stock_quotes_daily`
   - `int_stock_kdj_daily`
5. 确认 `mart_stock_quotes_daily` 与 calculation/KDJ 资产在 Dagster asset graph 中是否有稳定依赖顺序。

验证命令：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
cd scheduler
uv run dg check defs
```

完成标准：

- 现有测试节点、作业 selection 和 asset graph 事实已记录到运行报告或实施 notes。
- 能明确说明日常作业是否可以由单个 Dagster asset job 表达；若不能，后续 Phase 2 必须采用有序 job 或 runbook 步骤。

### Phase 1: 删除低价值 mart 字段匹配测试

目标：直接移除 `131.27s` 量级的低价值全表对比成本。

修改范围：

- `pipeline/elt/tests/marts/mart_stock_quotes_daily_quote_passthrough_matches.sql`
- `pipeline/elt/tests/marts/mart_stock_quotes_daily_adjusted_passthrough_matches.sql`
- `pipeline/elt/tests/marts/mart_stock_quotes_daily_financial_valuation_asof_matches.sql`
- 可选更新 `docs/issues/baostock-2026-06-26-downstream-performance.md`

实施项：

1. 删除 3 个 singular tests。
2. 运行 dbt parse，确认 manifest 不再包含这些 test nodes。
3. 保留以下测试：
   - `mart_stock_quotes_daily_key_set_matches_quotes`
   - `mart_stock_quotes_daily_adjusted_key_coverage`
   - `unique_combination_of_columns_mart_stock_quotes_daily_security_code__trade_date`
   - `not_null`
   - `cn_security_code_format`
4. 运行相关 dbt build，记录删除测试后的耗时。

验证命令：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_quotes_daily
```

完成标准：

- 被删除的 3 个 test nodes 不再出现在 manifest 中。
- 相关 int/mart build 通过。
- 新运行报告记录模型耗时、测试耗时和相对 `374.86s` baseline 的差异。

### Phase 2: 收敛存量日常作业 selection

目标：优化现有 `stock__daily_build_job` / `stock__daily_build_schedule`，避免日常路径触发全量 dbt group。

修改范围：

- `pipeline/scheduler/src/scheduler/defs/dbt_jobs.py`
- `pipeline/scheduler/tests/` 中 transformation job 或 definitions 相关测试
- 可选更新 Dagster runbook 文档

实施项：

1. 保留 `dbt__marts_build_job` 作为手动完整验证入口。
2. 将日常股票作业从 `DBT_MODEL_SELECTION | calculation` 收窄为：
   - BaoStock 股票行情 int 链路。
   - 现有 calculation 资产。
   - `mart_stock_quotes_daily`。
3. 不向 dbt 传 `_sync_at`、`validation_year` 或其他增量窗口变量。
4. 若 Dagster asset graph 能表达正确依赖顺序，则用一个瘦身后的 `stock__daily_build_job`。
5. 若不能稳定表达 KDJ/calculation 先于 mart，则改为明确的有序执行方案：
   - 先 materialize dbt int 链路。
   - 再 materialize calculation assets。
   - 最后 materialize `mart_stock_quotes_daily`。
6. 保持 `stock_daily_run_config()` 中 Furnace/calc asset 的现有业务参数；本阶段不调整指标公式、输入表或模式。
7. 添加测试或 definitions 检查，证明 daily job 不再选择全部 `dbt_staging`、全部 `dbt_intermediate` 和全部 `dbt_marts`。

验证命令：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

完成标准：

- `stock__daily_build_job` 不再等价于全 dbt model group build。
- `dbt__marts_build_job` 仍可用于完整手动验证。
- daily schedule 的 run config 只服务现有 calculation append-latest 语义，不携带 dbt 增量水位线。
- raw sync 成功后可触发瘦身后的日常下游作业。

### Phase 3: raw sync 后触发日常下游链路

目标：把 RFC 0036 的“raw latest year 成功后触发相关 int/mart 全量重建”落到存量 Dagster 作业关系中。

修改范围：

- `pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/dbt_jobs.py`
- 可选新增或调整 sensor/schedule/runbook
- `docs/skills/fleur-dagster-backfill-runbook/` 中相关命令示例

实施项：

1. 确认 `clickhouse__raw_sync_baostock_job` 使用最新 `year` 分区运行时的 partition key 来源。
2. 在 raw sync 成功后触发 Phase 2 的日常下游作业。
3. 触发链路只传递运行事实，不传递 `_sync_at` 或 affected window。
4. dbt 或 calculation 失败时，重跑同一 selector / asset selection。
5. 不做基于上次失败点的局部恢复。
6. 记录 run metadata：
   - raw sync run id。
   - raw partition key。
   - downstream job name。
   - dbt invocation id 或 Dagster run id。
   - 总耗时和失败原因。

验证命令：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

完成标准：

- raw sync 成功后存在明确的下游触发路径。
- raw sync 失败时不会触发 dbt/calc 下游。
- 下游失败后的恢复方式是重跑同一固定 selector 或同一 Dagster asset selection。
- 运行记录足够排查，但不作为模型选择条件。

### Phase 4: mart SQL 基准和低风险优化

目标：在删除低价值测试和作业收敛后，如果日常路径仍偏慢，再对重型 SQL 做有证据的优化。

修改范围：

- `pipeline/elt/models/marts/mart_stock_quotes_daily.sql`
- 可选涉及：
  - `pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`
  - `pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`
  - `pipeline/elt/models/intermediate/int_stock_adjustment_factor.sql`
- `docs/jobs/reports/`

实施项：

1. 对 `mart_stock_quotes_daily` 做 CTE 分段 `FORMAT Null` 基准：
   - quotes + adjusted join。
   - quotes + financial valuation ASOF。
   - quotes + KDJ join。
   - 完整 select。
2. 采集 ClickHouse query log：
   - `query_duration_ms`
   - `read_rows`
   - `read_bytes`
   - `memory_usage`
   - `ProfileEvents`
   - partition 和 primary key pruning 情况。
3. 若 KDJ join 是主耗时，评估 `LEFT ANY JOIN`，但必须先证明右表 `(security_code, trade_date)` 唯一。
4. 若 ASOF 是主耗时，先记录证据；不默认调整 `int_stock_financial_valuation` 排序键。
5. 若 join 成本不高但 table materialization 慢，单独评估 ClickHouse 写入、part 生成、列数和压缩成本。
6. 单项优化预计或实测收益低于 `10%` 时不实施。

验证命令：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt \
  --select mart_stock_quotes_daily
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_quotes_daily
```

完成标准：

- `docs/jobs/reports/` 有 mart SQL before/after 或 baseline-only 报告。
- 任何 SQL 改写都有 query log 或 `FORMAT Null` 证据。
- key coverage、key set、唯一性和基础测试通过。
- 如果没有明确收益，保留现有 SQL，不做低收益复杂化。

### Phase 5: 可选测试窗口化评估

目标：只有当 Phase 1-4 后仍无法满足日常窗口时，再评估窗口化测试。

候选对象：

- `mart_stock_quotes_daily_adjusted_key_coverage`
- `mart_stock_quotes_daily_key_set_matches_quotes`
- `int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date`

实施项：

1. 为候选 tests 设计可选 vars：
   - `validation_start_date`
   - `validation_end_date`
   - `validation_year`
2. 默认不传 vars 时必须仍为全历史验证。
3. key set 类测试必须对两侧 key 集合应用完全相同的日期谓词。
4. 窗口化先作为 runbook 命令，不新增长期 schedule。
5. 命令名、运行报告和操作说明必须明确 `window` 语义，避免误读为全表保证。

完成标准：

- 只有在 Phase 1-4 后仍超标时才进入本阶段。
- 窗口化不会替代发版、历史修复和模型 SQL 变更时的完整验证。

### Phase 6: 运行报告、issue 和文档收敛

目标：让优化结果可追溯、可复测，并关闭或更新原性能问题。

修改范围：

- `docs/jobs/reports/`
- `docs/issues/baostock-2026-06-26-downstream-performance.md`
- `docs/RFC/0036-dbt-baostock-downstream-performance-optimization.md`
- `docs/plans/README.md`

实施项：

1. 新增运行报告，至少包含：
   - 原 baseline：`374.86s`。
   - 删除字段匹配测试后的相关 int/mart build。
   - 日常瘦身作业的 Dagster run 结果。
   - 如果进入 Phase 4，补充 mart SQL query log 或 `FORMAT Null` 基准。
2. 更新 issue 状态：
   - 已完成动作。
   - 剩余风险。
   - 是否仍需 SQL 优化或窗口化。
3. 更新 runbook，给出：
   - raw sync 后日常下游作业命令。
   - 手动完整验证命令。
   - 失败后重跑策略。
4. 若后续要做 int incremental，另起 RFC，不在本计划内直接实施。

完成标准：

- 能从计划、RFC、issue 和运行报告串起完整决策链。
- 操作者知道日常路径、完整验证路径和失败恢复方式。
- 没有把 `_sync_at`、raw 状态表或 dbt vars 水位线写入首期作业语义。

## 验证命令

文档-only 变更：

```bash
make docs-check
git diff --check
```

dbt 变更：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_quotes_daily
```

Dagster/scheduler 变更：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

必要完整验证：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+
```

## 完成标准

- 3 个 mart 字段逐列匹配 tests 已删除，dbt manifest 中不再出现对应 test nodes。
- `stock__daily_build_job` 或等价日常链路不再触发全 dbt model group。
- raw ClickHouse 最新 `year` 分区刷新成功后，能触发相关 int/mart/calc 日常下游链路。
- 日常链路保留 key coverage、key set、unique、not null 和 security code format 等基础门禁。
- 日常链路不依赖 `_sync_at`、raw sync 变更状态表或 dbt vars 水位线。
- `dbt__marts_build_job` 或手动 dbt full downstream build 仍可用于完整验证。
- 至少一份运行报告证明优化前后耗时差异，并记录剩余瓶颈。
- 若实施 SQL 优化，必须有 before/after query log 或 `FORMAT Null` 基准，且基础测试通过。

## 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| 日常作业 selection 收窄后漏跑必要上游 | Phase 0 先确认 asset graph；Phase 2 通过 definitions 检查和相关 dbt build 验证。 |
| mart 依赖 KDJ wrapper，但 calculation 未先更新 | 实施前确认 Dagster 依赖顺序；不能保证时改为有序步骤或补显式依赖。 |
| 字段错接不再由全表逐列测试发现 | 保留 key coverage、key set、唯一性、基础测试；SQL 改动走 code review 和抽样验证。 |
| raw sync 成功后下游失败导致数据不一致 | 恢复策略固定为重跑同一 selector / asset selection；运行报告记录 raw run id 和 downstream run id。 |
| SQL 优化引入语义漂移 | 任何 SQL 改动必须通过 before/after 基准、基础测试和抽样验证。 |
| 后续又把 `_sync_at` 用作业务水位线 | 本计划明确 `_sync_at` 只做 raw 审计；int 增量必须另起 RFC 定义 affected business window。 |
