# mart_benchmark_returns_daily

状态：Proposed

## 粒度

每个 benchmark 指数、每个交易日一行：`security_code + trade_date`。

## 职责

- 从 `int_benchmark_returns_daily` 透传 `security_code`、`trade_date`、`return_daily`。
- 作为 portfolio worker 读取 benchmark 日收益率的稳定 mart contract。
- 维持 price index benchmark 口径，不在 mart 层派生窗口指标。

## 非职责

- 不透出 `close_price` 和 `prev_close_price`；价格审计保留在 intermediate 层。
- 不计算 benchmark 年化收益、累计收益、Alpha、Beta 或信息比率。
- 不维护新的 benchmark universe。
