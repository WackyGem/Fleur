import { describe, expect, it } from "vitest"

import { indicatorCatalog } from "@/features/strategy/catalog"
import type { IndicatorCatalog, MetricOption } from "@/features/strategy/types"
import { getTrendMovingAverageCatalogs } from "@/features/strategy/utils"

describe("getTrendMovingAverageCatalogs", () => {
  it("keeps only trend main-chart moving average metrics from the fallback catalog", () => {
    const metrics = getTrendMovingAverageCatalogs(indicatorCatalog).flatMap(
      (catalog) => catalog.metrics.map((metric) => metric.id)
    )

    expect(metrics).toEqual(
      expect.arrayContaining([
        "price_ma_3",
        "price_ma_250",
        "price_avg_ma_3_6_12_24",
        "price_avg_ma_14_28_57_114",
        "price_ema2_10",
      ])
    )
    expect(metrics).not.toContain("boll_lower_20_2")
  })

  it("excludes trend previous metrics from backend catalog options", () => {
    const catalogs: IndicatorCatalog[] = [
      catalog("trend", [
        metric("price_ma_5"),
        metric("price_avg_ma_3_6_12_24"),
        metric("price_ema2_10"),
        metric("boll_lower_20_2"),
      ]),
      catalog("trend_previous", [metric("prev_price_ma_5")]),
    ]

    const metrics = getTrendMovingAverageCatalogs(catalogs).flatMap((item) =>
      item.metrics.map((metric) => metric.id)
    )

    expect(metrics).toEqual([
      "price_ma_5",
      "price_avg_ma_3_6_12_24",
      "price_ema2_10",
    ])
  })
})

function catalog(id: string, metrics: MetricOption[]): IndicatorCatalog {
  return {
    id,
    label: id,
    source: "mart_stock_trend_indicator_daily",
    metrics,
  }
}

function metric(id: string): MetricOption {
  return {
    id,
    label: id,
    valueType: "number",
    allowedOps: ["gt"],
  }
}
