import { keepPreviousData, useMutation, useQuery } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import {
  createStrategyPortfolio,
  createStrategyBacktest,
  explainRule,
  getStrategyPortfolio,
  getStrategyPortfolioDashboard,
  getStrategyPortfolioPerformance,
  getStrategyBacktest,
  getStrategyBacktestOptions,
  getStrategyBacktestPerformance,
  getStrategyBacktestPerformanceUi,
  getStrategyBacktestStatus,
  getDefaultMarketFeeTemplate,
  listStrategyBacktestClosedTrades,
  listStrategyBacktestEvents,
  listStrategyBacktestNav,
  listStrategyBacktestNavUi,
  listStrategyBacktestOrders,
  listStrategyBacktestPositions,
  listStrategyBacktestRebalanceRecords,
  listStrategyBacktestRebalanceRecordsUi,
  listStrategyBacktestTargets,
  listStrategyBacktestTradeMetrics,
  listStrategyBacktestTrades,
  listStrategyPortfolioNav,
  listStrategyPortfolioPositions,
  listStrategyPortfolioRebalanceRecords,
  listStrategyPortfolioSignals,
  listStrategyPortfolioSignalTimeline,
  listMetrics,
  openStrategyPreview,
  previewChartContext,
  previewStrategy,
  previewStrategyPoolPage,
  securityAnalysis,
  previewStrategyTimeline,
  validateStrategyBacktest,
} from "@/api/rearview"
import type {
  MetricsQuery,
  PreviewChartContextRequest,
  RuleVersionSpec,
  SecurityAnalysisRequest,
  StrategyBacktestCreateRequest,
  StrategyBacktestValidateRequest,
  StrategyPortfolioCreateRequest,
  StrategyPreviewPoolPageRequest,
  StrategyPreviewOpenRequest,
  StrategyPreviewRequest,
  StrategyPreviewTimelineRequest,
} from "@/types/rearview"
import type { QueryParams } from "@/api/client"

export function useMetricsQuery(query: MetricsQuery = {}) {
  return useQuery({
    queryKey: queryKeys.metrics(query),
    queryFn: () => listMetrics(query),
    retry: 1,
  })
}

export function useExplainMutation() {
  return useMutation({
    mutationFn: ({
      rule,
      range,
    }: {
      rule: RuleVersionSpec
      range?: { start_date?: string; end_date?: string; top_n?: number }
    }) => explainRule(rule, range),
  })
}

export function useStrategyPreviewMutation() {
  return useMutation({
    mutationFn: (request: StrategyPreviewRequest) => previewStrategy(request),
  })
}

export function useStrategyPreviewTimelineMutation() {
  return useMutation({
    mutationFn: (request: StrategyPreviewTimelineRequest) =>
      previewStrategyTimeline(request),
  })
}

export function useStrategyPreviewOpenMutation() {
  return useMutation({
    mutationFn: (request: StrategyPreviewOpenRequest) =>
      openStrategyPreview(request),
  })
}

export function useDefaultMarketFeeTemplateQuery(market = "CN_A_SHARE") {
  return useQuery({
    queryKey: queryKeys.defaultMarketFeeTemplate(market),
    queryFn: () => getDefaultMarketFeeTemplate(market),
    retry: 1,
  })
}

export function useStrategyBacktestValidateMutation() {
  return useMutation({
    mutationFn: (request: StrategyBacktestValidateRequest) =>
      validateStrategyBacktest(request),
  })
}

export function useStrategyBacktestValidateQuery(
  request: StrategyBacktestValidateRequest | null,
  enabled = true
) {
  return useQuery({
    enabled: Boolean(request && enabled),
    queryKey: queryKeys.strategyBacktestValidate(request),
    queryFn: () => {
      if (!request) {
        throw new Error("strategy backtest validate request is missing")
      }
      return validateStrategyBacktest(request)
    },
    retry: 1,
  })
}

export function useStrategyBacktestOptionsQuery(benchmarkSecurityCode: string) {
  return useQuery({
    queryKey: queryKeys.strategyBacktestOptions(benchmarkSecurityCode),
    queryFn: () => getStrategyBacktestOptions(benchmarkSecurityCode),
    retry: 1,
    staleTime: 30_000,
  })
}

export function useStrategyBacktestCreateMutation() {
  return useMutation({
    mutationFn: (request: StrategyBacktestCreateRequest) =>
      createStrategyBacktest(request),
  })
}

export function useStrategyPortfolioCreateMutation() {
  return useMutation({
    mutationFn: (request: StrategyPortfolioCreateRequest) =>
      createStrategyPortfolio(request),
  })
}

export function useStrategyPortfolioDashboardQuery() {
  return useQuery({
    queryKey: queryKeys.strategyPortfolioDashboard(),
    queryFn: () => getStrategyPortfolioDashboard(),
    retry: 1,
  })
}

export function useStrategyPortfolioQuery(strategyPortfolioId: string | null) {
  return useQuery({
    enabled: Boolean(strategyPortfolioId),
    queryKey: queryKeys.strategyPortfolio(strategyPortfolioId),
    queryFn: () => {
      if (!strategyPortfolioId) {
        throw new Error("strategy portfolio id is missing")
      }
      return getStrategyPortfolio(strategyPortfolioId)
    },
    retry: 1,
  })
}

export function useStrategyPortfolioNavQuery(strategyPortfolioId: string | null) {
  return useQuery({
    enabled: Boolean(strategyPortfolioId),
    queryKey: queryKeys.strategyPortfolioNav(strategyPortfolioId),
    queryFn: () => {
      if (!strategyPortfolioId) {
        throw new Error("strategy portfolio id is missing")
      }
      return listStrategyPortfolioNav(strategyPortfolioId)
    },
    retry: 1,
  })
}

export function useStrategyPortfolioPerformanceQuery(
  strategyPortfolioId: string | null
) {
  return useQuery({
    enabled: Boolean(strategyPortfolioId),
    queryKey: queryKeys.strategyPortfolioPerformance(strategyPortfolioId),
    queryFn: () => {
      if (!strategyPortfolioId) {
        throw new Error("strategy portfolio id is missing")
      }
      return getStrategyPortfolioPerformance(strategyPortfolioId)
    },
    retry: 1,
  })
}

export function useStrategyPortfolioSignalTimelineQuery(
  strategyPortfolioId: string | null
) {
  return useQuery({
    enabled: Boolean(strategyPortfolioId),
    queryKey: queryKeys.strategyPortfolioSignalTimeline(strategyPortfolioId),
    queryFn: () => {
      if (!strategyPortfolioId) {
        throw new Error("strategy portfolio id is missing")
      }
      return listStrategyPortfolioSignalTimeline(strategyPortfolioId)
    },
    retry: 1,
  })
}

export function useStrategyPortfolioSignalsQuery(
  strategyPortfolioId: string | null,
  tradeDate: string | null
) {
  const query = tradeDate ? { signal_date: tradeDate, limit: 200 } : { limit: 200 }
  return useQuery({
    enabled: Boolean(strategyPortfolioId),
    queryKey: queryKeys.strategyPortfolioSignals(strategyPortfolioId, query),
    queryFn: () => {
      if (!strategyPortfolioId) {
        throw new Error("strategy portfolio id is missing")
      }
      return listStrategyPortfolioSignals(strategyPortfolioId, query)
    },
    retry: 1,
  })
}

export function useStrategyPortfolioPositionsQuery(
  strategyPortfolioId: string | null,
  tradeDate: string | null
) {
  const query = tradeDate ? { trade_date: tradeDate, limit: 200 } : { limit: 200 }
  return useQuery({
    enabled: Boolean(strategyPortfolioId),
    queryKey: queryKeys.strategyPortfolioPositions(strategyPortfolioId, query),
    queryFn: () => {
      if (!strategyPortfolioId) {
        throw new Error("strategy portfolio id is missing")
      }
      return listStrategyPortfolioPositions(strategyPortfolioId, query)
    },
    retry: 1,
  })
}

export function useStrategyPortfolioRebalanceRecordsQuery(
  strategyPortfolioId: string | null,
  tradeDate: string | null
) {
  return useQuery({
    enabled: Boolean(strategyPortfolioId),
    queryKey: queryKeys.strategyPortfolioRebalanceRecords(
      strategyPortfolioId,
      tradeDate
    ),
    queryFn: () => {
      if (!strategyPortfolioId) {
        throw new Error("strategy portfolio id is missing")
      }
      return listStrategyPortfolioRebalanceRecords(strategyPortfolioId, tradeDate)
    },
    placeholderData: keepPreviousData,
    retry: 1,
  })
}

export function useStrategyBacktestQuery(strategyBacktestRunId: string | null) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId),
    queryKey: queryKeys.strategyBacktest(strategyBacktestRunId),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return getStrategyBacktest(strategyBacktestRunId)
    },
    refetchInterval: (query) => {
      const status = query.state.data?.status
      return status &&
        !status.startsWith("failed_") &&
        status !== "succeeded" &&
        status !== "cancelled"
        ? 1_000
        : false
    },
    retry: 1,
  })
}

export function useStrategyBacktestStatusQuery(
  strategyBacktestRunId: string | null
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId),
    queryKey: queryKeys.strategyBacktestStatus(strategyBacktestRunId),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return getStrategyBacktestStatus(strategyBacktestRunId)
    },
    refetchInterval: (query) => {
      const status = query.state.data?.status
      return status &&
        !status.startsWith("failed_") &&
        status !== "succeeded" &&
        status !== "cancelled"
        ? 1_000
        : false
    },
    retry: 1,
  })
}

export function useStrategyBacktestNavQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestNav(strategyBacktestRunId),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestNav(strategyBacktestRunId)
    },
    retry: 1,
  })
}

export function useStrategyBacktestNavUiQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestNavUi(strategyBacktestRunId),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestNavUi(strategyBacktestRunId)
    },
    retry: 1,
  })
}

export function useStrategyBacktestRebalanceRecordsQuery(
  strategyBacktestRunId: string | null,
  tradeDate: string | null,
  enabled: boolean
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestRebalanceRecords(
      strategyBacktestRunId,
      tradeDate
    ),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestRebalanceRecords(
        strategyBacktestRunId,
        tradeDate
      )
    },
    placeholderData: keepPreviousData,
    retry: 1,
  })
}

export function useStrategyBacktestRebalanceRecordsUiQuery(
  strategyBacktestRunId: string | null,
  tradeDate: string | null,
  enabled: boolean
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestRebalanceRecordsUi(
      strategyBacktestRunId,
      tradeDate
    ),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestRebalanceRecordsUi(
        strategyBacktestRunId,
        tradeDate
      )
    },
    placeholderData: keepPreviousData,
    retry: 1,
  })
}

export function useStrategyBacktestPerformanceQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestPerformance(strategyBacktestRunId),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return getStrategyBacktestPerformance(strategyBacktestRunId)
    },
    retry: 1,
  })
}

export function useStrategyBacktestPerformanceUiQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestPerformanceUi(strategyBacktestRunId),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return getStrategyBacktestPerformanceUi(strategyBacktestRunId)
    },
    retry: 1,
  })
}

export function useStrategyBacktestTargetsQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean,
  query: QueryParams = {}
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestTargets(strategyBacktestRunId, query),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestTargets(strategyBacktestRunId, query)
    },
    retry: 1,
  })
}

export function useStrategyBacktestOrdersQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean,
  query: QueryParams = {}
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestOrders(strategyBacktestRunId, query),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestOrders(strategyBacktestRunId, query)
    },
    retry: 1,
  })
}

export function useStrategyBacktestTradesQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean,
  query: QueryParams = {}
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestTrades(strategyBacktestRunId, query),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestTrades(strategyBacktestRunId, query)
    },
    retry: 1,
  })
}

export function useStrategyBacktestPositionsQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean,
  query: QueryParams = {}
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestPositions(strategyBacktestRunId, query),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestPositions(strategyBacktestRunId, query)
    },
    retry: 1,
  })
}

export function useStrategyBacktestEventsQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean,
  query: QueryParams = {}
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestEvents(strategyBacktestRunId, query),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestEvents(strategyBacktestRunId, query)
    },
    retry: 1,
  })
}

export function useStrategyBacktestClosedTradesQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean,
  query: QueryParams = {}
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestClosedTrades(
      strategyBacktestRunId,
      query
    ),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestClosedTrades(strategyBacktestRunId, query)
    },
    retry: 1,
  })
}

export function useStrategyBacktestTradeMetricsQuery(
  strategyBacktestRunId: string | null,
  enabled: boolean,
  query: QueryParams = {}
) {
  return useQuery({
    enabled: Boolean(strategyBacktestRunId && enabled),
    queryKey: queryKeys.strategyBacktestTradeMetrics(
      strategyBacktestRunId,
      query
    ),
    queryFn: () => {
      if (!strategyBacktestRunId) {
        throw new Error("strategy backtest run id is missing")
      }
      return listStrategyBacktestTradeMetrics(strategyBacktestRunId, query)
    },
    retry: 1,
  })
}

export function useStrategyPreviewPoolPageQuery(
  previewId: string | null,
  request: StrategyPreviewPoolPageRequest | null
) {
  return useQuery({
    enabled: Boolean(previewId && request),
    queryKey: request
      ? queryKeys.previewPoolPage(
          previewId ?? "",
          request.trade_date,
          request.limit,
          request.offset
        )
      : queryKeys.previewPoolPage("", "", 0, 0),
    queryFn: () => {
      if (!request) {
        throw new Error("preview pool-page request is missing")
      }
      return previewStrategyPoolPage(request)
    },
    retry: 1,
  })
}

export function usePreviewSecurityAnalysisQuery(
  previewId: string | null,
  request: SecurityAnalysisRequest | null
) {
  return useQuery({
    enabled: Boolean(previewId && request),
    queryKey: request
      ? queryKeys.previewSecurityAnalysis(
          previewId ?? "",
          request.trade_date,
          request.security_code,
          request.adjustment ?? "",
          request.ma_windows ?? "",
          request.include_quote_rows ?? true
        )
      : queryKeys.previewSecurityAnalysis("", "", "", "", "", true),
    queryFn: ({ signal }) => {
      if (!request) {
        throw new Error("preview security-analysis request is missing")
      }
      return securityAnalysis(request, signal)
    },
    placeholderData: keepPreviousData,
    retry: 1,
    staleTime: 30_000,
  })
}

export function usePreviewChartContextQuery(
  previewId: string | null,
  request: PreviewChartContextRequest | null
) {
  return useQuery({
    enabled: Boolean(previewId && request),
    queryKey: request
      ? queryKeys.previewChartContext(
          previewId ?? "",
          request.trade_date,
          request.security_code,
          request.adjustment ?? "",
          request.ma_windows ?? ""
        )
      : queryKeys.previewChartContext("", "", "", "", ""),
    queryFn: () => {
      if (!request) {
        throw new Error("preview chart-context request is missing")
      }
      return previewChartContext(request)
    },
    placeholderData: keepPreviousData,
    retry: 1,
    staleTime: 30_000,
  })
}
