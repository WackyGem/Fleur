import { buildPath, jsonBody, normalizeList, requestJson } from "@/api/client"
import type {
  AccountTemplateRecord,
  BuySignalRecord,
  CreateAccountTemplateRequest,
  CreatePortfolioRunRequest,
  CreateRuleSetRequest,
  CreateRuleVersionRequest,
  CreateRunRequest,
  ExplainResponse,
  HealthResponse,
  ListResult,
  MarketFeeTemplateRecord,
  MetricDefinition,
  MetricsQuery,
  PortfolioClosedTradeQuery,
  PortfolioClosedTradeRecord,
  PortfolioEventQuery,
  PortfolioEventRecord,
  PortfolioNavRecord,
  PortfolioOrderQuery,
  PortfolioOrderRecord,
  PortfolioPerformanceQuery,
  PortfolioPerformanceResponse,
  PortfolioPositionQuery,
  PortfolioPositionRecord,
  PortfolioRunRecord,
  PortfolioRunsQuery,
  PortfolioTargetQuery,
  PortfolioTargetRecord,
  PortfolioTradeQuery,
  PortfolioTradeMetricQuery,
  PortfolioTradeMetricRecord,
  PortfolioTradeRecord,
  PatchAccountTemplateRequest,
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
  query: MetricsQuery = {}
): Promise<MetricDefinition[]> {
  const value = await requestJson<
    MetricDefinition[] | { items: MetricDefinition[] }
  >(buildPath("/rearview/metrics", query))
  return Array.isArray(value) ? value : value.items
}

export async function listRuleSets(
  query: RuleSetsQuery = {}
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
    })
  )
}

export async function listRuleVersions(
  ruleSetId: string,
  query: RuleVersionsQuery = {}
): Promise<ListResult<RuleVersionRecord>> {
  const value = await requestJson<
    RuleVersionRecord[] | Partial<ListResult<RuleVersionRecord>>
  >(buildPath(`/rearview/rule-sets/${ruleSetId}/versions`, query))
  return normalizeList(value, query.limit)
}

export function createRuleVersion(
  ruleSetId: string,
  request: CreateRuleVersionRequest
) {
  return requestJson<RuleVersionRecord>(
    `/rearview/rule-sets/${ruleSetId}/versions`,
    jsonBody(request)
  )
}

export function explainRule(
  rule: RuleVersionSpec,
  range?: { start_date?: string; end_date?: string; top_n?: number }
) {
  const body = range?.start_date && range.end_date ? { rule, ...range } : rule
  return requestJson<ExplainResponse>("/rearview/explain", jsonBody(body))
}

export async function listRuns(
  query: RunsQuery = {}
): Promise<ListResult<RunRecord>> {
  const value = await requestJson<RunRecord[] | Partial<ListResult<RunRecord>>>(
    buildPath("/rearview/runs", query)
  )
  return normalizeList(value, query.limit)
}

export function createRun(request: CreateRunRequest) {
  return requestJson<RunRecord>("/rearview/runs", jsonBody(request))
}

export function getDefaultMarketFeeTemplate(market = "CN_A_SHARE") {
  return requestJson<MarketFeeTemplateRecord>(
    buildPath("/rearview/market-fee-templates/default", { market })
  )
}

export function listAccountTemplates(ruleSetId: string) {
  return requestJson<AccountTemplateRecord[]>(
    `/rearview/rule-sets/${ruleSetId}/account-templates`
  )
}

export function createAccountTemplate(
  ruleSetId: string,
  request: CreateAccountTemplateRequest
) {
  return requestJson<AccountTemplateRecord>(
    `/rearview/rule-sets/${ruleSetId}/account-templates`,
    jsonBody(request)
  )
}

export function updateAccountTemplate(
  accountTemplateId: string,
  request: PatchAccountTemplateRequest
) {
  return requestJson<AccountTemplateRecord>(
    `/rearview/account-templates/${accountTemplateId}`,
    {
      ...jsonBody(request),
      method: "PATCH",
    }
  )
}

export async function listPortfolioRuns(
  query: PortfolioRunsQuery = {}
): Promise<ListResult<PortfolioRunRecord>> {
  const value = await requestJson<
    PortfolioRunRecord[] | Partial<ListResult<PortfolioRunRecord>>
  >(buildPath("/rearview/portfolio-runs", query))
  return normalizeList(value, query.limit)
}

export function createPortfolioRun(request: CreatePortfolioRunRequest) {
  return requestJson<PortfolioRunRecord>(
    "/rearview/portfolio-runs",
    jsonBody(request)
  )
}

export function getPortfolioRun(portfolioRunId: string) {
  return requestJson<PortfolioRunRecord>(
    `/rearview/portfolio-runs/${portfolioRunId}`
  )
}

export function listPortfolioNav(portfolioRunId: string) {
  return requestJson<PortfolioNavRecord[]>(
    `/rearview/portfolio-runs/${portfolioRunId}/nav`
  )
}

export async function listPortfolioTargets(
  portfolioRunId: string,
  query: PortfolioTargetQuery = {}
): Promise<ListResult<PortfolioTargetRecord>> {
  const value = await requestJson<
    PortfolioTargetRecord[] | Partial<ListResult<PortfolioTargetRecord>>
  >(buildPath(`/rearview/portfolio-runs/${portfolioRunId}/targets`, query))
  return normalizeList(value, query.limit)
}

export async function listPortfolioOrders(
  portfolioRunId: string,
  query: PortfolioOrderQuery = {}
): Promise<ListResult<PortfolioOrderRecord>> {
  const value = await requestJson<
    PortfolioOrderRecord[] | Partial<ListResult<PortfolioOrderRecord>>
  >(buildPath(`/rearview/portfolio-runs/${portfolioRunId}/orders`, query))
  return normalizeList(value, query.limit)
}

export async function listPortfolioTrades(
  portfolioRunId: string,
  query: PortfolioTradeQuery = {}
): Promise<ListResult<PortfolioTradeRecord>> {
  const value = await requestJson<
    PortfolioTradeRecord[] | Partial<ListResult<PortfolioTradeRecord>>
  >(buildPath(`/rearview/portfolio-runs/${portfolioRunId}/trades`, query))
  return normalizeList(value, query.limit)
}

export async function listPortfolioPositions(
  portfolioRunId: string,
  query: PortfolioPositionQuery = {}
): Promise<ListResult<PortfolioPositionRecord>> {
  const value = await requestJson<
    PortfolioPositionRecord[] | Partial<ListResult<PortfolioPositionRecord>>
  >(buildPath(`/rearview/portfolio-runs/${portfolioRunId}/positions`, query))
  return normalizeList(value, query.limit)
}

export async function listPortfolioEvents(
  portfolioRunId: string,
  query: PortfolioEventQuery = {}
): Promise<ListResult<PortfolioEventRecord>> {
  const value = await requestJson<
    PortfolioEventRecord[] | Partial<ListResult<PortfolioEventRecord>>
  >(buildPath(`/rearview/portfolio-runs/${portfolioRunId}/events`, query))
  return normalizeList(value, query.limit)
}

export function getPortfolioPerformance(
  portfolioRunId: string,
  query: PortfolioPerformanceQuery = {}
) {
  return requestJson<PortfolioPerformanceResponse>(
    buildPath(`/rearview/portfolio-runs/${portfolioRunId}/performance`, query)
  )
}

export async function listPortfolioClosedTrades(
  portfolioRunId: string,
  query: PortfolioClosedTradeQuery = {}
): Promise<ListResult<PortfolioClosedTradeRecord>> {
  const value = await requestJson<
    PortfolioClosedTradeRecord[] | Partial<ListResult<PortfolioClosedTradeRecord>>
  >(buildPath(`/rearview/portfolio-runs/${portfolioRunId}/closed-trades`, query))
  return normalizeList(value, query.limit)
}

export async function listPortfolioTradeMetrics(
  portfolioRunId: string,
  query: PortfolioTradeMetricQuery = {}
): Promise<ListResult<PortfolioTradeMetricRecord>> {
  const value = await requestJson<
    PortfolioTradeMetricRecord[] | Partial<ListResult<PortfolioTradeMetricRecord>>
  >(buildPath(`/rearview/portfolio-runs/${portfolioRunId}/trade-metrics`, query))
  return normalizeList(value, query.limit)
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
  query: ResultRowsQuery
): Promise<ListResult<PoolMemberRecord>> {
  const value = await requestJson<
    PoolMemberRecord[] | Partial<ListResult<PoolMemberRecord>>
  >(buildPath(`/rearview/runs/${runId}/pool`, query))
  return normalizeList(value, query.limit)
}

export async function listBuySignals(
  runId: string,
  query: ResultRowsQuery
): Promise<ListResult<BuySignalRecord>> {
  const value = await requestJson<
    BuySignalRecord[] | Partial<ListResult<BuySignalRecord>>
  >(buildPath(`/rearview/runs/${runId}/signals`, query))
  return normalizeList(value, query.limit)
}

export function getSecurityAnalysis(
  runId: string,
  securityCode: string,
  query: SecurityAnalysisQuery
) {
  return requestJson<SecurityAnalysisResponse>(
    buildPath(
      `/rearview/runs/${runId}/securities/${securityCode}/analysis`,
      query
    )
  )
}
