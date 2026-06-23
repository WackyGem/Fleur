import { indicatorCatalog, operatorOptions } from "@/features/strategy/catalog"
import type {
  ComparableIndicator,
  CompareTarget,
  ConditionOperator,
  IndicatorCatalog,
  MetricValueType,
  MetricOption,
  ScaledWeightIndicator,
  StrategyCondition,
  WeightIndicator,
} from "@/features/strategy/types"

type CompatibleMetric = {
  catalogId: string
  metricId: string
}

const defaultOperatorPreference: ConditionOperator[] = [
  "gte",
  "eq",
  "gt",
  "is_null",
]
const trendMovingAverageMetricPattern =
  /^(?:price_ma_\d+|price_avg_ma_\d+(?:_\d+)+|price_ema\d+_\d+)$/

export function createId(prefix: string) {
  if (globalThis.crypto?.randomUUID) {
    return `${prefix}-${globalThis.crypto.randomUUID()}`
  }

  return `${prefix}-${Date.now()}-${Math.round(Math.random() * 10000)}`
}

export function createComparableIndicator(
  catalogOptions: IndicatorCatalog[] = indicatorCatalog
): ComparableIndicator {
  const catalogs = getCatalogOptions(catalogOptions)
  const firstCatalog = catalogs[0]
  const firstMetric =
    firstCatalog.metrics.find((metric) => metric.allowedOps.length > 0) ??
    firstCatalog.metrics[0]
  const defaultOperator = getDefaultOperator(firstMetric)
  const defaultTarget = getCompatibleCompareTarget(
    "value",
    defaultOperator,
    firstMetric.valueType
  )
  const compareMetric = findCompatibleMetric(
    firstMetric.valueType,
    undefined,
    undefined,
    catalogs
  )

  return {
    catalogId: firstCatalog.id,
    metric: firstMetric.id,
    target: defaultTarget,
    operator: defaultOperator,
    value: getDefaultValue(firstMetric.valueType),
    valueEnd: "",
    compareCatalogId: compareMetric.catalogId,
    compareMetric: compareMetric.metricId,
  }
}

export function createCondition(
  catalogOptions: IndicatorCatalog[] = indicatorCatalog
): StrategyCondition {
  return {
    id: createId("condition"),
    logic: "and",
    ...createComparableIndicator(catalogOptions),
  }
}

export function createWeightIndicator(
  catalogOptions: IndicatorCatalog[] = indicatorCatalog
): WeightIndicator {
  return {
    id: createId("weight"),
    ...createComparableIndicator(catalogOptions),
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

  if (indicator.operator === "is_null") {
    return `${indicator.metric} ${operatorLabel}`
  }

  if (indicator.target === "metric") {
    return `${indicator.metric} ${operatorLabel} ${indicator.compareMetric}`
  }

  if (indicator.operator === "between") {
    return `${indicator.metric} ${operatorLabel} ${indicator.value} - ${indicator.valueEnd}`
  }

  return `${indicator.metric} ${operatorLabel} ${indicator.value}`
}

export function getCatalog(
  catalogId: string,
  catalogOptions: IndicatorCatalog[] = indicatorCatalog
) {
  const catalogs = getCatalogOptions(catalogOptions)
  return catalogs.find((catalog) => catalog.id === catalogId) ?? catalogs[0]
}

export function getMetric(
  catalogId: string,
  metricId: string,
  catalogOptions: IndicatorCatalog[] = indicatorCatalog
) {
  const catalog = getCatalog(catalogId, catalogOptions)
  return (
    catalog.metrics.find((metric) => metric.id === metricId) ??
    catalog.metrics[0]
  )
}

export function findCompatibleMetric(
  valueType: MetricValueType,
  preferredCatalogId?: string,
  preferredMetricId?: string,
  catalogOptions: IndicatorCatalog[] = indicatorCatalog,
  options: { requireCrossing?: boolean } = {}
): CompatibleMetric {
  const catalogs = getCatalogOptions(catalogOptions)
  const preferredCatalog = preferredCatalogId
    ? getCatalog(preferredCatalogId, catalogs)
    : undefined
  const preferredMetric = preferredCatalog?.metrics.find(
    (metric) =>
      metric.id === preferredMetricId &&
      metric.valueType === valueType &&
      (!options.requireCrossing || metric.supportsCrossing)
  )

  if (preferredCatalog && preferredMetric) {
    return {
      catalogId: preferredCatalog.id,
      metricId: preferredMetric.id,
    }
  }

  for (const catalog of catalogs) {
    const metric = catalog.metrics.find(
      (candidate) =>
        candidate.valueType === valueType &&
        (!options.requireCrossing || candidate.supportsCrossing)
    )
    if (metric) {
      return {
        catalogId: catalog.id,
        metricId: metric.id,
      }
    }
  }

  return {
    catalogId: catalogs[0].id,
    metricId: catalogs[0].metrics[0].id,
  }
}

export function getOperatorOptions(
  target: CompareTarget,
  valueType: MetricValueType,
  allowedOps?: ConditionOperator[]
) {
  return operatorOptions.filter(
    (option) =>
      option.targets.includes(target) &&
      option.valueTypes.includes(valueType) &&
      (!allowedOps || allowedOps.includes(option.value))
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
  valueType: MetricValueType,
  allowedOps?: ConditionOperator[]
) {
  const options = getOperatorOptions(target, valueType, allowedOps)
  return options.some((option) => option.value === operator)
    ? operator
    : (options[0]?.value ?? "eq")
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
  valueType: MetricValueType,
  catalogOptions: IndicatorCatalog[] = indicatorCatalog,
  options: { requireCrossing?: boolean } = {}
) {
  return getCatalog(catalogId, catalogOptions).metrics.filter(
    (metric) =>
      metric.valueType === valueType &&
      (!options.requireCrossing || metric.supportsCrossing)
  )
}

export function getTrendMovingAverageCatalogs(
  catalogOptions: IndicatorCatalog[] = indicatorCatalog
): IndicatorCatalog[] {
  return getCatalogOptions(catalogOptions).flatMap((catalog) => {
    if (!isTrendCatalog(catalog)) {
      return []
    }

    const metrics = catalog.metrics.filter(
      (metric) =>
        metric.valueType === "number" &&
        trendMovingAverageMetricPattern.test(metric.id)
    )

    return metrics.length > 0 ? [{ ...catalog, metrics }] : []
  })
}

export function getCompatibleValue(value: string, valueType: MetricValueType) {
  if (valueType === "boolean") {
    return value === "true" || value === "false" ? value : "false"
  }

  if (valueType === "number") {
    return value === "true" || value === "false" ? "0" : value
  }

  return value
}

export function getComparableMetricPatch(
  indicator: ComparableIndicator,
  catalogId: string,
  metricId: string,
  catalogOptions: IndicatorCatalog[] = indicatorCatalog
): Partial<ComparableIndicator> {
  const nextMetric = getMetric(catalogId, metricId, catalogOptions)
  const nextOperator = getCompatibleOperator(
    indicator.operator,
    indicator.target,
    nextMetric.valueType,
    nextMetric.allowedOps
  )
  const nextTarget = getCompatibleCompareTarget(
    indicator.target,
    nextOperator,
    nextMetric.valueType
  )
  const compatibleCompare = findCompatibleMetric(
    nextMetric.valueType,
    indicator.compareCatalogId,
    indicator.compareMetric,
    catalogOptions,
    { requireCrossing: isCrossingOperator(nextOperator) }
  )

  return {
    catalogId,
    metric: metricId,
    target: nextTarget,
    operator: getCompatibleOperator(
      nextOperator,
      nextTarget,
      nextMetric.valueType,
      nextMetric.allowedOps
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

function getCatalogOptions(catalogOptions: IndicatorCatalog[]) {
  return catalogOptions.length > 0 ? catalogOptions : indicatorCatalog
}

function isTrendCatalog(catalog: IndicatorCatalog) {
  return (
    catalog.id === "trend" ||
    catalog.source === "mart_stock_trend_indicator_daily"
  )
}

function getDefaultOperator(metric: MetricOption): ConditionOperator {
  return (
    defaultOperatorPreference.find((operator) =>
      metric.allowedOps.includes(operator)
    ) ??
    metric.allowedOps[0] ??
    "eq"
  )
}

function getDefaultValue(valueType: MetricValueType) {
  if (valueType === "boolean") {
    return "false"
  }
  if (valueType === "string" || valueType === "date") {
    return ""
  }

  return "0"
}

function isCrossingOperator(operator: ConditionOperator) {
  return operator === "crosses_above" || operator === "crosses_below"
}
