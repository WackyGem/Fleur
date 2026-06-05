# int_stock_adjustment_factor 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_quotes_daily_unadj')`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_quotes_daily_unadj.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_adjustment_factor.sql`

## 1. 模型定位

A 股股票日频复权因子 intermediate 模型。模型基于 `int_stock_quotes_daily_unadj` 中的未复权收盘价、前一交易日未复权收盘价和 BaoStock 原始前收盘价，计算每只股票的后复权因子和前复权因子。

本模型只负责复权因子计算，不输出复权价格宽表，不重新过滤股票 universe，不重算 `prev_close_price_unadj`，也不处理跨源行情优先级或复权因子供应商对账。

## 2. 数据粒度与依赖

- 依赖：`int_stock_quotes_daily_unadj`。
- 粒度：一行一个 `security_code` + `trade_date` 的股票日频复权因子记录。
- 候选键：`security_code`, `trade_date`。
- 后复权排序口径：每只股票内按 `trade_date` 升序向后累乘。
- 前复权排序口径：每只股票内按 `trade_date` 降序向前累乘；某日比例作用于该日之前的历史价格。

输入字段依赖：

- `close_price`：当日未复权收盘价。
- `prev_close_price`：BaoStock 原始 `preclose` 口径的当日昨日收盘价。
- `prev_close_price_unadj`：同一股票在交易日历前一交易日的未复权收盘价。

## 3. 核心公式

后复权日比例：

```text
backward_adjustment_ratio = prev_close_price_unadj / prev_close_price
```

后复权因子：

```text
backward_adjustment_factor =
    product(backward_adjustment_ratio) over (
        partition by security_code
        order by trade_date asc
        rows between unbounded preceding and current row
    )
```

前复权日比例：

```text
forward_adjustment_ratio = prev_close_price / prev_close_price_unadj
```

前复权因子：

```text
forward_adjustment_factor =
    product(forward_adjustment_ratio) over (
        partition by security_code
        order by trade_date asc
        rows between 1 following and unbounded following
    )
```

语义说明：

- `prev_close_price_unadj` 是“前一交易日实际未复权收盘价”。
- `prev_close_price` 是“当日行情记录携带的昨日收盘价口径”，通常已反映除权除息后的参考前收盘。
- 当发生除权除息且两个字段不一致时，`prev_close_price_unadj / prev_close_price` 形成后复权日比例，`prev_close_price / prev_close_price_unadj` 形成前复权日比例。
- 后复权因子按时间向后累乘，越靠后的日期包含越多历史除权除息影响。
- 前复权因子按时间向前累乘，越靠前的日期包含越多后续除权除息影响；最新一条行情记录的前复权因子应为 `1.0`。
- 后复权价格可在下游用 `未复权价格 * backward_adjustment_factor` 派生，前复权价格可在下游用 `未复权价格 * forward_adjustment_factor` 派生；本模型第一版不直接输出复权 OHLC。

## 4. NULL、零值与异常处理

后复权日比例计算规则：

- 当 `prev_close_price_unadj` 为 `NULL` 时，`backward_adjustment_ratio` 取 `1.0`。这覆盖股票首个交易日、前一交易日无行情和 source 缺口场景，避免中断累乘。
- 当 `prev_close_price` 为 `NULL` 或 `0` 时，`backward_adjustment_ratio` 取 `1.0`，并依靠测试或异常监控暴露问题；不在模型内除以 0。
- 当任一价格字段小于等于 `0` 时，`backward_adjustment_ratio` 取 `1.0`，避免 `log()` 累乘异常。
- 当两个字段都有效时，`backward_adjustment_ratio = prev_close_price_unadj / prev_close_price`。

前复权日比例计算规则：

- 当 `prev_close_price_unadj` 为 `NULL` 时，`forward_adjustment_ratio` 取 `1.0`。
- 当 `prev_close_price` 为 `NULL` 或 `0` 时，`forward_adjustment_ratio` 取 `1.0`。
- 当任一价格字段小于等于 `0` 时，`forward_adjustment_ratio` 取 `1.0`，避免 `log()` 累乘异常。
- 当两个字段都有效时，`forward_adjustment_ratio = prev_close_price / prev_close_price_unadj`。

复权因子计算规则：

- 每只股票第一条记录的 `backward_adjustment_factor` 至少为 `1.0`。
- 每只股票最新一条记录的 `forward_adjustment_factor` 应为 `1.0`。
- 如某日无法计算有效日比例，因子沿用前值，等价于当日比例为 `1.0`。
- 不把日比例和因子四舍五入；精度控制留给下游展示层。

## 5. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | `int_stock_quotes_daily_unadj.security_code` | `String` | 股票标准连接代码。 |
| `trade_date` | `int_stock_quotes_daily_unadj.trade_date` | `Date` | 行情交易日期。 |
| `prev_close_price_unadj` | `int_stock_quotes_daily_unadj.prev_close_price_unadj` | `Nullable(Float64)` | 前一交易日未复权收盘价，保留用于因子可解释性。 |
| `prev_close_price` | `int_stock_quotes_daily_unadj.prev_close_price` | `Nullable(Float64)` | 当日记录携带的昨日收盘价口径，保留用于因子可解释性。 |
| `backward_adjustment_ratio` | `prev_close_price_unadj / prev_close_price` | `Float64` | 后复权单步比例；无法有效计算时取 `1.0`。 |
| `backward_adjustment_factor` | `backward_adjustment_ratio` 向后累乘 | `Float64` | 后复权因子。下游可用未复权价格乘该字段得到后复权价格。 |
| `forward_adjustment_ratio` | `prev_close_price / prev_close_price_unadj` | `Float64` | 前复权单步比例；无法有效计算时取 `1.0`。 |
| `forward_adjustment_factor` | `forward_adjustment_ratio` 向前累乘 | `Float64` | 前复权因子。下游可用未复权价格乘该字段得到前复权价格。 |

字段顺序建议：

1. 主键字段：`security_code`, `trade_date`
2. 解释字段：`prev_close_price_unadj`, `prev_close_price`
3. 因子字段：`backward_adjustment_ratio`, `backward_adjustment_factor`, `forward_adjustment_ratio`, `forward_adjustment_factor`

## 6. SQL 逻辑建议

ClickHouse 没有通用窗口乘积函数时，可使用 `exp(sum(log(ratio)))` 表达正数比例的累乘。由于本设计把无效比例归一为 `1.0`，并且价格应为非负，两个单步比例字段都应保持大于 0。

```sql
with stock_quotes as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price
    from {{ ref('int_stock_quotes_daily_unadj') }}
),

price_pairs as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price,
        prev_close_price_unadj is not null
            and prev_close_price is not null
            and prev_close_price_unadj > 0
            and prev_close_price > 0 as has_valid_adjustment_pair
    from stock_quotes
),

adjustment_ratios as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price,
        if(
            has_valid_adjustment_pair,
            prev_close_price_unadj / prev_close_price,
            1.0
        ) as backward_adjustment_ratio,
        if(
            has_valid_adjustment_pair,
            prev_close_price / prev_close_price_unadj,
            1.0
        ) as forward_adjustment_ratio
    from price_pairs
),

adjustment_factors as (
    select
        security_code,
        trade_date,
        prev_close_price_unadj,
        prev_close_price,
        backward_adjustment_ratio,
        forward_adjustment_ratio,
        exp(
            sum(log(backward_adjustment_ratio)) over (
                partition by security_code
                order by trade_date
                rows between unbounded preceding and current row
            )
        ) as backward_adjustment_factor,
        exp(
            coalesce(
                sum(log(forward_adjustment_ratio)) over (
                    partition by security_code
                    order by trade_date
                    rows between 1 following and unbounded following
                ),
                0.0
            )
        ) as forward_adjustment_factor
    from adjustment_ratios
)

select
    security_code,
    trade_date,
    prev_close_price_unadj,
    prev_close_price,
    backward_adjustment_ratio,
    backward_adjustment_factor,
    forward_adjustment_ratio,
    forward_adjustment_factor
from adjustment_factors
```

实现注意：

- 不直接依赖 `stg_baostock__query_history_k_data_plus_daily`，避免重复股票过滤和前交易日取值逻辑。
- 不使用 `close_price / prev_close_price` 计算复权比例；`close_price` 是当日收盘价，不是前一交易日收盘价。
- 如果后续 `int_stock_quotes_daily_unadj` 输出 `prev_trade_date`，本模型仍不需要依赖该字段。
- `has_valid_adjustment_pair` 统一封装价格有效性判断，避免前复权和后复权比例各自复制 NULL/非正数防护逻辑。

## 7. 测试建议

- 模型级组合唯一：`security_code`, `trade_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- `backward_adjustment_ratio`: `not_null`，且应大于 0。
- `backward_adjustment_factor`: `not_null`，且应大于 0。
- `forward_adjustment_ratio`: `not_null`，且应大于 0。
- `forward_adjustment_factor`: `not_null`，且应大于 0。
- 增加定向数据测试：
  - 当 `prev_close_price_unadj` 和 `prev_close_price` 均有效且都大于 0 时，`backward_adjustment_ratio` 等于 `prev_close_price_unadj / prev_close_price`。
  - 每只股票第一条记录的 `backward_adjustment_factor = backward_adjustment_ratio`；在首日无前值的常见情况下应为 `1.0`。
  - 对同一股票相邻交易记录，当前 `backward_adjustment_factor` 应等于上一条 `backward_adjustment_factor * backward_adjustment_ratio`。
  - 当 `prev_close_price_unadj` 和 `prev_close_price` 均有效且都大于 0 时，`forward_adjustment_ratio` 等于 `prev_close_price / prev_close_price_unadj`。
  - 每只股票最新一条记录的 `forward_adjustment_factor = 1.0`。
  - 对同一股票相邻交易记录，当前较早记录的 `forward_adjustment_factor` 应等于下一条记录的 `forward_adjustment_factor * forward_adjustment_ratio`，其中 `forward_adjustment_ratio` 取下一条记录的值。

## 8. 延后事项

- 输出后复权 OHLC、成交量或成交金额调整字段。
- 输出前复权 OHLC、成交量或成交金额调整字段。
- 引入 BaoStock `query_adjust_factor` 或其他复权因子源做对账。
- 对现金分红、送转、配股等事件拆分复权贡献。
- 针对异常比例设置数据质量告警阈值，例如 `backward_adjustment_ratio` 或 `forward_adjustment_ratio` 过大或过小。
