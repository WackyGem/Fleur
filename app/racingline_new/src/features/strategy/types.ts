import type { MetricDefinition, Operator } from "@/types/rearview"

export type IndicatorCatalog = {
  id: string
  label: string
  source: string
  metrics: MetricOption[]
}

export type MetricValueType = "number" | "boolean" | "string" | "date"

export type MetricOption = {
  allowedOps: ConditionOperator[]
  defaultOutput?: boolean
  description?: string | null
  id: string
  label: string
  previousMetric?: string
  sourceMetric?: MetricDefinition
  supportsCrossing?: boolean
  valueType: MetricValueType
}

export type GroupLogic = "and" | "or"

export type CompareTarget = "value" | "metric"

export type ConditionOperator = Operator

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
    commissionRateMaxPercent: number
    minCommission: number
    buySlippageRatePercent: number
    sellSlippageRatePercent: number
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
