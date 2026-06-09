# mart_stock_quotes_daily 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_quotes_daily_unadj')`
- Intermediate model：`ref('int_stock_shares_history')`
- Intermediate model：`ref('int_stock_financial_valuation')`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_quotes_daily_unadj.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_shares_history.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_financial_valuation.md`
- 目标位置：`pipeline/elt/models/marts/mart_stock_quotes_daily.sql`

## 1. 模型定位

A 股股票日频行情 mart 宽表。模型以 `int_stock_quotes_daily_unadj` 为主事实表，保留其全部原有字段，并补充面向分析消费的日频交易指标、市值股本指标、估值指标和股息率指标。

本模型服务于 BI、研究查询、行情看板和下游应用读取，不承载复杂口径计算的唯一事实源。换手率、市值、估值和股息率的核心计算应来自 intermediate 层；mart 层只负责按 `security_code`, `trade_date` 对齐、命名收敛和字段排序。

本模型不做复权价格输出，不重新计算复权因子，不直接读取 staging 或 raw 表，不在 mart 内实现财报窗口、股本区间、分红事件解析等复杂逻辑。

## 2. 数据粒度与依赖

- 主行情依赖：`int_stock_quotes_daily_unadj`。
- 财报期估值依赖：`int_stock_financial_valuation`。
- 粒度：一行一个 `security_code` + `trade_date` 的股票日频行情记录。
- 候选键：`security_code`, `trade_date`。
- Join 策略：以行情表为左表。缺少财报期估值输入时，保留行情行，对应扩展字段输出 `NULL`。

第一版依赖边界：

- 已有 `int_stock_quotes_daily_unadj` 可直接提供基础行情、换手率、涨跌幅、振幅、涨跌停价、A 股市值、A 股股本和股息率字段。
- 已有 `int_stock_financial_valuation` 可按 `report_date <= trade_date` as-of 取最近报告期，提供 PE/PB、每股净资产、ROE、ROA、ROAA 和 ROAE。

## 3. 指标口径

### 3.1 行情字段

`int_stock_quotes_daily_unadj` 的所有原有字段原样透传，字段名不改：

- `security_code`
- `trade_date`
- `open_price`
- `high_price`
- `low_price`
- `close_price`
- `prev_close_price`
- `prev_close_price_unadj`
- `volume`
- `amount`
- `is_suspend`
- `is_st`

价格字段均为未复权口径。mart 字段名不再追加 `_unadj`，因为来源模型和文档已明确未复权；如后续同表并列复权价格，必须使用 `*_forward_adj` 或 `*_backward_adj` 显式命名。

### 3.2 日频交易、市值、股本和股息率字段

以下字段均来自 `int_stock_quotes_daily_unadj`，mart 层原样透传，不重复计算：

- `turnover_rate`
- `turnover_rate_actual`
- `pct_amplitude`
- `pct_change`
- `limit_up_price`
- `limit_down_price`
- `a_market_cap`
- `a_float_market_cap`
- `a_free_float_market_cap`
- `a_shares`
- `a_float_shares`
- `a_free_float_shares`
- `dy_static`
- `dy_ttm`

规则：

- `turnover_rate`、`turnover_rate_actual`、`pct_amplitude`、`pct_change`、`dy_static` 和 `dy_ttm` 使用百分数口径，`1.23` 表示 `1.23%`。
- `roe`、`roa`、`roaa` 和 `roae` 使用比率口径，不乘以 `100`；这些字段来自 `int_stock_financial_valuation`。
- 涨跌停价、市值、股本和股息率口径以 `int_stock_quotes_daily_unadj` 设计为准。

### 3.3 财报期指标

本 mart 需要输出财报期 as-of 指标：

- `pe_static`
- `pe_ttm`
- `pe_forecast`
- `pb_mrq`
- `book_value_per_share`
- `roe`
- `roa`
- `roaa`
- `roae`

财报期指标来源：

- `pe_static`、`pe_ttm`、`pe_forecast`、`pb_mrq`、`book_value_per_share`、`roe`、`roa`、`roaa`、`roae` 均来自 `int_stock_financial_valuation`。
- Join 规则：同一证券 `financial_valuation.report_date <= quotes.trade_date` 的最近一条报告期记录。
- 这些字段是财报期指标按交易日 as-of 展开，不使用当日收盘价在 mart 内重算。
- 第一版不按公告日判断财报市场可知性；如果后续要支持可投资口径，应在 `int_stock_financial_valuation` 或专门公告日模型中处理，不在 mart 内混合。

规则：

- 财报期指标的 `report_date` 不能晚于 `trade_date`。
- PE/PB 和每股净资产字段沿用 `int_stock_financial_valuation` 的 NULL 和非负估值规则。
- `roe`、`roa`、`roaa` 和 `roae` 沿用 `int_stock_financial_valuation` 的比率口径，不乘以 `100`；归母净利润为负时允许输出负值。

## 4. 字段设计

| Mart 字段 | 来源/派生 | 类型建议 | 设计说明 |
|-----------|-----------|----------|----------|
| `security_code` | `int_stock_quotes_daily_unadj.security_code` | `String` | 股票标准连接代码。 |
| `trade_date` | `int_stock_quotes_daily_unadj.trade_date` | `Date` | 行情交易日期。 |
| `open_price` | `int_stock_quotes_daily_unadj.open_price` | `Nullable(Float64)` | 交易日未复权开盘价。 |
| `high_price` | `int_stock_quotes_daily_unadj.high_price` | `Nullable(Float64)` | 交易日未复权最高价。 |
| `low_price` | `int_stock_quotes_daily_unadj.low_price` | `Nullable(Float64)` | 交易日未复权最低价。 |
| `close_price` | `int_stock_quotes_daily_unadj.close_price` | `Nullable(Float64)` | 交易日未复权收盘价。 |
| `prev_close_price` | `int_stock_quotes_daily_unadj.prev_close_price` | `Nullable(Float64)` | BaoStock 原始 `preclose` 口径前收盘价。 |
| `prev_close_price_unadj` | `int_stock_quotes_daily_unadj.prev_close_price_unadj` | `Nullable(Float64)` | 前一交易日实际未复权收盘价。 |
| `volume` | `int_stock_quotes_daily_unadj.volume` | `Nullable(Int64)` | 交易日成交量。 |
| `amount` | `int_stock_quotes_daily_unadj.amount` | `Nullable(Float64)` | 交易日成交金额，单位沿用上游。 |
| `turnover_rate` | `int_stock_quotes_daily_unadj.turnover_rate` | `Nullable(Float64)` | 换手率，百分数口径。 |
| `turnover_rate_actual` | `int_stock_quotes_daily_unadj.turnover_rate_actual` | `Nullable(Float64)` | 实际换手率，使用 A 股自由流通股本作分母。 |
| `pct_amplitude` | `int_stock_quotes_daily_unadj.pct_amplitude` | `Nullable(Float64)` | 振幅，百分数口径。 |
| `pct_change` | `int_stock_quotes_daily_unadj.pct_change` | `Nullable(Float64)` | 涨跌幅，百分数口径。 |
| `limit_up_price` | `int_stock_quotes_daily_unadj.limit_up_price` | `Nullable(Float64)` | 涨停价。 |
| `limit_down_price` | `int_stock_quotes_daily_unadj.limit_down_price` | `Nullable(Float64)` | 跌停价。 |
| `a_market_cap` | `int_stock_quotes_daily_unadj.a_market_cap` | `Nullable(Float64)` | A 股市值，单位元。 |
| `a_float_market_cap` | `int_stock_quotes_daily_unadj.a_float_market_cap` | `Nullable(Float64)` | A 股流通市值，单位元。 |
| `a_free_float_market_cap` | `int_stock_quotes_daily_unadj.a_free_float_market_cap` | `Nullable(Float64)` | A 股自由流通市值，单位元。 |
| `a_shares` | `int_stock_quotes_daily_unadj.a_shares` | `Nullable(Float64)` | A 股股本，单位股。 |
| `a_float_shares` | `int_stock_quotes_daily_unadj.a_float_shares` | `Nullable(Float64)` | A 股流通股，单位股。 |
| `a_free_float_shares` | `int_stock_quotes_daily_unadj.a_free_float_shares` | `Nullable(Float64)` | A 股自由流通股，单位股。 |
| `pe_static` | `int_stock_financial_valuation.pe_static` | `Nullable(Float64)` | 市盈率（静），按最近报告期 as-of 取值。 |
| `pe_ttm` | `int_stock_financial_valuation.pe_ttm` | `Nullable(Float64)` | 市盈率（TTM），按最近报告期 as-of 取值。 |
| `pe_forecast` | `int_stock_financial_valuation.pe_forecast` | `Nullable(Float64)` | 市盈率（动），按最近报告期 as-of 取值。 |
| `pb_mrq` | `int_stock_financial_valuation.pb_mrq` | `Nullable(Float64)` | 市净率（MRQ），按最近报告期 as-of 取值。 |
| `book_value_per_share` | `int_stock_financial_valuation.book_value_per_share` | `Nullable(Float64)` | 每股净资产，按最近报告期 as-of 取值。 |
| `roe` | `int_stock_financial_valuation.roe` | `Nullable(Float64)` | ROE，按最近报告期 as-of 取值。 |
| `roa` | `int_stock_financial_valuation.roa` | `Nullable(Float64)` | ROA，按最近报告期 as-of 取值。 |
| `roaa` | `int_stock_financial_valuation.roaa` | `Nullable(Float64)` | ROAA，按最近报告期 as-of 取值。 |
| `roae` | `int_stock_financial_valuation.roae` | `Nullable(Float64)` | ROAE，按最近报告期 as-of 取值。 |
| `dy_static` | `int_stock_quotes_daily_unadj.dy_static` | `Nullable(Float64)` | 股息率（静），百分数口径。 |
| `dy_ttm` | `int_stock_quotes_daily_unadj.dy_ttm` | `Nullable(Float64)` | 股息率（TTM），百分数口径。 |
| `is_suspend` | `int_stock_quotes_daily_unadj.is_suspend` | `Bool` | 是否停牌。 |
| `is_st` | `int_stock_quotes_daily_unadj.is_st` | `Nullable(Bool)` | 是否 ST 或风险警示。 |

字段顺序建议：

1. 主键字段：`security_code`, `trade_date`
2. OHLC 与前收盘：`open_price`, `high_price`, `low_price`, `close_price`, `prev_close_price`, `prev_close_price_unadj`
3. 成交与交易指标：`volume`, `amount`, `turnover_rate`, `turnover_rate_actual`, `pct_amplitude`, `pct_change`
4. 涨跌停价：`limit_up_price`, `limit_down_price`
5. 市值与股本：`a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap`, `a_shares`, `a_float_shares`, `a_free_float_shares`
6. 财报期指标与股息率：`pe_static`, `pe_ttm`, `pe_forecast`, `pb_mrq`, `book_value_per_share`, `roe`, `roa`, `roaa`, `roae`, `dy_static`, `dy_ttm`
7. 状态字段：`is_suspend`, `is_st`

## 5. SQL 逻辑建议

```sql
with quotes as (
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
    from {{ ref('int_stock_quotes_daily_unadj') }}
)
```

最终 select 建议从 `quotes` 左连接：

1. `int_stock_quotes_daily_unadj`：提供基础行情、交易指标、涨跌停价、市值、股本和股息率字段。
2. `int_stock_financial_valuation`：按最近 `report_date <= trade_date` 提供 `pe_static`, `pe_ttm`, `pe_forecast`, `pb_mrq`, `book_value_per_share`, `roe`, `roa`, `roaa`, `roae`。

实现注意：

- 不在 mart 内重新 join `int_stock_shares_history` 或 `int_stock_exrights_event` 计算交易指标、市值、股本或股息率；这些字段由 `int_stock_quotes_daily_unadj` 维护。
- 不在 mart 内从 `stg_eastmoney__income_*` 或 `stg_eastmoney__balance` 直接计算 PE/PB、每股净资产、ROE、ROA、ROAA 或 ROAE。

## 6. 测试建议

- 模型级组合唯一：`security_code`, `trade_date`。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- `is_suspend`: `not_null`。
- `turnover_rate`, `turnover_rate_actual`, `pct_amplitude`, `dy_static`, `dy_ttm`: 可空；非空时应大于等于 `0`。
- `pct_change`: 可空；允许为负值。
- `limit_up_price`, `limit_down_price`: 可空；非空时应大于 `0`，且 `limit_up_price >= limit_down_price`。
- `a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap`: 可空；非空时应大于等于 `0`。
- `a_shares`, `a_float_shares`, `a_free_float_shares`: 可空；非空时应大于等于 `0`。
- `pe_static`, `pe_ttm`, `pe_forecast`, `pb_mrq`, `book_value_per_share`: 可空；非空时应大于 `0`。
- `roe`, `roa`, `roaa`, `roae`: 可空；允许为负值。
- 定向数据测试：
  - 本模型行数应等于 `int_stock_quotes_daily_unadj` 行数。
  - 输出主键集合应等于 `int_stock_quotes_daily_unadj` 主键集合。
  - 当 `a_float_shares > 0` 且 `volume` 非空时，`turnover_rate = volume / a_float_shares * 100`。
  - 当 `a_free_float_shares > 0` 且 `volume` 非空时，`turnover_rate_actual = volume / a_free_float_shares * 100`。
  - 当 `prev_close_price > 0` 且 `high_price`, `low_price` 非空时，`pct_amplitude = (high_price - low_price) / prev_close_price * 100`。
  - 当 `prev_close_price > 0` 且 `close_price` 非空时，`pct_change = (close_price - prev_close_price) / prev_close_price * 100`。
  - 当市值和股本字段均非空时，应满足 `a_free_float_market_cap <= a_float_market_cap <= a_market_cap`，允许因股本源异常单独做质量告警而非硬失败。
  - `int_stock_financial_valuation.report_date` as-of 来源日期不得大于 `trade_date`。
  - `roe`、`roa`、`roaa` 和 `roae` 来源值应与最近报告期 `int_stock_financial_valuation` 对应字段一致。
  - `turnover_rate`, `turnover_rate_actual`, `pct_amplitude`, `pct_change`, `limit_up_price`, `limit_down_price`, `a_market_cap`, `a_float_market_cap`, `a_free_float_market_cap`, `a_shares`, `a_float_shares`, `a_free_float_shares`, `dy_static`, `dy_ttm` 应与 `int_stock_quotes_daily_unadj` 对应字段一致。

## 7. 延后事项

- 如后续需要用当日收盘价重算日频 PE/PB，应新增专门日频估值模型，不覆盖本 mart 中来自 `int_stock_financial_valuation` 的财报期 as-of 字段。
- 输出前复权或后复权价格宽表字段。
- 引入多行情源优先级合并、跨源对账和异常价格修正。
- 对停牌日换手率、振幅和涨跌停价是否输出做更细业务裁决。
- 增加行业、市值分层、上市状态、证券简称等维度字段。
