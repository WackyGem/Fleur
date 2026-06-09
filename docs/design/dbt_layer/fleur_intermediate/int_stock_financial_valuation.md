# int_stock_financial_valuation 设计

状态：Design

依据：

- Intermediate model：`ref('int_stock_quotes_daily_unadj')`
- Intermediate model：`ref('int_stock_shares_history')`
- Staging model：`ref('stg_eastmoney__income_sq')`
- Staging model：`ref('stg_eastmoney__income_ytd')`
- Staging model：`ref('stg_eastmoney__balance')`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_quotes_daily_unadj.md`
- Intermediate 设计：`docs/design/dbt_layer/fleur_intermediate/int_stock_shares_history.md`
- 目标位置：`pipeline/elt/models/intermediate/int_stock_financial_valuation.sql`

## 1. 模型定位

财报期末估值 intermediate 模型。模型以 `security_code` + `report_date` 为粒度，基于财报期末对应的最近交易日收盘价、报告期 as-of 股本、利润表和资产负债表，输出静态市盈率、TTM 市盈率、动态市盈率、MRQ 市净率、每股净资产、ROE、ROA、ROAA 和 ROAE。

本模型服务于财报期维度的基本面估值分析，不是日频估值事实表。日频市值、日频 PE/PB、公告日可投资口径和交易日展开应放到后续日频 valuation 模型。

## 2. 输出字段

第一版只输出用户指定字段：

| 字段 | 类型建议 | 说明 |
|------|----------|------|
| `security_code` | `String` | A 股标准证券代码。 |
| `report_date` | `Date` | 财报报告期截止日。 |
| `pe_static` | `Nullable(Float64)` | 静态市盈率，使用最近年度年报归母净利润。 |
| `pe_ttm` | `Nullable(Float64)` | TTM 市盈率，使用最近四个单季度归母净利润合计。 |
| `pe_forecast` | `Nullable(Float64)` | 动态市盈率，使用最新一期季报归母净利润按季度数年化估算。 |
| `pb_mrq` | `Nullable(Float64)` | MRQ 市净率，使用当前报告期或最近季度归母权益。 |
| `book_value_per_share` | `Nullable(Float64)` | 每股净资产，使用最近资产负债表的归属于母公司股东权益合计除以实收资本。 |
| `roe` | `Nullable(Float64)` | ROE，使用归母净利润除以期末归母净资产。 |
| `roa` | `Nullable(Float64)` | ROA，使用归母净利润除以期末资产总计。 |
| `roaa` | `Nullable(Float64)` | ROAA，使用归母净利润除以平均资产总计。 |
| `roae` | `Nullable(Float64)` | ROAE，使用归母净利润除以平均归母净资产。 |

候选键：`security_code`, `report_date`。

## 3. 依赖与取值规则

价格与市值：

- 价格来源：`int_stock_quotes_daily_unadj.close_price`。
- 观察价格日期：同一证券 `trade_date <= report_date` 的最近交易日。
- 股本来源：`int_stock_shares_history.total_shares`。
- 股本 as-of：`effective_date <= report_date` 且 `expiry_date is null or expiry_date >= report_date`。
- 总市值：`close_price * total_shares`，单位为元。

利润表：

- `stg_eastmoney__income_ytd.parent_netprofit`：年初至报告期末归母净利润。
- `stg_eastmoney__income_sq.parent_netprofit`：单季度归母净利润。
- `stg_eastmoney__income_ytd.report_type` 用于判断最新一期季报对应的季度数。

资产负债表：

- `stg_eastmoney__balance.total_assets`：资产总计。
- `stg_eastmoney__balance.total_parent_equity`：归属于母公司股东权益合计。
- `stg_eastmoney__balance.share_capital`：实收资本（股本）。
- `pb_mrq`、`book_value_per_share`、`roe` 和 `roa` 使用同一证券 `balance.report_date <= current report_date` 的最近一条资产负债表记录。
- `roaa` 使用期初和期末资产总计的平均值。第一版按自然财年口径取期初资产：同一证券上一年度年报 `report_date = toDate(concat(toString(toYear(current report_date) - 1), '-12-31'))` 的 `total_assets`。
- `roae` 使用期初和期末归母净资产的平均值。第一版按自然财年口径取期初净资产：同一证券上一年度年报 `report_date = toDate(concat(toString(toYear(current report_date) - 1), '-12-31'))` 的 `total_parent_equity`。

报告期集合：

- 第一版以 `stg_eastmoney__income_ytd` 和 `stg_eastmoney__balance` 的 `security_code + report_date` 并集作为报告期集合。
- 如某个报告期缺少价格、股本、利润或权益输入，对应估值字段输出 `NULL`，不回填到其他证券或其他报告期。

## 4. 指标口径

统一防御规则：

- 分母为 `NULL`、`0` 或小于 `0` 时，对应估值输出 `NULL`。
- 价格或总股本为 `NULL`、`0` 或小于 `0` 时，所有估值输出 `NULL`。
- 不输出负 PE/PB。负利润或负权益在估值语义上不可比，保留给后续质量诊断模型处理。
- `roe`、`roa`、`roaa` 和 `roae` 是盈利能力指标，不套用“非空必须大于 0”的估值规则；归母净利润为负时允许输出负值。

静态市盈率：

```text
pe_static =
    market_cap_at_report_date
    / latest_annual_parent_netprofit_as_of_report_date
```

TTM 市盈率：

```text
pe_ttm =
    market_cap_at_report_date
    / sum(last 4 stg_eastmoney__income_sq.parent_netprofit)
```

TTM 规则：

- 最近四个单季度必须覆盖当前 `report_date` 及其之前的四个季度。
- 不足四个季度时输出 `NULL`。
- 不用 YTD 差分替代单季度利润，避免混合口径。

动态市盈率：

```text
pe_forecast =
    market_cap_at_report_date
    / forecast_full_year_parent_netprofit
```

预估全年净利润：

```text
forecast_full_year_parent_netprofit =
    latest_ytd_parent_netprofit * (4 / quarter_count)
```

季度数映射：

| `report_type` | `quarter_count` | 预估全年净利润 |
|---------------|-----------------|----------------|
| `一季报` | `1` | `Q1 归母净利润 * 4` |
| `中报` | `2` | `H1 归母净利润 * 2` |
| `三季报` | `3` | `Q1~Q3 归母净利润 * 4 / 3` |
| `年报` | `4` | `全年归母净利润` |

年报按 `quarter_count = 4` 参与动态年化估算，此时 `pe_forecast` 与使用同一年度年报分母的 `pe_static` 结果一致；保留两个字段是为了让所有报告期都有统一的动态 PE 字段。

MRQ 市净率：

```text
pb_mrq =
    market_cap_at_report_date
    / latest_total_parent_equity_as_of_report_date
```

每股净资产：

```text
book_value_per_share =
    latest_total_parent_equity_as_of_report_date
    / latest_share_capital_as_of_report_date
```

规则：

- `latest_total_parent_equity_as_of_report_date` 来自 `stg_eastmoney__balance.total_parent_equity`。
- `latest_share_capital_as_of_report_date` 来自 `stg_eastmoney__balance.share_capital`，即资产负债表中的“实收资本（股本）”。
- 当归母权益或实收资本为 `NULL`、`0` 或小于 `0` 时，`book_value_per_share` 输出 `NULL`。

ROE：

```text
roe =
    latest_ytd_parent_netprofit
    / latest_total_parent_equity_as_of_report_date
```

ROA：

```text
roa =
    latest_ytd_parent_netprofit
    / latest_total_assets_as_of_report_date
```

ROAA：

```text
average_total_assets =
    (
        beginning_total_assets_for_report_year
        + latest_total_assets_as_of_report_date
    ) / 2

roaa =
    latest_ytd_parent_netprofit
    / average_total_assets
```

ROAE：

```text
average_parent_equity =
    (
        beginning_total_parent_equity_for_report_year
        + latest_total_parent_equity_as_of_report_date
    ) / 2

roae =
    latest_ytd_parent_netprofit
    / average_parent_equity
```

规则：

- `latest_ytd_parent_netprofit` 来自当前报告期 `stg_eastmoney__income_ytd.parent_netprofit`。
- `latest_total_assets_as_of_report_date` 来自当前报告期 as-of 最近资产负债表的 `total_assets`。
- `latest_total_parent_equity_as_of_report_date` 来自当前报告期 as-of 最近资产负债表的 `total_parent_equity`。
- `beginning_total_assets_for_report_year` 来自上一年度年报资产负债表的 `total_assets`。
- `beginning_total_parent_equity_for_report_year` 来自上一年度年报资产负债表的 `total_parent_equity`。
- `roe`、`roa`、`roaa` 和 `roae` 使用比率口径，不乘以 `100`；`0.12` 表示 `12%`。
- 当归母净利润为 `NULL` 时，`roe`、`roa`、`roaa` 和 `roae` 输出 `NULL`。
- 当期末资产总计为 `NULL`、`0` 或小于 `0` 时，`roa` 输出 `NULL`。
- 当期初资产总计、期末资产总计或平均资产总计为 `NULL`、`0` 或小于 `0` 时，`roaa` 输出 `NULL`。
- 当期末归母净资产为 `NULL`、`0` 或小于 `0` 时，`roe` 输出 `NULL`。
- 当期初归母净资产、期末归母净资产或平均归母净资产为 `NULL`、`0` 或小于 `0` 时，`roae` 输出 `NULL`。

## 5. SQL 逻辑建议

```sql
with report_periods as (
    select security_code, report_date
    from {{ ref('stg_eastmoney__income_ytd') }}

    union distinct

    select security_code, report_date
    from {{ ref('stg_eastmoney__balance') }}
),

report_price as (
    select
        report_periods.security_code,
        report_periods.report_date,
        argMax(quotes.close_price, quotes.trade_date) as close_price
    from report_periods
    left join {{ ref('int_stock_quotes_daily_unadj') }} as quotes
        on report_periods.security_code = quotes.security_code
        and quotes.trade_date <= report_periods.report_date
    group by
        report_periods.security_code,
        report_periods.report_date
),

report_shares as (
    select
        report_periods.security_code,
        report_periods.report_date,
        shares.total_shares
    from report_periods
    left join {{ ref('int_stock_shares_history') }} as shares
        on report_periods.security_code = shares.security_code
        and shares.effective_date <= report_periods.report_date
        and (
            shares.expiry_date is null
            or shares.expiry_date >= report_periods.report_date
        )
),

market_cap as (
    select
        report_periods.security_code,
        report_periods.report_date,
        report_price.close_price * report_shares.total_shares as market_cap
    from report_periods
    left join report_price
        on report_periods.security_code = report_price.security_code
        and report_periods.report_date = report_price.report_date
    left join report_shares
        on report_periods.security_code = report_shares.security_code
        and report_periods.report_date = report_shares.report_date
)
```

后续 CTE：

1. `latest_annual_income`：按 `report_type = '年报'` as-of 取最近年度 `parent_netprofit`。
2. `ttm_income`：从 `stg_eastmoney__income_sq` 对当前报告期向前取最近四个季度并求和。
3. `forecast_income`：从 `stg_eastmoney__income_ytd` 取当前报告期 `parent_netprofit`，按 `report_type` 映射季度数后计算 `parent_netprofit * (4 / quarter_count)`；年报的季度数为 4。
4. `latest_mrq_equity`：资产负债表 as-of 最近 `total_assets`、`total_parent_equity` 和 `share_capital`。
5. `beginning_balance_sheet`：按当前 `report_date` 所属自然财年，取上一年度年报 `total_assets` 和 `total_parent_equity`。
6. `profitability_ratios`：使用当前报告期 YTD 归母净利润、期末资产总计、平均资产总计、期末归母净资产和平均归母净资产计算 `roe`、`roa`、`roaa`、`roae`。
7. 最终 select 只输出十一个字段。

估值除法建议封装为局部表达式：

```sql
if(
    numerator is null
    or numerator <= 0
    or denominator is null
    or denominator <= 0,
    cast(null, 'Nullable(Float64)'),
    numerator / denominator
)
```

## 6. 测试建议

- 模型级组合唯一：`security_code`, `report_date`。
- `security_code`: `not_null`, `cn_security_code_format`。
- `report_date`: `not_null`。
- `pe_static`, `pe_ttm`, `pe_forecast`, `pb_mrq`, `book_value_per_share`, `roe`, `roa`, `roaa`, `roae`: 可空，不做 `not_null`。
- 增加定向数据测试：
  - 输出每个 `security_code + report_date` 最多一行。
  - 非空 `pe_static`, `pe_ttm`, `pe_forecast`, `pb_mrq`, `book_value_per_share` 必须大于 `0`。
  - `pe_ttm` 只有在四个单季度归母净利润齐全且 TTM 分母大于 0 时非空。
  - `pe_forecast` 只有在 YTD 归母净利润分母大于 0 且 `report_type` 可识别时非空。
  - `pb_mrq` 使用的最近权益日期不得大于当前 `report_date`。
  - `book_value_per_share` 使用的最近资产负债表日期不得大于当前 `report_date`，且非空值必须大于 `0`。
  - `roe` 使用的期末资产负债表日期不得大于当前 `report_date`。
  - `roa` 使用的期末资产负债表日期不得大于当前 `report_date`。
  - `roaa` 使用的期初资产负债表日期应等于上一年度年报日期，期末资产负债表日期不得大于当前 `report_date`。
  - `roae` 使用的期初资产负债表日期应等于上一年度年报日期，期末资产负债表日期不得大于当前 `report_date`。
  - `roe`、`roa`、`roaa` 和 `roae` 允许为负值；非空时只要求分母大于 `0`。

## 7. 边界与延后事项

- 不按公告日判断可投资性。第一版 `report_date` 是财报期末日，不代表市场已知日期。
- 不输出 `trade_date`、`market_cap`、利润分母、权益分母、平均权益分母、实收资本分母等诊断字段；如果后续调试需要，可另建宽诊断模型或在 YAML meta 中记录口径。
- 不接入分析师预测。`pe_forecast` 是 YTD 年化动态 PE，不是券商一致预期 forward PE。
- 不做金融、保险、银行等行业专用估值口径差异化处理。
- 不处理复权价格；估值价格使用未复权收盘价。
- 不对负利润、负权益输出负估值倍数。
- 不展开到日频。日频表建议命名为 `int_stock_valuation_daily`，并使用 `trade_date` 粒度。
