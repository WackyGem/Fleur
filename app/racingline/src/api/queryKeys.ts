import type {
  MetricsQuery,
  StrategyBacktestValidateRequest,
} from "@/types/rearview"
import type { QueryParams } from "@/api/client"

export const queryKeys = {
  metrics: (query: MetricsQuery = {}) => ["metrics", query] as const,
  defaultMarketFeeTemplate: (market: string) =>
    ["market-fee-templates", "default", market] as const,
  strategyBacktestValidate: (
    request: StrategyBacktestValidateRequest | null
  ) => ["strategy-backtests", "validate", request] as const,
  strategyBacktestOptions: (benchmarkSecurityCode: string) =>
    ["strategy-backtests", "options", benchmarkSecurityCode] as const,
  strategyBacktest: (strategyBacktestRunId: string | null) =>
    ["strategy-backtests", strategyBacktestRunId] as const,
  strategyBacktestStatus: (strategyBacktestRunId: string | null) =>
    ["strategy-backtests", strategyBacktestRunId, "status"] as const,
  strategyBacktestNav: (strategyBacktestRunId: string | null) =>
    ["strategy-backtests", strategyBacktestRunId, "nav"] as const,
  strategyBacktestNavUi: (strategyBacktestRunId: string | null) =>
    ["strategy-backtests", strategyBacktestRunId, "nav", "ui"] as const,
  strategyBacktestRebalanceRecords: (
    strategyBacktestRunId: string | null,
    tradeDate?: string | null
  ) =>
    [
      "strategy-backtests",
      strategyBacktestRunId,
      "rebalance-records",
      tradeDate ?? null,
    ] as const,
  strategyBacktestRebalanceRecordsUi: (
    strategyBacktestRunId: string | null,
    tradeDate?: string | null
  ) =>
    [
      "strategy-backtests",
      strategyBacktestRunId,
      "rebalance-records",
      "ui",
      tradeDate ?? null,
    ] as const,
  strategyBacktestPerformance: (strategyBacktestRunId: string | null) =>
    ["strategy-backtests", strategyBacktestRunId, "performance"] as const,
  strategyBacktestPerformanceUi: (strategyBacktestRunId: string | null) =>
    ["strategy-backtests", strategyBacktestRunId, "performance", "ui"] as const,
  strategyBacktestTargets: (
    strategyBacktestRunId: string | null,
    query: QueryParams = {}
  ) => ["strategy-backtests", strategyBacktestRunId, "targets", query] as const,
  strategyBacktestOrders: (
    strategyBacktestRunId: string | null,
    query: QueryParams = {}
  ) => ["strategy-backtests", strategyBacktestRunId, "orders", query] as const,
  strategyBacktestTrades: (
    strategyBacktestRunId: string | null,
    query: QueryParams = {}
  ) => ["strategy-backtests", strategyBacktestRunId, "trades", query] as const,
  strategyBacktestPositions: (
    strategyBacktestRunId: string | null,
    query: QueryParams = {}
  ) =>
    ["strategy-backtests", strategyBacktestRunId, "positions", query] as const,
  strategyBacktestEvents: (
    strategyBacktestRunId: string | null,
    query: QueryParams = {}
  ) => ["strategy-backtests", strategyBacktestRunId, "events", query] as const,
  strategyBacktestClosedTrades: (
    strategyBacktestRunId: string | null,
    query: QueryParams = {}
  ) =>
    [
      "strategy-backtests",
      strategyBacktestRunId,
      "closed-trades",
      query,
    ] as const,
  strategyBacktestTradeMetrics: (
    strategyBacktestRunId: string | null,
    query: QueryParams = {}
  ) =>
    [
      "strategy-backtests",
      strategyBacktestRunId,
      "trade-metrics",
      query,
    ] as const,
  strategyPortfolioDashboard: () =>
    ["strategy-portfolios", "dashboard"] as const,
  strategyPortfolio: (strategyPortfolioId: string | null) =>
    ["strategy-portfolios", strategyPortfolioId] as const,
  strategyPortfolioNav: (strategyPortfolioId: string | null) =>
    ["strategy-portfolios", strategyPortfolioId, "nav"] as const,
  strategyPortfolioPerformance: (strategyPortfolioId: string | null) =>
    ["strategy-portfolios", strategyPortfolioId, "performance"] as const,
  strategyPortfolioSignals: (
    strategyPortfolioId: string | null,
    query: QueryParams = {}
  ) => ["strategy-portfolios", strategyPortfolioId, "signals", query] as const,
  strategyPortfolioSignalTimeline: (strategyPortfolioId: string | null) =>
    ["strategy-portfolios", strategyPortfolioId, "signal-timeline"] as const,
  strategyPortfolioPositions: (
    strategyPortfolioId: string | null,
    query: QueryParams = {}
  ) =>
    ["strategy-portfolios", strategyPortfolioId, "positions", query] as const,
  strategyPortfolioRebalanceRecords: (
    strategyPortfolioId: string | null,
    tradeDate?: string | null
  ) =>
    [
      "strategy-portfolios",
      strategyPortfolioId,
      "rebalance-records",
      tradeDate ?? null,
    ] as const,
  previewTimeline: (previewId: string, startDate: string, endDate: string) =>
    ["preview-timeline", previewId, startDate, endDate] as const,
  previewOpen: (startDate: string, endDate: string, rowLimit: number) =>
    ["preview-open", startDate, endDate, rowLimit] as const,
  previewPoolPage: (
    previewId: string,
    tradeDate: string,
    limit: number,
    offset: number
  ) => ["preview-pool-page", previewId, tradeDate, limit, offset] as const,
  previewSecurityAnalysis: (
    previewId: string,
    tradeDate: string,
    securityCode: string,
    adjustment: string,
    maWindows: string,
    includeQuoteRows: boolean
  ) =>
    [
      "preview-security-analysis",
      previewId,
      tradeDate,
      securityCode,
      adjustment,
      maWindows,
      includeQuoteRows,
    ] as const,
  previewChartContext: (
    previewId: string,
    tradeDate: string,
    securityCode: string,
    adjustment: string,
    maWindows: string
  ) =>
    [
      "preview-chart-context",
      previewId,
      tradeDate,
      securityCode,
      adjustment,
      maWindows,
    ] as const,
}
