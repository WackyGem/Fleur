---
name: fleur-dagster-http-source
description: mono-fleur 的 Dagster HTTP 数据源资产开发流程。用于当用户提供远端 HTTP/HTTPS 链接、API endpoint 或接口样例，希望新增或修改 Dagster source asset，将远端数据抓取并写入 S3 Parquet，再通过 contract 驱动同步到 ClickHouse raw 层；覆盖 endpoint profiling、数据契约、scheduler SourceBundle、PyArrow schema 转换、测试和 dg 验收。
---

# Dagster HTTP Source Asset

当用户只提供一个远端 HTTP 链接，并希望生成 `HTTP -> S3 Parquet -> ClickHouse raw`
链路代码时，使用本 skill。

目标不是生成孤立 asset，而是把新数据集纳入 mono-fleur 的长期边界：

- 远端接口事实记录在 `docs/references/remote_endpoint/`。
- 字段事实源记录在 `pipeline/contracts/datasets/*.yml`。
- Dagster source asset 写入 contract 精确匹配的 S3 Parquet。
- ClickHouse raw asset 由 `pipeline/contracts` 中的 `clickhouse_raw` 自动生成。

## 配合使用

- 先使用 `dagster-expert`，并按需读取 asset metadata、dependency 和 `dg check` reference。
- 涉及 contract、Parquet schema、ClickHouse raw 字段时使用 `fleur-contract-data-dictionary`。
- 写 Python 代码或测试时使用 `dignified-python`。
- 需要实际 materialize、回填或重跑时使用 `fleur-dagster-backfill-runbook`。
- 涉及 Dagster、ClickHouse client、PyArrow、dbt 等库/CLI 的 API 细节时，按 `AGENTS.md` 使用 Context7。

## 输入归一化

最小输入可以只有一个 URL。先把它归一化成下面事实，不足时自己探测；只有鉴权、
业务口径或目标数据集命名无法安全推断时才问用户。

- `source`：供应商或站点名，作为 source bundle 名和 asset tag。
- `dataset`：形如 `<source>__<entity>`，作为 source asset 尾段、contract 文件名和 raw table 名。
- `endpoint`：HTTP method、URL、query params、headers、referer、分页和速率限制。
- `payload_format`：JSON、CSV、TXT、ZIP 等。
- `grain`：一行代表什么。
- `storage_mode`：`latest_snapshot` 或 `partitioned`。
- `partition_key_name`：仅 `partitioned` 需要，如 `year`、`trade_date`。
- `raw_sync`：是否同步到 ClickHouse raw；不能确定时默认做 source-only，并在最终说明原因。

如果 URL 需要 token、cookie、账号密码或私有 header，不能把密钥写进代码、文档或最终回答；
应使用现有 `.env`/resource/config 模式，缺少变量时向用户要变量名或鉴权方式。

## Endpoint Profiling

1. 先读入口文档：`AGENTS.md`、`docs/architecture/scheduler-architecture.md`、
   `docs/architecture/scheduler-module-boundaries.md`。
2. 查已有事实：`docs/references/remote_endpoint/<dataset>.md`、
   `pipeline/contracts/datasets/<dataset>.yml`、同源 `pipeline/scheduler/src/scheduler/defs/sources/<source>/`。
3. 用 `curl` 或现有 HTTP client 探测 URL。公开接口可以直接取小样本；需要鉴权或大量数据时只记录阻塞原因。
4. 记录或更新 `docs/references/remote_endpoint/<dataset>.md`，至少包含：
   endpoint、请求示例、参数表、响应结构、分页/游标、错误码、保留期限、headers、速率限制和样例字段。
5. 不从字段名凭空编业务含义。中文描述无法确认时按 contract skill 使用“待核实”格式。

## Contract First

新增或修改 source asset 前，先维护 `pipeline/contracts/datasets/<dataset>.yml`。

基础规则：

- `dataset`、文件名、`clickhouse_raw.table` 保持一致。
- `source_asset_key` 使用 `["source", dataset]`。
- 只有确实需要 raw sync 时才写 `raw_asset_key: ["clickhouse", "raw", dataset]` 和 `clickhouse_raw`。
- 不要用假的 `clickhouse_raw` 伪装 source-only asset。
- 字段链路必须是 `source.fields -> parquet.fields -> clickhouse_raw.fields`。
- `parquet.fields` 是 S3 Parquet schema 的权威事实源；source conversion 必须产出完全匹配的 `pyarrow.Table`。
- `clickhouse_raw.database` 使用当前 raw database 约定 `fleur_raw`。
- `LowCardinality(String)` 必须有真实理由；不确定时先用 `String`。

ClickHouse raw 策略：

- 当前 raw sync factory 支持 `partition_strategy: snapshot` 和 `partition_strategy: year`。
- `latest_snapshot` source 通常对应 `partition_strategy: snapshot`。
- 年分区 source 对应 `partition_strategy: year`，`partition_key_name` 必须是 `year`。
- 日分区、交易日稀疏分区或接口保留期很短的数据，优先先落 source Parquet；若要进 raw，通常先新增 compact/year 资产，再让 compact 后数据同步 raw。
- source-only dataset 不会生成 dbt raw source 或 ClickHouse raw asset，这是预期行为。

## Scheduler Implementation

代码位置优先复用现有 source package；新供应商使用：

```text
pipeline/scheduler/src/scheduler/defs/sources/<source>/
├── assets.py 或 <entity>.py
├── schema.py
├── services.py
└── definitions.py
```

实现规则：

- 新 source 必须在自己的 `definitions.py` 导出 `SourceBundle`，再由
  `pipeline/scheduler/src/scheduler/defs/definitions.py` 的 `SOURCE_BUNDLES` 显式聚合。
- 业务代码通过 `HttpClientFactoryResource`、`S3SettingsResource`、ClickHouse/S3 resources 或 factory 注入外部能力。
- 不在业务模块直接读取环境变量，不直接 new 底层 HTTP client，不在 `http/` 聚合 source definitions。
- 资产 lineage 只表达真实数据依赖；限速、并发、分页、retry、回填窗口通过 config、service、schedule、pool 或 runbook 表达。
- 标准 S3 Parquet source asset 使用 `io_manager_key="s3_io_manager"` 并返回
  `dg.MaterializeResult(value=table_or_partition_map, metadata=metadata)`。
- 特殊稀疏日分区可复用 `scheduler.defs.http.partitioning.materialize_trade_date_range()`，
  但必须保留 metadata 和 contract schema 约束。

Asset decorator 必须保留项目契约：

```python
@dg.asset(
    name="<dataset>",
    key_prefix=[SOURCE_ASSET_KEY_PREFIX],
    group_name="s3_sources",
    io_manager_key="s3_io_manager",
    metadata=latest_snapshot_metadata(),  # 或 year_partition_metadata()/daily_sparse_partition_metadata()
    owners=source_owners(),
    kinds=s3_parquet_kinds("http"),
    tags=source_tags("<source>"),
)
def <dataset>(...) -> dg.MaterializeResult:
    ...
```

根据实际情况补 `partitions_def`、`deps`、`backfill_policy`、`pool` 和 `automation_condition`。
依赖用参数依赖或 `deps=` 都可以，但必须反映真实数据依赖。

## Schema Conversion

- 转换函数放在 source 自己的 `schema.py`，或在已有通用 HTTP schema helper 中扩展。
- 使用 `scheduler.defs.contract_schemas` 暴露的 schema map/constant，禁止在 source 业务代码中直接 import `fleur_contracts`。
- 构造 `pa.Table` 时按 contract 字段顺序输出，并让类型与 nullable 精确匹配。
- 记录 `unknown_field_count`、`row_count`、`column_count`、页数、请求次数、重试次数、解码错误等 metadata。
- 对日期、时间戳、布尔、数字和空字符串做显式转换；不要依赖 S3 IO manager 自动 cast。
- 空表必须是显式设计：contract/asset metadata 使用 `allow_empty=True`，并记录接口为何可能为空。

## Jobs And Schedules

- snapshot asset 需要定期刷新时，用 source `definitions.py` 里的 `automation_schedules.build_asset_job()` 和项目 schedule 工厂。
- A 股交易日驱动的日分区资产使用 `market.schedules.build_trade_date_schedule()`。
- 年分区批量刷新使用现有 year partitions 和单分区 backfill policy。
- 不要为了限流制造虚假 asset dependency；使用 pool、config 或 service concurrency。

## Tests

按风险添加最小测试：

- endpoint/schema conversion：给样例 payload，断言 `pa.Table` schema、字段顺序、类型、row count、unknown field count。
- service：用 fake HTTP client 验证分页、错误码、retry/统计、空响应。
- asset definition：必要时断言 tags、owners、metadata、partitions、deps、pool。
- source bundle：新增 source 或 assets 后更新 `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py` 的稳定断言。
- contract/schema：raw 或 parquet 字段变化后运行 contract schema boundary tests。
- ClickHouse raw：新增 raw sync 后检查 `ENABLED_CLICKHOUSE_RAW_TABLE_SPECS`、asset deps、pool 和 SQL tests 是否需要扩展。

## 验收命令

文档和 contract-only 改动至少运行：

```bash
cd pipeline
uv run fleur-contracts validate
uv run fleur-contracts generate --check
uv run pytest contract_tools/tests -q
uv run pytest scheduler/tests/unit/test_contract_schemas.py -q
git diff --check
```

改了 scheduler Python 代码时追加：

```bash
cd pipeline
uv run ruff check scheduler/src scheduler/tests
uv run ruff format --check scheduler/src scheduler/tests
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests -q
cd scheduler
uv run dg check defs
```

新增 raw sync 或修改 ClickHouse contract 时追加：

```bash
cd pipeline
uv run pytest scheduler/tests/unit/clickhouse scheduler/tests/integration/test_definitions_and_schedules.py -q
uv run fleur-contracts validate-clickhouse --all-available
```

真实 materialization、回填或 raw sync 验证按 `docs/skills/fleur-dagster-backfill-runbook/SKILL.md`
选择 `dg launch` 命令，先跑一个小切片再扩大范围。

## 最终汇报

最终回答必须说明：

- 新增/修改的数据集、source asset key、raw asset key。
- URL 探测结果和记录到的 remote endpoint 文档。
- contract 字段链路和是否启用 ClickHouse raw sync。
- 验证命令和结果；未运行的命令要说明原因。
- 如果只完成 source-only，明确 raw sync 缺少什么事实或为什么需要 compact。
