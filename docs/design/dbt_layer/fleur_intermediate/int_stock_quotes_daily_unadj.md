# int_stock_quotes_daily_unadj 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_basic_snapshot')`
- Staging model：`ref('stg_baostock__query_history_k_data_plus_daily')`
- Intermediate model：`ref('int_trade_calendar')`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_basic_snapshot.md`
- Staging 设计：`docs/design/dbt_layer/fleur_staging/stg_baostock__query_history_k_data_plus_daily.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_trade_calendar.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`

## 1. 模型定位

A 股股票日频行情 intermediate 模型。模型从 BaoStock 未复权日频 K 线 staging 中取行情事实，并通过股票基础信息快照 intermediate 限定股票 universe，输出只包含股票代码且交易日大于 `1995-01-01` 的未复权日频行情记录。

本模型负责把 source-local 未复权日 K 数据收敛为“股票未复权日行情”事实，并补充一个明确语义的未复权前一交易日收盘价字段：`prev_close_price_unadj`。停牌布尔字段 `is_suspend` 由上游 staging 从 BaoStock `trade_status` 派生后透传。

本模型不做复权价格计算、行情异常修正、跨源停牌口径裁决、跨源行情对账或证券主数据最终裁决。`is_suspend` 仅表达上游 BaoStock staging 的 source-local 停牌语义。

## 2. 数据粒度与依赖

- 主行情依赖：`stg_baostock__query_history_k_data_plus_daily`。
- 股票过滤依赖：`int_stock_basic_snapshot`。
- 前交易日依赖：`int_trade_calendar`。
- 粒度：一行一个 `security_code` + `trade_date` 的股票日频行情记录。
- 候选键：`security_code`, `trade_date`。

股票过滤规则：

- 使用 `int_stock_basic_snapshot.security_code` 作为股票 universe。
- `stg_baostock__query_history_k_data_plus_daily.security_code` 必须命中 `int_stock_basic_snapshot` 后才进入本模型。
- 第一版不使用 `int_stock_basic_snapshot.listing_status` 或 `is_listed` 过滤行情记录；已退市股票的历史行情仍应保留。是否限制上市区间由后续证券主数据模型决定。

复权口径规则：

- `stg_baostock__query_history_k_data_plus_daily` 已只输出 `adjustflag = 3` 的未复权行情。
- `adjust_flag` 不输出到 staging 或本模型。
- 如后续需要前复权、后复权或多复权口径并存，应新增或调整专门模型，不在本模型中混合输出。

交易制度日期规则：

- 只保留 `trade_date > '1995-01-01'` 的 K 线行情。
- `1995-01-01` 是 A 股恢复 T+1 交易制度的日期。本模型第一版聚焦恢复 T+1 后的行情，避免把早期 T+0 制度时期行情混入统一日行情事实。

## 3. `prev_close_price_unadj` 设计判断

需要依赖 `int_trade_calendar` 获取 `prev_close_price_unadj`。

原因：

- `prev_close_price_unadj` 的字段含义是“当前 A 股交易日的前一个交易日，该证券的未复权收盘价”。
- `stg_baostock__query_history_k_data_plus_daily.prev_close_price` 来自 BaoStock 原始 `preclose` 字段，属于 source-local 的“前收盘价”口径。该字段可用于保留供应商原始口径，但不能替代本模型要表达的“前一个交易日的实际收盘价”。
- 如果只对单只证券按 `trade_date` 做 `lag(close_price)`，得到的是“该证券上一条可见行情记录的收盘价”。当 source 缺行、上市首日前后、停牌记录缺失或后续接入其他行情源时，它不一定等于 A 股交易日历中的前一个交易日。
- 因此本模型应以 `int_trade_calendar.prev_trade_date` 定义“前一个交易日”，再按 `security_code` + `prev_trade_date` 自连接行情表取 `close_price`。

NULL 语义：

- 该证券首个交易日没有可用前一交易日行情时，`prev_close_price_unadj` 为 `NULL`。
- 如果 `int_trade_calendar.prev_trade_date` 存在，但该证券在前一交易日没有行情行，`prev_close_price_unadj` 为 `NULL`，用于显式暴露 source 缺口或证券未上市等情况。
- 不用 `prev_close_price` 回填 `prev_close_price_unadj`，避免混淆供应商“前收盘价”和“前一交易日收盘价”两个口径。

## 4. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | `stg_baostock__query_history_k_data_plus_daily.security_code` | `String` | 股票标准连接代码。只保留命中股票 universe 的证券。 |
| `trade_date` | `stg_baostock__query_history_k_data_plus_daily.trade_date` | `Date` | 行情交易日期。 |
| `open_price` | `stg_baostock__query_history_k_data_plus_daily.open_price` | `Nullable(Float64)` | 交易日开盘价，沿用 staging source-local 字段。 |
| `high_price` | `stg_baostock__query_history_k_data_plus_daily.high_price` | `Nullable(Float64)` | 交易日最高价，沿用 staging source-local 字段。 |
| `low_price` | `stg_baostock__query_history_k_data_plus_daily.low_price` | `Nullable(Float64)` | 交易日最低价，沿用 staging source-local 字段。 |
| `close_price` | `stg_baostock__query_history_k_data_plus_daily.close_price` | `Nullable(Float64)` | 交易日收盘价，上游 BaoStock staging 已限定未复权口径。 |
| `prev_close_price` | `stg_baostock__query_history_k_data_plus_daily.prev_close_price` | `Nullable(Float64)` | BaoStock 原始 `preclose` 口径的前收盘价，保留用于对账和源口径追踪。 |
| `prev_close_price_unadj` | 前一交易日行情自连接 `close_price` | `Nullable(Float64)` | 当前证券在 `int_trade_calendar.prev_trade_date` 对应日期的未复权收盘价。 |
| `volume` | `stg_baostock__query_history_k_data_plus_daily.volume` | `Nullable(Int64)` | 交易日成交量，0 值保留。 |
| `amount` | `stg_baostock__query_history_k_data_plus_daily.amount` | `Nullable(Float64)` | 交易日成交金额，单位沿用 source-local 口径。 |
| `is_suspend` | `stg_baostock__query_history_k_data_plus_daily.is_suspend` | `Bool` | 是否停牌。由上游 staging 从 BaoStock `trade_status` 派生；不输出原始 `trade_status`。 |
| `is_st` | `stg_baostock__query_history_k_data_plus_daily.is_st` | `Nullable(Bool)` | 是否 ST 或风险警示，沿用 staging。 |

字段顺序建议：

1. 主键字段：`security_code`, `trade_date`
2. OHLC 价格字段：`open_price`, `high_price`, `low_price`, `close_price`
3. 前收盘价字段：`prev_close_price`, `prev_close_price_unadj`
4. 成交与状态字段：`volume`, `amount`, `is_suspend`, `is_st`

## 5. SQL 逻辑建议

```sql
with stock_universe as (
    select
        security_code
    from {{ ref('int_stock_basic_snapshot') }}
),

stock_quotes as (
    select
        quotes.security_code,
        quotes.trade_date,
        quotes.open_price,
        quotes.high_price,
        quotes.low_price,
        quotes.close_price,
        quotes.prev_close_price,
        quotes.volume,
        quotes.amount,
        quotes.is_suspend,
        quotes.is_st
    from {{ ref('stg_baostock__query_history_k_data_plus_daily') }} as quotes
    inner join stock_universe
        on quotes.security_code = stock_universe.security_code
    where quotes.trade_date > toDate('1995-01-01')
),

quotes_with_prev_trade_date as (
    select
        stock_quotes.security_code,
        stock_quotes.trade_date,
        stock_quotes.open_price,
        stock_quotes.high_price,
        stock_quotes.low_price,
        stock_quotes.close_price,
        stock_quotes.prev_close_price,
        stock_quotes.volume,
        stock_quotes.amount,
        stock_quotes.is_suspend,
        stock_quotes.is_st,
        trade_calendar.prev_trade_date
    from stock_quotes
    left join {{ ref('int_trade_calendar') }} as trade_calendar
        on stock_quotes.trade_date = trade_calendar.trade_date
),

quotes_with_prev_close_unadj as (
    select
        current_quotes.security_code,
        current_quotes.trade_date,
        current_quotes.open_price,
        current_quotes.high_price,
        current_quotes.low_price,
        current_quotes.close_price,
        current_quotes.prev_close_price,
        previous_quotes.close_price as prev_close_price_unadj,
        current_quotes.volume,
        current_quotes.amount,
        current_quotes.is_suspend,
        current_quotes.is_st
    from quotes_with_prev_trade_date as current_quotes
    left join stock_quotes as previous_quotes
        on current_quotes.security_code = previous_quotes.security_code
        and current_quotes.prev_trade_date = previous_quotes.trade_date
)

select
    security_code,
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    prev_close_price,
    prev_close_price_unadj,
    volume,
    amount,
    is_suspend,
    is_st
from quotes_with_prev_close_unadj
```

实现注意：

- `int_trade_calendar` 应作为前交易日定义来源，不在本模型内重复窗口计算交易日序列。
- `int_stock_basic_snapshot` 应作为股票 universe 来源，不在本模型内重复读取证券基础信息 staging。
- 从 staging 取行情时应过滤 `trade_date > toDate('1995-01-01')`，只保留 A 股恢复 T+1 后的交易制度时期。
- 自连接应使用已经过滤后的 `stock_quotes`，避免指数、ETF、可转债行情进入前值查找。
- `adjustflag` 过滤和 `trade_status` 到 `is_suspend` 的派生已在上游 staging 完成。
- `turnover_rate` 和 `pct_change` 不输出到第一版模型；如后续下游需要，应单独评估 source-local 口径、单位和重算策略。
- `prev_trade_date` 仅作为中间计算字段，不建议输出到第一版模型；如后续调试或下游需要，可单独设计字段。

## 6. 测试建议

- 模型级组合唯一：`security_code`, `trade_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- `is_suspend`: `not_null`。
- `prev_close_price_unadj`: 不加 `not_null`，因为首个交易日、证券前一交易日无行情、source 缺口都应允许为 `NULL`。
- 增加定向数据测试：
  - 输出证券必须存在于 `int_stock_basic_snapshot`。
  - 输出记录继承上游 staging 的未复权口径。
  - `trade_date` 必须大于 `1995-01-01`。
  - `is_suspend` 由上游 staging 保证与源 `trade_status` 映射一致：`trade_status = 0` 时为 `true`，`trade_status = 1` 时为 `false`。
  - 除 `int_trade_calendar.prev_trade_date is null` 或前一交易日缺行情外，`prev_close_price_unadj` 应等于同一证券前一交易日的 `close_price`。
  - `prev_close_price_unadj` 与 `prev_close_price` 不要求一致；两者口径不同，不能用相等性测试约束。

## 7. 延后事项

- 基于上市/退市日期裁剪行情范围。
- 输出 `prev_trade_date` 或其他交易日派生字段。
- 输出 `adjust_flag`、`trade_status`、`turnover_rate` 或 `pct_change`。
- 复权行情、复权因子和多复权口径支持。
- 停牌日价格、成交量和涨跌幅的业务裁决。
- 与其他行情源的优先级合并和对账。
- 股票 universe 的跨源主数据裁决、北交所扩展和历史证券类型变化处理。
