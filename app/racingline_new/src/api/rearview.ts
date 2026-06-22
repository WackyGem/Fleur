import { buildPath, jsonBody, requestJson } from "@/api/client"
import type {
  ExplainResponse,
  MetricDefinition,
  MetricsQuery,
  RuleVersionSpec,
  StrategyPreviewRequest,
  StrategyPreviewResponse,
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
