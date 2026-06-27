# int_stock_quotes_daily_adj 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_quotes_daily_unadj')`
- Intermediate model：`ref('int_stock_adjustment_factor')`
- Intermediate 设计：`docs/architecture/dbt_layer/fleur_intermediate/int_stock_quotes_daily_unadj.md`
- Intermediate 设计：`docs/architecture/dbt_layer/fleur_intermediate/int_stock_adjustment_factor.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_quotes_daily_adj.sql`

## 1. 模型定位

A 股股票日频复权价格 intermediate 模型。模型基于未复权股票日行情和复权因子，同时输出前复权价格和后复权价格。

本模型只负责价格复权派生，不重复存储成交量、成交金额、换手率、交易状态、ST 标记、涨跌幅等非价格字段。这些字段由 `int_stock_quotes_daily_unadj` 维护，下游如需使用，应按 `security_code`, `trade_date` 回查或 join 未复权行情表。

## 2. 数据粒度与依赖

- 未复权行情依赖：`int_stock_quotes_daily_unadj`。
- 复权因子依赖：`int_stock_adjustment_factor`。
- 粒度：一行一个 `security_code` + `trade_date` 的股票日频复权价格记录。
- 候选键：`security_code`, `trade_date`。
- Join 方式：按 `security_code`, `trade_date` `INNER ANY JOIN` 复权因子。缺少因子的行情行不应静默输出为复权价格。

## 3. 价格口径

后复权价格公式：

```text
<price>_backward_adj = <price> * backward_adjustment_factor
```

前复权价格公式：

```text
<price>_forward_adj = <price> * forward_adjustment_factor
```

适用字段：

- `open_price_backward_adj`, `open_price_forward_adj`
- `high_price_backward_adj`, `high_price_forward_adj`
- `low_price_backward_adj`, `low_price_forward_adj`
- `close_price_backward_adj`, `close_price_forward_adj`
- `prev_close_price_backward_adj`, `prev_close_price_forward_adj`

语义说明：

- `backward_adjustment_factor` 和 `forward_adjustment_factor` 来自 `int_stock_adjustment_factor`。
- 字段名必须显式包含 `backward_adj` 或 `forward_adj`，避免泛化的 `_adj` 字段在前/后复权并存时语义不清。
- `prev_close_price_*_adj` 使用 BaoStock 原始 `prev_close_price` 乘以对应复权因子，表达“当日记录携带的昨日收盘价口径在复权尺度下的值”。
- 本模型不输出未复权 OHLC；如需要对比未复权价格，应回查 `int_stock_quotes_daily_unadj`。

## 4. NULL 与零值处理

- 当未复权价格为 `NULL` 时，对应复权价格保留 `NULL`。
- 当任一复权因子为 `NULL` 时，该行不应进入本模型；应由 join 和上游因子测试暴露。
- 价格为 `0` 时按 source-local 事实保留，乘以复权因子后仍可能为 `0`，不在本模型中转 NULL。
- 不在本模型中重算涨跌幅；前/后复权涨跌幅如有需要，应作为后续模型单独设计。

## 5. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | `int_stock_quotes_daily_unadj.security_code` | `String` | 股票标准连接代码。 |
| `trade_date` | `int_stock_quotes_daily_unadj.trade_date` | `Date` | 行情交易日期。 |
| `open_price_backward_adj` | `open_price * backward_adjustment_factor` | `Nullable(Float64)` | 后复权开盘价。 |
| `high_price_backward_adj` | `high_price * backward_adjustment_factor` | `Nullable(Float64)` | 后复权最高价。 |
| `low_price_backward_adj` | `low_price * backward_adjustment_factor` | `Nullable(Float64)` | 后复权最低价。 |
| `close_price_backward_adj` | `close_price * backward_adjustment_factor` | `Nullable(Float64)` | 后复权收盘价。 |
| `prev_close_price_backward_adj` | `prev_close_price * backward_adjustment_factor` | `Nullable(Float64)` | 当日记录携带的昨日收盘价口径在后复权尺度下的值。 |
| `open_price_forward_adj` | `open_price * forward_adjustment_factor` | `Nullable(Float64)` | 前复权开盘价。 |
| `high_price_forward_adj` | `high_price * forward_adjustment_factor` | `Nullable(Float64)` | 前复权最高价。 |
| `low_price_forward_adj` | `low_price * forward_adjustment_factor` | `Nullable(Float64)` | 前复权最低价。 |
| `close_price_forward_adj` | `close_price * forward_adjustment_factor` | `Nullable(Float64)` | 前复权收盘价。 |
| `prev_close_price_forward_adj` | `prev_close_price * forward_adjustment_factor` | `Nullable(Float64)` | 当日记录携带的昨日收盘价口径在前复权尺度下的值。 |
| `backward_adjustment_factor` | `int_stock_adjustment_factor.backward_adjustment_factor` | `Float64` | 后复权因子，保留用于解释后复权价格。 |
| `backward_adjustment_ratio` | `int_stock_adjustment_factor.backward_adjustment_ratio` | `Float64` | 后复权单步比例，保留用于识别除权除息影响日。 |
| `forward_adjustment_factor` | `int_stock_adjustment_factor.forward_adjustment_factor` | `Float64` | 前复权因子，保留用于解释前复权价格。 |
| `forward_adjustment_ratio` | `int_stock_adjustment_factor.forward_adjustment_ratio` | `Float64` | 前复权单步比例，保留用于识别除权除息影响日。 |

字段顺序建议：

1. 主键字段：`security_code`, `trade_date`
2. 后复权价格字段：`open_price_backward_adj`, `high_price_backward_adj`, `low_price_backward_adj`, `close_price_backward_adj`, `prev_close_price_backward_adj`
3. 前复权价格字段：`open_price_forward_adj`, `high_price_forward_adj`, `low_price_forward_adj`, `close_price_forward_adj`, `prev_close_price_forward_adj`
4. 因子解释字段：`backward_adjustment_factor`, `backward_adjustment_ratio`, `forward_adjustment_factor`, `forward_adjustment_ratio`

## 6. SQL 逻辑建议

```sql
with stock_quotes_unadj as (
    select
        security_code,
        trade_date,
        open_price,
        high_price,
        low_price,
        close_price,
        prev_close_price
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

adjustment_factors as (
    select
        security_code,
        trade_date,
        backward_adjustment_factor,
        backward_adjustment_ratio,
        forward_adjustment_factor,
        forward_adjustment_ratio
    from {{ ref('int_stock_adjustment_factor') }}
),

stock_quotes_adj as (
    select
        stock_quotes_unadj.security_code,
        stock_quotes_unadj.trade_date,
        stock_quotes_unadj.open_price * adjustment_factors.backward_adjustment_factor as open_price_backward_adj,
        stock_quotes_unadj.high_price * adjustment_factors.backward_adjustment_factor as high_price_backward_adj,
        stock_quotes_unadj.low_price * adjustment_factors.backward_adjustment_factor as low_price_backward_adj,
        stock_quotes_unadj.close_price * adjustment_factors.backward_adjustment_factor as close_price_backward_adj,
        stock_quotes_unadj.prev_close_price * adjustment_factors.backward_adjustment_factor as prev_close_price_backward_adj,
        stock_quotes_unadj.open_price * adjustment_factors.forward_adjustment_factor as open_price_forward_adj,
        stock_quotes_unadj.high_price * adjustment_factors.forward_adjustment_factor as high_price_forward_adj,
        stock_quotes_unadj.low_price * adjustment_factors.forward_adjustment_factor as low_price_forward_adj,
        stock_quotes_unadj.close_price * adjustment_factors.forward_adjustment_factor as close_price_forward_adj,
        stock_quotes_unadj.prev_close_price * adjustment_factors.forward_adjustment_factor as prev_close_price_forward_adj,
        adjustment_factors.backward_adjustment_factor,
        adjustment_factors.backward_adjustment_ratio,
        adjustment_factors.forward_adjustment_factor,
        adjustment_factors.forward_adjustment_ratio
    from stock_quotes_unadj
    inner any join adjustment_factors
        on stock_quotes_unadj.security_code = adjustment_factors.security_code
        and stock_quotes_unadj.trade_date = adjustment_factors.trade_date
)

select
    security_code,
    trade_date,
    open_price_backward_adj,
    high_price_backward_adj,
    low_price_backward_adj,
    close_price_backward_adj,
    prev_close_price_backward_adj,
    open_price_forward_adj,
    high_price_forward_adj,
    low_price_forward_adj,
    close_price_forward_adj,
    prev_close_price_forward_adj,
    backward_adjustment_factor,
    backward_adjustment_ratio,
    forward_adjustment_factor,
    forward_adjustment_ratio
from stock_quotes_adj
```

实现注意：

- 只投影复权价格计算需要的未复权价格字段，避免重复存储未复权表中的非价格事实。
- 不输出同名 `open_price`、`close_price` 等字段，避免下游误把复权价格当作未复权价格。
- 不在本模型内重新计算 adjustment ratio 或 adjustment factor。
- `int_stock_adjustment_factor` 以 `security_code`, `trade_date` 唯一为前提；ClickHouse 实现中可使用 `INNER ANY JOIN`，减少不必要的多匹配 join 成本。

## 7. 测试建议

- 模型级组合唯一：`security_code`, `trade_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- `backward_adjustment_factor`: `not_null`，且应大于 0。
- `backward_adjustment_ratio`: `not_null`，且应大于 0。
- `forward_adjustment_factor`: `not_null`，且应大于 0。
- `forward_adjustment_ratio`: `not_null`，且应大于 0。
- 增加定向数据测试：
  - 本模型行数应等于 `int_stock_quotes_daily_unadj` 和 `int_stock_adjustment_factor` 按主键 inner join 后的行数。
  - 当未复权 `close_price` 非 NULL 时，`close_price_backward_adj = close_price * backward_adjustment_factor`，`close_price_forward_adj = close_price * forward_adjustment_factor`。
  - 当未复权 `open_price`、`high_price`、`low_price` 非 NULL 时，对应前/后复权字段等于未复权价格乘以对应复权因子。
  - 不要求复权价格非 NULL，因为源价格字段可能为 NULL。

## 8. 延后事项

- 输出未复权和复权价格并列的宽表模型。
- 成交量、成交金额按复权因子的反向调整口径。
- 基于复权价格重算涨跌幅。
- 与 BaoStock 前复权/后复权 K 线或其他供应商复权价格做对账。
