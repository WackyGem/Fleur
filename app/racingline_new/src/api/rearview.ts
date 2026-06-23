import { buildPath, jsonBody, requestJson } from "@/api/client"
import type {
  MarketFeeTemplateRecord,
  ExplainResponse,
  MetricDefinition,
  MetricsQuery,
  RuleVersionSpec,
  SecurityAnalysisRequest,
  SecurityAnalysisResponse,
  StrategyBacktestCreateRequest,
  StrategyBacktestDraftResponse,
  StrategyBacktestNavPoint,
  StrategyBacktestOptionsResponse,
  StrategyBacktestPerformanceView,
  StrategyBacktestRebalanceRecordsResponse,
  StrategyBacktestRunRecord,
  StrategyBacktestValidateRequest,
  StrategyPreviewPoolPageRequest,
  StrategyPreviewPoolPageResponse,
  StrategyPreviewRequest,
  StrategyPreviewResponse,
  StrategyPreviewTimelineRequest,
  StrategyPreviewTimelineResponse,
} from "@/types/rearview"

export async function listMetrics(
  query: MetricsQuery = {}
): Promise<MetricDefinition[]> {
  const value = await requestJson<
    MetricDefinition[] | { items: MetricDefinition[] }
  >(buildPath("/rearview/metrics", query))
  return Array.isArray(value) ? value : value.items
}

export function explainRule(
  rule: RuleVersionSpec,
  range?: { start_date?: string; end_date?: string; top_n?: number }
) {
  const body = range?.start_date && range.end_date ? { rule, ...range } : rule
  return requestJson<ExplainResponse>("/rearview/explain", jsonBody(body))
}

export function previewStrategy(request: StrategyPreviewRequest) {
  return requestJson<StrategyPreviewResponse>(
    "/rearview/strategy-preview",
    jsonBody(request)
  )
}

export function previewStrategyTimeline(
  request: StrategyPreviewTimelineRequest
) {
  return requestJson<StrategyPreviewTimelineResponse>(
    "/rearview/strategy-preview/timeline",
    jsonBody(request)
  )
}

export function previewStrategyPoolPage(
  request: StrategyPreviewPoolPageRequest
) {
  return requestJson<StrategyPreviewPoolPageResponse>(
    "/rearview/strategy-preview/pool-page",
    jsonBody(request)
  )
}

export function getDefaultMarketFeeTemplate(market = "CN_A_SHARE") {
  return requestJson<MarketFeeTemplateRecord>(
    buildPath("/rearview/market-fee-templates/default", { market })
  )
}

export function validateStrategyBacktest(
  request: StrategyBacktestValidateRequest
) {
  return requestJson<StrategyBacktestDraftResponse>(
    "/rearview/strategy-backtests/validate",
    jsonBody(request)
  )
}

export function getStrategyBacktestOptions(benchmarkSecurityCode: string) {
  return requestJson<StrategyBacktestOptionsResponse>(
    buildPath("/rearview/strategy-backtests/options", {
      benchmark_security_code: benchmarkSecurityCode,
    })
  )
}

export function createStrategyBacktest(request: StrategyBacktestCreateRequest) {
  return requestJson<StrategyBacktestRunRecord>(
    "/rearview/strategy-backtests",
    jsonBody(request)
  )
}

export function getStrategyBacktest(strategyBacktestRunId: string) {
  return requestJson<StrategyBacktestRunRecord>(
    `/rearview/strategy-backtests/${strategyBacktestRunId}`
  )
}

export function listStrategyBacktestNav(strategyBacktestRunId: string) {
  return requestJson<StrategyBacktestNavPoint[]>(
    `/rearview/strategy-backtests/${strategyBacktestRunId}/nav`
  )
}

export function listStrategyBacktestRebalanceRecords(
  strategyBacktestRunId: string,
  tradeDate?: string | null
) {
  return requestJson<StrategyBacktestRebalanceRecordsResponse>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/rebalance-records`,
      { trade_date: tradeDate ?? undefined }
    )
  )
}

export function getStrategyBacktestPerformance(strategyBacktestRunId: string) {
  return requestJson<StrategyBacktestPerformanceView>(
    `/rearview/strategy-backtests/${strategyBacktestRunId}/performance`
  )
}

export function securityAnalysis(
  request: SecurityAnalysisRequest,
  signal?: AbortSignal
) {
  return requestJson<SecurityAnalysisResponse>(
    "/rearview/security-analysis",
    { ...jsonBody(request), signal }
  )
}
