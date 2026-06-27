# mart_risk_free_rate_daily

状态：Proposed

## 粒度

每个 A 股交易日、每个无风险利率来源期限一行：`trade_date + source_tenor`。

## 职责

- 从 `int_risk_free_rate_daily` 选择稳定字段。
- 作为 portfolio worker 和 dbt wrapper 读取无风险日收益率的稳定 mart contract。
- 保持字段集小而稳定：`trade_date`、`source_date`、`source_tenor`、`annual_rate`、`daily_rate`。

## 非职责

- 不重新实现期限选择、单位转换、forward-fill 或日频折算。
- 不保存配置元数据字段。
- 不计算窗口级绩效指标。
