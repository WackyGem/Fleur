# Plan 0003: 东方财富 F10 年度分区采集

状态：草案

关联 RFC：

- `docs/RFC/0002-eastmoney-f10-ingestion.md`

参考资料：

- `docs/references/remote_endpoint/eastmoney__balance.md`
- `docs/references/remote_endpoint/eastmoney__cashflow_sq.md`
- `docs/references/remote_endpoint/eastmoney__cashflow_ytd.md`
- `docs/references/remote_endpoint/eastmoney__dividend_allotment.md`
- `docs/references/remote_endpoint/eastmoney__dividend_main.md`
- `docs/references/remote_endpoint/eastmoney__equity_history.md`
- `docs/references/remote_endpoint/eastmoney__income_sq.md`
- `docs/references/remote_endpoint/eastmoney__income_ytd.md`
- `docs/references/openapi/eastmoney__balance.yaml`
- `docs/references/openapi/eastmoney__cashflow_sq.yaml`
- `docs/references/openapi/eastmoney__cashflow_ytd.yaml`
- `docs/references/openapi/eastmoney__dividend_allotment.yaml`
- `docs/references/openapi/eastmoney__dividend_main.yaml`
- `docs/references/openapi/eastmoney__equity_history.yaml`
- `docs/references/openapi/eastmoney__income_sq.yaml`
- `docs/references/openapi/eastmoney__income_ytd.yaml`
- `docs/ADR/0001-market-data-raw-assets-on-dagster.md`
- `docs/ADR/0002-s3-parquet-storage-layout.md`
- `docs/ADR/0004-baostock-tcp-client-and-daily-kline-ranges.md`
- aiohttp 当前文档：`ClientSession` 复用、`ClientTimeout`、`TCPConnector(limit, limit_per_host)`。

## 目标

实现 RFC 0002 中定义的 8 个东方财富 F10 raw asset：

1. `eastmoney__balance`
2. `eastmoney__cashflow_sq`
3. `eastmoney__cashflow_ytd`
4. `eastmoney__dividend_allotment`
5. `eastmoney__dividend_main`
6. `eastmoney__equity_history`
7. `eastmoney__income_sq`
8. `eastmoney__income_ytd`

这些 asset 需要满足：

- 复用现有 Dagster asset、S3 Parquet、`year` 分区和 materialization metadata 模式。
- 复用 `baostock__query_stock_basic` 最新快照作为证券范围来源。
- 复用 `filter_active_security_ranges(..., allowed_security_types=frozenset({"1"}))` 过滤股票，不新增另一套证券有效区间逻辑。
- 每个远端请求只覆盖“单个 EastMoney code + start_date + end_date”。
- 使用 `aiohttp` 按 code 并发请求，单 asset run 内 code 并发上限为 20。
- 单个 code 内分页顺序请求，不做分页并发。
- 所有接口使用稳定多字段排序，分页重复时 asset 失败。
- 宽表字段名来自 OpenAPI YAML，东方财富业务字段全部以 string 写入 Parquet。
- 分区、日频刷新和历史回填语义与 BaoStock K 线 year 分区资产对齐。
- 日频调度按自然日每天 16:00 触发，8 个 asset 在 daily job 内顺序执行。

## 非目标

本计划不包含：

- ClickHouse 建表或加载。
- dbt 模型。
- Web 查询接口。
- 东方财富以外的数据源。
- 对 BaoStock K 线资产的行为变更。
- 按 OpenAPI 数值或日期类型转换东方财富业务字段。
- 分页级并发优化。

## 现有前置条件

当前项目已经具备：

- `S3IOManager` 与 `write_parquet_dataset`，用于 raw Parquet 写入。
- `asset_key_to_parquet_object_key`，用于稳定生成 `source/<asset>/year=YYYY/000000_0.parquet` 路径。
- `read_baostock_stock_basic_from_s3`，用于读取证券基础信息快照。
- `filter_active_security_ranges`，用于基于 `code`、`ipoDate`、`outDate`、`type` 和请求日期范围生成有效证券请求范围。
- `ExponentialBackoffPolicy` 与 `DEFAULT_RETRY_POLICY`，用于指数退避重试。
- `baostock__query_history_k_data_plus_daily` 的 `year` 分区、S3 输出、当年刷新和历史回填模式。

需要先补齐：

- `S3IOManager` 对 `allow_empty=True` 的 0 行 Parquet 写入支持。
- `eastmoney_run_pool` 的 Dagster concurrency 配置。
- `aiohttp` 依赖。

## 总体结构

新增模块建议：

```text
pipeline/scheduler/src/scheduler/defs/eastmoney/
  __init__.py
  assets.py
  client.py
  schemas.py
  schedules.py
```

职责划分：

- `client.py`：`aiohttp` session、timeout、connector、code 级 semaphore、重试、分页、响应校验。
- `schemas.py`：从 OpenAPI YAML 读取字段名，构建全 string `pa.Schema`，将响应行转换为 `pa.Table`。
- `assets.py`：定义 8 个 year 分区 asset，读取 stock basic，生成 code 请求范围，调用 client，拼接表并返回分区映射。
- `schedules.py`：定义 `eastmoney__daily_job` 和 `eastmoney__daily_schedule`。
- `pipeline_defs.py`：注册 EastMoney assets、job、schedule。

不建议新增通用框架层；第一版先保持 EastMoney 域内封装，等接口数量继续增长后再抽象。

## 资产配置矩阵

| Asset | 接口族 | 固定参数 | 稳定排序 | 输出路径 |
| --- | --- | --- | --- | --- |
| `eastmoney__balance` | `data/get` | `type=RPT_F10_FINANCE_GBALANCE`, `sty=F10_FINANCE_GBALANCE` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` | `source/eastmoney__balance/year=YYYY/000000_0.parquet` |
| `eastmoney__cashflow_sq` | `data/get` | `type=RPT_F10_FINANCE_GCASHFLOWQC`, `sty=PC_F10_GCASHFLOWQC` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` | `source/eastmoney__cashflow_sq/year=YYYY/000000_0.parquet` |
| `eastmoney__cashflow_ytd` | `data/get` | `type=RPT_F10_FINANCE_GCASHFLOW`, `sty=APP_F10_GCASHFLOW` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` | `source/eastmoney__cashflow_ytd/year=YYYY/000000_0.parquet` |
| `eastmoney__dividend_allotment` | `data/v1/get` | `reportName=RPT_F10_DIVIDEND_ALLOTMENT`, `columns=ALL` | `sortColumns=NOTICE_DATE,SECURITY_CODE`, `sortTypes=-1,-1` | `source/eastmoney__dividend_allotment/year=YYYY/000000_0.parquet` |
| `eastmoney__dividend_main` | `data/v1/get` | `reportName=RPT_F10_DIVIDEND_MAIN`, `columns=ALL` | `sortColumns=NOTICE_DATE,SECURITY_CODE`, `sortTypes=-1,-1` | `source/eastmoney__dividend_main/year=YYYY/000000_0.parquet` |
| `eastmoney__equity_history` | `data/v1/get` | `reportName=RPT_F10_EH_EQUITY`, `columns=ALL` | `sortColumns=NOTICE_DATE,SECURITY_CODE`, `sortTypes=-1,-1` | `source/eastmoney__equity_history/year=YYYY/000000_0.parquet` |
| `eastmoney__income_sq` | `data/get` | `type=RPT_F10_FINANCE_GINCOMEQC`, `sty=PC_F10_GINCOMEQC` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` | `source/eastmoney__income_sq/year=YYYY/000000_0.parquet` |
| `eastmoney__income_ytd` | `data/get` | `type=RPT_F10_FINANCE_GINCOME`, `sty=APP_F10_GINCOME` | `st=REPORT_DATE,SECURITY_CODE`, `sr=-1,-1` | `source/eastmoney__income_ytd/year=YYYY/000000_0.parquet` |

共同配置：

- 日期过滤字段：`NOTICE_DATE`。
- 分区字段：`year`。
- 字段类型：东方财富业务字段全部 `pa.string()`。
- asset metadata：`{"storage_mode": "partitioned", "partition_key_name": "year", "allow_empty": True}`。
- asset group：`eastmoney`。
- pool：`eastmoney_run_pool`。

## 前置改造计划

### ADR 0002 更新

在 `docs/ADR/0002-s3-parquet-storage-layout.md` 中补充：

- `allow_empty=True` 是 `S3IOManager` 的显式 opt-in 行为。
- 默认仍拒绝空表，避免空结果意外覆盖有效快照或分区。
- 对 `allow_empty=True` 的分区资产，允许写入带 schema 的 0 行 Parquet。
- 0 行写入仍必须校验 schema、分区键和输出路径。

### S3IOManager 空表支持

扩展要求：

- `context.definition_metadata.get("allow_empty")` 为 true 时，`_validate_table` 允许 `num_rows == 0`。
- 分区表映射仍不能为空；但每个分区的 table 可以是 0 行。
- 0 行 table 必须带完整 schema。
- metadata 中记录 `allow_empty` 和 `empty_partition_keys`。
- 现有默认行为不变：没有 `allow_empty=True` 时继续拒绝空表。

测试要求：

- 默认空表仍失败。
- `allow_empty=True` 时可写入 0 行 Parquet。
- 分区映射键与 Dagster partition keys 不一致时仍失败。

### 依赖与配置

Python 依赖：

- 增加 `aiohttp`。

Dagster concurrency：

- 新增 Makefile 或文档化命令：

```bash
cd pipeline
uv run dagster instance concurrency set eastmoney_run_pool 3
```

运行配置默认值：

```text
EASTMONEY_CODE_CONCURRENCY=20
EASTMONEY_HTTP_TOTAL_TIMEOUT_SECONDS=60
EASTMONEY_HTTP_CONNECT_TIMEOUT_SECONDS=5
EASTMONEY_HTTP_READ_TIMEOUT_SECONDS=30
EASTMONEY_PAGE_SIZE_DATA_GET=500
EASTMONEY_PAGE_SIZE_DATA_V1_GET=500
```

这些配置可以第一版硬编码为模块常量；如果后续需要运行时调参，再集中放入 `config.py`。

## Endpoint 配置设计

建议定义不可变配置结构：

```python
@dataclass(frozen=True)
class EastmoneyEndpointConfig:
    asset_name: str
    api_family: Literal["data_get", "data_v1_get"]
    openapi_path: str
    date_field: str
    sort_fields: tuple[str, ...]
    sort_directions: tuple[str, ...]
    page_size: int
    fixed_params: Mapping[str, str]
```

要求：

- 所有 8 个 asset 的固定参数集中维护。
- `date_field` 第一版统一为 `NOTICE_DATE`。
- `data/get` 的分页参数使用 `p`、`ps`，排序参数使用 `st`、`sr`。
- `data/v1/get` 的分页参数使用 `pageNumber`、`pageSize`，排序参数使用 `sortColumns`、`sortTypes`。
- 不允许 asset 函数内散落硬编码接口参数。

## 字段与 Schema 计划

字段来源：

- 从 `docs/references/openapi/eastmoney__*.yaml` 的 `result.data.items.properties` 读取字段名。
- 保持东方财富原始字段名。
- 所有业务字段映射为 `pa.string()`。

补充字段：

```text
request_code
request_start_date
request_end_date
partition_year
source_endpoint
ingested_at
```

补充字段也使用 string，除非后续 ADR 明确需要结构化类型。

转换规则：

- JSON `null` 写为 null。
- 字符串保持原样。
- 数字、布尔值等通过稳定字符串化写入。
- 未出现在 OpenAPI 字段列表中的返回字段第一版不写入，但计入 `unknown_field_count` metadata。
- OpenAPI 中存在但响应缺失的字段写为 null。

## EastMoney Client 计划

客户端职责：

- 复用单个 `aiohttp.ClientSession`。
- 使用 `aiohttp.ClientTimeout(total=60, sock_connect=5, sock_read=30)`。
- 使用 `aiohttp.TCPConnector(limit=20, limit_per_host=20)`。
- 使用 `asyncio.Semaphore(20)` 控制 code 级并发。
- 复用 `DEFAULT_RETRY_POLICY`，最多 4 次请求，总等待 1、2、4 秒。
- 支持 `data/get` 与 `data/v1/get` 两种接口族。
- 单个 code 内顺序分页。

建议接口：

```python
async def fetch_code_range(
    self,
    endpoint: EastmoneyEndpointConfig,
    code: str,
    start_date: date,
    end_date: date,
) -> list[dict[str, object]]:
    ...
```

分页规则：

1. 请求第 1 页。
2. 从 `result.pages` 读取总页数。
3. 如果 `result` 为空或 `result.data` 为空，返回空列表。
4. 如果 `data/v1/get` 返回 `code=9201` 且 `result=null`，返回空列表。
5. 对同一 code 按页码顺序请求后续页面。
6. 合并页面并做 fingerprint 重复检测。
7. 发现跨页重复时抛出错误，不静默去重。

重试范围：

- `aiohttp.ClientError`。
- `asyncio.TimeoutError`。
- HTTP 429。
- HTTP 5xx。
- 非空响应体 JSON 解析失败。

不重试范围：

- 参数构造错误。
- `code=9201, result=null`。
- 字段配置错误。
- 跨页重复检测失败。

## 证券范围计划

读取：

```python
stock_basic = read_baostock_stock_basic_from_s3(S3Config.from_env())
```

过滤：

```python
security_ranges = filter_active_security_ranges(
    stock_basic,
    requested_start_date=start_date,
    requested_end_date=end_date,
    allowed_security_types=frozenset({"1"}),
)
```

转换：

```text
sh.600000 -> 600000.SH
sz.000001 -> 000001.SZ
```

跳过：

- 不是 `sh.` 或 `sz.` 前缀的 code。
- `filter_active_security_ranges` 已过滤掉的无效日期区间。

metadata：

- `candidate_security_count`
- `selected_security_count`
- `skipped_security_count`
- `unsupported_market_code_count`
- `selected_security_types`

## Asset 计划

每个 asset：

- 使用同一个 `year_partitions` 定义。
- 返回 `dg.MaterializeResult[dict[str, pa.Table]]`。
- 返回值 key 必须与 Dagster partition key 完全一致。
- 每个 `year` 分区输出一个 `pa.Table`。
- 支持 0 行输出，前提是 table schema 完整。

日频 run config：

```text
refresh_until_date = YYYY-MM-DD
```

构造 year range：

- 如果传入 `refresh_until_date`：
  - 只允许单个 year partition。
  - `refresh_until_date.year` 必须等于 partition key。
  - 范围为 `YYYY-01-01` 到 `refresh_until_date`。
- 如果未传入：
  - 每个 partition 范围为 `YYYY-01-01` 到 `YYYY-12-31`。

这与 BaoStock K 线资产的当年刷新和历史回填语义保持一致。

## Job 与 Schedule 计划

### `eastmoney__daily_job`

资产执行顺序：

1. `eastmoney__balance`
2. `eastmoney__cashflow_sq`
3. `eastmoney__cashflow_ytd`
4. `eastmoney__dividend_allotment`
5. `eastmoney__dividend_main`
6. `eastmoney__equity_history`
7. `eastmoney__income_sq`
8. `eastmoney__income_ytd`

实现要求：

- 顺序执行是调度控制需求，不代表业务血缘依赖。
- 如果通过显式依赖链实现顺序，必须在注释或 metadata 中说明该依赖是 execution ordering dependency。
- 如果通过专用 job/op 实现顺序，也必须保持 8 个 asset 的独立 materialization 和 S3 输出路径。

### `eastmoney__daily_schedule`

调度：

```text
cron_schedule = "0 16 * * *"
execution_timezone = "Asia/Shanghai"
```

行为：

- 自然日运行。
- 不读取 `sina__trade_calendar`。
- 不跳过非交易日。
- 每次提交当年 `year` 分区。
- run config 设置 `refresh_until_date` 为调度自然日。

tags：

```text
market.natural_date=<YYYY-MM-DD>
market.year=<YYYY>
source=eastmoney
```

## Metadata 计划

每个 asset materialization 至少包含：

- `row_count`
- `column_count`
- `partition_keys`
- `candidate_security_count`
- `selected_security_count`
- `skipped_security_count`
- `unsupported_market_code_count`
- `request_count`
- `empty_response_count`
- `page_count`
- `retry_count`
- `duplicate_page_row_count`
- `unknown_field_count`
- `selected_date_field`
- `sort_columns`
- `sort_types`
- `source_endpoint`
- `code_concurrency_limit`
- `s3_bucket`
- `s3_keys`
- `eastmoney_remote_fetch_seconds`
- `table_convert_seconds`
- `asset_function_seconds`

## 测试计划

单元测试：

- `S3IOManager` 默认拒绝空表。
- `S3IOManager` 在 `allow_empty=True` 时写入 0 行 Parquet。
- OpenAPI 字段读取能提取字段名，并全部映射为 `pa.string()`。
- EastMoney code 转换：
  - `sh.600000 -> 600000.SH`
  - `sz.000001 -> 000001.SZ`
  - 不支持前缀返回跳过。
- `filter_active_security_ranges` 调用时只允许 `{"1"}`。
- `data/get` 参数构造使用 `p`、`ps`、`st`、`sr`。
- `data/v1/get` 参数构造使用 `pageNumber`、`pageSize`、`sortColumns`、`sortTypes`。
- 单 code 分页顺序请求。
- 跨页重复 fingerprint 导致失败。
- `code=9201, result=null` 作为空结果处理。

真实接口验证：

- 每个接口选择 1 个股票和 1 个短日期范围，请求第一页。
- 对至少一个可能多页的接口验证顺序分页。
- 验证 `sortColumns=NOTICE_DATE,SECURITY_CODE` 在 `data/v1/get` 三个事件类接口上可用。
- 如果 `data/v1/get` 不接受该排序组合，暂停实现并更新 RFC/ADR。

Dagster 验证：

- `uv run dg check defs --target-path scheduler`。
- 手动 materialize 单个 asset 单个 year 分区。
- 手动 materialize 8 个 asset 的小范围日频 job。
- 检查 S3 路径和 metadata。

## 验收标准

实现完成后应满足：

- Dagster UI 中能看到 8 个 `eastmoney` group 下的 asset。
- 8 个 asset 都按 `year` 分区。
- 每个 asset 输出路径为 `source/<asset>/year=YYYY/000000_0.parquet`。
- 每个 asset 只请求 BaoStock `type="1"` 股票。
- 每个 HTTP 请求只包含一个 EastMoney code 和一个日期范围。
- 单 asset run 内 code 并发不超过 20。
- 同一个 code 内分页按页码顺序请求。
- 所有分页请求使用稳定多字段排序。
- 跨页重复时 asset 失败。
- 东方财富业务字段全部写为 string。
- 稀疏资产可以写入 0 行 Parquet。
- `eastmoney_run_pool` 最大 3 个 job 并行。
- `eastmoney__daily_schedule` 每天 `Asia/Shanghai` 16:00 自然日触发。
- `eastmoney__daily_job` 内 8 个 asset 按计划顺序执行。

## 实施顺序

1. 更新 ADR 0002，补充 `allow_empty=True` 空表写入决策。
2. 扩展 `S3IOManager` 支持 0 行 Parquet opt-in，并补测试。
3. 增加 `aiohttp` 依赖。
4. 增加 `eastmoney_run_pool` 本地 concurrency 配置。
5. 新增 `eastmoney` defs 包和 endpoint 配置结构。
6. 实现 OpenAPI 字段名读取和全 string schema 构造。
7. 实现 EastMoney code 转换工具。
8. 实现 `EastmoneyAioHttpClient`，先覆盖单 code、单页请求。
9. 增加 `data/get` 与 `data/v1/get` 参数构造测试。
10. 实现单 code 顺序分页、空结果处理和跨页重复检测。
11. 实现 code 级 semaphore 并发。
12. 接入第一个 asset `eastmoney__balance`，完成单分区 materialize 验证。
13. 复制配置方式接入其余 7 个 asset。
14. 接入 `eastmoney__daily_job`，保证 8 个 asset 顺序执行。
15. 接入 `eastmoney__daily_schedule`。
16. 更新 `pipeline_defs.py` 注册 assets、job、schedule。
17. 使用 `uv run dg check defs --target-path scheduler` 校验 definitions。
18. 使用小范围真实接口验证排序、分页、空结果和 S3 输出。
19. 使用完整 year 分区回填验证性能、metadata 和空表行为。

## 风险与待确认

- `data/v1/get` 三个事件类接口可能不支持 `sortColumns=NOTICE_DATE,SECURITY_CODE`。如果真实接口不接受，应回退到已验证的稳定排序字段，并更新 RFC/ADR。
- 东方财富可能对请求频率有隐性限制。若 HTTP 429 或 5xx 较多，应降低 `EASTMONEY_CODE_CONCURRENCY` 或增加 jitter。
- 宽表字段可能随接口变化新增。第一版未知字段不写入但记录 metadata；如果未知字段频繁出现，应更新 OpenAPI 文档。
- 0 行 Parquet 写入会改变现有 IO manager 默认假设，必须通过 `allow_empty=True` 限定影响范围。
