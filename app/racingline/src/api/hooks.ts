import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"

import { queryKeys } from "@/api/queryKeys"
import {
  createRuleSet,
  createRuleVersion,
  createRun,
  explainRule,
  getHealth,
  getRun,
  listBuySignals,
  listMetrics,
  listPoolMembers,
  listRuleSets,
  listRuleVersions,
  listRunChunks,
  listRunDays,
  listRuns,
} from "@/api/rearview"
import { isRunActiveStatus } from "@/lib/status"
import type {
  CreateRuleSetRequest,
  CreateRuleVersionRequest,
  CreateRunRequest,
  MetricsQuery,
  ResultRowsQuery,
  RuleSetsQuery,
  RuleVersionSpec,
  RuleVersionsQuery,
  RunsQuery,
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
  query: RuleVersionsQuery = {},
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
  runStatus?: string,
) {
  return useQuery({
    queryKey: queryKeys.runChunks(runId ?? ""),
    queryFn: () => listRunChunks(runId ?? ""),
    enabled: Boolean(runId),
    retry: 1,
    refetchInterval: () => (isRunActiveStatus(runStatus) ? 3_000 : false),
  })
}

export function useRunDaysQuery(
  runId: string | undefined,
  runStatus?: string,
) {
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
  query: ResultRowsQuery | undefined,
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
  query: ResultRowsQuery | undefined,
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
