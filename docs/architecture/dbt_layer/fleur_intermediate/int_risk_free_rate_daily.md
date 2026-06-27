# int_risk_free_rate_daily

状态：Proposed

## 粒度

每个 A 股交易日、每个无风险利率来源期限一行：`trade_date + source_tenor`。

## 职责

- 从 `int_government_bond_yields_daily.one_year_yield_pct` 读取 1 年期国债收益率。
- 将百分比点口径转换为小数比例 `annual_rate`。
- 使用 `int_trade_calendar` 生成交易日栅格。
- 对 `source_date <= trade_date` 的历史收益率做 forward-fill，不使用未来值。
- 按 `(1 + annual_rate) ^ (1 / 252) - 1` 生成 `daily_rate`。

## 非职责

- 不在行字段中保存 `annualization_days`、`day_count_basis`、`daily_method` 或 `fill_strategy`。
- 不负责绩效指标公式计算。
- 不做多期限插值；第一版只支持 `source_tenor = '1y'`。

## 下游

- `mart_risk_free_rate_daily`
- portfolio worker 绩效指标计算输入
