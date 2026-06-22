import { formatComparableIndicator } from "@/features/strategy/utils"
import type { WeightIndicator } from "@/features/strategy/types"
import type {
  JsonValue,
  MetricDefinition,
  RuleVersionSpec,
  StrategyPreviewResponse,
  StrategyPreviewSignal,
} from "@/types/rearview"

export type PreviewRange = {
  endDate: string
  previewRowLimit: number
  startDate: string
}

export type PreviewSnapshot = {
  appliedRuleSpec: RuleVersionSpec
  createdAt: string
  labels: {
    metrics: Record<string, string>
    scoringRules: Record<string, string>
  }
  previewId: string
  range: PreviewRange
  result: StrategyPreviewResponse
  stale: boolean
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

export type PreviewStockRow = {
  code: string
  exchangeCode?: string | null
  name: string
  rank: number
  rawValueRows: PreviewValueRow[]
  score: number
  scoreItems: PreviewScoreItem[]
  selectedMetricRows: PreviewValueRow[]
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
  createdAt,
  metrics,
  range,
  result,
  weightIndicators,
}: {
  appliedRuleSpec: RuleVersionSpec
  createdAt: string
  metrics: MetricDefinition[]
  range: PreviewRange
  result: StrategyPreviewResponse
  weightIndicators: WeightIndicator[]
}): PreviewSnapshot {
  return {
    appliedRuleSpec,
    createdAt,
    labels: {
      metrics: buildMetricLabels(metrics),
      scoringRules: buildScoringRuleLabels(weightIndicators),
    },
    previewId: result.preview_id,
    range,
    result,
    stale: false,
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

export function buildPreviewPresentation(
  snapshot: PreviewSnapshot
): PreviewPresentation {
  return {
    tradeDates: snapshot.result.trade_dates.map((tradeDate) => {
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
        poolCount: tradeDate.pool_count,
        stocks,
      }
    }),
  }
}

export function buildPreviewStockRow(
  signal: StrategyPreviewSignal,
  labels: PreviewSnapshot["labels"]
): PreviewStockRow {
  return {
    code: signal.security_code,
    exchangeCode: signal.exchange_code,
    name: signal.security_name?.trim() || signal.security_code,
    rank: signal.signal_rank,
    rawValueRows: buildValueRows(signal.raw_values, labels.metrics),
    score: signal.score,
    scoreItems: buildScoreItems(signal.score_breakdown, labels.scoringRules),
    selectedMetricRows: buildValueRows(signal.selected_metrics, labels.metrics),
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
      formatComparableIndicator(indicator),
    ])
  )
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

function isJsonRecord(value: JsonValue): value is Record<string, JsonValue> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}
