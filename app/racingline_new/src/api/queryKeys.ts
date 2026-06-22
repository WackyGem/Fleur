import type { MetricsQuery } from "@/types/rearview"

export const queryKeys = {
  metrics: (query: MetricsQuery = {}) => ["metrics", query] as const,
  previewPoolPage: (
    previewId: string,
    tradeDate: string,
    limit: number,
    offset: number
  ) => ["preview-pool-page", previewId, tradeDate, limit, offset] as const,
  previewSecurityAnalysis: (
    previewId: string,
    tradeDate: string,
    securityCode: string
  ) =>
    [
      "preview-security-analysis",
      previewId,
      tradeDate,
      securityCode,
    ] as const,
}
