# RFC 0038: BaoStock dbt 下游构建性能分层优化

状态：Proposed
日期：2026-06-28
领域：dbt, ClickHouse, BaoStock
关联系统：pipeline/elt, pipeline/scheduler, fleur_intermediate, fleur_marts
性能记录：docs/issues/baostock-2026-06-26-downstream-performance.md
实测报告：docs/jobs/reports/2026-06-26-dbt-baostock-downstream-performance.md
优化报告：docs/jobs/reports/2026-06-29-dbt-baostock-downstream-performance-optimization.md

## 摘要

2026-06-26 的 BaoStock 下游 dbt 构建基线显示：

```text
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+
```

总耗时 `374.86s`，覆盖约 `18.08M` 行股票日行情 int/mart 表。耗时主要集中在两类工作：

| 类别 | 耗时 | 占比 | 主要对象 |
| --- | ---: | ---: | --- |
| 表模型构建 | `205.89s` | `54.9%` | `mart_stock_quotes_daily`, `int_stock_quotes_daily_adj`, `int_stock_quotes_daily_unadj`, `int_stock_adjustment_factor` |
| 数据测试 | `168.06s` | `44.8%` | mart 宽表字段匹配、ASOF 和 key coverage 回归测试 |

本 RFC 的核心判断是：**首期不把 int 层改为复杂的分区感知增量构建**。当前 int 层全量重建虽然不是最短路径，但语义稳定、验证简单，且重建耗时仍在几十秒级。真正值得优先优化的是：

1. 删除低价值、高成本的 mart 字段逐列匹配测试。
2. 在 raw ClickHouse 最新 year 分区刷新后，触发相关 int/mart 全量重建。
3. 日常路径只保留 key coverage、key set、唯一性、not null、security code format 等基础质量门禁。
4. 将 int/mart SQL 基准与优化作为后续主要性能抓手。
5. 将窗口化测试、当前年验证和多 job profile 降级为后续可选项，不作为首期调度复杂度。

目标是在保留基础质量门禁和 key coverage 的前提下，把 BaoStock 下游构建从约 `6.2min` 降到可接受的日常窗口；完整验证不再包含 mart 字段逐列匹配类测试，也不引入多个长期调度 job。

## 当前事实

### 数据规模

| 表 | 行数 | 日期范围 |
| --- | ---: | --- |
| `fleur_raw.baostock__query_history_k_data_plus_daily_compacted` | `20,292,499` | n/a |
| `fleur_staging.stg_baostock__query_history_k_data_plus_daily` | `20,292,499` | `1990-12-19..2026-06-25` |
| `fleur_intermediate.int_stock_quotes_daily_unadj` | `18,079,273` | `1995-01-03..2026-06-25` |
| `fleur_intermediate.int_stock_adjustment_factor` | `18,079,273` | n/a |
| `fleur_intermediate.int_stock_quotes_daily_adj` | `18,079,273` | `1995-01-03..2026-06-25` |
| `fleur_marts.mart_stock_quotes_daily` | `18,079,273` | `1995-01-03..2026-06-25` |

### 最重模型

| 模型 | 耗时 | 判断 |
| --- | ---: | --- |
| `mart_stock_quotes_daily` | `119.31s` | 第一优先级模型优化对象；单点占总耗时 `31.8%`。 |
| `int_stock_quotes_daily_adj` | `35.83s` | 依赖全量 quotes 和 adjustment factor，受复权链路语义约束。 |
| `int_stock_quotes_daily_unadj` | `28.76s` | 首个重型 enrichment 表，仍可接受。 |
| `int_stock_adjustment_factor` | `12.68s` | 跨证券内全历史窗口计算，不适合首期局部分区重建。 |
| `int_stock_financial_valuation` | `6.84s` | 不是当前主要瓶颈。 |

### 最重测试

| 测试 | 耗时 | 判断 |
| --- | ---: | --- |
| `mart_stock_quotes_daily_quote_passthrough_matches` | `63.64s` | 首期最高收益对象。 |
| `mart_stock_quotes_daily_adjusted_passthrough_matches` | `37.10s` | 首期最高收益对象。 |
| `mart_stock_quotes_daily_financial_valuation_asof_matches` | `30.53s` | 首期最高收益对象。 |
| `mart_stock_quotes_daily_adjusted_key_coverage` | `9.27s` | 可窗口化。 |
| `mart_stock_quotes_daily_key_set_matches_quotes` | `8.80s` | 可窗口化。 |

前三个字段逐列匹配类测试合计 `131.27s`，约占整次构建 `35.0%`。它们主要证明 mart 字段与 upstream int 字段逐列一致，能抓字段错接，但对当前宽表的工程收益低于运行成本。本 RFC 决定删除这 3 个测试。`key_coverage` 和 `key_set_matches_quotes` 仍保留，并作为后续窗口化对象。

## 设计判断

本 RFC 按“先减少无效工作，再稳定触发相关重建，最后才讨论局部增量”的顺序决策。核心边界如下：

| 边界 | 结论 |
| --- | --- |
| raw 年分区 | 是摄入和存储边界，适合 raw ClickHouse `year` 分区替换。 |
| int/mart 输出分区 | 是业务日期边界，可能是 `trade_date`、`report_date`、`effective_date` 或 `ex_dividend_date`，不总是等于 raw 输入 `year`。 |
| Dagster | 负责知道 raw sync 是否成功，并在成功后触发固定 dbt selector。 |
| dbt | 负责按已选择的模型图执行转换和测试，不负责推断 raw 哪些分区变化。 |
| `_sync_at` | 可作为摄入审计和排障字段，但不作为 int 层增量水位线。它只能说明数据何时同步，不能说明哪些业务日期、证券和下游窗口需要重算。 |

因此，首期设计不把 raw `year` 或 `_sync_at` 直接传导为 int/mart 增量范围。历史数据今天被重刷时，`_sync_at` 会落在今天，但受影响的输出可能是历史 `trade_date`、`report_date` 或其后的 ASOF/window 区间；分红、股本、估值和复权链路还会产生证券内级联影响。用 `_sync_at` 驱动 int 层局部替换会把“摄入时间”误当成“业务影响范围”。

### 首期优化

1. **删除低价值 mart 字段匹配测试**

   `quote_passthrough_matches`、`adjusted_passthrough_matches` 和 `financial_valuation_asof_matches` 主要做字段逐列匹配。它们能捕捉 mart 字段错接，但代价是每次全历史扫描或重跑 ASOF，对日常和完整验证都不再保留。

2. **raw latest year 刷新后的相关 int/mart 全量重建**

   raw ClickHouse 最新 year 分区替换成功后，由 Dagster 显式触发相关 dbt 模型全量重建。当前 18M 行规模下，股票行情链路的 int 全量重建仍比引入分区状态机更稳。

3. **保留 key 类和基础质量测试**

   `key_set_matches_quotes` 和 `adjusted_key_coverage` 仍承担宽表 key 覆盖和复权因子非空门禁。日常路径保留这些测试及 `unique`、`not_null`、security code format 等基础门禁。

4. **把 SQL 构建路径作为主要性能抓手**

   选择全量重建后，int/mart SQL 本身成为主要优化对象。优化必须先做分段基准，再按耗时和风险排序实施。首要对象是 `mart_stock_quotes_daily`，其次是 `int_stock_quotes_daily_adj` 和 `int_stock_quotes_daily_unadj`。

### 延后优化

1. **测试窗口化**

   `unique_combination_of_columns_*` 单项耗时不高，不进入首期。若相关 int/mart 全量重建仍无法满足 SLA，再统一评估剩余 key custom tests 和 generic tests 的窗口化。

2. **int 层分区感知增量**

   raw S3 parquet 和 raw ClickHouse 层按年分区替换是正确的，但 int 层并不天然等价于“只重建同一年”。`int_stock_adjustment_factor` 使用证券内全历史和未来窗口计算复权因子；`int_stock_quotes_daily_unadj` 还包含前一交易日、股本 ASOF、分红事件 ASOF 和累计分红逻辑。若按 raw year 直接局部替换，需要维护 affected securities、affected years 和级联验证。

3. **raw sync 变更状态表驱动 dbt**

   该方案能做，但会把 Dagster raw sync、dbt vars、int/mart incremental、测试窗口和状态恢复绑在一起。当前性能瓶颈还没有大到需要这套复杂状态机。

## 目标

- 删除 mart 字段逐列匹配类测试，降低全量和日常构建成本。
- raw ClickHouse 最新 year 分区刷新后，触发相关 int/mart 全量重建。
- 日常路径保留 key coverage、key set、唯一性、not null 和 security code format 等基础质量测试。
- 保留手动完整验证命令，用于历史修复、发版和模型语义变更，但不恢复已删除的字段逐列匹配类测试。
- 首期不改变 int 层业务语义和物化方式。
- 明确 `_sync_at` 只承担摄入审计/排障语义，不承担 int affected window 识别。
- 对重型 int/mart SQL 建立可重复的优化基准，避免无基准改 SQL。
- 不新增多个长期调度 dbt job，避免调度面复杂化。

## 非目标

- 不在首期把 `int_stock_quotes_daily_unadj`、`int_stock_adjustment_factor` 或 `int_stock_quotes_daily_adj` 改为 incremental。
- 不改变 raw ClickHouse sync 的按年分区替换语义。
- 不把 `_sync_at` 作为 int/mart 增量选择条件。
- 不在首期新增 raw sync 变更状态表、dbt vars 水位线或跨系统状态恢复机制。
- 不改变 `mart_stock_quotes_daily` 的字段契约。
- 不移除 key coverage、key set、唯一性、not null、security code format 等基础质量门禁。
- 不为了性能放宽基础质量门禁。
- 不在首期新增 current-year/window/full 三套长期调度 job。

## 方案

方案分为三层，后一层只有在前一层复测后仍不能满足日常窗口时才进入：

| 层级 | 定位 | 是否首期必做 |
| --- | --- | --- |
| P0 | 删除低价值测试，并把 raw latest year 成功刷新后的下游动作收敛为固定 selector 的 int/mart 全量重建。 | 是 |
| P1 | 对重型 SQL 和保留测试做可度量优化，先基准后改动。 | 视 P0 复测结果推进 |
| P2 | 重新设计测试窗口化或 int 分区增量，需要明确 affected window 和恢复语义。 | 否 |

### P0：删除 mart 字段逐列匹配测试

删除以下 singular tests：

| 测试 | 耗时 | 删除理由 |
| --- | ---: | --- |
| `mart_stock_quotes_daily_quote_passthrough_matches` | `63.64s` | 逐列证明 mart 未复权字段等于 int 字段，收益低于全表 join 成本。 |
| `mart_stock_quotes_daily_adjusted_passthrough_matches` | `37.10s` | 逐列证明 mart 复权字段等于 int 字段，收益低于全表 join 成本。 |
| `mart_stock_quotes_daily_financial_valuation_asof_matches` | `30.53s` | 重跑 ASOF 逻辑验证 mart 估值字段，成本高且与模型 SQL 形态强绑定。 |

保留以下门禁：

| 测试 | 保留理由 |
| --- | --- |
| `mart_stock_quotes_daily_key_set_matches_quotes` | 防止 mart 相对主事实 quotes 漏行或多行。 |
| `mart_stock_quotes_daily_adjusted_key_coverage` | 防止 mart key 无对应 adjusted row，并检查复权因子缺失。 |
| `unique_combination_of_columns_mart_stock_quotes_daily_security_code__trade_date` | 保证 mart grain。 |
| `not_null` / `cn_security_code_format` | 保证基础字段质量。 |

删除字段匹配测试后，字段错接风险主要通过以下方式控制：

- mart SQL code review。
- mart 字段描述和上游来源文档。
- key coverage、唯一性和基础字段测试。
- 下游业务查询或发版前抽样验证。

### P0：raw latest year 刷新后触发相关 int/mart 全量重建

dbt 不提供自动识别 raw ClickHouse 哪个 year 分区变化并驱动 int 表局部刷新的机制。当前项目采用更简单的首期路径：

```text
clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted year=<latest_year> refresh
  -> trigger dbt build for related int/mart models
  -> rebuild related int/mart tables fully
  -> keep key and basic quality tests
```

推荐日常命令：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt \
  --select int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_quotes_daily
```

这条路径不要求 dbt 判断 raw 数据变化，只要求 Dagster 在 raw sync 成功后触发确定的下游重建。它牺牲了局部重建的最短运行时间，换取调度简单、语义清晰和可验证性。

职责边界：

| 环节 | 责任方 | 行为 | 失败恢复 |
| --- | --- | --- | --- |
| raw sync | Dagster + ClickHouse | 刷新 raw ClickHouse 最新 `year` 分区。 | raw 分区替换失败时不触发 dbt。 |
| 下游触发 | Dagster | raw 分区替换成功后，调用固定 dbt selector。 | dbt 失败时重跑同一 selector。 |
| 转换和测试 | dbt | 全量重建被选中的 int/mart table，并运行附着在这些模型上的基础/key 测试。 | 不读取 raw sync 状态表，不依赖 `_sync_at` 恢复。 |
| 运行记录 | Dagster + dbt artifact | 记录 raw sync 结果、dbt invocation 和测试结果。 | 通过运行记录排障，不把运行记录作为模型选择条件。 |

`_sync_at` 可以保留为 raw 审计字段，例如排查某批数据何时进入 ClickHouse；但它不进入本 RFC 的 dbt selector、model SQL 或测试窗口变量。需要局部重建时，必须由模型的业务日期和依赖传播规则推导 affected output range。

保留手动完整验证命令：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+
```

完整验证用于历史修复、模型 SQL 改动、字段契约变更、发版或必要巡检。它保留基础测试和 key 类 custom tests 的全历史模式，但不恢复已删除的字段逐列匹配类测试。

### P1：int/mart SQL 基准与优化路线

选择“raw latest year 刷新后触发相关 int/mart 全量重建”后，SQL 优化变成主要性能方向。所有 SQL 改动必须先有基准，不允许凭直觉重写。

优化优先级：

| 优先级 | 对象 | 基线耗时 | 优化判断 |
| --- | --- | ---: | --- |
| P1.1 | `mart_stock_quotes_daily` | `119.31s` | 最大单点成本，优先定位 join、ASOF、KDJ wrapper 和宽表写入成本。 |
| P1.2 | `int_stock_quotes_daily_adj` | `35.83s` | 全量 join unadj 与 adjustment factor，重点看 join key、排序键和读取列。 |
| P1.3 | `int_stock_quotes_daily_unadj` | `28.76s` | enrichment 逻辑复杂，优化必须保持前一交易日、ASOF、分红累计和市值语义清晰。 |
| P1.4 | `int_stock_adjustment_factor` | `12.68s` | 成本相对低且窗口语义强，暂不作为首轮 SQL 改写重点。 |

`mart_stock_quotes_daily` 是第一批基准对象。首期不直接改字段契约，而是做可回滚实验：

1. **分段 FORMAT Null 基准**

   对模型中的关键 CTE 分别运行 `FORMAT Null` 或 scratch SQL：

   - quotes + adjusted join
   - quotes + financial valuation ASOF
   - quotes + KDJ join
   - 完整 select

   记录 `elapsed`, `read_rows`, `read_bytes`, `memory_usage`。

2. **ClickHouse query log 基准**

   对 dbt build 运行中的目标 SQL 采集 query log，记录：

   - `query_duration_ms`
   - `read_rows`
   - `read_bytes`
   - `memory_usage`
   - `ProfileEvents`
   - 是否命中 partition 和 primary key pruning

   查询基准必须保存在 `docs/jobs/reports/`，后续 SQL 改动用同一口径对比。

3. **KDJ join 形态评估**

   `int_stock_kdj_daily` 已有 `(security_code, trade_date)` 唯一性测试。若基准显示 KDJ join 成本明显，可评估 `LEFT ANY JOIN` 是否与当前结果一致，并用测试证明 key 唯一性仍成立。

4. **ASOF join 验证**

   `int_stock_financial_valuation` 当前已按 `(security_code, report_date)` 排序，符合 mart ASOF 访问形态。首期只做基准，不默认调整排序键。若 ASOF 是主耗时，再评估是否需要 validity interval 表或辅助日频展开表。

5. **宽表写入成本评估**

   如果 join 分段查询成本不高，但 dbt table materialization 仍慢，应单独评估 ClickHouse 写入、part 生成、列数和压缩成本。此时优先考虑物理表设置或字段瘦身评估，而不是继续改 join。

验收要求：

- 任何 SQL 改动前必须有基准记录。
- 任何 join 形态变化必须通过 key coverage、key set、唯一性和人工抽样验证。
- 若单次优化收益低于 `10%`，不进入实施，避免低收益复杂化。

### P1：可选窗口化和 runbook 命令

只有当“删除字段匹配测试 + 相关 int/mart 全量重建 + SQL 基准优化”仍无法满足日常 SLA 时，再为保留的重型 tests 增加窗口变量。窗口化优先作为 runbook 命令，不先新增长期调度 job。

可选变量：

| 变量 | 示例 | 语义 |
| --- | --- | --- |
| `validation_start_date` | `"2026-06-01"` | 只验证 `trade_date >= validation_start_date`。 |
| `validation_end_date` | `"2026-06-25"` | 可选上界，配合 start date 使用。 |
| `validation_year` | `2026` | 只验证 `toYear(trade_date) = validation_year`。 |

首批可选窗口化对象：

- `mart_stock_quotes_daily_adjusted_key_coverage`
- `mart_stock_quotes_daily_key_set_matches_quotes`
- `int_stock_quotes_daily_unadj_prev_volume_matches_previous_trade_date`

约束：

- 未传 vars 时必须仍为全历史。
- key set 类测试必须对两侧 key 集合使用相同窗口。
- current-year/window/full 三类模式先作为 runbook 命令存在，不作为首期 Dagster schedule。

### P2：慢速 generic tests 的可选窗口版本

`unique_combination_of_columns_*` 单项耗时在 `1-3s` 之间，不是首要瓶颈；若 P1 窗口化 runbook 后续落地，再评估是否提供窗口参数。

候选对象：

- `unique_combination_of_columns_stg_baostock__query_history_k_data_plus_daily_security_code__trade_date`
- `unique_combination_of_columns_int_stock_quotes_daily_unadj_security_code__trade_date`
- `unique_combination_of_columns_int_stock_adjustment_factor_security_code__trade_date`
- `unique_combination_of_columns_int_stock_quotes_daily_adj_security_code__trade_date`
- `unique_combination_of_columns_mart_stock_quotes_daily_security_code__trade_date`

约束：

- 默认仍为全历史唯一性测试。
- 日常窗口唯一性不能替代夜间 full uniqueness。
- 若只验证窗口内唯一性，测试名称或 job 名称必须体现 `window`，避免误读为全表保证。

### P2：int 层增量适配盘点

只有在以下条件成立时，才进入 int incremental 设计：

- 日常全量 int/mart 构建加窗口测试仍超过目标 SLA。
- 数据量增长到当前数量级数倍，且 ClickHouse full rebuild 成本明显上升。
- 已有 full validation job 能稳定捕捉分区增量错误。
- 能明确列出每个模型的 affected date/year 传播规则。

本盘点基于当前 `pipeline/elt/models/intermediate/*.sql` 的实际 SQL 依赖和窗口语义。判定口径如下：

| 结论 | 含义 |
| --- | --- |
| 适合分区增量 | 当前输出分区只依赖同分区或静态输入，可优先评估 dbt `incremental_strategy='insert_overwrite'`。 |
| 需窗口/级联 | 技术上可做增量，但必须先由 Dagster/dbt 明确 affected date/year/security 传播规则。 |
| 全量重建 | 当前 SQL 不具备分区局部重建的语义边界，或模型很小且全量替换更简单。 |
| 不适用 | dbt 中只是 view/ephemeral wrapper，增量语义属于上游 calculation/portfolio 资产。 |

增量判定必须以输出表的业务分区字段为准，而不是以 raw `_sync_at` 为准：

| 输出字段类型 | 代表模型 | 增量含义 |
| --- | --- | --- |
| `trade_date` | 日行情、指数、基准、利率 | 需要证明输入变化只影响同一交易日期或明确的后续交易日窗口。 |
| `ex_dividend_date` | 除权除息事件 | 需要从 staging 结果推导受影响的除权日期年份。 |
| `effective_date` | 股本历史 | 需要按证券传播到相邻 effective interval。 |
| `report_date` | 财务估值 | 需要按证券传播到 TTM、MRQ 和 ASOF 相关报告期。 |

| 模型 | 当前物化 | 结论 | 依据与最小安全条件 |
| --- | --- | --- | --- |
| `int_government_bond_yields_daily` | table, `toYear(trade_date)` | 适合分区增量 | 直接读取 `stg_chinabond__government_bond` 并按 `trade_date` 输出；当输入刷新年份等于输出年份时，可按 year `insert_overwrite`。 |
| `int_index_quotes_daily` | table, `toYear(trade_date)` | 适合分区增量 | 从 BaoStock 日行情筛选指数并计算单行日收益；日行情 year 分区可映射到输出 year。若 `int_index_basic_snapshot` 变化，应全量重建。 |
| `int_benchmark_returns_daily` | table, `toYear(trade_date)` | 适合分区增量 | 只连接静态 benchmark universe 与 `int_index_quotes_daily`；若 benchmark 列表或指数基础快照不变，可跟随指数行情 year 增量。 |
| `int_stock_exrights_event` | table, `toYear(ex_dividend_date)` | 适合分区增量 | 对分红配股源按 `(security_code, ex_dividend_date)` 聚合，无窗口和 ASOF；但 raw 输入 year 未必等于 `ex_dividend_date` year，必须从 staging 结果推导 affected output years。 |
| `int_stock_quotes_daily_unadj` | table, `toYear(trade_date)` | 需窗口/级联 | 包含前一交易日自连接、股本 ASOF、分红事件 ASOF、现金分红累计和 TTM 逻辑；行情日变更会影响当日和下一交易日，股本/分红变更会影响区间。不能简单按 raw year 替换。 |
| `int_stock_shares_history` | table, `toYear(effective_date)` | 需窗口/级联 | 使用 equity/freeholder ASOF，并用 `leadInFrame` 生成 `expiry_date`；新变更点会影响同证券相邻区间，至少需要 security-scoped 或 affected-year 级联。 |
| `int_stock_financial_valuation` | table, `toYear(report_date)` | 需窗口/级联 | 依赖报价 ASOF、股本 ASOF、4 季度 TTM 窗口、MRQ balance 和年初 balance；输入修正会传播到后续 report periods。 |
| `int_risk_free_rate_daily` | table, `toYear(trade_date)` | 需窗口/级联 | 从国债收益率向交易日网格 forward fill；某个 source date 变更会影响它到下一条 source date 之前的所有交易日，可能跨年。 |
| `int_stock_adjustment_factor` | table, `toYear(trade_date)` | 全量重建 | 后复权因子使用 `unbounded preceding`，前复权因子使用 `1 following and unbounded following`；新增未来行情会改变同证券历史前复权因子。 |
| `int_stock_quotes_daily_adj` | table, `toYear(trade_date)` | 全量重建 | 直接依赖 `int_stock_adjustment_factor`；在复权因子没有安全 affected window 前，继承全量重建要求。 |
| `int_trade_calendar` | table | 全量重建 | 使用全表 `lagInFrame` 生成前一交易日；日历数据量小，且历史补日会影响下一交易日，保留全量最简单。 |
| `int_stock_basic_snapshot` | table | 全量重建 | 当前股票基础信息快照，无时间分区；全量替换比维护局部状态更可靠。 |
| `int_index_basic_snapshot` | table | 全量重建 | 当前指数基础信息快照，无时间分区；全量替换。 |
| `int_benchmark_basic_snapshot` | table | 全量重建 | 静态 benchmark universe join 当前指数基础快照；输入小，保留全量替换。 |
| `int_stock_kdj_daily` | view | 不适用 | dbt 仅包装 `fleur_calculation.calc_stock_kdj_daily`；增量由 Furnace/calculation 资产的 `append-latest` 或 `replace-cascade` 语义负责。 |
| `int_stock_ma_daily` | view | 不适用 | dbt 仅包装 `fleur_calculation.calc_stock_ma_daily`；增量由 calculation 资产负责。 |
| `int_stock_rsi_daily` | view | 不适用 | dbt 仅包装 `fleur_calculation.calc_stock_rsi_daily`；增量由 calculation 资产负责。 |
| `int_stock_boll_daily` | view | 不适用 | dbt 仅包装 `fleur_calculation.calc_stock_boll_daily`；增量由 calculation 资产负责。 |
| `int_stock_macd_daily` | view | 不适用 | dbt 仅包装 `fleur_calculation.calc_stock_macd_daily`；增量由 calculation 资产负责。 |
| `int_stock_price_pattern_daily` | view | 不适用 | dbt 仅包装 `fleur_calculation.calc_stock_price_pattern_daily`；增量由 calculation 资产负责。 |
| `int_portfolio_closed_trade` | view | 不适用 | dbt 仅连接 portfolio run snapshot 与 calculation result；存储和增量由上游 portfolio/calculation 表负责。 |
| `int_portfolio_trade_metric` | view | 不适用 | dbt 仅连接 portfolio run snapshot 与 calculation result；存储和增量由上游 portfolio/calculation 表负责。 |
| `int_portfolio_performance_metric` | view | 不适用 | dbt 仅连接 portfolio run snapshot 与 calculation result；存储和增量由上游 portfolio/calculation 表负责。 |
| `int_portfolio_performance_metric_status` | view | 不适用 | dbt 仅连接 portfolio run snapshot 与 calculation result；存储和增量由上游 portfolio/calculation 表负责。 |
| `int_portfolio_performance_metric_rank_catalog` | ephemeral | 不适用 | 静态 rank catalog，不落 ClickHouse 物理表。 |

若后续重开 int 增量，优先级应从“适合分区增量”组开始；`int_stock_adjustment_factor`、`int_stock_quotes_daily_adj` 和 `int_stock_quotes_daily_unadj` 不应作为第一批试点。`mart_stock_quotes_daily` 只有在其上游 int 表已经具备稳定 affected window 后，才评估按 year `insert_overwrite`。

## 实施顺序

### 阶段 1：删除低价值字段匹配测试

1. 删除 3 个 mart 字段逐列匹配 singular tests。
2. 运行 dbt parse，确认 manifest 不再包含这些 tests。
3. 记录删除后的 dbt build 基线。

### 阶段 2：raw latest year 后触发相关 int/mart 全量重建

1. 在 raw ClickHouse 最新 year 分区刷新成功后触发相关 dbt build。
2. 首期 selector 固定为：

```text
int_stock_quotes_daily_unadj int_stock_adjustment_factor int_stock_quotes_daily_adj mart_stock_quotes_daily
```

3. 保留 key coverage、key set、unique、not null、security code format 等基础测试。
4. 不向 dbt 传 `_sync_at`、`validation_year` 或其他增量窗口变量。
5. dbt 失败时重跑同一 selector；不做基于上次失败点的局部恢复。
6. 不新增 current-year/window/full 三套 schedule。

### 阶段 3：int/mart SQL 基准与优化

1. 先对 `mart_stock_quotes_daily` 做 CTE 分段基准和 query log 基准。
2. 如果 mart 基准显示明确瓶颈，再实施局部 SQL 改写。
3. 若 mart 优化后仍超标，再依次评估 `int_stock_quotes_daily_adj` 和 `int_stock_quotes_daily_unadj`。
4. 只对收益明确的 join 形态、读取列、排序键或写入设置创建后续 issue。
5. 若无明显收益，保留全量 rebuild 路径，不引入复杂调度。

### 阶段 4：复测与决策

1. 重跑删除字段匹配测试后的相关 int/mart build。
2. 对比 2026-06-26 baseline。
3. 若日常相关 int/mart 全量重建已满足 SLA，关闭 window/full job profile、`_sync_at` 水位线和 int incremental 首期讨论。
4. 若仍超标，先完成 P1 SQL 基准优化，再考虑测试窗口化；最后才进入 P2 int 增量 RFC。

## 预期收益

### 日常相关 int/mart 全量重建

最直接收益来自删除 3 个字段逐列匹配测试。理论上可直接移除 `131.27s` 历史基线成本，使完整路径从 `374.86s` 降到约 `243.59s` 量级。日常路径使用相关 int/mart 全量重建，不引入 changed-partition 状态机。

### SQL 优化收益

由于首期不做 int 分区增量，后续主要收益来自 SQL 路径。`mart_stock_quotes_daily` 是第一目标；如果它的 join 或写入路径能下降 `10%` 以上，应优先实施。若 mart 优化收益不足，再看 `int_stock_quotes_daily_adj` 和 `int_stock_quotes_daily_unadj`。

### 可选窗口化

若相关 int/mart 全量重建和 SQL 优化后仍不满足 SLA，再对剩余 key 类 custom tests 做窗口化，优先以 runbook 命令存在。

### 手动完整验证

完整验证不以字段逐列匹配为门禁，追求可复现、可比较、可作为发版和历史修复验收门禁。

## 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| 字段错接不再由全表逐列测试自动发现 | mart SQL code review、字段来源文档、key coverage、基础测试和发版前抽样验证共同覆盖。 |
| 相关 int/mart 全量重建仍偏慢 | 先复测删除字段匹配测试后的基线，再做 P1 SQL 基准优化；窗口化和增量放在后面。 |
| 日常路径没有 raw 分区级增量感知 | 由 Dagster raw sync 成功后显式触发相关 dbt build，不让 dbt 猜测变化范围。 |
| `_sync_at` 被误用为业务增量水位线 | RFC 明确 `_sync_at` 只做摄入审计；任何 int 增量必须按业务日期和模型依赖推导 affected range。 |
| 日常窗口测试漏掉历史 key 回归 | 若启用窗口化，发版、历史修复和必要巡检必须跑手动完整验证；模型 SQL 变更必须跑完整验证。 |
| key set 测试窗口化产生误报 | 两侧 key 集合必须应用完全相同的 trade_date 谓词。 |
| operator 误把窗口命令当全表保证 | 窗口化先作为 runbook 命令，命令名和报告必须记录 validation vars。 |
| mart SQL 优化改变宽表语义 | 所有 mart SQL 改动必须通过 key coverage、唯一性、基础测试和人工抽样验证。 |
| SQL 优化缺少事实依据 | 所有 SQL 改动前必须提交 query log 或 FORMAT Null 基准，收益低于 `10%` 不实施。 |
| 后续引入 int incremental 复杂化 | 本 RFC 明确将其放入 P2，且必须另写传播规则。 |

## 验收标准

- 3 个 mart 字段逐列匹配 tests 已删除，dbt manifest 中不再出现这些 test nodes。
- raw ClickHouse 最新 year 分区刷新后，能触发相关 int/mart 全量重建。
- 日常相关 int/mart build 保留 key coverage、key set、unique、not null、security code format 等基础门禁。
- 日常 dbt build 不依赖 `_sync_at`、raw sync 变更状态表或 dbt vars 水位线。
- 首期不新增 current-year/window/full 三套长期调度 job。
- 新增至少一份运行报告，对比：
  - baseline full job：`374.86s`
  - 删除字段匹配测试后的相关 int/mart build
  - 可选手动完整验证
- `mart_stock_quotes_daily` 若发生 SQL 优化，必须附带前后 query log 或 FORMAT Null 基准和 key coverage/抽样验证结果。
- 若继续优化 int 层，必须按 `int_stock_quotes_daily_adj`、`int_stock_quotes_daily_unadj`、`int_stock_adjustment_factor` 的优先级推进，并记录每步收益。
- int 层仍保持当前 full rebuild 语义，除非另有 RFC 接管。

## 后续文档

- 将本 RFC 关联到 `docs/issues/baostock-2026-06-26-downstream-performance.md`。
- 新运行报告写入 `docs/jobs/reports/`。
- 若 P2 int incremental 被重新打开，应新建独立 RFC，先定义每个模型的 affected window 传播规则。
