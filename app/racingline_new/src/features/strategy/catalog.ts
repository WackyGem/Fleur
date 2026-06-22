import type {
  ConditionOperator,
  IndicatorCatalog,
  MetricValueType,
  Step,
} from "@/features/strategy/types"

const numericOps: ConditionOperator[] = [
  "gt",
  "gte",
  "lt",
  "lte",
  "eq",
  "ne",
  "between",
  "is_null",
]

const booleanOps: ConditionOperator[] = ["eq", "ne", "is_null"]
const crossingOps: ConditionOperator[] = [
  ...numericOps,
  "crosses_above",
  "crosses_below",
]

export const indicatorCatalog: IndicatorCatalog[] = [
  {
    id: "quotes",
    label: "行情与涨跌",
    source: "mart_stock_quotes_daily",
    metrics: [
      {
        id: "close_price",
        label: "close_price",
        valueType: "number",
        allowedOps: numericOps,
      },
      {
        id: "volume",
        label: "volume",
        valueType: "number",
        allowedOps: numericOps,
      },
      {
        id: "pct_change",
        label: "pct_change",
        valueType: "number",
        allowedOps: numericOps,
      },
      {
        id: "pct_amplitude",
        label: "pct_amplitude",
        valueType: "number",
        allowedOps: numericOps,
      },
    ],
  },
  {
    id: "trend",
    label: "趋势均线",
    source: "mart_stock_trend_indicator_daily",
    metrics: [
      {
        id: "price_ma_5",
        label: "price_ma_5",
        valueType: "number",
        allowedOps: crossingOps,
        previousMetric: "prev_price_ma_5",
        supportsCrossing: true,
      },
      {
        id: "price_ma_10",
        label: "price_ma_10",
        valueType: "number",
        allowedOps: crossingOps,
        previousMetric: "prev_price_ma_10",
        supportsCrossing: true,
      },
      {
        id: "price_ma_20",
        label: "price_ma_20",
        valueType: "number",
        allowedOps: crossingOps,
        previousMetric: "prev_price_ma_20",
        supportsCrossing: true,
      },
      {
        id: "price_ma_30",
        label: "price_ma_30",
        valueType: "number",
        allowedOps: crossingOps,
        previousMetric: "prev_price_ma_30",
        supportsCrossing: true,
      },
      {
        id: "price_ma_60",
        label: "price_ma_60",
        valueType: "number",
        allowedOps: crossingOps,
        previousMetric: "prev_price_ma_60",
        supportsCrossing: true,
      },
      {
        id: "price_ma_250",
        label: "price_ma_250",
        valueType: "number",
        allowedOps: crossingOps,
        previousMetric: "prev_price_ma_250",
        supportsCrossing: true,
      },
      {
        id: "boll_lower_20_2",
        label: "boll_lower_20_2",
        valueType: "number",
        allowedOps: crossingOps,
        previousMetric: "prev_boll_lower_20_2",
        supportsCrossing: true,
      },
    ],
  },
  {
    id: "momentum",
    label: "动量指标",
    source: "mart_stock_momentum_indicator",
    metrics: [
      {
        id: "kdj_j_value",
        label: "kdj_j_value",
        valueType: "number",
        allowedOps: numericOps,
      },
      {
        id: "rsi_6",
        label: "rsi_6",
        valueType: "number",
        allowedOps: numericOps,
      },
    ],
  },
  {
    id: "volume",
    label: "量能指标",
    source: "mart_stock_volume_indicator",
    metrics: [
      {
        id: "volume_ma_5",
        label: "volume_ma_5",
        valueType: "number",
        allowedOps: numericOps,
      },
    ],
  },
  {
    id: "pattern",
    label: "形态特征",
    source: "mart_stock_price_pattern_daily",
    metrics: [
      {
        id: "close_down_streak_days",
        label: "close_down_streak_days",
        valueType: "number",
        allowedOps: numericOps,
      },
      {
        id: "n_structure_20_is_valid",
        label: "n_structure_20_is_valid",
        valueType: "boolean",
        allowedOps: booleanOps,
      },
      {
        id: "n_structure_20_second_low_ratio",
        label: "n_structure_20_second_low_ratio",
        valueType: "number",
        allowedOps: numericOps,
      },
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
    valueTypes: ["number", "boolean", "string"],
  },
  {
    value: "ne",
    label: "!=",
    targets: ["value", "metric"],
    valueTypes: ["number", "boolean", "string"],
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
    targets: ["value", "metric"],
    valueTypes: ["number"],
  },
  {
    value: "crosses_below",
    label: "下穿",
    targets: ["value", "metric"],
    valueTypes: ["number"],
  },
  {
    value: "is_null",
    label: "为空",
    targets: ["value"],
    valueTypes: ["number", "boolean", "string", "date"],
  },
]

export const strategySteps: Array<{ id: Step; label: string }> = [
  { id: "indicators", label: "策略选股" },
  { id: "weights", label: "权重配置" },
  { id: "preview", label: "股池预览" },
  { id: "simulation", label: "模拟建仓" },
  { id: "backtest", label: "策略回测" },
]
