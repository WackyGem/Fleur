import { formatWeightIndicator } from "@/features/strategy/utils"
import type { ConditionFilterPath } from "@/features/strategy/adapters"
import type {
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import type {
  JsonValue,
  MetricDefinition,
  RuleVersionSpec,
  StrategyPreviewResponse,
  StrategyPreviewSignal,
  StrategyPreviewTimelineResponse,
} from "@/types/rearview"

export type PreviewRange = {
  endDate: string
  previewRowLimit: number
  selectedTradeDate?: string | null
  startDate: string
}

export type PreviewSnapshot = {
  appliedRuleSpec: RuleVersionSpec
  createdAt: string
  labels: {
    filterMetrics: PreviewFilterMetric[]
    metrics: Record<string, string>
    scoringRules: Record<string, string>
  }
  previewId: string
  range: PreviewRange
  result: StrategyPreviewResponse
  stale: boolean
  timeline: StrategyPreviewTimelineResponse | null
}

export type PreviewValueRow = {
  id: string
  label: string
  value: string
}

export type PreviewScoreItem = {
  id: string
  label: string
  score: number
}

export type PreviewFilterMetric = {
  conditionId: string
  groupId: string
  label: string
  metric: string
  path: string
}

export type PreviewStockRow = {
  board?: string | null
  boardLabel: string
  code: string
  exchangeCode?: string | null
  name: string
  rank: number
  filterMetricRows: PreviewValueRow[]
  rawValueRows: PreviewValueRow[]
  score: number
  scoreItems: PreviewScoreItem[]
}

export type PreviewTradeDateRow = {
  averageScore: number
  date: string
  poolCount: number
  stocks: PreviewStockRow[]
}

export type PreviewPresentation = {
  tradeDates: PreviewTradeDateRow[]
}

export function buildPreviewSnapshot({
  appliedRuleSpec,
  conditionGroups,
  conditionPaths,
  createdAt,
  metrics,
  range,
  result,
  timeline,
  weightIndicators,
}: {
  appliedRuleSpec: RuleVersionSpec
  conditionGroups: StrategyConditionGroup[]
  conditionPaths: ConditionFilterPath[]
  createdAt: string
  metrics: MetricDefinition[]
  range: PreviewRange
  result: StrategyPreviewResponse
  timeline?: StrategyPreviewTimelineResponse | null
  weightIndicators: WeightIndicator[]
}): PreviewSnapshot {
  return {
    appliedRuleSpec,
    createdAt,
    labels: {
      filterMetrics: buildFilterMetricRows(conditionGroups, conditionPaths, metrics),
      metrics: buildMetricLabels(metrics),
      scoringRules: buildScoringRuleLabels(weightIndicators),
    },
    previewId: result.preview_id,
    range,
    result,
    stale: false,
    timeline: timeline ?? null,
  }
}

export function markPreviewSnapshotStale(
  snapshot: PreviewSnapshot | null
): PreviewSnapshot | null {
  if (!snapshot || snapshot.stale) {
    return snapshot
  }

  return {
    ...snapshot,
    stale: true,
  }
}

export function buildPreviewTimelineRange(
  now = new Date(),
  previewRowLimit = 10
): PreviewRange {
  const end = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  const start = new Date(end)
  start.setFullYear(start.getFullYear() - 1)

  return {
    endDate: formatDate(startOfDay(end)),
    previewRowLimit,
    startDate: formatDate(startOfDay(start)),
  }
}

export function buildPreviewPresentation(
  snapshot: PreviewSnapshot
): PreviewPresentation {
  const signalsByDate = new Map(
    snapshot.result.trade_dates.map((tradeDate) => [
      tradeDate.trade_date,
      tradeDate.signals,
    ])
  )
  const poolCountsByDate = new Map(
    snapshot.result.trade_dates.map((tradeDate) => [
      tradeDate.trade_date,
      tradeDate.pool_count,
    ])
  )
  const tradeDates = (
    snapshot.timeline?.trade_dates.map((tradeDate) => ({
      pool_count: tradeDate.pool_count,
      signals: signalsByDate.get(tradeDate.trade_date) ?? [],
      trade_date: tradeDate.trade_date,
    })) ?? snapshot.result.trade_dates
  ).filter((tradeDate) =>
    isWithinPreviewRange(tradeDate.trade_date, snapshot.range)
  )

  return {
    tradeDates: tradeDates.map((tradeDate) => {
      const stocks = tradeDate.signals.map((signal) =>
        buildPreviewStockRow(signal, snapshot.labels)
      )
      const averageScore =
        stocks.length > 0
          ? stocks.reduce((total, stock) => total + stock.score, 0) /
            stocks.length
          : 0

      return {
        averageScore,
        date: tradeDate.trade_date,
        poolCount:
          tradeDate.pool_count ?? poolCountsByDate.get(tradeDate.trade_date) ?? 0,
        stocks,
      }
    }),
  }
}

function isWithinPreviewRange(tradeDate: string, range: PreviewRange) {
  return tradeDate >= range.startDate && tradeDate <= range.endDate
}

export function buildPreviewStockRow(
  signal: StrategyPreviewSignal,
  labels: PreviewSnapshot["labels"]
): PreviewStockRow {
  return {
    board: signal.security_board,
    boardLabel: formatSecurityBoard(signal.security_board),
    code: signal.security_code,
    exchangeCode: signal.exchange_code,
    name: signal.security_name?.trim() || signal.security_code,
    rank: signal.signal_rank,
    filterMetricRows: buildFilterValueRows(
      signal.selected_metrics,
      signal.raw_values,
      labels.filterMetrics
    ),
    rawValueRows: buildValueRows(signal.raw_values, labels.metrics),
    score: signal.score,
    scoreItems: buildScoreItems(signal.score_breakdown, labels.scoringRules),
  }
}

export function formatSecurityBoard(board: string | null | undefined): string {
  switch (board) {
    case "sse_main_board":
      return "沪市主板"
    case "szse_main_board":
      return "深市主板"
    case "chinext":
      return "创业板"
    case "star_market":
      return "科创板"
    default:
      return "-"
  }
}

function buildMetricLabels(metrics: MetricDefinition[]) {
  return Object.fromEntries(
    metrics.map((metric) => [
      metric.logical_metric,
      metric.display?.label_zh?.trim() || metric.logical_metric,
    ])
  )
}

function buildScoringRuleLabels(weightIndicators: WeightIndicator[]) {
  return Object.fromEntries(
    weightIndicators.map((indicator, index) => [
      `weight:${indicator.id}:${index + 1}`,
      formatWeightIndicator(indicator),
    ])
  )
}

function buildFilterMetricRows(
  conditionGroups: StrategyConditionGroup[],
  conditionPaths: ConditionFilterPath[],
  metrics: MetricDefinition[]
): PreviewFilterMetric[] {
  const metricLabels = buildMetricLabels(metrics)
  const metricRows: PreviewFilterMetric[] = []

  for (const conditionPath of conditionPaths) {
    const group = conditionGroups.find((item) => item.id === conditionPath.groupId)
    const condition = group?.conditions.find(
      (item) => item.id === conditionPath.conditionId
    )

    if (!condition) {
      continue
    }

    metricRows.push({
      conditionId: conditionPath.conditionId,
      groupId: conditionPath.groupId,
      label: metricLabels[condition.metric] ?? condition.metric,
      metric: condition.metric,
      path: conditionPath.path,
    })
  }

  return metricRows
}

function buildScoreItems(
  scoreBreakdown: JsonValue,
  scoringLabels: Record<string, string>
): PreviewScoreItem[] {
  if (!isJsonRecord(scoreBreakdown)) {
    return []
  }

  return Object.entries(scoreBreakdown)
    .map(([key, value]) => {
      if (typeof value !== "number") {
        return null
      }

      return {
        id: key,
        label: scoringLabels[key] ?? key,
        score: value,
      }
    })
    .filter((item): item is PreviewScoreItem => item !== null)
}

function buildFilterValueRows(
  selectedMetrics: JsonValue,
  rawValues: JsonValue,
  filterMetrics: PreviewFilterMetric[]
): PreviewValueRow[] {
  const selectedRecord = isJsonRecord(selectedMetrics) ? selectedMetrics : {}
  const rawRecord = isJsonRecord(rawValues) ? rawValues : {}

  return filterMetrics.map((filterMetric) => {
    const value =
      selectedRecord[filterMetric.metric] ?? rawRecord[filterMetric.metric] ?? null

    return {
      id: filterMetric.conditionId,
      label: filterMetric.label,
      value: formatJsonValue(value),
    }
  })
}

function buildValueRows(
  value: JsonValue,
  metricLabels: Record<string, string>
): PreviewValueRow[] {
  if (!isJsonRecord(value)) {
    return []
  }

  return Object.entries(value).map(([key, item]) => ({
    id: key,
    label: metricLabels[key] ?? key,
    value: formatJsonValue(item),
  }))
}

function formatJsonValue(value: JsonValue): string {
  if (value === null) {
    return "-"
  }

  if (typeof value === "number") {
    return Number.isInteger(value) ? String(value) : value.toFixed(2)
  }

  if (typeof value === "boolean") {
    return value ? "true" : "false"
  }

  if (typeof value === "string") {
    return value
  }

  return JSON.stringify(value)
}

function startOfDay(value: Date): Date {
  return new Date(value.getFullYear(), value.getMonth(), value.getDate())
}

function formatDate(value: Date): string {
  const year = value.getFullYear()
  const month = String(value.getMonth() + 1).padStart(2, "0")
  const day = String(value.getDate()).padStart(2, "0")

  return `${year}-${month}-${day}`
}

function isJsonRecord(value: JsonValue): value is Record<string, JsonValue> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}
