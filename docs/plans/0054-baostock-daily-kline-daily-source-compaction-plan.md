# Plan 0054: BaoStock 日 K 日分区采集与年度压缩改造

日期：2026-06-25

状态：Proposed

## 背景

BaoStock 官方当前对登录态做了限制，日 K 抓取不能通过频繁新建登录会话来扩吞吐。现有 `source/baostock__query_history_k_data_plus_daily` 资产按年分区运行，每个年份对所有有效证券执行一次日期范围请求；此前降到单连接后，补 2026 年分区已经表现为长时间单 step 运行，不适合作为日常增量路径。

本次优化目标不是恢复无限并发，而是采用固定 4 连接池：每个 TCP 连接创建后必须独立完成 BaoStock login，请求只允许在已登录连接上执行。当前代码中的登录态在 `BaostockAioTcpClient` 对象级别维护，连接池大于 1 时存在只有首个连接完成登录、后续连接复用 client 级登录态的实现风险；实施时必须改为连接级登录状态或连接创建后 login-on-create。

本计划采用一刀切方案，不保留旧年分区 source 兼容层：

1. 将 dev 环境 S3 中现有 `source/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet` 迁移到 `source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet`。
2. 将原 `baostock__query_history_k_data_plus_daily` 改成 `trade_date` 日分区 source asset，每个交易日只获取当天日 K。
3. 新增 `baostock__query_history_k_data_plus_daily_compacted` 年分区 source asset，将日分区日 K 聚合成年分区，参考 `ths__limit_up_pool_compacted` 和 `jiuyan__action_field_compacted`。
4. 日 K 证券范围从 BaoStock `type in {"1", "2", "5"}` 改为只采集 `{"1", "2"}`，不再获取 ETF `type="5"`。

## 目标

- BaoStock 日 K 日常调度只刷新当天交易日分区，避免每个交易日重刷全年。
- ClickHouse raw 同步只消费年度压缩资产 `_compacted`，继续保持 raw 年分区替换语义。
- dbt staging 模型名 `stg_baostock__query_history_k_data_plus_daily` 保持不变，但其 raw source 切换到 `_compacted` raw 表。
- dev S3 历史年分区数据通过对象迁移进入 `_compacted` 路径，不通过 BaoStock 远端重新抓历史全量。
- 证券过滤和测试明确排除 `type="5"`。
- BaoStock 日 K 远端抓取使用固定 4 连接池，最多 4 个证券请求并发。
- 每个 BaoStock TCP 连接独立登录，未登录或登录失败的连接不得承载数据请求。

## 非目标

- 不保留旧 `source/baostock__query_history_k_data_plus_daily/year=...` 读取兼容。
- 不为 ETF `type="5"` 建新资产或补偿表。
- 不改变 BaoStock 响应字段、Parquet 字段、ClickHouse 字段和 dbt canonical 字段语义。
- 不重写 ClickHouse raw sync 通用框架；通过 contract 和 asset key 调整复用现有 raw sync。
- 不在本计划阶段执行数据补数或代码改造。本文件只定义实施方案。

## 当前事实盘点

### BaoStock source 资产

- [assets.py](../../pipeline/scheduler/src/scheduler/defs/baostock/assets.py) 当前定义：
  - `year_partitions = TimeWindowPartitionsDefinition(start="1990", fmt="%Y")`
  - `baostock__query_stock_basic` 最新快照资产。
  - `baostock__query_history_k_data_plus_daily` 年分区资产，`deps=[stock_basic, sina__trade_calendar]`，`io_manager_key="s3_io_manager"`。
- [services.py](../../pipeline/scheduler/src/scheduler/defs/baostock/services.py) 当前按年份构造 `year_ranges`，在单个 BaoStock client 中对每个证券日期范围发起 `query_history_k_data_plus_daily(code, start_date, end_date)`。
- [schedules.py](../../pipeline/scheduler/src/scheduler/defs/baostock/schedules.py) 当前 `baostock__daily_schedule` 使用交易日调度，但分区键是 `str(trade_date.year)`，并通过 `refresh_until_trade_date` 让年分区刷到当天。
- [definitions.py](../../pipeline/scheduler/src/scheduler/defs/baostock/definitions.py) 当前 bundle 只注册 stock basic 和年分区日 K 两个资产。

### BaoStock client 与连接池

- [client.py](../../pipeline/scheduler/src/scheduler/defs/baostock/client.py) 当前 `BaostockAioTcpClient` 支持 `max_connections`，并通过 semaphore 和 idle queue 管理连接池。
- `BaostockAioTcpClient` 当前使用 client 级 `_logged_in` 和 `_login_expires_at` 表示登录态，`start()` 只调用一次 `_ensure_logged_in(force=True)`。
- `_ensure_logged_in()` 当前通过 `_send_with_retries()` 借用连接发送 `login` 请求，因此连接池扩到 4 后不能保证每个连接都已登录。
- [services.py](../../pipeline/scheduler/src/scheduler/defs/baostock/services.py) 当前 `BAOSTOCK_DAILY_KLINE_CONNECTIONS = 1`，并同时用于 `client(max_connections=...)` 和 `BoundedTaskRunner(max_concurrent_tasks=...)`。

### 证券范围过滤

- [market/securities.py](../../pipeline/scheduler/src/scheduler/defs/market/securities.py) 当前：
  - `BAOSTOCK_SECURITY_TYPE_DATA_START_DATES = {"1": 1990-12-19, "2": 2006-01-01, "5": 2026-01-05}`
  - `filter_active_security_ranges(..., allowed_security_types=frozenset({"1", "2", "5"}))`
- EastMoney 已通过显式参数使用 `allowed_security_types=frozenset({"1"})`，因此全局默认改为 `{"1", "2"}` 前必须确认 EastMoney 不受影响。

### 已有日分区与 compacted 模式

- [sources/ths/limit_up_pool.py](../../pipeline/scheduler/src/scheduler/defs/sources/ths/limit_up_pool.py) 使用 `DailyPartitionsDefinition`、`materialize_trade_date_range()` 和 `trade_date` 分区写 S3。
- [sources/ths/limit_up_pool_compact.py](../../pipeline/scheduler/src/scheduler/defs/sources/ths/limit_up_pool_compact.py) 使用 `compact_daily_asset_by_year()` 将 `trade_date` 日分区压成年分区。
- [sources/jiuyan/action_field.py](../../pipeline/scheduler/src/scheduler/defs/sources/jiuyan/action_field.py) 与 [action_field_compact.py](../../pipeline/scheduler/src/scheduler/defs/sources/jiuyan/action_field_compact.py) 也采用同一模式。
- [sources/daily_compact.py](../../pipeline/scheduler/src/scheduler/defs/sources/daily_compact.py) 已实现：
  - 根据年分区和 Sina 交易日历计算当年 trade_date 分区。
  - 读取 daily asset 的 `trade_date=YYYY-MM-DD` 分区。
  - concat 后按输出 dataset contract schema 选择和 cast 字段。

### Contract 与 raw sync

- [baostock__query_history_k_data_plus_daily.yml](../../pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml) 当前同时承担 source Parquet 和 ClickHouse raw 字段事实，`parquet.partition_key_name = year`，`clickhouse_raw.partition_strategy = year`。
- [contract schema](../../pipeline/contract_tools/src/fleur_contracts/schema.py) 要求 `clickhouse_raw.partition_strategy = year` 时，`parquet.partition_key_name` 必须是 `year`。因此日分区 source 不能继续直接绑定 ClickHouse raw。
- [clickhouse/specs.py](../../pipeline/scheduler/src/scheduler/defs/clickhouse/specs.py) 从 contracts 生成 raw sync specs，当前 `BAOSTOCK_DAILY_K_SPEC` 精确寻找 `clickhouse_table == "baostock__query_history_k_data_plus_daily"`。
- [clickhouse/definitions.py](../../pipeline/scheduler/src/scheduler/defs/clickhouse/definitions.py) 当前 `clickhouse__raw_sync_baostock_job` 通过 source asset name 前缀 `baostock__query_history_k_data_plus_daily` 选择 BaoStock raw sync 资产。
- [raw_sync.py](../../pipeline/scheduler/src/scheduler/defs/clickhouse/raw_sync.py) 当前 year raw sync 只读取一个 `source_asset_key/year=YYYY/000000_0.parquet` 对象。

### dbt 与下游

- [sources.yml](../../pipeline/elt/models/sources.yml) 由 contract 生成，当前暴露 `source('raw', 'baostock__query_history_k_data_plus_daily')`，物理表为 `fleur_raw.baostock__query_history_k_data_plus_daily`。
- [stg_baostock__query_history_k_data_plus_daily.sql](../../pipeline/elt/models/staging/baostock/stg_baostock__query_history_k_data_plus_daily.sql) 当前直接读取 `source('raw', 'baostock__query_history_k_data_plus_daily')`。
- 下游 `int_stock_quotes_daily_unadj`、`int_index_quotes_daily` 依赖 staging model，不直接读 raw source。计划应保持 staging model 名称和字段输出不变。

## 目标形态

### S3 source layout

```text
# 日分区 source，只保存每日增量
source/baostock__query_history_k_data_plus_daily/trade_date=YYYY-MM-DD/000000_0.parquet

# 年分区 compacted，作为 ClickHouse raw sync 输入
source/baostock__query_history_k_data_plus_daily_compacted/year=YYYY/000000_0.parquet
```

迁移后删除 dev S3 旧路径下的 `year=YYYY` 分区：

```text
source/baostock__query_history_k_data_plus_daily/year=YYYY/000000_0.parquet
```

### Dagster assets

| Asset | 分区 | 角色 |
|---|---|---|
| `source/baostock__query_stock_basic` | none | 证券基础信息快照 |
| `source/baostock__query_history_k_data_plus_daily` | `trade_date` daily | 单交易日日 K 远端抓取 |
| `source/baostock__query_history_k_data_plus_daily_compacted` | `year` | 聚合 daily partitions，供 raw sync |
| `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` | `year` | 从 compacted S3 年分区同步到 ClickHouse raw |

### ClickHouse 与 dbt

- 新 raw 表：`fleur_raw.baostock__query_history_k_data_plus_daily_compacted`。
- `stg_baostock__query_history_k_data_plus_daily` 改为读取 `source('raw', 'baostock__query_history_k_data_plus_daily_compacted')`。
- staging model 名称、字段名、字段语义和下游 `ref()` 不变。
- 旧 raw 表 `fleur_raw.baostock__query_history_k_data_plus_daily` 在新链路验证后可清理，不作为兼容源保留。

## 实施阶段

### 阶段 1：Contract 拆分与生成链路

1. 将 `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily.yml` 改为 source-only daily contract：
   - `source_asset_key: ["source", "baostock__query_history_k_data_plus_daily"]`
   - `parquet.storage_mode: partitioned`
   - `parquet.partition_key_name: trade_date`
   - 移除 `raw_asset_key` 和 `clickhouse_raw`
   - 字段列表保持不变。
2. 新增 `pipeline/contracts/datasets/baostock__query_history_k_data_plus_daily_compacted.yml`：
   - `source.protocol: generated`
   - `source_asset_key: ["source", "baostock__query_history_k_data_plus_daily_compacted"]`
   - `raw_asset_key: ["clickhouse", "raw", "baostock__query_history_k_data_plus_daily_compacted"]`
   - `parquet.partition_key_name: year`
   - `clickhouse_raw.partition_strategy: year`
   - 字段、类型、nullable、`order_by(date, code)` 继承当前日 K contract。
3. 更新 `pipeline/contracts/glossary/tables.yml`，为 daily source 和 compacted raw 分别写表描述。
4. 重新生成 `pipeline/elt/models/sources.yml` 和 `docs/references/data_dict/*.md`。

完成标准：

- `fleur-contracts validate` 通过。
- `fleur-contracts generate --check` 通过。
- `scheduler.defs.contract_schemas.PARQUET_SCHEMAS` 同时包含 daily 和 compacted 两个 dataset。

### 阶段 2：BaoStock 日分区 source asset

1. 在 `assets.py` 中新增或替换日分区定义：
   - 使用 `DailyPartitionsDefinition(start_date="1990-12-19", timezone="Asia/Shanghai")`。
   - metadata 使用 `daily_sparse_partition_metadata(partition_key_name="trade_date", trade_date_filter=sina__trade_calendar)`。
   - `backfill_policy=dg.BackfillPolicy.single_run()`。
2. 将 `baostock__query_history_k_data_plus_daily` 改为日分区 asset：
   - 每个 `trade_date` 分区读取 stock basic 和 Sina trade calendar。
   - 非交易日通过 `materialize_trade_date_range()` 过滤，不写远端请求。
   - 对每个交易日只请求当天：`query_history_k_data_plus_daily(code, trade_date, trade_date)`。
   - BaoStock client 使用固定 4 连接池，`max_connections=4`，证券请求 `max_concurrent_tasks=4`。
   - 每个 TCP 连接创建后必须完成 BaoStock login；连接级登录状态绑定到连接，不再依赖 client 级单一 `_logged_in` 表示整个连接池已登录。
   - 登录失败的连接不可放回池中复用，必须关闭并从 pool 中移除；替换连接在进入可借用状态前必须先登录成功。
   - 数据请求不能共享首连接的登录态，不能在未登录连接上发送 `query_history_k_data_plus`。
   - 证券范围显式 `allowed_security_types=frozenset({"1", "2"})`。
3. 修复 [client.py](../../pipeline/scheduler/src/scheduler/defs/baostock/client.py) 的连接池登录语义：
   - 移除或降级 client 级 `_logged_in` / `_login_expires_at` 对连接池整体登录状态的判断。
   - 在 `_create_connection()` 后对新连接执行 login，或实现连接级 `ensure_logged_in(connection)`。
   - `NO_LOGIN_ERROR_CODE` 只刷新当前承载请求的连接；刷新失败时关闭该连接并创建已登录替换连接。
   - 单测覆盖 `max_connections=4` 时创建 4 条连接会产生 4 次 login payload，且每条连接的数据请求发生在本连接 login 成功之后。
4. 将原 `BaostockDailyKlineRefreshService` 拆成更清晰的职责：
   - daily fetch service：输入 trade date，输出单日 table。
   - compact 不访问 BaoStock，只读 S3 daily partitions。
   - 删除或停止使用年分区 `refresh_until_trade_date` 配置。

完成标准：

- materialization metadata 包含 `processed_trade_date_count`、`selected_security_count`、`selected_security_types=["1","2"]`、`max_connections=4`、`max_concurrent_security_requests=4`、`row_count`。
- 日分区 S3 路径为 `trade_date=YYYY-MM-DD`。
- 单测证明 `type="5"` 不会被选中。
- 单测证明 4 连接池下每个连接均完成独立 login，且无数据请求运行在未登录连接上。

### 阶段 3：新增 yearly compacted asset

1. 新增 `baostock__query_history_k_data_plus_daily_compacted` asset：
   - 参考 `ths__limit_up_pool_compacted`。
   - `partitions_def` 使用年分区，start 与日 K 历史范围对齐。
   - deps 包含 daily asset 和 `sina__trade_calendar`。
   - 使用 `compact_daily_asset_by_year()`。
   - `output_dataset="baostock__query_history_k_data_plus_daily_compacted"`。
   - metadata 使用 `compacted_year_metadata(input_partition_key_name="trade_date", input_asset=<daily asset>)`。
2. 新增 `baostock__query_history_k_data_plus_daily_compacted_job`。
3. `baostock_bundle` 注册三个 source assets 和两个 jobs：
   - `baostock__daily_job`：stock basic + daily K。
   - `baostock__query_history_k_data_plus_daily_compacted_job`：compacted only。
4. `baostock__daily_schedule` 分区键改为交易日 `YYYY-MM-DD`，不再传 `refresh_until_trade_date`。
5. compacted asset 可使用 eager automation，或先只提供手动 job；第一阶段建议与 THS/Jiuyan 一致使用 eager。

完成标准：

- `dg list defs --json` 能看到 daily source、compacted source、compacted job。
- compacted asset materialization 读取 daily partitions，并输出 `year=YYYY/000000_0.parquet`。

### 阶段 4：ClickHouse raw sync 切换到 compacted

1. 让 contract 生成新的 raw asset：`clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted`。
2. 更新 `clickhouse/specs.py` 中 BaoStock 日 K 特化常量：
   - 从 `BAOSTOCK_DAILY_K_SPEC` 指向旧 table，切到 compacted spec。
   - 测试命名可以保留概念名，但断言新 asset key 和新 S3 object key。
3. 更新 `clickhouse__raw_sync_baostock_job` 选择逻辑：
   - 明确选择 `source_prefixes=("baostock__query_history_k_data_plus_daily_compacted",)`。
4. 更新 raw sync 单元测试：
   - S3 输入路径从 `source/baostock__query_history_k_data_plus_daily/year=2026/000000_0.parquet` 改为 `source/baostock__query_history_k_data_plus_daily_compacted/year=2026/000000_0.parquet`。

完成标准：

- raw sync 只依赖 compacted source asset。
- raw sync 仍按 year 分区替换 ClickHouse raw。
- 旧 daily source 不出现在 enabled ClickHouse raw specs 中。

### 阶段 5：dbt staging 和文档切换

1. 更新 `stg_baostock__query_history_k_data_plus_daily.sql`：
   - raw source 改为 `source('raw', 'baostock__query_history_k_data_plus_daily_compacted')`。
   - 输出字段和模型名不变。
2. 更新 staging YAML、design 文档、raw profile 文档中 raw source 指针。
3. 保持下游 intermediate models 不改 `ref()`。

完成标准：

- `dbt parse` 通过。
- targeted `dbt build --select stg_baostock__query_history_k_data_plus_daily+` 通过或在无法执行时记录阻塞原因。

### 阶段 6：dev S3 数据迁移

迁移前先列出旧 year 分区：

```text
source/baostock__query_history_k_data_plus_daily/year=*/000000_0.parquet
```

执行迁移：

1. 复制所有旧 year parquet 到：

```text
source/baostock__query_history_k_data_plus_daily_compacted/year=*/000000_0.parquet
```

2. 对每个 year 校验：
   - 旧对象存在。
   - 新对象存在。
   - row count 一致。
   - schema 等于 `baostock__query_history_k_data_plus_daily_compacted` contract schema。
3. 校验通过后删除旧路径下 `year=*` 对象，避免新 daily source 路径下混杂历史 year 分区。
4. 对 2026 分区单独标记质量：
   - 如果旧 2026 年分区仍是此前只含单证券的失败产物，不得作为完整历史验收依据。
   - 可以先迁移为当前 dev 快照以完成路径切换，但最终数据完整性必须通过后续补齐策略单独验收。

完成标准：

- compacted S3 路径包含历史年分区。
- daily source 路径只保留 `trade_date=*` 分区，不保留 `year=*`。
- 数据迁移报告写入 `docs/jobs/reports/`，包含 year、old row count、new row count、对象路径和异常年说明。

### 阶段 7：dev raw sync 和数据核验

1. 对已迁移 year 分区运行 compacted raw sync：

```bash
cd pipeline
uv run dg launch --target-path scheduler \
  --assets "key:clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted" \
  --partition 2026
```

2. 核验 ClickHouse：
   - `max(date)` 达到对应迁移分区最大交易日。
   - 每年 `count()` 与 compacted S3 row count 一致。
   - `uniqExact(code)` 不应退化到单证券。
   - `countIf(code LIKE '%159%' OR code LIKE '%510%')` 仅作为 ETF 排除抽样，最终以 stock basic `type` join 验证 `type="5"` 不存在。
3. 旧 raw 表清理：
   - 新表验证完成后，dev 可执行 `DROP TABLE fleur_raw.baostock__query_history_k_data_plus_daily`。
   - 清理前确认 dbt `sources.yml` 已不再引用旧 raw source。

完成标准：

- `fleur_raw.baostock__query_history_k_data_plus_daily_compacted` 可供 staging 使用。
- 旧 raw source 不再参与 Dagster/dbt 当前定义。

## 测试策略

### Scheduler 单元测试

- BaoStock client connection pool authentication：
  - `max_connections=4` 时最多创建 4 条 TCP 连接。
  - 每条创建的 TCP 连接都发送一次独立 login payload。
  - 每条连接上的第一个非 login 数据请求必须发生在该连接 login 成功之后。
  - 单连接收到 `NO_LOGIN_ERROR_CODE` 时只刷新当前连接登录态，不把首连接登录态当作全池登录态。
  - 被关闭或登录失败的连接不可复用；替换连接必须先 login 再进入可借用状态。
- BaoStock daily fetch：
  - 单交易日只生成 `trade_date` 分区。
  - 远端请求参数为 `start_date=end_date=trade_date`。
  - `client_factory.client(max_connections=4)` 与 `BoundedTaskRunner(max_concurrent_tasks=4)` 同步生效。
  - `selected_security_types` 只包含 `["1", "2"]`。
  - `type="5"` ETF 被过滤。
- Compacted：
  - 从 daily partitions 读取并 concat。
  - 输出 dataset schema 为 `_compacted` contract schema。
  - 缺失分区 metadata 与 THS/Jiuyan compact 行为一致。
- Definitions：
  - `baostock_bundle.assets` 包含 stock basic、daily K、compacted K。
  - `baostock_bundle.jobs` 包含 daily job 和 compacted job。
  - daily schedule 分区键为交易日，不再是年份。
- ClickHouse：
  - raw spec 指向 compacted dataset。
  - S3 object key 使用 `_compacted/year=YYYY/000000_0.parquet`。
  - raw sync job selection 不再选 daily source-only asset。

### Contract / generated outputs

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
```

### Scheduler / Dagster

```bash
cd pipeline
uv run pytest scheduler/tests/unit/baostock/test_baostock.py \
  scheduler/tests/unit/clickhouse/test_clickhouse_specs.py \
  scheduler/tests/unit/clickhouse/test_clickhouse_sql.py \
  scheduler/tests/unit/clickhouse/test_raw_sync.py \
  scheduler/tests/unit/storage/test_storage_and_services.py \
  scheduler/tests/integration/test_definitions_and_schedules.py -q
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
cd scheduler
uv run dg check defs
```

### dbt

```bash
cd pipeline
uv run dbt parse --project-dir elt --profiles-dir elt
uv run dbt build --project-dir elt --profiles-dir elt \
  --select stg_baostock__query_history_k_data_plus_daily+
uv run python elt/scripts/validate_staging_readiness.py
uv run python elt/scripts/validate_field_glossary.py
```

### 文档

```bash
make docs-check
git diff --check
```

## 风险与处理

| 风险 | 影响 | 处理 |
|---|---|---|
| 旧 2026 S3 年分区已不完整 | 迁移后 compacted 2026 仍不完整 | 迁移报告中标注异常；2026 完整性单独核验，不把迁移成功等同于数据完整 |
| daily source 全量回填比旧 year range 请求更多 | 历史补齐成本高 | 历史优先迁移旧 year parquet；daily source 主要承担未来增量 |
| 4 个已登录连接仍触发 BaoStock 登录或限流策略 | 日 K 抓取失败或降速 | 把连接数集中在常量/配置中，先按 4 执行；metadata 记录连接数和失败情况，必要时降配 |
| 连接池仍使用 client 级登录态 | 后续连接未登录却承载数据请求 | 改为连接级 login-on-create；单测断言每条连接 login 后才能发数据请求 |
| contract schema 要求 raw year 分区必须来自 year parquet | daily source 不能直接 raw sync | 新增 `_compacted` contract 作为 raw sync 输入 |
| dbt raw source 名变化 | staging parse/build 失败 | staging SQL 一刀切改读 `_compacted`；下游只依赖 staging ref |
| 旧 S3 `year=*` 和新 daily `trade_date=*` 共存在同一 prefix | 运维和排查混乱 | 迁移验证后删除旧 `year=*` |
| ETF `type="5"` 被排除 | ETF 行情下游不可用 | 这是显式目标；如需 ETF，后续单独建资产和模型 |

## 完成标准

- Dagster 当前定义中：
  - `source/baostock__query_history_k_data_plus_daily` 是 daily `trade_date` source-only asset。
  - `source/baostock__query_history_k_data_plus_daily_compacted` 是 yearly compacted asset。
  - `clickhouse/raw/baostock__query_history_k_data_plus_daily_compacted` 是唯一 BaoStock 日 K raw sync asset。
- dev S3 历史 year parquet 已迁移到 `_compacted/year=*`，旧 daily prefix 下的 `year=*` 已清理。
- dbt staging model 名称保持 `stg_baostock__query_history_k_data_plus_daily`，但 raw source 已切到 `_compacted`。
- `type="5"` 不再进入 BaoStock 日 K daily 或 compacted 数据。
- BaoStock 日 K daily asset 远端抓取固定使用 4 个已登录池化连接，且每条连接独立登录。
- 所有测试、contract 生成、Dagster definitions、dbt parse/build 和文档检查通过，或阻塞项已在 job report 中记录。
