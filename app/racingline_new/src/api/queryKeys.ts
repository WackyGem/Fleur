import type { MetricsQuery } from "@/types/rearview"

export const queryKeys = {
  metrics: (query: MetricsQuery = {}) => ["metrics", query] as const,
}
