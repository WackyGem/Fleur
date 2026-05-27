# ADR 0004: BaoStock TCP 客户端、分页聚合和日 K 范围过滤

状态：Accepted

日期：2026-05-27

## 背景

BaoStock 行情接口通过 TCP 协议提供数据。日频 K 线历史回填如果按“证券代码 + 单日”请求，会导致请求量过大。RFC 0001 和 Plan 0002 明确要求按“单个 code + 日期范围”请求，并使用证券基础信息缩小请求范围。

## 决策

BaoStock 访问通过异步 TCP 客户端统一处理协议、登录、连接池、分页和重试。

- 客户端在启动时登录，不在每个 API 请求前重复登录。
- 登录状态按本地 TTL 维护，遇到未登录错误码时刷新登录并重试一次。
- TCP 连接通过连接池复用，并用 semaphore 限制最大并发连接数。
- 网络错误、超时和协议错误会使连接不可复用，并触发退避重试。
- 分页接口由客户端聚合为单个 `BaostockResponse`，asset 层不处理分页细节。
- `query_stock_basic` 使用 BaoStock `query_stock_basic`。
- 日频 K 线使用 BaoStock `query_history_k_data_plus`，固定参数为 `frequency=d` 和 `adjustflag=3`。

日频 K 线资产采用年度分区：

```text
baostock__query_history_k_data_plus_daily
```

分区定义从 1990 年开始，分区键为年份字符串。日常刷新只刷新当年分区，并通过 `refresh_until_trade_date` 将请求截止到当前交易日；历史回填可以显式 materialize 目标年份分区。

日频 K 线请求范围由 `filter_active_security_ranges` 生成：

- 输入来自已物化的 `baostock__query_stock_basic` 快照。
- 必需字段为 `code`、`ipoDate`、`outDate`、`type`。
- 默认只允许 BaoStock `type` 为 `"1"`、`"2"`、`"5"` 的证券。
- 类型起始数据日期为：
  - `"1"` 股票：`1990-12-19`
  - `"2"` 指数：`2006-01-01`
  - `"5"` ETF：`2026-01-05`
- 实际请求区间是请求年份范围、证券上市日期、类型起始日期和退市日期的交集。
- 不请求 `type="3"` 其它和 `type="4"` 可转债，除非后续明确补充其日频 K 线可用范围。

## 依据

当前实现：

- `pipeline/scheduler/src/scheduler/defs/baostock/client.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/protocol.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/assets.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/schemas.py`
- `filter_active_security_ranges`
- `BAOSTOCK_SECURITY_TYPE_DATA_START_DATES`

当前测试覆盖：

- BaoStock request encode/decode
- 压缩响应 decode
- 日频 K 线参数保留 `d` 和 `3`
- 证券范围过滤交集和 ETF 起始日期

设计来源：

- `docs/RFC/0001-market-data-ingestion.md`
- `docs/plans/0002-baostock-aio-tcp-client.md`

## 后果

- asset 层只表达“采哪个资产、哪个分区、哪个日期范围”，协议和分页复杂度集中在客户端。
- 历史回填按年度分区和证券有效日期范围批量请求，避免按单日请求导致请求量爆炸。
- K 线资产依赖 S3 中的证券基础信息快照和交易日历快照。缺失这些上游对象时应失败或跳过，而不是临时请求远端补齐。
- 扩展到新 BaoStock API 时，应优先复用当前客户端、分页聚合和 schema 转换模式。
