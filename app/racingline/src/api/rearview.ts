import {
  buildPath,
  jsonBody,
  requestJson,
  type QueryParams,
} from "@/api/client"
import type {
  MarketFeeTemplateRecord,
  ExplainResponse,
  ListResult,
  MetricDefinition,
  MetricsQuery,
  PreviewChartContextRequest,
  PreviewChartContextResponse,
  RuleVersionSpec,
  SecurityAnalysisRequest,
  SecurityAnalysisResponse,
  StrategyBacktestClosedTradeRecord,
  StrategyBacktestCreateRequest,
  StrategyBacktestDraftResponse,
  StrategyBacktestEventRecord,
  StrategyBacktestNavPoint,
  StrategyBacktestOrderRecord,
  StrategyBacktestOptionsResponse,
  StrategyBacktestOverviewUiResponse,
  StrategyBacktestPerformanceUiView,
  StrategyBacktestPerformanceView,
  StrategyBacktestPositionRecord,
  StrategyBacktestRebalanceRecordsUiResponse,
  StrategyBacktestRebalanceRecordsResponse,
  StrategyBacktestRunRecord,
  StrategyBacktestRunStatusView,
  StrategyBacktestTargetRecord,
  StrategyBacktestTradeMetricRecord,
  StrategyBacktestTradeRecord,
  StrategyBacktestValidateRequest,
  StrategyPortfolioCreateRequest,
  StrategyPortfolioDashboardResponse,
  StrategyPortfolioListResult,
  StrategyPortfolioNavResponse,
  StrategyPortfolioPerformanceView,
  StrategyPortfolioPublishPreviewResponse,
  StrategyPortfolioRecord,
  StrategyPortfolioRebalanceRecordsResponse,
  StrategyPortfolioSignalsResponse,
  StrategyPortfolioSignalTimelineResponse,
  StrategyPreviewPoolPageRequest,
  StrategyPreviewPoolPageResponse,
  StrategyPreviewOpenRequest,
  StrategyPreviewOpenResponse,
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

export function openStrategyPreview(request: StrategyPreviewOpenRequest) {
  return requestJson<StrategyPreviewOpenResponse>(
    "/rearview/strategy-preview/open",
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

export function previewChartContext(
  request: PreviewChartContextRequest,
  signal?: AbortSignal
) {
  return requestJson<PreviewChartContextResponse>(
    "/rearview/strategy-preview/chart-context",
    { ...jsonBody(request), signal }
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

export function getStrategyBacktestStatus(strategyBacktestRunId: string) {
  return requestJson<StrategyBacktestRunStatusView>(
    `/rearview/strategy-backtests/${strategyBacktestRunId}/status`
  )
}

export function getStrategyBacktestOverviewUi(
  strategyBacktestRunId: string,
  tradeDate?: string | null
) {
  return requestJson<StrategyBacktestOverviewUiResponse>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/overview`,
      {
        trade_date: tradeDate ?? undefined,
        view: "ui",
      }
    )
  )
}

export function listStrategyBacktestNav(strategyBacktestRunId: string) {
  return requestJson<StrategyBacktestNavPoint[]>(
    `/rearview/strategy-backtests/${strategyBacktestRunId}/nav`
  )
}

export function listStrategyBacktestNavUi(strategyBacktestRunId: string) {
  return requestJson<StrategyBacktestNavPoint[]>(
    buildPath(`/rearview/strategy-backtests/${strategyBacktestRunId}/nav`, {
      view: "ui",
    })
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

export function listStrategyBacktestRebalanceRecordsUi(
  strategyBacktestRunId: string,
  tradeDate?: string | null
) {
  return requestJson<StrategyBacktestRebalanceRecordsUiResponse>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/rebalance-records`,
      { trade_date: tradeDate ?? undefined, view: "ui" }
    )
  )
}

export function listStrategyBacktestTargets(
  strategyBacktestRunId: string,
  query: QueryParams = {}
) {
  return requestJson<ListResult<StrategyBacktestTargetRecord>>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/targets`,
      query
    )
  )
}

export function listStrategyBacktestOrders(
  strategyBacktestRunId: string,
  query: QueryParams = {}
) {
  return requestJson<ListResult<StrategyBacktestOrderRecord>>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/orders`,
      query
    )
  )
}

export function listStrategyBacktestTrades(
  strategyBacktestRunId: string,
  query: QueryParams = {}
) {
  return requestJson<ListResult<StrategyBacktestTradeRecord>>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/trades`,
      query
    )
  )
}

export function listStrategyBacktestPositions(
  strategyBacktestRunId: string,
  query: QueryParams = {}
) {
  return requestJson<ListResult<StrategyBacktestPositionRecord>>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/positions`,
      query
    )
  )
}

export function listStrategyBacktestEvents(
  strategyBacktestRunId: string,
  query: QueryParams = {}
) {
  return requestJson<ListResult<StrategyBacktestEventRecord>>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/events`,
      query
    )
  )
}

export function getStrategyBacktestPerformance(strategyBacktestRunId: string) {
  return requestJson<StrategyBacktestPerformanceView>(
    `/rearview/strategy-backtests/${strategyBacktestRunId}/performance`
  )
}

export function getStrategyBacktestPerformanceUi(
  strategyBacktestRunId: string
) {
  return requestJson<StrategyBacktestPerformanceUiView>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/performance`,
      { view: "ui" }
    )
  )
}

export function listStrategyBacktestClosedTrades(
  strategyBacktestRunId: string,
  query: QueryParams = {}
) {
  return requestJson<ListResult<StrategyBacktestClosedTradeRecord>>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/closed-trades`,
      query
    )
  )
}

export function listStrategyBacktestTradeMetrics(
  strategyBacktestRunId: string,
  query: QueryParams = {}
) {
  return requestJson<ListResult<StrategyBacktestTradeMetricRecord>>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/trade-metrics`,
      query
    )
  )
}

export function createStrategyPortfolio(
  request: StrategyPortfolioCreateRequest
) {
  return requestJson<StrategyPortfolioRecord>(
    "/rearview/strategy-portfolios",
    jsonBody(request)
  )
}

export function getStrategyPortfolioPublishPreview(
  strategyBacktestRunId: string,
  sourceResultAttemptId: string
) {
  return requestJson<StrategyPortfolioPublishPreviewResponse>(
    buildPath(
      `/rearview/strategy-backtests/${strategyBacktestRunId}/portfolio-publish-preview`,
      { source_result_attempt_id: sourceResultAttemptId }
    )
  )
}

export function getStrategyPortfolioDashboard() {
  return requestJson<StrategyPortfolioDashboardResponse>(
    "/rearview/strategy-portfolios/dashboard"
  )
}

export function getStrategyPortfolio(strategyPortfolioId: string) {
  return requestJson<StrategyPortfolioRecord>(
    `/rearview/strategy-portfolios/${strategyPortfolioId}`
  )
}

export function listStrategyPortfolioNav(strategyPortfolioId: string) {
  return requestJson<StrategyPortfolioNavResponse>(
    `/rearview/strategy-portfolios/${strategyPortfolioId}/nav`
  )
}

export function getStrategyPortfolioPerformance(strategyPortfolioId: string) {
  return requestJson<StrategyPortfolioPerformanceView>(
    `/rearview/strategy-portfolios/${strategyPortfolioId}/performance`
  )
}

export function listStrategyPortfolioSignals(
  strategyPortfolioId: string,
  query: QueryParams = {}
) {
  return requestJson<StrategyPortfolioSignalsResponse>(
    buildPath(
      `/rearview/strategy-portfolios/${strategyPortfolioId}/signals`,
      query
    )
  )
}

export function listStrategyPortfolioSignalTimeline(
  strategyPortfolioId: string
) {
  return requestJson<StrategyPortfolioSignalTimelineResponse>(
    `/rearview/strategy-portfolios/${strategyPortfolioId}/signal-timeline`
  )
}

export function listStrategyPortfolioPositions(
  strategyPortfolioId: string,
  query: QueryParams = {}
) {
  return requestJson<
    StrategyPortfolioListResult<StrategyBacktestPositionRecord>
  >(
    buildPath(
      `/rearview/strategy-portfolios/${strategyPortfolioId}/positions`,
      query
    )
  )
}

export function listStrategyPortfolioRebalanceRecords(
  strategyPortfolioId: string,
  tradeDate?: string | null
) {
  return requestJson<StrategyPortfolioRebalanceRecordsResponse>(
    buildPath(
      `/rearview/strategy-portfolios/${strategyPortfolioId}/rebalance-records`,
      { trade_date: tradeDate ?? undefined }
    )
  )
}

export function securityAnalysis(
  request: SecurityAnalysisRequest,
  signal?: AbortSignal
) {
  return requestJson<SecurityAnalysisResponse>("/rearview/security-analysis", {
    ...jsonBody(request),
    signal,
  })
}
