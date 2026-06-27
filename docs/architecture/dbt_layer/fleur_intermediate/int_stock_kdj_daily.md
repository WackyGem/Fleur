# int_stock_kdj_daily 设计

状态：Design

依据：

- Calculation source：`source('fleur_calculation', 'calc_stock_kdj_daily')`
- 上游计划：`docs/plans/archive/0027-furnace-rsv-kdj-technical-indicators-implementation-plan.md`
- 目标 SQL：`pipeline/elt/models/intermediate/int_stock_kdj_daily.sql`
- 目标 YAML：`pipeline/elt/models/intermediate/int_stock_kdj_daily.yml`

## 1. 模型定位

A 股股票日频 KDJ intermediate wrapper。模型包装 Furnace/Dagster 已物化的 canonical KDJ 结果，保留参数字段用于自描述和下游核验。

本模型不在 dbt 中重算 RSV/KDJ 递推公式。生产第一版只允许 canonical `KDJ(9,3,3)`，如果未来需要多参数集，应重新设计 grain 或新建参数化结果表。

## 2. 数据粒度与依赖

- 直接依赖：`fleur_calculation.calc_stock_kdj_daily`。
- 粒度：每证券、交易日一行。
- 候选键：`security_code`, `trade_date`。
- 输入口径：上游 Furnace 固定使用前复权价格序列。
- 参数口径：`rsv_window = 9`, `k_smoothing = 3`, `d_smoothing = 3`。

## 3. 字段设计

| Intermediate 字段 | 来源/派生 | 类型建议 | 设计说明 |
|--------------------|-----------|----------|----------|
| `security_code` | calculation | `String` | 股票标准连接代码。 |
| `trade_date` | calculation | `Date` | 指标交易日。 |
| `rsv_window` | calculation | `UInt16` | RSV 滚动窗口；第一版必须为 9。 |
| `k_smoothing` | calculation | `UInt16` | K 平滑分母；第一版必须为 3。 |
| `d_smoothing` | calculation | `UInt16` | D 平滑分母；第一版必须为 3。 |
| `rsv` | calculation | `Nullable(Float64)` | RSV 值；当前行无完整有效 RSV 窗口时为 NULL。 |
| `k_value` | calculation | `Nullable(Float64)` | K 值；当前行 RSV 不可用时为 NULL。 |
| `d_value` | calculation | `Nullable(Float64)` | D 值；当前行 RSV 不可用时为 NULL。 |
| `j_value` | calculation | `Nullable(Float64)` | J 值；当前行 RSV 不可用时为 NULL。 |

## 4. 测试建议

- 组合键：`security_code`, `trade_date` 唯一。
- `security_code`: `not_null`，`cn_security_code_format`。
- `trade_date`: `not_null`。
- 参数字段非空且固定为 `9 / 3 / 3`。
