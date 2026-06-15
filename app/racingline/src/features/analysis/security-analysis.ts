import type {
  AnalysisSource,
  PriceAdjustment,
  QuoteMartRow,
  SecurityAnalysisQuery,
} from "@/types/rearview"

export const DEFAULT_PRICE_ADJUSTMENT: PriceAdjustment = "forward_adjusted"
export const DEFAULT_ANALYSIS_SOURCE: AnalysisSource = "signals"
export const DEFAULT_MA_WINDOWS = [5, 10, 30] as const

const analysisSources = ["signals", "pool"] as const
const priceAdjustments = [
  "forward_adjusted",
  "backward_adjusted",
  "unadjusted",
] as const

export function parseAnalysisSource(value: string | null) {
  return analysisSources.find((source) => source === value) ?? null
}

export function parsePriceAdjustment(value: string | null) {
  return (
    priceAdjustments.find((adjustment) => adjustment === value) ??
    DEFAULT_PRICE_ADJUSTMENT
  )
}

export function buildSecurityAnalysisPath({
  runId,
  securityCode,
  source,
  tradeDate,
  adjustment = DEFAULT_PRICE_ADJUSTMENT,
}: {
  runId: string
  securityCode: string
  source: AnalysisSource
  tradeDate: string
  adjustment?: PriceAdjustment
}) {
  const params = new URLSearchParams({
    adjustment,
    source,
    trade_date: tradeDate,
  })
  return `/runs/${runId}/securities/${securityCode}?${params.toString()}`
}

export function buildRunDetailPath({
  runId,
  source,
  tradeDate,
}: {
  runId: string
  source: AnalysisSource
  tradeDate: string
}) {
  const params = new URLSearchParams({
    source,
    trade_date: tradeDate,
  })
  return `/runs/${runId}?${params.toString()}`
}

export function buildSecurityAnalysisQuery({
  adjustment,
  source,
  tradeDate,
}: {
  adjustment: PriceAdjustment
  source: AnalysisSource | null
  tradeDate: string
}): SecurityAnalysisQuery | undefined {
  if (!source || !tradeDate) {
    return undefined
  }

  return {
    adjustment,
    source,
    trade_date: tradeDate,
  }
}

export function quoteForTradeDate(rows: QuoteMartRow[], tradeDate: string) {
  return rows.find((row) => row.trade_date === tradeDate) ?? null
}

export function nextMaWindows(
  currentWindows: number[],
  window: number,
  checked: boolean,
) {
  const next = new Set(currentWindows)
  if (checked) {
    next.add(window)
  } else {
    next.delete(window)
  }
  return DEFAULT_MA_WINDOWS.filter((candidate) => next.has(candidate))
}
