import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import {
  createAccountTemplate,
  createPortfolioRun,
  createRuleSet,
  createRuleVersion,
  createRun,
  explainRule,
  getDefaultMarketFeeTemplate,
  getPortfolioPerformance,
  getHealth,
  getPortfolioRun,
  getRun,
  getSecurityAnalysis,
  listBuySignals,
  listAccountTemplates,
  listMetrics,
  listPoolMembers,
  listPortfolioClosedTrades,
  listPortfolioEvents,
  listPortfolioNav,
  listPortfolioOrders,
  listPortfolioPositions,
  listPortfolioRuns,
  listPortfolioTargets,
  listPortfolioTradeMetrics,
  listPortfolioTrades,
  listRuleSets,
  listRuleVersions,
  listRunChunks,
  listRunDays,
  listRuns,
  updateAccountTemplate,
} from "@/api/rearview"
import { isPortfolioActiveStatus, isRunActiveStatus } from "@/lib/status"
import type {
  CreateAccountTemplateRequest,
  CreatePortfolioRunRequest,
  CreateRuleSetRequest,
  CreateRuleVersionRequest,
  CreateRunRequest,
  MetricsQuery,
  PortfolioClosedTradeQuery,
  PatchAccountTemplateRequest,
  PortfolioEventQuery,
  PortfolioOrderQuery,
  PortfolioPerformanceQuery,
  PortfolioPositionQuery,
  PortfolioRunsQuery,
  PortfolioTargetQuery,
  PortfolioTradeMetricQuery,
  PortfolioTradeQuery,
  ResultRowsQuery,
  RuleSetsQuery,
  RuleVersionSpec,
  RuleVersionsQuery,
  RunsQuery,
  SecurityAnalysisQuery,
} from "@/types/rearview"

export function useHealthQuery() {
  return useQuery({
    queryKey: queryKeys.health,
    queryFn: getHealth,
    retry: 1,
    staleTime: 10_000,
  })
}

export function useMetricsQuery(query: MetricsQuery = {}) {
  return useQuery({
    queryKey: queryKeys.metrics(query),
    queryFn: () => listMetrics(query),
    retry: 1,
  })
}

export function useRuleSetsQuery(query: RuleSetsQuery = {}) {
  return useQuery({
    queryKey: queryKeys.ruleSets(query),
    queryFn: () => listRuleSets(query),
    retry: 1,
  })
}

export function useRuleVersionsQuery(
  ruleSetId: string | undefined,
  query: RuleVersionsQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.ruleVersions(ruleSetId ?? "", query),
    queryFn: () => listRuleVersions(ruleSetId ?? "", query),
    enabled: Boolean(ruleSetId),
    retry: 1,
  })
}

export function useRunsQuery(query: RunsQuery = {}) {
  return useQuery({
    queryKey: queryKeys.runs(query),
    queryFn: () => listRuns(query),
    retry: 1,
    refetchInterval: (queryState) => {
      const runs = queryState.state.data?.items ?? []
      return runs.some((run) => isRunActiveStatus(run.status)) ? 3_000 : false
    },
  })
}

export function useDefaultMarketFeeTemplateQuery(market = "CN_A_SHARE") {
  return useQuery({
    queryKey: queryKeys.defaultMarketFeeTemplate(market),
    queryFn: () => getDefaultMarketFeeTemplate(market),
    retry: 1,
  })
}

export function useAccountTemplatesQuery(ruleSetId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.accountTemplates(ruleSetId ?? ""),
    queryFn: () => listAccountTemplates(ruleSetId ?? ""),
    enabled: Boolean(ruleSetId),
    retry: 1,
  })
}

export function usePortfolioRunsQuery(query: PortfolioRunsQuery = {}) {
  return useQuery({
    queryKey: queryKeys.portfolioRuns(query),
    queryFn: () => listPortfolioRuns(query),
    retry: 1,
    refetchInterval: (queryState) => {
      const runs = queryState.state.data?.items ?? []
      return runs.some((run) => isPortfolioActiveStatus(run.status))
        ? 3_000
        : false
    },
  })
}

export function usePortfolioRunQuery(portfolioRunId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.portfolioRun(portfolioRunId ?? ""),
    queryFn: () => getPortfolioRun(portfolioRunId ?? ""),
    enabled: Boolean(portfolioRunId),
    retry: 1,
    refetchInterval: (queryState) =>
      isPortfolioActiveStatus(queryState.state.data?.status) ? 3_000 : false,
  })
}

export function usePortfolioNavQuery(
  portfolioRunId: string | undefined,
  runStatus?: string
) {
  return useQuery({
    queryKey: queryKeys.portfolioNav(portfolioRunId ?? ""),
    queryFn: () => listPortfolioNav(portfolioRunId ?? ""),
    enabled: Boolean(portfolioRunId),
    retry: 1,
    refetchInterval: () => (isPortfolioActiveStatus(runStatus) ? 3_000 : false),
  })
}

export function usePortfolioTargetsQuery(
  portfolioRunId: string | undefined,
  query: PortfolioTargetQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioTargets(portfolioRunId ?? "", query),
    queryFn: () => listPortfolioTargets(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function usePortfolioOrdersQuery(
  portfolioRunId: string | undefined,
  query: PortfolioOrderQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioOrders(portfolioRunId ?? "", query),
    queryFn: () => listPortfolioOrders(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function usePortfolioTradesQuery(
  portfolioRunId: string | undefined,
  query: PortfolioTradeQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioTrades(portfolioRunId ?? "", query),
    queryFn: () => listPortfolioTrades(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function usePortfolioPositionsQuery(
  portfolioRunId: string | undefined,
  query: PortfolioPositionQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioPositions(portfolioRunId ?? "", query),
    queryFn: () => listPortfolioPositions(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function usePortfolioEventsQuery(
  portfolioRunId: string | undefined,
  query: PortfolioEventQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioEvents(portfolioRunId ?? "", query),
    queryFn: () => listPortfolioEvents(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function usePortfolioPerformanceQuery(
  portfolioRunId: string | undefined,
  query: PortfolioPerformanceQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioPerformance(portfolioRunId ?? "", query),
    queryFn: () => getPortfolioPerformance(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function usePortfolioClosedTradesQuery(
  portfolioRunId: string | undefined,
  query: PortfolioClosedTradeQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioClosedTrades(portfolioRunId ?? "", query),
    queryFn: () => listPortfolioClosedTrades(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function usePortfolioTradeMetricsQuery(
  portfolioRunId: string | undefined,
  query: PortfolioTradeMetricQuery = {}
) {
  return useQuery({
    queryKey: queryKeys.portfolioTradeMetrics(portfolioRunId ?? "", query),
    queryFn: () => listPortfolioTradeMetrics(portfolioRunId ?? "", query),
    enabled: Boolean(portfolioRunId),
    retry: 1,
  })
}

export function useRunQuery(runId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.run(runId ?? ""),
    queryFn: () => getRun(runId ?? ""),
    enabled: Boolean(runId),
    retry: 1,
    refetchInterval: (queryState) =>
      isRunActiveStatus(queryState.state.data?.status) ? 3_000 : false,
  })
}

export function useRunChunksQuery(
  runId: string | undefined,
  runStatus?: string
) {
  return useQuery({
    queryKey: queryKeys.runChunks(runId ?? ""),
    queryFn: () => listRunChunks(runId ?? ""),
    enabled: Boolean(runId),
    retry: 1,
    refetchInterval: () => (isRunActiveStatus(runStatus) ? 3_000 : false),
  })
}

export function useRunDaysQuery(runId: string | undefined, runStatus?: string) {
  return useQuery({
    queryKey: queryKeys.runDays(runId ?? ""),
    queryFn: () => listRunDays(runId ?? ""),
    enabled: Boolean(runId),
    retry: 1,
    refetchInterval: () => (isRunActiveStatus(runStatus) ? 3_000 : false),
  })
}

export function usePoolMembersQuery(
  runId: string | undefined,
  query: ResultRowsQuery | undefined
) {
  return useQuery({
    queryKey: query
      ? queryKeys.pool(runId ?? "", query)
      : queryKeys.pool(runId ?? "", { trade_date: "" }),
    queryFn: () => listPoolMembers(runId ?? "", query ?? { trade_date: "" }),
    enabled: Boolean(runId && query?.trade_date),
    retry: 1,
  })
}

export function useBuySignalsQuery(
  runId: string | undefined,
  query: ResultRowsQuery | undefined
) {
  return useQuery({
    queryKey: query
      ? queryKeys.signals(runId ?? "", query)
      : queryKeys.signals(runId ?? "", { trade_date: "" }),
    queryFn: () => listBuySignals(runId ?? "", query ?? { trade_date: "" }),
    enabled: Boolean(runId && query?.trade_date),
    retry: 1,
  })
}

export function useSecurityAnalysisQuery(
  runId: string | undefined,
  securityCode: string | undefined,
  query: SecurityAnalysisQuery | undefined
) {
  return useQuery({
    queryKey: query
      ? queryKeys.securityAnalysis(runId ?? "", securityCode ?? "", query)
      : queryKeys.securityAnalysis(runId ?? "", securityCode ?? "", {
          trade_date: "",
          source: "signals",
        }),
    queryFn: () =>
      getSecurityAnalysis(
        runId ?? "",
        securityCode ?? "",
        query ?? {
          trade_date: "",
          source: "signals",
        }
      ),
    enabled: Boolean(
      runId && securityCode && query?.trade_date && query.source
    ),
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

export function useCreateRuleSetMutation() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (request: CreateRuleSetRequest) => createRuleSet(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["rule-sets"] })
    },
  })
}

export function useCreateRuleVersionMutation(ruleSetId?: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({
      targetRuleSetId,
      request,
    }: {
      targetRuleSetId: string
      request: CreateRuleVersionRequest
    }) => createRuleVersion(targetRuleSetId, request),
    onSuccess: async (_, variables) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["rule-sets"] }),
        queryClient.invalidateQueries({
          queryKey: ["rule-sets", variables.targetRuleSetId, "versions"],
        }),
        ruleSetId
          ? queryClient.invalidateQueries({
              queryKey: ["rule-sets", ruleSetId, "versions"],
            })
          : Promise.resolve(),
      ])
    },
  })
}

export function useCreateRunMutation() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (request: CreateRunRequest) => createRun(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["runs"] })
    },
  })
}

export function useCreateAccountTemplateMutation(ruleSetId?: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({
      targetRuleSetId,
      request,
    }: {
      targetRuleSetId: string
      request: CreateAccountTemplateRequest
    }) => createAccountTemplate(targetRuleSetId, request),
    onSuccess: async (_, variables) => {
      await queryClient.invalidateQueries({
        queryKey: queryKeys.accountTemplates(variables.targetRuleSetId),
      })
      if (ruleSetId && ruleSetId !== variables.targetRuleSetId) {
        await queryClient.invalidateQueries({
          queryKey: queryKeys.accountTemplates(ruleSetId),
        })
      }
    },
  })
}

export function useUpdateAccountTemplateMutation(ruleSetId?: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({
      accountTemplateId,
      request,
    }: {
      accountTemplateId: string
      request: PatchAccountTemplateRequest
    }) => updateAccountTemplate(accountTemplateId, request),
    onSuccess: async () => {
      if (ruleSetId) {
        await queryClient.invalidateQueries({
          queryKey: queryKeys.accountTemplates(ruleSetId),
        })
      } else {
        await queryClient.invalidateQueries({ queryKey: ["rule-sets"] })
      }
    },
  })
}

export function useCreatePortfolioRunMutation() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (request: CreatePortfolioRunRequest) =>
      createPortfolioRun(request),
    onSuccess: async (portfolioRun) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["portfolio-runs"] }),
        queryClient.invalidateQueries({
          queryKey: ["portfolio-runs", portfolioRun.portfolio_run_id],
        }),
      ])
    },
  })
}
