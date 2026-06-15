import type {
  MetricsQuery,
  ResultRowsQuery,
  RuleSetsQuery,
  RuleVersionsQuery,
  RunsQuery,
  SecurityAnalysisQuery,
} from "@/types/rearview"

export const queryKeys = {
  health: ["health"] as const,
  metrics: (query: MetricsQuery = {}) => ["metrics", query] as const,
  ruleSets: (query: RuleSetsQuery = {}) => ["rule-sets", query] as const,
  ruleVersions: (ruleSetId: string, query: RuleVersionsQuery = {}) =>
    ["rule-sets", ruleSetId, "versions", query] as const,
  runs: (query: RunsQuery = {}) => ["runs", query] as const,
  run: (runId: string) => ["runs", runId] as const,
  runChunks: (runId: string) => ["runs", runId, "chunks"] as const,
  runDays: (runId: string) => ["runs", runId, "days"] as const,
  pool: (runId: string, query: ResultRowsQuery) =>
    ["runs", runId, "pool", query] as const,
  signals: (runId: string, query: ResultRowsQuery) =>
    ["runs", runId, "signals", query] as const,
  securityAnalysis: (
    runId: string,
    securityCode: string,
    query: SecurityAnalysisQuery,
  ) => ["runs", runId, "securities", securityCode, "analysis", query] as const,
}
