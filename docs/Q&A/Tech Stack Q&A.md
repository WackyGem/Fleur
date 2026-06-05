# Tech Stack 指标计算 Q&A

本文件记录常见行情、股本、估值和分红指标的计算口径，以及当前 dbt staging 层是否已经具备计算所需字段。

当前判断基于以下 staging 输入：

- `stg_baostock__query_history_k_data_plus_daily`：日线行情，含 OHLC、昨收、成交量、成交额、换手率、涨跌幅、ST 标记。
- `stg_baostock__query_stock_basic`：证券基础信息，含交易所、证券类型、板块和上市状态。
- `stg_eastmoney__equity_history`：股本变动历史，含总股本、流通股、A 股流通股、自由流通股本和非自由流通字段。
- `stg_eastmoney__income_sq` / `stg_eastmoney__income_ytd`：利润表单季度 / 年初至今，含归母净利润和 EPS。
- `stg_eastmoney__balance`：资产负债表，含归属于母公司股东权益。
- `stg_eastmoney__dividend_main`：分红主表，含分红方案、分红总额、A 股分红总额和关键事件日期。
- `stg_ths__limit_up_pool_compacted`：同花顺涨停池事件，只覆盖涨停池样本，不是全市场行情事实。

## 指标支持矩阵

| 指标 | 公式 / 口径 | 当前 stg 支持状态 | 主要字段 | 备注 |
|---|---|---|---|---|
| 换手（换手率） | `当日成交总股数 / 流通股本 * 100%` | 现成 + 可复算 | `turnover_rate`, `volume`, `listed_a_shares` / `unlimited_shares` | BaoStock 已提供 `turnover_rate`。复算时需确认成交量单位并按交易日 as-of join 股本。 |
| 振幅 | `(当日最高价 - 当日最低价) / 前一日收盘价 * 100%` | 可计算 | `high_price`, `low_price`, `prev_close_price` | 建议采用昨收口径；停牌或昨收为 0 时置 NULL。 |
| 换手(实) / 实际换手率 | `当日成交总股数 / 自由流通股本 * 100%` | 可计算 | `volume`, `free_shares` | `free_shares` 来自 EastMoney `FREE_SHARES`，需按交易日 as-of join。 |
| 涨停价 | `前一日收盘价 * (1 + 涨跌幅限制比例)` | 可计算理论价 | `prev_close_price`, `security_board`, `is_st` | 需要 A 股涨跌幅规则和价格 tick 四舍五入。THS 涨停池可校验部分涨停事件。 |
| 跌停价 | `前一日收盘价 * (1 - 涨跌幅限制比例)` | 可计算理论价 | `prev_close_price`, `security_board`, `is_st` | 当前无全市场跌停事件表，只能从日线理论价与行情匹配判断。 |
| 外盘 | 以卖出价成交的累计手数（主动买入） | 尚缺失 | 无 | 需要逐笔成交或盘口成交方向数据；当前 stg 没有。 |
| 内盘 | 以买入价成交的累计手数（主动卖出） | 尚缺失 | 无 | 同上。 |
| 市盈(静) | `总市值 / 上一年度归母净利润` 或 `当前股价 / 上一年度 EPS` | 可计算 | `close_price`, `total_shares`, `parent_netprofit`, `basic_eps` | 需选择上一年度年报，按公告 / 报告期口径定版本。 |
| 市盈(TTM) | `总市值 / 最近 12 个月归母净利润` 或 `当前股价 / TTM EPS` | 可计算 | `close_price`, `total_shares`, `income_sq.parent_netprofit` | 优先用最近四个单季度归母净利润滚动求和。 |
| 市盈(动) | `总市值 / 预估全年净利润`，常见为最新季报年化 | 可计算但需定规则 | `close_price`, `total_shares`, `basic_eps`, `parent_netprofit` | Q1 * 4、H1 * 2、Q3 * 4/3 等规则需在 mart 明确定义。 |
| 总市值 | `总股本 * 当前股价` | 可计算 | `total_shares`, `close_price` | 股本按交易日 as-of join。 |
| 流通值 / 流通市值 | `流通股本 * 当前股价` | 可计算 | `listed_a_shares` / `unlimited_shares`, `close_price` | A 股场景优先明确使用 `listed_a_shares` 还是 `unlimited_shares`。 |
| 总股本 | 公司发行的全部股份数量 | 现成 | `total_shares` | 来自 EastMoney 股本变动历史。 |
| 流通股 | 可以在二级市场交易的股份数量 | 现成 | `listed_a_shares`, `unlimited_shares` | 两个字段口径不同，指标模型需固定默认口径。 |
| 自由流通股 | 剔除非自由流通部分后的股本 | 现成 | `free_shares`, `non_free_shares`, `non_free_shares_ratio`, `is_free_window` | `free_shares` 是供应商字段，不由 `total_shares` 临时推导。 |
| 自由流通值 / 自由流通市值 | `自由流通股本 * 当前股价` | 可计算 | `free_shares`, `close_price` | 需按交易日 as-of join。 |
| 市净(MRQ) | `总市值 / 最新季度归母净资产` 或 `当前股价 / 最新每股净资产` | 可计算 | `close_price`, `total_shares`, `total_parent_equity` | 最新季度取 MRQ；版本选择放在 intermediate/mart。 |
| 股息率 | `上一年度每股现金分红 / 当前股价 * 100%`，也可用近 12 个月现金分红 | 可计算但需清洗分红口径 | `new_profile`, `total_dividend`, `total_dividend_a`, `ex_dividend_date`, `close_price` | 可解析 `10派X元` 得每股分红，或用分红总额 / 股本推导；需过滤“不分配不转增”和选择实施方案。 |

## 当前仍缺失的数据

1. 外盘 / 内盘：缺逐笔成交、盘口价位和主动买卖方向。
2. 全市场涨停 / 跌停事件：可以计算理论价格，但当前只有涨停池事件样本，没有全市场涨停/跌停事件事实。
3. 精确涨跌停规则表：需要沉淀交易所、板块、ST、北交所、历史制度变化和价格 tick 规则。
4. 分红每股现金字段：当前有分红方案文本和分红总额，尚未解析为结构化每股现金分红。

## 推荐落地模型

1. `int_stock_share_capital_asof`：把 `stg_eastmoney__equity_history` 按交易日展开 / as-of 到每日股本口径。
2. `int_stock_quotes_daily_unadj`：统一日线行情、交易状态和证券基础信息。
3. `int_stock_valuation_daily`：计算总市值、流通市值、自由流通市值、PE、PB、股息率等估值指标。
4. `int_stock_price_limit_daily`：计算理论涨停价、跌停价和是否触及涨跌停。
