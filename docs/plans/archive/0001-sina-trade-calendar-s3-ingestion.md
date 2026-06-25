# Plan 0001: 新浪交易日历采集到 S3

状态：草案

关联 RFC：

- `docs/RFC/archive/0001-market-data-ingestion.md`

参考资料：

- `docs/references/remote_endpoint/sina__calendar.md`
- `docs/references/openapi/sina__calendar.yaml`
- `deploy/docker-compose.yml`
- Dagster 当前文档：资产定义、`io_manager_key`、自定义 IO manager、分区资产 IO manager 处理方式
- Apache Hive 文档：分区通过 `PARTITION (key='value')` 表达，落盘路径通常对应 `key=value` 目录；数据文件名本身没有强制的 Hive 规范，常见 `000000_0`、`part-*` 是写入引擎生成的文件名。

## 目标

实现 RFC 0001 中第一阶段的 `sina__trade_calendar` 资产设计，用 Dagster 采集新浪财经 A 股交易日历，并通过自定义 `s3_io_manager` 写入本地 S3 兼容对象存储。

本计划只覆盖设计，不包含代码实现。

## 范围

本计划包含：

- 使用 `requests` 请求新浪交易日历接口。
- 请求头使用 Chrome User-Agent。
- 请求超时或请求错误时使用指数退避重试，最多 3 次重试，等待间隔为 1、2、4 秒。
- 根据参考文档中的 `SinaCalendarParser` 解码逻辑设计 parser。
- 将解析结果作为 Dagster asset 输出。
- 通过自定义 `s3_io_manager` 以 Parquet 格式、Zstandard 压缩写入 `deploy/docker-compose.yml` 中定义的 S3 兼容服务。
- 按 Dagster 工程规范组织资产、资源、IO manager 和后续调度。

本计划不包含：

- BaoStock 证券基础信息采集。
- BaoStock 日频 K 线采集。
- ClickHouse 建表或加载。
- dbt 模型。
- 数据质量检查。

## 数据源

资产名：

- `sina__trade_calendar`

请求地址：

- `https://finance.sina.com.cn/realstock/company/klc_td_sh.txt`

请求方式：

- `GET`

请求库：

- `requests`

请求重试：

- 对请求超时、连接错误、HTTP 非 2xx 响应进行重试。
- 最多 3 次重试；加上首次请求，总共最多 4 次请求。
- 指数退避等待间隔固定为 1、2、4 秒。
- 3 次重试后仍失败时，让 Dagster run 失败，不写入 S3。

请求头：

- `User-Agent`: 使用 Chrome UA，例如桌面 Chrome 的 `Mozilla/5.0 ... Chrome/... Safari/537.36` 格式。
- `Accept`: `text/plain,*/*`

响应格式：

- `text/plain`
- JavaScript 变量声明，格式为 `var datelist="<encoded>";var KLC_TD_SH=datelist;`
- 该编码不是标准 Base64，不能直接用标准 Base64 解码。

## 解析设计

新增一个面向新浪交易日历的 parser，命名建议：

- `SinaCalendarParser`

职责：

- 从响应文本中提取 `var datelist="..."` 的编码字符串。
- 使用参考文档中的新浪自定义 bitstream 解码算法解析交易日。
- 补充参考文档中指出的缺失日期 `1992-05-04`。
- 输出稳定、可序列化的数据结构。

输出建议：

- 资产返回二维数组，保持 RFC 0001 定义：
  - 每行只有一个日期字符串。
  - 日期格式为 `YYYY-MM-DD`。

示例形态：

```text
[
  ["1990-12-19"],
  ["1990-12-20"],
  ["1990-12-21"]
]
```

错误处理：

- 未找到 `datelist` 时，parser 返回空结果，并由 asset 记录失败元数据。
- 解码校验失败时，parser 返回空结果，并由 asset 记录失败元数据。
- 请求失败、超时或非 2xx 响应先按 1、2、4 秒指数退避重试。
- 重试耗尽后仍失败时，让 Dagster run 失败，避免写入空文件覆盖有效数据。

## S3 存储设计

`deploy/docker-compose.yml` 中的 S3 兼容服务为 `rustfs`：

- 容器名：`mono-fleur-rustfs`
- 容器内 API 端口：`9000`
- 宿主机默认 API 端口：`${RUSTFS_API_PORT:-34050}`
- 访问密钥环境变量：`RUSTFS_ACCESS_KEY`
- 密钥环境变量：`RUSTFS_SECRET_KEY`

建议本地开发 endpoint：

- `http://127.0.0.1:34050`

建议容器网络内 endpoint：

- `http://rustfs:9000`

建议 bucket：

- `mono-fleur`

建议对象 key：

- `source/sina__trade_calendar/000000_0.parquet`
- 该 key 不包含 Hive 分区目录；`000000_0.parquet` 是单个对象文件名。

写入格式：

- 第一版即使用 Parquet。
- 压缩算法使用 Zstandard，即 `zstd`。
- Parquet schema 至少包含：
  - `trade_date`: `date32` 或 `string`，语义为 A 股交易日。
- asset 内部仍可保持 RFC 定义的二维数组作为 parser 输出，但写入 S3 前需要转换为表结构。

写入语义：

- `sina__trade_calendar` 是不分区资产。
- S3 目录固定为 `source/sina__trade_calendar/`，不创建 `snapshot_date=YYYY-MM-DD/` 这类 Hive 分区目录。
- 文件名使用 Hive 常见单文件输出形态 `000000_0.parquet`。
- 每次 materialize 都覆盖同一个 `source/sina__trade_calendar/000000_0.parquet` 对象。
- 不使用 `snapshot_date=YYYY-MM-DD.parquet` 作为文件名；交易日历只保留当前最新快照。
- IO manager 负责把 asset 返回值转换为 Parquet bytes，并使用 zstd 压缩写入 S3。
- asset 本身只负责请求和解析，不直接处理对象存储细节。

## 自定义 `s3_io_manager` 设计

新增自定义 IO manager，命名建议：

- `s3_io_manager`

职责：

- 读取 endpoint、bucket、access key、secret key 等配置。
- 将 asset 输出转换成 Parquet bytes。
- 使用 zstd 压缩写入 Parquet。
- 写入 S3 兼容对象存储。
- 在 Dagster metadata 中返回 bucket、key、endpoint、row_count、content_type、compression 等信息。

配置来源：

- `RUSTFS_ENDPOINT`
- `RUSTFS_ACCESS_KEY`
- `RUSTFS_SECRET_KEY`
- `RUSTFS_BUCKET`

默认值建议：

- `RUSTFS_ENDPOINT=http://127.0.0.1:34050`
- `RUSTFS_BUCKET=mono-fleur`

工程约束：

- IO manager 作为 Dagster resource 注册。
- asset 通过 `io_manager_key="s3_io_manager"` 绑定该 IO manager。
- 后续 BaoStock raw parquet 资产也可以复用同一个 IO manager，并通过对象 key 模板区分不同资产路径。

## Dagster 工程组织

当前 Dagster 工程位于：

- `pipeline/scheduler`

当前 definitions 入口：

- `pipeline/scheduler/src/scheduler/definitions.py`

该入口使用 `load_from_defs_folder` 加载 `defs` 目录，因此新增定义应放在：

- `pipeline/scheduler/src/scheduler/defs/`

建议目录结构：

```text
pipeline/scheduler/src/scheduler/
  defs/
    http_resources/
      __init__.py
      assets.py
      schedules.py
      sina__trade_calendar.py
    io_managers/
      s3_io_manager.py
```


命名与分组：

- asset key：`sina__trade_calendar`
- group：`http_sources`
- tags：
  - `source=sina`
  - `layer=source`
  - `storage=s3`

## Asset 设计

`sina__trade_calendar`：

- 类型：普通不分区 asset。
- 输入：无上游 Dagster asset。
- 输出：交易日历二维数组。
- IO manager：`s3_io_manager`。
- 存储位置：`source/sina__trade_calendar/000000_0.parquet`。
- 存储格式：Parquet。
- 压缩格式：zstd。

asset 执行步骤：

1. 使用 `requests` 发送 GET 请求。
2. 使用 Chrome UA。
3. 设置合理超时，例如 connect timeout 和 read timeout。
4. 请求超时、连接错误或 HTTP 非 2xx 时按 1、2、4 秒进行最多 3 次重试。
5. 重试耗尽后仍失败则让 asset 失败。
6. 将响应文本传入 `SinaCalendarParser`。
7. 验证解析结果非空。
8. 返回解析结果，由 `s3_io_manager` 以 Parquet + zstd 写入 S3。

Dagster metadata 建议：

- `source_url`
- `row_count`
- `min_trade_date`
- `max_trade_date`
- `s3_bucket`
- `s3_key`
- `file_format=parquet`
- `compression=zstd`
- `retry_policy=exponential_backoff_1_2_4`

## 调度设计

RFC 0001 中 `sina__trade_calendar` 的调度是“每年的最后一周”。

第一版建议：

- 使用 Dagster `ScheduleDefinition`。
- execution timezone 使用 `Asia/Shanghai`。
- cron 设置为每年 12 月最后一周内的固定时间运行。

可选实现方式：

- 更严格方案：12 月 25-31 日每天 09:00 触发，schedule 内判断是否为最后一周需要执行的目标日期。


原因：

- 新浪接口返回到当前年份年末的交易日历。
- 交易日历资产不需要按交易日频繁刷新。

## 依赖与配置计划

Python 依赖建议：

- `requests`
- S3 客户端库，例如 `boto3`
- Parquet 写入库，例如 `pyarrow`

配置通过环境变量注入，不在代码中硬编码凭据。

本地运行前置条件：

- `deploy/docker-compose.yml` 中的 `rustfs` 已启动。
- 已配置 `RUSTFS_ACCESS_KEY` 和 `RUSTFS_SECRET_KEY`。
- bucket `mono-fleur` 已存在，或 `s3_io_manager` 在启动时确保 bucket 存在。

## 验收标准

设计实现完成后应满足：

- Dagster UI 中能看到 `sina__trade_calendar` asset。
- 正常情况下 materialize 该 asset 时只访问新浪交易日历接口一次。
- 请求超时或错误时按 1、2、4 秒退避重试，最多 3 次重试。
- 请求使用 Chrome UA。
- parser 能解析出从 `1990-12-19` 开始的交易日数组。
- 输出包含 `1992-05-04`。
- 成功运行后 S3 中存在 `source/sina__trade_calendar/000000_0.parquet`。
- `sina__trade_calendar` 不使用 Dagster 分区；S3 对象 key 不包含分区目录。
- S3 对象是 Parquet 格式，并使用 zstd 压缩。
- Dagster materialization metadata 中能看到行数、日期范围、S3 对象位置、文件格式和压缩格式。
- 请求或解析失败时 run 失败，不覆盖已有 S3 对象。

## 实施顺序

1. 增加 `http_resources` defs 子目录和 `io_managers` 目录。
2. 增加 `SinaCalendarParser`。
3. 增加支持 Parquet + zstd 的 `s3_io_manager`。
4. 增加 `sina__trade_calendar` asset。
5. 注册资源并绑定 asset 的 `io_manager_key`。
6. 增加年度刷新 schedule。
7. 使用 `uv run dg check` 校验 Dagster definitions。
8. 手动 materialize `sina__trade_calendar` 并检查 S3 对象。

## 后续扩展

后续 RFC 0001 的 BaoStock 资产可以复用本设计中的存储和组织方式：

- `baostock__query_stock_basic` 作为不分区快照资产。
- `baostock__query_history_k_data_plus_daily` 作为按 `trade_date` 分区的 raw parquet 资产。
- `s3_io_manager` 扩展支持不同对象 key 模板、Hive 分区字段和 Parquet schema。
