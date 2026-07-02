# RFC 0040: Dagster stg 到 mart 资产盘点

状态：Accepted（资产盘点基线；0066/0067 已落地，2026-07-01）
领域：Dagster, dbt, Furnace, Rearview, ClickHouse
关联系统：pipeline/scheduler, pipeline/elt, fleur_staging, fleur_intermediate, fleur_calculation, fleur_portfolio, fleur_marts
架构事实：
- docs/architecture/data-platform.md
- docs/architecture/scheduler-architecture.md
- docs/architecture/dbt_layer/
相关文档：
- docs/RFC/0038-dbt-baostock-downstream-performance-optimization.md
- docs/RFC/0036-racingline-strategy-portfolio-statement.md
- docs/jobs/dagster-definitions-lineage-2026-06-10.md

## 摘要

本文盘点当前 Dagster 资产图中从 dbt staging 到 marts 的资产，并把 calculation 和 portfolio 相关资产一起纳入同一张基线。目标是为后续梳理调度粒度、计算层归属、portfolio 数据链路和 mart 优化提供事实入口。

本文只记录当前实现，不提出最终重构方案，不修改 asset selection、dbt SQL、Furnace 公式或 Rearview worker 行为。

2026-07-01 补充实现结论：编排层已收敛为一个 daily network controller：`daily__fetch_history_sources_to_marts_schedule_job`。daily network 以每天变动的数据为 root，沿真实 asset dependency 扩展到 staging、intermediate、calculation、wrappers 和 marts。

2026-07-02 补充实现结论：RFC 0045 / Plan 0073 已将 portfolio live 控制面并入 `daily__fetch_history_sources_to_marts_schedule_job` 的 terminal step。`rearview/daily__portfolio_nav_liquidation` 是无分区结果资产，生产路径不接受用户日期范围，默认调用 Rearview settlement-target、single-day daily-runs、status 和 fact-count APIs。Portfolio backtest analytics 仍不纳入 source-to-marts network。

## 事实来源

本次盘点以当前代码和 Dagster 注册定义为准，主要依据：

```bash
cd pipeline
uv run dg list defs --target-path scheduler --response-schema
uv run dg list defs --target-path scheduler --json
```

补充读取：

- `pipeline/scheduler/src/scheduler/defs/definitions.py`
- `pipeline/scheduler/src/scheduler/components/fleur_dbt.py`
- `pipeline/scheduler/src/scheduler/defs/dbt/defs.yaml`
- `pipeline/scheduler/src/scheduler/defs/furnace/assets.py`
- `pipeline/scheduler/src/scheduler/defs/furnace/definitions.py`
- `pipeline/scheduler/src/scheduler/defs/rearview/assets.py`
- `pipeline/scheduler/src/scheduler/defs/dbt_jobs.py`
- `pipeline/elt/dbt_project.yml`
- `pipeline/elt/models/sources_fleur_calculation.yml`
- `pipeline/elt/models/sources_fleur_portfolio.yml`

`dg list defs` 不要并行运行多个实例；Dagster component 本地状态目录会在并行构建时出现临时竞态。串行执行可以正常得到定义清单。

## 装配边界

`scheduler.defs.definitions.defs()` 当前把四类定义合并为一个 Dagster definitions graph：

| 来源 | 装配入口 | 资产范围 |
| --- | --- | --- |
| source/raw | `SOURCE_BUNDLES` + `CLICKHOUSE_RAW_ASSETS` | S3 source、compacted、ClickHouse raw；不是本文主体，只作为 staging 上游边界 |
| dbt | `component_tree.build_defs("dbt")` | dbt model 资产，覆盖 `dbt_staging`、`dbt_intermediate`、`dbt_marts` |
| Furnace | `component_tree.build_defs("furnace")` | `fleur_calculation.calc_stock_*` 六个可执行技术指标资产 |
| Rearview | `REARVIEW_DEFS` | `rearview/daily__portfolio_nav_liquidation` 生产控制面资产，以及 `rearview/example_0051_portfolio_live_run` 手动 example 资产 |

`FleurDbtProjectComponent` 会把 dbt model asset key 扁平化为模型名，例如 `int_stock_quotes_daily_adj`，并按模型所在目录打 `dbt_staging`、`dbt_intermediate`、`dbt_marts` group；tag 包含 `layer`、`owner=dbt`、`storage=clickhouse`。

dbt source asset 仍保留外部表 key，例如 `fleur_calculation/calc_portfolio_trade_metric` 和 `fleur_portfolio/portfolio_run_snapshot`。其中股票技术指标 calculation source 被同名 Furnace 可执行资产覆盖；portfolio calculation source 和 portfolio snapshot 当前是外部 source asset，不由 scheduler 直接执行。

## 注册资产总览

当前 `dg list defs --json` 中与本文范围相关的资产计数如下：

| Group | 数量 | 说明 |
| --- | ---: | --- |
| `dbt_staging` | 17 | dbt staging model assets |
| `dbt_intermediate` | 24 | dbt intermediate model assets；不包含 ephemeral catalog |
| `dbt_marts` | 11 | dbt mart model assets |
| `calculation` | 6 | Furnace 可执行 ClickHouse calculation assets |
| `default` | 5 | dbt source 产生的外部 portfolio calculation/source assets |
| `rearview` | 2 | strategy portfolio production NAV 清算控制面资产和 0051 example 手动清算资产 |

补充上下游边界：

| Group | 数量 | 本文处理方式 |
| --- | ---: | --- |
| `clickhouse_raw` | 17 | staging 上游，不展开盘点 |
| `s3_sources` | 22 | raw 上游，不展开盘点 |

## Staging 资产

17 个 staging 资产全部来自 dbt component，group 为 `dbt_staging`，默认 materialization 为 view，直接读取 ClickHouse raw 资产。

| Asset | 直接上游 |
| --- | --- |
| `stg_baostock__query_history_k_data_plus_daily` | `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` |
| `stg_baostock__query_stock_basic` | `clickhouse/raw/baostock__query_stock_basic` |
| `stg_chinabond__government_bond` | `clickhouse/raw/chinabond__government_bond` |
| `stg_eastmoney__balance` | `clickhouse/raw/eastmoney__balance` |
| `stg_eastmoney__cashflow_sq` | `clickhouse/raw/eastmoney__cashflow_sq` |
| `stg_eastmoney__cashflow_ytd` | `clickhouse/raw/eastmoney__cashflow_ytd` |
| `stg_eastmoney__dividend_allotment` | `clickhouse/raw/eastmoney__dividend_allotment` |
| `stg_eastmoney__dividend_main` | `clickhouse/raw/eastmoney__dividend_main` |
| `stg_eastmoney__equity_history` | `clickhouse/raw/eastmoney__equity_history` |
| `stg_eastmoney__freeholders` | `clickhouse/raw/eastmoney__freeholders` |
| `stg_eastmoney__income_sq` | `clickhouse/raw/eastmoney__income_sq` |
| `stg_eastmoney__income_ytd` | `clickhouse/raw/eastmoney__income_ytd` |
| `stg_jiuyan__action_field_compacted` | `clickhouse/raw/jiuyan__action_field_compacted` |
| `stg_jiuyan__industry_list` | `clickhouse/raw/jiuyan__industry_list` |
| `stg_jiuyan__industry_ocr_snapshot` | `clickhouse/raw/jiuyan__industry_ocr_snapshot` |
| `stg_sina__trade_calendar` | `clickhouse/raw/sina__trade_calendar` |
| `stg_ths__limit_up_pool_compacted` | `clickhouse/raw/ths__limit_up_pool_compacted` |

这些资产是 raw 到业务层的清洗入口，后续优化重点不是单独拆 staging job，而是确认 raw sync 成功后要触发哪些 downstream selector。

## Intermediate 资产

`dbt_intermediate` 当前注册 24 个 Dagster asset。dbt 项目默认 intermediate 为 view，但多个模型在 SQL 中显式 materialized 为 table。

| Asset | Materialization | 直接上游 |
| --- | --- | --- |
| `int_benchmark_basic_snapshot` | table | `int_index_basic_snapshot` |
| `int_benchmark_returns_daily` | table | `int_benchmark_basic_snapshot`, `int_index_quotes_daily` |
| `int_government_bond_yields_daily` | table | `stg_chinabond__government_bond` |
| `int_index_basic_snapshot` | table | `stg_baostock__query_stock_basic` |
| `int_index_quotes_daily` | table | `int_index_basic_snapshot`, `stg_baostock__query_history_k_data_plus_daily` |
| `int_portfolio_closed_trade` | view | `fleur_calculation/calc_portfolio_closed_trade`, `fleur_portfolio/portfolio_run_snapshot` |
| `int_portfolio_performance_metric` | view | `fleur_calculation/calc_portfolio_performance_metric`, `fleur_portfolio/portfolio_run_snapshot` |
| `int_portfolio_performance_metric_status` | view | `fleur_calculation/calc_portfolio_performance_metric_status`, `fleur_portfolio/portfolio_run_snapshot` |
| `int_portfolio_trade_metric` | view | `fleur_calculation/calc_portfolio_trade_metric`, `fleur_portfolio/portfolio_run_snapshot` |
| `int_risk_free_rate_daily` | table | `int_government_bond_yields_daily`, `int_trade_calendar` |
| `int_stock_adjustment_factor` | table | `int_stock_quotes_daily_unadj` |
| `int_stock_basic_snapshot` | table | `stg_baostock__query_stock_basic` |
| `int_stock_boll_daily` | view | `fleur_calculation/calc_stock_boll_daily` |
| `int_stock_exrights_event` | table | `stg_eastmoney__dividend_allotment`, `stg_eastmoney__dividend_main` |
| `int_stock_financial_valuation` | table | `int_stock_quotes_daily_unadj`, `int_stock_shares_history`, `stg_eastmoney__balance`, `stg_eastmoney__income_sq`, `stg_eastmoney__income_ytd` |
| `int_stock_kdj_daily` | view | `fleur_calculation/calc_stock_kdj_daily` |
| `int_stock_ma_daily` | view | `fleur_calculation/calc_stock_ma_daily` |
| `int_stock_macd_daily` | view | `fleur_calculation/calc_stock_macd_daily` |
| `int_stock_price_pattern_daily` | view | `fleur_calculation/calc_stock_price_pattern_daily` |
| `int_stock_quotes_daily_adj` | table | `int_stock_adjustment_factor`, `int_stock_quotes_daily_unadj` |
| `int_stock_quotes_daily_unadj` | table | `int_stock_basic_snapshot`, `int_stock_exrights_event`, `int_stock_shares_history`, `int_trade_calendar`, `stg_baostock__query_history_k_data_plus_daily` |
| `int_stock_rsi_daily` | view | `fleur_calculation/calc_stock_rsi_daily` |
| `int_stock_shares_history` | table | `stg_eastmoney__equity_history`, `stg_eastmoney__freeholders` |
| `int_trade_calendar` | table | `stg_sina__trade_calendar` |

`int_portfolio_performance_metric_rank_catalog` 是 dbt ephemeral model，供 portfolio rank mart 使用，但不是 Dagster 注册 asset。

## Mart 资产

`dbt_marts` 当前注册 11 个 Dagster asset，全部显式 materialized 为 table。

| Asset | 直接上游 |
| --- | --- |
| `mart_benchmark_returns_daily` | `int_benchmark_returns_daily` |
| `mart_portfolio_performance_metric_rank` | `int_portfolio_performance_metric`, `int_portfolio_performance_metric_status` |
| `mart_portfolio_trade_metric_rank` | `int_portfolio_trade_metric` |
| `mart_risk_free_rate_daily` | `int_risk_free_rate_daily` |
| `mart_stock_basic_snapshot` | `int_stock_basic_snapshot` |
| `mart_stock_momentum_indicator_daily` | `int_stock_kdj_daily`, `int_stock_rsi_daily` |
| `mart_stock_price_pattern_daily` | `int_stock_price_pattern_daily` |
| `mart_stock_quotes_daily` | `int_stock_financial_valuation`, `int_stock_kdj_daily`, `int_stock_quotes_daily_adj`, `int_stock_quotes_daily_unadj` |
| `mart_stock_trend_indicator_daily` | `int_stock_boll_daily`, `int_stock_ma_daily`, `int_stock_macd_daily` |
| `mart_stock_volume_indicator_daily` | `int_stock_ma_daily` |
| `mart_trade_calendar` | `int_trade_calendar` |

后续性能优化应优先区分物理 table 和 thin wrapper：多数 calculation wrapper 是 view，真正重成本一般在上游 calculation 物理表和 mart table。

## Calculation 资产

### Furnace 可执行资产

Furnace 当前通过 `pipeline/scheduler/src/scheduler/defs/furnace/assets.py` 注册 6 个可执行 Dagster assets，group 为 `calculation`，tags 包含 `owner=furnace`、`layer=calculation`、`storage=clickhouse`、`modality=batch`。

| Asset | 直接上游 | dbt wrapper | 主要下游 mart |
| --- | --- | --- | --- |
| `fleur_calculation/calc_stock_kdj_daily` | `int_stock_quotes_daily_adj` | `int_stock_kdj_daily` | `mart_stock_momentum_indicator_daily`, `mart_stock_quotes_daily` |
| `fleur_calculation/calc_stock_ma_daily` | `int_stock_quotes_daily_adj`, `int_stock_quotes_daily_unadj` | `int_stock_ma_daily` | `mart_stock_trend_indicator_daily`, `mart_stock_volume_indicator_daily` |
| `fleur_calculation/calc_stock_rsi_daily` | `int_stock_quotes_daily_adj` | `int_stock_rsi_daily` | `mart_stock_momentum_indicator_daily` |
| `fleur_calculation/calc_stock_boll_daily` | `int_stock_quotes_daily_adj` | `int_stock_boll_daily` | `mart_stock_trend_indicator_daily` |
| `fleur_calculation/calc_stock_macd_daily` | `int_stock_quotes_daily_adj` | `int_stock_macd_daily` | `mart_stock_trend_indicator_daily` |
| `fleur_calculation/calc_stock_price_pattern_daily` | `int_stock_quotes_daily_adj`, `int_stock_quotes_daily_unadj` | `int_stock_price_pattern_daily` | `mart_stock_price_pattern_daily` |

Furnace run config 支持 `dry-run`、`append-latest` 和 `replace-cascade`。技术指标公式属于 Furnace/Rust 边界，dbt wrapper 不重算公式。

### Portfolio calculation source 资产

以下 4 个 `fleur_calculation` asset 在 Dagster 图里存在，但 `is_executable=false`，group 为 `default`。它们来自 dbt source，当前由 Rust portfolio worker 写入 ClickHouse，不由 scheduler 直接 materialize。

| Asset | 来源说明 | dbt wrapper / mart |
| --- | --- | --- |
| `fleur_calculation/calc_portfolio_closed_trade` | worker-authored closed trade ledger with FIFO lot pairing | `int_portfolio_closed_trade`; 当前无 mart |
| `fleur_calculation/calc_portfolio_performance_metric` | worker-authored portfolio performance metric rows | `int_portfolio_performance_metric` -> `mart_portfolio_performance_metric_rank` |
| `fleur_calculation/calc_portfolio_performance_metric_status` | metric-level status rows explaining NULL metrics | `int_portfolio_performance_metric_status` -> `mart_portfolio_performance_metric_rank` |
| `fleur_calculation/calc_portfolio_trade_metric` | worker-authored trade quality metrics | `int_portfolio_trade_metric` -> `mart_portfolio_trade_metric_rank` |

这组资产后续需要明确是否继续作为外部 source 被 dbt observe，还是由 Dagster 通过 Rearview/worker API 形成可执行或可观测资产。

## Portfolio 相关资产

### Backtest / ranking 链路

`fleur_portfolio/portfolio_run_snapshot` 是当前唯一注册到 Dagster 图里的 `fleur_portfolio` source asset，`is_executable=false`。它作为完整 result attempt marker，被 4 个 portfolio intermediate wrapper inner join：

| Wrapper | 上游 calculation | 下游 mart |
| --- | --- | --- |
| `int_portfolio_closed_trade` | `calc_portfolio_closed_trade` | 当前无 mart |
| `int_portfolio_performance_metric` | `calc_portfolio_performance_metric` | `mart_portfolio_performance_metric_rank` |
| `int_portfolio_performance_metric_status` | `calc_portfolio_performance_metric_status` | `mart_portfolio_performance_metric_rank` |
| `int_portfolio_trade_metric` | `calc_portfolio_trade_metric` | `mart_portfolio_trade_metric_rank` |

### Live strategy portfolio 链路

`rearview/daily__portfolio_nav_liquidation` 是 scheduler 内的 production Rearview strategy portfolio live 清算资产，group 为 `rearview`，由 `daily__fetch_history_sources_to_marts_schedule_job` 在 `all_source_to_marts + full` plan 的 terminal step 提交。它是无分区结果资产，不暴露 `trade_date/start_date/end_date/strategy_portfolio_id/chunk_size` 生产 config。默认路径调用 Rearview settlement-target 解析 latest target trade date，再调用 single-day daily-runs API，为所有 eligible active portfolios 创建或复用该 target 的 daily run，随后等待 worker 终态并把 `live_nav_daily`、`live_trade`、`live_closed_trade` 的行数写入 materialization metadata。

`rearview/example_0051_portfolio_live_run` 是 0051 低位反转 example 手动回归资产，仍通过 `example__portfolio_live_job` 独立运行，不挂 production schedule。

`pipeline/elt/models/sources_fleur_portfolio.yml` 已记录以下 live 表：

- `live_run_snapshot`
- `live_nav_daily`
- `live_target`
- `live_order`
- `live_trade`
- `live_position_day`
- `live_event`
- `live_performance_metric`
- `live_performance_metric_status`
- `live_closed_trade`
- `live_trade_metric`

这些 live 表当前没有出现在 `dg list defs --json` 的 asset 清单中；当前 dbt stg -> mart 图没有消费它们。它们主要由 Rearview read model 和 Racingline UI 消费，不属于现有 dbt mart 输出。

## Jobs、Schedules 和 Sensors

当前与本文范围直接相关的 registered Dagster entrypoint：

| 类型 | 名称 | 当前选择范围 |
| --- | --- | --- |
| job | `daily__fetch_history_sources_to_marts_schedule_job` | `target_date` 单日 source -> raw -> stg -> int -> calculation -> mart controller，并在 full/all scope 末尾提交 `rearview/daily__portfolio_nav_liquidation` terminal step |
| schedule | `daily__fetch_history_sources_to_marts_schedule` | 每日 `18:30` Asia/Shanghai 触发 daily controller，默认 stopped，当前 schedule config 为 `dry_run=true` |
| job | `backfill__fetch_history_sources_to_raw_job` | 手动 history source/raw 修复入口 |
| job | `backfill__fetch_history_sources_to_marts_job` | 手动 history source-to-marts 修复入口 |
| job | `example__portfolio_live_job` | 0051 example portfolio 手动清算回归入口；不挂 production schedule |
| sensor | `slack_asset_failure_sensor` | 跨 job/asset failure 告警 |

`dg list defs --json` 还会显示 Dagster 自动生成的 `default_automation_condition_sensor`；它不是手写 production daily trigger。

以下旧入口已经不再 registered：

- `dbt__staging_build_job`
- `dbt__marts_build_job`
- `stock__daily_build_job`
- `stock__daily_build_schedule`
- source-specific jobs/schedules
- `clickhouse__raw_sync_*_job`
- `baostock_raw_sync_success_triggers_stock_daily_build`

历史上 `stock__daily_build_job` 会执行所有 calculation group 资产，但 dbt 侧只显式选择 `int_stock_kdj_daily` 和 `mart_stock_quotes_daily` 作为 calculation 回接和 mart 重建对象。该不完整 selection 已由 daily controller 替代。

## Daily Network 设计方向

### 设计判断

新的编排层应先收敛为一个 daily network job，而不是把每天都会刷新的一组链路拆成多个独立 daily jobs。该 job 的边界不是 dbt layer，也不是单一 source domain，而是：

```text
给定一个 `target_date`，所有今天会变动或需要随今天输入重建的资产闭包。
```

第一版已实现为 `daily__fetch_history_sources_to_marts_schedule_job`。它是 `backfill__fetch_history_sources_to_marts_job` 的每日增量版：复用 source-to-marts scope、stage、asset key、run config 和排除规则，但把日期范围固定为 `target_date..target_date`，并将 Furnace 非 dry-run 模式从历史修正的 `replace-cascade` 改为日常增量的 `append-latest`。Dagster 仍负责 asset selection、运行配置、Furnace run config、source/raw 到 downstream 的触发和运行观测；dbt 仍负责 staging/intermediate/mart SQL 和 tests；Furnace 仍负责技术指标计算公式。

### 覆盖结论

daily source-to-marts network 覆盖从市场数据 source/raw 出发的 stg -> mart 链路，并在 full/all scope 末尾追加 portfolio live NAV 清算 terminal step。Portfolio backtest analytics 仍不纳入该 network。覆盖方式分两类：

| 范围 | 覆盖方式 | 说明 |
| --- | --- | --- |
| `s3_sources` | daily root 或 prerequisite | 16 个非 Jiuyan source / compacted source assets 可纳入本轮 source-to-marts；`source/sina__trade_calendar` 是交易日历 prerequisite，不一定每天刷新，但可作为 daily network 上游事实。 |
| `clickhouse_raw` | daily root downstream | 14 个非 Jiuyan raw assets 可纳入本轮 source-to-marts，作为 staging 的真实上游。 |
| `dbt_staging` | materialize | 14 个非 Jiuyan staging 资产可纳入 daily network。 |
| `dbt_intermediate` | materialize | 14 个非 Jiuyan、非 portfolio intermediate 资产可纳入 daily network，包括股票、指数、利率和 technical indicator wrapper。 |
| `calculation` | materialize | 6 个 Furnace 股票技术指标资产全部应进入 daily network。 |
| `dbt_marts` | materialize | 9 个非 portfolio mart 资产可纳入 daily network，包括股票、指标、基准和利率 marts。 |
| Jiuyan 全系列 | 不纳入本轮 source-to-marts | action field、industry list、images、OCR、OCR snapshot source/raw/staging/downstream 后续独立 job 规划。 |
| portfolio backtest analytics | 不纳入 source-to-marts | `fleur_portfolio/portfolio_run_snapshot`、`fleur_calculation/calc_portfolio_*`、`int_portfolio_*` 和 `mart_portfolio_*_rank` 属于 portfolio worker 输出后的 analytics 链路，应独立规划。 |
| portfolio live | 作为 terminal step 纳入 full/all daily controller | `rearview/daily__portfolio_nav_liquidation` 由 daily controller 在 source/raw/dbt/Furnace/marts 成功后提交；`fleur_portfolio.live_*` 仍主要由 Rearview read model 和 Racingline UI 消费，不进入当前 dbt mart 输出。 |

因此，如果“覆盖全部资产”指的是 market/source/raw 驱动的 stg -> mart 图和 Furnace calculation，答案是可以覆盖；如果包含 portfolio backtest analytics 或 portfolio live，则不应纳入 `source_to_marts` 范围，需要单独 job/backfill 计划。

### Daily root 分类

daily network 的 root 不应写成“所有 staging”，而应从每天变化的数据入口开始：

| Root 类别 | 当前资产 |
| --- | --- |
| 交易日事实 | `source/sina__trade_calendar`，`clickhouse/raw/sina__trade_calendar`，`stg_sina__trade_calendar`，`int_trade_calendar`，`mart_trade_calendar` |
| 股票基础和日行情 | `source/baostock__query_stock_basic`，`source/baostock__query_history_k_data_plus_daily`，`source/baostock__query_history_k_data_plus_daily_compacted`，对应 raw/staging/int/mart |
| F10 财务和股本 | EastMoney balance、cashflow、income、dividend、equity、freeholders 九组 source/raw/staging，以及 `int_stock_shares_history`、`int_stock_exrights_event`、`int_stock_financial_valuation` |
| 指数和基准 | `int_index_basic_snapshot`，`int_index_quotes_daily`，`int_benchmark_basic_snapshot`，`int_benchmark_returns_daily`，`mart_benchmark_returns_daily` |
| 利率和无风险收益 | `source/chinabond__government_bond`，`clickhouse/raw/chinabond__government_bond`，`stg_chinabond__government_bond`，`int_government_bond_yields_daily`，`int_risk_free_rate_daily`，`mart_risk_free_rate_daily` |
| 市场事件 | 仅 THS limit up pool 的 source/raw/staging 资产；Jiuyan action/industry/OCR 后续独立规划 |
| 股票技术指标 | 6 个 `fleur_calculation/calc_stock_*`，6 个 `int_stock_*` wrapper，4 个 indicator marts 和 `mart_stock_quotes_daily` |
| portfolio backtest analytics | 不纳入 source-to-marts；`portfolio_run_snapshot`、4 个 `calc_portfolio_*` source assets、4 个 portfolio intermediate wrappers、2 个 portfolio rank marts 另行规划 |

### 建议的 daily 阶段

Dagster 实际执行顺序由 asset dependency 决定；文档和代码中仍应保留阶段命名，方便 review 和排障：

| 阶段 | 资产范围 | 失败处理 |
| --- | --- | --- |
| `daily_source_fetch` | 当天需要刷新的 `s3_sources` | source 失败时不触发对应 raw/downstream；可按 source 重跑。 |
| `daily_raw_sync` | 14 个非 Jiuyan `clickhouse_raw` assets | raw sync 失败时停止进入 dbt 层。 |
| `daily_staging` | 14 个非 Jiuyan `dbt_staging` assets | staging tests 失败时停止进入 downstream。 |
| `daily_core_intermediate` | 行情、股本、除权、估值、指数、基准、利率、市场事件 intermediate tables | 用 dbt 基础 tests 控制 grain 和 key。 |
| `daily_calculation` | 6 个 Furnace calculation assets | 使用同一 `target_date` 配置 `append-latest`；历史修正仍走 `replace-cascade` 手动路径。 |
| `daily_calc_wrappers` | 6 个 calculation wrapper views | 只负责暴露字段和测试，不重算公式。 |
| `daily_marts` | 9 个非 portfolio mart assets | 日常只跑基础 checks；高成本全历史 tests 另设完整验证入口。 |
| `portfolio_live_liquidation` | `rearview/daily__portfolio_nav_liquidation` | full/all scope 的 terminal step；调用现有 Rearview APIs，等待 worker 终态并核验 fact counts。 |
| portfolio backtest analytics | 不在 daily source-to-marts 阶段内 | 依赖 portfolio worker 外部输出；后续单独定义 portfolio analytics job/backfill。 |

### 第一版 controller 形态

第一版没有使用单一 `AssetSelection` job，而是使用 controller job 展开阶段化 child materialization runs。原因是同一个 `target_date` 需要同时映射：

- 日分区 source：`target_date...target_date`。
- 年分区 source/raw：`target_date.year`，并向 partial current-year source 传 `refresh_until_date` 或 `cutoff_trade_date`。
- snapshot source/raw：无分区；`target_date` 保持必填但底层 snapshot plan 忽略日期窗口。
- Furnace calculation：`request_from=request_to=target_date`；daily 非 dry-run 使用 `append-latest`。
- dbt staging/intermediate/wrappers/marts：无 Dagster 分区，按 stage asset key 显式 materialize。

### 与历史 `stock__daily_build_job` 的差异

历史 `stock__daily_build_job` 是 daily network 的一个不完整子集：

```text
int_stock_quotes_daily_unadj
int_stock_adjustment_factor
int_stock_quotes_daily_adj
calculation group
int_stock_kdj_daily
mart_stock_quotes_daily
```

缺口包括：

- `int_stock_ma_daily`、`int_stock_rsi_daily`、`int_stock_boll_daily`、`int_stock_macd_daily`、`int_stock_price_pattern_daily`
- `mart_stock_momentum_indicator_daily`
- `mart_stock_trend_indicator_daily`
- `mart_stock_volume_indicator_daily`
- `mart_stock_price_pattern_daily`
- benchmark、risk free、stock basic 和 calendar marts
- portfolio backtest wrappers/rank marts；这组资产不再视为 source-to-marts 缺口，而是独立 portfolio analytics 范围

daily network 第一版已经替代这个不完整 selection，不再在它旁边增加新的 daily jobs。

### 旧编排面清理原则

daily network 不应叠加在旧 jobs/schedules/sensors 上渐进扩散。此前注册面同时存在 source-specific daily jobs、ClickHouse raw sync jobs、dbt layer jobs、stock daily downstream job 和 portfolio live job；如果继续在这些旧入口旁边增加新 job，后续很难判断哪一个入口才是日常生产事实。

第一版已采用 clean-slate 编排面：

1. 新增 `daily__fetch_history_sources_to_marts_schedule_job` 作为唯一日常 source -> raw -> stg -> int -> calculation -> mart 主入口。
2. 新增唯一 `ScheduleDefinition`：`daily__fetch_history_sources_to_marts_schedule`，触发 `daily__fetch_history_sources_to_marts_schedule_job`；不再新增或保留 `daily__source_to_marts_schedule`。
3. 旧 daily/transformation jobs 不再作为生产入口注册。
4. source/raw 的底层能力可以复用，但不再暴露多个彼此重叠的 daily schedules。
5. backfill 和 portfolio backtest analytics 不在本次清理范围内；portfolio live 已由 RFC 0045 / Plan 0073 收敛为 daily controller terminal step。

当前 registered definitions 清理结果：

| 定义 | 当前处理 | 理由 |
| --- | --- | --- |
| `dbt__staging_build_job` | 不再 registered | layer job 不应作为 daily production 入口。 |
| `dbt__marts_build_job` | 不再 registered | 全量 dbt build 适合作为人工/完整验证入口，不适合日常编排事实。 |
| `stock__daily_build_job` | 不再 registered，由 `daily__fetch_history_sources_to_marts_schedule_job` 替代 | 历史 selection 是不完整 daily network 子集。 |
| `stock__daily_build_schedule` | 不再 registered | daily network 应只有一个主 schedule 或触发器。 |
| `baostock_raw_sync_success_triggers_stock_daily_build` | 不再 registered | 不能继续只触发 stock 子集。 |
| source-specific daily jobs/schedules | 不再 registered | 避免多个 source schedule 与 daily downstream job 并行制造运行事实分叉。 |
| `clickhouse__raw_sync_*_job` | 不再 registered | raw sync 是 daily network 阶段，不应再是另一套生产入口。 |
| `backfill__fetch_history_sources_to_raw_job` | 保留 raw-only 语义；另新增 `backfill__fetch_history_sources_to_marts_job` | 手动 raw 修复和历史 source-to-marts 重建应分层。`backfill__fetch_snapshot_sources_to_raw_job` 移除，Jiuyan 异动、行业列表和 OCR 后续独立规划。 |
| `strategy_portfolio__daily_run_job`、`portfolio__daily_run_schedule` | 不再 registered | portfolio live 控制面已由 `daily__fetch_history_sources_to_marts_schedule_job` 的 `rearview/daily__portfolio_nav_liquidation` terminal step 承担。 |
| `slack_asset_failure_sensor` | 保留 | 跨 job 告警能力，不定义生产编排路径。 |

当前 dry-run 验收见 `docs/jobs/reports/2026-07-01-daily-fetch-history-sources-to-marts-schedule-job-dry-run.md`。

### 历史回填延展

`backfill__fetch_history_sources_to_raw_job` 不应被原地改成 source-to-marts。它仍是 source/raw 修复入口，负责 source、compacted source、snapshot reference data 和 ClickHouse raw sync。`start_date` 和 `end_date` 保持必填 config；snapshot reference data 主动忽略这两个参数。需要从历史 source/raw 修复继续推进到 dbt、Furnace 和 marts 时，应新增上层 `backfill__fetch_history_sources_to_marts_job`。

`backfill__fetch_snapshot_sources_to_raw_job` 不再作为独立入口保留。Jiuyan 异动、行业列表、images/OCR/snapshot pipeline 不纳入本次 backfill source-to-marts 迭代，后续单独规划。

该上层入口的阶段边界与 daily network 对齐，但运行语义不同：

| 阶段 | 回填语义 |
| --- | --- |
| source/raw | 复用 source/raw 回填 registry 和 child run 提交机制；snapshot reference data 使用同一入口，忽略日期参数 |
| dbt staging/intermediate/marts | 按 `target_scope` 显式选择相关 downstream，不默认重建全部 marts |
| Furnace calculation | 历史区间使用 `replace-cascade`，不使用 daily `append-latest` |
| portfolio backtest analytics | 不纳入 source-to-marts；另开 portfolio analytics job/backfill |
| portfolio live | 不纳入 history source-to-marts backfill；production daily 由 `daily__portfolio_nav_liquidation` terminal step 处理 |

对应执行计划见 [Plan 0066](../plans/archive/0066-backfill-source-to-marts-controller-plan.md)。

## Asset Checks 基线

dbt component 已把 dbt tests 注册为 Dagster asset checks。当前 checks 覆盖所有 staging、intermediate 和 mart asset；portfolio 和 calculation 外部 source 的 source tests 不作为 source checks 直接启用。

高层计数：

| 范围 | 资产数 | 说明 |
| --- | ---: | --- |
| staging | 17 | 每个 staging asset 至少有唯一性、not null、accepted values 或格式类 checks |
| intermediate | 24 | calculation/portfolio wrapper 也有 grain、not null、状态值或格式类 checks |
| marts | 11 | mart grain、not null、格式、关系和业务边界 checks |

后续优化 asset checks 时，应优先区分：

1. 日常必须保留的 grain、not null、key coverage 和状态值检查。
2. 高成本全表 join 或字段逐列匹配检查。
3. 只适合手动完整验证或窗口化验证的检查。

## 当前待梳理问题

1. **calculation 资产存在两种归属**

   股票技术指标 calculation 是 Furnace/Dagster 可执行资产；portfolio calculation 是 Rust portfolio worker 写入的外部 source asset。需要明确是否接受这种混合归属，还是引入观察资产或控制面资产统一 materialization metadata。

2. **portfolio live 表没有进入 dbt mart 图**

   `fleur_portfolio.live_*` 已在 dbt source YAML 中有契约说明，但当前没有注册为 Dagster source asset，也没有 downstream dbt mart。若后续希望在 dbt marts 支撑 live statement、portfolio analytics 或跨组合排行，需要新增明确的 model 边界，而不是让前端或 Rearview read model 与 dbt mart 职责混在一起。

3. **物理表和 wrapper 混在同一 group**

   `dbt_intermediate` 同时包含 table、view 和外部 calculation wrapper。优化时不能只按 group 判断成本，应先按 materialization、数据规模和外部 source 变更频率分类。

4. **ephemeral model 不在 Dagster asset 图中**

   `int_portfolio_performance_metric_rank_catalog` 是 rank mart 的静态 catalog，但不是独立 Dagster asset。后续如果需要观测 rank catalog 变化，需要改 materialization 或把 catalog 事实移到 seed/source。

## 后续建议

本盘点已拆出并完成两个可执行计划：

1. Plan 0066：`backfill__fetch_history_sources_to_marts_job` 设计和落地，保留 raw-only 回填入口，在其上方新增历史 source/raw -> stg -> int -> calculation -> mart 的 controller。
2. Plan 0067：`daily__fetch_history_sources_to_marts_schedule_job` 设计和落地，作为 `backfill__fetch_history_sources_to_marts_job` 的每日增量版，覆盖 source/raw、非 portfolio dbt staging/intermediate/marts 和 Furnace calculation；第一版排除 portfolio backtest analytics 和 portfolio live。

剩余建议：

1. portfolio analytics 独立编排计划：明确 backtest portfolio calculation 是否新增 observe/control assets，以及 `int_portfolio_*` / `mart_portfolio_*_rank` 如何刷新。
2. portfolio live production daily 已进入 `daily__fetch_history_sources_to_marts_schedule_job` terminal step；后续若 dbt marts 明确消费 `fleur_portfolio.live_*`，再单独设计 portfolio live marts 或 observe assets。
3. 真实生产启用 daily schedule 前，把 `daily__fetch_history_sources_to_marts_schedule` 的 schedule config 从 `dry_run=true` 切换为 `dry_run=false`，并记录小范围 non-dry-run 验证报告。

## 最小验证

文档更新后至少运行：

```bash
make docs-check
git diff --check
```

若后续 RFC 转入实现计划，再追加：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
cd scheduler
uv run dg check defs
```
