import { describe, expect, it } from "vitest"

import { indicatorCatalog } from "@/features/strategy/catalog"
import type { IndicatorCatalog, MetricOption } from "@/features/strategy/types"
import {
  formatComparableIndicator,
  formatWeightIndicator,
  getTrendMovingAverageCatalogs,
} from "@/features/strategy/utils"

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

describe("indicator formatting", () => {
  it("uses catalog labels for metric display text", () => {
    const catalogs: IndicatorCatalog[] = [
      catalog("trend", [
        metric("price_ma_5", "MA5"),
        metric("price_ma_20", "MA20"),
      ]),
    ]

    expect(
      formatComparableIndicator(
        {
          catalogId: "trend",
          compareCatalogId: "trend",
          compareMetric: "price_ma_20",
          compareMultiplier: "1.5",
          metric: "price_ma_5",
          operator: "gt",
          target: "metric",
          value: "",
          valueEnd: "",
        },
        { catalogOptions: catalogs }
      )
    ).toBe("MA5 > MA20 * 1.5")
  })

  it("uses catalog labels for weight indicator summaries", () => {
    const catalogs: IndicatorCatalog[] = [
      catalog("momentum", [metric("kdj_j_value", "KDJ J")]),
    ]

    expect(
      formatWeightIndicator(
        {
          catalogId: "momentum",
          compareCatalogId: "momentum",
          compareMetric: "kdj_j_value",
          extraConditions: [],
          id: "weight-1",
          metric: "kdj_j_value",
          operator: "gte",
          score: 50,
          target: "value",
          value: "80",
          valueEnd: "",
        },
        { catalogOptions: catalogs }
      )
    ).toBe("KDJ J >= 80")
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

function metric(id: string, label = id): MetricOption {
  return {
    id,
    label,
    valueType: "number",
    allowedOps: ["gt"],
  }
}
