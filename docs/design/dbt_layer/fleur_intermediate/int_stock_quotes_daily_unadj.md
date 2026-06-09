# int_stock_quotes_daily_unadj 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_basic_snapshot')`
- Staging model：`ref('stg_baostock__query_history_k_data_plus_daily')`
- Intermediate model：`ref('int_trade_calendar')`
- Intermediate model：`ref('int_stock_shares_history')`
- Intermediate model：`ref('int_stock_exrights_event')`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_basic_snapshot.md`
- Staging 设计：`docs/design/dbt_layer/fleur_staging/stg_baostock__query_history_k_data_plus_daily.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_trade_calendar.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_shares_history.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_exrights_event.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_quotes_daily_unadj.sql`

## 1. 模型定位

A 股股票日频行情 intermediate 模型。模型从 BaoStock 未复权日频 K 线 staging 中取行情事实，并通过股票基础信息快照 intermediate 限定股票 universe，输出只包含股票代码且交易日大于 `1995-01-01` 的未复权日频行情记录。

本模型负责把 source-local 未复权日 K 数据收敛为“股票未复权日行情”事实，并补充明确语义的前一交易日字段：`prev_close_price_unadj` 和 `prev_volume`。在此基础上，本模型按日频粒度派生换手率、涨跌幅、振幅、涨跌停价、A 股市值、A 股股本和股息率字段，供 mart 层直接复用。停牌布尔字段 `is_suspend` 由上游 staging 从 BaoStock `trade_status` 派生后透传。

本模型不做复权价格计算、行情异常修正、跨源停牌口径裁决、跨源行情对账、财报估值计算或证券主数据最终裁决。`is_suspend` 仅表达上游 BaoStock staging 的 source-local 停牌语义。

## 2. 数据粒度与依赖

- 主行情依赖：`stg_baostock__query_history_k_data_plus_daily`。
- 股票过滤依赖：`int_stock_basic_snapshot`。
- 前交易日依赖：`int_trade_calendar`。
- 股本区间依赖：`int_stock_shares_history`。
- 股息率依赖：`int_stock_exrights_event`。
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

## 3. 前一交易日字段设计判断

需要依赖 `int_trade_calendar` 获取 `prev_close_price_unadj` 和 `prev_volume`。

原因：

- `prev_close_price_unadj` 的字段含义是“当前 A 股交易日的前一个交易日，该证券的未复权收盘价”。
- `prev_volume` 的字段含义是“当前 A 股交易日的前一个交易日，该证券的成交量”，与 `prev_close_price_unadj` 使用同一交易日历口径。
- `stg_baostock__query_history_k_data_plus_daily.prev_close_price` 来自 BaoStock 原始 `preclose` 字段，属于 source-local 的“前收盘价”口径。该字段可用于保留供应商原始口径，但不能替代本模型要表达的“前一个交易日的实际收盘价”。
- 如果只对单只证券按 `trade_date` 做 `lag(close_price)`，得到的是“该证券上一条可见行情记录的收盘价”。当 source 缺行、上市首日前后、停牌记录缺失或后续接入其他行情源时，它不一定等于 A 股交易日历中的前一个交易日。
- 因此本模型应以 `int_trade_calendar.prev_trade_date` 定义“前一个交易日”，再按 `security_code` + `prev_trade_date` 自连接行情表取 `close_price` 和 `volume`。

NULL 语义：

- 该证券首个交易日没有可用前一交易日行情时，`prev_close_price_unadj` 为 `NULL`。
- 如果 `int_trade_calendar.prev_trade_date` 存在，但该证券在前一交易日没有行情行，`prev_close_price_unadj` 和 `prev_volume` 为 `NULL`，用于显式暴露 source 缺口或证券未上市等情况。
- 如果前一交易日行情行存在且 `volume = 0`，`prev_volume` 保留 `0`，不转换为 `NULL`。
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
| `prev_volume` | 前一交易日行情自连接 `volume` | `Nullable(Int64)` | 当前证券在 `int_trade_calendar.prev_trade_date` 对应日期的成交量，0 值保留。 |
| `volume` | `stg_baostock__query_history_k_data_plus_daily.volume` | `Nullable(Int64)` | 交易日成交量，0 值保留。 |
| `amount` | `stg_baostock__query_history_k_data_plus_daily.amount` | `Nullable(Float64)` | 交易日成交金额，单位沿用 source-local 口径。 |
| `turnover_rate` | `volume / a_float_shares * 100` | `Nullable(Float64)` | 换手率，百分数口径，`1.23` 表示 `1.23%`。 |
| `turnover_rate_actual` | `volume / a_free_float_shares * 100` | `Nullable(Float64)` | 实际换手率，使用 A 股自由流通股本作分母，百分数口径。 |
| `pct_amplitude` | `(high_price - low_price) / prev_close_price * 100` | `Nullable(Float64)` | 振幅，使用 BaoStock `preclose` 口径前收盘价计算，百分数口径。 |
| `pct_change` | `(close_price - prev_close_price) / prev_close_price * 100` | `Nullable(Float64)` | 涨跌幅，使用 BaoStock `preclose` 口径前收盘价计算，百分数口径。 |
| `limit_up_price` | `prev_close_price * (1 + price_limit_ratio)` | `Nullable(Float64)` | 涨停价，基于 BaoStock `preclose` 口径前收盘价按涨跌停比例四舍五入到分。 |
| `limit_down_price` | `prev_close_price * (1 - price_limit_ratio)` | `Nullable(Float64)` | 跌停价，基于 BaoStock `preclose` 口径前收盘价按涨跌停比例四舍五入到分。 |
| `a_market_cap` | `close_price * a_shares` | `Nullable(Float64)` | A 股市值，单位元。 |
| `a_float_market_cap` | `close_price * a_float_shares` | `Nullable(Float64)` | A 股流通市值，单位元。 |
| `a_free_float_market_cap` | `close_price * a_free_float_shares` | `Nullable(Float64)` | A 股自由流通市值，单位元。 |
| `a_shares` | `int_stock_shares_history.a_shares` | `Nullable(Float64)` | A 股股本，单位股。 |
| `a_float_shares` | `int_stock_shares_history.a_float_shares` | `Nullable(Float64)` | A 股流通股，单位股。 |
| `a_free_float_shares` | `int_stock_shares_history.a_free_float_shares` | `Nullable(Float64)` | A 股自由流通股，单位股。 |
| `dy_static` | 最近年度现金分红 / `close_price * 100` | `Nullable(Float64)` | 股息率（静），百分数口径。 |
| `dy_ttm` | 近 12 个月现金分红 / `close_price * 100` | `Nullable(Float64)` | 股息率（TTM），百分数口径。 |
| `is_suspend` | `stg_baostock__query_history_k_data_plus_daily.is_suspend` | `Bool` | 是否停牌。由上游 staging 从 BaoStock `trade_status` 派生；不输出原始 `trade_status`。 |
| `is_st` | `stg_baostock__query_history_k_data_plus_daily.is_st` | `Nullable(Bool)` | 是否 ST 或风险警示，沿用 staging。 |

字段顺序建议：

1. 主键字段：`security_code`, `trade_date`
2. OHLC 价格字段：`open_price`, `high_price`, `low_price`, `close_price`
3. 前收盘价和前成交量字段：`prev_close_price`, `prev_close_price_unadj`, `prev_volume`
4. 成交与日频交易指标：`volume`, `amount`, `turnover_rate`, `turnover_rate_actual`, `pct_amplitude`, `pct_change`
5. 涨跌停价：`limit_up_price`, `limit_down_price`
6. 市值与股本：`a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap`, `a_shares`, `a_float_shares`, `a_free_float_shares`
7. 股息率：`dy_static`, `dy_ttm`
8. 状态字段：`is_suspend`, `is_st`

## 5. 派生指标口径

### 5.1 股本 as-of 规则

股本字段来自 `int_stock_shares_history` 的 as-of 区间：

```text
quotes.trade_date >= shares.effective_date
and (
    shares.expiry_date is null
    or quotes.trade_date <= shares.expiry_date
)
```

`a_shares`、`a_float_shares`、`a_free_float_shares` 直接透传该区间表字段。若同一证券同一交易日无法命中股本区间，股本、市值和换手率字段均输出 `NULL`，行情行仍保留。

### 5.2 换手率、涨跌幅和振幅

```text
turnover_rate = volume / a_float_shares * 100
turnover_rate_actual = volume / a_free_float_shares * 100
pct_amplitude = (high_price - low_price) / prev_close_price * 100
pct_change = (close_price - prev_close_price) / prev_close_price * 100
```

规则：

- 百分比字段保留百分数口径，`1.23` 表示 `1.23%`，不使用 `0.0123`。
- 当分母为 `NULL`、`0` 或小于 `0` 时输出 `NULL`。
- 当 `volume` 为 `NULL` 时换手率输出 `NULL`；`volume = 0` 时换手率可为 `0`。
- `pct_amplitude` 和 `pct_change` 使用 BaoStock source-local `prev_close_price`，与交易所涨跌幅和涨跌停参考价口径保持一致。

### 5.3 涨跌停价

```text
limit_up_price = round(prev_close_price * (1 + price_limit_ratio), 2)
limit_down_price = round(prev_close_price * (1 - price_limit_ratio), 2)
```

`price_limit_ratio` 第一版规则：

| 条件 | `price_limit_ratio` |
|------|----------------------|
| `security_board in ('sse_main_board', 'szse_main_board')` 且 `is_st = true` 且 `trade_date < '2026-07-06'` | `0.05` |
| `security_board in ('sse_main_board', 'szse_main_board')` 且 `is_st = true` 且 `trade_date >= '2026-07-06'` | `0.10` |
| `security_board in ('sse_main_board', 'szse_main_board')` | `0.10` |
| `security_board = 'star_market'` | `0.20` |
| `security_board = 'chinext'` | `0.20` |

说明：

- `security_board` 来自 `int_stock_basic_snapshot`。
- 主板风险警示股票涨跌幅限制于 `2026-07-06` 起由 `5%` 调整为 `10%`，因此 `sse_main_board` 和 `szse_main_board` 的 ST/*ST 股票必须按 `trade_date` 分段计算。
- 科创板和创业板股票按 `20%` 计算；主板非风险警示股票按 `10%` 计算。
- 本模型第一版只对主板风险警示股票做 ST 分段；非主板风险警示股票按对应板块比例处理。
- 当前 `int_stock_basic_snapshot.security_board` 尚未覆盖北交所；北交所 `30%` 规则延后。
- `prev_close_price` 为 `NULL`、`0` 或小于 `0` 时，涨跌停价输出 `NULL`。
- 涨跌停价按人民币价格最小变动单位四舍五入到 2 位小数。若后续需要精确复刻交易所特殊舍入或无涨跌幅限制日，应新增专项规则，不在本模型中静默近似。

### 5.4 市值

```text
a_market_cap = close_price * a_shares
a_float_market_cap = close_price * a_float_shares
a_free_float_market_cap = close_price * a_free_float_shares
```

规则：

- 市值单位为元。
- 股本单位为股。
- 价格使用未复权 `close_price`。未复权价格与当日股本匹配时适合计算当日市值。
- 任一输入为 `NULL`、价格小于 `0` 或股本小于等于 `0` 时，对应市值输出 `NULL`。

### 5.5 股息率

股息率来自 `int_stock_exrights_event` 的已实施现金分红事件，使用 `cash_dividend_per_share` 和除权除息日 `ex_dividend_date` 做 as-of 计算。

```text
dy_static =
    latest_annual_cash_dividend_per_share_as_of_trade_date
    / close_price * 100

dy_ttm =
    sum(cash_dividend_per_share where ex_dividend_date in last 12 months)
    / close_price * 100
```

规则：

- `dy_static` 使用同一证券 `ex_dividend_date <= trade_date` 的最近年度现金分红。年度现金分红定义为 `report_date` 为自然年末，即 `toMonth(report_date) = 12 and toDayOfMonth(report_date) = 31` 的现金分红事件；同一 `security_code + report_date` 如有多条现金分红事件，先按报告期求和。
- `dy_ttm` 使用 `trade_date - interval 1 year < ex_dividend_date <= trade_date` 的现金分红事件合计。
- 股息率使用 `close_price` 作分母，保留百分数口径。
- 当现金分红输入缺失时输出 `NULL`；当现金分红为 `0` 时可输出 `0`。
- 当 `close_price` 为 `NULL`、`0` 或小于 `0` 时，`dy_static` 和 `dy_ttm` 输出 `NULL`。

## 6. SQL 逻辑建议

```sql
with stock_universe as (
    select
        security_code,
        security_board
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
        quotes.is_st,
        stock_universe.security_board
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
        stock_quotes.security_board,
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
        previous_quotes.volume as prev_volume,
        current_quotes.volume,
        current_quotes.amount,
        current_quotes.is_suspend,
        current_quotes.is_st,
        current_quotes.security_board
    from quotes_with_prev_trade_date as current_quotes
    left join stock_quotes as previous_quotes
        on current_quotes.security_code = previous_quotes.security_code
        and current_quotes.prev_trade_date = previous_quotes.trade_date
),

quotes_with_shares as (
    select
        quotes_with_prev_close_unadj.security_code,
        quotes_with_prev_close_unadj.trade_date,
        quotes_with_prev_close_unadj.open_price,
        quotes_with_prev_close_unadj.high_price,
        quotes_with_prev_close_unadj.low_price,
        quotes_with_prev_close_unadj.close_price,
        quotes_with_prev_close_unadj.prev_close_price,
        quotes_with_prev_close_unadj.prev_close_price_unadj,
        quotes_with_prev_close_unadj.prev_volume,
        quotes_with_prev_close_unadj.volume,
        quotes_with_prev_close_unadj.amount,
        quotes_with_prev_close_unadj.is_suspend,
        quotes_with_prev_close_unadj.is_st,
        quotes_with_prev_close_unadj.security_board,
        shares.a_shares,
        shares.a_float_shares,
        shares.a_free_float_shares
    from quotes_with_prev_close_unadj
    left any join {{ ref('int_stock_shares_history') }} as shares
        on quotes_with_prev_close_unadj.security_code = shares.security_code
        and quotes_with_prev_close_unadj.trade_date >= shares.effective_date
        and (
            shares.expiry_date is null
            or quotes_with_prev_close_unadj.trade_date <= shares.expiry_date
        )
),

cash_dividends as (
    select
        security_code,
        ex_dividend_date,
        report_date,
        cash_dividend_per_share
    from {{ ref('int_stock_exrights_event') }}
    where has_cash_dividend = true
      and cash_dividend_per_share > 0
),

annual_cash_dividends as (
    select
        security_code,
        report_date,
        sum(cash_dividend_per_share) as annual_cash_dividend_per_share,
        max(ex_dividend_date) as latest_ex_dividend_date
    from cash_dividends
    where report_date is not null
      and toMonth(report_date) = 12
      and toDayOfMonth(report_date) = 31
    group by
        security_code,
        report_date
),

quotes_with_static_dividend as (
    select
        quotes_with_shares.security_code,
        quotes_with_shares.trade_date,
        argMax(
            annual_cash_dividends.annual_cash_dividend_per_share,
            annual_cash_dividends.latest_ex_dividend_date
        ) as latest_annual_cash_dividend_per_share
    from quotes_with_shares
    left join annual_cash_dividends
        on quotes_with_shares.security_code = annual_cash_dividends.security_code
        and annual_cash_dividends.latest_ex_dividend_date <= quotes_with_shares.trade_date
    group by
        quotes_with_shares.security_code,
        quotes_with_shares.trade_date
),

quotes_with_ttm_dividend as (
    select
        quotes_with_shares.security_code,
        quotes_with_shares.trade_date,
        sumIf(
            cash_dividends.cash_dividend_per_share,
            cash_dividends.ex_dividend_date > quotes_with_shares.trade_date - interval 1 year
            and cash_dividends.ex_dividend_date <= quotes_with_shares.trade_date
        ) as ttm_cash_dividend_per_share
    from quotes_with_shares
    left join cash_dividends
        on quotes_with_shares.security_code = cash_dividends.security_code
        and cash_dividends.ex_dividend_date <= quotes_with_shares.trade_date
    group by
        quotes_with_shares.security_code,
        quotes_with_shares.trade_date
),

quotes_with_metrics as (
    select
        quotes_with_shares.security_code,
        quotes_with_shares.trade_date,
        quotes_with_shares.open_price,
        quotes_with_shares.high_price,
        quotes_with_shares.low_price,
        quotes_with_shares.close_price,
        quotes_with_shares.prev_close_price,
        quotes_with_shares.prev_close_price_unadj,
        quotes_with_shares.prev_volume,
        quotes_with_shares.volume,
        quotes_with_shares.amount,
        if(
            quotes_with_shares.volume is null
            or quotes_with_shares.a_float_shares is null
            or quotes_with_shares.a_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_shares.volume / quotes_with_shares.a_float_shares * 100
        ) as turnover_rate,
        if(
            quotes_with_shares.volume is null
            or quotes_with_shares.a_free_float_shares is null
            or quotes_with_shares.a_free_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_shares.volume / quotes_with_shares.a_free_float_shares * 100
        ) as turnover_rate_actual,
        if(
            quotes_with_shares.high_price is null
            or quotes_with_shares.low_price is null
            or quotes_with_shares.prev_close_price is null
            or quotes_with_shares.prev_close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            (quotes_with_shares.high_price - quotes_with_shares.low_price)
                / quotes_with_shares.prev_close_price * 100
        ) as pct_amplitude,
        if(
            quotes_with_shares.close_price is null
            or quotes_with_shares.prev_close_price is null
            or quotes_with_shares.prev_close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            (quotes_with_shares.close_price - quotes_with_shares.prev_close_price)
                / quotes_with_shares.prev_close_price * 100
        ) as pct_change,
        multiIf(
            quotes_with_shares.prev_close_price is null
                or quotes_with_shares.prev_close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_shares.security_board in ('sse_main_board', 'szse_main_board')
                and quotes_with_shares.is_st = true
                and quotes_with_shares.trade_date < toDate('2026-07-06'),
            round(quotes_with_shares.prev_close_price * 1.05, 2),
            quotes_with_shares.security_board in ('sse_main_board', 'szse_main_board')
                and quotes_with_shares.is_st = true
                and quotes_with_shares.trade_date >= toDate('2026-07-06'),
            round(quotes_with_shares.prev_close_price * 1.10, 2),
            quotes_with_shares.security_board in ('sse_main_board', 'szse_main_board'),
            round(quotes_with_shares.prev_close_price * 1.10, 2),
            quotes_with_shares.security_board = 'star_market',
            round(quotes_with_shares.prev_close_price * 1.20, 2),
            quotes_with_shares.security_board = 'chinext',
            round(quotes_with_shares.prev_close_price * 1.20, 2),
            cast(null, 'Nullable(Float64)')
        ) as limit_up_price,
        multiIf(
            quotes_with_shares.prev_close_price is null
                or quotes_with_shares.prev_close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_shares.security_board in ('sse_main_board', 'szse_main_board')
                and quotes_with_shares.is_st = true
                and quotes_with_shares.trade_date < toDate('2026-07-06'),
            round(quotes_with_shares.prev_close_price * 0.95, 2),
            quotes_with_shares.security_board in ('sse_main_board', 'szse_main_board')
                and quotes_with_shares.is_st = true
                and quotes_with_shares.trade_date >= toDate('2026-07-06'),
            round(quotes_with_shares.prev_close_price * 0.90, 2),
            quotes_with_shares.security_board in ('sse_main_board', 'szse_main_board'),
            round(quotes_with_shares.prev_close_price * 0.90, 2),
            quotes_with_shares.security_board = 'star_market',
            round(quotes_with_shares.prev_close_price * 0.80, 2),
            quotes_with_shares.security_board = 'chinext',
            round(quotes_with_shares.prev_close_price * 0.80, 2),
            cast(null, 'Nullable(Float64)')
        ) as limit_down_price,
        if(
            quotes_with_shares.close_price is null
            or quotes_with_shares.close_price < 0
            or quotes_with_shares.a_shares is null
            or quotes_with_shares.a_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_shares.close_price * quotes_with_shares.a_shares
        ) as a_market_cap,
        if(
            quotes_with_shares.close_price is null
            or quotes_with_shares.close_price < 0
            or quotes_with_shares.a_float_shares is null
            or quotes_with_shares.a_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_shares.close_price * quotes_with_shares.a_float_shares
        ) as a_float_market_cap,
        if(
            quotes_with_shares.close_price is null
            or quotes_with_shares.close_price < 0
            or quotes_with_shares.a_free_float_shares is null
            or quotes_with_shares.a_free_float_shares <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_shares.close_price * quotes_with_shares.a_free_float_shares
        ) as a_free_float_market_cap,
        quotes_with_shares.a_shares,
        quotes_with_shares.a_float_shares,
        quotes_with_shares.a_free_float_shares,
        if(
            quotes_with_static_dividend.latest_annual_cash_dividend_per_share is null
            or quotes_with_shares.close_price is null
            or quotes_with_shares.close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_static_dividend.latest_annual_cash_dividend_per_share
                / quotes_with_shares.close_price * 100
        ) as dy_static,
        if(
            quotes_with_ttm_dividend.ttm_cash_dividend_per_share is null
            or quotes_with_shares.close_price is null
            or quotes_with_shares.close_price <= 0,
            cast(null, 'Nullable(Float64)'),
            quotes_with_ttm_dividend.ttm_cash_dividend_per_share
                / quotes_with_shares.close_price * 100
        ) as dy_ttm,
        quotes_with_shares.is_suspend,
        quotes_with_shares.is_st
    from quotes_with_shares
    left join quotes_with_static_dividend
        on quotes_with_shares.security_code = quotes_with_static_dividend.security_code
        and quotes_with_shares.trade_date = quotes_with_static_dividend.trade_date
    left join quotes_with_ttm_dividend
        on quotes_with_shares.security_code = quotes_with_ttm_dividend.security_code
        and quotes_with_shares.trade_date = quotes_with_ttm_dividend.trade_date
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
    prev_volume,
    volume,
    amount,
    turnover_rate,
    turnover_rate_actual,
    pct_amplitude,
    pct_change,
    limit_up_price,
    limit_down_price,
    a_market_cap,
    a_float_market_cap,
    a_free_float_market_cap,
    a_shares,
    a_float_shares,
    a_free_float_shares,
    dy_static,
    dy_ttm,
    is_suspend,
    is_st
from quotes_with_metrics
```

实现注意：

- `int_trade_calendar` 应作为前交易日定义来源，不在本模型内重复窗口计算交易日序列。
- `int_stock_basic_snapshot` 应作为股票 universe 来源，不在本模型内重复读取证券基础信息 staging。
- 从 staging 取行情时应过滤 `trade_date > toDate('1995-01-01')`，只保留 A 股恢复 T+1 后的交易制度时期。
- 自连接应使用已经过滤后的 `stock_quotes`，避免指数、ETF、可转债行情进入前值查找。
- `adjustflag` 过滤和 `trade_status` 到 `is_suspend` 的派生已在上游 staging 完成。
- `prev_trade_date` 仅作为中间计算字段，不建议输出到第一版模型；如后续调试或下游需要，可单独设计字段。
- `int_stock_shares_history` 理论上同一证券同一日期只应命中一个区间；实现前应有区间不重叠测试。若 ClickHouse 使用 `ANY JOIN`，必须确保不是用它掩盖重复区间。
- 不使用 BaoStock staging 的 source-local `turn` 或 `pctChg`；本模型统一按项目口径重算换手率，并基于 BaoStock `prev_close_price` 重算 `pct_amplitude` 和 `pct_change`。
- 不直接从分红 staging 计算 `dy_static` 和 `dy_ttm`；股息率只依赖已结构化的 `int_stock_exrights_event`。

## 7. 测试建议

- 模型级组合唯一：`security_code`, `trade_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- `is_suspend`: `not_null`。
- `prev_close_price_unadj` 和 `prev_volume`: 不加 `not_null`，因为首个交易日、证券前一交易日无行情、source 缺口都应允许为 `NULL`。
- `turnover_rate`, `turnover_rate_actual`, `pct_amplitude`, `dy_static`, `dy_ttm`: 可空；非空时应大于等于 `0`。
- `pct_change`: 可空；允许为负值。
- `limit_up_price`, `limit_down_price`: 可空；非空时应大于 `0`，且 `limit_up_price >= limit_down_price`。
- `a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap`: 可空；非空时应大于等于 `0`。
- `a_shares`, `a_float_shares`, `a_free_float_shares`: 可空；非空时应大于等于 `0`。
- 增加定向数据测试：
  - 输出证券必须存在于 `int_stock_basic_snapshot`。
  - 输出记录继承上游 staging 的未复权口径。
  - `trade_date` 必须大于 `1995-01-01`。
  - `is_suspend` 由上游 staging 保证与源 `trade_status` 映射一致：`trade_status = 0` 时为 `true`，`trade_status = 1` 时为 `false`。
  - 除 `int_trade_calendar.prev_trade_date is null` 或前一交易日缺行情外，`prev_close_price_unadj` 应等于同一证券前一交易日的 `close_price`。
  - 除 `int_trade_calendar.prev_trade_date is null` 或前一交易日缺行情外，`prev_volume` 应等于同一证券前一交易日的 `volume`，且 `0` 成交量应保留。
  - `prev_close_price_unadj` 与 `prev_close_price` 不要求一致；两者口径不同，不能用相等性测试约束。
  - 当 `a_float_shares > 0` 且 `volume` 非空时，`turnover_rate = volume / a_float_shares * 100`。
  - 当 `a_free_float_shares > 0` 且 `volume` 非空时，`turnover_rate_actual = volume / a_free_float_shares * 100`。
  - 当 `prev_close_price > 0` 且 `high_price`, `low_price` 非空时，`pct_amplitude = (high_price - low_price) / prev_close_price * 100`。
  - 当 `prev_close_price > 0` 且 `close_price` 非空时，`pct_change = (close_price - prev_close_price) / prev_close_price * 100`。
  - 当市值和股本字段均非空时，应满足 `a_free_float_market_cap <= a_float_market_cap <= a_market_cap`，允许因股本源异常单独做质量告警而非硬失败。
  - `dy_static` 使用的年度现金分红除权除息日不得大于当前 `trade_date`。
  - `dy_ttm` 只聚合当前 `trade_date` 前 12 个月内已除权除息的现金分红事件。

## 8. 延后事项

- 基于上市/退市日期裁剪行情范围。
- 输出 `prev_trade_date` 或其他交易日派生字段。
- 输出 `adjust_flag` 或 `trade_status`。
- 北交所 `30%` 涨跌停规则、创业板历史制度分段和新股上市首日等无涨跌幅限制日期。
- 更精确的交易所涨跌停价舍入规则和特殊证券状态处理。
- 复权行情、复权因子和多复权口径支持。
- 停牌日价格、成交量和涨跌幅的业务裁决。
- 与其他行情源的优先级合并和对账。
- 股票 universe 的跨源主数据裁决、北交所扩展和历史证券类型变化处理。
