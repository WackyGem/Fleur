import { useMutation, useQuery } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import {
  explainRule,
  listMetrics,
  previewStrategy,
  previewStrategyPoolPage,
  previewStrategySecurityAnalysis,
} from "@/api/rearview"
import type {
  MetricsQuery,
  PreviewSecurityAnalysisRequest,
  RuleVersionSpec,
  StrategyPreviewPoolPageRequest,
  StrategyPreviewRequest,
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
  request: PreviewSecurityAnalysisRequest | null
) {
  return useQuery({
    enabled: Boolean(previewId && request),
    queryKey: request
      ? queryKeys.previewSecurityAnalysis(
          previewId ?? "",
          request.trade_date,
          request.security_code
        )
      : queryKeys.previewSecurityAnalysis("", "", ""),
    queryFn: () => {
      if (!request) {
        throw new Error("preview security-analysis request is missing")
      }
      return previewStrategySecurityAnalysis(request)
    },
    retry: 1,
  })
}
