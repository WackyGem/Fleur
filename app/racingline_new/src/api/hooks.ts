import { keepPreviousData, useMutation, useQuery } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import {
  explainRule,
  listMetrics,
  previewStrategy,
  previewStrategyPoolPage,
  securityAnalysis,
  previewStrategyTimeline,
} from "@/api/rearview"
import type {
  MetricsQuery,
  RuleVersionSpec,
  SecurityAnalysisRequest,
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
