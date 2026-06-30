# RFC 0039: Source/Raw 回填复杂度现状基线与统一入口草案

状态：Implemented（2026-06-30）
日期：2026-06-30
领域：Dagster, ClickHouse raw, source backfill
关联系统：pipeline/scheduler, pipeline/contracts, docs/jobs

## 摘要

本文先记录当前 Dagster `source/*` 层和 `clickhouse/raw/*` 层的注册资产清单，作为讨论“简化数据回填作业复杂度”的事实基线。

在此基础上，本文提出一个手动触发的统一回填 controller job 草案：用户只在 Dagster Web UI 中配置目标范围和日期区间，controller 负责把区间映射为 source、compacted source 和 ClickHouse raw 的实际 materialization 计划。

本草案不改变现有 source asset、raw sync、contract 或 dbt 语义。

## 事实来源

当前清单来自以下代码和命令：

- `pipeline/scheduler/src/scheduler/defs/definitions.py` 的 `SOURCE_BUNDLES` 和 `CLICKHOUSE_RAW_ASSETS` 聚合。
- `pipeline/scheduler/src/scheduler/defs/source_bundle.py` 的 source bundle 契约。
- `pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py` 的 `ENABLED_CLICKHOUSE_RAW_TABLE_SPECS`。
- `pipeline/scheduler` 下执行：

```bash
uv run dg list defs --json
```

统计口径：

| 层级 | 口径 | 数量 |
| --- | --- | ---: |
| Source assets | Dagster asset key 以 `source/` 开头的已注册资产 | 22 |
| Raw assets | Dagster asset key 以 `clickhouse/raw/` 开头的已注册资产 | 17 |
| Raw-covered source assets | 被 enabled ClickHouse raw spec 直接依赖的 source 资产 | 17 |
| Source-only assets | 参与 source 链路但不直接同步到 ClickHouse raw 的 source 资产 | 5 |

说明：部分 source bundle 资产的 tag `layer=compacted`，但 asset key 仍位于 `source/*`，Dagster group 仍为 `s3_sources`。本文按 asset key 前缀将其归入 source 层事实清单。

## 当前 Source Bundle

`SOURCE_BUNDLES` 当前显式聚合以下数据源，顺序如下：

| 顺序 | Bundle | Source asset 数量 |
| ---: | --- | ---: |
| 1 | `sina` | 1 |
| 2 | `jiuyan` | 6 |
| 3 | `ths` | 2 |
| 4 | `baostock` | 3 |
| 5 | `eastmoney` | 9 |
| 6 | `chinabond` | 1 |

## Source 资产清单

| Source asset | Bundle | 分区 | 直接上游 | 是否直接进入 raw |
| --- | --- | --- | --- | --- |
| `source/sina__trade_calendar` | `sina` | none | - | yes |
| `source/jiuyan__action_field` | `jiuyan` | daily | `source/sina__trade_calendar` | no |
| `source/jiuyan__action_field_compacted` | `jiuyan` | year | `source/jiuyan__action_field`, `source/sina__trade_calendar` | yes |
| `source/jiuyan__industry_list` | `jiuyan` | none | - | yes |
| `source/jiuyan__industry_images` | `jiuyan` | none | `source/jiuyan__industry_list` | no |
| `source/jiuyan__industry_ocr` | `jiuyan` | none | `source/jiuyan__industry_images` | no |
| `source/jiuyan__industry_ocr_snapshot` | `jiuyan` | none | `source/jiuyan__industry_ocr` | yes |
| `source/ths__limit_up_pool` | `ths` | daily | `source/sina__trade_calendar` | no |
| `source/ths__limit_up_pool_compacted` | `ths` | year | `source/ths__limit_up_pool`, `source/sina__trade_calendar` | yes |
| `source/baostock__query_stock_basic` | `baostock` | none | - | yes |
| `source/baostock__query_history_k_data_plus_daily` | `baostock` | daily | `source/baostock__query_stock_basic`, `source/sina__trade_calendar` | no |
| `source/baostock__query_history_k_data_plus_daily_compacted` | `baostock` | year | `source/baostock__query_history_k_data_plus_daily`, `source/sina__trade_calendar` | yes |
| `source/eastmoney__balance` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__cashflow_sq` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__cashflow_ytd` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__dividend_allotment` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__dividend_main` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__equity_history` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__freeholders` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__income_sq` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/eastmoney__income_ytd` | `eastmoney` | year | `source/baostock__query_stock_basic` | yes |
| `source/chinabond__government_bond` | `chinabond` | year | - | yes |

## Raw 资产清单

| Raw asset | 分区策略 | 直接 source 依赖 | ClickHouse 表 | 存储模式 |
| --- | --- | --- | --- | --- |
| `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` | year | `source/baostock__query_history_k_data_plus_daily_compacted` | `fleur_raw.baostock__query_history_k_data_plus_daily_compacted` | `partitioned` |
| `clickhouse/raw/baostock__query_stock_basic` | snapshot | `source/baostock__query_stock_basic` | `fleur_raw.baostock__query_stock_basic` | `latest_snapshot` |
| `clickhouse/raw/chinabond__government_bond` | year | `source/chinabond__government_bond` | `fleur_raw.chinabond__government_bond` | `partitioned` |
| `clickhouse/raw/eastmoney__balance` | year | `source/eastmoney__balance` | `fleur_raw.eastmoney__balance` | `partitioned` |
| `clickhouse/raw/eastmoney__cashflow_sq` | year | `source/eastmoney__cashflow_sq` | `fleur_raw.eastmoney__cashflow_sq` | `partitioned` |
| `clickhouse/raw/eastmoney__cashflow_ytd` | year | `source/eastmoney__cashflow_ytd` | `fleur_raw.eastmoney__cashflow_ytd` | `partitioned` |
| `clickhouse/raw/eastmoney__dividend_allotment` | year | `source/eastmoney__dividend_allotment` | `fleur_raw.eastmoney__dividend_allotment` | `partitioned` |
| `clickhouse/raw/eastmoney__dividend_main` | year | `source/eastmoney__dividend_main` | `fleur_raw.eastmoney__dividend_main` | `partitioned` |
| `clickhouse/raw/eastmoney__equity_history` | year | `source/eastmoney__equity_history` | `fleur_raw.eastmoney__equity_history` | `partitioned` |
| `clickhouse/raw/eastmoney__freeholders` | year | `source/eastmoney__freeholders` | `fleur_raw.eastmoney__freeholders` | `partitioned` |
| `clickhouse/raw/eastmoney__income_sq` | year | `source/eastmoney__income_sq` | `fleur_raw.eastmoney__income_sq` | `partitioned` |
| `clickhouse/raw/eastmoney__income_ytd` | year | `source/eastmoney__income_ytd` | `fleur_raw.eastmoney__income_ytd` | `partitioned` |
| `clickhouse/raw/jiuyan__action_field_compacted` | year | `source/jiuyan__action_field_compacted` | `fleur_raw.jiuyan__action_field_compacted` | `partitioned` |
| `clickhouse/raw/jiuyan__industry_list` | snapshot | `source/jiuyan__industry_list` | `fleur_raw.jiuyan__industry_list` | `latest_snapshot` |
| `clickhouse/raw/jiuyan__industry_ocr_snapshot` | snapshot | `source/jiuyan__industry_ocr_snapshot` | `fleur_raw.jiuyan__industry_ocr_snapshot` | `latest_snapshot` |
| `clickhouse/raw/sina__trade_calendar` | snapshot | `source/sina__trade_calendar` | `fleur_raw.sina__trade_calendar` | `latest_snapshot` |
| `clickhouse/raw/ths__limit_up_pool_compacted` | year | `source/ths__limit_up_pool_compacted` | `fleur_raw.ths__limit_up_pool_compacted` | `partitioned` |

## Source 到 Raw 覆盖关系

### 直接进入 raw 的 source 资产

| Source asset | Raw asset |
| --- | --- |
| `source/baostock__query_history_k_data_plus_daily_compacted` | `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` |
| `source/baostock__query_stock_basic` | `clickhouse/raw/baostock__query_stock_basic` |
| `source/chinabond__government_bond` | `clickhouse/raw/chinabond__government_bond` |
| `source/eastmoney__balance` | `clickhouse/raw/eastmoney__balance` |
| `source/eastmoney__cashflow_sq` | `clickhouse/raw/eastmoney__cashflow_sq` |
| `source/eastmoney__cashflow_ytd` | `clickhouse/raw/eastmoney__cashflow_ytd` |
| `source/eastmoney__dividend_allotment` | `clickhouse/raw/eastmoney__dividend_allotment` |
| `source/eastmoney__dividend_main` | `clickhouse/raw/eastmoney__dividend_main` |
| `source/eastmoney__equity_history` | `clickhouse/raw/eastmoney__equity_history` |
| `source/eastmoney__freeholders` | `clickhouse/raw/eastmoney__freeholders` |
| `source/eastmoney__income_sq` | `clickhouse/raw/eastmoney__income_sq` |
| `source/eastmoney__income_ytd` | `clickhouse/raw/eastmoney__income_ytd` |
| `source/jiuyan__action_field_compacted` | `clickhouse/raw/jiuyan__action_field_compacted` |
| `source/jiuyan__industry_list` | `clickhouse/raw/jiuyan__industry_list` |
| `source/jiuyan__industry_ocr_snapshot` | `clickhouse/raw/jiuyan__industry_ocr_snapshot` |
| `source/sina__trade_calendar` | `clickhouse/raw/sina__trade_calendar` |
| `source/ths__limit_up_pool_compacted` | `clickhouse/raw/ths__limit_up_pool_compacted` |

### 不直接进入 raw 的 source 资产

| Source asset | 当前角色 |
| --- | --- |
| `source/baostock__query_history_k_data_plus_daily` | daily S3 source，进入 `source/baostock__query_history_k_data_plus_daily_compacted` 后再同步 raw |
| `source/jiuyan__action_field` | daily S3 source，进入 `source/jiuyan__action_field_compacted` 后再同步 raw |
| `source/jiuyan__industry_images` | OCR 图片下载和 Postgres 状态链路中间资产 |
| `source/jiuyan__industry_ocr` | OCR 处理和 Postgres 状态链路中间资产 |
| `source/ths__limit_up_pool` | daily S3 source，进入 `source/ths__limit_up_pool_compacted` 后再同步 raw |

## 当前回填复杂度事实

后续讨论应基于以下已确认事实：

- Source 层同时存在 snapshot、daily、year 三类分区/非分区形态。
- Raw 层当前只有 snapshot 与 year 两类同步策略。
- BaoStock、Jiuyan market event、THS market event 都存在 daily source 到 year compacted source 再到 raw 的三段链路。
- EastMoney 与 ChinaBond 当前是 year source 直接同步 year raw。
- Sina trade calendar、BaoStock stock basic、Jiuyan industry list 和 Jiuyan OCR snapshot 是 snapshot raw。
- Jiuyan OCR 图片与 OCR 状态资产参与 materialization 链路，但不直接落 ClickHouse raw。
- ClickHouse raw assets 来自 contract registry 生成的 enabled raw specs；source-only 中间资产不一定有 raw contract。

## 方案草案：统一手动回填入口

新增一个手动触发的 Dagster job：

```text
backfill__fetch_sources_to_raw_job
```

该 job 的核心职责不是直接抓取远端数据或写 ClickHouse，而是作为 controller 生成并提交真实的 asset materialization runs。source S3 写入、compacted source 生成和 ClickHouse raw 同步仍由现有资产完成。

### 设计目标

- 用户在 Web UI 中只配置目标范围和日期区间，不直接选择具体 asset key 或手写多段命令。
- Controller 根据当前资产事实，把一个业务区间拆成 Dagster 可执行的 daily partition range、year partitions 和 snapshot prerequisites。
- 每个实际写数据步骤仍保留 Dagster asset event、日志、重试、分区状态和 lineage。
- 回填入口按数据域或 source bundle 收敛，替代散落在 runbook 中的多段手工命令。
- 支持 `dry_run`，让用户先在 Web UI run log 中审阅执行计划。

### 非目标

- 不把所有 source/raw 资产强行塞入一个普通 `define_asset_job`。当前资产分区定义不同，单个 asset job 不适合表达 daily -> year compacted -> year raw 的跨分区映射。
- 不在 controller op 内直接调用 source 业务函数或 raw sync service 绕过 asset materialization。
- 不改变现有 S3 layout、ClickHouse raw table、contract registry 或 dbt source。
- 不把 snapshot 资产改造成日期分区资产。
- 不默认触发 dbt staging/int/mart，下游转换是否纳入另一个 RFC 决定。

## Web UI 配置形态

用户在 Dagster Web UI 中手动 launch `backfill__fetch_sources_to_raw_job`，只需要填以下配置：

```yaml
ops:
  backfill__fetch_sources_to_raw_controller:
    config:
      target_scope: baostock_daily_kline
      start_date: "2020-01-01"
      end_date: "2024-12-31"
      execution_mode: full
      refresh_prerequisite_snapshots: true
      overwrite_source_partitions: false
      dry_run: true
```

字段语义：

| 字段 | 类型 | 语义 |
| --- | --- | --- |
| `target_scope` | enum/string | 用户选择的回填目标。首期建议支持 `baostock_daily_kline`、`market_events`、`eastmoney_f10`、`chinabond`、`snapshot_reference_data`、`jiuyan_ocr_pipeline`、`all_raw_yearly`、`all_fetch_sources_to_raw`。 |
| `start_date` | date string or null | 用户期望回填区间起点，使用 `YYYY-MM-DD`。对 daily/year scope 必填；对纯 snapshot 或 OCR pipeline scope 可为空。 |
| `end_date` | date string or null | 用户期望回填区间终点，使用 `YYYY-MM-DD`。对 daily/year scope 必填；对纯 snapshot 或 OCR pipeline scope 可为空。 |
| `execution_mode` | enum | 执行模式，默认 `full`。首期建议支持 `full` 和 `raw_only`；`raw_only` 仅用于 source/compacted 已成功、只需恢复 raw sync 的场景。 |
| `refresh_prerequisite_snapshots` | bool | 是否在区间回填前刷新当前 `target_scope` 显式声明的前置 source snapshot assets。 |
| `overwrite_source_partitions` | bool | 是否允许 source asset 覆盖已有 S3 partition。具体支持范围由各 source asset config 决定；不支持该 config 的 assets 忽略此字段。 |
| `jiuyan_ocr_limit` | int or null | 仅作用于 `jiuyan_ocr_pipeline` 或包含该子 scope 的组合 scope。默认建议为 `100`，避免手动全量 OCR 误操作；设为 `null` 才表示不限制。 |
| `jiuyan_force_download` | bool | 仅作用于 `source/jiuyan__industry_images`，传递给现有 `force_download` config。 |
| `jiuyan_force_ocr` | bool | 仅作用于 `source/jiuyan__industry_ocr`，传递给现有 `force_ocr` config。 |
| `dry_run` | bool | 只生成计划和日志，不提交实际 materialization runs。 |

日期字段处理规则：

| `target_scope` 类型 | `start_date` / `end_date` 语义 |
| --- | --- |
| daily/year scope | 必填，用于生成 daily partition range 和 year partitions。 |
| `snapshot_reference_data` | 可为空；如果填写，只作为 run tags 和审计信息，不参与 partition selection。 |
| `jiuyan_ocr_pipeline` | 可为空；如果填写，只作为 run tags 和审计信息，不参与 partition selection。OCR 范围由 `jiuyan_ocr_limit`、`jiuyan_force_download`、`jiuyan_force_ocr` 和现有 OCR 状态表决定。 |
| mixed scope | 只要包含 daily/year 子 scope 就必填；纯 snapshot/OCR 子步骤不使用日期做 partition selection。 |

`refresh_prerequisite_snapshots=true` 适合历史大范围回填或不确定前置快照是否最新的场景。设为 `false` 时，controller 假设前置 snapshot 已经可用，只回填目标区间，适合重复修复少量 year/day。

该参数只作用于当前 `target_scope` 的 `prerequisite_snapshots` 列表，不刷新无关 snapshot assets。组合 scope 使用子 scope prerequisite 的去重集合。前置 snapshot 刷新只 materialize source snapshot；如果需要把这些 snapshot 同步到 ClickHouse raw，应显式选择 `snapshot_reference_data` 或包含它的组合 scope。

| `target_scope` | `refresh_prerequisite_snapshots=true` 时刷新 |
| --- | --- |
| `baostock_daily_kline` | `source/sina__trade_calendar`、`source/baostock__query_stock_basic` |
| `market_events` | `source/sina__trade_calendar` |
| `eastmoney_f10` | `source/baostock__query_stock_basic` |
| `chinabond` | 无 |
| `snapshot_reference_data` | 无额外 prerequisite；该 scope 自身就是 snapshot source/raw 刷新目标 |
| `jiuyan_ocr_pipeline` | 无额外 prerequisite；`source/jiuyan__industry_list` 是 pipeline 的第一步 |
| `all_raw_yearly` | `baostock_daily_kline`、`market_events`、`eastmoney_f10`、`chinabond` 的 prerequisite 去重集合 |
| `all_fetch_sources_to_raw` | 所有子 scope 的 prerequisite 去重集合，不包含无关 snapshot |

如果某个 snapshot 同时是一个子 scope 的目标资产和另一个子 scope 的 prerequisite，controller 只应在同一个 `backfill.id` 下执行一次，并复用该 materialization 作为后续步骤的依赖。例如 `all_fetch_sources_to_raw` 同时包含 `snapshot_reference_data` 和 `jiuyan_ocr_pipeline` 时，`source/jiuyan__industry_list` 只执行一次。

`jiuyan_ocr_pipeline` 的配置示例：

```yaml
ops:
  backfill__fetch_sources_to_raw_controller:
    config:
      target_scope: jiuyan_ocr_pipeline
      start_date: null
      end_date: null
      execution_mode: full
      jiuyan_ocr_limit: 100
      jiuyan_force_download: false
      jiuyan_force_ocr: false
      dry_run: true
```

### 预期 Web UI 表现

`dry_run=true` 时，job run 应输出结构化执行计划：

```text
Backfill plan: target_scope=baostock_daily_kline start_date=2020-01-01 end_date=2024-12-31
1. snapshot prerequisite: source/sina__trade_calendar
2. snapshot prerequisite: source/baostock__query_stock_basic
3. daily source: source/baostock__query_history_k_data_plus_daily partition_range=2020-01-01...2020-12-31
4. compacted source: source/baostock__query_history_k_data_plus_daily_compacted partition=2020
5. raw sync: clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted partition=2020
...
```

`dry_run=false` 时，controller 应提交真实 asset runs，并为所有子 run 打统一 tags：

| Tag | 示例 | 用途 |
| --- | --- | --- |
| `backfill.kind` | `fetch_sources_to_raw` | 标识统一 source/raw 回填 |
| `backfill.id` | `20260630-baostock-2020-2024` | 关联一次 controller run 与所有子 runs |
| `backfill.target_scope` | `baostock_daily_kline` | 标识用户选择的回填范围 |
| `backfill.start_date` | `2020-01-01` | 记录用户输入区间 |
| `backfill.end_date` | `2024-12-31` | 记录用户输入区间 |
| `backfill.step` | `source_daily` / `source_compacted` / `raw` | 标识子 run 阶段 |
| `backfill.year` | `2022` | 标识 year partition 子 run |

子 run 在 Web UI 中应能按 `backfill.id` 搜索聚合，失败时用户能定位到具体 asset、partition 和阶段。

## 策略注册表

Controller 不应硬编码一长串 if/else。建议新增一个内部策略注册表，以 `target_scope` 映射到执行策略、资产选择和前置资产。

### 策略类型

| 策略 | 适用资产 | 执行动作 |
| --- | --- | --- |
| `snapshot_to_raw` | `sina__trade_calendar`, `baostock__query_stock_basic`, `jiuyan__industry_list`, `jiuyan__industry_ocr_snapshot` | 先 materialize source snapshot，再 materialize 对应 raw snapshot。 |
| `year_source_to_raw` | EastMoney 9 个资产、`chinabond__government_bond` | 将日期区间映射为 year partitions，逐 year materialize source，再 materialize 同 year raw。 |
| `daily_to_compacted_to_raw` | BaoStock 日 K、Jiuyan action field、THS limit up pool | 先 materialize daily source partition range，再 materialize year compacted source，再 materialize 同 year raw。 |
| `ocr_pipeline_to_snapshot_raw` | Jiuyan OCR 图片与识别链路 | materialize `industry_list -> images -> ocr -> industry_ocr_snapshot -> raw`，不强行套日期区间。 |

### 首期 target_scope

| `target_scope` | 策略 | 覆盖资产 |
| --- | --- | --- |
| `baostock_daily_kline` | `daily_to_compacted_to_raw` | `source/baostock__query_history_k_data_plus_daily` -> `source/baostock__query_history_k_data_plus_daily_compacted` -> `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` |
| `market_events` | `daily_to_compacted_to_raw` | `jiuyan__action_field` 与 `ths__limit_up_pool` 两条 market event 链路 |
| `eastmoney_f10` | `year_source_to_raw` | EastMoney 9 个 year source/raw assets |
| `chinabond` | `year_source_to_raw` | `source/chinabond__government_bond` -> `clickhouse/raw/chinabond__government_bond` |
| `snapshot_reference_data` | `snapshot_to_raw` | `source/sina__trade_calendar`、`source/baostock__query_stock_basic`、`source/jiuyan__industry_list` 及其对应 snapshot raw |
| `jiuyan_ocr_pipeline` | `ocr_pipeline_to_snapshot_raw` | `source/jiuyan__industry_list` -> `source/jiuyan__industry_images` -> `source/jiuyan__industry_ocr` -> `source/jiuyan__industry_ocr_snapshot` -> `clickhouse/raw/jiuyan__industry_ocr_snapshot` |
| `all_raw_yearly` | mixed | 当前所有 year raw 链路，不包含 snapshot reference data 和 OCR pipeline |
| `all_fetch_sources_to_raw` | mixed | 显式组合 `snapshot_reference_data`、`baostock_daily_kline`、`market_events`、`eastmoney_f10`、`chinabond` 和 `jiuyan_ocr_pipeline`，覆盖当前全部 source/raw 资产 |

组合 scope 必须对同一 asset key 去重，并保留依赖顺序。去重后仍应保证上游先于下游执行，例如 `source/jiuyan__industry_list` 先于 `source/jiuyan__industry_images`，`source/baostock__query_history_k_data_plus_daily` 先于 compacted source，compacted source 先于 raw sync。

### 覆盖完整性

补充 `snapshot_reference_data` 和 `jiuyan_ocr_pipeline` 后，`target_scope` 映射可以覆盖当前全部已注册 source/raw 资产。

| 资产类别 | 覆盖方式 |
| --- | --- |
| Snapshot source/raw | `snapshot_reference_data` 覆盖 `sina__trade_calendar`、`baostock__query_stock_basic`、`jiuyan__industry_list`。`jiuyan_ocr_pipeline` 覆盖 `jiuyan__industry_ocr_snapshot` 及其 raw。 |
| Daily -> compacted -> raw | `baostock_daily_kline` 覆盖 BaoStock 日 K；`market_events` 覆盖 Jiuyan action field 与 THS limit up pool。 |
| Year source -> raw | `eastmoney_f10` 覆盖 EastMoney 9 个 F10 year source/raw；`chinabond` 覆盖 ChinaBond government bond。 |
| Source-only 中间资产 | BaoStock/Jiuyan/THS daily source 由各自 compacted 链路覆盖；`jiuyan_ocr_pipeline` 覆盖 `industry_images` 和 `industry_ocr`。 |
| 全量组合入口 | `all_fetch_sources_to_raw` 显式组合所有首期 scope，覆盖当前 22 个 source assets 和 17 个 raw assets。 |

这里的“覆盖”指 asset graph 覆盖，不等同于无条件抓取每个远端接口的全量历史数据。特别是 OCR pipeline 的数据处理范围仍受 `jiuyan_ocr_limit`、`jiuyan_force_download`、`jiuyan_force_ocr` 和现有 OCR 状态表控制。

### 区间映射规则

| 输入区间 | daily source | year source/raw |
| --- | --- | --- |
| `2020-01-01..2020-12-31` | `2020-01-01...2020-12-31` | `2020` |
| `2020-03-15..2022-06-30` | 原始 daily 区间保持不变 | `2020`, `2021`, `2022` |
| `2026-01-01..2026-06-30` | daily 只跑到 `end_date` | year source/raw 跑 `2026`，需要向支持该语义的资产传 `refresh_until_date` 或 `cutoff_trade_date` |

Controller 必须在生成计划时显式记录 date range 到 year partitions 的映射，不能让用户手动推导。

`execution_mode=raw_only` 时，controller 不重新执行 source 或 compacted source，只对当前 `target_scope` 中已经有 raw spec 的 assets 生成 raw sync runs。对 `baostock_daily_kline`、`market_events` 等 daily -> compacted -> raw 链路，`raw_only` 仍按 year partitions 选择 raw assets；它假设对应 compacted source partition 已经成功存在。

## 执行方式建议

首期实现应替换当前 BaoStock 专用 controller 中的 shell-out 模式。当前 `baostock__history_k_data_year_range_backfill_job` 通过 `subprocess.run("uv run dg launch ...")` 串行提交多段命令，这能工作，但长期会让 Dagster job 内部再调用 Dagster CLI，排障和权限边界都不清晰。

建议实现为：

1. `backfill__fetch_sources_to_raw_controller` 是普通 Dagster op，暴露 typed config。
2. Controller 根据 `target_scope` 从策略注册表生成 `BackfillPlan`。
3. `dry_run=true` 时只记录 `BackfillPlan`。
4. `dry_run=false` 时通过 Dagster instance/run submission 能力提交真实 asset materialization runs，并写入统一 tags。
5. 每个子 run 只选择一个自然阶段：snapshot prerequisite、snapshot target、daily source range、year source、compacted source、OCR step 或 raw sync。
6. 任一子 run 失败后，controller 标记失败，用户通过 `backfill.id` 查找已完成和失败的子 runs。

如果首期实现受限，允许临时保留 CLI launch 作为兼容路径，但 RFC 的目标形态应是 Dagster 内部提交 run，而不是长期 shell-out。

## 边界与失败处理

- Controller 必须先校验 `start_date <= end_date`。
- `target_scope` 不支持的资产不得用模糊匹配兜底，必须显式报错。
- 当前日期之后的区间应拒绝执行，除非对应 source asset 已有明确未来日期语义。
- Snapshot prerequisite 失败时，不应继续执行依赖它的区间回填。
- Daily source 失败时，不应继续执行对应 year 的 compacted source 与 raw sync。
- Compacted source 失败时，不应继续执行对应 year raw sync。
- Raw sync 失败时，保留已完成的 source/compacted materialization，用户后续可用同一 `target_scope`、同一区间和 `execution_mode=raw_only` 只重跑 raw 阶段。
- OCR pipeline 不应默认并入 `all_raw_yearly`，因为它不是 year 分区链路；需要通过 `jiuyan_ocr_pipeline` 或 `all_fetch_sources_to_raw` 执行。
- `all_fetch_sources_to_raw` 包含 OCR pipeline 时，必须显式记录 `jiuyan_ocr_limit`、`jiuyan_force_download` 和 `jiuyan_force_ocr` 的实际值；默认不应无限制触发全量 OCR。

## 预期收益

- 用户从“记住多条 `dg launch` 命令和 asset key”变成“选择 target scope + 填日期区间”。
- 回填运行事实统一通过 `backfill.id` 串起来，便于 Web UI 检索、失败重试和运行报告沉淀。
- 策略注册表把 snapshot、year、daily-to-compacted-to-raw 的差异显式化，避免每个 source 继续复制一套 controller。
- BaoStock 现有 year range controller 可以迁移为统一 controller 的一个 `target_scope`，减少专用回填代码。

## 当前结论与待确认问题

### 已形成草案结论

| 问题 | 当前结论 |
| --- | --- |
| 回填入口按 source bundle、raw table、数据域还是 partition strategy 收敛 | 首期 Web UI 入口按数据域 `target_scope` 收敛。source bundle、raw table 和 partition strategy 只作为策略注册表内部字段，不直接暴露给用户。 |
| daily 到 compacted 到 raw 的三段链路是否需要统一 backfill controller | 需要。BaoStock 日 K、Jiuyan action field 和 THS limit up pool 统一归入 `daily_to_compacted_to_raw` 策略，由 `backfill__fetch_sources_to_raw_controller` 生成多阶段计划。 |
| snapshot raw 与 year raw 是否应该拆分不同操作语义 | 应该拆分。Snapshot 走 `snapshot_to_raw`，不使用日期做 partition selection；year source/raw 走 `year_source_to_raw`，由日期区间映射到 year partitions。 |
| source-only 中间资产是否应该暴露更明确的运行状态和重跑边界 | 首期通过 `target_scope` 和阶段 tags 暴露。BaoStock/Jiuyan/THS daily source 归入各自 compacted 链路；Jiuyan `industry_images`、`industry_ocr` 归入 `jiuyan_ocr_pipeline`，并通过 OCR 专用 config 控制范围。 |

### 仍待确认

| 问题 | 待确认点 |
| --- | --- |
| `docs/jobs/dagster-backfill-2026.md` 是否需要重写 | 需要在 controller 实现和一次 dry run/真实回填验证后重写，避免 runbook 先于实际能力落地。 |
| 首期实现直接使用 Dagster 内部 run submission，还是短期兼容 CLI launch | RFC 倾向内部 run submission，但要在实现阶段确认本地 OSS Dagster instance、权限、run coordinator 和 Web UI 可观测性是否满足需求。 |
| 是否需要在 `execution_mode=raw_only` 之外扩展恢复模式 | 当前只确定 `full` 和 `raw_only`。`source_only`、`resume_from_failed_step` 是否值得做，需等首期失败恢复体验和运行报告确认。 |

## 验证记录

已执行：

```bash
cd pipeline/scheduler
uv run dg list defs --json
```

补充说明：一次并发执行的 `dg list defs --json` 在 dbt `.local_defs_state` 清理时发生 `FileNotFoundError`，同批另一次 `dg list defs --json` 成功返回完整 definitions JSON。本文清单同时用源码聚合与 `ENABLED_CLICKHOUSE_RAW_TABLE_SPECS` 交叉核对。

## 实施记录

2026-06-30 已按 [Plan 0065](../../plans/archive/0065-source-raw-unified-backfill-controller-implementation-plan.md) 落地首期统一入口：

- 注册 `backfill__fetch_sources_to_raw_job` 和 `backfill__fetch_sources_to_raw_controller`。
- `target_scope` registry 覆盖当前 22 个 source assets 和 17 个 raw assets，并有单元测试防漂移。
- `dry_run=true` 会输出结构化 `BackfillPlan`。
- `dry_run=false` 在本地 OSS Dagster 中使用 in-process child run submitter，通过 implicit asset job 生成真实 asset materialization runs，并保留 `backfill.*` tags；不使用 `dg launch` shell-out。
- `baostock__history_k_data_year_range_backfill_job` 已停止注册，BaoStock 日 K 历史区间回填由 `target_scope=baostock_daily_kline` 覆盖。
- 成功真实验收见 [2026-06-30 Source/Raw 统一回填 Controller 验证记录](../../jobs/reports/2026-06-30-source-raw-unified-backfill-controller.md)：`target_scope=chinabond`、`2006` 年 full 模式完成 source child run 和 raw child run，ClickHouse raw 分区返回 214 行。
