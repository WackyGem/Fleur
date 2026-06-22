import { useMutation, useQuery } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import { explainRule, listMetrics } from "@/api/rearview"
import type { MetricsQuery, RuleVersionSpec } from "@/types/rearview"

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
