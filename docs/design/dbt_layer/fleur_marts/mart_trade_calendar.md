# mart_trade_calendar

状态：Proposed

## 粒度

每个 A 股交易日一行：`trade_date`。

## 职责

- 从 `int_trade_calendar` 透传 `trade_date` 和 `prev_trade_date`。
- 作为 Rearview 后端服务读取交易日历的稳定 mart contract。
- 支持组合发布预检解析 `source_signal_date` 之后的下一交易日，避免使用行情事实表推断未来交易日。

## 非职责

- 不补全自然日历。
- 不派生交易周、交易月、月末交易日或节假日标签。
- 不重写 `prev_trade_date` 计算逻辑；相邻交易日定义保留在 `int_trade_calendar`。
