# Plan 0042: ChinaBond government bond S3 raw implementation plan

日期：2026-06-16

状态：Proposed

关联文档：

- `docs/references/remote_endpoint/chinabond__government_bond_yield_curve.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0005-dagster-owns-clickhouse-raw-sync-dbt-owns-modeling.md`
- `docs/ADR/0007-dbt-staging-cleaning-boundary.md`
- `docs/ADR/0008-raw-source-profiling-before-dbt-staging.md`
- `docs/plans/archive/0017-dagster-clickhouse-raw-sync-implementation-plan.md`
- `docs/plans/0023-contract-driven-parquet-schema-adapter-backfill-test-plan.md`
- `docs/systems/data-platform.md`
- `docs/architecture/scheduler-module-boundaries.md`

Context7 Dagster 文档核验点：

- 分区资产应通过 Dagster partition key 驱动幂等分区处理。
- 大范围回填可采用每分区一个 run 的 multi-run backfill，以获得更好的失败隔离和分区级可观测性。
- 单 run backfill 需要显式处理 partition range；本数据集只有 21 个年度分区，优先使用每年一 run。

## 1. 背景

`chinabond__government_bond` 远端接口已完成探测。接口为公开 GET 接口，固定参数 `gjqx=0&locale=cn_ZH&qxmc=1` 可返回中债国债收益率曲线全期限历史数据。

当前探测事实：

| 项 | 结论 |
| --- | --- |
| 最早数据日期 | `2006-03-01` |
| 初始全量窗口 | `2006` 到 `2026` |
| 分区策略 | 按自然年分区 |
| 2026 初始截止 | 回填执行日，当前计划基线为 `2026-06-16` |
| 历史全量行数基线 | 截至 `2026-06-16` 合计 5,075 行 |
| 单次请求窗口 | 生产采集遵守一年以内窗口，按自然年请求 |

本计划定义如何把 ChinaBond 国债收益率曲线接入 Dagster，写入 S3 Parquet source/raw 事实源，并进入 ClickHouse raw 同步准入。计划只写实施步骤，不包含代码实现。

命名约定：

- 数据集、Dagster asset、dbt source、ClickHouse raw table 和 job 均使用短名 `chinabond__government_bond`。
- 已有远端接口调研文档仍保留文件名 `docs/references/remote_endpoint/chinabond__government_bond_yield_curve.md`，作为接口事实参考。

## 2. 目标

完成后应满足：

1. 新增 ChinaBond 国债收益率曲线数据契约，作为字段、Parquet schema 和 ClickHouse raw schema 的事实源。
2. 新增 Dagster 年分区 source asset，将 `2006` 到当前年分区写入 S3 Parquet。
3. S3 对象布局符合现有约定：

```text
source/chinabond__government_bond/year=YYYY/000000_0.parquet
```

4. 年度分区可单独重跑；重跑同一年应覆盖该年 Parquet 分区，不能追加重复数据。
5. 初始回填覆盖 `2006` 到 `2026` 全部可用数据。
6. ClickHouse raw sync spec 可从契约生成或注册，后续同步到 `clickhouse/raw/chinabond__government_bond`。
7. 回填完成后有 `docs/jobs/reports/` 运行报告，记录命令、Run ID、分区范围、行数和校验结果。
8. 规划 dbt staging 层 `stg_chinabond__government_bond`，明确 raw profiling、字段治理、测试和下游建模边界。

## 3. 非目标

- 不在本计划中编写代码。
- 不引入 A 股交易日历作为完整性判断；该数据以 ChinaBond 接口返回的 `workTime` 为事实日期。
- 不在 dbt staging 中重写 raw 层采集、请求、解析或分区替换逻辑。
- 不把收益率字段保留为业务层字符串；source 原始字段可记录在 contract `source.fields`，Parquet/raw 字段应使用规范化数值列。
- 不把 `fifteenYear`、`twentyYear` 因当前样本多为 null 而删掉；它们必须保留 nullable 字段。
- 不在 full backfill 前跳过单年小批量验证。
- 不在没有 contract schema 的情况下让 `S3IOManager` 写入生产路径。

## 4. 数据集设计

### 4.1 字段口径

推荐 Parquet 和 raw 字段：

| 字段 | Parquet 类型 | ClickHouse 类型 | 说明 |
| --- | --- | --- | --- |
| `year` | `int32` | `UInt16` | 分区年，从 `work_date` 派生 |
| `work_date` | `date32[day]` | `Date` | 曲线日期 |
| `curve_name` | `string` | `LowCardinality(String)` | 曲线名称，当前为中债国债收益率曲线 |
| `three_month_yield_pct` | `double` | `Float64` | 3 个月收益率，单位为百分比点 |
| `six_month_yield_pct` | `double` | `Float64` | 6 个月收益率，单位为百分比点 |
| `one_year_yield_pct` | `double` | `Float64` | 1 年收益率，单位为百分比点 |
| `two_year_yield_pct` | `double` | `Float64` | 2 年收益率，单位为百分比点 |
| `three_year_yield_pct` | `double` | `Float64` | 3 年收益率，单位为百分比点 |
| `five_year_yield_pct` | `double` | `Float64` | 5 年收益率，单位为百分比点 |
| `seven_year_yield_pct` | `double` | `Float64` | 7 年收益率，单位为百分比点 |
| `ten_year_yield_pct` | `double` | `Float64` | 10 年收益率，单位为百分比点 |
| `fifteen_year_yield_pct` | `double` nullable | `Nullable(Float64)` | 15 年收益率 |
| `twenty_year_yield_pct` | `double` nullable | `Nullable(Float64)` | 20 年收益率 |
| `thirty_year_yield_pct` | `double` | `Float64` | 30 年收益率，单位为百分比点 |

转换规则：

- `workTime` 转为 `work_date`。
- `qxmc` 转为 `curve_name`。
- 收益率字符串先 trim；空字符串、空格和 JSON null 统一转为 null。
- 非空收益率转为百分比点数值，不除以 100。
- 输出按 `work_date` 升序排序，保证 Parquet 和 raw 重跑结果稳定。
- 同一年内 `work_date` 必须唯一；重复日期应让 run 失败并记录异常 metadata。

### 4.2 Contract 设计

新增 contract：

```text
pipeline/contracts/datasets/chinabond__government_bond.yml
```

关键约束：

- `source_asset_key`: `["source", "chinabond__government_bond"]`
- `raw_asset_key`: `["clickhouse", "raw", "chinabond__government_bond"]`
- `external.provider`: `chinabond`
- `source.protocol`: `http`
- `source.payload_format`: `json`
- `parquet.storage_mode`: `partitioned`
- `clickhouse_raw.database`: `fleur_raw`
- `clickhouse_raw.table`: `chinabond__government_bond`
- `clickhouse_raw.partition_strategy`: `year`
- `clickhouse_raw.partition_by`: `toYYYY(work_date)`
- `clickhouse_raw.order_by`: `["work_date"]`
- `clickhouse_raw.allow_empty`: `false` for historical years and current-year normal refresh
- `clickhouse_raw.sync_enabled`: `true` only after S3 small batch passes

完成 contract 后必须通过生成器同步：

- `pipeline/elt/models/sources.yml`
- `docs/references/data_dict/chinabond__government_bond.md`
- generated Parquet schema used by scheduler contract schema module

## 5. Dagster source 设计

### 5.1 资产边界

新增 source bundle：

```text
pipeline/scheduler/src/scheduler/defs/sources/chinabond/
```

建议资产：

- asset key: `source/chinabond__government_bond`
- group: `s3_sources`
- storage: `s3_io_manager`
- partitions: year partitions, start `2006`, end offset includes current year
- backfill policy: multi-run, max one partition per run
- pool: `chinabond_run_pool`
- tags: `source=chinabond`, `layer=source`, `storage=s3`
- kinds: `s3`, `parquet`, `http`

资产不依赖 `sina__trade_calendar`。ChinaBond 自有 `workTime` 是唯一日期事实源。

### 5.2 请求窗口

每个年度 partition 的请求窗口：

| partition | startDate | endDate |
| --- | --- | --- |
| `2006` | `2006-03-01` | `2006-12-31` |
| `2007` 到 `2025` | `YYYY-01-01` | `YYYY-12-31` |
| `2026` 初始回填 | `2026-01-01` | 回填执行日，当前为 `2026-06-16` |
| 当前年日常刷新 | `YYYY-01-01` | 运行日 |

固定 query parameters：

```text
gjqx=0
locale=cn_ZH
qxmc=1
```

建议 headers：

```text
User-Agent: Mozilla/5.0
Accept: application/json,text/plain,*/*
```

### 5.3 业务错误处理

接口可能 HTTP 200 但返回 `flag=1, heList=null`。处理规则：

- 完全早于 `2006-03-01` 的窗口、未来窗口、纯空日期区间可以视为空结果，但本资产正常分区不会主动请求这些窗口。
- `2006` 到当前年目标窗口返回空时应失败，不能写空 Parquet 覆盖有效分区。
- 非 2xx、超时、JSON 解析失败、业务结构缺失应按现有 HTTP retry 策略重试；重试耗尽后让 Dagster run 失败。
- 响应编码异常导致中文曲线名称退化为问号时，应记录并失败，避免写入不可逆脏值。

### 5.4 Materialization metadata

每个分区至少记录：

- `source_url_host`
- `request_start_date`
- `request_end_date`
- `partition_key`
- `row_count`
- `min_work_date`
- `max_work_date`
- `curve_name_count`
- `business_flag`
- `s3_key`
- `contract_dataset`
- `contract_schema_hash`
- `parquet_schema_hash`

禁止记录 access key、secret、完整带敏感参数的连接串或 Cookie。

## 6. ClickHouse raw 同步设计

本阶段只在 source/S3 小批量通过后启用。

目标 raw asset：

```text
clickhouse/raw/chinabond__government_bond
```

同步策略：

- 从对应 S3 年分区 Parquet 读取。
- ClickHouse `fleur_raw.chinabond__government_bond` 使用 MergeTree。
- `PARTITION BY toYYYY(work_date)`。
- `ORDER BY work_date`。
- 按年度 staging + partition replace，同年重跑覆盖该年 raw 分区。
- 不使用 `ALTER TABLE UPDATE/DELETE` 修补历史。

raw 同步准入：

- contract 已生成 data_dict 和 dbt source。
- `validate-parquet` 能识别所有已写入分区。
- 小批量 `2026` source/S3 通过。
- ClickHouse raw sync dry run 或 dev 小批量通过后，再把 `sync_enabled` 固定为 true。

## 7. 实施阶段

### Phase 0: 命名和 contract 前置

实施内容：

- 确认最终 dataset 名称：默认 `chinabond__government_bond`。
- 新增 contract，覆盖 source、Parquet 和 ClickHouse raw 字段。
- 运行 contract 生成和校验，确保 generated schema 被 scheduler 使用。
- 补充或确认 data_dict 中中文字段描述、单位和 nullable 口径。

完成标准：

- `fleur-contracts validate` 通过。
- `fleur-contracts generate --check` 通过。
- `docs/references/data_dict/chinabond__government_bond.md` 已生成并审阅。

### Phase 1: Source asset 和解析服务设计

实施内容：

- 新增 `chinabond` source bundle。
- 复用现有 HTTP resource/client factory，不在 definitions 加载阶段发起网络请求。
- 实现年度请求窗口构造。
- 实现 JSON parser、字段规范化、日期唯一性校验和 pyarrow table 构造。
- 资产返回 `dict[str, pyarrow.Table]`，key 必须与 Dagster partition key 完全一致。
- metadata 使用 `year_partition_metadata(partition_key_name="year")`。

完成标准：

- definitions 能加载该 source asset、job、schedule 和资源依赖。
- `S3IOManager` 能用 contract Parquet schema 校验输出。
- 单元测试覆盖 2006 首年窗口、完整历史年窗口、当前年窗口、空字符串收益率、null 收益率、重复 `workTime` 和 `flag=1`。

### Phase 2: 小批量 S3 写入验证

先跑 2026 单分区，验证当前年窗口、nullable 字段和 S3 写入：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/chinabond__government_bond" \
  --partition 2026
```

建议再跑 2006 单分区，验证首年 `2006-03-01` 起始边界：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:source/chinabond__government_bond" \
  --partition 2006
```

完成标准：

- 两个 Dagster runs 成功。
- S3 路径中存在 `year=2026/000000_0.parquet` 和 `year=2006/000000_0.parquet`。
- 2026 行数接近 endpoint 参考基线；执行日不同导致行数增长时，报告中记录实际截止日。
- 2006 最小日期为 `2006-03-01`。
- `S3IOManager` 没有 schema mismatch。

### Phase 3: 初始全量 S3 回填

全量回填范围：

```text
2006,2007,2008,2009,2010,2011,2012,2013,2014,2015,2016,2017,2018,2019,2020,2021,2022,2023,2024,2025,2026
```

执行策略：

- 使用每年一个 run。
- 不使用单 run partition range。
- 若某一年失败，只重跑失败年份及必要下游，不重刷已成功年份。
- 当前年 `2026` 可在小批量成功后重跑一次，确保全量报告使用同一执行窗口。

命令模板：

```bash
cd pipeline
for year in 2006 2007 2008 2009 2010 2011 2012 2013 2014 2015 2016 2017 2018 2019 2020 2021 2022 2023 2024 2025 2026; do
  uv run dg launch --target-path scheduler \
    --assets "key:source/chinabond__government_bond" \
    --partition "$year"
done
```

完成标准：

- 21 个年度分区全部成功。
- 每个分区均有非空 Parquet。
- 年度行数与 `docs/references/remote_endpoint/chinabond__government_bond_yield_curve.md` 基线一致，或因当前年执行日变化有合理说明。
- 全部分区合计行数在报告中记录；若以 `2026-06-16` 为截止日，应为 5,075 行。

### Phase 4: ClickHouse raw 小批量同步

Source/S3 全量不必等待 ClickHouse raw 启用；但 raw 同步启用前必须至少完成 `2006` 和 `2026` 小批量。

命令模板：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/chinabond__government_bond" \
  --partition 2026

uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/chinabond__government_bond" \
  --partition 2006
```

完成标准：

- staging 表创建、S3 Parquet 读取和 partition replace 成功。
- `fleur_raw.chinabond__government_bond` 中 `2006`、`2026` 分区行数与 S3 分区一致。
- `work_date` 唯一性和 min/max 日期与 source metadata 一致。

### Phase 5: ClickHouse raw 全量同步

S3 全量完成且 raw 小批量通过后，按年同步 raw：

```bash
cd pipeline
for year in 2006 2007 2008 2009 2010 2011 2012 2013 2014 2015 2016 2017 2018 2019 2020 2021 2022 2023 2024 2025 2026; do
  uv run dg launch --target-path scheduler \
    --assets "key:clickhouse/raw/chinabond__government_bond" \
    --partition "$year"
done
```

完成标准：

- 21 个 raw 分区全部同步成功。
- raw 总行数与 S3 总行数一致。
- 不出现重复 `work_date`。
- contract ClickHouse 校验通过。

### Phase 6: 调度和日常刷新

日常调度建议：

- 每日北京时间傍晚刷新当前年分区。
- 只调度当前年 source 分区及其 raw sync 下游。
- 历史年份不自动重刷；如发现上游历史修正，按年手动重刷受影响年份。

调度完成标准：

- 当前年 schedule 能生成正确 partition key。
- schedule 不会触发历史全量。
- 失败告警能定位到具体年份分区。

### Phase 7: dbt staging 层设计

本阶段只规划 `chinabond__government_bond` 从 raw 到 staging 的第一层建模，不在 staging 中重新采集、解析接口，也不改变数据粒度。

目标模型：

```text
pipeline/elt/models/staging/chinabond/stg_chinabond__government_bond.sql
pipeline/elt/models/staging/chinabond/stg_chinabond__government_bond.yml
```

模型约定：

- dbt model 名称：`stg_chinabond__government_bond`
- dbt source：`{{ source('raw', 'chinabond__government_bond') }}`
- materialization：沿用 `dbt_project.yml` staging 默认配置，落 `fleur_staging` view
- grain：每个 ChinaBond 国债收益率曲线 `trade_date` 一行，来自 raw `work_date`
- natural key：`trade_date`
- 字段策略：raw 字段已是 canonical snake_case，staging 第一版以显式 select 为主；`work_date` 在 staging 中改名为 `trade_date`，`curve_name` 不输出
- 排序策略：staging view 不依赖物理排序；下游如需时间序列顺序，在查询或 intermediate 中显式 `order by trade_date`

前置 raw profiling：

- 新增 `docs/references/raw_profile/chinabond__government_bond.md`，按 `_template.md` 记录 profiling 证据。
- profiling 范围至少覆盖 2006-2026 全量 raw 数据：总行数、日期范围、年度分区行数、`work_date` 唯一性、`curve_name` 枚举核验、各期限收益率 null rate 和 min/max。
- profiling 需要确认 `fifteen_year_yield_pct`、`twenty_year_yield_pct` 的缺失是否为上游自然缺失；不得在 staging 中填补。
- 若发现重复 `work_date`，staging 不做静默去重，应先回到 raw 分区或上游采集链路定位。

Staging 输出字段：

| 字段 | 类型 | 口径 | YAML metadata |
| --- | --- | --- | --- |
| `trade_date` | `Date` | 中债曲线日期，来自 raw `work_date` | `dictionary_scope: local`，记录 source column |
| `three_month_yield_pct` | `Nullable(Float64)` | 3 个月收益率，百分比点，不除以 100 | `dictionary_scope: local`，`unit: percent`，`scale: percent_value_not_fraction` |
| `six_month_yield_pct` | `Nullable(Float64)` | 6 个月收益率，百分比点，不除以 100 | 同上 |
| `one_year_yield_pct` | `Nullable(Float64)` | 1 年收益率，百分比点，不除以 100 | 同上 |
| `two_year_yield_pct` | `Nullable(Float64)` | 2 年收益率，百分比点，不除以 100 | 同上 |
| `three_year_yield_pct` | `Nullable(Float64)` | 3 年收益率，百分比点，不除以 100 | 同上 |
| `five_year_yield_pct` | `Nullable(Float64)` | 5 年收益率，百分比点，不除以 100 | 同上 |
| `seven_year_yield_pct` | `Nullable(Float64)` | 7 年收益率，百分比点，不除以 100 | 同上 |
| `ten_year_yield_pct` | `Nullable(Float64)` | 10 年收益率，百分比点，不除以 100 | 同上 |
| `fifteen_year_yield_pct` | `Nullable(Float64)` | 15 年收益率，百分比点，允许 null | 同上 |
| `twenty_year_yield_pct` | `Nullable(Float64)` | 20 年收益率，百分比点，允许 null | 同上 |
| `thirty_year_yield_pct` | `Nullable(Float64)` | 30 年收益率，百分比点，不除以 100 | 同上 |

不把 raw 分区字段 `year` 或曲线名称字段 `curve_name` 作为 staging 输出字段。需要年度过滤时，下游使用 `toYear(trade_date)` 或在 intermediate/mart 明确派生，避免把存储分区字段提升为业务字段。

测试设计：

- `trade_date`：`not_null`、`unique`。
- 核心期限收益率：根据 profiling 结果对稳定非空字段加 `not_null`。初始候选为 3 个月、6 个月、1 年、2 年、3 年、5 年、7 年、10 年、30 年。
- 15 年、20 年收益率：第一版不加 `not_null`，只记录 nullable 事实和缺失分布。
- 数值范围：不假设已有通用 range test。若 profiling 发现异常值风险，新增专用 singular test 或 generic test，例如收益率百分比点落在合理区间内，并在计划后续阶段单独实现。

字段治理：

- `trade_date` 和收益率期限列第一版作为 `dictionary_scope: local` 字段维护；`trade_date` 虽使用统一日期列名，但语义来自 ChinaBond 曲线工作日，不复用 A 股交易日历口径。
- 每个 staging YAML column 必须声明 `description`、`data_type`、`config.meta.source_columns`。
- 收益率列 metadata 必须记录 `unit: percent` 和 `scale: percent_value_not_fraction`，防止下游误把 `_yield_pct` 除以 100。
- 如果后续利率曲线建模需要跨 source 复用日期或期限字段，再新增全局 glossary key 和 docs block，不在第一版 staging 中提前抽象。

SQL 结构建议：

```sql
with source as (
    select
        work_date,
        three_month_yield_pct,
        six_month_yield_pct,
        one_year_yield_pct,
        two_year_yield_pct,
        three_year_yield_pct,
        five_year_yield_pct,
        seven_year_yield_pct,
        ten_year_yield_pct,
        fifteen_year_yield_pct,
        twenty_year_yield_pct,
        thirty_year_yield_pct
    from {{ source('raw', 'chinabond__government_bond') }}
)

select
    work_date as trade_date,
    three_month_yield_pct,
    six_month_yield_pct,
    one_year_yield_pct,
    two_year_yield_pct,
    three_year_yield_pct,
    five_year_yield_pct,
    seven_year_yield_pct,
    ten_year_yield_pct,
    fifteen_year_yield_pct,
    twenty_year_yield_pct,
    thirty_year_yield_pct
from source
```

延后到 intermediate/mart：

- 曲线期限宽表转长表。
- 利率期限结构指标、期限利差、斜率、曲率和交易信号。
- 百分比点到小数比例的派生别名。
- 与交易日历、证券行情或宏观数据的 join。
- 对缺失期限收益率的插值、补点或跨期限推断。

完成标准：

- raw profile 文档完成并能支撑 tests 选择。
- `stg_chinabond__government_bond.sql` 和 YAML 只依赖 `source('raw', 'chinabond__government_bond')`。
- staging 输出 12 个业务字段，不输出 raw 存储分区字段 `year` 或 raw 曲线名称字段 `curve_name`。
- YAML column metadata 满足 `validate_field_glossary.py`。
- dbt build 只选本模型时通过，且 `trade_date` 唯一性测试通过。

### Phase 8: dbt intermediate 日频收益率设计

目标模型：

```text
pipeline/elt/models/intermediate/int_government_bond_yields_daily.sql
pipeline/elt/models/intermediate/int_government_bond_yields_daily.yml
```

模型约定：

- dbt model 名称：`int_government_bond_yields_daily`
- 上游模型：`{{ ref('stg_chinabond__government_bond') }}`
- materialization：沿用 `dbt_project.yml` intermediate 默认配置，落 `fleur_intermediate` view
- grain：每个 ChinaBond 国债收益率曲线 `trade_date` 一行
- natural key：`trade_date`
- 字段策略：完整透传 `stg_chinabond__government_bond` 的 12 个输出字段，不新增、不删除、不改单位
- 单位策略：收益率继续保留百分比点口径，不转换为小数比例
- nullable 策略：15 年、20 年期限收益率当前上游全为空，intermediate 保留 nullable 字段

测试设计：

- `trade_date`：`not_null`、`unique`。
- 核心期限收益率：3 个月、6 个月、1 年、2 年、3 年、5 年、7 年、10 年、30 年加 `not_null`。
- 15 年、20 年收益率：不加 `not_null`，保留当前缺失事实。
- 完整性 singular test：比较 `stg_chinabond__government_bond` 与 `int_government_bond_yields_daily` 的 `trade_date` 集合，任一方向缺失都失败。

延后处理：

- 宽表转期限长表。
- 期限利差、斜率、曲率、插值和期限结构信号。
- 百分比点到小数比例的派生列。
- 与交易日历、行情、宏观或其他利率曲线数据的 join。

完成标准：

- `dbt build --select +int_government_bond_yields_daily` 通过。
- intermediate 行数与 staging 行数一致，当前应为 5,075 行。
- `min(trade_date)`、`max(trade_date)` 与 staging 一致，当前应为 `2006-03-01` 到 `2026-06-16`。
- 完整性 singular test 无返回行。

## 8. 验证命令

Contract 和生成物：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run python elt/scripts/validate_field_glossary.py
```

Scheduler 质量门禁：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run ruff format --check scheduler/src scheduler/tests contract_tools/src contract_tools/tests
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests -q
cd scheduler
uv run dg check defs
```

dbt 解析：

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
```

dbt staging 实施后校验：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select stg_chinabond__government_bond
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
```

dbt intermediate 实施后校验：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select +int_government_bond_yields_daily
uv run dbt show --project-dir elt --profiles-dir elt --inline "select 'stg' as layer, count(*) as row_count, min(trade_date) as min_trade_date, max(trade_date) as max_trade_date from {{ ref('stg_chinabond__government_bond') }} union all select 'int' as layer, count(*) as row_count, min(trade_date) as min_trade_date, max(trade_date) as max_trade_date from {{ ref('int_government_bond_yields_daily') }}"
```

Parquet 和 ClickHouse 校验：

```bash
cd pipeline
uv run fleur-contracts validate-parquet --all-available
uv run fleur-contracts validate-clickhouse --all-available
```

文档校验：

```bash
make docs-check
git diff --check
```

## 9. 回填报告

全量回填完成后新增报告：

```text
docs/jobs/reports/YYYY-MM-DD-chinabond-government-bond-yield-curve-backfill.md
```

报告至少包含：

- 执行环境和 UTC 时间窗口。
- 使用的数据集名和 asset key。
- Source/S3 小批量 Run ID：`2006`、`2026`。
- Source/S3 全量 Run ID 列表：`2006` 到 `2026`。
- ClickHouse raw 小批量和全量 Run ID 列表。
- 每年 source 行数、raw 行数、min/max `work_date`。
- 合计行数。
- `validate-parquet` 和 `validate-clickhouse` 输出摘要。
- 失败分区、修复方式和补跑命令。

如果小批量或全量任一分区失败，应先新增失败报告，再修复和重跑：

```text
docs/jobs/reports/YYYY-MM-DD-chinabond-government-bond-yield-curve-failure.md
```

## 10. 风险和处置

| 风险 | 处置 |
| --- | --- |
| 接口返回 HTTP 200 但 `flag=1` | 对目标年份视为失败；记录 partition、请求窗口和响应摘要 |
| 中文编码退化为问号 | 保留浏览器 UA 和 JSON/text Accept；检测 `curve_name` 是否异常 |
| 当前年行数随执行日变化 | 报告中记录 `request_end_date`，不要和固定 2026-06-16 基线硬比 |
| 15 年、20 年期限未来补齐 | contract 已保留 nullable 数值列，schema 不变 |
| 上游修正历史年份 | 按受影响年份重刷 source 分区，再重刷 raw 分区 |
| dataset 命名变更 | 实施前一次性确认并同步所有 contract、asset、raw、dbt、报告命名 |

## 11. 禁止模式

- 禁止跳过 contract 直接写 S3。
- 禁止把空年度响应写成空 Parquet 覆盖目标年份。
- 禁止用 A 股交易日历补全或过滤 ChinaBond 日期。
- 禁止把收益率百分比点除以 100 后写入 `_yield_pct` 字段。
- 禁止在 raw 层用 mutation 修补年度数据；应重刷 S3 年分区并替换 raw 年分区。
- 禁止把 secret、Cookie 或完整敏感 URL 写入 materialization metadata 或 job report。
- 禁止 full backfill 早于 `2006` 和 `2026` 小批量成功。

## 12. 完成标准

计划完成时应有：

- contract、data_dict、dbt source 和 generated Parquet schema 全部一致。
- `source/chinabond__government_bond` 年分区 asset 可在 Dagster definitions 中加载。
- S3 中存在 `2006` 到 `2026` 的 21 个年度 Parquet 分区。
- ClickHouse raw 中存在对应 21 个年度分区，若本轮启用 raw sync。
- 回填报告已写入 `docs/jobs/reports/`。
- 若推进 staging，`docs/references/raw_profile/chinabond__government_bond.md`、`stg_chinabond__government_bond.sql` 和 YAML 已按 Phase 7 完成。
- `fleur-contracts validate-parquet --all-available` 通过。
- `fleur-contracts validate-clickhouse --all-available` 通过，若本轮启用 raw sync。
- `ruff`、`pyright`、`pytest`、`dg check defs`、`dbt parse` 通过。
- 若推进 staging，`dbt build --select stg_chinabond__government_bond`、`validate_staging_readiness.py` 和 `validate_field_glossary.py` 通过。
