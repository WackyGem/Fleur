import {
  buildPath,
  jsonBody,
  normalizeList,
  requestJson,
} from "@/api/client"
import type {
  BuySignalRecord,
  CreateRuleSetRequest,
  CreateRuleVersionRequest,
  CreateRunRequest,
  ExplainResponse,
  HealthResponse,
  ListResult,
  MetricDefinition,
  MetricsQuery,
  PoolMemberRecord,
  ResultRowsQuery,
  RuleSetRecord,
  RuleSetsQuery,
  RuleVersionRecord,
  RuleVersionsQuery,
  RuleVersionSpec,
  RunChunkRecord,
  RunDayRecord,
  RunRecord,
  RunsQuery,
  SecurityAnalysisQuery,
  SecurityAnalysisResponse,
} from "@/types/rearview"

export function getHealth() {
  return requestJson<HealthResponse>("/healthz")
}

export async function listMetrics(
  query: MetricsQuery = {},
): Promise<MetricDefinition[]> {
  const value = await requestJson<MetricDefinition[] | { items: MetricDefinition[] }>(
    buildPath("/rearview/metrics", query),
  )
  return Array.isArray(value) ? value : value.items
}

export async function listRuleSets(
  query: RuleSetsQuery = {},
): Promise<ListResult<RuleSetRecord>> {
  const value = await requestJson<
    RuleSetRecord[] | Partial<ListResult<RuleSetRecord>>
  >(buildPath("/rearview/rule-sets", query))
  return normalizeList(value, query.limit)
}

export function createRuleSet(request: CreateRuleSetRequest) {
  return requestJson<RuleSetRecord>(
    "/rearview/rule-sets",
    jsonBody({
      ...request,
      tags: request.tags ?? [],
    }),
  )
}

export async function listRuleVersions(
  ruleSetId: string,
  query: RuleVersionsQuery = {},
): Promise<ListResult<RuleVersionRecord>> {
  const value = await requestJson<
    RuleVersionRecord[] | Partial<ListResult<RuleVersionRecord>>
  >(buildPath(`/rearview/rule-sets/${ruleSetId}/versions`, query))
  return normalizeList(value, query.limit)
}

export function createRuleVersion(
  ruleSetId: string,
  request: CreateRuleVersionRequest,
) {
  return requestJson<RuleVersionRecord>(
    `/rearview/rule-sets/${ruleSetId}/versions`,
    jsonBody(request),
  )
}

export function explainRule(
  rule: RuleVersionSpec,
  range?: { start_date?: string; end_date?: string; top_n?: number },
) {
  const body =
    range?.start_date && range.end_date
      ? { rule, ...range }
      : rule
  return requestJson<ExplainResponse>("/rearview/explain", jsonBody(body))
}

export async function listRuns(
  query: RunsQuery = {},
): Promise<ListResult<RunRecord>> {
  const value = await requestJson<RunRecord[] | Partial<ListResult<RunRecord>>>(
    buildPath("/rearview/runs", query),
  )
  return normalizeList(value, query.limit)
}

export function createRun(request: CreateRunRequest) {
  return requestJson<RunRecord>("/rearview/runs", jsonBody(request))
}

export function getRun(runId: string) {
  return requestJson<RunRecord>(`/rearview/runs/${runId}`)
}

export function listRunChunks(runId: string) {
  return requestJson<RunChunkRecord[]>(`/rearview/runs/${runId}/chunks`)
}

export function listRunDays(runId: string) {
  return requestJson<RunDayRecord[]>(`/rearview/runs/${runId}/days`)
}

export async function listPoolMembers(
  runId: string,
  query: ResultRowsQuery,
): Promise<ListResult<PoolMemberRecord>> {
  const value = await requestJson<
    PoolMemberRecord[] | Partial<ListResult<PoolMemberRecord>>
  >(buildPath(`/rearview/runs/${runId}/pool`, query))
  return normalizeList(value, query.limit)
}

export async function listBuySignals(
  runId: string,
  query: ResultRowsQuery,
): Promise<ListResult<BuySignalRecord>> {
  const value = await requestJson<
    BuySignalRecord[] | Partial<ListResult<BuySignalRecord>>
  >(buildPath(`/rearview/runs/${runId}/signals`, query))
  return normalizeList(value, query.limit)
}

export function getSecurityAnalysis(
  runId: string,
  securityCode: string,
  query: SecurityAnalysisQuery,
) {
  return requestJson<SecurityAnalysisResponse>(
    buildPath(
      `/rearview/runs/${runId}/securities/${securityCode}/analysis`,
      query,
    ),
  )
}
