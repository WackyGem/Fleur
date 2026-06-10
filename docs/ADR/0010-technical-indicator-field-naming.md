# ADR 0010: 技术指标字段命名区分窗口参数和算子重数

状态：Accepted

日期：2026-06-08

## 背景

Furnace 技术指标逐步覆盖 KDJ、MA、RSI、BOLL 和价格行为结构。Moving Average 计划中同时存在价格均线、成交量均线、组合均线和二重 EMA。如果字段名只使用裸 `ma_*`、`avg_ma_*` 或把算子重数写成额外下划线，容易让窗口参数、价格口径、成交量口径和算子次数混在一起。

需要一个长期命名规则，避免 dbt、Rust、ClickHouse 表和 marts 消费侧形成多个等价字段名。

## 决策

技术指标字段命名必须显式表达口径、算子和参数：

- 价格指标使用 `price_` 前缀，例如 `price_ma_5`。
- 成交量指标使用 `volume_` 前缀，例如 `volume_ma_5`。
- 多窗口组合指标把窗口参数依次写入字段名，例如 `price_avg_ma_3_6_12_24`。
- EMA 复合次数作为算子名的一部分，二重 EMA 写作 `ema2`，窗口参数仍使用下划线分隔，例如 `price_ema2_10`。
- RSI、KDJ、BOLL 等不存在口径歧义的指标可以使用指标名前缀和参数字段，例如 `rsi_14`、`rsv_window`、`k_smoothing`。

禁止新增以下形态：

- 裸价格均线字段：`ma_5`、`avg_ma_3_6_12_24`。
- 缺少口径的成交量均线字段：`ma_volume_5`。
- 紧凑窗口字段：`price_ma5`、`volume_ma5`。
- 把 EMA 重数误写成窗口参数：`price_ema_2_10`。

## 后果

- Furnace Rust 输出 schema、ClickHouse calculation 表、dbt source、thin wrapper 和 mart 字段必须使用同一命名。
- 历史 RFC 或计划中保留的裸字段名只作为历史需求表述，不作为实现契约。
- dbt 和 Rust 测试应覆盖错误字段名不存在，避免兼容别名长期滞留。

## 关联文档

- `docs/RFC/0017-furnace-moving-average-technical-indicators.md`
- `docs/plans/archive/0029-furnace-moving-average-technical-indicators-implementation-plan.md`
- `docs/jobs/reports/2026-06-08-furnace-ma-full-market-parallel-validation.md`
