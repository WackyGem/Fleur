# ADR 0006: ClickHouse raw sync 使用官方 Python HTTP client

状态：Accepted

日期：2026-05-31

## 背景

ADR 0005 已决定 ClickHouse raw 层写入由 Dagster raw sync assets 负责，dbt 只负责 staging/marts 建模。RFC 0009 进一步要求 raw sync 实现 staging 装载、schema/row count 校验、分区替换或 snapshot 替换，并记录可排查的 materialization metadata。

实现 raw sync resource 时需要选择 ClickHouse client：

- 手写 HTTP client。
- 使用 native TCP client。
- 引入官方 Python client。

当前 `pipeline/scheduler` 尚未引入 ClickHouse Python 依赖。已有依赖包括 Dagster、psycopg、pyarrow、aiohttp 等。

## 决策

ClickHouse raw sync 使用官方 Python client `clickhouse-connect`。该 client 使用 ClickHouse HTTP interface。

不在第一阶段使用手写 HTTP client，也不使用 native TCP client 作为默认路径。

实现边界：

- 在 `pipeline/scheduler/src/scheduler/defs/resources/clickhouse.py` 中新增 `ClickHouseResource`。
- resource 负责读取 `CLICKHOUSE_*` 环境配置并构造 `clickhouse-connect` client。
- 业务 asset 和 raw sync service 不直接依赖 `clickhouse_connect.Client`，而是依赖项目内定义的窄协议。
- 第一阶段 protocol 只暴露 raw sync 需要的能力，例如 `command()`、`query()`、`ping()`、`server_version` 和关闭连接。
- raw sync 主路径让 ClickHouse 通过 `s3()` 读取 S3 Parquet，不默认把 Parquet 读入 Python 后再写入 ClickHouse。
- `insert_arrow()` 可作为测试工具或 fallback，但不是生产 raw sync 的默认装载路径。

建议的内部协议形态：

```python
class ClickHouseClientProtocol(Protocol):
    @property
    def server_version(self) -> str: ...

    def ping(self) -> bool: ...

    def command(
        self,
        sql: str,
        *,
        settings: Mapping[str, object] | None = None,
    ) -> object: ...

    def query(
        self,
        sql: str,
        *,
        settings: Mapping[str, object] | None = None,
    ) -> ClickHouseQueryResult: ...

    def close(self) -> None: ...
```

## 依据

- `clickhouse-connect` 是 ClickHouse 官方 Python driver，支持 HTTP interface、TLS、压缩、连接超时、查询超时、query settings、SQL command/query，以及 PyArrow insert。
- ClickHouse HTTP interface 更适合负载均衡、防火墙、代理和 ClickHouse Cloud 风格部署。
- 当前 raw sync 的主要数据流是 ClickHouse server-side `INSERT ... SELECT ... FROM s3(...)`。Python 侧主要发起 DDL/DML、校验查询并记录 metadata，native TCP 的吞吐优势不是当前瓶颈。
- 手写 HTTP client 会把认证、settings、timeout、错误解析、结果格式、连接关闭和后续兼容性维护留给项目自己承担，收益不足。
- native TCP client 可在高吞吐客户端直写、需要 native 协议高级能力、且部署环境稳定开放 native 端口时再评估。

## 后果

- `pipeline/scheduler` 需要新增 `clickhouse-connect` 依赖。
- ClickHouse 连接配置集中在 resource，不允许业务 asset 直接读取 `CLICKHOUSE_*` 环境变量。
- raw sync service 可以使用 fake protocol 做单元测试，不需要在测试中启动 ClickHouse。
- 生产装载优先通过 ClickHouse `s3()` table function 从 S3 读取 Parquet，减少 Dagster worker 的数据面压力。
- 如果后续指标显示 HTTP client 成为瓶颈，或确实需要 native 协议能力，应新增 RFC/ADR 重新评估 native client，不能在业务代码中局部绕过本 ADR。
