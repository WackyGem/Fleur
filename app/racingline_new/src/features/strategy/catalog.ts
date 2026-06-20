import type {
  ConditionOperator,
  IndicatorCatalog,
  MetricValueType,
  Step,
} from "@/features/strategy/types"

export const indicatorCatalog: IndicatorCatalog[] = [
  {
    id: "quotes",
    label: "行情与涨跌",
    source: "mart_stock_quotes_daily",
    metrics: [
      { id: "close_price", valueType: "number" },
      { id: "volume", valueType: "number" },
      { id: "pct_change", valueType: "number" },
      { id: "pct_amplitude", valueType: "number" },
    ],
  },
  {
    id: "trend",
    label: "趋势均线",
    source: "mart_stock_trend_indicator",
    metrics: [
      { id: "price_ma_5", valueType: "number" },
      { id: "price_ma_10", valueType: "number" },
      { id: "price_ma_20", valueType: "number" },
      { id: "price_ma_30", valueType: "number" },
      { id: "price_ma_60", valueType: "number" },
      { id: "price_ma_250", valueType: "number" },
      { id: "boll_dn_20_2", valueType: "number" },
    ],
  },
  {
    id: "momentum",
    label: "动量指标",
    source: "mart_stock_momentum_indicator",
    metrics: [
      { id: "kdj_j_value", valueType: "number" },
      { id: "rsi_6", valueType: "number" },
    ],
  },
  {
    id: "volume",
    label: "量能指标",
    source: "mart_stock_volume_indicator",
    metrics: [{ id: "volume_ma_5", valueType: "number" }],
  },
  {
    id: "pattern",
    label: "形态特征",
    source: "mart_stock_price_pattern_daily",
    metrics: [
      { id: "close_down_streak_days", valueType: "number" },
      { id: "n_structure_20_is_valid", valueType: "boolean" },
      { id: "n_structure_20_second_low_ratio", valueType: "number" },
    ],
  },
]

export const operatorOptions: Array<{
  value: ConditionOperator
  label: string
  targets: Array<"value" | "metric">
  valueTypes: MetricValueType[]
}> = [
  {
    value: "gt",
    label: ">",
    targets: ["value", "metric"],
    valueTypes: ["number"],
  },
  {
    value: "gte",
    label: ">=",
    targets: ["value", "metric"],
    valueTypes: ["number"],
  },
  {
    value: "lt",
    label: "<",
    targets: ["value", "metric"],
    valueTypes: ["number"],
  },
  {
    value: "lte",
    label: "<=",
    targets: ["value", "metric"],
    valueTypes: ["number"],
  },
  {
    value: "eq",
    label: "=",
    targets: ["value", "metric"],
    valueTypes: ["number", "boolean"],
  },
  {
    value: "neq",
    label: "!=",
    targets: ["value", "metric"],
    valueTypes: ["number", "boolean"],
  },
  {
    value: "between",
    label: "区间内",
    targets: ["value"],
    valueTypes: ["number"],
  },
  {
    value: "crosses_above",
    label: "上穿",
    targets: ["metric"],
    valueTypes: ["number"],
  },
  {
    value: "crosses_below",
    label: "下穿",
    targets: ["metric"],
    valueTypes: ["number"],
  },
]

export const strategySteps: Array<{ id: Step; label: string }> = [
  { id: "indicators", label: "策略选股" },
  { id: "weights", label: "权重配置" },
  { id: "preview", label: "股池预览" },
  { id: "simulation", label: "模拟建仓" },
  { id: "backtest", label: "策略回测" },
]
