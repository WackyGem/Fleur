import { keepPreviousData, useMutation, useQuery } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import {
  createStrategyBacktest,
  explainRule,
  getStrategyBacktest,
  getStrategyBacktestOptions,
  getStrategyBacktestPerformance,
  getDefaultMarketFeeTemplate,
  listStrategyBacktestNav,
  listStrategyBacktestRebalanceRecords,
  listMetrics,
  previewStrategy,
  previewStrategyPoolPage,
  securityAnalysis,
  previewStrategyTimeline,
  validateStrategyBacktest,
} from "@/api/rearview"
import type {
  MetricsQuery,
  RuleVersionSpec,
  SecurityAnalysisRequest,
  StrategyBacktestCreateRequest,
  StrategyBacktestValidateRequest,
  StrategyPreviewPoolPageRequest,
  StrategyPreviewRequest,
  StrategyPreviewTimelineRequest,
} from "@/types/rearview"

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
  request: StrategyBacktestValidateRequest | null
) {
  return useQuery({
    enabled: Boolean(request),
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
        ? 2_000
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
