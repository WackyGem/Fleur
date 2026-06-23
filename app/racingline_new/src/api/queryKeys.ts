import type {
  MetricsQuery,
  StrategyBacktestValidateRequest,
} from "@/types/rearview"

export const queryKeys = {
  metrics: (query: MetricsQuery = {}) => ["metrics", query] as const,
  defaultMarketFeeTemplate: (market: string) =>
    ["market-fee-templates", "default", market] as const,
  strategyBacktestValidate: (
    request: StrategyBacktestValidateRequest | null
  ) => ["strategy-backtests", "validate", request] as const,
  previewTimeline: (previewId: string, startDate: string, endDate: string) =>
    ["preview-timeline", previewId, startDate, endDate] as const,
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
}
