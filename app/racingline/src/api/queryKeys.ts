import type {
  MetricsQuery,
  PortfolioEventQuery,
  PortfolioOrderQuery,
  PortfolioPositionQuery,
  PortfolioRunsQuery,
  PortfolioTargetQuery,
  PortfolioTradeQuery,
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
  defaultMarketFeeTemplate: (market: string) =>
    ["market-fee-templates", "default", market] as const,
  accountTemplates: (ruleSetId: string) =>
    ["rule-sets", ruleSetId, "account-templates"] as const,
  portfolioRuns: (query: PortfolioRunsQuery = {}) =>
    ["portfolio-runs", query] as const,
  portfolioRun: (portfolioRunId: string) =>
    ["portfolio-runs", portfolioRunId] as const,
  portfolioNav: (portfolioRunId: string) =>
    ["portfolio-runs", portfolioRunId, "nav"] as const,
  portfolioTargets: (
    portfolioRunId: string,
    query: PortfolioTargetQuery = {}
  ) => ["portfolio-runs", portfolioRunId, "targets", query] as const,
  portfolioOrders: (portfolioRunId: string, query: PortfolioOrderQuery = {}) =>
    ["portfolio-runs", portfolioRunId, "orders", query] as const,
  portfolioTrades: (portfolioRunId: string, query: PortfolioTradeQuery = {}) =>
    ["portfolio-runs", portfolioRunId, "trades", query] as const,
  portfolioPositions: (
    portfolioRunId: string,
    query: PortfolioPositionQuery = {}
  ) => ["portfolio-runs", portfolioRunId, "positions", query] as const,
  portfolioEvents: (portfolioRunId: string, query: PortfolioEventQuery = {}) =>
    ["portfolio-runs", portfolioRunId, "events", query] as const,
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
    query: SecurityAnalysisQuery
  ) => ["runs", runId, "securities", securityCode, "analysis", query] as const,
}
