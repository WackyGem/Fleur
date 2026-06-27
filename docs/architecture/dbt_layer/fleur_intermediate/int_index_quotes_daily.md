# int_index_quotes_daily 设计

状态：Design

依据：

- Upstream model：`ref('int_index_basic_snapshot')`
- Staging model：`ref('stg_baostock__query_history_k_data_plus_daily')`
- 目标位置：`pipeline/elt/models/intermediate/int_index_quotes_daily.sql`
- 实施计划：`docs/plans/0042-index-benchmark-intermediate-implementation-plan.md`

## 1. 模型定位

BaoStock 指数日频价格行情 intermediate 模型。模型使用 `int_index_basic_snapshot` 限定指数 universe，再读取 BaoStock 日行情 staging，输出指数日行情和价格指数简单日收益。

本模型不输出本地指数代码和交易所代码；需要这些维度时由下游通过 `security_code` join `int_index_basic_snapshot`。

## 2. 数据粒度与依赖

- 直接依赖：`int_index_basic_snapshot`、`stg_baostock__query_history_k_data_plus_daily`。
- 粒度：一行代表一个 `security_code` 在一个 `trade_date` 的指数日行情。
- 候选键：`security_code`, `trade_date`。
- 收益口径：价格指数简单收益，不包含分红再投资。

## 3. 字段设计

| 字段 | 来源/派生 | 类型建议 | 设计说明 |
|---|---|---|---|
| `security_code` | `stg_baostock__query_history_k_data_plus_daily.security_code` | `String` | canonical 指数代码。 |
| `trade_date` | `stg_baostock__query_history_k_data_plus_daily.trade_date` | `Date` | 交易日。 |
| `open_price` | staging | `Nullable(Float64)` | 开盘点位。 |
| `high_price` | staging | `Nullable(Float64)` | 最高点位。 |
| `low_price` | staging | `Nullable(Float64)` | 最低点位。 |
| `close_price` | staging | `Nullable(Float64)` | 收盘点位。 |
| `prev_close_price` | staging | `Nullable(Float64)` | BaoStock 原始 preclose 口径前收盘点位。 |
| `return_daily` | `close_price / prev_close_price - 1` | `Nullable(Float64)` | 价格指数简单日收益；前收盘点位缺失或小于等于 0 时为 NULL。 |
| `volume` | staging | `Nullable(Int64)` | 成交量，沿用 BaoStock source-local 口径。 |
| `amount` | staging | `Nullable(Float64)` | 成交金额，沿用 BaoStock source-local 口径。 |

## 4. SQL 逻辑

```sql
with index_universe as (
    select security_code
    from {{ ref('int_index_basic_snapshot') }}
)

select ...
from {{ ref('stg_baostock__query_history_k_data_plus_daily') }} as quotes
inner join index_universe
    on quotes.security_code = index_universe.security_code
```

实现注意：

- 不从 raw 表直接读取。
- 不重复输出 `security_local_code` 和 `exchange_code`。
- `return_daily` 使用简单收益，不使用百分数口径。

## 5. 测试建议

- 组合键 `security_code`, `trade_date`: 唯一。
- `security_code`: `not_null`，`cn_security_code_format`，relationships 到 `int_index_basic_snapshot.security_code`。
- `trade_date`: `not_null`。

## 6. 延后事项

- 指数停牌、缺口和异常行情的业务修正。
- 全收益指数或分红再投资 benchmark 构造。
