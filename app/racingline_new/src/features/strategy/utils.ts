import { indicatorCatalog, operatorOptions } from "@/features/strategy/catalog"
import type {
  ComparableIndicator,
  CompareTarget,
  ConditionOperator,
  MetricValueType,
  ScaledWeightIndicator,
  StrategyCondition,
  WeightIndicator,
} from "@/features/strategy/types"

type CompatibleMetric = {
  catalogId: string
  metricId: string
}

export function createId(prefix: string) {
  if (globalThis.crypto?.randomUUID) {
    return `${prefix}-${globalThis.crypto.randomUUID()}`
  }

  return `${prefix}-${Date.now()}-${Math.round(Math.random() * 10000)}`
}

export function createComparableIndicator(): ComparableIndicator {
  const firstCatalog = indicatorCatalog[0]
  const compareCatalog = indicatorCatalog[1] ?? firstCatalog

  return {
    catalogId: firstCatalog.id,
    metric: (firstCatalog.metrics[2] ?? firstCatalog.metrics[0]).id,
    target: "value",
    operator: "gte",
    value: "0",
    valueEnd: "",
    compareCatalogId: compareCatalog.id,
    compareMetric: compareCatalog.metrics[0].id,
  }
}

export function createCondition(): StrategyCondition {
  return {
    id: createId("condition"),
    logic: "and",
    ...createComparableIndicator(),
  }
}

export function createWeightIndicator(): WeightIndicator {
  return {
    id: createId("weight"),
    ...createComparableIndicator(),
    score: 50,
  }
}

export function clampScore(score: number) {
  if (Number.isNaN(score)) {
    return 0
  }

  return Math.min(100, Math.max(0, Math.round(score)))
}

export function clampWeightTotal(score: number) {
  if (Number.isNaN(score)) {
    return 0
  }

  return Math.min(100, Math.max(0, score))
}

export function formatComparableIndicator(indicator: ComparableIndicator) {
  const operatorLabel = getOperatorLabel(indicator.operator)

  if (indicator.target === "metric") {
    return `${indicator.metric} ${operatorLabel} ${indicator.compareMetric}`
  }

  if (indicator.operator === "between") {
    return `${indicator.metric} ${operatorLabel} ${indicator.value} - ${indicator.valueEnd}`
  }

  return `${indicator.metric} ${operatorLabel} ${indicator.value}`
}

export function getCatalog(catalogId: string) {
  return (
    indicatorCatalog.find((catalog) => catalog.id === catalogId) ??
    indicatorCatalog[0]
  )
}

export function getMetric(catalogId: string, metricId: string) {
  const catalog = getCatalog(catalogId)
  return (
    catalog.metrics.find((metric) => metric.id === metricId) ??
    catalog.metrics[0]
  )
}

export function findCompatibleMetric(
  valueType: MetricValueType,
  preferredCatalogId?: string,
  preferredMetricId?: string
): CompatibleMetric {
  const preferredCatalog = preferredCatalogId
    ? getCatalog(preferredCatalogId)
    : undefined
  const preferredMetric = preferredCatalog?.metrics.find(
    (metric) =>
      metric.id === preferredMetricId && metric.valueType === valueType
  )

  if (preferredCatalog && preferredMetric) {
    return {
      catalogId: preferredCatalog.id,
      metricId: preferredMetric.id,
    }
  }

  for (const catalog of indicatorCatalog) {
    const metric = catalog.metrics.find(
      (candidate) => candidate.valueType === valueType
    )
    if (metric) {
      return {
        catalogId: catalog.id,
        metricId: metric.id,
      }
    }
  }

  return {
    catalogId: indicatorCatalog[0].id,
    metricId: indicatorCatalog[0].metrics[0].id,
  }
}

export function getOperatorOptions(
  target: CompareTarget,
  valueType: MetricValueType
) {
  return operatorOptions.filter(
    (option) =>
      option.targets.includes(target) && option.valueTypes.includes(valueType)
  )
}

export function getOperatorLabel(operator: ConditionOperator) {
  return (
    operatorOptions.find((option) => option.value === operator)?.label ??
    operator
  )
}

export function getCompatibleOperator(
  operator: ConditionOperator,
  target: CompareTarget,
  valueType: MetricValueType
) {
  const options = getOperatorOptions(target, valueType)
  return options.some((option) => option.value === operator)
    ? operator
    : options[0].value
}

export function getCompatibleCompareTarget(
  target: CompareTarget,
  operator: ConditionOperator,
  valueType: MetricValueType
) {
  const option = operatorOptions.find(
    (candidate) => candidate.value === operator
  )
  return option?.targets.includes(target) &&
    option.valueTypes.includes(valueType)
    ? target
    : "value"
}

export function getCatalogMetricsByType(
  catalogId: string,
  valueType: MetricValueType
) {
  return getCatalog(catalogId).metrics.filter(
    (metric) => metric.valueType === valueType
  )
}

export function getCompatibleValue(value: string, valueType: MetricValueType) {
  if (valueType === "boolean") {
    return value === "true" || value === "false" ? value : "false"
  }

  return value === "true" || value === "false" ? "0" : value
}

export function getComparableMetricPatch(
  indicator: ComparableIndicator,
  catalogId: string,
  metricId: string
): Partial<ComparableIndicator> {
  const nextMetric = getMetric(catalogId, metricId)
  const nextTarget = getCompatibleCompareTarget(
    indicator.target,
    indicator.operator,
    nextMetric.valueType
  )
  const compatibleCompare = findCompatibleMetric(
    nextMetric.valueType,
    indicator.compareCatalogId,
    indicator.compareMetric
  )

  return {
    catalogId,
    metric: metricId,
    target: nextTarget,
    operator: getCompatibleOperator(
      indicator.operator,
      nextTarget,
      nextMetric.valueType
    ),
    value: getCompatibleValue(indicator.value, nextMetric.valueType),
    compareCatalogId: compatibleCompare.catalogId,
    compareMetric: compatibleCompare.metricId,
  }
}

export function getScaledWeightIndicators(weightIndicators: WeightIndicator[]) {
  const clampedWeightIndicators = weightIndicators.map((indicator) => ({
    ...indicator,
    clampedScore: clampScore(indicator.score),
  }))
  const rawTotal = clampedWeightIndicators.reduce(
    (total, indicator) => total + indicator.clampedScore,
    0
  )
  const scaledTotal = clampWeightTotal(rawTotal)
  const scaleRatio = rawTotal > 0 ? scaledTotal / rawTotal : 0
  const indicators: ScaledWeightIndicator[] = clampedWeightIndicators.map(
    (indicator) => {
      const scaledScore = indicator.clampedScore * scaleRatio
      return {
        ...indicator,
        scaledScore,
        ratio: scaledTotal > 0 ? (scaledScore / scaledTotal) * 100 : 0,
      }
    }
  )

  return {
    indicators,
    rawTotal,
    scaledTotal,
    scaleRatio,
  }
}
