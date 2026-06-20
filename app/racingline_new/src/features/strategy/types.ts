export type IndicatorCatalog = {
  id: string
  label: string
  source: string
  metrics: MetricOption[]
}

export type MetricValueType = "number" | "boolean"

export type MetricOption = {
  id: string
  valueType: MetricValueType
}

export type GroupLogic = "and" | "or"

export type CompareTarget = "value" | "metric"

export type ConditionOperator =
  | "gt"
  | "gte"
  | "lt"
  | "lte"
  | "eq"
  | "neq"
  | "between"
  | "crosses_above"
  | "crosses_below"

export type ComparableIndicator = {
  catalogId: string
  metric: string
  target: CompareTarget
  operator: ConditionOperator
  value: string
  valueEnd: string
  compareCatalogId: string
  compareMetric: string
}

export type StrategyCondition = ComparableIndicator & {
  id: string
  logic: GroupLogic
}

export type StrategyConditionGroup = {
  id: string
  name: string
  conditions: StrategyCondition[]
}

export type WeightIndicator = ComparableIndicator & {
  id: string
  score: number
}

export type ScaledWeightIndicator = WeightIndicator & {
  clampedScore: number
  ratio: number
  scaledScore: number
}

export type SimulationSettings = {
  initialCapital: number
  buyTopN: number
  singlePositionLimitPercent: number
  transactionFees: {
    commissionRatePercent: number
    slippageRatePercent: number
    stampDutyRatePercent: number
    transferFeeRatePercent: number
  }
  fixedStopLoss: {
    enabled: boolean
    lossPercent: number
  }
  indicatorStopLoss: {
    enabled: boolean
    catalogId: string
    metric: string
  }
  takeProfit: {
    enabled: boolean
    profitPercent: number
  }
  timeStopLoss: {
    enabled: boolean
    holdingDays: number
    minimumReturnPercent: number
  }
}

export type Step =
  | "indicators"
  | "weights"
  | "preview"
  | "simulation"
  | "backtest"
